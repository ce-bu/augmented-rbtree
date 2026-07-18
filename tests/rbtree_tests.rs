#![cfg(any(feature = "alloc", feature = "allocator-api", feature = "nightly"))]
#![cfg_attr(feature = "nightly", feature(allocator_api))]

mod helpers;
use std::iter::repeat_with;

use crate::helpers::common::test_rng;
use augmented_rbtree::RBTree;
use itertools::Itertools;
use rand::RngExt;

#[test]
fn test_simple_rbtree() {
    let mut tree = RBTree::new();
    let mut rng = test_rng();
    let keys: Vec<i32> = repeat_with(|| rng.random_range(1..1000))
        .unique()
        .take(500)
        .collect();
    for &key in &keys {
        tree.insert(key, key);
    }
    for &key in &keys {
        assert!(tree.contains_key(&key));
    }
}

#[test]
fn create_rbtree_ergonomic() {
    let mut tree = RBTree::new();
    let mut rng = test_rng();
    let keys: Vec<i32> = repeat_with(|| rng.random_range(1..1000))
        .unique()
        .take(500)
        .collect();
    for &key in &keys {
        tree.insert(key, key);
    }
    for &key in &keys {
        assert!(tree.contains_key(&key));
    }
}

#[test]
fn create_rbtree_using_allocator() {
    let mut tree = RBTree::new_in(augmented_rbtree::Global);
    let mut rng = test_rng();
    let keys: Vec<i32> = repeat_with(|| rng.random_range(1..1000))
        .unique()
        .take(500)
        .collect();
    for &key in &keys {
        tree.insert(key, key);
    }
    for &key in &keys {
        assert!(tree.contains_key(&key));
    }
}
