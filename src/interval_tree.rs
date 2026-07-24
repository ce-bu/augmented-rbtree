//! A production-grade interval tree built on the augmented red-black tree.
//!
//! An interval tree stores closed intervals `[lo, hi]` and supports efficient
//! overlap and containment queries in O(log n) average / O(k log n) for k results.
//!
//! ## How it works
//!
//! Each node stores an interval `[lo, hi]`. The tree is ordered by `lo` (left endpoint).
//! Each subtree tracks the maximum `hi` value it contains. This augmentation makes it
//! possible to prune entire subtrees during overlap queries without visiting every node.
//!
//! ## Example
//!
//! ```
//! use augmented_rbtree::interval_tree::{Interval, IntervalTree};
//!
//! let mut tree = IntervalTree::new();
//! tree.insert(Interval::new(1, 5), "task A");
//! tree.insert(Interval::new(3, 8), "task B");
//! tree.insert(Interval::new(10, 15), "task C");
//!
//! // All intervals overlapping [4, 6]
//! let matches: Vec<_> = tree.query_overlap(&4, &6).collect();
//! assert_eq!(matches.len(), 2); // "task A" [1,5] and "task B" [3,8] overlap [4,6]
//!
//! // Check if any interval contains a point
//! assert!(tree.any_contains_point(&4));
//! assert!(!tree.any_contains_point(&9));
//! ```

use crate::{
    Augment, AugmentedRBTree,
    alloc_proxy::proxy::{Allocator, Global},
    augmented_rbtree::internal_details::NavCursorLocation,
    search::{InOrderIter, InOrderPruningPolicy},
};
use core::{borrow::Borrow, fmt, marker::PhantomData};

// ============================================================================
// Interval type
// ============================================================================

/// A closed interval `[lo, hi]` used as a key in [`IntervalTree`].
///
/// Intervals are ordered by their lower endpoint (`lo`). Two intervals with the
/// same `lo` are further ordered by their upper endpoint (`hi`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Interval<T> {
    /// Inclusive lower bound.
    pub lo: T,
    /// Inclusive upper bound.
    pub hi: T,
}

impl<T: Ord> Interval<T> {
    /// Creates a new interval `[lo, hi]`.
    ///
    /// # Panics
    ///
    /// Panics if `lo > hi`.
    #[must_use]
    pub fn new(lo: T, hi: T) -> Self {
        assert!(lo <= hi, "Interval requires lo <= hi");
        Self { lo, hi }
    }

    /// Returns `true` if this interval overlaps with `other`.
    ///
    /// Two intervals overlap if they share at least one point:
    /// `[a, b]` overlaps `[c, d]` iff `a <= d && c <= b`.
    #[must_use]
    pub fn overlaps(&self, other: &Self) -> bool {
        self.lo <= other.hi && other.lo <= self.hi
    }

    /// Returns `true` if this interval overlaps with `[lo, hi]`.
    #[must_use]
    pub fn overlaps_range(&self, lo: &T, hi: &T) -> bool {
        &self.lo <= hi && lo <= &self.hi
    }

    /// Returns `true` if this interval contains `point`.
    #[must_use]
    pub fn contains_point(&self, point: &T) -> bool {
        &self.lo <= point && point <= &self.hi
    }

    /// Returns the length of the interval as `hi - lo`.
    #[must_use]
    pub fn len(&self) -> T
    where
        T: core::ops::Sub<Output = T> + Copy,
    {
        self.hi - self.lo
    }

    /// Returns `true` if `lo == hi` (a degenerate point interval).
    #[must_use]
    pub fn is_point(&self) -> bool
    where
        T: PartialEq,
    {
        self.lo == self.hi
    }
}

impl<T: Ord> PartialOrd for Interval<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Ord> Ord for Interval<T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.lo.cmp(&other.lo).then_with(|| self.hi.cmp(&other.hi))
    }
}

impl<T: fmt::Display> fmt::Display for Interval<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}, {}]", self.lo, self.hi)
    }
}

// ============================================================================
// Augmentation: max endpoint per subtree
// ============================================================================
/// The augmentation tracks the maximum `hi` value in each subtree, enabling efficient
/// pruning during overlap queries.
#[derive(Debug, Clone, Copy)]
pub struct MaxHi<T>(core::marker::PhantomData<T>);

impl<T: Ord + Clone + Default, V> Augment<Interval<T>, V> for MaxHi<T> {
    /// Maximum `hi` value in this subtree. `None` only for the transient uninitialized state.
    type Stats = T;

