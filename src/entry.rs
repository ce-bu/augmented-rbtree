//! The [`Entry`] API for in-place manipulation of tree entries.
//!
//! This mirrors the `Entry` API found in [`alloc::collections::BTreeMap`].

use crate::alloc_proxy::proxy::{Allocator, Layout, handle_alloc_error};
use crate::policy::internal_details::TreePolicy;
use crate::{
    augmented_rbtree::{OutOfMemoryError, internal_details::AugmentedRBTreeInt},
    node::Node,
};
use core::ptr::NonNull;

/// A view into a single entry in a tree, which may either be vacant or occupied.
///
/// This `enum` is constructed from the `AugmentedRBTreeInt::entry` method on
/// [`AugmentedRBTree`](crate::AugmentedRBTree).
#[derive(Debug)]
pub enum Entry<'a, K, V, S, A: Allocator, P: TreePolicy<K = K, V = V, S = S>>
where
    K: Ord,
{
    /// An occupied entry.
    Occupied(OccupiedEntry<'a, K, V, S, A, P>),
    /// A vacant entry.
    Vacant(VacantEntry<'a, K, V, S, A, P>),
}

/// A view into an occupied entry in an `AugmentedRBTree`.
///
/// The entry holds a raw pointer to the existing node, so no key clone is needed.
#[derive(Debug)]
pub struct OccupiedEntry<'a, K, V, S, A, P>
where
    K: Ord,
    P: TreePolicy<K = K, V = V, S = S>,
    A: Allocator,
{
    tree: &'a mut AugmentedRBTreeInt<K, V, S, A, P>,
    // Raw pointer to the existing node — valid for 'a because the tree is mutably borrowed.
    node: NonNull<Node<K, V, S>>,
}

/// A view into a vacant entry in an `AugmentedRBTree`.
#[derive(Debug)]
pub struct VacantEntry<'a, K, V, S, A, P>
where
    P: TreePolicy<K = K, V = V, S = S>,
    K: Ord,
    A: Allocator,
{
    tree: &'a mut AugmentedRBTreeInt<K, V, S, A, P>,
    key: K,
}

impl<'a, K, V, S, A: Allocator, P: TreePolicy<K = K, V = V, S = S>> Entry<'a, K, V, S, A, P>
where
    K: Ord,
{
    pub(crate) fn new(tree: &'a mut AugmentedRBTreeInt<K, V, S, A, P>, key: K) -> Self {
        // Safety: the tree is mutably borrowed for 'a, so the node pointer is stable.
        if let Some(node) = tree.layout.find_node(&key) {
            // Key exists — store the node pointer, discard the key (no clone needed)
            drop(key);
            Entry::Occupied(OccupiedEntry {
                tree,
                node: node.ptr,
            })
        } else {
            Entry::Vacant(VacantEntry { tree, key })
        }
    }

    /// Ensures a value is in the entry by inserting the default if empty, and returns
    /// a mutable reference to the value in the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// # use augmented_rbtree::{AugmentedRBTree, augmentations::SubtreeSize};
    /// let mut tree = AugmentedRBTree::<&str, u32, SubtreeSize>::new();
    /// tree.entry("hello").or_insert(3);
    /// assert_eq!(tree.get(&"hello"), Some(&3));
    ///
    /// *tree.entry("hello").or_insert(10) += 2;
    /// assert_eq!(tree.get(&"hello"), Some(&5));
    /// ```
    pub fn or_insert(self, default: V) -> &'a mut V {
        match self {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => e.insert(default),
        }
    }

    /// Ensures a value is in the entry by inserting the result of the default function if empty,
    /// and returns a mutable reference to the value in the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// # use augmented_rbtree::{AugmentedRBTree, augmentations::SubtreeSize};
    /// let mut tree = AugmentedRBTree::<&str, Vec<i32>, SubtreeSize>::new();
    /// tree.entry("hello").or_insert_with(Vec::new).push(1);
    /// assert_eq!(tree.get(&"hello"), Some(&vec![1]));
    /// ```
    pub fn or_insert_with(self, default: impl FnOnce() -> V) -> &'a mut V {
        match self {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => e.insert(default()),
        }
    }

    /// Ensures a value is in the entry by inserting the default value if empty, and returns a
    /// mutable reference to the value in the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// # use augmented_rbtree::{AugmentedRBTree, augmentations::SubtreeSize};
    /// let mut tree = AugmentedRBTree::<&str, u32, SubtreeSize>::new();
    /// tree.entry("hello").or_default();
    /// assert_eq!(tree.get(&"hello"), Some(&0));
    /// ```
    pub fn or_default(self) -> &'a mut V
    where
        V: Default,
    {
        self.or_insert_with(V::default)
    }

    /// Returns a reference to this entry's key.
    ///
    /// # Examples
    ///
    /// ```
    /// # use augmented_rbtree::{AugmentedRBTree, Unit};
    /// let mut tree = AugmentedRBTree::<&str, u32, Unit>::new();
    /// assert_eq!(tree.entry("hello").key(), &"hello");
    /// ```
    pub fn key(&self) -> &K {
        match self {
            Entry::Occupied(e) => e.key(),
            Entry::Vacant(e) => &e.key,
        }
    }

    /// Provides in-place mutable access to an occupied entry before any potential inserts.
    ///
    /// # Examples
    ///
    /// ```
    /// # use augmented_rbtree::{AugmentedRBTree, Unit};
    /// let mut tree = AugmentedRBTree::<&str, u32, Unit>::new();
    /// tree.entry("hello").and_modify(|e| *e += 1).or_insert(42);
    /// assert_eq!(tree.get(&"hello"), Some(&42));
    ///
    /// tree.entry("hello").and_modify(|e| *e += 1).or_insert(42);
    /// assert_eq!(tree.get(&"hello"), Some(&43));
    /// ```
    #[must_use]
    pub fn and_modify(self, f: impl FnOnce(&mut V)) -> Self {
        match self {
            Entry::Occupied(mut e) => {
                f(e.get_mut());
                Entry::Occupied(e)
            }
            Entry::Vacant(e) => Entry::Vacant(e),
        }
    }
}

