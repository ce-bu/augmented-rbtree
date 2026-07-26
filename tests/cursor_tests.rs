#![cfg(any(feature = "alloc", feature = "allocator-api", feature = "nightly"))]
#![cfg_attr(feature = "nightly", feature(allocator_api))]

mod helpers;

use augmented_rbtree::{AugmentedRBTreeFactory, SubtreeSize, TreeLocation};
use rand::seq::SliceRandom;

use crate::helpers::common::{
    custom_augment_a::{CustomAugment, CustomKey, CustomValue, reset_drop_loggers},
    test_rng,
};

#[test]
fn test_nav_cursor_advance_and_yield_mechanics() {
    // 1. Build a randomized tree with 39 elements out of 49 (10 intentional gaps)
    let mut tree = AugmentedRBTreeFactory::<SubtreeSize>::new_tree();
    let mut keys = (1..50).collect::<Vec<i32>>();

    let mut rng = test_rng();
    keys.shuffle(&mut rng);
    keys.truncate(39);

    for &key in &keys {
        tree.insert(key, key);
    }

    // Sort keys to assert precise sequential layout movement
    let mut sorted = keys.clone();
    sorted.sort_unstable();

    let min_val = sorted[0];
    let max_val = sorted[sorted.len() - 1];

    // =========================================================================
    // 1. VERIFY INITIAL POSITION HOOKS VIA `.get()`
    // =========================================================================
    // Leftmost and Rightmost must resolve exactly to structural bounds
    let cursor_left = tree.nav_cursor(TreeLocation::Leftmost);
    assert_eq!(cursor_left.get().map(|(k, _, _)| *k), Some(min_val));

    let cursor_right = tree.nav_cursor(TreeLocation::Rightmost);
    assert_eq!(cursor_right.get().map(|(k, _, _)| *k), Some(max_val));

    // =========================================================================
    // 2. VERIFY ADVANCE-AND-YIELD MECHANICS WITH `.next()` AND `.prev()`
    // =========================================================================
    let mut cursor = tree.nav_cursor(TreeLocation::Leftmost);

    // As per doc specs, calling .next() instantly shifts position *before* yielding
    let first_next_yield = cursor.next();
    assert_eq!(first_next_yield.map(|(k, _, _)| *k), Some(sorted[1]));
    assert_eq!(cursor.get().map(|(k, _, _)| *k), Some(sorted[1]));

    // Step completely through the sorted key space to verify consecutive next jumps
    for expected_key in sorted.iter().skip(2) {
        let yielded = cursor.next();
        assert_eq!(yielded.map(|(k, _, _)| *k), Some(*expected_key));
    }

    // Terminal step past max value must force cursor state into None and yield None
    assert!(cursor.next().is_none());
    assert!(cursor.get().is_none()); // State remains broken/None

    // Reset cursor to the end to check backward advance mechanics
    let mut rev_cursor = tree.nav_cursor(TreeLocation::Rightmost);
    assert_eq!(rev_cursor.get().map(|(k, _, _)| *k), Some(max_val));

    for expected_key in sorted.iter().rev().skip(1) {
        let yielded = rev_cursor.prev();
        assert_eq!(yielded.map(|(k, _, _)| *k), Some(*expected_key));
    }

    // Terminal step past min value must break the backward state into None
    assert!(rev_cursor.prev().is_none());
    assert!(rev_cursor.get().is_none());

    // =========================================================================
    // 3. VERIFY PEEK METRIC STABILITY (No Mutation)
    // =========================================================================
    let cursor_peek = tree.nav_cursor(TreeLocation::Leftmost);
    // Peeking must read the neighbor without altering where the current tracker is resting
    assert_eq!(cursor_peek.peek_next().map(|(k, _, _)| *k), Some(sorted[1]));
    assert_eq!(cursor_peek.get().map(|(k, _, _)| *k), Some(min_val));
}

