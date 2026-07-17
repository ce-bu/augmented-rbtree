#![cfg(any(feature = "alloc", feature = "allocator-api", feature = "nightly"))]
#![cfg_attr(feature = "nightly", feature(allocator_api))]

mod helpers;

use crate::helpers::limited_allocator::LimitedAllocator;
use augmented_rbtree::{AugmentedRBTree, Unit};

#[test]
fn test_limited_allocator() {
    let allocator = LimitedAllocator::new(5, 1024);
    let mut tree = AugmentedRBTree::<i32, i32, Unit, LimitedAllocator>::new_in(allocator);
    tree.insert(1, 10);
    tree.insert(2, 20);
    tree.insert(3, 30);
    tree.insert(4, 40);
    tree.insert(5, 50);
    assert!(tree.try_insert(6, 60).is_err()); // This should fail due to limited allocator
    let msg = tree.try_insert(10, 20).map_err(|e| format!("{e:?}"));
    assert!(msg.is_err());
    assert!(msg.unwrap_err().contains("OutOfMemoryError"));
    assert_eq!(tree.get(&1), Some(&10));
    assert_eq!(tree.get(&2), Some(&20));
    assert_eq!(tree.get(&3), Some(&30));
    assert_eq!(tree.get(&4), Some(&40));
    assert_eq!(tree.get(&5), Some(&50));
    assert!(tree.get(&6).is_none());

    tree.remove(&3);
    assert!(tree.try_insert(6, 60).is_ok()); // Now this should succeed since we freed up a slot
    assert_eq!(tree.get(&6), Some(&60));
    assert!(tree.get(&3).is_none());
}

#[test]
fn check_that_all_slots_are_available_after_drop() {
    let allocator = LimitedAllocator::new(5, 1024);
    {
        let mut tree =
            AugmentedRBTree::<i32, i32, Unit, LimitedAllocator>::new_in(allocator.clone());
        tree.insert(1, 10);
        tree.insert(2, 20);
        tree.insert(3, 30);
        tree.insert(4, 40);
        tree.insert(5, 50);
        assert!(tree.try_insert(6, 60).is_err()); // This should fail due to limited allocator
    }
    assert_eq!(allocator.num_free_slots(), 5); // All slots should be available again
}

#[test]
fn check_that_clone_works_with_cusom_allocator() {
    let allocator = LimitedAllocator::new(5, 1024);
    let mut tree1 = AugmentedRBTree::<i32, i32, Unit, LimitedAllocator>::new_in(allocator.clone());
    tree1.insert(1, 10);
    tree1.insert(2, 20);
    let tree2 = tree1.try_clone();
    assert!(tree2.is_ok());
    let mut tree2 = tree2.unwrap();
    assert_eq!(tree2.get(&1), Some(&10));
    assert_eq!(tree2.get(&2), Some(&20));
    tree2.insert(3, 30);
    assert_eq!(tree2.get(&3), Some(&30));
    assert!(tree1.get(&3).is_none());
}

#[test]
fn check_that_clone_fails_with_insufficient_memory() {
    let allocator = LimitedAllocator::new(5, 1024);
    let mut tree1 = AugmentedRBTree::<i32, i32, Unit, LimitedAllocator>::new_in(allocator.clone());
    tree1.insert(1, 10);
    tree1.insert(2, 20);
    tree1.insert(3, 30);
    tree1.insert(4, 40);
    tree1.insert(5, 50);

    let tree2 = tree1.try_clone();
    assert!(tree2.is_err()); // This should fail due to limited allocator
}
