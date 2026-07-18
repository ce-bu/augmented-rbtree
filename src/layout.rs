use crate::augmented_rbtree::OutOfMemoryError;
use crate::iterators::RangeBoundsLimits;
use crate::node::internal_details::NodeRef;
use crate::node_allocator::NodeAllocator;
use crate::policy::internal_details::TreePolicy;
use crate::{alloc_proxy::proxy::Allocator, node::Color};
use core::{borrow::Borrow, cmp, marker::PhantomData};

/// A layout for an augmented Red-Black Tree that supports augmentation through the `Augment` trait.
#[derive(Debug)]
pub(crate) struct AugmentedRBTreeLayout<K, V, S, A, P>
where
    P: TreePolicy<K = K, V = V, S = S>,
{
    pub(crate) root: Option<NodeRef<K, V, S>>,
    pub(crate) node_allocator: NodeAllocator<A>,
    pub(crate) _marker: PhantomData<fn() -> P>,
}

unsafe impl<K, V, S, A, P> Send for AugmentedRBTreeLayout<K, V, S, A, P>
where
    K: Send,
    V: Send,
    S: Send,
    A: Allocator + Send,
    P: TreePolicy<K = K, V = V, S = S>,
{
}

unsafe impl<K, V, S, A, P> Sync for AugmentedRBTreeLayout<K, V, S, A, P>
where
    K: Sync,
    V: Sync,
    S: Sync,
    A: Allocator + Sync,
    P: TreePolicy<K = K, V = V, S = S>,
{
}