#[test]
fn test_nav_cursor_mut_traversal_and_mutation_mechanics() {
    // 1. Build a randomized tree layout with 39 keys (leaving 10 gaps)
    let mut tree = AugmentedRBTreeFactory::<SubtreeSize>::new_tree();
    let mut keys = (1..50).collect::<Vec<i32>>();

    let mut rng = test_rng();
    keys.shuffle(&mut rng);
    keys.truncate(39);

    for &key in &keys {
        tree.insert(key, key); // Value matches Key initially
    }

    let mut sorted = keys.clone();
    sorted.sort_unstable();

    // =========================================================================
    // 1. TEST PEEKING AND SEQUENTIAL MUTABLE ITERATION
    // =========================================================================
    {
        // Obtain exclusive mutable cursor handle over the tree layout
        let mut cursor_mut = tree.nav_cursor_mut(TreeLocation::Leftmost);

        // Assert initial position matching look-before-you-leap design rules
        assert_eq!(
            cursor_mut.get().map(|node_guard| *node_guard.key()),
            Some(sorted[0])
        );

        // Validate peek transparency without altering state placement
        assert_eq!(
            cursor_mut.peek_next().map(|node_guard| *node_guard.key()),
            Some(sorted[1])
        );
        assert_eq!(
            cursor_mut.get().map(|node_guard| *node_guard.key()),
            Some(sorted[0])
        );

        let mut idx = 0;
        loop {
            // Scope the borrow completely inside this block
            let has_next = match cursor_mut.get() {
                Some(mut node_guard) => {
                    *node_guard.value_mut() = sorted[idx] * 10;
                    idx += 1;
                    true
                }
                None => false,
            }; // <-- `val_guard` is dropped here, releasing the tree layout borrow

            if !has_next {
                break;
            }

            // Now cursor_mut can be mutably borrowed again safely
            cursor_mut.next();
        }
        // Terminal out-of-bounds check
        assert!(cursor_mut.get().is_none());
    }

    // =========================================================================
    // 2. TEST RAII MUTATION INTEGRITY (Verify updates persisted safely)
    // =========================================================================
    {
        let mut cursor_check = tree.nav_cursor(TreeLocation::Leftmost);
        for &expected_key in &sorted {
            let (k, v, _s) = cursor_check.get().unwrap();
            assert_eq!(*k, expected_key);
            assert_eq!(*v, expected_key * 10); // Value was updated mutably
            cursor_check.next();
        }
    }

    // =========================================================================
    // 3. TEST IN-PLACE DELETION WITH AUTOMATIC CURSOR POINTER FORWARDING
    // =========================================================================
    {
        // Position at the middle element of the sorted layout space
        let target_mid_key = sorted[19];
        let expected_next_key = sorted[20];

        let mut cursor_del = tree.nav_cursor_mut(TreeLocation::At(&target_mid_key));
        assert_eq!(
            cursor_del.get().map(|node_guard| *node_guard.key()),
            Some(target_mid_key)
        );

        // Execute removal operation
        let removed_data = cursor_del.remove();
        assert_eq!(removed_data, Some((target_mid_key, target_mid_key * 10)));

        // CRUCIAL: The cursor must automatically slide forward to protect from invalidation
        assert_eq!(
            cursor_del.get().map(|node_guard| *node_guard.key()),
            Some(expected_next_key)
        );
    }

    // =========================================================================
    // 4. TEST SEAMLESS DELETION ITERATION UNTIL EMPTY
    // =========================================================================
    {
        // Reposition at absolute beginning of the remaining elements
        let mut cursor_drain = tree.nav_cursor_mut(TreeLocation::Leftmost);

        // Continually call remove inside a loop until the tree layout empties out
        let mut drain_count = 0;
        while cursor_drain.get().is_some() {
            let res = cursor_drain.remove();
            assert!(res.is_some());
            drain_count += 1;
        }

        // 38 elements total left (39 initial minus the 1 single deletion from step 3)
        assert_eq!(drain_count, 38);
        assert!(tree.nav_cursor(TreeLocation::Leftmost).get().is_none());
    }
}

