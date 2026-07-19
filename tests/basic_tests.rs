#![cfg(any(feature = "alloc", feature = "allocator-api", feature = "nightly"))]
#![cfg_attr(feature = "nightly", feature(allocator_api))]

mod helpers;
use crate::helpers::common::test_rng;
use augmented_rbtree::{
    AugmentedRBTree, AugmentedRBTreeFactory, MinAugmentation, SubtreeSize, Unit, constant_augment,
};
use itertools::Itertools;
use rand::RngExt;
use std::iter::repeat_with;

#[test]
fn check_empty_tree() {
    let tree = AugmentedRBTree::<i32, i32, Unit>::new();
    assert_eq!(tree.get(&1), None);
    assert!(tree.verify_augmentation());
    assert!(tree.verify_properties());
}

#[cfg(test)]
mod insert {
    use super::*;

    #[test]
    fn single_root_node() {
        let mut tree = AugmentedRBTree::<i32, i32, Unit>::new();
        tree.insert(1, 10);
        assert_eq!(tree.get(&1), Some(&10));
        assert!(tree.verify_augmentation());
        assert!(tree.verify_properties());
    }

    #[test]
    fn check_fixup_case_3r_root() {
        let mut tree = AugmentedRBTree::<i32, i32, Unit>::new();
        let keys = [20, 15, 10];
        for &key in &keys {
            tree.insert(key, key);
        }
        for &key in &keys {
            assert_eq!(tree.get(&key), Some(&key));
        }
        assert!(tree.verify_augmentation());
        assert!(tree.verify_properties());
    }

    #[test]
    fn check_fixup_case_3l_root() {
        let mut tree = AugmentedRBTree::<i32, i32, Unit>::new();
        let keys = [10, 15, 20];
        for &key in &keys {
            tree.insert(key, key);
        }
        for &key in &keys {
            assert_eq!(tree.get(&key), Some(&key));
        }

        assert!(tree.verify_augmentation());
        assert!(tree.verify_properties());
    }

    #[test]
    fn coverage() {
        let mut tree = AugmentedRBTree::<i32, i32, Unit>::new();
        let keys = [
            5, 17, 74, 37, 87, 27, 51, 65, 25, 5, 14, 40, 1, 49, 10, 80, 78, 16, 98, 87, 15, 42,
            93, 69, 34, 74, 48, 6, 97, 12, 51, 32, 86, 40, 70, 26, 31, 37, 97, 75, 65, 74, 62, 22,
            41, 37, 97, 35, 72, 31, 17, 49, 75, 49, 24, 14, 40, 94, 68, 82, 62, 13, 64, 68, 12, 76,
            46, 54, 84, 59, 36, 95, 3, 40, 87, 97, 82, 77, 98, 95, 2, 8, 64, 72, 31, 46, 54, 46,
            47, 79, 4, 54, 92, 95, 76, 32, 26, 99, 38, 8,
        ];

        for &key in &keys {
            tree.insert(key, key);
        }

        for &key in &keys {
            assert_eq!(tree.get(&key), Some(&key));
        }
        assert!(tree.verify_augmentation());
        assert!(tree.verify_properties());
    }
}

#[cfg(test)]
mod delete {
    use super::*;

    #[test]
    fn check_delete_root() {
        let mut tree = AugmentedRBTree::<i32, i32, Unit>::new();
        tree.insert(1, 10);
        tree.remove(&1);
        assert_eq!(tree.get(&1), None);
        assert!(tree.verify_augmentation());
        assert!(tree.verify_properties());
    }

    #[test]
    fn root_not_found() {
        let mut tree = AugmentedRBTree::<i32, i32, Unit>::new();
        tree.insert(1, 10);
        assert!(tree.remove(&2).is_none());
        assert!(tree.verify_augmentation());
        assert!(tree.verify_properties());
    }

    #[test]
    fn check_case1_x_nil() {
        let mut tree = AugmentedRBTree::<i32, i32, Unit>::new();
        let keys = [10, 20, 30, 40];
        for key in &keys {
            tree.insert(*key, *key);
        }
        tree.remove(&10);
        tree.remove(&40);
        assert!(tree.verify_augmentation());
        assert!(tree.verify_properties());
    }

    #[test]
    fn check_case1_x_full() {
        let mut tree = AugmentedRBTree::<i32, i32, Unit>::new();

        let keys = [10, 20, 30, 40];

        for key in &keys {
            tree.insert(*key, *key);
        }

        tree.remove(&30);
        assert!(tree.verify_augmentation());
        assert!(tree.verify_properties());
    }

    #[test]
    fn check_case2_xfull() {
        let mut tree = AugmentedRBTree::<i32, i32, Unit>::new();

        let keys = [40, 30, 20, 10];

        for key in &keys {
            tree.insert(*key, *key);
        }

        tree.remove(&20);
        assert!(tree.verify_augmentation());
        assert!(tree.verify_properties());
    }

