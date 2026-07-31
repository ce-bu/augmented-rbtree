use core::{fmt::Debug, ops::Bound};

use crate::{
    Augment, AugmentedRBTreeInt,
    alloc_proxy::proxy::{AllocError, Allocator, Global},
    augmentations,
    layout::try_join_layout,
    policy::internal_details::{
        DefaultTraitPolicy, FullAugmentationStrategy, NullAugmentationStrategy, TreePolicy,
    },
};

/// An error type representing an out-of-memory condition when a tree tries to allocate a node.
pub struct OutOfMemoryError {
    pub(crate) error: AllocError,
    pub(crate) size: usize,
}

impl Debug for OutOfMemoryError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("OutOfMemoryError")
            .field("error", &self.error)
            .field("size", &self.size)
            .finish()
    }
}

impl OutOfMemoryError {
    pub(crate) fn new(size: usize, error: AllocError) -> Self {
        Self { error, size }
    }
}

impl core::fmt::Display for OutOfMemoryError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Out of memory when trying to allocate {} bytes: {:?}",
            self.size, self.error
        )
    }
}

impl core::error::Error for OutOfMemoryError {}

/// It is used to specify the location of the node in the tree where the cursor should be initially positioned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeLocation<Q> {
    /// Root of the tree
    Root,

    /// Position at a specific key
    At(Q),

    /// The cursor will be positioned at the leftmost (minimum) node in the tree.
    Leftmost,

    /// Rightmost (maximum) node in the tree.
    Rightmost,

    /// The cursor will be positioned at the first node whose key is greater than (or equal) to the given key depending on the bound type.
    LowerBound(Bound<Q>),

    /// The cursor will be positioned at the first node whose key is less than (or equal) to the given key depending on the bound type.
    UpperBound(Bound<Q>),
}

/// A Red-Black Tree that supports augmentation through the `Augment` trait.
/// This is the main type that users will interact with.
/// The `AugmentedRBTree` type is a wrapper around the internal [`AugmentedRBTreeInt`](internal_details::AugmentedRBTreeInt) type, which handles the actual tree operations and augmentation logic.
pub type AugmentedRBTree<K, V, G, A = Global> = internal_details::AugmentedRBTreeInt<
    K,
    V,
    <G as Augment<K, V>>::Stats,
    A,
    DefaultTraitPolicy<K, V, G, <G as Augment<K, V>>::Stats, A, FullAugmentationStrategy>,
>;

/// A standard Red-Black Tree without augmentation.
/// This is equivalent to `AugmentedRBTree<K, V, Unit>`.
pub type RBTree<K, V, A = Global> = internal_details::AugmentedRBTreeInt<
    K,
    V,
    (),
    A,
    DefaultTraitPolicy<K, V, augmentations::Unit, (), A, NullAugmentationStrategy>,
>;

/// A factory for creating `AugmentedRBTree` instances with default parameters.
#[derive(Debug)]
pub struct AugmentedRBTreeFactory<G> {
    _marker: core::marker::PhantomData<fn() -> G>,
}

impl<G> AugmentedRBTreeFactory<G> {
    /// Creates a tree using the `Global` allocator.
    ///
    /// # Examples
    ///
    /// ```
    /// # use augmented_rbtree::{AugmentedRBTreeFactory, augmentations::SubtreeSize};
    /// let mut tree = AugmentedRBTreeFactory::<SubtreeSize>::new_tree();
    /// tree.insert(1, 100);
    /// assert!(tree.contains_key(&1));
    /// ```
    /// Type inference for K and V works automatically here!
    #[must_use]
    pub fn new_tree<K, V>() -> crate::AugmentedRBTree<K, V, G, Global>
    where
        K: Ord,
        G: Augment<K, V>,
    {
        AugmentedRBTree::new_in(Global)
    }

    /// Creates a tree using a custom allocator.
    ///
    /// # Examples
    /// ```
    /// #![cfg_attr(feature = "nightly", feature(allocator_api))]
    /// use augmented_rbtree::{AugmentedRBTreeFactory, augmentations::SubtreeSize, Global};
    /// let mut tree = AugmentedRBTreeFactory::<SubtreeSize>::new_tree_in(Global);
    /// tree.insert(1, 100);
    /// assert!(tree.contains_key(&1));
    /// ```
    #[must_use]
    pub fn new_tree_in<K, V, A: Allocator>(alloc: A) -> crate::AugmentedRBTree<K, V, G, A>
    where
        K: Ord,
        G: Augment<K, V>,
    {
        AugmentedRBTree::new_in(alloc)
    }
}

#[doc(hidden)]
pub mod internal_details {
    use core::{
        borrow::Borrow,
        fmt::{self},
        marker::PhantomData,
        mem,
        ops::{Bound, RangeBounds},
    };

    use crate::{
        Entry, TreeLocation,
        alloc_proxy::proxy::{Allocator, Global, Layout, handle_alloc_error},
        augmented_rbtree::OutOfMemoryError,
        cursor::{NavCursor, NavCursorMut},
        iterators::{
            IntoIter, Iter, IterMut, Keys, NodeGuard, Range, RangeBoundsLimits, RangeMut, Stats,
            Values,
        },
        layout::AugmentedRBTreeLayout,
        node::{Color, Node, internal_details::NodeRef},
        node_allocator::NodeAllocator,
        policy::internal_details::TreePolicy,
    };

    /// A Red-Black Tree that supports augmentation through the `Augment` trait.
    pub struct AugmentedRBTreeInt<K, V, S, A, P>
    where
        P: TreePolicy<K = K, V = V, S = S>,
        A: Allocator,
    {
        pub(crate) layout: AugmentedRBTreeLayout<K, V, S, A, P>,
    }

