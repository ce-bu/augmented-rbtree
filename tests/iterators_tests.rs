#![cfg(any(feature = "alloc", feature = "allocator-api", feature = "nightly"))]
#![cfg_attr(feature = "nightly", feature(allocator_api))]

mod helpers;

use std::iter::repeat_with;

use augmented_rbtree::{AugmentedRBTree, SubtreeSize};
use itertools::Itertools;
use rand::RngExt;

use crate::helpers::common::test_rng;

#[test]
fn check_iter() {
    let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new();
    let mut rng = test_rng();

    let keys: Vec<i32> = repeat_with(|| rng.random_range(1..100))
        .unique()
        .take(10)
        .collect();
    for &key in &keys {
        tree.insert(key, key);
    }

    for (key, value, _stats) in &tree {
        assert!(keys.contains(key));
        assert_eq!(tree.get(key), Some(value));
    }

    for (key, value, _stats) in tree.iter().rev() {
        assert!(keys.contains(key));
        assert_eq!(tree.get(key), Some(value));
    }

    assert_eq!(tree.iter().len(), keys.len());

    let arr = tree.iter().collect::<Vec<_>>();
    assert_eq!(arr.len(), keys.len());
}

#[test]
fn check_iter_mut() {
    let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new();
    let mut rng = test_rng();

    let keys: Vec<i32> = repeat_with(|| rng.random_range(1..100))
        .unique()
        .take(10)
        .collect();
    for &key in &keys {
        tree.insert(key, key);
    }

    for (key, mut value, _stats) in &mut tree {
        assert!(keys.contains(key));
        *value += 1000; // Modify the value mutably
    }

    for (key, mut value, _stats) in tree.iter_mut().rev() {
        assert!(keys.contains(key));
        assert!(*value > 1000);
        *value -= 1000;
    }

    assert_eq!(tree.iter_mut().len(), keys.len());

    for &key in &keys {
        assert_eq!(tree.get(&key), Some(&key));
    }
}

#[test]
fn check_iter_mut_collect() {
    let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new();
    let mut rng = test_rng();

    let keys: Vec<i32> = repeat_with(|| rng.random_range(1..100))
        .unique()
        .take(10)
        .collect();
    for &key in &keys {
        tree.insert(key, key);
    }

    let arr = tree.iter_mut().collect::<Vec<_>>();
    assert_eq!(arr.len(), keys.len());
}

fn change_mut_node_value<V: AsMut<i32>>(mut val: V) {
    *val.as_mut() += 1;
}

#[test]
fn check_valmut_explicit_reborrow() {
    let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new();
    let keys = vec![10, 20, 30];
    for &key in &keys {
        tree.insert(key, key);
    }
    for (_key, mut val, _stats) in &mut tree {
        change_mut_node_value(&mut val);
        *val += 1;
    }
    for &key in &keys {
        assert_eq!(tree.get(&key).copied(), Some(key + 2));
    }
}

#[test]
fn check_back_field_isolated() {
    let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new();
    tree.insert(10, 100);
    tree.insert(20, 200);

    let mut iter = tree.iter_mut();

    assert_eq!(iter.len(), 2);

    let (key_back, _, _) = iter.next_back().unwrap();
    assert_eq!(*key_back, 20);
    assert_eq!(iter.len(), 1);

    let (key_front, _, _) = iter.next().unwrap();
    assert_eq!(*key_front, 10);
    assert_eq!(iter.len(), 0);

    assert!(iter.next_back().is_none());
    assert!(iter.next().is_none());
}

#[test]
fn check_into_iter() {
    let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new();
    let mut rng = test_rng();
    let keys: Vec<i32> = repeat_with(|| rng.random_range(1..1000))
        .unique()
        .take(200)
        .collect();
    for &key in &keys {
        tree.insert(key, key);
    }
    let cloned_tree_for_lookup = tree.clone();
    for (key, value) in tree {
        assert!(keys.contains(&key));
        // Use the clone for lookups because `tree` is now moved/consumed
        assert_eq!(cloned_tree_for_lookup.get(&key), Some(&value));
    }
}

