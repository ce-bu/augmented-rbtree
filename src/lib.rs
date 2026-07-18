//! # augmented-rbtree
//!
//! An augmented red-black tree with generic, user-defined per-node statistics.
//!
//! ## What is an Augmented Red-Black Tree?
//!
//! A standard [red-black tree] is a self-balancing binary search tree that guarantees
//! O(log n) insert, delete, and lookup. An *augmented* variant extends each node with
//! extra data (called *statistics* or *augmentation*), computed from its subtree.
//! Because the tree stays balanced, the statistics at the root always reflect the entire
//! collection, and any prefix or suffix can be queried in O(log n) time.
//!
//! Common examples built on this primitive:
//!
//! | Use case | Key | Value | Augmentation |
//! |----------|-----|-------|-------------|
//! | **Order-statistics tree** | any | any | subtree size |
//! | **Interval tree** | interval start | interval end | max endpoint in subtree |
//! | **Range-sum tree** | any | numeric | subtree sum |
//! | **Range-max tree** | any | numeric | subtree max |
//!
//! [red-black tree]: https://en.wikipedia.org/wiki/Red%E2%80%93black_tree
//!
//! ## Quick Start
//!
//! Add to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! augmented-rbtree = "0.1"
//! ```
//!
//! Implement the [`Augment`] trait or use one of the built-in augmentations in [`augmentations`]. Then create a tree:
//!
//!
//! # Examples
//! ```
//! use augmented_rbtree::{Augment, AugmentedRBTreeFactory};
//!
//! /// Tracks the number of nodes in each subtree.
//! struct SubtreeCount;
//!
//! impl<K, V> Augment<K, V> for SubtreeCount {
//!     type Stats = usize;
//!
//!     fn compute(_k: &K, _v: &V,
//!                left:  Option<(&K, &V, &usize)>,
//!                right: Option<(&K, &V, &usize)>) -> usize {
//!         1 + left.map(|(_, _, &c)| c).unwrap_or(0)
//!           + right.map(|(_, _, &c)| c).unwrap_or(0)
//!     }
//! }
//! fn main() {
//!     let mut tree = AugmentedRBTreeFactory::<SubtreeCount>::new_tree();
//!     tree.insert(3, "c");
//!     tree.insert(1, "a");
//!     tree.insert(2, "b");
//!
//!     // Total count is always at the root
//!     assert_eq!(tree.root_stats(), Some(&3));
//!
//!     // Standard ordered-map operations
//!     assert_eq!(tree.get(&2), Some(&"b"));
//!     assert_eq!(tree.first_key_value_stats(), Some((&1, &"a", &1)));
//!
//!     // Iterate in sorted order; each entry exposes (key, value, stats)
//!     for (k, v, count) in tree.iter() {
//!         println!("key={k}, value={v}, subtree_size={count}");
//!     }
//! }
//! ```
//!
//! ## Feature Flags
//!
//! | Feature | Default | Description |
//! |---------|---------|-------------|
//! | `alloc` | **Yes** | Uses the standard `alloc` crate for baseline heap allocation support. |
//! | `allocator-api` | No | Enables custom local allocator support on stable Rust via `allocator-api2`. |
//! | `nightly` | No | Opts into the upstream standard `core::alloc::Allocator` API (Requires Nightly). |
//! | `serde` | No | Implements [`serde::Serialize`] and [`serde::Deserialize`] for the tree. |
//! | `debug` | No | Makes `verify_properties` and `verify_augmentation` available in release builds. |
//! | `interval_tree` | No | Enables the [`interval_tree`] module, which implements an interval tree using this crate. |
//!
//! ## Red-Black Tree Properties
//!
//! 1. Every node is Red or Black.
//! 2. The root is Black.
//! 3. All nil leaves are Black.
//! 4. A Red node's children are both Black.
//! 5. Every path from a node to a descendant nil has the same number of Black nodes.

#![no_std]
// enable the allocator_api feature on nightly toolchains to access the `Allocator` trait and related APIs
#![cfg_attr(feature = "nightly", feature(allocator_api))]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![warn(rust_2018_idioms)]
#![allow(clippy::type_complexity)]
#![deny(rustdoc::broken_intra_doc_links)]
#![warn(clippy::doc_markdown)]
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

// If both are enabled, prioritize nightly and throw a helpful compiler warning
#[cfg(all(feature = "nightly", feature = "allocator-api", not(doc)))]
compile_error!(
    "The features 'nightly' and 'allocator-api' are mutually exclusive. \
     Please enable 'nightly' for nightly toolchains, or 'allocator-api' for stable toolchains."
);

#[cfg(any(
    feature = "alloc",
    feature = "allocator-api",
    feature = "nightly",
    test
))]
extern crate alloc;

mod alloc_proxy;
mod augment;
pub mod augmentations;
mod augmented_rbtree;
pub mod entry;
#[cfg(feature = "interval-tree")]
pub mod interval_tree;
mod iterators;
mod layout;
mod node;
mod node_allocator;
mod policy;

#[cfg(feature = "serde")]
mod serde_impl;

#[cfg(any(feature = "nightly", feature = "allocator-api", feature = "alloc"))]
pub use alloc_proxy::proxy::{AllocError, Allocator, Global, Layout};

pub use augment::Augment;
pub use augmentations::{IntervalMaxEnd, MinAugmentation, SubtreeSize, Unit};
pub use augmented_rbtree::{
    AugmentedRBTree, AugmentedRBTreeFactory, OutOfMemoryError, RBTree,
    internal_details::AugmentedRBTreeInt,
};
pub use entry::{Entry, OccupiedEntry, VacantEntry};
pub use iterators::{Iter, Keys, Range, RangeMut, ValMut, Values, ValuesMut};
pub use node::Color;
pub use policy::internal_details::TreePolicy;

#[cfg(feature = "serde")]
pub use serde_impl::AugmentedRBTreeSeed;