    fn compute(
        key: &Interval<T>,
        _value: &V,
        left: Option<(&Interval<T>, &V, &Self::Stats)>,
        right: Option<(&Interval<T>, &V, &Self::Stats)>,
    ) -> Self::Stats {
        let mut max = key.hi.clone();
        if let Some((_, _, l_max)) = left {
            if l_max > &max {
                max = l_max.clone();
            }
        }
        if let Some((_, _, r_max)) = right {
            if r_max > &max {
                max = r_max.clone();
            }
        }
        max
    }
}

/// internal details
pub mod internal_details {
    use core::marker::PhantomData;

    /// An internal pruning strategy that governs interval intersection queries.
    #[derive(Debug)]
    pub struct IntervalOverlapPolicy<T, KBound> {
        pub(crate) lo: KBound,
        pub(crate) hi: KBound,
        pub(crate) _marker: PhantomData<T>,
    }
}

impl<T: Ord, KBound, V> InOrderPruningPolicy<Interval<T>, V, T>
    for internal_details::IntervalOverlapPolicy<T, KBound>
where
    KBound: Borrow<T>,
{
    #[inline]
    fn is_match(&self, key: &Interval<T>, _value: &V, _stats: &T) -> bool {
        key.lo <= *self.hi.borrow() && key.hi >= *self.lo.borrow()
    }

    #[inline]
    fn should_explore_left(
        &self,
        _left_key: &Interval<T>,
        _left_value: &V,
        left_max_hi: &T,
        _current_key: &Interval<T>,
    ) -> bool {
        *left_max_hi >= *self.lo.borrow()
    }

    #[inline]
    fn should_explore_right(
        &self,
        _right_key: &Interval<T>,
        _right_value: &V,
        right_max_hi: &T,
        current_key: &Interval<T>,
    ) -> bool {
        *right_max_hi >= *self.lo.borrow() && current_key.lo <= *self.hi.borrow()
    }
}

// ============================================================================
// IntervalTree
// ============================================================================

/// Am interval tree that supports O(log n) overlap queries.
///
/// Built on an augmented red-black tree where each subtree tracks the maximum
/// upper bound (`hi`) of all intervals it contains. This enables efficient
/// pruning during overlap queries.
///
/// # Type Parameters
///
/// - `T`: The endpoint type. Must be `Ord + Clone`.
/// - `V`: The value associated with each interval.
/// - `A`: Allocator (defaults to `Global`]).
///
/// # Examples
///
/// ```
/// use augmented_rbtree::interval_tree::{Interval, IntervalTree};
///
/// let mut tree = IntervalTree::new();
/// tree.insert(Interval::new(1, 5), "a");
/// tree.insert(Interval::new(3, 9), "b");
/// tree.insert(Interval::new(7, 10), "c");
///
/// let overlapping: Vec<_> = tree.query_overlap(4, 8).map(|(iv, v)| (*iv, *v)).collect();
/// assert_eq!(overlapping.len(), 3);
/// ```
pub struct IntervalTree<T: Ord + Clone + Default, V, A: Allocator = Global> {
    inner: AugmentedRBTree<Interval<T>, V, MaxHi<T>, A>,
}

impl<T: Ord + Clone + Default, V> IntervalTree<T, V> {
    /// Creates a new, empty interval tree.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: AugmentedRBTree::new(),
        }
    }
}

impl<T: Ord + Clone + Default, V> Default for IntervalTree<T, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Ord + Clone + Default, V, A: Allocator> IntervalTree<T, V, A> {
    /// Creates a new interval tree with the given allocator.
    #[must_use]
    pub fn new_in(alloc: A) -> Self {
        Self {
            inner: AugmentedRBTree::new_in(alloc),
        }
    }

    /// Returns a reference to the underlying augmented red-black tree.
    /// This can be used for advanced operations or visualization.
    #[must_use]
    pub fn inner_tree(&self) -> &AugmentedRBTree<Interval<T>, V, MaxHi<T>, A> {
        &self.inner
    }

    /// Inserts an interval-value pair.
    ///
    /// If the exact interval already exists, the old value is replaced and returned.
    pub fn insert(&mut self, interval: Interval<T>, value: V) -> Option<V> {
        self.inner.insert(interval, value)
    }

    /// Removes an interval from the tree, returning its value if it existed.
    pub fn remove(&mut self, interval: &Interval<T>) -> Option<V> {
        self.inner.remove(interval)
    }

    /// Returns a reference to the value associated with `interval`, if present.
    #[must_use]
    pub fn get(&self, interval: &Interval<T>) -> Option<&V> {
        self.inner.get(interval)
    }

