#![cfg(any(feature = "alloc", feature = "allocator-api", feature = "nightly"))]

use augmented_rbtree::{AugmentedRBTree, Unit};
use proptest::prelude::*;
use std::collections::BTreeMap;

// ============================================================================
// Augmentation: identity (no-op, for oracle comparison)
// ============================================================================

// ============================================================================
// Strategy helpers
// ============================================================================

/// Operations to fuzz
#[derive(Debug, Clone)]
enum Op {
    Insert(i32, i32),
    Remove(i32),
    Range(i32, i32),
}

fn op_strategy() -> impl Strategy<Value = Op> {
    prop_oneof![
        (
            any::<i8>().prop_map(i32::from),
            any::<i8>().prop_map(i32::from)
        )
            .prop_map(|(k, v)| Op::Insert(k, v)),
        any::<i8>().prop_map(|k| Op::Remove(i32::from(k))),
        (
            any::<i8>().prop_map(i32::from),
            any::<i8>().prop_map(i32::from)
        )
            .prop_map(|(a, b)| Op::Range(a.min(b), a.max(b))),
    ]
}

// ============================================================================
// Property: tree matches BTreeMap as oracle
// ============================================================================

fn miri_safe_config() -> ProptestConfig {
    // Completely disable shrinking under Miri to bypass the generation hang
    #[cfg(miri)]
    {
        let mut config = ProptestConfig::default();
        config.cases = 8;
        config.max_shrink_iters = 0;
        config.failure_persistence = None;
        config
    }

    #[cfg(not(miri))]
    ProptestConfig::default()
}

#[cfg(not(miri))]
fn input_strategy() -> impl Strategy<Value = Vec<Op>> {
    // Keep your original 0..200 vector generator for standard tests
    proptest::collection::vec(op_strategy(), 0..200)
}

#[cfg(miri)]
fn input_strategy() -> impl Strategy<Value = Vec<Op>> {
    proptest::collection::vec(op_strategy(), 5..15)
}

