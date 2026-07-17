#![cfg(all(
    feature = "serde",
    any(feature = "alloc", feature = "allocator-api", feature = "nightly")
))]
#![cfg_attr(feature = "nightly", feature(allocator_api))]

mod helpers;
use crate::helpers::{
    Result,
    common::{
        custom_augment_a::{CustomAugment, CustomKey, CustomValue},
        test_rng,
    },
};
use augmented_rbtree::{AugmentedRBTree, AugmentedRBTreeSeed, Global, SubtreeSize};
use rand::RngExt;
use serde::de::DeserializeSeed;

#[test]
fn serde_test_same_tree() -> Result<()> {
    let mut tree = AugmentedRBTree::<CustomKey, CustomValue, CustomAugment>::new();
    let mut rng = test_rng();

    for _ in 0..100 {
        let key = rng.random_range(1..100);
        tree.insert(CustomKey(key), CustomValue(key.to_string()));
    }

    let serialized = serde_json::to_string(&tree)?;
    let seed = AugmentedRBTreeSeed::<CustomKey, CustomValue, CustomAugment>::new(Global);

    let deserialized: AugmentedRBTree<CustomKey, CustomValue, CustomAugment> =
        seed.deserialize(&mut serde_json::Deserializer::from_str(&serialized))?;

    assert_eq!(tree.len(), deserialized.len());
    for (key, value, _stats) in &tree {
        assert_eq!(deserialized.get(key), Some(value));
    }

    Ok(())
}

#[test]
fn serde_test_ok_data() -> Result<()> {
    let seed = AugmentedRBTreeSeed::<i32, i32, SubtreeSize>::new(Global);
    let deserialized: AugmentedRBTree<i32, i32, SubtreeSize> =
        seed.deserialize(&mut serde_json::Deserializer::from_str("[[1,1], [2,2]]"))?;

    assert!(deserialized.verify_properties());
    assert!(deserialized.verify_augmentation());
    assert_eq!(deserialized.len(), 2);
    assert_eq!(deserialized.get(&1), Some(&1));
    assert_eq!(deserialized.get(&2), Some(&2));
    Ok(())
}

#[test]
fn serde_test_bad_data() {
    {
        let seed = AugmentedRBTreeSeed::<i32, i32, SubtreeSize>::new(Global);
        let deserialized1: std::result::Result<AugmentedRBTree<i32, i32, SubtreeSize>, _> =
            seed.deserialize(&mut serde_json::Deserializer::from_str("[[1,1], [2]]"));
        assert!(deserialized1.is_err());
    }
    {
        let seed = AugmentedRBTreeSeed::<i32, i32, SubtreeSize>::new(Global);
        let deserialized2: std::result::Result<AugmentedRBTree<i32, i32, SubtreeSize>, _> =
            seed.deserialize(&mut serde_json::Deserializer::from_str("1234"));
        assert!(deserialized2.is_err());
    }
}
