use crate::alloc_proxy::proxy::Allocator;
use crate::augmented_rbtree::internal_details::free_subtree;
use crate::iterators::internal_details::ValMutInt;
use crate::layout::AugmentedRBTreeLayout;
use crate::node::internal_details::NodeRef;
use crate::policy::internal_details::TreePolicy;
use core::borrow::Borrow;
use core::marker::PhantomData;
use core::ops::{Bound, RangeBounds};

/// A guarded mutable reference to an augmented tree value.
///
/// Modifying the underlying value via this guard automatically
/// propagates augmentation updates up the tree when it goes out of scope.
pub type ValMut<'a, K, V, S, P> = ValMutInt<'a, K, V, S, P>;

/// A mutable iterator over the values of an augmented tree.
pub type ValuesMut<'a, K, V, S, P> =
    crate::iterators::internal_details::ValuesMutInt<'a, K, V, S, P>;

pub mod internal_details {
    use core::{
        marker::PhantomData,
        ops::{Deref, DerefMut},
    };

    use crate::{node::internal_details::NodeRef, policy::internal_details::TreePolicy};

    /// A mutable reference to a value in an `AugmentedRBTree`.
    ///
    /// When this reference is dropped, it triggers augmentation recalculation
    /// up the tree, ensuring the augmented data remains correct.
    #[derive(Debug)]
    pub struct ValMutInt<'a, K, V, S, P>
    where
        P: TreePolicy<K = K, V = V, S = S>,
    {
        node: NodeRef<K, V, S>,
        marker: PhantomData<(&'a mut (K, V, S), &'a P)>,
    }

    impl<K, V, S, P> ValMutInt<'_, K, V, S, P>
    where
        P: TreePolicy<K = K, V = V, S = S>,
    {
        pub(crate) fn new(node: NodeRef<K, V, S>) -> Self {
            Self {
                node,
                marker: PhantomData,
            }
        }
    }

    impl<K, V, S, P> AsMut<V> for ValMutInt<'_, K, V, S, P>
    where
        P: TreePolicy<K = K, V = V, S = S>,
    {
        fn as_mut(&mut self) -> &mut V {
            unsafe { self.node.value_mut() }
        }
    }

    impl<K, V, S, P> Deref for ValMutInt<'_, K, V, S, P>
    where
        P: TreePolicy<K = K, V = V, S = S>,
    {
        type Target = V;

        #[inline]
        fn deref(&self) -> &Self::Target {
            unsafe { self.node.value() }
        }
    }

    impl<K, V, S, P> DerefMut for ValMutInt<'_, K, V, S, P>
    where
        P: TreePolicy<K = K, V = V, S = S>,
    {
        #[inline]
        fn deref_mut(&mut self) -> &mut Self::Target {
            unsafe { self.node.value_mut() }
        }
    }

    impl<K, V, S, P> Drop for ValMutInt<'_, K, V, S, P>
    where
        P: TreePolicy<K = K, V = V, S = S>,
    {
        fn drop(&mut self) {
            P::augment(self.node);
            P::augment_upstream(self.node);
        }
    }

    /// A mutable iterator over the values of an `AugmentedRBTree`.
    ///
    /// This struct is created by the [`values_mut`](crate::AugmentedRBTreeInt::values_mut) method.
    #[derive(Debug)]
    pub struct ValuesMutInt<'a, K, V, S, P>
    where
        P: TreePolicy<K = K, V = V, S = S>,
    {
        inner: IterMut<'a, K, V, S, P>,
    }

    impl<'a, K, V, S, P> ValuesMutInt<'a, K, V, S, P>
    where
        P: TreePolicy<K = K, V = V, S = S>,
    {
        pub(crate) fn new(inner: IterMut<'a, K, V, S, P>) -> Self {
            Self { inner }
        }
    }

    impl<'a, K, V, S: 'a, P> Iterator for ValuesMutInt<'a, K, V, S, P>
    where
        P: TreePolicy<K = K, V = V, S = S>,
    {
        type Item = ValMutInt<'a, K, V, S, P>;

        fn next(&mut self) -> Option<Self::Item> {
            self.inner.next().map(|(_, v, _)| v)
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            self.inner.size_hint()
        }
    }

    impl<'a, K, V, S: 'a, P> DoubleEndedIterator for ValuesMutInt<'a, K, V, S, P>
    where
        P: TreePolicy<K = K, V = V, S = S>,
    {
        fn next_back(&mut self) -> Option<Self::Item> {
            self.inner.next_back().map(|(_, v, _)| v)
        }
    }