    #[test]
    fn coverage() {
        let mut tree = AugmentedRBTree::<i32, i32, Unit>::new();
        let keys = [
            10, 34, 148, 74, 175, 53, 102, 131, 50, 10, 27, 80, 1, 97, 20, 159, 156, 31, 197, 175,
            31, 84, 186, 138, 68, 148, 96, 11, 195, 24, 102, 64, 172, 79, 140, 52, 61, 73, 194,
            150, 131, 149, 125, 43, 81, 74, 194, 70, 145, 63, 33, 98, 149, 98, 47, 28, 81, 188,
            137, 164, 123, 26, 128, 135, 24, 153, 93, 108, 168, 118, 72, 190, 6, 81, 174, 194, 164,
            154, 197, 191, 3, 16, 128, 143, 62, 91, 107, 92, 95, 158, 8, 108, 185, 190, 151, 63,
            53, 199, 76, 16,
        ];

        for &key in &keys {
            tree.insert(key, key);
        }

        for key in &keys {
            tree.remove(key);
        }

        for &key in &keys {
            assert_eq!(tree.get(&key), None);
        }
        assert!(tree.verify_augmentation());
        assert!(tree.verify_properties());
    }
}

#[test]
fn clones_must_be_equal() {
    let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new();
    let mut rng = test_rng();
    let keys: Vec<i32> = repeat_with(|| rng.random_range(1..1000))
        .unique()
        .take(500)
        .collect();
    for &key in &keys {
        tree.insert(key, key);
    }
    let clone = tree.clone();
    assert!(clone.verify_properties());
    assert!(clone.verify_augmentation());
    let mut it_tree = tree.iter();
    let mut it_clone = clone.iter();
    loop {
        let next_tree = it_tree.next();
        let next_clone = it_clone.next();

        assert_eq!(next_tree, next_clone);

        if next_tree.is_none() {
            break;
        }
    }
}

#[test]
fn check_clear() {
    let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new();
    let mut rng = test_rng();
    let keys: Vec<i32> = repeat_with(|| rng.random_range(1..1000))
        .unique()
        .take(500)
        .collect();
    for &key in &keys {
        tree.insert(key, key);
    }
    assert_eq!(tree.len(), 500);
    tree.clear();
    assert!(tree.is_empty());
    assert_eq!(tree.len(), 0);
    assert!(tree.verify_augmentation());
    assert!(tree.verify_properties());
}

#[test]
fn check_keys_and_values() {
    let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new();
    let mut rng = test_rng();
    let keys: Vec<i32> = repeat_with(|| rng.random_range(1..1000))
        .unique()
        .take(200)
        .collect();
    for &key in &keys {
        tree.insert(key, key);
    }
    let mut sorted_keys = keys.clone();
    sorted_keys.sort_unstable();
    let tree_keys: Vec<i32> = tree.keys().copied().collect();
    let tree_values: Vec<i32> = tree.values().copied().collect();
    assert_eq!(tree_keys, sorted_keys);
    assert_eq!(tree_values, sorted_keys);
    assert!(tree.verify_augmentation());
    assert!(tree.verify_properties());
}

#[test]
fn check_contains_key() {
    let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new();
    let mut rng = test_rng();
    let keys: Vec<i32> = repeat_with(|| rng.random_range(1..1000))
        .unique()
        .take(200)
        .collect();
    for &key in &keys {
        tree.insert(key, key);
    }
    for &key in &keys {
        assert!(tree.contains_key(&key));
    }
    let missing_keys: Vec<i32> = repeat_with(|| rng.random_range(1001..2000))
        .unique()
        .take(50)
        .collect();
    for &key in &missing_keys {
        assert!(!tree.contains_key(&key));
    }
    assert!(tree.verify_augmentation());
    assert!(tree.verify_properties());
}

constant_augment!(MyCustomAugment, i32, 1001);

#[test]
fn check_get_key_value_stat() {
    let mut tree = AugmentedRBTree::<i32, i32, MyCustomAugment>::new();

    let mut rng = test_rng();
    let keys: Vec<i32> = repeat_with(|| rng.random_range(1..1000))
        .unique()
        .take(200)
        .collect();

    for &key in &keys {
        tree.insert(key, key);
    }

    for &key in &keys {
        let (k, value, stats) = tree.get_key_value_stats(&key).unwrap();
        assert_eq!(k, &key);
        assert_eq!(value, &key);
        assert_eq!(stats, &1001);
    }

    for &key in &keys {
        let (k, value) = tree.get_key_value(&key).unwrap();
        assert_eq!(k, &key);
        assert_eq!(value, &key);
        assert_eq!(tree.get_key_value_stats(&key).unwrap().2, &1001);
    }

    let missing_keys: Vec<i32> = repeat_with(|| rng.random_range(1001..2000))
        .unique()
        .take(50)
        .collect();

    for &key in &missing_keys {
        assert!(tree.get_key_value_stats(&key).is_none());
    }

    assert!(tree.verify_augmentation());
    assert!(tree.verify_properties());

    assert_eq!(tree.root_stats(), Some(&1001));
}