impl<K, V, S, A: Allocator, P: TreePolicy<K = K, V = V, S = S>>
    AugmentedRBTreeLayout<K, V, S, A, P>
{
    /// Creates a new, empty `AugmentedRBTree`.
    #[inline]
    pub fn new_in(alloc: A) -> Self {
        Self {
            root: None,
            node_allocator: NodeAllocator::new(alloc),
            _marker: PhantomData,
        }
    }

    /// Inserts a key-value pair into the tree. If the key already exists, its value is updated and the old value is returned.
    /// If the key does not exist, it is inserted and `None` is returned.
    pub fn try_insert_node(&mut self, key: K, value: V) -> Result<Option<V>, OutOfMemoryError>
    where
        K: Ord,
    {
        let Some(mut current) = self.root else {
            let stats = P::compute(&key, &value, None, None);
            let new_node = self.node_allocator.alloc_node(key, value, stats)?;
            new_node.set_color(Color::Black);
            self.root = Some(new_node);
            return Ok(None);
        };

        let (parent, insert_left) = loop {
            let current_key = unsafe { current.key() };

            match key.cmp(current_key) {
                cmp::Ordering::Less => {
                    if let Some(left) = current.left() {
                        current = left;
                    } else {
                        break (current, true);
                    }
                }
                cmp::Ordering::Equal => {
                    let old_value = core::mem::replace(unsafe { current.value_mut() }, value);
                    P::augment(current); // Recompute this node first, then propagate up
                    P::augment_upstream(current);
                    return Ok(Some(old_value));
                }
                cmp::Ordering::Greater => {
                    if let Some(right) = current.right() {
                        current = right;
                    } else {
                        break (current, false);
                    }
                }
            }
        };

        let stats = P::compute(&key, &value, None, None);
        let new_node = self.node_allocator.alloc_node(key, value, stats)?;
        new_node.set_parent(Some(parent));

        if insert_left {
            parent.set_left(Some(new_node));
        } else {
            parent.set_right(Some(new_node));
        }

        P::augment(parent);
        P::augment_upstream(parent);
        self.insert_fixup(new_node);

        Ok(None)
    }

    /// Like `insert_node` but returns the `NodeRef` of the inserted (or updated) node.
    /// Used by the `Entry` API to avoid requiring `K: Clone`.    
    pub(crate) fn try_insert_node_get_ref(
        &mut self,
        key: K,
        value: V,
    ) -> Result<NodeRef<K, V, S>, OutOfMemoryError>
    where
        K: Ord,
    {
        let Some(mut current) = self.root else {
            let stats = P::compute(&key, &value, None, None);
            let new_node = self.node_allocator.alloc_node(key, value, stats)?;
            new_node.set_color(Color::Black);
            self.root = Some(new_node);
            return Ok(new_node);
        };

        let (parent, insert_left) = loop {
            let current_key = unsafe { current.key() };
            match key.cmp(current_key) {
                cmp::Ordering::Less => {
                    if let Some(left) = current.left() {
                        current = left;
                    } else {
                        break (current, true);
                    }
                }
                cmp::Ordering::Equal => {
                    let _ = core::mem::replace(unsafe { current.value_mut() }, value);
                    P::augment(current);
                    P::augment_upstream(current);
                    return Ok(current);
                }
                cmp::Ordering::Greater => {
                    if let Some(right) = current.right() {
                        current = right;
                    } else {
                        break (current, false);
                    }
                }
            }
        };

        let stats = P::compute(&key, &value, None, None);
        let new_node = self.node_allocator.alloc_node(key, value, stats)?;
        new_node.set_parent(Some(parent));
        if insert_left {
            parent.set_left(Some(new_node));
        } else {
            parent.set_right(Some(new_node));
        }
        P::augment(parent);
        P::augment_upstream(parent);
        self.insert_fixup(new_node);
        Ok(new_node)
    }

    /// Adjusts the tree after an insertion to maintain Red-Black Tree properties.
    ///
    /// ## Violation Context
    /// A new node `Z` is always inserted as **Red**. A violation occurs only if its
    /// parent `P` is also **Red**. `G` represents the Grandparent, and `U` represents the Uncle.
    ///
    /// ## Insertion Fixup Cases (Left-Side Parent P)
    /// The diagrams below assume `P` is the **left** child of `G`.
    ///
    /// ### Case 1: Uncle `U` is RED (Recolor Only)
    /// ```text
    ///        Before Recoloring                     After Recoloring
    ///              G (Black)                             G (Red)
    ///             / \                                   / \
    ///            /   \                                 /   \
    ///       P (Red)   U (Red)                     P (Black) U (Black)
    ///         /                                     /
    ///        /                                     /
    ///     Z (Red)                               Z (Red)
    /// ```
    /// * **Action:** Recolor `P` and `U` to Black. Recolor `G` to Red.
    /// * **Next:** Move pointer up (`Z = G`) and repeat the loop.
    ///
    /// ### Case 2: Uncle `U` is BLACK, `Z` is a Right Child (Triangle)
    /// ```text
    ///         Before Rotation                       After Rotation
    ///              G (Black)                             G (Black)
    ///             / \                                   / \
    ///            /   \                                 /   \
    ///       P (Red)   U (Black)                   Z (Red)   U (Black)
    ///         \                                     /
    ///          \                                   /
    ///           Z (Red)                         P (Red)
    /// ```
    /// * **Action:** Left-Rotate around `P`.
    /// * **Next:** Point `Z` to `P` (the new child) to immediately execute Case 3.
    ///
    /// ### Case 3: Uncle `U` is BLACK, `Z` is a Left Child (Line)
    /// ```text
    ///         Before Rotation                       After Rotation
    ///              G (Black)                             P (Black)
    ///             / \                                   / \
    ///            /   \                                 /   \
    ///       P (Red)   U (Black)                     Z (Red) G (Red)
    ///         /                                               \
    ///        /                                                 \
    ///     Z (Red)                                               U (Black)
    /// ```
    /// * **Action:** Recolor `P` to Black, `G` to Red. Right-Rotate around `G`.
    /// * **Next:** Violation is resolved. Loop terminates.
    ///
    /// ## Symmetric Cases (Right-Side Parent P)
    /// If `P` is the **right** child of `G`, the operations are perfectly mirrored:
    /// * **Case 1 (Symmetric):** `U` (left child) is Red. Recolor `P` and `U` Black, `G` Red. Move `Z = G`.
    /// * **Case 2 (Symmetric):** `U` is Black, `Z` is a left child. Right-Rotate around `P` to form a line.
    /// * **Case 3 (Symmetric):** `U` is Black, `Z` is a right child. Recolor `P` Black, `G` Red. Left-Rotate around `G`.
    fn insert_fixup(&mut self, mut node: NodeRef<K, V, S>)
    where
        K: Ord,
    {
        while let Some(mut parent) = node.parent() {
            if parent.color() == Color::Black {
                break;
            }

            let grandparent = parent
                .parent()
                .expect("A Red parent node cannot be the root and must have a grandparent node.");

            if grandparent.left() == Some(parent) {
                let uncle = grandparent.right();
                if uncle.map(NodeRef::color) == Some(Color::Red) {
                    let uncle = uncle.unwrap();
                    // LXr Case: Recoloring
                    parent.set_color(Color::Black);
                    uncle.set_color(Color::Black);
                    grandparent.set_color(Color::Red);

                    node = grandparent;
                } else {
                    if parent.right() == Some(node) {
                        // LRb Case: Left Rotate to get to LLb Case
                        let tmp = node;
                        node = parent;
                        self.left_rotate(node);
                        parent = tmp;
                    }
                    // LLb Case: Right Rotate
                    parent.set_color(Color::Black);
                    grandparent.set_color(Color::Red);

                    self.right_rotate(grandparent);
                }
            } else {
                let uncle = grandparent.left();
                if uncle.map(NodeRef::color) == Some(Color::Red) {
                    let uncle = uncle.unwrap();
                    // RXr Case: Recoloring
                    parent.set_color(Color::Black);
                    uncle.set_color(Color::Black);
                    grandparent.set_color(Color::Red);
                    node = grandparent;
                } else {
                    if parent.left() == Some(node) {
                        //RLb Case: Right Rotate to get to RRb Case
                        let tmp = node;
                        node = parent;
                        self.right_rotate(node);
                        parent = tmp;
                    }
                    // RRb Case: Left Rotate
                    parent.set_color(Color::Black);
                    grandparent.set_color(Color::Red);
                    self.left_rotate(grandparent);
                }
            }
        }

        self.root.expect("Root must exists").set_color(Color::Black);

        P::augment_upstream(node);
    }

    /// Performs a Left-Rotation around node `x` to pivot its right child `y` upward.
    ///
    /// This structural operation maintains the Binary Search Tree ordering property
    /// while shifting subtrees to rebalance depth.
    ///
    /// ## Complete Left-Rotation Structural Transition
    /// ```text
    ///            Before Rotation                       After Rotation
    ///              X.Parent                              X.Parent
    ///                 |                                     |
    ///                (x)                                   (y)
    ///               /   \                                 /   \
    ///              α     (y)                ==>         (x)    γ
    ///                   /   \                          /   \
    ///                  β     γ                        α     β
    /// ```
    ///
    /// ## Step-by-Step Code Execution Trace
    ///
    /// ### Step 1: Unlinking β from `y` and making it `x`'s new right child
    /// ```text
    ///      let y = Node::right(x).expect(...);
    ///      Node::set_right(x, Node::left(y));
    ///
    ///             (x)                                   (y)
    ///            /   \                                 /   \
    ///           α     β  <-- [Moved]                  NIL   γ
    /// ```
    ///
    /// ### Step 2: Updating β's parent pointer (if β is a real node)
    /// ```text
    ///      if let Some(left_y) = Node::left(y) {
    ///          Node::set_parent(left_y, Some(x));
    ///      }
    ///
    ///            (x)
    ///           /   \
    ///          α     β  <-- Parent now points back up to x
    /// ```
    ///
    /// ### Step 3: Splicing `y` into `x`'s original position under `X.Parent`
    /// ```text
    ///      Node::set_parent(y, Node::parent(x));
    ///      if let Some(x_parent) = Node::parent(x) {
    ///          if Some(x) == Node::left(x_parent) { Node::set_left(x_parent, Some(y)); }
    ///          else { Node::set_right(x_parent, Some(y)); }
    ///      } else { self.root = Some(y); }
    ///
    ///             X.Parent
    ///                |
    ///               (y)  <-- Now correctly linked underneath x's old parent
    ///              /   \
    ///            (NIL)  γ
    /// ```
    ///
    /// ### Step 4: Making `x` the left child of `y` (Completing the pivot)
    /// ```text
    ///      Node::set_left(y, Some(x));
    ///      Node::set_parent(x, Some(y));
    ///
    ///               (y)
    ///              /   \
    ///            (x)    γ
    ///           /   \
    ///          α     β
    /// ```
    ///
    /// ### Step 5: Metadata Recomputation
    /// ```text
    ///      Node::augment(x);
    ///      Node::augment(y);
    /// ```
    /// * **Note:** `x` must be augmented **before** `y` because `x` is now a child of `y`.
    ///   Its values must be completely correct so `y` can read them to update itself.
    fn left_rotate(&mut self, x: NodeRef<K, V, S>) {
        let y = x.right().expect("Right child must exist for left rotation");

        x.set_right(y.left());

        if let Some(left_y) = y.left() {
            left_y.set_parent(Some(x));
        }

        y.set_parent(x.parent());

        if let Some(x_parent) = x.parent() {
            if Some(x) == x_parent.left() {
                x_parent.set_left(Some(y));
            } else {
                x_parent.set_right(Some(y));
            }
        } else {
            self.root = Some(y);
        }

        y.set_left(Some(x));
        x.set_parent(Some(y));

        P::augment(x);
        P::augment(y);
        P::augment_upstream(y);
    }

    /// Performs a Right-Rotation around node `y` to pivot its left child `x` upward.
    ///
    /// This structural operation maintains the Binary Search Tree ordering property
    /// while shifting subtrees to rebalance depth.
    ///
    /// ## Complete Right-Rotation Structural Transition
    /// ```text
    ///            Before Rotation                       After Rotation
    ///              Y.Parent                              Y.Parent
    ///                 |                                     |
    ///                (y)                                   (x)
    ///               /   \                                 /   \
    ///              (x)   γ                  ==>          α    (y)
    ///             /   \                                      /   \
    ///            α     β                                    β     γ
    /// ```
    ///
    /// ## Step-by-Step Code Execution Trace
    ///
    /// ### Step 1: Unlinking β from `x` and making it `y`s new left child
    /// ```text
    ///      let x = Node::left(y).expect(...);
    ///      Node::set_left(y, Node::right(x));
    ///
    ///             (y)                                   (x)
    ///            /   \                                 /   \
    ///  [Moved] --> β   γ                              α    NIL
    /// ```
    ///
    /// ### Step 2: Updating β's parent pointer (if β is a real node)
    /// ```text
    ///      if let Some(right_x) = Node::right(x) {
    ///          Node::set_parent(right_x, Some(y));
    ///      }
    ///
    ///            (y)
    ///           /   \
    ///          β     γ  <-- Parent now points back up to y
    /// ```
    ///
    /// ### Step 3: Splicing `x` into `y`'s original position under `Y.Parent`
    /// ```text
    ///      Node::set_parent(x, Node::parent(y));
    ///      if let Some(y_parent) = Node::parent(y) {
    ///          if Some(y) == Node::left(y_parent) { Node::set_left(y_parent, Some(x)); }
    ///          else { Node::set_right(y_parent, Some(x)); }
    ///      } else { self.root = Some(x); }
    ///
    ///             Y.Parent
    ///                |
    ///               (x)  <-- Now correctly linked underneath y's old parent
    ///              /   \
    ///             α   (NIL)
    /// ```
    ///
    /// ### Step 4: Making `y` the right child of `x` (Completing the pivot)
    /// ```text
    ///      Node::set_right(x, Some(y));
    ///      Node::set_parent(y, Some(x));
    ///
    ///               (x)
    ///              /   \
    ///             α     (y)
    ///                  /   \
    ///                 β     γ
    /// ```
    ///
    /// ### Step 5: Metadata Recomputation
    /// ```text
    ///      Node::augment(y);
    ///      Node::augment(x);
    /// ```
    /// * **Note:** `y` must be augmented **before** `x` because `y` is now a child of `x`.
    ///   Its values must be completely correct so `x` can read them to update itself.
    fn right_rotate(&mut self, y: NodeRef<K, V, S>) {
        let x = y.left().expect("Left child must exist for right rotation");

        y.set_left(x.right());

        if let Some(right_x) = x.right() {
            right_x.set_parent(Some(y));
        }

        x.set_parent(y.parent());
        if let Some(y_parent) = y.parent() {
            if Some(y) == y_parent.left() {
                y_parent.set_left(Some(x));
            } else {
                y_parent.set_right(Some(x));
            }
        } else {
            self.root = Some(x);
        }

        x.set_right(Some(y));
        y.set_parent(Some(x));

        P::augment(y);
        P::augment(x);
        P::augment_upstream(x);
    }

    /// Replaces the subtree rooted at node `u` with the subtree rooted at node `v`.
    ///
    /// This structural operation binds `v` to `u`'s original parent. It is
    /// decoupled from colors and handles three pointer configuration cases:
    ///
    /// ## Case 1: `u` is the Left Child of its Parent
    /// ```text
    ///        Before Transplant                       After Transplant
    ///             Parent                                  Parent
    ///             /    \                                  /    \
    ///          [u]      Sibling             ==>          v      Sibling
    ///          / \                                      / \
    ///       u.L   u.R                                v.L   v.R
    /// ```
    /// * **Parent Update:** `Parent.Left` is reassigned to `v`.
    /// * **Child Update:** `v.Parent` is pointed up to `Parent`.
    ///
    /// ## Case 2: `u` is the Right Child of its Parent
    /// ```text
    ///        Before Transplant                       After Transplant
    ///             Parent                                  Parent
    ///             /    \                                  /    \
    ///       Sibling    [u]                  ==>    Sibling     v
    ///                  / \                                    / \
    ///               u.L   u.R                              v.L   v.R
    /// ```
    /// * **Parent Update:** `Parent.Right` is reassigned to `v`.
    /// * **Child Update:** `v.Parent` is pointed up to `Parent`.
    ///
    /// ## Case 3: `u` is the Root of the Tree
    /// ```text
    ///        Before Transplant                       After Transplant
    ///          Tree.Root = [u]                         Tree.Root = v
    ///                / \                                     / \
    ///             u.L   u.R                               v.L   v.R
    /// ```
    /// * **Tree Update:** `Tree.Root` is updated directly to point to `v`.
    /// * **Child Update:** `v.Parent` is set to null/None.
    fn transplant(&mut self, u: NodeRef<K, V, S>, v: Option<NodeRef<K, V, S>>) {
        if let Some(parent) = u.parent() {
            if Some(u) == parent.left() {
                parent.set_left(v);
            } else {
                parent.set_right(v);
            }
        } else {
            self.root = v;
        }

        if let Some(v) = v {
            v.set_parent(u.parent());
        }
    }

    pub(crate) fn find_node<Q>(&self, key: &Q) -> Option<NodeRef<K, V, S>>
    where
        K: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
    {
        let mut current = self.root;
        while let Some(current_ref) = current {
            let current_key = unsafe { current_ref.key() };
            match key.cmp(current_key.borrow()) {
                core::cmp::Ordering::Less => {
                    current = current_ref.left();
                }
                core::cmp::Ordering::Equal => {
                    return Some(current_ref);
                }
                core::cmp::Ordering::Greater => {
                    current = current_ref.right();
                }
            }
        }

        None
    }

    /// =============================================================================
    /// RED-BLACK TREE DELETION CASES
    /// =============================================================================
    /// DEFINITION OF "DOUBLE BLACK"
    /// =============================================================================
    /// A "Double Black" is not a structural node type or an actual structural color
    /// field value in the final tree. Instead, it is a structural concept or property
    /// assigned to a specific pointer location (or a node variable `x`) during
    /// deletion.
    /// When a Black node is removed or shifted, all paths passing through its
    /// replacement node `x` suddenly contain one fewer Black node than all other
    /// paths in the tree, violating the core Red-Black Tree property (that every
    /// path from a node to a descendant leaf contains the same number of black nodes).
    /// To track this structural deficit, we conceptually "charge" the node `x` or
    /// its pointer location with an extra layer of blackness:
    /// 1. If `x` was Red: It absorbs the Double Black charge and simply becomes
    ///    a regular Black node. The deficit is instantly fixed.
    /// 2. If `x` is Black (or is a `None` / Null leaf): It becomes Double Black.
    ///    It counts as TWO Black nodes for the purpose of path calculations.
    ///    The Deletion Fixup algorithm must then run to push this extra black layer
    ///    up or around the tree using rotations and recoloring until it can be
    ///    absorbed by a Red node or eliminated at the root.
    ///    =============================================================================
    ///    Legenda:
    ///    [z]   = Node to be deleted
    ///    (x)   = Node being promoted (tracks the potential Double Black location)
    ///    (y)   = In-order successor (minimum node of z's right subtree)
    ///    z.L   = Left child / subtree of z
    ///    z.R   = Right child / subtree of z
    ///    x.L/R = Left/Right child of x
    ///    None  = Empty child pointer
    ///    =============================================================================
    /// -----------------------------------------------------------------------------
    /// Case 1: z.left is None
    /// -----------------------------------------------------------------------------
    /// Its right child x is promoted directly to z's position.
    /// ```text
    ///      Before Deletion             After Deletion
    ///         z.parent                    z.parent
    ///            |                           |
    ///           [z]          ==>            (x)
    ///          /   \                       /   \
    ///       None   (x)                    x.L   x.R
    /// ```
    /// Tracking:      x = z.right
    /// Action:        1. transplant(z, x)
    ///                2. if z.color == Black { `delete_fixup(x)` }
    /// Fixup Trigger: If z was Black, x becomes Double Black.
    /// Why:           Paths through x just lost node z. They are now short exactly 1
    ///                Black node compared to the rest of the tree. x is charged with an
    ///                extra "Double Black" layer to track and account for this path deficit.
    /// -----------------------------------------------------------------------------
    /// Case 2: z.right is None
    /// -----------------------------------------------------------------------------
    /// Its left child x is promoted directly to z's position.
    /// ```text
    ///      Before Deletion             After Deletion
    ///         z.parent                    z.parent
    ///            |                           |
    ///           [z]          ==>            (x)
    ///          /   \                       /   \
    ///         (x)  None                   x.L   x.R
    /// ```
    /// Tracking:      x = z.left
    /// Action:        1. transplant(z, x)
    ///                2. if z.color == Black { `delete_fixup(x)` }
    /// Fixup Trigger: If z was Black, x becomes Double Black.
    /// Why:           Unlinking Black node z removes a Black node from all paths
    ///                running down through x. x carries a temporary "Double Black"
    ///                charge to hold the place of that missing Black node until the tree
    ///                is rebalanced.
    /// -----------------------------------------------------------------------------
    /// Case 3: z has Two Children (z.left and z.right are both valid nodes)
    /// -----------------------------------------------------------------------------
    /// The in-order successor node y is located. x is set to y's right child.
    /// -----------------------------------------------------------------------------
    /// Subcase 3a: Successor y is the Direct Right Child of z
    /// -----------------------------------------------------------------------------
    /// x is promoted to y's position, and y replaces z.
    /// ```text
    ///        Before Deletion                   After Deletion
    ///           z.parent                          z.parent
    ///              |                                 |
    ///             [z]                               (y)
    ///            /   \                             /   \
    ///          (z.L)  (y)          ==>           (z.L)  (x)
    ///                /   \                             /   \
    ///              None  (x)                         x.L   x.R
    /// ```
    /// Tracking:      y = z.right, x = y.right, `y_original_color` = y.color
    /// Action:        1. transplant(z, y)
    ///                2. y.left = z.left; y.left.parent = y
    ///                3. y.color = z.color
    ///                4. if `y_original_color` == Black `delete_fixup(x)`x) }
    /// Fixup Trigger: If y's original color was Black, x becomes Double Black.
    /// Why:           When y leaves its position to replace z, paths passing down
    ///                through x lose y's original Black contribution. Even though y
    ///                takes z's position, it adopts z's color, meaning y's original
    ///                Black identity is lost to x's subtree. x must absorb a
    ///                "Double Black" layer to compensate for this local deficit.
    /// Color Scenarios for Subcase 3a:
    /// * Scenario A: z is BLACK, y is BLACK
    ///   - Net impact on upper tree: y leaves its spot and takes z's place. Since
    ///     both were Black, the upper structural colors feel like 2 Black nodes
    ///     became 1. Upper paths lose 1 Black node.
    ///   - Net impact on x paths: Paths through x specifically lose BOTH z and y from
    ///     their path chain, while gaining y (acting as Black) back at the top.
    ///     Result: -1 -1 +1 = -1 Black node deficit. Fixup is triggered at x.
    /// * Scenario B: z is RED, y is BLACK
    ///   - Net impact on upper tree: y leaves its spot and takes z's place, adopting
    ///     z's RED color. The upper structural paths lose 0 Black nodes (a Red node
    ///     simply became a Red node).
    ///   - Net impact on x paths: Paths through x lose Black node y and Red node z
    ///     from their chain, while gaining y (acting as RED) back at the top.
    ///     Result: -1 -0 +0 = -1 Black node deficit. Fixup is triggered at x.
    /// * Scenario C: z is BLACK or RED, y is RED
    ///   - If y is originally Red, paths running down through x never counted y as
    ///     a Black node anyway. When y shifts and changes color to match z, paths
    ///     passing through x experience no change in their total Black node count.
    ///     Result: 0 Black node deficit. Fixup is NOT triggered.
    /// -----------------------------------------------------------------------------
    /// Subcase 3b: Successor y is Deeper in z's Right Subtree
    /// -----------------------------------------------------------------------------
    /// y must first be detached from its current position by promoting its right child x.
    /// Then y replaces z in the tree.
    /// ```text
    ///        Before Deletion                   After Deletion
    ///           z.parent                          z.parent
    ///              |                                 |
    ///             [z]                               (y)
    ///            /   \                             /   \
    ///          (z.L) (z.R)                        (z.L) (z.R)
    ///                /                                  /
    ///              ...          ==>                   ...
    ///              /                                  /
    ///            (y)                                (x)
    ///           /   \                              /   \
    ///         None  (x)                          x.L   x.R
    /// ```
    /// Tracking:      y = minimum(z.right), x = y.right, `y_original_color` = y.color
    /// Action:        1. transplant(y, x)  <-- Detach y, promote x to y's old spot
    ///                2. y.right = z.right; y.right.parent = y
    ///                3. transplant(z, y)  <-- Put y into z's original spot
    ///                4. y.left = z.left; y.left.parent = y
    ///                5. y.color = z.color
    ///                6. if `y_original_color` == Black `delete_fixup(x)`x) }
    /// Fixup Trigger: If y's original color was Black, x becomes Double Black.
    /// Why:           Just like Subcase 3a, x moves up into the position of a Black node (y)
    ///                that is leaving this subtree. Regardless of z's color, the paths
    ///                passing specifically through x lose exactly one Black node (y's
    ///                original color), triggering the Double Black state at x's position.
    /// Color Scenarios for Subcase 3b:
    /// * Scenario A: z is BLACK, y is BLACK
    ///   - Net impact on upper tree: y leaves its spot deep in the tree and takes z's
    ///     place at the top. Since both were Black, the upper path structural count drops
    ///     by 1 Black node overall.
    ///   - Net impact on x paths: Paths running through x lose y from their path chain.
    ///     They do NOT care about z, because x is located deep under z.R, completely separate
    ///     from z's left or upper context.
    ///     Result: -1 Black node (the loss of y). Fixup is triggered at x.
    /// * Scenario B: z is RED, y is BLACK
    ///   - Net impact on upper tree: y leaves its spot and takes z's place, turning RED.
    ///     Upper paths lose 0 Black nodes overall (a Red node became a Red node).
    ///   - Net impact on x paths: Paths through x lose Black node y from their local path
    ///     chain. The fact that z was Red and y turns Red at the top is completely invisible
    ///     to x's subtree context.
    ///     Result: -1 Black node (the loss of y). Fixup is triggered at x.
    /// * Scenario C: z is BLACK or RED, y is RED
    ///   - If y is originally Red, paths running through x do not rely on y for their Black
    ///     node count. When y is detached and promoted up to replace z, its removal leaves
    ///     the local Black count through x entirely intact.
    ///     Result: 0 Black node deficit. Fixup is NOT triggered.
    ///     =============================================================================
    pub(crate) fn delete_node(&mut self, z: NodeRef<K, V, S>) -> (K, V) {
        let original_node = z;
        let original_color = z.color();

        if z.left().is_none() {
            let x = z.right();
            let fixup_parent = z.parent();

            // Determine which side z is on relative to its parent
            let nil_side = if x.is_none() {
                fixup_parent.map(|p| {
                    if Some(z) == p.left() {
                        crate::node::NilSide::Left
                    } else {
                        crate::node::NilSide::Right
                    }
                })
            } else {
                None
            };

            self.transplant(z, x);

            if let Some(parent) = fixup_parent {
                P::augment(parent);
                P::augment_upstream(parent);
            }
            if original_color == Color::Black {
                self.delete_fixup(x, fixup_parent, nil_side);
            }
        } else if z.right().is_none() {
            let x = z.left();
            let fixup_parent = z.parent();

            let nil_side = None;

            self.transplant(z, x);

            if let Some(parent) = fixup_parent {
                P::augment(parent);
                P::augment_upstream(parent);
            }
            if original_color == Color::Black {
                self.delete_fixup(x, fixup_parent, nil_side);
            }
        } else if let Some(z_right) = z.right() {
            let y = z_right.leftmost();
            let y_original_color = y.color();
            let x = y.right();

            if y.parent() == Some(z) {
                // Case 3a: y is the direct right child of z
                self.transplant(z, Some(y));
                y.set_left(z.left());
                if let Some(z_left) = z.left() {
                    z_left.set_parent(Some(y));
                }
                y.set_color(z.color());

                P::augment(y);
                P::augment_upstream(y);
                if y_original_color == Color::Black {
                    // x is the right child of y, so if x is None, it's on the right side of y
                    let nil_side = if x.is_none() {
                        Some(crate::node::NilSide::Right)
                    } else {
                        None
                    };
                    self.delete_fixup(x, Some(y), nil_side);
                }
            } else {
                // Case 3b: y is not the direct right child of z
                let y_parent = y
                    .parent()
                    .expect("y must have a parent since it's not the direct child of z");

                // Determine which side x is on relative to y_parent
                // x will be on the left side of y_parent (since y was leftmost in that subtree)
                let nil_side = if x.is_none() {
                    Some(crate::node::NilSide::Left)
                } else {
                    None
                };

                // Replace y with its right child
                self.transplant(y, x);

                // Make z's right subtree y's right subtree
                y.set_right(z.right());
                if let Some(z_right_node) = z.right() {
                    z_right_node.set_parent(Some(y));
                }

                // Replace z with y
                self.transplant(z, Some(y));

                // Make z's left subtree y's left subtree
                y.set_left(z.left());
                if let Some(z_left) = z.left() {
                    z_left.set_parent(Some(y));
                }

                // Copy z's color to y
                y.set_color(z.color());

                // Augment from y_parent upward
                P::augment(y_parent);
                P::augment_upstream(y_parent);

                if y_original_color == Color::Black {
                    self.delete_fixup(x, Some(y_parent), nil_side);
                }
            }
        }

        let (key, value, _) = unsafe { self.node_allocator.dealloc_node(original_node) };
        (key, value)
    }

    /// =============================================================================
    /// RED-BLACK TREE DELETION FIXUP CASES
    /// =============================================================================
    /// Legenda:
    /// (p)   = Parent of the Double Black node (can be Red or Black)
    /// [x]   = Double Black node (or a None/Null leaf carrying the deficit)
    /// (w)   = Sibling of the Double Black node
    /// w.L/R = Left and Right children of the sibling node
    /// =============================================================================
    /// Context: The fixup loop runs while `x` is not the root and `x` is Black.
    /// The following diagrams assume `x` is the LEFT child of its parent `p`.
    /// (The RIGHT child scenarios are perfectly symmetric mirror images).
    ///
    /// Invariant Notice on Sibling Nullability:
    /// In a structurally valid Red-Black Tree, the sibling node `w` can NEVER be
    /// None/Null during a Double Black fixup. The path down through `x` is missing
    /// exactly one Black node relative to the rest of the tree. If `w` were None,
    /// the path through `w` would contain zero key-bearing Black nodes, implying
    /// the tree was already unbalanced prior to deletion. Therefore, `w` can be
    /// safely resolved using an unwrap or assert variant (e.g., `.expect(...)`).
    /// =============================================================================
    /// -----------------------------------------------------------------------------
    /// Fixup Case 1: Sibling w is RED
    /// -----------------------------------------------------------------------------
    /// Sibling w must have a Black parent p and Black children (w.left and w.right).
    /// ```text
    ///      Before Rotation & Recoloring            After Rotation & Recoloring
    ///                   (p)_B                                   (w)_B
    ///                  /     \                                 /     \
    ///                [x]     (w)_R             ==>           (p)_R   (w.R)_B
    ///                       /     \                         /     \
    ///                   (w.L)_B  (w.R)_B                  [x]    (w.L)_B
    ///                      ^                                        ^
    ///                  New sibling                              Old sibling
    /// ```
    /// Tracking:      w = p.right
    /// Action:        1. p.color = Red; w.color = Black
    ///                2. `left_rotate(p)`
    ///                3. w = p.right (Update w to point to the new sibling w.L)
    /// Outcome:       This case does not fix the Double Black status of x. Instead,
    ///                it transforms the tree structure so that x now has a BLACK
    ///                sibling (w.L), reducing the problem to Case 2, 3, or 4.
    /// -----------------------------------------------------------------------------
    /// Fixup Case 2: Sibling w is BLACK, and Both of w's Children are BLACK
    /// -----------------------------------------------------------------------------
    /// The sibling cannot take the Double Black layer, so we pull one Black layer
    /// off both x and w, passing it up to their parent p.
    /// ```text
    ///      Before Recoloring                             After Recoloring
    ///            (p)                                          (p) <-- Gains 1 Black
    ///           /   \                                        /   \
    ///         [x]   (w)_B                 ==>              (x)   (w)_R
    ///              /     \                                /   \
    ///           (w.L)_B  (w.R)_B                       x.L   x.R
    /// ```
    /// Tracking:      w = p.right
    /// Action:        1. w.color = Red
    ///                2. x = p (The parent inherits the extra Black layer)
    /// Outcome:       - If p was Red: Loop terminates. p becomes a regular Black node.
    ///                - If p was Black: p becomes the new Double Black node. The loop
    ///                  continues one level higher up the tree.
    /// -----------------------------------------------------------------------------
    /// Fixup Case 3: Sibling w is BLACK, w.left is RED, and w.right is BLACK
    /// -----------------------------------------------------------------------------
    /// This is a setup case to turn w's Red child into a right-child position.
    /// ```text
    ///      Before Rotation & Recoloring            After Rotation & Recoloring
    ///                   (p)                                     (p)
    ///                  /   \                                   /   \
    ///                [x]   (w)_B               ==>           [x]   (w.L)_B
    ///                     /     \                                     \
    ///                 (w.L)_R  (w.R)_B                                (w)_R
    ///                                                                    \
    ///                                                                  (w.R)_B
    ///                                                                     ^
    ///                                                                 New sibling
    /// ```
    /// Tracking:      w = p.right
    /// Action:        1. w.L.color = Black; w.color = Red
    ///                2. `right_rotate(w)`
    ///                3. w = p.right (Update w to point to the new sibling w.L)
    /// Outcome:       The tree configuration is transformed into Case 4, where the
    ///                sibling is Black and its right child is Red.
    /// -----------------------------------------------------------------------------
    /// Fixup Case 4: Sibling w is BLACK, and w.right is RED (w.left can be any color)
    /// -----------------------------------------------------------------------------
    /// This is the terminal case. A rotation absorbs and eliminates the Double Black layer.
    /// ```text
    ///      Before Rotation & Recoloring            After Rotation & Recoloring
    ///                   (p)_?                                   (w)_? (Takes p's color)
    ///                  /     \                                 /     \
    ///                [x]     (w)_B             ==>           (p)_B   (w.R)_B
    ///                       /     \                         /     \
    ///                     (?)    (w.R)_R                  (x)     (?)
    /// ```
    /// Tracking:      w = p.right
    /// Action:        1. w.color = p.color
    ///                2. p.color = Black; w.R.color = Black
    ///                3. `left_rotate(p)`
    ///                4. x = root (Forces the loop to terminate)
    /// Outcome:       The extra Black layer from x is successfully distributed.
    ///                The paths through x gain a Black node (p), while paths through
    ///                w maintain their count via w.R turning Black. The tree is
    ///                now perfectly balanced, and the fixup is complete.
    /// =============================================================================
    /// NIL NODE HANDLING AND ROOT COMPARISON
    /// =============================================================================
    /// When x is None (nil), the loop condition `x != self.root` correctly handles
    /// all edge cases:
    ///
    /// Case 1: Tree is empty after deletion (self.root is None)
    ///   - Comparison: None != None evaluates to false
    ///   - Result: Loop exits immediately (correct - no fixup needed for empty tree)
    ///
    /// Case 2: Tree has nodes, x is nil at a non-root position (self.root is Some(_))
    ///   - Comparison: None != Some(_) evaluates to true
    ///   - Result: Loop continues to fix the double-black nil node (correct)
    ///
    /// Case 3: x moves up to become Some(node) during fixup
    ///   - Comparison: `Some(x_node)` != `Some(root_node)` compares node references
    ///   - Result: Exits when x reaches root (correct - root can absorb extra black)
    ///
    /// The `nil_side` parameter tracks which child position the nil node occupies,
    /// enabling correct sibling identification when x is None.
    fn delete_fixup(
        &mut self,
        mut x: Option<NodeRef<K, V, S>>,
        mut x_parent: Option<NodeRef<K, V, S>>,
        mut nil_side: Option<crate::node::NilSide>,
    ) {
        while x != self.root && NodeRef::is_black(x) {
            let Some(parent) = x_parent else { break };
            // Determine which side x is on
            let x_is_left = if let Some(x_node) = x {
                Some(x_node) == parent.left()
            } else {
                nil_side == Some(crate::node::NilSide::Left)
            };

            if x_is_left {
                let mut w = parent
                    .right()
                    .expect("Sibling must exist for deletion fixup because x is not the root and is double black");
                if w.color() == Color::Red {
                    w.set_color(Color::Black);
                    parent.set_color(Color::Red);
                    self.left_rotate(parent);
                    w = parent
                        .right()
                        .expect("Sibling must exist for deletion fixup because x is not the root and is double black");
                }
                let w_left_black = NodeRef::is_black(w.left());
                let w_right_black = NodeRef::is_black(w.right());
                if w_left_black && w_right_black {
                    // Case 2: Sibling w is Black and both of w's children are Black
                    w.set_color(Color::Red);
                    x = Some(parent);
                    x_parent = parent.parent();
                    nil_side = None; // x is now Some, so we don't need to track nil_side
                } else {
                    if w_right_black {
                        // Case 3: Sibling w is Black, w's left child is Red, and w's right child is Black
                        if let Some(w_left) = w.left() {
                            w_left.set_color(Color::Black);
                        }
                        w.set_color(Color::Red);
                        self.right_rotate(w);
                        w = parent.right().expect(
                            "Sibling must exist for deletion fixup because x is not the root and is double black",
                        );
                    }
                    // Case 4: Sibling w is Black and w's right child is Red
                    w.set_color(parent.color());
                    parent.set_color(Color::Black);
                    if let Some(w_right) = w.right() {
                        w_right.set_color(Color::Black);
                    }
                    self.left_rotate(parent);
                    x = self.root;
                    x_parent = None;
                    nil_side = None; // Loop will exit
                }
            } else {
                let mut w = parent
                    .left()
                    .expect("Sibling must exist for deletion fixup because x is not the root and is double black");
                if w.color() == Color::Red {
                    w.set_color(Color::Black);
                    parent.set_color(Color::Red);
                    self.right_rotate(parent);
                    w = parent
                        .left()
                        .expect("Sibling must exist for deletion fixup because x is not the root and is double black");
                }
                let w_left_black = NodeRef::is_black(w.left());
                let w_right_black = NodeRef::is_black(w.right());
                if w_left_black && w_right_black {
                    // Case 2: Sibling w is Black and both of w's children are Black
                    w.set_color(Color::Red);
                    x = Some(parent);
                    x_parent = parent.parent();
                    nil_side = None; // x is now Some, so we don't need to track nil_side
                } else {
                    if w_left_black {
                        // Case 3: Sibling w is Black, w's right child is Red, and w's left child is Black
                        if let Some(w_right) = w.right() {
                            w_right.set_color(Color::Black);
                        }
                        w.set_color(Color::Red);
                        self.left_rotate(w);
                        w = parent.left().expect(
                            "Sibling must exist for deletion fixup because x is not the root and is double black",
                        );
                    }
                    // Case 4: Sibling w is Black and w's left child is Red (Now correctly un-nested)
                    w.set_color(parent.color());
                    parent.set_color(Color::Black);
                    if let Some(w_left) = w.left() {
                        w_left.set_color(Color::Black);
                    }
                    self.right_rotate(parent);
                    x = self.root;
                    x_parent = None;
                    nil_side = None; // Loop will exit
                }
            }
        }

        if let Some(x) = x {
            x.set_color(Color::Black);
        }
    }

    /// Deletes a node `z` from a Binary Search Tree following the CLRS algorithm.
    ///
    /// This implementation relies on the `TRANSPLANT` subroutine to physically
    /// replace one subtree with another. It maps to the 4 structural CLRS cases:
    ///
    /// ### Case A: `z` has no left child (CLRS 12.3 Line 1-2)
    /// Replace `z` by its right child `r` (handles both a single right child or a leaf).
    /// ```text
    ///        (z.p)                   (z.p)
    ///          |                       |
    ///         [z]        --->         [r]
    ///        /   \                   /   \
    ///      NIL   [r]               (a)   (b)
    /// ```
    ///
    /// ### Case B: `z` has a left child but no right child (CLRS 12.3 Line 3-4)
    /// Replace `z` by its left child `l`.
    /// ```text
    ///        (z.p)                   (z.p)
    ///          |                       |
    ///         [z]        --->         [l]
    ///        /   \                   /   \
    ///      [l]   NIL               (a)   (b)
    /// ```
    ///
    /// ### Case C: `z` has two children; successor `y` is the immediate right child (CLRS 12.3 Line 6)
    /// `y` replaces `z` directly. `z`'s left child becomes `y`'s left child.
    /// ```text
    ///         [z]                     [y]
    ///        /   \                   /   \
    ///      [l]   [y]     --->      [l]   [r]
    ///           /   \
    ///         NIL   [r]
    /// ```
    ///
    /// ### Case D: `z` has two children; successor `y` is deeper in the right subtree (CLRS 12.3 Line 7-12)
    /// `y`'s own right child `x` is transplanted to `y`'s position. `y` is then extracted,
    /// taking ownership of `z`'s original right child `r` before replacing `z` entirely.
    /// ```text
    ///         [z]                     [z]                     [y]
    ///        /   \                   /   \                   /   \
    ///      [l]   [r]               [l]   [y]               [l]   [r]
    ///           /                       /                       /
    ///         ...       --->          [x]       --->          ...
    ///         /                       /                       /
    ///       [y]                     ...                     [x]
    ///      /   \                                           /   \
    ///    NIL   [x]                                       (a)   (b)
    /// ```
    pub(crate) fn delete_node_no_fixup(&mut self, z: NodeRef<K, V, S>) -> (K, V) {
        if z.left().is_none() {
            self.transplant(z, z.right());
        } else if z.right().is_none() {
            self.transplant(z, z.left());
        } else {
            let z_right = z.right().expect("z.right should not be None here");
            let y = z_right.leftmost();

            if y != z_right {
                self.transplant(y, y.right());
                y.set_right(z.right());
                if let Some(right) = y.right() {
                    right.set_parent(Some(y));
                }
            }
            self.transplant(z, Some(y));
            y.set_left(z.left());
            if let Some(left) = y.left() {
                left.set_parent(Some(y));
            }
        }

        let (key, value, _) = unsafe { self.node_allocator.dealloc_node(z) };
        (key, value)
    }

    /// Verifies the red-black tree properties and returns true if they are satisfied.
    /// This is a recursive function that checks the properties of each node in the tree.
    /// Its use is for testing and debugging purposes.
    #[cfg_attr(coverage_nightly, coverage(off))]
    pub(crate) fn verify_properties(&self) -> bool
    where
        K: Ord,
    {
        if let Some(root) = self.root {
            if root.color() != Color::Black {
                return false; // Root must be black
            }
            let black_height = Self::verify_node_properties(root);
            black_height.is_some()
        } else {
            true // An empty tree is valid
        }
    }

    #[cfg_attr(coverage_nightly, coverage(off))]
    fn verify_node_properties(node: NodeRef<K, V, S>) -> Option<usize>
    where
        K: Ord,
    {
        // Verify left subtree
        let left_black_height = if let Some(left) = node.left() {
            Self::verify_node_properties(left)?
        } else {
            1 // Null leaves are considered black
        };

        // Verify right subtree
        let right_black_height = if let Some(right) = node.right() {
            Self::verify_node_properties(right)?
        } else {
            1 // Null leaves are considered black
        };

        // Check that both subtrees have the same black height
        if left_black_height != right_black_height {
            return None;
        }

        // Check red-black properties
        if node.color() == Color::Red {
            if node.left().is_some_and(|left| left.color() == Color::Red) {
                return None; // Red node cannot have red child
            }
            if node
                .right()
                .is_some_and(|right| right.color() == Color::Red)
            {
                return None; // Red node cannot have red child
            }
        }

        // verify chilld pointers are correct
        if let Some(left) = node.left() {
            if left.parent() != Some(node) {
                return None; // Left child's parent pointer is incorrect
            }
        }
        if let Some(right) = node.right() {
            if right.parent() != Some(node) {
                return None; // Right child's parent pointer is incorrect
            }
        }

        Some(left_black_height + usize::from(node.color() == Color::Black))
    }

    /// Verifies the augmentation of the tree and returns true if it is correct.
    /// This is a recursive function that checks the augmentation of each node in the tree.
    /// Its use is for testing and debugging purposes.
    #[cfg_attr(coverage_nightly, coverage(off))]
    pub(crate) fn verify_augmentation(&self) -> bool
    where
        K: Ord,
        S: PartialEq,
    {
        Self::verify_node_augmentation(self.root)
    }

    #[cfg_attr(coverage_nightly, coverage(off))]
    fn verify_node_augmentation(node: Option<NodeRef<K, V, S>>) -> bool
    where
        K: Ord,
        S: PartialEq,
    {
        let Some(current_node) = node else {
            return true;
        }; // Virtual Null leaves are always implicitly valid

        unsafe {
            let left_data = current_node.left().map(|l| (l.key(), l.value(), l.stats()));

            let right_data = current_node
                .right()
                .map(|r| (r.key(), r.value(), r.stats()));

            let computed_data = P::compute(
                current_node.key(),
                current_node.value(),
                left_data,
                right_data,
            );

            if computed_data != *current_node.stats() {
                return false;
            }
        }

        Self::verify_node_augmentation(current_node.left())
            && Self::verify_node_augmentation(current_node.right())
    }
}