    impl<'a, K, V, S: 'a, P> ExactSizeIterator for ValuesMutInt<'a, K, V, S, P>
    where
        P: TreePolicy<K = K, V = V, S = S>,
    {
        fn len(&self) -> usize {
            self.inner.len()
        }
    }

    impl<'a, K, V, S: 'a, P> core::iter::FusedIterator for ValuesMutInt<'a, K, V, S, P> where
        P: TreePolicy<K = K, V = V, S = S>
    {
    }

    /// A mutable iterator over the entries of an `AugmentedRBTree`.
    ///
    /// This struct is created by the [`iter_mut`](crate::AugmentedRBTreeInt::iter_mut) method.
    #[derive(Debug)]
    pub struct IterMut<'a, K, V, S, P>
    where
        P: TreePolicy<K = K, V = V, S = S>,
    {
        next: Option<NodeRef<K, V, S>>,
        back: Option<NodeRef<K, V, S>>,
        len: usize,
        _marker: PhantomData<(&'a mut (K, V, S), &'a P)>,
    }

    impl<K, V, S, P> IterMut<'_, K, V, S, P>
    where
        P: TreePolicy<K = K, V = V, S = S>,
    {
        pub(crate) fn new(root: Option<NodeRef<K, V, S>>, len: usize) -> Self {
            let next = root.map(NodeRef::leftmost);
            let back = root.map(NodeRef::rightmost);
            Self {
                next,
                back,
                len,
                _marker: PhantomData,
            }
        }
    }

    impl<'a, K, V, S, P> Iterator for IterMut<'a, K, V, S, P>
    where
        P: TreePolicy<K = K, V = V, S = S>,
    {
        type Item = (&'a K, ValMutInt<'a, K, V, S, P>, &'a S);

        fn next(&mut self) -> Option<Self::Item> {
            if self.len == 0 {
                return None;
            }

            let node = self.next?;

            // Check if we've crossed paths with the back iterator
            if self.next == self.back {
                self.next = None;
                self.back = None;
            } else {
                self.next = node.next_node();
            }

            self.len -= 1;

            unsafe {
                let key_ref = node.key();
                let stats_ref = node.stats();
                let val_mut = ValMutInt::new(node);
                Some((key_ref, val_mut, stats_ref))
            }
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            (self.len, Some(self.len))
        }
    }

    impl<'a, K, V, S: 'a, P> DoubleEndedIterator for IterMut<'a, K, V, S, P>
    where
        P: TreePolicy<K = K, V = V, S = S>,
    {
        fn next_back(&mut self) -> Option<Self::Item> {
            if self.len == 0 {
                return None;
            }

            let node = self.back?;

            // Check if we've crossed paths with the front iterator
            if self.next == self.back {
                self.next = None;
                self.back = None;
            } else {
                self.back = node.prev_node();
            }

            self.len -= 1;

            unsafe {
                let key_ref = node.key();
                let stats_ref = node.stats();
                let val_mut = ValMutInt::new(node);
                Some((key_ref, val_mut, stats_ref))
            }
        }
    }

    impl<'a, K, V, S: 'a, P> ExactSizeIterator for IterMut<'a, K, V, S, P>
    where
        P: TreePolicy<K = K, V = V, S = S>,
    {
        fn len(&self) -> usize {
            self.len
        }
    }

    impl<'a, K, V, S: 'a, P> core::iter::FusedIterator for IterMut<'a, K, V, S, P> where
        P: TreePolicy<K = K, V = V, S = S>
    {
    }
}

/// An iterator over the entries of an `AugmentedRBTree`.
///
/// This struct is created by the `AugmentedRBTreeInt::iter` method.
#[derive(Debug)]
pub struct Iter<'a, K, V, S> {
    next: Option<NodeRef<K, V, S>>,
    back: Option<NodeRef<K, V, S>>,
    len: usize,
    _marker: PhantomData<&'a (K, V, S)>,
}

impl<K, V, S> Iter<'_, K, V, S> {
    pub(crate) fn new(root: Option<NodeRef<K, V, S>>, len: usize) -> Self {
        let next = root.map(NodeRef::leftmost);
        let back = root.map(NodeRef::rightmost);
        Self {
            next,
            back,
            len,
            _marker: PhantomData,
        }
    }
}