impl<'a, K, V, S, A, P> OccupiedEntry<'a, K, V, S, A, P>
where
    K: Ord,
    P: TreePolicy<K = K, V = V, S = S>,
    A: Allocator,
{
    /// Returns a reference to the key of this entry.
    #[must_use]
    pub fn key(&self) -> &K {
        // Safety: node is valid for 'a (tree is mutably borrowed).
        unsafe { &(*self.node.as_ptr()).key }
    }

    /// Gets a reference to the value in the entry.
    #[must_use]
    pub fn get(&self) -> &V {
        unsafe { &(*self.node.as_ptr()).value }
    }

    /// Gets a mutable reference to the value in the entry.
    pub fn get_mut(&mut self) -> &mut V {
        unsafe { &mut (*self.node.as_ptr()).value }
    }

    /// Converts the `OccupiedEntry` into a mutable reference to the value in the entry with a
    /// lifetime bound to the tree itself.
    #[must_use]
    pub fn into_mut(self) -> &'a mut V {
        unsafe { &mut (*self.node.as_ptr()).value }
    }

    /// Sets the value of the entry, and returns the old value.
    pub fn insert(&mut self, value: V) -> V {
        unsafe { core::mem::replace(&mut (*self.node.as_ptr()).value, value) }
    }

    /// Takes the value out of the entry, and returns it.
    ///
    /// # Panics
    ///
    /// Panics if the tree is corrupted (entry exists but key cannot be found on removal).
    #[must_use]
    pub fn remove(self) -> V
    where
        K: Clone,
    {
        let key = self.key().clone();
        self.tree
            .remove(&key)
            .expect("occupied entry must have a value")
    }
}

impl<'a, K, V, S, A, P> VacantEntry<'a, K, V, S, A, P>
where
    K: Ord,
    P: TreePolicy<K = K, V = V, S = S>,
    A: Allocator,
{
    /// Gets a reference to the key that would be used when inserting a value through the `VacantEntry`.
    pub fn key(&self) -> &K {
        &self.key
    }

    /// Take ownership of the key.
    pub fn into_key(self) -> K {
        self.key
    }

    /// Try to insert a value with the `VacantEntry`'s key, and returns a mutable reference
    /// to it.
    ///
    /// # Errors
    ///
    /// Returns an `OutOfMemoryError` if the allocation fails.
    pub fn try_insert(self, value: V) -> Result<&'a mut V, OutOfMemoryError>
    where
        P: TreePolicy<K = K, V = V, S = S>,
    {
        // We own the key — insert it, then retrieve a pointer to the newly inserted node.
        let node = self.tree.layout.try_insert_node_get_ref(self.key, value)?;
        Ok(unsafe { &mut (*node.ptr.as_ptr()).value })
    }

    /// Set the value of the entry with the `VacantEntry`'s key, and returns a mutable reference
    /// to it.    
    pub fn insert(self, value: V) -> &'a mut V {
        self.try_insert(value)
            .unwrap_or_else(|_| handle_alloc_error(Layout::new::<Node<K, V, S>>()))
    }
}