#[test]
fn check_into_iter_partial_drop() {
    let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new();
    let mut rng = test_rng();

    let keys: Vec<i32> = repeat_with(|| rng.random_range(1..100))
        .unique()
        .take(10)
        .collect();
    for &key in &keys {
        tree.insert(key, key);
    }

    let cloned_tree_for_lookup = tree.clone();

    // Force an early break after processing 3 nodes
    let mut count = 0;
    for (key, value) in tree {
        assert!(keys.contains(&key));
        assert_eq!(cloned_tree_for_lookup.get(&key), Some(&value));

        count += 1;
        if count == 3 {
            // BREAK EARLY: This leaves 7 nodes unconsumed inside the iterator!
            // When we break, the loop scope ends, forcing IntoIter's Drop to run.
            break;
        }
    }

    // MIRI will now actively scan the remaining 7 unconsumed nodes
    // to ensure they are cleaned up cleanly by your internal teardown code.
}

mod range_tests {
    use core::ops::Bound;

    use augmented_rbtree::{AugmentedRBTree, SubtreeSize};

    fn setup_test_tree() -> AugmentedRBTree<i32, String, SubtreeSize> {
        let mut tree = AugmentedRBTree::new();
        tree.insert(20, "twenty".to_string());
        tree.insert(10, "ten".to_string());
        tree.insert(30, "thirty".to_string());
        tree.insert(15, "fifteen".to_string());
        tree.insert(25, "twenty-five".to_string());
        tree
    }

    // ========================================================================
    // 1. Coverage for range_bounds_to_nodes (Start/End Bound combinations)
    // ========================================================================

    #[test]
    fn test_bounds_unbounded_to_unbounded() {
        let tree = setup_test_tree();
        // Branch: Bound::Unbounded for both start and end
        let range = tree.range(..);
        let items: Vec<&i32> = range.map(|(k, _, _)| k).collect();
        assert_eq!(items, vec![&10, &15, &20, &25, &30]);
    }

    #[test]
    fn test_bounds_included_to_included() {
        let tree = setup_test_tree();
        // Branch: Bound::Included for both boundaries matching exact existing keys
        let range = tree.range(10..=25);
        let items: Vec<&i32> = range.map(|(k, _, _)| k).collect();
        assert_eq!(items, vec![&10, &15, &20, &25]);
    }

    #[test]
    fn test_bounds_excluded_to_excluded() {
        let tree = setup_test_tree();
        // Branch: Bound::Excluded for both boundaries matching exact existing keys
        // Triggers the nested layout logic: `if unsafe { n.key() }.borrow() == k` -> true
        let range = tree.range((Bound::Excluded(10), Bound::Excluded(25)));
        let items: Vec<&i32> = range.map(|(k, _, _)| k).collect();
        assert_eq!(items, vec![&15, &20]);
    }

    #[test]
    fn test_bounds_excluded_missing_keys() {
        let tree = setup_test_tree();
        // Branch: Bound::Excluded where boundary values DO NOT match keys present in the tree
        // Triggers the else branches: `if unsafe { n.key() }.borrow() == k` -> false
        let range = tree.range((Bound::Excluded(12), Bound::Excluded(27)));
        let items: Vec<&i32> = range.map(|(k, _, _)| k).collect();
        assert_eq!(items, vec![&15, &20, &25]);
    }

    // ========================================================================
    // 2. Coverage for Empty/Exhausted/Out-of-Bounds Branch Scenarios
    // ========================================================================

    #[test]
    #[allow(clippy::reversed_empty_ranges)]
    fn test_bounds_exhausted_inverted_range() {
        let tree = setup_test_tree();
        // Branch: front is past back (30 > 10) -> exhausted becomes true instantly
        let mut range = tree.range(30..=10);
        assert!(range.next().is_none());
        assert!(range.next_back().is_none());
    }

    #[test]
    fn test_bounds_exhausted_out_of_tree_range() {
        let tree = setup_test_tree();
        // Branch: (Some(_), None) or (None, _) condition scenarios
        let mut range = tree.range(50..100);
        assert!(range.next().is_none());
    }

    // ========================================================================
    // 3. Bidirectional DoubleEndedIterator Coverage (Range)
    // ========================================================================

