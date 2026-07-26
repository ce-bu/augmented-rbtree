use core::marker::PhantomData;

use crate::{
    NodeGuard, alloc_proxy::proxy::Allocator, layout::AugmentedRBTreeLayout,
    node::internal_details::NodeRef, policy::internal_details::TreePolicy,
};

/// A graph-navigational cursor that sits directly on a tree node, exposing raw
/// topography traversal and augmented subtree metrics.
///
/// Unlike a standard, linear cursor that tracks abstract gaps between elements,
/// the `NavCursor` provides power-users with low-level access to the tree's actual
/// graph edges (`left`, `right`, and `parent`) along with the node's sorted
/// sequence links (`next`, `prev`).
///
/// # The State Model: Advance-and-Yield
///
/// All mutation methods (`next`, `prev`, `left`, `right`, `parent`) follow a
/// **look-before-you-leap** sequence:
/// 1. They move the internal cursor pointer to the target destination node first.
/// 2. If the destination exists, they update the state and yield its data.
/// 3. If the edge points to an empty child or terminal bound, the internal state
///    becomes `None`, and the method yields `None`.
///
/// ⚠️ **Crucial Warning:** Because movement methods advance the cursor *before*
/// reading, calling a movement method immediately after initialization will **skip**
/// the data of the node you started on. Always check [`.get()`](#method.get) first
/// if you need to process the initial search node.
///
///
#[derive(Debug, Copy)]
pub struct NavCursor<'a, K, V, S> {
    pub(crate) current: Option<NodeRef<K, V, S>>,
    _marker: PhantomData<&'a ()>,
}

impl<K, V, S> Clone for NavCursor<'_, K, V, S> {
    fn clone(&self) -> Self {
        Self {
            current: self.current,
            _marker: PhantomData,
        }
    }
}

impl<'a, K, V, S> NavCursor<'a, K, V, S> {
    pub(crate) fn new(current: Option<NodeRef<K, V, S>>) -> Self {
        Self {
            current,
            _marker: PhantomData,
        }
    }

    /// Returns a reference to the current node's key, value, and stats.
    #[must_use]
    pub fn get(&self) -> Option<(&'a K, &'a V, &'a S)> {
        let node = self.current?;
        unsafe { Some((node.key(), node.value(), node.stats())) }
    }

    /// Returns a reference to the next node's key, value, and stats without moving the cursor.
    #[must_use]
    pub fn peek_next(&self) -> Option<(&'a K, &'a V, &'a S)> {
        let next = self.current?.next_node()?;
        unsafe { Some((next.key(), next.value(), next.stats())) }
    }

    /// Returns a reference to the previous node's key, value, and stats without moving the cursor.
    #[must_use]
    pub fn peek_prev(&self) -> Option<(&'a K, &'a V, &'a S)> {
        let prev = self.current?.prev_node()?;
        unsafe { Some((prev.key(), prev.value(), prev.stats())) }
    }

    /// Returns a reference to the parent node's key, value, and stats without moving the cursor.
    #[must_use]
    pub fn peek_parent(&self) -> Option<(&'a K, &'a V, &'a S)> {
        let parent = self.current?.parent()?;
        unsafe { Some((parent.key(), parent.value(), parent.stats())) }
    }

    /// Returns a reference to the left child node's key, value, and stats without moving the cursor.
    #[must_use]
    pub fn peek_left(&self) -> Option<(&'a K, &'a V, &'a S)> {
        let left = self.current?.left()?;
        unsafe { Some((left.key(), left.value(), left.stats())) }
    }

    /// Returns a reference to the right child node's key, value, and stats without moving the cursor.
    #[must_use]
    pub fn peek_right(&self) -> Option<(&'a K, &'a V, &'a S)> {
        let right = self.current?.right()?;
        unsafe { Some((right.key(), right.value(), right.stats())) }
    }

    /// Moves the cursor to the next node in sorted order and returns its key, value, and stats.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<(&'a K, &'a V, &'a S)> {
        self.current = self.current?.next_node();
        let current = self.current?;
        unsafe { Some((current.key(), current.value(), current.stats())) }
    }

    /// Moves the cursor to the previous node in sorted order and returns its key, value, and stats.
    pub fn prev(&mut self) -> Option<(&'a K, &'a V, &'a S)> {
        self.current = self.current?.prev_node();
        let current = self.current?;
        unsafe { Some((current.key(), current.value(), current.stats())) }
    }

    /// Moves the cursor to the parent node and returns its key, value, and stats.
    pub fn parent(&mut self) -> Option<(&'a K, &'a V, &'a S)> {
        self.current = self.current?.parent();
        let current = self.current?;
        unsafe { Some((current.key(), current.value(), current.stats())) }
    }

    /// Moves the cursor to the left child node and returns its key, value, and stats.
    pub fn left(&mut self) -> Option<(&'a K, &'a V, &'a S)> {
        self.current = self.current?.left();
        let current = self.current?;
        unsafe { Some((current.key(), current.value(), current.stats())) }
    }

    /// Moves the cursor to the right child node and returns its key, value, and stats.
    pub fn right(&mut self) -> Option<(&'a K, &'a V, &'a S)> {
        self.current = self.current?.right();
        let current = self.current?;
        unsafe { Some((current.key(), current.value(), current.stats())) }
    }
}

