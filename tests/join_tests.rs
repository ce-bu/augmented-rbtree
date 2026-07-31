#![cfg(any(feature = "alloc", feature = "allocator-api", feature = "nightly"))]
#![cfg_attr(feature = "nightly", feature(allocator_api))]

use std::iter::repeat_with;

use augmented_rbtree::{AugmentedRBTree, AugmentedRBTreeFactory, SubtreeSize, try_join};
use itertools::Itertools;
use rand::{RngExt, rngs::SmallRng};

use crate::helpers::common::test_rng;

mod helpers;

fn create_test_tree(
    size: usize,
    range: std::ops::Range<i32>,
    rng: &mut SmallRng,
) -> AugmentedRBTree<i32, i32, SubtreeSize> {
    let mut tree = AugmentedRBTreeFactory::<SubtreeSize>::new_tree();

    let keys: Vec<i32> = repeat_with(|| rng.random_range(range.clone()))
        .unique()
        .take(size)
        .collect();

    tree.extend(keys.iter().map(|&key| (key, key)));

    tree
}

#[test]
fn test_join_two_empty_trees() {
    let left_tree = AugmentedRBTreeFactory::<SubtreeSize>::new_tree();
    let right_tree = AugmentedRBTreeFactory::<SubtreeSize>::new_tree();

    let tree = try_join(left_tree, right_tree, 0, 0).expect("Join failed");

    assert_eq!(tree.len(), 1);
    assert!(tree.verify_properties());
    assert!(tree.verify_augmentation());
}

#[test]
fn test_join_big_small() {
    let mut rng = test_rng();

    let left_tree = create_test_tree(20, 0..50, &mut rng);
    let right_tree = create_test_tree(10, 60..100, &mut rng);

    let len_left = left_tree.len();
    let len_right = right_tree.len();
    let tree = try_join(left_tree, right_tree, 55, 55).expect("Join failed");

    assert_eq!(tree.len(), len_left + len_right + 1);

    assert!(tree.verify_properties());
    assert!(tree.verify_augmentation());
}

#[test]
fn test_join_small_big() {
    let mut rng = test_rng();
    let left_tree = create_test_tree(10, 0..50, &mut rng);
    let right_tree = create_test_tree(20, 60..100, &mut rng);
    let len_left = left_tree.len();
    let len_right = right_tree.len();
    let tree = try_join(left_tree, right_tree, 55, 55).expect("Join failed");

    assert_eq!(tree.len(), len_left + len_right + 1);
    assert!(tree.verify_properties());
    assert!(tree.verify_augmentation());
}

#[test]
fn test_join_with_empty_left() {
    let mut rng = test_rng();
    let left_tree = AugmentedRBTreeFactory::<SubtreeSize>::new_tree();
    let right_tree = create_test_tree(20, 60..100, &mut rng);
    let len_right = right_tree.len();
    let tree = try_join(left_tree, right_tree, 55, 55).expect("Join failed");

    assert_eq!(tree.len(), len_right + 1);
    assert!(tree.verify_properties());
    assert!(tree.verify_augmentation());
}

#[test]
fn test_join_with_empty_right() {
    let mut rng = test_rng();
    let left_tree = create_test_tree(20, 0..50, &mut rng);
    let right_tree = AugmentedRBTreeFactory::<SubtreeSize>::new_tree();
    let len_left = left_tree.len();
    let tree = try_join(left_tree, right_tree, 55, 55).expect("Join failed");

    assert_eq!(tree.len(), len_left + 1);
    assert!(tree.verify_properties());
    assert!(tree.verify_augmentation());
}