    #[test]
    fn test_range_bidirectional_meeting_in_middle() {
        let tree = setup_test_tree();
        let mut range = tree.range(10..=30);

        // Advance front
        let f1 = range.next().unwrap();
        assert_eq!(f1.0, &10); // front is now 15

        // Advance back
        let b1 = range.next_back().unwrap();
        assert_eq!(b1.0, &30); // back is now 25

        // Advance front again
        let f2 = range.next().unwrap();
        assert_eq!(f2.0, &15); // front is now 20

        // Advance back again
        let b2 = range.next_back().unwrap();
        assert_eq!(b2.0, &25); // back is now 20

        // Branch condition check: front == back (both pointing to key 20)
        let f3 = range.next().unwrap();
        assert_eq!(f3.0, &20); // Hits `self.front == self.back` -> sets exhausted = true

        // Fused Iterator Branch Check: subsequent calls return None instantly
        assert!(range.next().is_none());
        assert!(range.next_back().is_none());
    }

    #[test]
    fn test_range_back_exhaustion_first() {
        let tree = setup_test_tree();
        let mut range = tree.range(15..=20);

        // Call next_back until it exhausts from the reverse side
        assert_eq!(range.next_back().unwrap().0, &20); // front=15, back=15
        assert_eq!(range.next_back().unwrap().0, &15); // front=15, back=15 -> sets exhausted = true
        assert!(range.next_back().is_none());
        assert!(range.next().is_none());
    }

    // ========================================================================
    // 4. Full Mutable Coverage (RangeMut Iterator)
    // ========================================================================

    #[test]
    fn test_range_mut_forward_and_backward() {
        let mut tree = setup_test_tree();

        // Setup mut range block boundary to isolate updates
        {
            let mut range_mut = tree.range_mut(15..=25);

            // Mutate via forward iteration
            if let Some((key, mut val, _stats)) = range_mut.next() {
                assert_eq!(key, &15);
                val.push_str("_mut1");
            }

            // Mutate via backward iteration to verify next_back for RangeMut
            if let Some((key, mut val, _stats)) = range_mut.next_back() {
                assert_eq!(key, &25);
                val.push_str("_mut2");
            }

            // Meet at node 20
            if let Some((key, mut val, _stats)) = range_mut.next() {
                assert_eq!(key, &20);
                val.push_str("_mut3");
            }

            // Ensure Fused/Exhausted branch triggers on RangeMut
            assert!(range_mut.next().is_none());
            assert!(range_mut.next_back().is_none());
        }

        // Verify mutations were successfully committed back into tree nodes
        assert_eq!(tree.get(&15).unwrap(), "fifteen_mut1");
        assert_eq!(tree.get(&20).unwrap(), "twenty_mut3");
        assert_eq!(tree.get(&25).unwrap(), "twenty-five_mut2");
    }
}

mod into_iter_tests {
    use augmented_rbtree::{AugmentedRBTree, SubtreeSize};

    // Helper to generate a structured testing tree
    fn setup_tree() -> AugmentedRBTree<i32, String, SubtreeSize> {
        let mut tree = AugmentedRBTree::new();
        tree.insert(30, "thirty".to_string());
        tree.insert(10, "ten".to_string());
        tree.insert(20, "twenty".to_string());
        tree.insert(40, "forty".to_string());
        tree
    }

    #[test]
    fn test_into_iter_double_ended() {
        let tree = setup_tree();

        // 1. Consume the tree into its IntoIter representation
        let mut into_iter = tree.into_iter();

        // Total elements = 4 (In-order: 10, 20, 30, 40)

        // 2. Pull from the FRONT (Iterator)
        let front1 = into_iter.next().unwrap();
        assert_eq!(front1.0, 10);
        assert_eq!(front1.1, "ten");

        // 3. Pull from the BACK (DoubleEndedIterator)
        let back1 = into_iter.next_back().unwrap();
        assert_eq!(back1.0, 40);
        assert_eq!(back1.1, "forty");

        // 4. Pull from the BACK again
        let back2 = into_iter.next_back().unwrap();
        assert_eq!(back2.0, 30);
        assert_eq!(back2.1, "thirty");

        // 5. Pull the final remaining element from the FRONT
        let front2 = into_iter.next().unwrap();
        assert_eq!(front2.0, 20);
        assert_eq!(front2.1, "twenty");

        // 6. Verify exhaustion/fused behavior on both boundaries
        assert!(into_iter.next().is_none());
        assert!(into_iter.next_back().is_none());
    }

