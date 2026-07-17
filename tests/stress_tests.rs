#![cfg(any(feature = "alloc", feature = "allocator-api", feature = "nightly"))]
#![cfg_attr(feature = "nightly", feature(allocator_api))]

mod helpers;
use augmented_rbtree::{AugmentedRBTree, SubtreeSize};
use rand::seq::SliceRandom;

use crate::helpers::common::test_rng;

#[test]
fn stress_drop() {
    let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new();
    let num_nodes = || {
        #[cfg(miri)]
        {
            1000
        }
        #[cfg(not(miri))]
        {
            5_000_000
        }
    };
    for value in 1..num_nodes() {
        tree.insert(value, value);
    }

    assert!(tree.verify_augmentation());
    assert!(tree.verify_properties());
}

#[test]
fn stress_test_nil_fixup() {
    // Stress test specifically designed to trigger nil node cases in delete_fixup
    let mut rng = test_rng();
    let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new();

    // Build a large tree
    for i in 1..=500 {
        tree.insert(i, i);
    }

    // Delete in a pattern that often creates nil nodes
    // Delete every other node first (creates many nil siblings)
    for i in (1..=500).step_by(2) {
        tree.remove(&i);
        assert!(tree.verify_properties());
    }

    // Then randomly delete from remaining
    let mut remaining: Vec<i32> = (2..=500).step_by(2).collect();
    remaining.shuffle(&mut rng);

    for &value in &remaining {
        tree.remove(&value);
        assert!(tree.verify_properties());
        assert!(tree.verify_augmentation());
    }

    assert!(tree.is_empty());
}

#[test]
fn delete_min_max_repeatedly() {
    // Repeatedly delete minimum and maximum values
    let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new();

    for i in 1..=100 {
        tree.insert(i, i);
    }

    let mut min = 1;
    let mut max = 100;

    while min <= max {
        // Delete min
        tree.remove(&min);
        assert!(tree.verify_properties());
        assert!(tree.verify_augmentation());
        min += 1;

        if min <= max {
            // Delete max
            tree.remove(&max);
            assert!(tree.verify_properties());
            assert!(tree.verify_augmentation());
            max -= 1;
        }
    }

    assert!(tree.is_empty());
}
