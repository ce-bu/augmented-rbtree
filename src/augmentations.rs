//! Ready-to-use [`Augment`] implementations for common use cases.
//!
//! These types can be used directly as the `G` type parameter of
//! [`AugmentedRBTree`](crate::AugmentedRBTree) without implementing the trait yourself.
//!
//! # Examples
//!
//! ```
//! use augmented_rbtree::{AugmentedRBTree, augmentations::SubtreeSize};
//!
//! let mut tree = AugmentedRBTree::<i32, f64, SubtreeSize>::new();
//! tree.insert(1, 1.0);
//! tree.insert(2, 2.0);
//! tree.insert(3, 3.0);
//!
//! assert_eq!(tree.root_stats(), Some(&3));
//! ```

use core::{marker::PhantomData, ops::Add};

use crate::Augment;

// ============================================================================
// UnitAugmentation
// ============================================================================

/// Augmentation that does not store any additional data.
#[derive(Debug)]
pub struct Unit;

impl<K, V> Augment<K, V> for Unit {
    type Stats = ();

    fn compute(
        _key: &K,
        _value: &V,
        _left: Option<(&K, &V, &Self::Stats)>,
        _right: Option<(&K, &V, &Self::Stats)>,
    ) -> Self::Stats {
    }

    fn identity() -> Self::Stats {}
}

// ============================================================================
// SubtreeSize
// ============================================================================

/// Augmentation that tracks the number of nodes in each subtree.
///
/// This enables O(log n) rank queries (find the k-th smallest element) and
/// select operations when combined with custom traversal.
///
/// # Examples
///
/// ```
/// use augmented_rbtree::{AugmentedRBTree, augmentations::SubtreeSize};
///
/// let mut tree = AugmentedRBTree::<i32, &str, SubtreeSize>::new();
/// tree.insert(3, "c");
/// tree.insert(1, "a");
/// tree.insert(2, "b");
///
/// assert_eq!(tree.root_stats(), Some(&3));
/// assert_eq!(tree.len(), 3);
/// ```
#[derive(Debug, Default)]
pub struct SubtreeSize<S = usize>(PhantomData<S>);

impl<K, V, S> Augment<K, V> for SubtreeSize<S>
where
    S: Add<Output = S> + From<usize> + Copy,
{
    type Stats = S;

    #[inline]
    fn identity() -> S {
        S::from(0usize)
    }

    fn compute(_key: &K, _value: &V, left: Option<(&K, &V, &S)>, right: Option<(&K, &V, &S)>) -> S {
        S::from(1usize)
            + left.map_or(S::from(0usize), |(_, _, &c)| c)
            + right.map_or(S::from(0), |(_, _, &c)| c)
    }
}

// ============================================================================
// SumAugmentation
// ============================================================================

/// Augmentation that tracks the sum of all values in each subtree.
///
/// Useful for range-sum queries: after locating the range boundaries, access
/// the subtree statistics to get aggregate sums.
///
/// The value type `V` must implement [`core::ops::Add`], [`Copy`], and provide a
/// zero element via [`Default`].
///
/// # Examples
///
/// ```
/// use augmented_rbtree::{AugmentedRBTree, augmentations::SumAugmentation};
///
/// let mut tree = AugmentedRBTree::<i32, i64, SumAugmentation>::new();
/// tree.insert(1, 10);
/// tree.insert(2, 20);
/// tree.insert(3, 30);
///
/// // Root stats holds the sum of all values
/// assert_eq!(tree.root_stats(), Some(&60));
/// ```

#[derive(Debug)]
pub struct SumAugmentation;

impl<K, V> Augment<K, V> for SumAugmentation
where
    V: core::ops::Add<Output = V> + Copy + Default,
{
    type Stats = V;

    #[inline]
    fn identity() -> V {
        V::default()
    }

    fn compute(_key: &K, value: &V, left: Option<(&K, &V, &V)>, right: Option<(&K, &V, &V)>) -> V {
        let left_sum = left.map(|(_, _, &s)| s).unwrap_or_default();
        let right_sum = right.map(|(_, _, &s)| s).unwrap_or_default();
        left_sum + *value + right_sum
    }
}

// ============================================================================
// MaxAugmentation
// ============================================================================

/// Augmentation that tracks the maximum value in each subtree.
///
/// Useful for segment-tree-style range-max queries.
///
/// # Examples
///
/// ```
/// use augmented_rbtree::{AugmentedRBTree, augmentations::MaxAugmentation};
///
/// let mut tree = AugmentedRBTree::<i32, i32, MaxAugmentation>::new();
/// tree.insert(1, 5);
/// tree.insert(2, 12);
/// tree.insert(3, 3);
///
/// assert_eq!(tree.root_stats(), Some(&Some(12)));
/// ```
#[derive(Debug)]
pub struct MaxAugmentation;