proptest! {
    #![proptest_config(miri_safe_config())]
    /// The augmented tree must behave identically to BTreeMap for all ordered-map operations.
    #[test]
    fn prop_matches_btreemap(ops in input_strategy()) {
        let mut tree = AugmentedRBTree::<i32, i32, Unit>::new();
        let mut oracle = BTreeMap::<i32, i32>::new();

        for op in &ops {
            match *op {
                Op::Insert(k, v) => {
                    let tree_result = tree.insert(k, v);
                    let oracle_result = oracle.insert(k, v);
                    prop_assert_eq!(tree_result, oracle_result);
                }
                Op::Remove(k) => {
                    let tree_result = tree.remove(&k);
                    let oracle_result = oracle.remove(&k);
                    prop_assert_eq!(tree_result, oracle_result);
                }
                Op::Range(lo, hi) => {
                    let tree_range: Vec<_> = tree.range(lo..=hi).map(|(k, v, ())| (*k, *v)).collect();
                    let oracle_range: Vec<_> = oracle.range(lo..=hi).map(|(&k, &v)| (k, v)).collect();
                    prop_assert_eq!(&tree_range, &oracle_range);
                }
            }
        }

        // Final length must match
        prop_assert_eq!(tree.len(), oracle.len(), "final len mismatch");

        // Full sorted iteration must match
        let tree_all: Vec<_> = tree.iter().map(|(k, v, ())| (*k, *v)).collect();
        let oracle_all: Vec<_> = oracle.iter().map(|(&k, &v)| (k, v)).collect();
        prop_assert_eq!(&tree_all, &oracle_all, "final iteration mismatch");
    }

    /// first_key_value / last_key_value must match BTreeMap
    #[test]
    fn prop_first_last_match_btreemap(ops in proptest::collection::vec(op_strategy(), 0..50)) {
        let mut tree = AugmentedRBTree::<i32, i32, Unit>::new();
        let mut oracle = BTreeMap::<i32, i32>::new();

        for op in &ops {
            match *op {
                Op::Insert(k, v) => { tree.insert(k, v); oracle.insert(k, v); }
                Op::Remove(k) => { tree.remove(&k); oracle.remove(&k); }
                Op::Range(..) => {}
            }
        }

        let tree_first = tree.first_key_value_stats().map(|(k, v, ())| (*k, *v));
        let oracle_first = oracle.first_key_value().map(|(&k, &v)| (k, v));
        prop_assert_eq!(tree_first, oracle_first);

        let tree_last = tree.last_key_value_stats().map(|(k, v, ())| (*k, *v));
        let oracle_last = oracle.last_key_value().map(|(&k, &v)| (k, v));
        prop_assert_eq!(tree_last, oracle_last);
    }

    /// pop_first / pop_last must match BTreeMap
    #[test]
    fn prop_pop_first_last(ops in proptest::collection::vec(
        (any::<i8>().prop_map(i32::from), any::<i8>().prop_map(i32::from)),
        1..30
    )) {
        let mut tree = AugmentedRBTree::<i32, i32, Unit>::new();
        let mut oracle = BTreeMap::<i32, i32>::new();

        for (k, v) in ops {
            tree.insert(k, v);
            oracle.insert(k, v);
        }

        while !oracle.is_empty() {
            if oracle.len().is_multiple_of(2) {
                let t = tree.pop_first();
                let o = oracle.pop_first();
                prop_assert_eq!(t, o, "pop_first mismatch");
            } else {
                let t = tree.pop_last();
                let o = oracle.pop_last();
                prop_assert_eq!(t, o, "pop_last mismatch");
            }
        }
        prop_assert!(tree.is_empty());
    }

    /// contains_key must match BTreeMap
    #[test]
    fn prop_contains_key(
        inserts in proptest::collection::vec(any::<i8>().prop_map(i32::from), 0..50),
        queries in proptest::collection::vec(any::<i8>().prop_map(i32::from), 0..50),
    ) {
        let mut tree = AugmentedRBTree::<i32, i32, Unit>::new();
        let mut oracle = BTreeMap::<i32, i32>::new();
        for k in &inserts {
            tree.insert(*k, *k);
            oracle.insert(*k, *k);
        }
        for k in &queries {
            prop_assert_eq!(tree.contains_key(k), oracle.contains_key(k));
        }
    }

    /// SubtreeSize augmentation must always equal actual subtree sizes.
    #[test]
    fn prop_subtree_size_correct(ops in proptest::collection::vec(
        prop_oneof![
            any::<i8>().prop_map(|k| (i32::from(k), true)),
            any::<i8>().prop_map(|k| (i32::from(k), false)),
        ],
        0..100,
    )) {
        use augmented_rbtree::augmentations::SubtreeSize;
        let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new();

        for (k, insert) in &ops {
            if *insert {
                tree.insert(*k, *k);
            } else {
                tree.remove(k);
            }
        }

        // Root stats must equal tree length
        if tree.is_empty() {
            prop_assert_eq!(tree.root_stats(), None);
        } else {
            let expected_size = tree.len();
            prop_assert_eq!(tree.root_stats(), Some(&expected_size));
        }

        prop_assert!(tree.verify_properties());
        prop_assert!(tree.verify_augmentation());
    }

    /// SumAugmentation root_stats must equal sum of all values.
    #[test]
    fn prop_sum_augmentation_correct(ops in proptest::collection::vec(
        (any::<i8>().prop_map(i32::from), any::<i8>().prop_map(i64::from)),
        0..100,
    )) {
        use augmented_rbtree::augmentations::SumAugmentation;
        let mut tree = AugmentedRBTree::<i32, i64, SumAugmentation>::new();
        let mut oracle = BTreeMap::<i32, i64>::new();

        for (k, v) in ops {
            tree.insert(k, v);
            oracle.insert(k, v);
        }

        let expected_sum: i64 = oracle.values().sum();
        let actual_sum = tree.root_stats().copied().unwrap_or(0);
        prop_assert_eq!(actual_sum, expected_sum);
    }

    /// entry() API must behave like BTreeMap's entry API
    #[test]
    fn prop_entry_api(words in proptest::collection::vec(
        any::<u8>().prop_map(|b| i32::from(b % 5)), // small key space
        0..100,
    )) {
        let mut tree = AugmentedRBTree::<i32, u32, Unit>::new();
        let mut oracle = BTreeMap::<i32, u32>::new();

        for word in &words {
            let tc = tree.entry(*word).or_insert(0);
            *tc += 1;
            let oc = oracle.entry(*word).or_insert(0);
            *oc += 1;
        }

        let tree_all: Vec<_> = tree.iter().map(|(k, v, ())| (*k, *v)).collect();
        let oracle_all: Vec<_> = oracle.iter().map(|(&k, &v)| (k, v)).collect();
        prop_assert_eq!(tree_all, oracle_all);
    }


    /// Tree debug output must be consistent with BTreeMap
    #[test]
    fn prop_debug_format(inserts in proptest::collection::vec(
        (any::<i8>().prop_map(i32::from), any::<i8>().prop_map(i32::from)),
        0..20,
    )) {
        let mut tree = AugmentedRBTree::<i32, i32, Unit>::new();
        let mut oracle = BTreeMap::<i32, i32>::new();
        for (k, v) in inserts {
            tree.insert(k, v);
            oracle.insert(k, v);
        }
        // Both should format as {k: v, ...}
        prop_assert_eq!(format!("{tree:?}"), format!("{oracle:?}"));
    }
}