impl<K, V, S, A: Allocator, P: TreePolicy<K = K, V = V, S = S>, Q: Ord + ?Sized>
    RangeBoundsLimits<K, V, S, Q> for AugmentedRBTreeLayout<K, V, S, A, P>
where
    K: Borrow<Q> + Ord,
{
    /// Finds the first node whose key is greater than or equal to `key` (lower bound).
    fn lower_bound(&self, key: &Q) -> Option<NodeRef<K, V, S>> {
        let mut current = self.root;
        let mut result = None;
        while let Some(node) = current {
            let node_key = unsafe { node.key() };
            match key.cmp(node_key.borrow()) {
                core::cmp::Ordering::Less => {
                    result = Some(node);
                    current = node.left();
                }
                core::cmp::Ordering::Equal => {
                    return Some(node);
                }
                core::cmp::Ordering::Greater => {
                    current = node.right();
                }
            }
        }
        result
    }

    /// Finds the last node whose key is less than or equal to `key` (upper bound).
    fn upper_bound(&self, key: &Q) -> Option<NodeRef<K, V, S>> {
        let mut current = self.root;
        let mut result = None;
        while let Some(node) = current {
            let node_key = unsafe { node.key() };
            match key.cmp(node_key.borrow()) {
                core::cmp::Ordering::Less => {
                    current = node.left();
                }
                core::cmp::Ordering::Equal => {
                    return Some(node);
                }
                core::cmp::Ordering::Greater => {
                    result = Some(node);
                    current = node.right();
                }
            }
        }
        result
    }

    fn leftmost(&self) -> Option<NodeRef<K, V, S>> {
        self.root.map(NodeRef::leftmost)
    }

    fn rightmost(&self) -> Option<NodeRef<K, V, S>> {
        self.root.map(NodeRef::rightmost)
    }
}
