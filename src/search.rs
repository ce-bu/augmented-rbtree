//! Custom, Zero-Allocation Search and Traversal for Augmented Red-Black Trees.
//!
//! This module provides highly optimized primitives for executing targeted queries over an
//! augmented binary search tree. Instead of performing traditional, unguided tree traversals,
//! this module abstracts structural pruning logic to allow fast, custom-indexed lookups
//! (such as range queries, interval intersections, or weight-based selections).
//!
//! # Architecture & Pruning Philosophy
//!
//! Traditional search algorithms tightly couple tree geometry with search criteria. This module
//! breaks that coupling using a decoupled, policy-driven architecture via the [`InOrderPruningPolicy`] trait:
//!
//! 1. **Tree Geometry**: Managed natively by the stateful [`InOrderIter`].
//! 2. **Pruning Strategy**: Dictated by an external component implementing [`InOrderPruningPolicy`].
//!    Before descending into a subtree, the iterator asks the policy whether that branch should
//!    be evaluated or structurally pruned.
//!
//! # Memory & Performance Characteristics
//!
//! - **Space Complexity**: `O(1)` auxiliary space. The traversal uses a persistent, re-entrant
//!   state machine ([`InOrderIter`]) tracking structural geometry and the current [`TraversalPhase`].
//!   It requires **zero allocations**, avoiding heap-allocated backtracking vectors or call stacks.
//! - **Time Complexity**: Bounds range from `O(log N)` for highly constrained pruning policies
//!   (e.g., singular key lookups) up to `O(N)` for exhaustive scans.
//!
//! # Subtree Constraints & Re-entrancy
//!
//! To support flexible querying, [`InOrderIter`] can be bound to a specific `subtree_root`. The iterator
//! enforces strict structural boundaries: it will **never drift higher** than or escape the bounds of
//! the initialized subtree. Because it preserves its exact geometric location across invocations,
//! it can be safely paused, resumed, or used to build higher-level streaming interfaces.
//!
//! # Examples
//!
//! ```
//! # use augmented_rbtree::{InOrderPruningPolicy, InOrderIter};
//! // Define a policy that skips metadata categories
//! struct MyCustomSearchPolicy;
//!
//! impl InOrderPruningPolicy<u64, String, u32> for MyCustomSearchPolicy {
//!     fn is_match(&self, _k: &u64, _v: &String, stats: &u32) -> bool {
//!         *stats > 100 // Only match nodes with high augmented weight
//!     }
//!     fn should_explore_left(&self, left: (&u64, &String, &u32), _: (&u64, &String, &u32)) -> bool {
//!         *left.2 > 50 // Prune left child if its subtree total weight is too low
//!     }
//!     fn should_explore_right(&self, right: (&u64, &String, &u32), _: (&u64, &String, &u32)) -> bool {
//!         true // Always check right
//!     }
//! }
//! ```

use core::{borrow::Borrow, marker::PhantomData};

