#![cfg(any(feature = "alloc", feature = "allocator-api", feature = "nightly"))]
#![cfg_attr(feature = "nightly", feature(allocator_api))]

mod helpers;

use augmented_rbtree::{AugmentedRBTree, Entry, SubtreeSize};

// Helper function to initialize a standard base tree for testing
fn setup_empty_tree() -> AugmentedRBTree<String, i32, SubtreeSize> {
    AugmentedRBTree::new()
}

// ========================================================================
// 1. Entry Enum Operations & General Methods
// ========================================================================

#[test]
fn test_entry_key_retrieval() {
    let mut tree = setup_empty_tree();

    // Scenario A: Key method on a Vacant entry
    let vacant_entry = tree.entry("vacant_key".to_string());
    assert_eq!(vacant_entry.key(), "vacant_key");
    vacant_entry.or_insert(100);

    // Scenario B: Key method on an Occupied entry
    let occupied_entry = tree.entry("vacant_key".to_string());
    assert_eq!(occupied_entry.key(), "vacant_key");
}

#[test]
fn test_entry_and_modify() {
    let mut tree = setup_empty_tree();

    // Path A: and_modify called on a Vacant entry (should do nothing)
    let entry = tree
        .entry("key".to_string())
        .and_modify(|v| *v += 10)
        .or_insert(42);
    assert_eq!(*entry, 42);

    // Path B: and_modify called on an Occupied entry (should execute the closure)
    let entry = tree
        .entry("key".to_string())
        .and_modify(|v| *v += 10)
        .or_insert(100); // 100 is skipped because entry is occupied
    assert_eq!(*entry, 52);
    assert_eq!(tree.get(&"key".to_string()), Some(&52));
}

#[test]
fn test_or_insert_variants() {
    let mut tree = setup_empty_tree();

    // 1. Test `or_insert`
    assert_eq!(*tree.entry("a".to_string()).or_insert(1), 1); // Vacant path
    assert_eq!(*tree.entry("a".to_string()).or_insert(99), 1); // Occupied path

    // 2. Test `or_insert_with`
    assert_eq!(*tree.entry("b".to_string()).or_insert_with(|| 2), 2); // Vacant path
    assert_eq!(*tree.entry("b".to_string()).or_insert_with(|| 99), 2); // Occupied path

    // 3. Test `or_default`
    assert_eq!(*tree.entry("c".to_string()).or_default(), 0); // Vacant path
    // Fill item with something non-default
    *tree.get_mut(&"c".to_string()).unwrap() = 5;
    assert_eq!(*tree.entry("c".to_string()).or_default(), 5); // Occupied path
}

// ========================================================================
// 2. OccupiedEntry Specific Methods
// ========================================================================

#[test]
fn test_occupied_entry_manipulation() {
    let mut tree = setup_empty_tree();
    tree.insert("target".to_string(), 10);

    if let Entry::Occupied(mut entry) = tree.entry("target".to_string()) {
        // Test key() and get()
        assert_eq!(entry.key(), "target");
        assert_eq!(entry.get(), &10);

        // Test get_mut()
        *entry.get_mut() += 5;
        assert_eq!(entry.get(), &15);

        // Test insert() (overwrites and returns the old value)
        let old_val = entry.insert(42);
        assert_eq!(old_val, 15);
        assert_eq!(entry.get(), &42);

        // Test into_mut() (consumes entry, extends lifetime to tree reference)
        let val_ref: &mut i32 = entry.into_mut();
        *val_ref = 100;
    } else {
        panic!("Expected entry to be Occupied");
    }

    assert_eq!(tree.get(&"target".to_string()), Some(&100));
}

#[test]
fn test_occupied_entry_remove() {
    let mut tree = setup_empty_tree();
    tree.insert("remove_me".to_string(), 500);
    assert_eq!(tree.len(), 1);

    if let Entry::Occupied(entry) = tree.entry("remove_me".to_string()) {
        // Test remove()
        let value = entry.remove();
        assert_eq!(value, 500);
    } else {
        panic!("Expected entry to be Occupied");
    }

    // Verify structure is entirely cleared out and len counter dropped down
    assert_eq!(tree.len(), 0);
    assert!(tree.get(&"remove_me".to_string()).is_none());
}

// ========================================================================
// 3. VacantEntry Specific Methods
// ========================================================================

#[test]
fn test_vacant_entry_methods() {
    let mut tree = setup_empty_tree();

    // 1. Test key() on VacantEntry
    if let Entry::Vacant(entry) = tree.entry("vacant".to_string()) {
        assert_eq!(entry.key(), "vacant");
    } else {
        panic!("Expected entry to be Vacant");
    }

    // 2. Test into_key() on VacantEntry
    if let Entry::Vacant(entry) = tree.entry("into_key_test".to_string()) {
        let extracted_key = entry.into_key();
        assert_eq!(extracted_key, "into_key_test");
    } else {
        panic!("Expected entry to be Vacant");
    }

    // 3. Test try_insert() on VacantEntry
    if let Entry::Vacant(entry) = tree.entry("try_insert_test".to_string()) {
        let insert_res = entry.try_insert(777);
        assert!(insert_res.is_ok());
        let val_ref = insert_res.unwrap();
        assert_eq!(*val_ref, 777);
    } else {
        panic!("Expected entry to be Vacant");
    }

    assert_eq!(tree.get(&"try_insert_test".to_string()), Some(&777));
    assert_eq!(tree.len(), 1);
}

#[test]
fn test_vacant_entry_graceful_oom() {
    use crate::helpers::limited_allocator::LimitedAllocator;

    let allocator = LimitedAllocator::new(1, 1024);
    let mut bounded_tree =
        AugmentedRBTree::<String, i32, SubtreeSize, LimitedAllocator>::new_in(allocator);

    // 1. Fill up the single available slot
    bounded_tree.insert("item1".to_string(), 10);

    // 2. Safely verify that try_insert surfaces the OutOfMemoryError gracefully
    if let Entry::Vacant(entry) = bounded_tree.entry("item2".to_string()) {
        let result = entry.try_insert(20);
        assert!(result.is_err());
    } else {
        panic!("Expected entry to be Vacant");
    }
}