    impl<K, V, S, P> AugmentedRBTreeInt<K, V, S, Global, P>
    where
        P: TreePolicy<K = K, V = V, S = S>,
    {
        /// Creates a new, empty `AugmentedRBTree` using the global allocator.
        #[inline]
        #[must_use]
        pub fn new() -> Self {
            Self {
                layout: AugmentedRBTreeLayout::<K, V, S, Global, P> {
                    root: None,
                    node_allocator: NodeAllocator::new(Global),
                    len: 0,
                    bh: 0,
                    _marker: PhantomData,
                },
            }
        }
    }

    impl<K, V, S, A, P> AugmentedRBTreeInt<K, V, S, A, P>
    where
        P: TreePolicy<K = K, V = V, S = S>,
        A: Allocator,
    {
        /// Inserts a key-value pair into the tree. If the key already exists, its value is updated.
        ///
        /// # Returns
        /// Returns `Some(old_value)` if the key was already present, or `None` if the key was newly inserted.
        ///
        /// # Examples
        ///
        /// ```
        /// # use augmented_rbtree::{AugmentedRBTree, augmentations::SubtreeSize};
        /// let mut tree = AugmentedRBTree::<String, i32, SubtreeSize>::new();
        /// tree.insert("hello".to_string(), 1);
        /// assert_eq!(tree.insert("hello".to_string(), 2), Some(1));
        /// assert_eq!(tree.insert("world".to_string(), 3), None);
        /// ```
        pub fn insert(&mut self, key: K, value: V) -> Option<V>
        where
            K: Ord,
        {
            self.try_insert(key, value)
                .unwrap_or_else(|_| handle_alloc_error(Layout::new::<Node<K, V, S>>()))
        }

        /// Try to insert a key with a value
        pub fn try_insert(&mut self, key: K, value: V) -> Result<Option<V>, OutOfMemoryError>
        where
            K: Ord,
        {
            self.layout.try_insert_node(key, value)
        }

        /// Returns the number of elements in the tree.
        pub fn len(&self) -> usize {
            self.layout.len
        }

        /// Returns a reference to the value associated with the given key, if it exists in the tree.
        ///
        /// The key may be any borrowed form of the tree's key type, but the ordering on the borrowed
        /// form *must* match the ordering on the key type.
        ///
        /// # Examples
        ///
        /// ```
        /// # use augmented_rbtree::{AugmentedRBTree, augmentations::SubtreeSize};
        /// let mut tree = AugmentedRBTree::<String, i32, SubtreeSize>::new();
        /// tree.insert("hello".to_string(), 1);
        /// assert_eq!(tree.get("hello"), Some(&1));
        /// assert_eq!(tree.get("world"), None);
        /// ```
        pub fn get<Q>(&self, key: &Q) -> Option<&V>
        where
            K: Borrow<Q> + Ord,
            Q: Ord + ?Sized,
        {
            self.layout
                .find_node(key)
                .map(|node| unsafe { node.value() })
        }

        /// Returns a mutable reference to the value associated with the given key, if it exists.
        ///
        /// The key may be any borrowed form of the tree's key type, but the ordering on the borrowed
        /// form *must* match the ordering on the key type.
        ///
        /// # Examples
        ///
        /// ```
        /// # use augmented_rbtree::{AugmentedRBTree, augmentations::SubtreeSize};
        /// let mut tree = AugmentedRBTree::<String, i32, SubtreeSize>::new();
        /// tree.insert("hello".to_string(), 1);
        /// if let Some(v) = tree.get_mut("hello") {
        ///     *v = 42;
        /// }
        /// assert_eq!(tree.get("hello"), Some(&42));
        /// ```
        pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
        where
            K: Borrow<Q> + Ord,
            Q: Ord + ?Sized,
        {
            self.layout
                .find_node(key)
                .map(|node| unsafe { node.value_mut() })
        }

        /// Returns `true` if the tree contains a value for the given key.
        ///
        /// The key may be any borrowed form of the tree's key type, but the ordering on the borrowed
        /// form *must* match the ordering on the key type.
        ///
        /// # Examples
        ///
        /// ```
        /// # use augmented_rbtree::{AugmentedRBTree, augmentations::SubtreeSize};
        /// let mut tree = AugmentedRBTree::<String, i32, SubtreeSize>::new();
        /// tree.insert("hello".to_string(), 1);
        /// assert!(tree.contains_key("hello"));
        /// assert!(!tree.contains_key("world"));
        /// ```
        pub fn contains_key<Q>(&self, key: &Q) -> bool
        where
            K: Borrow<Q> + Ord,
            Q: Ord + ?Sized,
        {
            self.layout.find_node(key).is_some()
        }

        /// Returns a reference to the key-value-stats tuple for the given key, if it exists.
        ///
        /// # Examples
        ///
        /// ```
        /// # use augmented_rbtree::{AugmentedRBTree, augmentations::SubtreeSize};
        /// let mut tree = AugmentedRBTree::<i32, &str, SubtreeSize>::new();
        /// tree.insert(1, "a");
        /// assert_eq!(tree.get_key_value_stats(&1), Some((&1, &"a", &1)));
        /// ```
        pub fn get_key_value_stats<Q>(&self, key: &Q) -> Option<(&K, &V, &S)>
        where
            K: Borrow<Q> + Ord,
            Q: Ord + ?Sized,
        {
            self.layout
                .find_node(key)
                .map(|node| unsafe { (node.key(), node.value(), node.stats()) })
        }

        /// Returns a reference to the key-value-stats tuple for the given key, if it exists.
        ///
        /// # Examples
        ///
        /// ```
        /// # use augmented_rbtree::{AugmentedRBTree, augmentations::SubtreeSize};
        /// let mut tree = AugmentedRBTree::<i32, &str, SubtreeSize>::new();
        /// tree.insert(1, "a");
        /// assert_eq!(tree.get_value_stats(&1), Some((&"a", &1)));
        /// ```
        pub fn get_value_stats<Q>(&self, key: &Q) -> Option<(&V, &S)>
        where
            K: Borrow<Q> + Ord,
            Q: Ord + ?Sized,
        {
            self.layout
                .find_node(key)
                .map(|node| unsafe { (node.value(), node.stats()) })
        }

        /// Returns a reference to the key-value tuple for the given key, if it exists.
        ///
        /// # Examples
        ///
        /// ```
        /// # use augmented_rbtree::{AugmentedRBTree, augmentations::SubtreeSize};
        /// let mut tree = AugmentedRBTree::<i32, &str, SubtreeSize>::new();
        /// tree.insert(1, "a");
        /// assert_eq!(tree.get_key_value(&1), Some((&1, &"a")));
        /// ```
        pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&K, &V)>
        where
            K: Borrow<Q> + Ord,
            Q: Ord + ?Sized,
        {
            self.layout
                .find_node(key)
                .map(|node| unsafe { (node.key(), node.value()) })
        }