    /// Returns `true` if the tree contains the exact interval.
    #[must_use]
    pub fn contains(&self, interval: &Interval<T>) -> bool {
        self.inner.contains_key(interval)
    }

    /// Returns the number of intervals in the tree.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if the tree is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns an iterator over all `(interval, value)` pairs in sorted order (by `lo`, then `hi`).
    pub fn iter(&self) -> impl Iterator<Item = (&Interval<T>, &V)> {
        self.inner.iter().map(|(k, v, _)| (k, v))
    }

    /// Returns an iterator over all intervals that **overlap** with `[lo, hi]`.
    ///
    /// Complexity: O(k log n) where k is the number of overlapping intervals.
    ///
    /// # Examples
    ///
    /// ```
    /// use augmented_rbtree::interval_tree::{Interval, IntervalTree};
    ///
    /// let mut tree = IntervalTree::new();
    /// tree.insert(Interval::new(1, 5), ());
    /// tree.insert(Interval::new(6, 10), ());
    /// tree.insert(Interval::new(3, 8), ());
    ///
    /// let overlapping: Vec<_> = tree.query_overlap(4, 7).collect();
    /// assert_eq!(overlapping.len(), 3); // [1,5], [3,8] and [6,10] all overlap [4,7]
    /// ```
    pub fn query_overlap<K>(&self, lo: K, hi: K) -> OverlapIter<'_, T, V, K>
    where
        K: Borrow<T>,
    {
        let cursor = self.inner.nav_cursor(NavCursorLocation::Root);
        let root_node = cursor.current;

        // The policy owns the reference `&Q`, which is perfectly fine
        // since the reference lives for the duration of the query.
        let policy = internal_details::IntervalOverlapPolicy {
            lo,
            hi,
            _marker: PhantomData,
        };

        InOrderIter::new(root_node, policy)
    }
    /// Returns an iterator over all intervals that **contain** the point `p`.
    ///
    /// An interval `[a, b]` contains `p` iff `a <= p <= b`.
    ///
    /// Complexity: O(k log n) where k is the number of matching intervals.
    pub fn query_point<K>(&self, point: K) -> impl Iterator<Item = (&Interval<T>, &V)>
    where
        K: Borrow<T> + Clone,
    {
        let lo = point.clone();
        let hi = point;
        self.query_overlap::<K>(lo, hi)
    }

    /// Returns `true` if any interval in the tree overlaps with `[lo, hi]`.
    ///
    /// Complexity: O(log n).
    #[must_use]
    pub fn any_overlaps<K>(&self, lo: K, hi: K) -> bool
    where
        K: Borrow<T>,
    {
        self.query_overlap(lo, hi).next().is_some()
    }

    /// Returns `true` if any interval contains the given point.
    ///
    /// Complexity: O(log n).
    #[must_use]
    pub fn any_contains_point<K>(&self, point: K) -> bool
    where
        K: Borrow<T> + Clone,
    {
        self.any_overlaps(point.clone(), point)
    }

    /// Returns the first overlapping interval with `[lo, hi]`, if any.
    ///
    /// When multiple intervals overlap, returns the one with the smallest `lo`.
    ///
    /// Complexity: O(log n).
    #[must_use]
    pub fn first_overlap<K>(&self, lo: K, hi: K) -> Option<(&'_ Interval<T>, &'_ V)>
    where
        K: Borrow<T>,
    {
        self.query_overlap(lo, hi).next()
    }
}

impl<T: Ord + Clone + Default + fmt::Debug, V: fmt::Debug> fmt::Debug for IntervalTree<T, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

// ============================================================================
// Iterator
// ============================================================================

/// Iterator over overlapping intervals. Created by [`IntervalTree::query_overlap`].
pub type OverlapIter<'a, T, V, K> =
    InOrderIter<'a, Interval<T>, V, T, internal_details::IntervalOverlapPolicy<T, K>>;

#[cfg(test)]
mod tests {
    #[test]
    fn test_interval_functions() {
        use super::Interval;

        let iv1 = Interval::new(1, 5);
        let iv2 = Interval::new(4, 8);
        let iv3 = Interval::new(6, 10);

        assert!(iv1.overlaps(&iv2));
        assert!(!iv1.overlaps(&iv3));
        assert!(iv2.overlaps(&iv3));

        assert!(iv1.contains_point(&3));
        assert!(!iv1.contains_point(&6));

        assert_eq!(iv1.len(), 4);
        assert!(!iv1.is_point());
    }
}