impl<K, V> Augment<K, V> for MaxAugmentation
where
    V: Ord + Copy,
{
    type Stats = Option<V>;

    #[inline]
    fn identity() -> Option<V> {
        None
    }

    fn compute(
        _key: &K,
        value: &V,
        left: Option<(&K, &V, &Option<V>)>,
        right: Option<(&K, &V, &Option<V>)>,
    ) -> Option<V> {
        let mut max = *value;
        if let Some((_, _, Some(ls))) = left {
            if *ls > max {
                max = *ls;
            }
        }
        if let Some((_, _, Some(rs))) = right {
            if *rs > max {
                max = *rs;
            }
        }
        Some(max)
    }
}

// ============================================================================
// MinAugmentation
// ============================================================================

/// Augmentation that tracks the minimum value in each subtree.
///
/// # Examples
///
/// ```
/// use augmented_rbtree::{AugmentedRBTree, augmentations::MinAugmentation};
///
/// let mut tree = AugmentedRBTree::<i32, i32, MinAugmentation>::new();
/// tree.insert(1, 5);
/// tree.insert(2, 1);
/// tree.insert(3, 8);
///
/// assert_eq!(tree.root_stats(), Some(&Some(1)));
/// ```
#[derive(Debug)]
pub struct MinAugmentation;

impl<K, V> Augment<K, V> for MinAugmentation
where
    V: Ord + Copy,
{
    type Stats = Option<V>;

    #[inline]
    fn identity() -> Option<V> {
        None
    }

    fn compute(
        _key: &K,
        value: &V,
        left: Option<(&K, &V, &Option<V>)>,
        right: Option<(&K, &V, &Option<V>)>,
    ) -> Option<V> {
        let mut min = *value;
        if let Some((_, _, Some(ls))) = left {
            if *ls < min {
                min = *ls;
            }
        }
        if let Some((_, _, Some(rs))) = right {
            if *rs < min {
                min = *rs;
            }
        }
        Some(min)
    }
}

// ============================================================================
// IntervalMaxEnd
// ============================================================================

/// Augmentation for interval trees: tracks the maximum interval endpoint in each subtree.
///
/// When keys are interval start points and values are interval end points, this augmentation
/// allows efficient overlap queries: if `root_stats()` < `query_start`, no interval overlaps.
///
/// # Examples
///
/// ```
/// use augmented_rbtree::{AugmentedRBTree, augmentations::IntervalMaxEnd};
///
/// // Key = interval start, Value = interval end
/// let mut tree = AugmentedRBTree::<i32, i32, IntervalMaxEnd>::new();
/// tree.insert(1, 5);   // interval [1, 5]
/// tree.insert(3, 10);  // interval [3, 10]
/// tree.insert(8, 12);  // interval [8, 12]
///
/// // Max endpoint in the whole tree
/// assert_eq!(tree.root_stats(), Some(&Some(12)));
/// ```
#[derive(Debug)]
pub struct IntervalMaxEnd;

impl<K> Augment<K, K> for IntervalMaxEnd
where
    K: Ord + Copy,
{
    type Stats = Option<K>;

    #[inline]
    fn identity() -> Option<K> {
        None
    }

    fn compute(
        _key: &K,
        value: &K,
        left: Option<(&K, &K, &Option<K>)>,
        right: Option<(&K, &K, &Option<K>)>,
    ) -> Option<K> {
        let mut max_end = *value;
        if let Some((_, _, Some(ls))) = left {
            if *ls > max_end {
                max_end = *ls;
            }
        }
        if let Some((_, _, Some(rs))) = right {
            if *rs > max_end {
                max_end = *rs;
            }
        }
        Some(max_end)
    }
}

/// Dynamically generates a custom constant [`Augment`] type.
///
/// This eliminates the need to manage external traits or deal with
/// const-generic limitations when configuring a tree with a fixed baseline value.
///
/// # Examples
///
/// ```
/// # use augmented_rbtree::constant_augment;
/// constant_augment!(MyConstantAugment, i32, 42);
/// ```
#[macro_export]
macro_rules! constant_augment {
    ($name:ident, $type:ty, $val:expr) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name;

        impl $crate::Augment<$type, $type> for $name {
            type Stats = $type;

            #[inline]
            fn identity() -> Self::Stats {
                $val
            }

            #[inline]
            fn compute(
                _key: &$type,
                _value: &$type,
                _left: Option<(&$type, &$type, &Self::Stats)>,
                _right: Option<(&$type, &$type, &Self::Stats)>,
            ) -> Self::Stats {
                $val
            }
        }
    };
}