        /// Removes the node with the given key from the tree, if it exists, and returns its value.
        /// If the key does not exist in the tree, returns `None`.
        ///
        /// The key may be any borrowed form of the tree's key type, but the ordering on the borrowed
        /// form *must* match the ordering on the key type.
        ///
        /// # Examples
        ///
        /// ```
        /// # use augmented_rbtree::{AugmentedRBTree, augmentations::SubtreeSize};
        /// let mut tree = AugmentedRBTree::<String, i32, SubtreeSize>::new();
        /// tree.insert("hello".to_string(), 1);
        /// assert_eq!(tree.remove("hello"), Some(1));
        /// assert_eq!(tree.remove("hello"), None);
        /// ```
        pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
        where
            K: Borrow<Q> + Ord,
            Q: Ord + ?Sized,
        {
            self.layout.find_node(key).map(|node| {
                let (_key, value) = self.layout.delete_node(node);
                value
            })
        }

        /// Removes and returns the key-value pair for the given key if it exists.
        ///
        /// # Examples
        ///
        /// ```
        /// # use augmented_rbtree::{AugmentedRBTree, augmentations::SubtreeSize};
        /// let mut tree = AugmentedRBTree::<i32, &str, SubtreeSize>::new();
        /// tree.insert(1, "a");
        /// assert_eq!(tree.remove_entry(&1), Some((1, "a")));
        /// assert_eq!(tree.remove_entry(&1), None);
        /// ```
        pub fn remove_entry<Q>(&mut self, key: &Q) -> Option<(K, V)>
        where
            K: Borrow<Q> + Ord,
            Q: Ord + ?Sized,
        {
            self.layout.find_node(key).map(|node| {
                let (k, v) = self.layout.delete_node(node);
                (k, v)
            })
        }

        /// Returns a reference to the first (minimum) key-value-stats entry in the tree.
        ///
        /// # Examples
        ///
        /// ```
        /// # use augmented_rbtree::{AugmentedRBTree, augmentations::SubtreeSize};
        /// let mut tree = AugmentedRBTree::<i32, &str, SubtreeSize>::new();
        /// tree.insert(3, "c");
        /// tree.insert(1, "a");
        /// tree.insert(2, "b");
        /// assert_eq!(tree.first_key_value_stats(), Some((&1, &"a", &1)));
        /// ```
        pub fn first_key_value_stats(&self) -> Option<(&K, &V, &S)>
        where
            K: Ord,
        {
            self.layout.root.map(|r| {
                let node = r.leftmost();
                unsafe { (node.key(), node.value(), node.stats()) }
            })
        }

        /// Returns a reference to the last (maximum) key-value-stats entry in the tree.
        ///
        /// # Examples
        ///
        /// ```
        /// # use augmented_rbtree::{AugmentedRBTree, augmentations::SubtreeSize};
        /// let mut tree = AugmentedRBTree::<i32, &str, SubtreeSize>::new();
        /// tree.insert(3, "c");
        /// tree.insert(1, "a");
        /// tree.insert(2, "b");
        /// assert_eq!(tree.last_key_value_stats(), Some((&3, &"c", &1)));
        /// ```
        pub fn last_key_value_stats(&self) -> Option<(&K, &V, &S)>
        where
            K: Ord,
        {
            self.layout.root.map(|r| {
                let node = r.rightmost();
                unsafe { (node.key(), node.value(), node.stats()) }
            })
        }

        /// Removes and returns the first (minimum) key-value pair from the tree.
        ///
        /// # Examples
        ///
        /// ```
        /// # use augmented_rbtree::{AugmentedRBTree, augmentations::SubtreeSize};
        /// let mut tree = AugmentedRBTree::<i32, &str, SubtreeSize>::new();
        /// tree.insert(3, "c");
        /// tree.insert(1, "a");
        /// tree.insert(2, "b");
        /// assert_eq!(tree.pop_first(), Some((1, "a")));
        /// assert_eq!(tree.len(), 2);
        /// ```
        pub fn pop_first(&mut self) -> Option<(K, V)>
        where
            K: Ord,
        {
            #[allow(clippy::redundant_closure_for_method_calls)]
            let node = self.layout.root.map(|r| r.leftmost())?;
            let (k, v) = self.layout.delete_node(node);
            Some((k, v))
        }

