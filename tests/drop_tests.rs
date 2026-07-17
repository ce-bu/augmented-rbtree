#![cfg(any(feature = "alloc", feature = "allocator-api", feature = "nightly"))]
#![cfg_attr(feature = "nightly", feature(allocator_api))]

mod helpers;

use crate::helpers::common::custom_augment_a::{
    CustomAugment, CustomKey, CustomValue, DROP_CUSTOM_KEY_LOGGER, DROP_CUSTOM_STATS_LOGGER,
    DROP_CUSTOM_VALUE_LOGGER, reset_drop_loggers,
};
use crate::helpers::common::test_rng;
use augmented_rbtree::AugmentedRBTree;
use itertools::Itertools;
use rand::RngExt;
use std::iter::repeat_with;

#[test]
fn check_keys_values_stats_are_droped() {
    reset_drop_loggers();
    let mut rng = test_rng();
    let keys: Vec<i32> = repeat_with(|| rng.random_range(1..100))
        .unique()
        .take(20)
        .collect();
    {
        let mut tree = AugmentedRBTree::<CustomKey, CustomValue, CustomAugment>::new();

        for &key in &keys {
            tree.insert(CustomKey(key), CustomValue(key.to_string()));
        }
    }
    let mut dropped_keys = DROP_CUSTOM_KEY_LOGGER.with_borrow(std::clone::Clone::clone);
    let mut orig_keys = keys
        .iter()
        .map(std::string::ToString::to_string)
        .collect::<Vec<_>>();
    dropped_keys.sort();
    orig_keys.sort();
    assert_eq!(dropped_keys, orig_keys);
    let mut dropped_values = DROP_CUSTOM_VALUE_LOGGER.with_borrow(std::clone::Clone::clone);
    let mut orig_values = keys
        .iter()
        .map(std::string::ToString::to_string)
        .collect::<Vec<_>>();
    dropped_values.sort();
    orig_values.sort();
    assert_eq!(dropped_values, orig_values);
    let dropped_stats_len = DROP_CUSTOM_STATS_LOGGER.with_borrow(std::vec::Vec::len);
    assert!(dropped_stats_len > dropped_keys.len());
}

#[test]
fn check_into_iter_drops_remaining_values() {
    reset_drop_loggers();
    let mut rng = test_rng();
    let keys: Vec<i32> = repeat_with(|| rng.random_range(1..100))
        .unique()
        .take(20)
        .collect();

    {
        let mut tree = AugmentedRBTree::<CustomKey, CustomValue, CustomAugment>::new();

        for &key in &keys {
            tree.insert(CustomKey(key), CustomValue(key.to_string()));
        }

        let mut consumed = tree.into_iter();

        let _ = consumed.next();
        let _ = consumed.next();

        assert_eq!(consumed.len(), 18);
    }

    let mut dropped_keys = DROP_CUSTOM_KEY_LOGGER.with_borrow(std::clone::Clone::clone);
    let mut orig_keys = keys
        .iter()
        .map(std::string::ToString::to_string)
        .collect::<Vec<_>>();
    dropped_keys.sort();
    orig_keys.sort();
    assert_eq!(dropped_keys, orig_keys);

    let mut dropped_values = DROP_CUSTOM_VALUE_LOGGER.with_borrow(std::clone::Clone::clone);
    let mut orig_values = keys
        .iter()
        .map(std::string::ToString::to_string)
        .collect::<Vec<_>>();
    dropped_values.sort();
    orig_values.sort();
    assert_eq!(dropped_values, orig_values);
}