use crate::{
    AugmentedRBTreeInt, alloc_proxy::proxy::Allocator, augmented_rbtree::TreeLocation,
    cursor::NavCursor, node::internal_details::NodeRef, policy,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TraversalPhase {
    Above,
    Left,
    Right,
}

/// A policy trait that separates structural pruning rules from the tree architecture.
pub trait InOrderPruningPolicy<K, V, S> {
    /// Evaluates if the current node satisfies the lookup constraints.
    fn is_match(&self, key: &K, value: &V, stats: &S) -> bool;

    /// Determines if the left child branch should be explored or pruned.
    fn should_explore_left(&self, left: (&K, &V, &S), current: (&K, &V, &S)) -> bool;

    /// Determines if the right child branch should be explored or pruned.
    fn should_explore_right(&self, right: (&K, &V, &S), current: (&K, &V, &S)) -> bool;
}

/// A stateful, direction-aware iterator that performs an in-order Depth-First Search (DFS)
/// over an augmented binary search tree.
///
/// This iterator leverages the physical geometry of the tree structure combined with a persistent
/// direction state to avoid allocating a backtracking vector or an internal node call stack.
/// It is completely re-entrant, allowing it to yield intermediate values safely across successive
/// invocations of `.next()`.
///
/// To protect against drifting out of bounds during a targeted search, it enforces a structural
/// boundary check that prevents the cursor from overshooting the original subtree root node.
///
/// # Type Parameters
/// * `K` - The tree node key type.
/// * `V` - The tree node value type.
/// * `S` - The augmented subtree statistics type used by the pruning policy.
/// * `P` - A type implementing [`InOrderPruningPolicy`] to dictate matching and pruning criteria.

#[derive(Debug)]
pub struct InOrderIter<'a, K, V, S, P>
where
    P: InOrderPruningPolicy<K, V, S>,
{
    cur: Option<NodeRef<K, V, S>>,
    policy: P,
    subtree_root: Option<NodeRef<K, V, S>>,
    direction: TraversalPhase,
    _marker: PhantomData<&'a (K, V, S)>,
}

impl<'a, K, V, S, P> Iterator for InOrderIter<'a, K, V, S, P>
where
    P: InOrderPruningPolicy<K, V, S>,
{
    type Item = (&'a K, &'a V, &'a S);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Retrieve current node details. If None, the cursor space has been exhausted.
            let node = self.cur?;

            let (key, value, stats) = unsafe { (node.key(), node.value(), node.stats()) };

            match self.direction {
                TraversalPhase::Above => {
                    // Check if we can traverse the left subtree first
                    if let Some(left_node) = node.left() {
                        let (left_key, left_value, left_stats) =
                            unsafe { (left_node.key(), left_node.value(), left_node.stats()) };
                        if self.policy.should_explore_left(
                            (left_key, left_value, left_stats),
                            (key, value, stats),
                        ) {
                            self.cur = Some(left_node);
                            self.direction = TraversalPhase::Above; // Reset direction for the left sub-hierarchy
                            continue;
                        }
                    }
                    // Left subtree is absent or pruned. We say that we returned to the current from left
                    self.direction = TraversalPhase::Left;
                }

                TraversalPhase::Left => {
                    // We hit the current node from the left child, so we can evaluate it now
                    // The policy is to yield the node coming from the bottom

                    let is_matching_node = self.policy.is_match(key, value, stats);

                    if let Some(right_node) = node.right() {
                        let (right_key, right_value, right_stats) =
                            unsafe { (right_node.key(), right_node.value(), right_node.stats()) };
                        if self.policy.should_explore_right(
                            (right_key, right_value, right_stats),
                            (key, value, stats),
                        ) {
                            // OK this tells me I can resume search in the right subtree.
                            self.cur = Some(right_node);
                            self.direction = TraversalPhase::Above; // Reset direction for the right sub-hierarchy

                            // Yield the current matching parent node.
                            // The cursor is staged inside the fresh right subtree for the next loop.
                            if is_matching_node {
                                return Some((key, value, stats));
                            }
                            continue;
                        }
                    }

                    // I am not able to move to the right subtree, so I need to ascend and update the direction state
                    if is_matching_node {
                        self.ascend_and_update_state();
                        return Some((key, value, stats));
                    }

                    self.ascend_and_update_state();
                }

                TraversalPhase::Right => {
                    // Done with both subtrees, we yielded the current node and now we need to ascend to the parent
                    self.ascend_and_update_state();
                }
            }
        }
    }
}

impl<K, V, S, P> InOrderIter<'_, K, V, S, P>
where
    P: InOrderPruningPolicy<K, V, S>,
{
    /// Constructs a new `InOrderIter` starting at the provided node position.
    ///
    /// This method automatically captures the starting node position as the structural ceiling
    /// for the traversal, ensuring it does not overshoot into adjacent tree families.
    pub fn new<A, R, Q>(
        tree: &AugmentedRBTreeInt<K, V, S, A, R>,
        location: TreeLocation<&Q>,
        policy: P,
    ) -> Self
    where
        A: Allocator,
        R: policy::internal_details::TreePolicy<K = K, V = V, S = S>,
        K: Borrow<Q> + Ord,
        Q: Ord,
    {
        let cur = tree.get_tree_location(location);
        Self {
            cur,
            policy,
            subtree_root: cur, // Directly passes the option without conditional blocks
            direction: TraversalPhase::Above,
            _marker: PhantomData,
        }
    }

    /// Constructs a new `InOrderIter` starting at the node currently pointed to by a [`NavCursor`].
    ///
    /// This allows power-users to initialize a highly customized pruning search originating from
    /// any arbitrary bookmark or position in the tree structure.
    ///
    pub fn from_cursor(cursor: &NavCursor<'_, K, V, S>, policy: P) -> Self {
        // Safe bridge: Unpack the internal Option<NodeRef> from the public cursor
        let starting_node = cursor.current;

        Self {
            cur: starting_node,
            policy,
            subtree_root: starting_node,
            direction: TraversalPhase::Above,
            _marker: PhantomData,
        }
    }

    /// Shifts the cursor upward by exactly one level while protecting against overshooting
    /// the designated subtree root boundary.
    fn ascend_and_update_state(&mut self) {
        // Stop instantly if the current node matches our initial subtree ceiling.
        if self.cur == self.subtree_root {
            self.cur = None; // O(1) instant termination
            return;
        }

        if let Some(node) = self.cur {
            // Invariant: Because we checked the subtree_root boundary above,
            // this node is guaranteed to have a parent in a valid tree.
            let parent = node
                .parent()
                .expect("Invariant violation: Node must have a parent");

            if parent.left() == Some(node) {
                self.direction = TraversalPhase::Left;
            } else {
                self.direction = TraversalPhase::Right;
            }

            self.cur = Some(parent);
        }
    }
}
