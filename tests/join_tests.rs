#![cfg(any(feature = "alloc", feature = "allocator-api", feature = "nightly"))]
#![cfg_attr(feature = "nightly", feature(allocator_api))]

use std::iter::repeat_with;

use augmented_rbtree::{AugmentedRBTreeFactory, SubtreeSize};
use itertools::Itertools;
use rand::RngExt;

use crate::helpers::{Result, common::test_rng, dumper::dump_tree};

mod helpers;

// dump_tree(&tree, Some("before"), true);
// println!("Black height before deletion: {}", tree.black_height());

// dump_tree(&tree, Some("after"), true);
// println!("Black height before deletion: {}", tree.black_height());

#[test]
fn test_join() -> Result<()> {
    let mut rng = test_rng();

    let mut tree1 = AugmentedRBTreeFactory::<SubtreeSize>::new_tree();

    let keys1: Vec<i32> = repeat_with(|| rng.random_range(1..50))
        .unique()
        .take(10)
        .collect();

    tree1.extend(keys1.iter().map(|&key| (key, key)));

    dump_tree(&tree1, Some("tree1"), true)?;

    let mut tree2 = AugmentedRBTreeFactory::<SubtreeSize>::new_tree();

    let keys2: Vec<i32> = repeat_with(|| rng.random_range(60..100))
        .unique()
        .take(3)
        .collect();

    tree2.extend(keys2.iter().map(|&key| (key, key)));
    dump_tree(&tree2, Some("tree2"), true)?;

    let tree = tree1.try_join(55, 55, tree2).expect("Join failed");

    dump_tree(&tree, Some("tree"), true)?;

    assert!(tree.verify_properties());
    assert!(tree.verify_augmentation());
    Ok(())
}