        /// Removes and returns the last (maximum) key-value pair from the tree.
        ///
        /// # Examples
        ///
        /// ```
        /// # use augmented_rbtree::{AugmentedRBTree, augmentations::SubtreeSize};
        /// let mut tree = AugmentedRBTree::<i32, &str, SubtreeSize>::new();
        /// tree.insert(3, "c");
        /// tree.insert(1, "a");
        /// tree.insert(2, "b");
        /// assert_eq!(tree.pop_last(), Some((3, "c")));
        /// assert_eq!(tree.len(), 2);
        /// ```
        pub fn pop_last(&mut self) -> Option<(K, V)>
        where
            K: Ord,
        {
            #[allow(clippy::redundant_closure_for_method_calls)]
            let node = self.layout.root.map(|r| r.rightmost())?;
            let (k, v) = self.layout.delete_node(node);
            Some((k, v))
        }

        /// Returns the augmentation data (stats) stored at the root, covering the entire tree.
        ///
        /// For augmentations like sum or count, this gives the aggregate result over all elements.
        /// Returns `None` if the tree is empty.
        ///
        /// # Examples
        ///
        /// ```
        /// # use augmented_rbtree::{AugmentedRBTree, Augment};
        /// # struct Sum;
        /// # impl Augment<i32, i32> for Sum {
        /// #     type Stats = i32;
        /// #     
        /// #     fn compute(k: &i32, v: &i32, l: Option<(&i32, &i32, &i32)>, r: Option<(&i32, &i32, &i32)>) -> i32 {
        /// #         v + l.map(|x| *x.2).unwrap_or(0) + r.map(|x| *x.2).unwrap_or(0)
        /// #     }
        /// # }
        /// let mut tree = AugmentedRBTree::<i32, i32, Sum>::new();
        /// tree.insert(1, 10);
        /// tree.insert(2, 20);
        /// tree.insert(3, 30);
        /// assert_eq!(tree.root_stats(), Some(&60));
        /// ```
        pub fn root_stats(&self) -> Option<&S> {
            self.layout.root.map(|r| unsafe { r.stats() })
        }

        /// Verify red-black tree structural invariants. Exposed for testing and debugging.
        #[doc(hidden)]
        pub fn verify_properties(&self) -> bool
        where
            K: Ord,
        {
            self.layout.verify_properties()
        }

        /// Verify augmentation correctness. Exposed for testing and debugging.
        #[doc(hidden)]
        pub fn verify_augmentation(&self) -> bool
        where
            K: Ord,
            S: PartialEq,
        {
            self.layout.verify_augmentation()
        }

        /// Returns `true` if the tree contains no elements.
        ///
        /// # Examples
        ///
        /// ```
        /// # use augmented_rbtree::{AugmentedRBTree, augmentations::SubtreeSize};
        /// let mut tree = AugmentedRBTree::<i32, &str, SubtreeSize>::new();
        /// assert!(tree.is_empty());
        /// tree.insert(1, "a");
        /// assert!(!tree.is_empty());
        /// ```
        pub fn is_empty(&self) -> bool {
            self.layout.len == 0
        }

        /// Clears the tree, removing all elements.
        ///
        /// # Examples
        ///
        /// ```
        /// # use augmented_rbtree::{AugmentedRBTree, augmentations::SubtreeSize};
        /// let mut tree = AugmentedRBTree::<i32, &str, SubtreeSize>::new();
        /// tree.insert(1, "a");
        /// tree.clear();
        /// assert!(tree.is_empty());
        /// ```
        pub fn clear(&mut self) {
            self.layout.clear();
        }