    #[test]
    fn test_into_iter_partial_iteration_drop() {
        // Track drops using your favorite custom tracking mechanism or ensure no leak
        let tree = setup_tree();
        let mut into_iter = tree.into_iter();

        // Only consume half the elements
        assert_eq!(into_iter.next().unwrap().0, 10);
        assert_eq!(into_iter.next_back().unwrap().0, 40);

        // Dropping `into_iter` here must safely clean up nodes 20 and 30
        core::mem::drop(into_iter);
    }

    #[test]
    fn test_into_iter_pointers_meet() {
        let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new();

        // 1. Insert exactly 3 elements to cleanly observe the pointers meeting
        tree.insert(10, 10);
        tree.insert(20, 20);
        tree.insert(30, 30);

        // Convert to IntoIter (consuming the tree)
        let mut into_iter = tree.into_iter();

        // 2. Advance the front pointer to the middle element (20)
        // self.next now points to node 20
        let first = into_iter.next();
        assert_eq!(first.map(|(k, _)| k), Some(10));

        // 3. Advance the back pointer to the middle element (20)
        // self.back now points to node 20
        let last = into_iter.next_back();
        assert_eq!(last.map(|(k, _)| k), Some(30));

        // CRITICAL STATE: self.next == self.back (both point to node 20)
        // This next call will execute your crossed-paths branch!
        let middle = into_iter.next_back();
        assert_eq!(middle.map(|(k, _)| k), Some(20));

        // 4. Verify everything is fully drained and cleaned up
        assert_eq!(into_iter.len(), 0);
        assert!(into_iter.next().is_none());
        assert!(into_iter.next_back().is_none());
    }
}

#[test]
fn check_into_iter_collect() {
    let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new();
    let mut rng = test_rng();

    let keys: Vec<i32> = repeat_with(|| rng.random_range(1..100))
        .unique()
        .take(10)
        .collect();
    for &key in &keys {
        tree.insert(key, key);
    }

    let arr = tree.into_iter().collect::<Vec<_>>();
    assert_eq!(arr.len(), keys.len());
}

#[test]
fn check_values_iterator() {
    let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new();
    let mut rng = test_rng();

    let keys: Vec<i32> = repeat_with(|| rng.random_range(1..100))
        .unique()
        .take(10)
        .collect();
    for &key in &keys {
        tree.insert(key, key);
    }

    for value in tree.values() {
        assert!(keys.contains(value));
    }
}

#[test]
fn test_keys_double_ended_and_exact_size() {
    let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new();

    // Insert 3 distinct elements
    tree.insert(10, 100);
    tree.insert(20, 200);
    tree.insert(30, 300);

    // Get the Keys iterator
    let mut keys_iter = tree.keys();

    // 1. Test ExactSizeIterator initially
    assert_eq!(keys_iter.len(), 3);

    // 2. Test DoubleEndedIterator by pulling from the back
    // This calls your Keys::next_back() implementation under the hood
    assert_eq!(keys_iter.next_back(), Some(&30));

    // Check that length correctly decremented to 2
    assert_eq!(keys_iter.len(), 2);

    // 3. Alternate between front and back to thoroughly test the state
    assert_eq!(keys_iter.next(), Some(&10));
    assert_eq!(keys_iter.len(), 1);

    assert_eq!(keys_iter.next_back(), Some(&20));
    assert_eq!(keys_iter.len(), 0);

    // 4. Verify bounds and empty state behavior
    assert_eq!(keys_iter.next_back(), None);
    assert_eq!(keys_iter.len(), 0);
}