#[test]
fn test_nav_cursor_topography_and_peeks() {
    // Build a clean randomized tree
    let mut tree = AugmentedRBTreeFactory::<SubtreeSize>::new_tree();
    let keys = vec![20, 10, 30, 5, 15]; // Explicit structure to guarantee depth layout

    for key in keys {
        tree.insert(key, key);
    }

    // Position cursor at the root node (20)
    let mut cursor = tree.nav_cursor(TreeLocation::Root);
    assert_eq!(cursor.get().map(|(k, _, _)| *k), Some(20));

    // =========================================================================
    // 1. TEST PEEK_LEFT / LEFT MOVEMENT
    // =========================================================================
    // Peek left should reveal 10, but cursor remains at 20
    assert_eq!(cursor.peek_left().map(|(k, _, _)| *k), Some(10));
    assert_eq!(cursor.get().map(|(k, _, _)| *k), Some(20));

    // Jump left -> state advances to 10 and yields 10
    assert_eq!(cursor.left().map(|(k, _, _)| *k), Some(10));
    assert_eq!(cursor.get().map(|(k, _, _)| *k), Some(10));

    // =========================================================================
    // 2. TEST PEEK_PARENT / PARENT MOVEMENT
    // =========================================================================
    // From 10, peek parent should reveal 20
    assert_eq!(cursor.peek_parent().map(|(k, _, _)| *k), Some(20));

    // Jump up to parent -> state advances to 20 and yields 20
    assert_eq!(cursor.parent().map(|(k, _, _)| *k), Some(20));
    assert_eq!(cursor.get().map(|(k, _, _)| *k), Some(20));

    // At root, parent should be None (does not change cursor)
    assert!(cursor.peek_parent().is_none());
    assert_eq!(cursor.get().map(|(k, _, _)| *k), Some(20));

    // =========================================================================
    // 3. TEST PEEK_RIGHT / RIGHT MOVEMENT
    // =========================================================================
    // From 20, peek right should reveal 30
    assert_eq!(cursor.peek_right().map(|(k, _, _)| *k), Some(30));

    // Jump right -> state advances to 30 and yields 30
    assert_eq!(cursor.right().map(|(k, _, _)| *k), Some(30));
    assert_eq!(cursor.get().map(|(k, _, _)| *k), Some(30));

    // =========================================================================
    // 4. TEST TERMINAL COLLAPSE ON STRUCTURAL EDGES
    // =========================================================================
    // 30 has no right child. Jumping right should cause terminal None state collapse
    assert!(cursor.peek_right().is_none());
    assert!(cursor.right().is_none());
    assert!(cursor.get().is_none()); // Cursor is now completely invalidated/exhausted
}

#[test]
fn test_nav_cursor_mut_topography_and_peeks() {
    let mut tree = AugmentedRBTreeFactory::<SubtreeSize>::new_tree();
    let keys = vec![20, 10, 30, 5, 15];

    for key in keys {
        tree.insert(key, key);
    }

    let mut cursor_mut = tree.nav_cursor_mut(TreeLocation::Root);
    assert_eq!(
        cursor_mut.get().map(|node_guard| *node_guard.key()),
        Some(20)
    );

    // =========================================================================
    // 1. TEST MUTABLE PEEK_LEFT / LEFT MOVEMENT
    // =========================================================================
    // Scoped block to ensure temporary val_guards from peeks are fully dropped
    {
        assert_eq!(
            cursor_mut.peek_left().map(|node_guard| *node_guard.key()),
            Some(10)
        );
    }

    // Left method moves cursor to 10
    {
        assert_eq!(
            cursor_mut.left().map(|node_guard| *node_guard.key()),
            Some(10)
        );
        assert_eq!(
            cursor_mut.get().map(|node_guard| *node_guard.key()),
            Some(10)
        );
    }

    // =========================================================================
    // 2. TEST MUTABLE PEEK_PARENT / PARENT MOVEMENT
    // =========================================================================
    {
        assert_eq!(
            cursor_mut.peek_parent().map(|node_guard| *node_guard.key()),
            Some(20)
        );
    }
    {
        assert_eq!(
            cursor_mut.parent().map(|node_guard| *node_guard.key()),
            Some(20)
        );
        assert_eq!(
            cursor_mut.get().map(|node_guard| *node_guard.key()),
            Some(20)
        );
    }

    // =========================================================================
    // 3. TEST MUTABLE PEEK_RIGHT / RIGHT MOVEMENT
    // =========================================================================
    {
        assert_eq!(
            cursor_mut.peek_right().map(|node_guard| *node_guard.key()),
            Some(30)
        );
    }
    {
        assert_eq!(
            cursor_mut.right().map(|node_guard| *node_guard.key()),
            Some(30)
        );
        assert_eq!(
            cursor_mut.get().map(|node_guard| *node_guard.key()),
            Some(30)
        );
    }

    // =========================================================================
    // 4. TEST TERMINAL COLLAPSE ON STRUCTURAL EDGES
    // =========================================================================
    {
        assert!(cursor_mut.peek_right().is_none());
    }
    {
        assert!(cursor_mut.right().is_none());
        assert!(cursor_mut.get().is_none()); // Becomes None
    }
}

