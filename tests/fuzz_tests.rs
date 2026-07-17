#![cfg(any(feature = "alloc", feature = "allocator-api", feature = "nightly"))]
#![cfg_attr(feature = "nightly", feature(allocator_api))]

mod helpers;

use crate::helpers::common::test_rng;
use augmented_rbtree::{AugmentedRBTree, SubtreeSize};
use rand::RngExt;
use rand::seq::SliceRandom;

#[test]
fn fuzz_insert_delete_small() {
    let mut rng = test_rng();
    let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new();
    let mut expected = std::collections::HashSet::new();

    for _ in 0..50 {
        let value = rng.random_range(1..=20);

        if rng.random_bool(0.5) {
            tree.insert(value, value);
            println!("tree.insert({value}, {value});");
            expected.insert(value);
            println!("expected.insert({value});");
        } else {
            tree.remove(&value);
            println!("tree.remove(&{value});");
            expected.remove(&value);
            println!("expected.remove(&{value});");
        }

        assert!(tree.verify_properties(), "Properties violated");
        assert!(tree.verify_augmentation(), "Augmentation violated");
    }

    assert_eq!(tree.len(), expected.len());
    for &val in &expected {
        assert_eq!(tree.get(&val), Some(&val));
    }
}

#[test]
fn fuzz_insert_delete_medium() {
    let mut rng = test_rng();
    let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new();
    let mut expected = std::collections::HashSet::new();

    for _ in 0..200 {
        let value = rng.random_range(1..=100);

        if rng.random_bool(0.6) {
            tree.insert(value, value);
            expected.insert(value);
        } else {
            tree.remove(&value);
            expected.remove(&value);
        }

        assert!(tree.verify_properties(), "Properties violated");
        assert!(tree.verify_augmentation(), "Augmentation violated");
    }

    assert_eq!(tree.len(), expected.len());
}

#[test]
fn fuzz_insert_delete_large() {
    let mut rng = test_rng();
    let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new();
    let mut expected = std::collections::HashSet::new();

    for iteration in 0..1000 {
        let value = rng.random_range(1..=500);

        if rng.random_bool(0.6) {
            tree.insert(value, value);
            expected.insert(value);
        } else {
            tree.remove(&value);
            expected.remove(&value);
        }

        // Check every 100 iterations to avoid excessive overhead
        if iteration % 100 == 0 {
            assert!(
                tree.verify_properties(),
                "Properties violated at iteration {iteration}"
            );
            assert!(
                tree.verify_augmentation(),
                "Augmentation violated at iteration {iteration}"
            );
        }
    }

    // Final verification
    assert!(tree.verify_properties());
    assert!(tree.verify_augmentation());
    assert_eq!(tree.len(), expected.len());
}

#[test]
fn fuzz_delete_heavy() {
    let mut rng = test_rng();
    let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new();

    for i in 1..=200 {
        tree.insert(i, i);
    }

    for _ in 0..300 {
        let value = rng.random_range(1..=200);
        tree.remove(&value);

        assert!(tree.verify_properties(), "Properties violated");
        assert!(tree.verify_augmentation(), "Augmentation violated");
    }
}

#[test]
fn fuzz_alternating_insert_delete() {
    let mut rng = test_rng();
    let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new();
    let mut inserted = Vec::new();

    for _ in 0..100 {
        let value = rng.random_range(1..=1000);
        tree.insert(value, value);
        inserted.push(value);
        assert!(tree.verify_properties());
        assert!(tree.verify_augmentation());

        // Delete
        if !inserted.is_empty() {
            let idx = rng.random_range(0..inserted.len());
            let to_remove = inserted.swap_remove(idx);
            tree.remove(&to_remove);
            assert!(tree.verify_properties());
            assert!(tree.verify_augmentation());
        }
    }
}

#[test]
fn fuzz_sequential_insert_random_delete() {
    let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new();
    let mut rng = test_rng();

    // Sequential insert
    for i in 1..=300 {
        tree.insert(i, i);
    }

    assert!(tree.verify_properties());
    assert!(tree.verify_augmentation());

    // Random delete
    let mut values: Vec<i32> = (1..=300).collect();
    values.shuffle(&mut rng);

    for (idx, &value) in values.iter().enumerate() {
        tree.remove(&value);

        if idx % 50 == 0 {
            assert!(
                tree.verify_properties(),
                "Properties failed at deletion {idx}"
            );
            assert!(
                tree.verify_augmentation(),
                "Augmentation failed at deletion {idx}"
            );
        }
    }

    assert!(tree.is_empty());
}

#[test]
fn fuzz_duplicate_operations() {
    let mut rng = test_rng();
    let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new();

    for _ in 0..200 {
        let value = rng.random_range(1..=20); // Small range for many duplicates

        if rng.random_bool(0.5) {
            tree.insert(value, value);
        } else {
            tree.remove(&value);
        }

        assert!(tree.verify_properties());
        assert!(tree.verify_augmentation());
    }
}