        /// Splits the tree into two separate trees based on a pivot `key`.
        ///
        /// Returns a tuple containing:
        /// 1. A tree with all keys strictly less than `key` (`keys < pivot`).
        /// 2. The value associated with the pivot `key`, if it existed in the tree.
        /// 3. A tree with all keys strictly greater than `key` (`keys > pivot`).
        ///
        /// # Algorithm (Tarjan / Sleator-Tarjan Path Split)
        /// 1. Descend the tree searching for the target `key`.
        /// 2. As the path diverges, subtrees branching off to the left and right are collected.
        /// 3. Upon returning or iteratively rebuilding, pieces are stitched back together using
        ///    the `try_join` algorithm, preserving the strict Red-Black tree properties.
        ///
        /// ```text
        ///          [Root]             Split at K
        ///          /    \                ===>        (Left Tree)      (Right Tree)
        ///       [Left]  [Right]                     All keys < K      All keys > K
        /// ```
        ///
        /// # Errors
        /// Returns [`OutOfMemoryError`] if re-joining structural fragments requires pivot
        /// allocations that fail due to memory exhaustion.
        pub fn try_split(self, _key: &K) -> Result<(Self, Option<V>, Self), OutOfMemoryError>
        where
            K: Ord,
        {
            // // Base case: If the tree is empty, splitting yields two empty trees
            // if self.root.is_none() {
            //     return Ok((Self::new(), None, Self::new()));
            // }

            // // We delegate the destructive pointer swapping to the internal layout
            // let (left_layout, match_val, right_layout) = self.layout.try_split_internal(key)?;

            // Ok((
            //     Self {
            //         layout: left_layout,
            //     },
            //     match_val,
            //     Self {
            //         layout: right_layout,
            //     },
            // ))
            todo!("Implement try_split for AugmentedRBTreeInt");
        }
    }

    impl<K, V, S, P> Default for AugmentedRBTreeInt<K, V, S, Global, P>
    where
        P: TreePolicy<K = K, V = V, S = S>,
    {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<K, V, S, A: Allocator, P> fmt::Debug for AugmentedRBTreeInt<K, V, S, A, P>
    where
        P: TreePolicy<K = K, V = V, S = S>,
        K: fmt::Debug,
        V: fmt::Debug,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_map()
                .entries(self.iter().map(|(k, v, _)| (k, v)))
                .finish()
        }
    }

    impl<K, V, S, A, P> PartialEq for AugmentedRBTreeInt<K, V, S, A, P>
    where
        P: TreePolicy<K = K, V = V, S = S>,
        K: PartialEq,
        V: PartialEq,
        A: Allocator,
    {
        fn eq(&self, other: &Self) -> bool {
            if self.len() != other.len() {
                return false;
            }
            self.iter()
                .zip(other.iter())
                .all(|((k1, v1, _), (k2, v2, _))| k1 == k2 && v1 == v2)
        }
    }

    impl<K, V, S, A, P> Eq for AugmentedRBTreeInt<K, V, S, A, P>
    where
        P: TreePolicy<K = K, V = V, S = S>,
        K: Eq,
        V: Eq,
        A: Allocator,
    {
    }

    impl<K, V, S, A: Allocator, P> AugmentedRBTreeInt<K, V, S, A, P>
    where
        P: TreePolicy<K = K, V = V, S = S>,
    {
        /// Creates a new, empty `AugmentedRBTree` with the specified allocator.
        #[inline]
        pub fn new_in(alloc: A) -> Self {
            Self {
                layout: AugmentedRBTreeLayout::new_in(alloc),
            }
        }

        /// Returns an iterator over the entries of the tree in sorted order by key.
        ///
        /// # Examples
        ///
        /// ```
        /// # use augmented_rbtree::{AugmentedRBTree, augmentations::Unit};
        /// let mut tree = AugmentedRBTree::<i32, &str, Unit>::new();
        /// tree.insert(2, "b");
        /// tree.insert(1, "a");
        /// tree.insert(3, "c");
        ///
        /// let entries: Vec<_> = tree.iter().collect();
        /// assert_eq!(entries, vec![(&1, &"a", &()), (&2, &"b", &()), (&3, &"c", &())]);
        /// ```
        pub fn iter(&self) -> Iter<'_, K, V, S> {
            Iter::new(self.layout.root, self.len())
        }

        /// Returns a mutable iterator over the entries of the tree in sorted order by key.
        ///
        /// # Examples
        ///
        /// ```
        /// # use augmented_rbtree::{AugmentedRBTree, Unit};
        /// let mut tree = AugmentedRBTree::<i32, i32, Unit>::new();
        /// tree.insert(1, 10);
        /// tree.insert(2, 20);
        ///
        /// for mut node_guard in tree.iter_mut() {
        ///     *node_guard.value_mut() *= 2;
        /// }
        ///
        /// assert_eq!(tree.get(&1), Some(&20));
        /// assert_eq!(tree.get(&2), Some(&40));
        /// ```
        pub fn iter_mut(&mut self) -> crate::iterators::IterMut<'_, K, V, S, P>
        where
            P: TreePolicy<K = K, V = V, S = S>,
        {
            IterMut::new(self.layout.root, self.len())
        }

        /// Returns an iterator over the keys of the tree in sorted order.
        ///
        /// # Examples
        ///
        /// ```
        /// # use augmented_rbtree::{AugmentedRBTree, Unit};
        /// let mut tree = AugmentedRBTree::<i32, &str, Unit>::new();
        /// tree.insert(2, "b");
        /// tree.insert(1, "a");
        /// tree.insert(3, "c");
        ///
        /// let keys: Vec<_> = tree.keys().collect();
        /// assert_eq!(keys, vec![&1, &2, &3]);
        /// ```
        pub fn keys(&self) -> Keys<'_, K, V, S> {
            Keys::new(self.iter())
        }

        /// Returns an iterator over the values of the tree in order by key.
        ///
        /// # Examples
        ///
        /// ```
        /// # use augmented_rbtree::{AugmentedRBTree, Unit};
        /// let mut tree = AugmentedRBTree::<i32, &str, Unit>::new();
        /// tree.insert(2, "b");
        /// tree.insert(1, "a");
        /// tree.insert(3, "c");
        ///
        /// let values: Vec<_> = tree.values().collect();
        /// assert_eq!(values, vec![&"a", &"b", &"c"]);
        /// ```
        pub fn values(&self) -> Values<'_, K, V, S> {
            Values::new(self.iter())
        }

        /// Returns a mutable iterator over the values of the tree in order by key.
        ///
        /// # Note
        ///
        /// Because this is an augmented tree, this iterator yields a smart guard [`NodeGuard`](crate::NodeGuard) rather than a raw reference. You must declare the loop variable as `mut`.
        ///
        ///  # Examples
        ///
        /// ```
        /// # use augmented_rbtree::{AugmentedRBTree, augmentations::SubtreeSize};
        /// let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new();
        /// tree.insert(1, 10);
        /// tree.insert(2, 20);
        ///
        /// for mut v in tree.values_mut() {
        ///     *v *= 2;
        /// }
        ///
        /// assert_eq!(tree.get(&1), Some(&20));
        /// assert_eq!(tree.get(&2), Some(&40));
        /// ```
        pub fn values_mut(&mut self) -> crate::iterators::ValuesMut<'_, K, V, S, P>
        where
            P: TreePolicy<K = K, V = V, S = S>,
        {
            crate::iterators::ValuesMut::new(self.layout.root, self.len())
        }

        /// Returns an iterator over the stats of the tree in order by key.
        ///
        /// # Examples
        ///
        /// ```
        /// # use augmented_rbtree::{AugmentedRBTree, augmentations::SumAugmentation};
        /// let mut tree = AugmentedRBTree::<i32, i32, SumAugmentation>::new();
        /// tree.insert(1, 1);
        /// tree.insert(2, 2);
        /// tree.insert(3, 3);
        ///
        /// let stats: Vec<_> = tree.stats().map(|x| *x).collect();
        /// assert_eq!(stats, vec![1, 6, 3]);
        /// ```
        pub fn stats(&self) -> Stats<'_, K, V, S>
        where
            P: TreePolicy<K = K, V = V, S = S>,
        {
            Stats::new(self.iter())
        }

        /// Returns an iterator over a sub-range of entries in the tree.
        ///
        /// Constructs a double-ended iterator over a sub-range of entries in the tree.
        /// The simplest way is to use the range syntax `min..max`, thus `range(min..max)` will
        /// yield elements from `min` (inclusive) to `max` (exclusive).
        /// The range may also be entered as `(Bound<T>, Bound<T>)`.
        ///
        /// # Panics
        ///
        /// Panics if the range start is greater than the range end, or if the range start equals the
        /// range end and both bounds are `Excluded`.
        ///
        /// # Examples
        ///
        /// ```
        /// # use augmented_rbtree::{AugmentedRBTree, augmentations::SubtreeSize};
        /// let mut tree = AugmentedRBTree::<i32, &str, SubtreeSize>::new();
        /// for (k, v) in [(1, "a"), (2, "b"), (3, "c"), (4, "d"), (5, "e")] {
        ///     tree.insert(k, v);
        /// }
        ///
        /// let range: Vec<_> = tree.range(2..=4).map(|(k, v, _)| (*k, *v)).collect();
        /// assert_eq!(range, vec![(2, "b"), (3, "c"), (4, "d")]);
        /// ```
        pub fn range<'a, Q, R>(&'a self, range: R) -> Range<'a, K, V, S>
        where
            K: Borrow<Q> + Ord,
            Q: Ord + ?Sized + 'a,
            R: RangeBounds<Q>,
        {
            Range::new(&self.layout, range)
        }

        /// Returns a mutable iterator over a sub-range of entries in the tree.
        ///
        /// # Examples
        ///
        /// ```
        /// # use augmented_rbtree::{AugmentedRBTree, augmentations::SubtreeSize};
        /// let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new();
        /// for i in 1..=5 { tree.insert(i, i * 10); }
        ///
        /// for mut node_guard in tree.range_mut(2..=4) {
        ///     *node_guard.value_mut() += 1;
        /// }
        ///
        /// assert_eq!(tree.get(&2), Some(&21));
        /// assert_eq!(tree.get(&3), Some(&31));
        /// assert_eq!(tree.get(&4), Some(&41));
        /// assert_eq!(tree.get(&1), Some(&10)); // untouched
        /// ```
        pub fn range_mut<'a, Q, R>(&'a mut self, range: R) -> RangeMut<'a, K, V, S, P>
        where
            K: Borrow<Q> + Ord,
            Q: Ord + ?Sized + 'a,
            R: RangeBounds<Q>,
        {
            RangeMut::new(&self.layout, range)
        }

        /// Gets the given key's corresponding entry in the tree for in-place manipulation.
        ///
        /// # Examples
        ///
        /// ```
        /// # use augmented_rbtree::{AugmentedRBTree, Entry, augmentations::SubtreeSize};
        /// let mut tree = AugmentedRBTree::<&str, u32, SubtreeSize>::new();
        ///
        /// for word in ["hello", "world", "hello", "rust"] {
        ///     let count = tree.entry(word).or_insert(0);
        ///     *count += 1;
        /// }
        ///
        /// assert_eq!(tree.get(&"hello"), Some(&2));
        /// assert_eq!(tree.get(&"world"), Some(&1));
        /// assert_eq!(tree.get(&"rust"), Some(&1));
        /// ```
        pub fn entry(&mut self, key: K) -> crate::entry::Entry<'_, K, V, S, A, P>
        where
            K: Ord,
        {
            Entry::new(self, key)
        }

        /// Resolves an abstract [`TreeLocation`] request into a physical node reference within the tree.
        ///
        /// This function serves as the central router for cursor initialization, mapping logical
        /// positioning variants cleanly onto the underlying traversal and boundary methods:
        ///
        /// | `TreeLocation` Request | Underlying Method | Target Condition |
        /// | :--- | :--- | :--- |
        /// | `Root` | `self.layout.root` | Returns raw root node if present |
        /// | `At(key)` | `find_node(key)` | Find node x for which x == key |
        /// | `LowerBound(Included(key))` | `lower_bound(key)` | Find the smallest node x for which x >= key |
        /// | `LowerBound(Excluded(key))` | `lower_bound_excluded(key)` | Find the smallest node x for which x > key |
        /// | `LowerBound(Unbounded)` | `leftmost()` | Find the smallest node x in the tree |
        /// | `UpperBound(Included(key))` | `floor(key)` | Find the largest node x for which x <= key |
        /// | `UpperBound(Excluded(key))` | `floor_excluded(key)` | Find the largest node x for which x < key |
        /// | `UpperBound(Unbounded)` | `rightmost()` | Find the largest node x in the tree |
        /// | `Leftmost` | `leftmost()` | Find the smallest node x in the tree |
        /// | `Rightmost` | `rightmost()` | Find the largest node x in the tree |
        pub(crate) fn get_tree_location<Q>(
            &self,
            location: TreeLocation<&Q>,
        ) -> Option<NodeRef<K, V, S>>
        where
            K: Borrow<Q> + Ord,
            Q: Ord,
        {
            match location {
                TreeLocation::Root => self.layout.root,
                TreeLocation::At(key) => self.layout.find_node(key),

                TreeLocation::LowerBound(bound) => match bound {
                    Bound::Included(key) => self.layout.lower_bound(key),
                    Bound::Excluded(key) => self.layout.lower_bound_excluded(key),
                    Bound::Unbounded => self.layout.leftmost(),
                },
                TreeLocation::UpperBound(bound) => match bound {
                    Bound::Included(key) => self.layout.floor(key),
                    Bound::Excluded(key) => self.layout.floor_excluded(key),
                    Bound::Unbounded => self.layout.rightmost(),
                },
                TreeLocation::Leftmost => self.layout.leftmost(),
                TreeLocation::Rightmost => self.layout.rightmost(),
            }
        }

        /// Visits each node in the tree and invokes the provided callback function with the current node's key, color, and its children's keys (if they exist).
        pub fn visit_topology<F>(&self, mut visitor: F)
        where
            F: FnMut(&K, Color, Option<&K>, Option<&K>),
        {
            self.visit_nodes(|node, left, right| {
                let key = unsafe { node.key() };
                let color = node.color();

                let left_key = if let Some(l) = left {
                    let l_ptr = l.ptr.as_ptr();
                    unsafe { Some(&(*l_ptr).key) }
                } else {
                    None
                };

                let right_key = if let Some(r) = right {
                    let r_ptr = r.ptr.as_ptr();
                    unsafe { Some(&(*r_ptr).key) }
                } else {
                    None
                };
                visitor(key, color, left_key, right_key);
            });
        }

        fn visit_nodes<F>(&self, mut visitor: F)
        where
            F: FnMut(NodeRef<K, V, S>, Option<NodeRef<K, V, S>>, Option<NodeRef<K, V, S>>),
        {
            let mut current = self.layout.root;
            let mut prev = None;

            while let Some(current_ref) = current {
                if prev == current_ref.parent() {
                    // Coming down from parent
                    let left = current_ref.left();
                    if let Some(left_ref) = left {
                        // try to visit left child
                        prev = Some(current_ref);
                        current = Some(left_ref);
                        continue;
                    }
                    // Visit current node
                    visitor(current_ref, current_ref.left(), current_ref.right());
                    if let Some(right_ref) = current_ref.right() {
                        // try to visit right child
                        prev = Some(current_ref);
                        current = Some(right_ref);
                    } else {
                        // nothing to be doen we need to go up
                        prev = Some(current_ref);
                        current = current_ref.parent();
                    }
                } else if prev == current_ref.left() {
                    // Coming up from left child
                    // Visit current node
                    visitor(current_ref, current_ref.left(), current_ref.right());
                    if let Some(right_ref) = current_ref.right() {
                        // try to visit right child
                        prev = Some(current_ref);
                        current = Some(right_ref);
                    } else {
                        // nothing to be doen we need to go up
                        prev = Some(current_ref);
                        current = current_ref.parent();
                    }
                } else if prev == current_ref.right() {
                    // Coming up from right child
                    prev = Some(current_ref);
                    current = current_ref.parent();
                }
            }
        }

        /// Attempts to clone the entire tree, returning a new tree with the same structure and values.
        pub fn try_clone(&self) -> Result<Self, OutOfMemoryError>
        where
            K: Clone,
            V: Clone,
            A: Allocator + Clone,
        {
            let clone_root = self.layout.try_clone()?;
            let node_allocator = self.layout.node_allocator.clone();
            Ok(Self {
                layout: AugmentedRBTreeLayout {
                    root: clone_root,
                    node_allocator,
                    len: self.len(),
                    bh: self.black_height(),
                    _marker: PhantomData,
                },
            })
        }

        /// Initializes an immutable navigation cursor positioned at the specified location within the tree.
        ///
        /// The cursor's starting node is determined dynamically based on the requested variant:
        ///
        /// | `TreeLocation` Request | Target Condition |
        /// | :--- | :--- |
        /// | `Root` | Position at the root node of the tree |
        /// | `At(key)` | Find node x for which x == key |
        /// | `LowerBound(Included(key))` | Find the smallest node x for which x >= key |
        /// | `LowerBound(Excluded(key))` | Find the smallest node x for which x > key |
        /// | `LowerBound(Unbounded)` | Find the smallest node x in the tree |
        /// | `UpperBound(Included(key))` | Find the largest node x for which x <= key |
        /// | `UpperBound(Excluded(key))` | Find the largest node x for which x < key |
        /// | `UpperBound(Unbounded)` | Find the largest node x in the tree |
        /// | `Leftmost` | Find the smallest node x in the tree |
        /// | `Rightmost` | Find the largest node x in the tree |
        pub fn nav_cursor<Q>(&self, location: TreeLocation<&Q>) -> NavCursor<'_, K, V, S>
        where
            K: Borrow<Q> + Ord,
            Q: Ord,
        {
            let node = self.get_tree_location(location);
            NavCursor::new(node)
        }

        /// Initializes a mutable navigation cursor positioned at the specified location within the tree.
        ///
        /// This cursor allows safely mutating node values or removing the current node from the tree.
        /// The initial position rules are identical to the immutable variant:
        ///
        /// | `TreeLocation` Request | Target Condition |
        /// | :--- | :--- |
        /// | `Root` | Position at the root node of the tree |
        /// | `At(key)` | Find node x for which x == key |
        /// | `LowerBound(Included(key))` | Find the smallest node x for which x >= key |
        /// | `LowerBound(Excluded(key))` | Find the smallest node x for which x > key |
        /// | `LowerBound(Unbounded)` | Find the smallest node x in the tree |
        /// | `UpperBound(Included(key))` | Find the largest node x for which x <= key |
        /// | `UpperBound(Excluded(key))` | Find the largest node x for which x < key |
        /// | `UpperBound(Unbounded)` | Find the largest node x in the tree |
        /// | `Leftmost` | Find the smallest node x in the tree |
        /// | `Rightmost` | Find the largest node x in the tree |
        pub fn nav_cursor_mut<Q>(
            &mut self,
            location: TreeLocation<&Q>,
        ) -> NavCursorMut<'_, K, V, S, A, P>
        where
            K: Borrow<Q> + Ord,
            Q: Ord,
        {
            let node = self.get_tree_location(location);
            NavCursorMut::new(&mut self.layout, node)
        }

        /// The black height of a node x is the number of black nodes on the path from x to a leaf, not counting x itself if it is black.
        /// Leaft are considered black.
        /// An empty tree has a black height of 0, a tree with only a black root has a black height of 1, and so on.
        pub fn black_height(&self) -> usize {
            self.layout.bh
        }
    }

    impl<K, V, S, A: Allocator, P: TreePolicy<K = K, V = V, S = S>> IntoIterator
        for AugmentedRBTreeInt<K, V, S, A, P>
    {
        type Item = (K, V);
        type IntoIter = IntoIter<K, V, S, A, P>;

        /// Consumes the tree and returns an iterator over its entries in sorted order by key.
        ///
        /// # Examples
        ///
        /// ```
        /// # use augmented_rbtree::{AugmentedRBTree, augmentations::Unit};
        /// let mut tree = AugmentedRBTree::<i32, &str, Unit>::new();
        /// tree.insert(2, "b");
        /// tree.insert(1, "a");
        /// tree.insert(3, "c");
        ///
        /// let entries: Vec<_> = tree.into_iter().collect();
        /// assert_eq!(entries, vec![(1, "a"), (2, "b"), (3, "c")]);
        /// ```
        fn into_iter(self) -> Self::IntoIter {
            let layout = unsafe { core::ptr::read(&raw const self.layout) };
            // do not run the destructor for self, since we are taking ownership of the allocator and root
            mem::forget(self);
            IntoIter::new(layout)
        }
    }

    impl<'a, K, V, S, A: Allocator, P: TreePolicy<K = K, V = V, S = S>> IntoIterator
        for &'a AugmentedRBTreeInt<K, V, S, A, P>
    where
        P: TreePolicy<K = K, V = V, S = S>,
    {
        type Item = (&'a K, &'a V, &'a S);
        type IntoIter = Iter<'a, K, V, S>;

        fn into_iter(self) -> Self::IntoIter {
            self.iter()
        }
    }

    impl<'a, K, V, S, A: Allocator, P: TreePolicy<K = K, V = V, S = S>> IntoIterator
        for &'a mut AugmentedRBTreeInt<K, V, S, A, P>
    {
        type Item = NodeGuard<'a, K, V, S, P>;
        type IntoIter = IterMut<'a, K, V, S, P>;

        fn into_iter(self) -> Self::IntoIter {
            self.iter_mut()
        }
    }

    impl<K, V, S, A, P> FromIterator<(K, V)> for AugmentedRBTreeInt<K, V, S, A, P>
    where
        K: Ord,
        A: Allocator + Default,
        P: TreePolicy<K = K, V = V, S = S>,
    {
        fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
            let mut tree = Self::new_in(A::default());
            for (k, v) in iter {
                tree.insert(k, v);
            }
            tree
        }
    }

    impl<K, V, S, A: Allocator, P: TreePolicy<K = K, V = V, S = S>> Extend<(K, V)>
        for AugmentedRBTreeInt<K, V, S, A, P>
    where
        K: Ord,
    {
        fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
            for (k, v) in iter {
                self.insert(k, v);
            }
        }
    }

    impl<K, V, S, A, P> Clone for AugmentedRBTreeInt<K, V, S, A, P>
    where
        K: Clone,
        V: Clone,
        A: Allocator + Clone,
        P: TreePolicy<K = K, V = V, S = S>,
    {
        fn clone(&self) -> Self {
            self.try_clone()
                .expect("Failed to clone AugmentedRBTree due to memory allocation failure")
        }
    }
}