#[test]
#[allow(clippy::clone_on_copy)]
fn test_nav_cursor_topography_prev_and_clone() {
    let mut tree = AugmentedRBTreeFactory::<SubtreeSize>::new_tree();
    // Use an explicit layout to ensure a known structural topography
    let keys = vec![20, 10, 30];
    for key in keys {
        tree.insert(key, key);
    }

    // Position at Rightmost (30) to test backward sequencing
    let mut cursor = tree.nav_cursor(TreeLocation::Rightmost);
    assert_eq!(cursor.get().map(|(k, _, _)| *k), Some(30));

    // =========================================================================
    // 1. TEST PEEK_PREV / PREV MOVEMENT
    // =========================================================================
    // Peeking backward should reveal 20, leaving the cursor unmoved at 30
    assert_eq!(cursor.peek_prev().map(|(k, _, _)| *k), Some(20));
    assert_eq!(cursor.get().map(|(k, _, _)| *k), Some(30));

    // Jump backward -> state advances to 20 and yields 20
    assert_eq!(cursor.prev().map(|(k, _, _)| *k), Some(20));
    assert_eq!(cursor.get().map(|(k, _, _)| *k), Some(20));

    // =========================================================================
    // 2. TEST CLONE TRAIT ISOLATION
    // =========================================================================
    // Clone the cursor while it is sitting exactly on 20
    let mut cloned_cursor = cursor.clone();
    assert_eq!(cloned_cursor.get().map(|(k, _, _)| *k), Some(20));

    // Move the original cursor forward; the clone must remain independent on 20
    assert_eq!(cursor.next().map(|(k, _, _)| *k), Some(30));
    assert_eq!(cloned_cursor.get().map(|(k, _, _)| *k), Some(20));

    // Move the clone backward to 10; original cursor stays unaffected on 30
    assert_eq!(cloned_cursor.prev().map(|(k, _, _)| *k), Some(10));
    assert_eq!(cursor.get().map(|(k, _, _)| *k), Some(30));

    // =========================================================================
    // 3. TOPOGRAPHY & TERMINAL COLLAPSE FOR PREV
    // =========================================================================
    // From 10, peeking left child should be None
    assert_eq!(cloned_cursor.get().map(|(k, _, _)| *k), Some(10));
    assert!(cloned_cursor.peek_left().is_none());

    // 10 is the minimum element, so stepping prev again must collapse to None
    assert!(cloned_cursor.peek_prev().is_none());
    assert!(cloned_cursor.prev().is_none());
    assert!(cloned_cursor.get().is_none());
}

#[test]
fn test_nav_cursor_mut_topography_and_prev() {
    let mut tree = AugmentedRBTreeFactory::<SubtreeSize>::new_tree();
    let keys = vec![20, 10, 30];
    for key in keys {
        tree.insert(key, key);
    }

    // Start at the maximum element (30)
    let mut cursor_mut = tree.nav_cursor_mut(TreeLocation::Rightmost);
    assert_eq!(
        cursor_mut.get().map(|node_guard| *node_guard.key()),
        Some(30)
    );

    // =========================================================================
    // 1. TEST MUTABLE PEEK_PREV
    // =========================================================================
    {
        // Peek backward reveals 20 without moving from 30
        assert_eq!(
            cursor_mut.peek_prev().map(|node_guard| *node_guard.key()),
            Some(20)
        );
    }
    assert_eq!(
        cursor_mut.get().map(|node_guard| *node_guard.key()),
        Some(30)
    );

    // =========================================================================
    // 2. TEST MUTABLE PREV MOVEMENT
    // =========================================================================
    {
        // Move backward to 20
        assert_eq!(
            cursor_mut.prev().map(|node_guard| *node_guard.key()),
            Some(20)
        );
    }
    assert_eq!(
        cursor_mut.get().map(|node_guard| *node_guard.key()),
        Some(20)
    );

    {
        // Move backward again to 10
        assert_eq!(
            cursor_mut.prev().map(|node_guard| *node_guard.key()),
            Some(10)
        );
    }
    assert_eq!(
        cursor_mut.get().map(|node_guard| *node_guard.key()),
        Some(10)
    );

    // =========================================================================
    // 3. TERMINAL COLLAPSE ON PREV
    // =========================================================================
    {
        // 10 is the absolute minimum element; peeking backward yields None
        assert!(cursor_mut.peek_prev().is_none());
    }
    {
        // Moving backward past the minimum forces state collapse to None
        assert!(cursor_mut.prev().is_none());
    }
    assert!(cursor_mut.get().is_none());
}

#[test]
fn check_cursor_bounds_with_custom_key_value_stats() {
    reset_drop_loggers();
    let mut tree = AugmentedRBTreeFactory::<CustomAugment>::new_tree();
    tree.insert(CustomKey(1), CustomValue("one".to_string()));
    tree.insert(CustomKey(2), CustomValue("two".to_string()));
    tree.insert(CustomKey(3), CustomValue("three".to_string()));

    let cursor = tree.nav_cursor(TreeLocation::At(&CustomKey(1)));
    assert_eq!(cursor.get().map(|nav| nav.0), Some(&CustomKey(1)));
}
