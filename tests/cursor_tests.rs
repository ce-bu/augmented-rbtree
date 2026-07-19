#![cfg(any(feature = "alloc", feature = "allocator-api", feature = "nightly"))]
#![cfg(feature = "cursor")]
#![cfg_attr(feature = "nightly", feature(allocator_api))]

// mod helpers;

// use std::fmt::Debug;

// use crate::helpers::common::test_rng;
// use augmented_rbtree::{AugmentedRBTree, AugmentedRBTreeFactory, SubtreeSize};
// use rand::seq::SliceRandom;

// struct Inspector<T: Debug>(T);

// impl<T> Drop for Inspector<T>
// where
//     T: Debug,
// {
//     fn drop(&mut self) {
//         println!("~Inspector: {:?}", self.0);
//     }
// }
// #[test]
// fn cursor_check_all_bounds() {
//     let mut y = 42;
//     let mut tree = AugmentedRBTreeFactory::<SubtreeSize>::new_tree();
//     tree.insert(100, Inspector(&mut y));
//     println!("{}", y);
// }

// fn setup_test_tree() -> AugmentedRBTree<i32, i32, SubtreeSize> {
//     let mut tree = AugmentedRBTreeFactory::<SubtreeSize>::new_tree();
//     let mut keys = (1..30).collect::<Vec<_>>();
//     let mut rng = test_rng();
//     keys.shuffle(&mut rng);

//     for key in keys {
//         tree.insert(key, key);
//     }
//     tree
// }

/*
#[test]
fn cursor_check_all_bounds() {
    let tree = setup_test_tree();
    let cur_lower_bound_included = tree.lower_bound(std::ops::Bound::Included(&12));
    assert!(cur_lower_bound_included.get().is_some());
    assert_eq!(cur_lower_bound_included.get().unwrap().0, &12);

    let cur_lower_bound_excluded = tree.lower_bound(std::ops::Bound::Excluded(&12));
    assert!(cur_lower_bound_excluded.get().is_some());
    assert_eq!(cur_lower_bound_excluded.get().unwrap().0, &11);

    let cur_upper_bound_included = tree.upper_bound(std::ops::Bound::Included(&12));
    assert!(cur_upper_bound_included.get().is_some());
    assert_eq!(cur_upper_bound_included.get().unwrap().0, &12);

    let cur_upper_bound_excluded = tree.upper_bound(std::ops::Bound::Excluded(&12));
    assert!(cur_upper_bound_excluded.get().is_some());
    assert_eq!(cur_upper_bound_excluded.get().unwrap().0, &13);
}

#[test]
fn cursor_check_all_adjancent_pointers() {
    let tree = setup_test_tree();

    let mut cur = tree.lower_bound(std::ops::Bound::Included(&12));
    assert!(cur.get().is_some());
    assert_eq!(cur.get().unwrap().0, &12);

    assert!(cur.peek_next().is_some());
    assert_eq!(cur.peek_next().unwrap().0, &13);

    assert!(cur.peek_prev().is_some());
    assert_eq!(cur.peek_prev().unwrap().0, &11);

    assert!(cur.peek_parent().is_some());
    assert_eq!(cur.peek_parent().unwrap().0, &9);

    assert!(cur.peek_right().is_some());
    assert_eq!(cur.peek_right().unwrap().0, &14);

    assert!(cur.peek_left().is_some());
    assert_eq!(cur.peek_left().unwrap().0, &11);

    assert!(cur.next().is_some());
    assert_eq!(cur.get().unwrap().0, &13);

    assert!(cur.parent().is_some());
    assert_eq!(cur.get().unwrap().0, &14);

    assert!(cur.prev().is_some());
    assert_eq!(cur.get().unwrap().0, &13);

    assert!(cur.left().is_none());
    assert!(cur.right().is_none());
}

#[test]
fn cursor_mut_check_all_adjancent_pointers() {
    let mut tree = setup_test_tree();

    let mut cur = tree.lower_bound_mut(std::ops::Bound::Included(&12));
    assert!(cur.get().is_some());
    assert_eq!(cur.get().unwrap().0, &12);

    assert!(cur.peek_next().is_some());
    assert_eq!(cur.peek_next().unwrap().0, &13);

    assert!(cur.peek_prev().is_some());
    assert_eq!(cur.peek_prev().unwrap().0, &11);

    assert!(cur.peek_parent().is_some());
    assert_eq!(cur.peek_parent().unwrap().0, &9);

    assert!(cur.peek_right().is_some());
    assert_eq!(cur.peek_right().unwrap().0, &14);

    assert!(cur.peek_left().is_some());
    assert_eq!(cur.peek_left().unwrap().0, &11);

    assert!(cur.next().is_some());
    assert_eq!(cur.get().unwrap().0, &13);

    assert!(cur.parent().is_some());
    assert_eq!(cur.get().unwrap().0, &14);

    assert!(cur.prev().is_some());
    assert_eq!(cur.get().unwrap().0, &13);

    assert!(cur.left().is_none());
    assert!(cur.right().is_none());
}

#[test]
fn test_change_value_with_mutable_cursor() {
    let mut tree = setup_test_tree();
    let mut cur = tree.lower_bound_mut(std::ops::Bound::Excluded(&10));
    assert!(cur.get().is_some());
    if let Some((_, mut value, _)) = cur.get() {
        *value.as_mut() = 100;
    }

    assert_eq!(*cur.get().unwrap().1, 100);

    assert!(tree.verify_properties());
    assert!(tree.verify_augmentation());
}

// #[test]
// fn test_remove_from_cursor() {
//     let mut rng = test_rng();
//     let mut tree = AugmentedRBTreeFactory::<SubtreeSize>::new_tree();
//     let mut keys = (1..20).collect::<Vec<_>>();
//     keys.shuffle(&mut rng);

//     for key in keys {
//         tree.insert(key, key);
//     }

//     let mut cur = tree.lower_bound_mut(std::ops::Bound::Excluded(&15));
//     let _ = cur.remove().unwrap();

//     assert_eq!(tree.len(), 18);

//     assert!(tree.verify_properties());
//     assert!(tree.verify_augmentation());

//     println!("{}", tree.len());
// }
*/

// cur = tree.lower_bound(std::ops::Bound::Included(&15));
// assert!(cur.get().is_some());
// assert_eq!(cur.get().unwrap().0, &15);

// cur.left();
// assert!(cur.get().is_some());
// assert_eq!(cur.get().unwrap().0, &11);

// cur.right();
// assert!(cur.get().is_some());
// assert_eq!(cur.get().unwrap().0, &13);