impl<'a, K, V, S> Iterator for Iter<'a, K, V, S> {
    type Item = (&'a K, &'a V, &'a S);

    fn next(&mut self) -> Option<Self::Item> {
        if self.len == 0 {
            return None;
        }

        let node = self.next?;

        // Check if we've crossed paths with the back iterator
        if self.next == self.back {
            self.next = None;
            self.back = None;
        } else {
            self.next = node.next_node();
        }

        self.len -= 1;

        unsafe { Some((node.key(), node.value(), node.stats())) }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a, K, V, S: 'a> DoubleEndedIterator for Iter<'a, K, V, S> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.len == 0 {
            return None;
        }

        let node = self.back?;

        // Check if we've crossed paths with the front iterator
        if self.next == self.back {
            self.next = None;
            self.back = None;
        } else {
            self.back = node.prev_node();
        }

        self.len -= 1;

        unsafe { Some((node.key(), node.value(), node.stats())) }
    }
}

impl<'a, K, V, S: 'a> ExactSizeIterator for Iter<'a, K, V, S> {
    fn len(&self) -> usize {
        self.len
    }
}

impl<'a, K, V, S: 'a> core::iter::FusedIterator for Iter<'a, K, V, S> {}

/// An owning iterator over the entries of an `AugmentedRBTree`.
///
/// This struct is created by the [`into_iter`](IntoIterator::into_iter) method
/// on [`AugmentedRBTreeInt`](crate::AugmentedRBTreeInt) (provided by the [`IntoIterator`] trait).
///
/// Nodes are removed from the tree as they are iterated over but the augmented data is not recalculated.
///
#[derive(Debug)]
pub struct IntoIter<K, V, S, A, P>
where
    P: TreePolicy<K = K, V = V, S = S>,
    A: Allocator,
{
    next: Option<NodeRef<K, V, S>>,
    back: Option<NodeRef<K, V, S>>,
    layout: AugmentedRBTreeLayout<K, V, S, A, P>,
    len: usize,
}

impl<K, V, S, A: Allocator, P> IntoIter<K, V, S, A, P>
where
    P: TreePolicy<K = K, V = V, S = S>,
    A: Allocator,
{
    pub(crate) fn new(layout: AugmentedRBTreeLayout<K, V, S, A, P>, len: usize) -> Self {
        let next = layout.root.map(NodeRef::leftmost);
        let back = layout.root.map(NodeRef::rightmost);
        Self {
            next,
            back,
            layout,
            len,
        }
    }
}

impl<K, V, S, A, P> Drop for IntoIter<K, V, S, A, P>
where
    P: TreePolicy<K = K, V = V, S = S>,
    A: Allocator,
{
    fn drop(&mut self) {
        if let Some(root) = self.layout.root.take() {
            unsafe { free_subtree(root, &self.layout.node_allocator.alloc) };
        }
    }
}

impl<K, V, S, A: Allocator, P> Iterator for IntoIter<K, V, S, A, P>
where
    P: TreePolicy<K = K, V = V, S = S>,
{
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        if self.len == 0 {
            return None;
        }

        let current = self
            .next
            .take()
            .expect("This node must exists because len > 0");

        let next_node = current.next_node();

        self.next = next_node;

        let (key, value) = self.layout.delete_node_no_fixup(current);

        self.len -= 1;

        Some((key, value))
    }
}

impl<K, V, S, A: Allocator, P> DoubleEndedIterator for IntoIter<K, V, S, A, P>
where
    P: TreePolicy<K = K, V = V, S = S>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.len == 0 {
            return None;
        }

        let current = self
            .back
            .take()
            .expect("This node must exists because len > 0");

        let back_node = current.prev_node();

        self.back = back_node;

        let (key, value) = self.layout.delete_node_no_fixup(current);

        self.len -= 1;

        Some((key, value))
    }
}

impl<K, V, S, A: Allocator, P: TreePolicy<K = K, V = V, S = S>> ExactSizeIterator
    for IntoIter<K, V, S, A, P>
{
    fn len(&self) -> usize {
        self.len
    }
}

impl<K, V, S, A: Allocator, P: TreePolicy<K = K, V = V, S = S>> core::iter::FusedIterator
    for IntoIter<K, V, S, A, P>
{
}

/// An iterator over the keys of an `AugmentedRBTree`.
///
/// This struct is created by the `AugmentedRBTreeInt::keys` method.
#[derive(Debug)]
pub struct Keys<'a, K, V, S> {
    inner: Iter<'a, K, V, S>,
}

