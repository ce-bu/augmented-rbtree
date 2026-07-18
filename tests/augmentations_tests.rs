use augmented_rbtree::{
    Augment, IntervalMaxEnd, MinAugmentation,
    augmentations::{MaxAugmentation, SubtreeSize, SumAugmentation},
};

#[test]
fn test_subtree_size_augmentation() {
    // Leaf node case (both children are None)
    let leaf_size = <SubtreeSize as Augment<i32, &str>>::compute(&10, &"val", None, None);
    assert_eq!(leaf_size, 1);

    // Fully populated internal parent node layout setup
    let mock_left_child = (&5, &"left_val", &3);
    let mock_right_child = (&15, &"right_val", &4);

    let parent_size = <SubtreeSize as Augment<i32, &str>>::compute(
        &10,
        &"val",
        Some(mock_left_child),
        Some(mock_right_child),
    );

    assert_eq!(parent_size, 8);
}

#[test]
fn test_sum_augmentation() {
    // Leaf node case
    let leaf_sum = <SumAugmentation as Augment<&str, i64>>::compute(&"key", &15i64, None, None);
    assert_eq!(leaf_sum, 15);

    // Complete internal parent aggregation step
    let mock_left_child = (&"k1", &10i64, &50i64);
    let mock_right_child = (&"k3", &20i64, &100i64);

    let parent_sum = <SumAugmentation as Augment<&str, i64>>::compute(
        &"k2",
        &30i64,
        Some(mock_left_child),
        Some(mock_right_child),
    );

    assert_eq!(parent_sum, 180);
}

#[test]
fn test_max_augmentation() {
    // Leaf node case
    let leaf_max = <MaxAugmentation as Augment<i32, i32>>::compute(&10, &42, None, None);
    assert_eq!(leaf_max, Some(42));

    // Parent node containing a lower value than its left child maximum
    let mock_left_child = (&1, &5, &Some(100));
    let mock_right_child = (&3, &2, &Some(50));

    let parent_max = <MaxAugmentation as Augment<i32, i32>>::compute(
        &2,
        &12,
        Some(mock_left_child),
        Some(mock_right_child),
    );

    assert_eq!(parent_max, Some(100));
}

#[test]
fn test_min_augmentation() {
    // Leaf node case
    let leaf_min = <MinAugmentation as Augment<i32, i32>>::compute(&10, &7, None, None);
    assert_eq!(leaf_min, Some(7));

    // Parent node containing a higher value than its right child minimum
    let mock_left_child = (&1, &20, &Some(15));
    let mock_right_child = (&3, &5, &Some(2));

    let parent_min = <MinAugmentation as Augment<i32, i32>>::compute(
        &2,
        &10,
        Some(mock_left_child),
        Some(mock_right_child),
    );

    assert_eq!(parent_min, Some(2));
}

#[test]
fn test_interval_max_end_augmentation() {
    // Leaf node case
    let leaf_max_end = <IntervalMaxEnd as Augment<i32, i32>>::compute(&5, &12, None, None);
    assert_eq!(leaf_max_end, Some(12));

    // Branching condition verification
    let mock_left_child = (&1, &15, &Some(90));
    let mock_right_child = (&10, &25, &Some(45));

    let parent_max_end = <IntervalMaxEnd as Augment<i32, i32>>::compute(
        &4,
        &30,
        Some(mock_left_child),
        Some(mock_right_child),
    );

    assert_eq!(parent_max_end, Some(90));
}

#[test]
fn test_max_augmentation_hits_right_child_branch() {
    // Left subtree has a max value of 50
    let mock_left_child = (&1, &5, &Some(50));

    // Right subtree has a global max value of 200 (This forces the branch execution!)
    let mock_right_child = (&3, &2, &Some(200));

    let parent_max = <MaxAugmentation as Augment<i32, i32>>::compute(
        &2,
        &12, // Current node value is smaller than child maxima
        Some(mock_left_child),
        Some(mock_right_child),
    );

    // This assertion guarantees that the right branch successfully overwrote `max`
    assert_eq!(parent_max, Some(200));
}

#[test]
fn test_min_augmentation_hits_left_child_branch() {
    // Left subtree has the global minimum value of 2 (This forces the branch execution!)
    let mock_left_child = (&1, &5, &Some(2));

    // Right subtree has a minimum value of 50
    let mock_right_child = (&3, &12, &Some(50));

    let parent_min = <MinAugmentation as Augment<i32, i32>>::compute(
        &2,
        &15, // Current node value is larger than child minima
        Some(mock_left_child),
        Some(mock_right_child),
    );

    // This assertion guarantees that the left branch successfully overwrote `min`
    assert_eq!(parent_min, Some(2));
}

#[test]
fn test_interval_max_end_hits_right_child_branch() {
    // Left subtree has a max endpoint of 45
    let mock_left_child = (&1, &15, &Some(45));

    // Right subtree has the global max endpoint of 120 (This forces the branch execution!)
    let mock_right_child = (&10, &25, &Some(120));

    let parent_max_end = <IntervalMaxEnd as Augment<i32, i32>>::compute(
        &4,
        &30, // Current node interval end value is smaller than child maxima
        Some(mock_left_child),
        Some(mock_right_child),
    );

    // This assertion guarantees that the right branch successfully overwrote `max_end`
    assert_eq!(parent_max_end, Some(120));
}
