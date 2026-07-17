#![cfg(feature = "allocator-api")]

mod helpers;
use augmented_rbtree::{AugmentedRBTree, SubtreeSize};

#[test]
fn check_wrks_with_global() {
    let allocator = allocator_api2::alloc::Global;
    let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new_in(allocator);
    tree.insert(1, 10);
    tree.insert(2, 20);
    assert_eq!(tree.get(&1), Some(&10));
}