impl<'a, K, V, S> Keys<'a, K, V, S> {
    pub(crate) fn new(inner: Iter<'a, K, V, S>) -> Self {
        Self { inner }
    }
}

impl<'a, K, V, S: 'a> Iterator for Keys<'a, K, V, S> {
    type Item = &'a K;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(k, _, _)| k)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, K, V, S: 'a> DoubleEndedIterator for Keys<'a, K, V, S> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(|(k, _, _)| k)
    }
}

impl<'a, K, V, S: 'a> ExactSizeIterator for Keys<'a, K, V, S> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<'a, K, V, S: 'a> core::iter::FusedIterator for Keys<'a, K, V, S> {}

/// An iterator over the values of an `AugmentedRBTree`.
///
/// This struct is created by the `AugmentedRBTreeInt::values` method.
#[derive(Debug)]
pub struct Values<'a, K, V, S> {
    inner: Iter<'a, K, V, S>,
}

impl<'a, K, V, S> Values<'a, K, V, S> {
    pub(crate) fn new(inner: Iter<'a, K, V, S>) -> Self {
        Self { inner }
    }
}

impl<'a, K, V, S: 'a> Iterator for Values<'a, K, V, S> {
    type Item = &'a V;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(_, v, _)| v)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, K, V, S: 'a> DoubleEndedIterator for Values<'a, K, V, S> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(|(_, v, _)| v)
    }
}

impl<'a, K, V, S: 'a> ExactSizeIterator for Values<'a, K, V, S> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<'a, K, V, S: 'a> core::iter::FusedIterator for Values<'a, K, V, S> {}

/// An iterator over the values of an `AugmentedRBTree`.
///
/// This struct is created by the [`values`](crate::AugmentedRBTreeInt::values) method.
#[derive(Debug)]
pub struct Stats<'a, K, V, S> {
    inner: Iter<'a, K, V, S>,
}

impl<'a, K, V, S> Stats<'a, K, V, S> {
    pub(crate) fn new(inner: Iter<'a, K, V, S>) -> Self {
        Self { inner }
    }
}

impl<'a, K, V, S: 'a> Iterator for Stats<'a, K, V, S> {
    type Item = &'a S;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(_, _, s)| s)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, K, V, S: 'a> DoubleEndedIterator for Stats<'a, K, V, S> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(|(_, _, s)| s)
    }
}

impl<'a, K, V, S: 'a> ExactSizeIterator for Stats<'a, K, V, S> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<'a, K, V, S: 'a> core::iter::FusedIterator for Stats<'a, K, V, S> {}

// ============================================================================
// Range iterators
// ============================================================================

/// An iterator over a sub-range of entries in an `AugmentedRBTree`.
///
/// This struct is created by the `AugmentedRBTreeInt::range` method.
#[derive(Debug)]
pub struct Range<'a, K, V, S> {
    front: Option<NodeRef<K, V, S>>,
    back: Option<NodeRef<K, V, S>>,
    exhausted: bool,
    _marker: PhantomData<&'a (K, V, S)>,
}

impl<'a, K, V, S> Range<'a, K, V, S>
where
    K: Ord,
{
    pub(crate) fn new<Q, R>(layout: &'a dyn RangeBoundsLimits<K, V, S, Q>, range: R) -> Self
    where
        K: Borrow<Q> + Ord,
        Q: Ord + ?Sized + 'a,
        R: RangeBounds<Q>,
    {
        let (front, back, exhausted) = range_bounds_to_nodes(layout, &range);
        Self {
            front,
            back,
            exhausted,
            _marker: PhantomData,
        }
    }
}

impl<'a, K, V, S: 'a> Iterator for Range<'a, K, V, S> {
    type Item = (&'a K, &'a V, &'a S);

    fn next(&mut self) -> Option<Self::Item> {
        if self.exhausted {
            return None;
        }
        let node = self.front?;
        if self.front == self.back {
            self.exhausted = true;
        } else {
            self.front = node.next_node();
        }
        unsafe { Some((node.key(), node.value(), node.stats())) }
    }
}

impl<'a, K, V, S: 'a> DoubleEndedIterator for Range<'a, K, V, S> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.exhausted {
            return None;
        }
        let node = self.back?;
        if self.front == self.back {
            self.exhausted = true;
        } else {
            self.back = node.prev_node();
        }
        unsafe { Some((node.key(), node.value(), node.stats())) }
    }
}

impl<'a, K, V, S: 'a> core::iter::FusedIterator for Range<'a, K, V, S> {}

