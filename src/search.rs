use core::marker::PhantomData;

use crate::{NavCursor, node::internal_details::NodeRef};

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
    fn should_explore_left(
        &self,
        left_key: &K,
        left_value: &V,
        left_stats: &S,
        current_key: &K,
    ) -> bool;

    /// Determines if the right child branch should be explored or pruned.
    fn should_explore_right(
        &self,
        right_key: &K,
        right_value: &V,
        right_stats: &S,
        current_key: &K,
    ) -> bool;
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
    type Item = (&'a K, &'a V);

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
                        if self
                            .policy
                            .should_explore_left(left_key, left_value, left_stats, key)
                        {
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
                            right_key,
                            right_value,
                            right_stats,
                            key,
                        ) {
                            // OK this tells me I can resume search in the right subtree.
                            self.cur = Some(right_node);
                            self.direction = TraversalPhase::Above; // Reset direction for the right sub-hierarchy

                            // Yield the current matching parent node.
                            // The cursor is staged inside the fresh right subtree for the next loop.
                            if is_matching_node {
                                return Some((key, value));
                            }
                            continue;
                        }
                    }

                    // I am not able to move to the right subtree, so I need to ascend and update the direction state
                    if is_matching_node {
                        self.ascend_and_update_state();
                        return Some((key, value));
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
    pub(crate) fn new(cur: Option<NodeRef<K, V, S>>, policy: P) -> Self {
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
    /// # Example
    /// ```rust
    /// // 1. Grab a safe public cursor from anywhere in the tree
    /// let mut cursor = tree.lower_bound(Bound::Included(&10));
    ///
    /// // 2. Spin up an InOrderIter from that cursor position with a custom policy
    /// let custom_iterator = InOrderIter::from_cursor(cursor, MyCustomPolicy);
    /// ```
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
            if let Some(parent) = node.parent() {
                if parent.left() == Some(node) {
                    self.direction = TraversalPhase::Left;
                } else {
                    self.direction = TraversalPhase::Right;
                }

                self.cur = Some(parent);
            } else {
                self.cur = None;
            }
        }
    }
}
