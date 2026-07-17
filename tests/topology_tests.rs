#![cfg(any(feature = "alloc", feature = "allocator-api", feature = "nightly"))]
#![cfg_attr(feature = "nightly", feature(allocator_api))]

mod helpers;
use augmented_rbtree::{AugmentedRBTree, SubtreeSize};

#[test]
fn check_topology() {
    let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new();
    let mut keys = vec![20, 15, 10, 25, 30];
    for &key in &keys {
        tree.insert(key, key);
    }
    let mut topo_keys = Vec::new();
    tree.visit_topology(|key, _color, _left, _right| {
        topo_keys.push(*key);
    });
    keys.sort_unstable();
    topo_keys.sort_unstable();
    assert_eq!(keys, topo_keys);
    assert!(tree.verify_augmentation());
    assert!(tree.verify_properties());
}