/// A mutable iterator over a sub-range of entries in an `AugmentedRBTree`.
///
/// This struct is created by the `AugmentedRBTreeInt::range_mut` method.
#[derive(Debug)]
pub struct RangeMut<'a, K, V, S, P>
where
    P: TreePolicy<K = K, V = V, S = S>,
{
    front: Option<NodeRef<K, V, S>>,
    back: Option<NodeRef<K, V, S>>,
    exhausted: bool,
    _marker: PhantomData<(&'a mut (K, V, S), &'a P)>,
}

impl<'a, K, V, S, P> RangeMut<'a, K, V, S, P>
where
    K: Ord,
    P: TreePolicy<K = K, V = V, S = S>,
{
    pub(crate) fn new<Q, R>(layout: &'a dyn RangeBoundsLimits<K, V, S, Q>, range: R) -> Self
    where
        K: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
        R: RangeBounds<Q>,
    {
        let (front, back, exhausted) = range_bounds_to_nodes(layout, &range);

        Self {
            front,
            back,
            exhausted,
            _marker: PhantomData,
        }
    }
}

impl<'a, K, V, S, P> Iterator for RangeMut<'a, K, V, S, P>
where
    P: TreePolicy<K = K, V = V, S = S>,
{
    type Item = (&'a K, ValMutInt<'a, K, V, S, P>, &'a S);

    fn next(&mut self) -> Option<Self::Item> {
        if self.exhausted {
            return None;
        }
        let node = self.front?;
        if self.front == self.back {
            self.exhausted = true;
        } else {
            self.front = node.next_node();
        }
        unsafe {
            let key_ref = node.key();
            let stats_ref = node.stats();
            Some((key_ref, ValMutInt::new(node), stats_ref))
        }
    }
}

impl<'a, K, V, S: 'a, P> DoubleEndedIterator for RangeMut<'a, K, V, S, P>
where
    P: TreePolicy<K = K, V = V, S = S>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.exhausted {
            return None;
        }
        let node = self.back?;
        if self.front == self.back {
            self.exhausted = true;
        } else {
            self.back = node.prev_node();
        }
        unsafe {
            let key_ref = node.key();
            let stats_ref = node.stats();
            Some((key_ref, ValMutInt::new(node), stats_ref))
        }
    }
}

impl<'a, K, V, S: 'a, P> core::iter::FusedIterator for RangeMut<'a, K, V, S, P> where
    P: TreePolicy<K = K, V = V, S = S>
{
}

// Helper: resolves a RangeBounds into (front_node, back_node, exhausted)
type RangeBoundsResult<K, V, S> = (Option<NodeRef<K, V, S>>, Option<NodeRef<K, V, S>>, bool);

pub(crate) trait RangeBoundsLimits<K, V, S, Q: ?Sized> {
    fn lower_bound(&self, key: &Q) -> Option<NodeRef<K, V, S>>;
    fn upper_bound(&self, key: &Q) -> Option<NodeRef<K, V, S>>;
    fn leftmost(&self) -> Option<NodeRef<K, V, S>>;
    fn rightmost(&self) -> Option<NodeRef<K, V, S>>;
}

fn range_bounds_to_nodes<K, V, S, R, Q>(
    layout: &dyn RangeBoundsLimits<K, V, S, Q>,
    range: &R,
) -> RangeBoundsResult<K, V, S>
where
    R: RangeBounds<Q>,
    K: Borrow<Q> + Ord,
    Q: Ord + ?Sized,
{
    let front = match range.start_bound() {
        Bound::Included(k) => layout.lower_bound(k),
        Bound::Excluded(k) => {
            // First node strictly greater than k
            layout.lower_bound(k).and_then(|n| {
                if unsafe { n.key() }.borrow() == k {
                    n.next_node()
                } else {
                    Some(n)
                }
            })
        }
        Bound::Unbounded => layout.leftmost(),
    };

    let back = match range.end_bound() {
        Bound::Included(k) => layout.upper_bound(k),
        Bound::Excluded(k) => {
            // Last node strictly less than k
            layout.upper_bound(k).and_then(|n| {
                if unsafe { n.key() }.borrow() == k {
                    n.prev_node()
                } else {
                    Some(n)
                }
            })
        }
        Bound::Unbounded => layout.rightmost(),
    };

    // Check if the range is empty (front is past back)
    let exhausted = match (front, back) {
        (Some(f), Some(b)) => unsafe { f.key() > b.key() },
        (Some(_), None) | (None, _) => true,
    };

    (front, back, exhausted)
}