#[test]
fn check_first_and_last_key_value() {
    let mut tree = AugmentedRBTree::<i32, i32, MyCustomAugment>::new();

    let mut rng = test_rng();
    let keys: Vec<i32> = repeat_with(|| rng.random_range(1..1000))
        .unique()
        .take(200)
        .collect();

    for &key in &keys {
        tree.insert(key, key);
    }

    let min_key = keys.iter().min().unwrap();
    let max_key = keys.iter().max().unwrap();

    let (first_key, first_value, first_stats) = tree.first_key_value_stats().unwrap();
    assert_eq!(first_key, min_key);
    assert_eq!(first_value, min_key);
    assert_eq!(first_stats, &1001);

    let (last_key, last_value, last_stats) = tree.last_key_value_stats().unwrap();
    assert_eq!(last_key, max_key);
    assert_eq!(last_value, max_key);
    assert_eq!(last_stats, &1001);
}

#[test]
fn check_pop_first_and_pop_last() {
    let mut tree = AugmentedRBTree::<i32, i32, MyCustomAugment>::new();

    let mut rng = test_rng();
    let keys: Vec<i32> = repeat_with(|| rng.random_range(1..1000))
        .unique()
        .take(200)
        .collect();

    for &key in &keys {
        tree.insert(key, key);
    }

    let mut sorted_keys = keys.clone();
    sorted_keys.sort_unstable();

    for &key in &sorted_keys {
        let (k, v) = tree.pop_first().unwrap();
        assert_eq!(k, key);
        assert_eq!(v, key);
    }

    assert!(tree.is_empty());

    for &key in &sorted_keys {
        tree.insert(key, key);
    }

    for &key in sorted_keys.iter().rev() {
        let (k, v) = tree.pop_last().unwrap();
        assert_eq!(k, key);
        assert_eq!(v, key);
    }

    assert!(tree.is_empty());
}

#[test]
fn test_remove_key_value() {
    let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new();

    let mut rng = test_rng();
    let keys: Vec<i32> = repeat_with(|| rng.random_range(1..1000))
        .unique()
        .take(200)
        .collect();

    for &key in &keys {
        tree.insert(key, key);
    }

    for &key in &keys {
        let removed = tree.remove_entry(&key);
        assert_eq!(removed, Some((key, key)));
    }

    assert!(tree.is_empty());
    assert!(tree.verify_augmentation());
    assert!(tree.verify_properties());
}

#[test]
fn test_values_mut() {
    let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new();

    let mut rng = test_rng();
    let keys: Vec<i32> = repeat_with(|| rng.random_range(1..1000))
        .unique()
        .take(200)
        .collect();

    for &key in &keys {
        tree.insert(key, key);
    }

    for mut value in tree.values_mut() {
        *value *= 2;
    }

    for &key in &keys {
        assert_eq!(tree.get(&key), Some(&(key * 2)));
    }

    assert!(tree.verify_augmentation());
    assert!(tree.verify_properties());
}

#[test]
fn check_number_of_root_nodes() {
    let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new();
    let mut rng = test_rng();
    let keys: Vec<i32> = repeat_with(|| rng.random_range(1..1000))
        .unique()
        .take(200)
        .collect();

    for &key in &keys {
        tree.insert(key, key);
    }

    assert_eq!(tree.root_stats(), Some(&200));
}

#[test]
fn check_minimum_augmentation() {
    let mut tree = AugmentedRBTree::<i32, i32, MinAugmentation>::new();
    let mut rng = test_rng();
    let keys: Vec<i32> = repeat_with(|| rng.random_range(1..1000))
        .unique()
        .take(200)
        .collect();

    for &key in &keys {
        tree.insert(key, key);
    }

    tree.insert(1, 1);

    let root_stats = tree.root_stats();
    assert_eq!(root_stats, Some(&Some(1)));
}

#[test]
fn test_create_tree_with_factory() {
    let mut tree = AugmentedRBTreeFactory::<Unit>::new_tree();
    tree.insert(1, 10);
    tree.insert(2, 20);
    tree.insert(3, 30);
    assert_eq!(tree.get(&1), Some(&10));
    assert_eq!(tree.get(&2), Some(&20));
    assert_eq!(tree.get(&3), Some(&30));
}