/// Joins two collections (`self` and `other`) into a single ordered tree using a pivot `key` and `value`.
///
/// # Preconditions (Strict Key Ordering)
/// For the tree invariants to remain valid, all keys in `self` must be strictly smaller than the pivot `key`,
/// and the pivot `key` must be strictly smaller than all keys in `other`:
///
/// ```text
/// Max(self.keys) < key < Min(other.keys)
/// ```
///
/// # Complexity
/// * **Time:** `O(abs(self.black_height - other.black_height) + 1)` amortised. It scales with the height difference,
///   making it significantly faster than inserting elements one by one.
/// * **Space:** `O(1)` auxiliary space, requiring exactly one node allocation for the pivot.
///
/// # Errors
/// Returns [`OutOfMemoryError`] if the system fails to allocate memory for the new balancing pivot node.
/// If an error occurs, ownership of the input trees is consumed.
pub fn try_join<K, V, S, A, P>(
    left: AugmentedRBTreeInt<K, V, S, A, P>,
    right: AugmentedRBTreeInt<K, V, S, A, P>,
    key: K,
    value: V,
) -> Result<AugmentedRBTreeInt<K, V, S, A, P>, OutOfMemoryError>
where
    K: Ord,
    A: Allocator,
    P: TreePolicy<K = K, V = V, S = S>,
{
    try_join_layout(left.layout, right.layout, key, value)
        .map(|layout| AugmentedRBTreeInt { layout })
}

#[cfg(test)]
mod test {

    use alloc::string::String;

    use super::*;
    use crate::augmentations::Unit;
    #[test]
    fn covariance() {
        fn assert_covariance<'a, 'b: 'a>(
            x: AugmentedRBTree<&'b str, i32, Unit>,
        ) -> AugmentedRBTree<&'a str, i32, Unit> {
            x
        }
        let p = AugmentedRBTree::<&'static str, i32, Unit>::new();
        let _q = assert_covariance(p);
    }

    #[test]
    fn test_strict_trait_exclusions() {
        type YesX = AugmentedRBTree<String, i32, Unit>;

        static_assertions::assert_impl_all!(YesX: Send, Sync);

        type NoX = AugmentedRBTree<*const u8, i32, Unit>;
        type NoY = AugmentedRBTree<i32, *const u8, Unit>;

        #[cfg(not(under_rust_analyzer))]
        {
            static_assertions::assert_not_impl_all!(NoX: Send, Sync);
            static_assertions::assert_not_impl_all!(NoY: Send, Sync);
        }
    }
}