#[test]
fn test_value_double_ended_and_exact_size() {
    let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new();

    // Insert 3 distinct elements
    tree.insert(10, 100);
    tree.insert(20, 200);
    tree.insert(30, 300);

    // Get the Values iterator
    let mut values_iter = tree.values();

    // 1. Test ExactSizeIterator initially
    assert_eq!(values_iter.len(), 3);

    // 2. Test DoubleEndedIterator by pulling from the back
    // This calls your Values::next_back() implementation under the hood
    assert_eq!(values_iter.next_back(), Some(&300));

    // Check that length correctly decremented to 2
    assert_eq!(values_iter.len(), 2);

    // 3. Alternate between front and back to thoroughly test the state
    assert_eq!(values_iter.next(), Some(&100));
    assert_eq!(values_iter.len(), 1);

    assert_eq!(values_iter.next_back(), Some(&200));
    assert_eq!(values_iter.len(), 0);

    // 4. Verify bounds and empty state behavior
    assert_eq!(values_iter.next_back(), None);
    assert_eq!(values_iter.len(), 0);
}

#[test]
fn test_values_mut_all_traits() {
    let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new();

    // Insert 3 distinct elements
    tree.insert(10, 100);
    tree.insert(20, 200);
    tree.insert(30, 300);

    // Get the ValuesMutInt iterator
    let mut values_mut_iter = tree.values_mut();

    // 1. Test ExactSizeIterator & Iterator::size_hint initially
    assert_eq!(values_mut_iter.len(), 3);
    assert_eq!(values_mut_iter.size_hint(), (3, Some(3)));

    // 2. Test DoubleEndedIterator::next_back
    // This extracts the value wrapper from the back (key 30 -> value 300)
    if let Some(mut val_mut) = values_mut_iter.next_back() {
        // Assuming ValMut dereferences mutably or has a method to update the value
        // Update 300 to 350 to verify mutability works
        *val_mut = 350;
    } else {
        panic!("Expected a value from next_back");
    }

    // Verify size tracking updated correctly
    assert_eq!(values_mut_iter.len(), 2);
    assert_eq!(values_mut_iter.size_hint(), (2, Some(2)));

    // 3. Test Iterator::next
    // This extracts from the front (key 10 -> value 100)
    if let Some(mut val_mut) = values_mut_iter.next() {
        *val_mut = 150;
    } else {
        panic!("Expected a value from next");
    }

    // 4. Consume the remaining middle element (key 20 -> value 200)
    assert!(values_mut_iter.next().is_some());
    assert_eq!(values_mut_iter.len(), 0);

    // 5. Test FusedIterator behavior
    // Once empty, repeated calls to next/next_back must consistently return None
    assert!(values_mut_iter.next().is_none());
    assert!(values_mut_iter.next_back().is_none());
    assert!(values_mut_iter.next().is_none());

    // 6. Verify that mutations successfully persisted back into the tree
    let final_values: Vec<_> = tree.values().collect();
    assert_eq!(final_values, vec![&150, &200, &350]);
}

#[test]
fn test_stats_iterator_all_traits() {
    let mut tree = AugmentedRBTree::<i32, i32, SubtreeSize>::new();

    // Insert 3 distinct elements
    tree.insert(10, 100);
    tree.insert(20, 200);
    tree.insert(30, 300);

    // Get the Stats iterator (assuming a .stats() method exists on the tree)
    let mut stats_iter = tree.stats();

    // 1. Test ExactSizeIterator & Iterator::size_hint initially
    assert_eq!(stats_iter.len(), 3);
    assert_eq!(stats_iter.size_hint(), (3, Some(3)));

    // 2. Test DoubleEndedIterator::next_back
    // This extracts the stats from the back (key 30)
    let last_stats = stats_iter.next_back();
    assert!(last_stats.is_some());

    // Verify size tracking updated correctly after pulling from the back
    assert_eq!(stats_iter.len(), 2);
    assert_eq!(stats_iter.size_hint(), (2, Some(2)));

    // 3. Test Iterator::next
    // This extracts from the front (key 10)
    let first_stats = stats_iter.next();
    assert!(first_stats.is_some());

    // 4. Consume the remaining middle element (key 20)
    assert!(stats_iter.next().is_some());
    assert_eq!(stats_iter.len(), 0);

    // 5. Test FusedIterator behavior
    // Repeated calls to next/next_back must cleanly and consistently return None
    assert!(stats_iter.next().is_none());
    assert!(stats_iter.next_back().is_none());
    assert!(stats_iter.next().is_none());
}