/// A mutable navigation cursor for an augmented Red-Black tree layout.
///
/// `NavCursorMut` provides a stateful, bidirectionally navigable handle over the tree nodes.
/// It uniquely allows for **read-only key access**, **read-only tree statistics access**,
/// and **mutable value adjustments** via a specialized internal RAII guard ([`crate::NodeGuard`]).
///
/// Because values can be modified mutably through this cursor, any mutations that alter
/// secondary tree properties will trigger the augmentation.
///
/// # Lifetime Architecture
/// * `'a` - Binds the exclusive mutable borrow of the underlying tree structural layout.
///   This ensures that the tree cannot be mutated or invalidated by other access vectors while
///   the cursor is actively operating.
///
#[derive(Debug)]
pub struct NavCursorMut<'a, K, V, S, A, P>
where
    P: TreePolicy<K = K, V = V, S = S>,
    A: Allocator,
{
    layout: &'a mut AugmentedRBTreeLayout<K, V, S, A, P>,
    current: Option<NodeRef<K, V, S>>,
    _marker: PhantomData<(&'a mut (K, V, S), P)>,
}

impl<'a, K, V, S, A, P> NavCursorMut<'a, K, V, S, A, P>
where
    P: TreePolicy<K = K, V = V, S = S>,
    A: Allocator,
{
    /// Constructs a new mutable navigation cursor rooted or positioned at a targeted node.
    #[inline]
    pub(crate) fn new(
        layout: &'a mut AugmentedRBTreeLayout<K, V, S, A, P>,
        current: Option<NodeRef<K, V, S>>,
    ) -> Self {
        Self {
            layout,
            current,
            _marker: PhantomData,
        }
    }

    /// Returns a tuple containing an immutable reference to the key, a mutable value guard,
    /// and an immutable reference to the statistics of the **current** node.
    ///
    /// Returns `None` if the cursor is invalid or exhausted.
    ///
    pub fn get(&mut self) -> Option<NodeGuard<'_, K, V, S, P>> {
        let node = self.current?;
        let guard = NodeGuard::new(node);
        Some(guard)
    }

    /// Peeks forward to the next in-order node's data without moving the cursor's position.
    ///
    /// Returns `None` if there is no subsequent in-order node.
    pub fn peek_next(&mut self) -> Option<NodeGuard<'_, K, V, S, P>> {
        let next = self.current?.next_node()?;
        let guard = NodeGuard::new(next);
        Some(guard)
    }

    /// Peeks backward to the previous in-order node's data without moving the cursor's position.
    ///
    /// Returns `None` if there is no prior in-order node.
    pub fn peek_prev(&mut self) -> Option<NodeGuard<'_, K, V, S, P>> {
        let prev = self.current?.prev_node()?;
        let guard = NodeGuard::new(prev);
        Some(guard)
    }

    /// Peeks upward to the parent node's data without moving the cursor's position.
    ///
    /// Returns `None` if the cursor is at the root of the tree.
    pub fn peek_parent(&mut self) -> Option<NodeGuard<'_, K, V, S, P>> {
        let parent = self.current?.parent()?;
        let guard = NodeGuard::new(parent);
        Some(guard)
    }

    /// Peeks downward to the left child node's data without moving the cursor's position.
    ///
    /// Returns `None` if there is no left child.
    pub fn peek_left(&mut self) -> Option<NodeGuard<'_, K, V, S, P>> {
        let left = self.current?.left()?;
        let guard = NodeGuard::new(left);
        Some(guard)
    }

    /// Peeks downward to the right child node's data without moving the cursor's position.
    ///
    /// Returns `None` if there is no right child.
    pub fn peek_right(&mut self) -> Option<NodeGuard<'_, K, V, S, P>> {
        let right = self.current?.right()?;
        let guard = NodeGuard::new(right);
        Some(guard)
    }

    /// Moves the cursor to the next sequential node in-order and returns its data components.
    ///
    /// If no subsequent node exists, the cursor is advanced to an empty (`None`) state.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<NodeGuard<'_, K, V, S, P>> {
        self.current = self.current?.next_node();
        let current = self.current?;
        let guard = NodeGuard::new(current);
        Some(guard)
    }

    /// Moves the cursor to the previous sequential node in-order and returns its data components.
    ///
    /// If no prior node exists, the cursor is advanced to an empty (`None`) state.
    pub fn prev(&mut self) -> Option<NodeGuard<'_, K, V, S, P>> {
        self.current = self.current?.prev_node();
        let current = self.current?;
        let guard = NodeGuard::new(current);
        Some(guard)
    }

    /// Moves the cursor up to the parent node and returns its data components.
    ///
    /// Returns `None` and leaves the cursor unchanged if no parent exists.
    pub fn parent(&mut self) -> Option<NodeGuard<'_, K, V, S, P>> {
        self.current = self.current?.parent();
        let current = self.current?;
        let guard = NodeGuard::new(current);
        Some(guard)
    }

    /// Moves the cursor down into the left child node and returns its data components.
    ///
    /// Returns `None` and leaves the cursor unchanged if no left child exists.
    pub fn left(&mut self) -> Option<NodeGuard<'_, K, V, S, P>> {
        self.current = self.current?.left();
        let current = self.current?;
        let guard = NodeGuard::new(current);
        Some(guard)
    }

    /// Moves the cursor down into the right child node and returns its data components.
    ///
    /// Returns `None` and leaves the cursor unchanged if no right child exists.
    pub fn right(&mut self) -> Option<NodeGuard<'_, K, V, S, P>> {
        self.current = self.current?.right();
        let current = self.current?;
        let guard = NodeGuard::new(current);
        Some(guard)
    }

    /// Removes the node currently pointed to by the cursor from the tree structure,
    /// returning its owned key and value coordinates.
    ///
    /// This operation triggers full internal Red-Black tree rebalancing and augmented stat
    /// updates along the tree lineage.
    ///
    /// Returns `None` if the cursor is already empty or invalid.
    ///
    pub fn remove(&mut self) -> Option<(K, V)> {
        let node = self.current?;
        let next_node = node.next_node();
        self.current = next_node;
        Some(self.layout.delete_node(node))
    }
}
