#![cfg(feature = "interval-tree")]
#![cfg_attr(feature = "nightly", feature(allocator_api))]

use augmented_rbtree::Global;
use augmented_rbtree::interval_tree::{Interval, IntervalTree};

#[test]
fn interval_basic_insert_query() {
    let mut tree = IntervalTree::new();
    tree.insert(Interval::new(1, 5), "a");
    tree.insert(Interval::new(3, 8), "b");
    tree.insert(Interval::new(10, 15), "c");

    assert_eq!(tree.len(), 3);
    assert!(!tree.is_empty());
    assert_eq!(tree.get(&Interval::new(1, 5)), Some(&"a"));
    assert_eq!(tree.get(&Interval::new(99, 100)), None);
}

#[test]
fn interval_overlap_all() {
    let mut tree = IntervalTree::new();
    tree.insert(Interval::new(1, 5), "a");
    tree.insert(Interval::new(3, 8), "b");
    tree.insert(Interval::new(7, 10), "c");

    // [4, 8] overlaps all three
    let mut matches: Vec<_> = tree.query_overlap(4, 8).map(|(iv, v)| (*iv, *v)).collect();
    matches.sort_by_key(|(iv, _)| *iv);
    assert_eq!(matches.len(), 3);
}

#[test]
fn interval_overlap_none() {
    let mut tree = IntervalTree::new();
    tree.insert(Interval::new(1, 3), "a");
    tree.insert(Interval::new(7, 10), "b");

    // [4, 6] has no overlap
    let matches: Vec<_> = tree.query_overlap(&4, &6).collect();
    assert_eq!(matches.len(), 0);
    assert!(!tree.any_overlaps(4, 6));
}

#[test]
fn interval_overlap_boundary() {
    let mut tree = IntervalTree::new();
    tree.insert(Interval::new(1, 5), "a");
    tree.insert(Interval::new(5, 10), "b");

    // [5, 5] is a point that touches both intervals at boundary
    let matches: Vec<_> = tree.query_overlap(&5, &5).collect();
    assert_eq!(matches.len(), 2);
    assert!(tree.any_contains_point(5));
}

#[test]
fn interval_query_point() {
    let mut tree = IntervalTree::new();
    tree.insert(Interval::new(1, 10), "wide");
    tree.insert(Interval::new(3, 5), "narrow");
    tree.insert(Interval::new(20, 30), "distant");

    let at_4: Vec<_> = tree.query_point(&4).map(|(_, v)| *v).collect();
    assert_eq!(at_4.len(), 2);
    assert!(at_4.contains(&"wide"));
    assert!(at_4.contains(&"narrow"));

    let at_20: Vec<_> = tree.query_point(&20).map(|(_, v)| *v).collect();
    assert_eq!(at_20.len(), 1); // only "distant" [20,30] contains 20; "wide" [1,10] does not
    assert!(at_20.contains(&"distant"));
}

#[test]
fn interval_query_point_correct() {
    let mut tree = IntervalTree::new();
    tree.insert(Interval::new(1, 10), "a");
    tree.insert(Interval::new(20, 30), "b");

    let at_15: Vec<_> = tree.query_point(&15).collect();
    assert_eq!(at_15.len(), 0);
    assert!(!tree.any_contains_point(15));

    let at_5_interior: Vec<_> = tree.query_point(&5).collect();
    assert_eq!(at_5_interior.len(), 1);
    assert!(tree.any_contains_point(5));
}

#[test]
fn interval_any_overlaps() {
    let mut tree = IntervalTree::new();
    tree.insert(Interval::new(1, 5), ());
    tree.insert(Interval::new(10, 20), ());

    assert!(tree.any_overlaps(&3, &7));
    assert!(tree.any_overlaps(&0, &1)); // touches lower boundary
    assert!(tree.any_overlaps(&5, &5)); // touches upper boundary
    assert!(!tree.any_overlaps(&6, &9)); // gap between intervals
}

#[test]
fn interval_first_overlap() {
    let mut tree = IntervalTree::new();
    tree.insert(Interval::new(1, 5), "first");
    tree.insert(Interval::new(3, 8), "second");
    tree.insert(Interval::new(10, 15), "third");

    let first = tree.first_overlap(&4, &12);
    assert!(first.is_some());
    assert_eq!(first.unwrap().1, &"first"); // [1,5] has smallest lo

    assert!(tree.first_overlap(&20, &25).is_none());
}

#[test]
fn interval_remove() {
    let mut tree = IntervalTree::new();
    tree.insert(Interval::new(1, 5), "a");
    tree.insert(Interval::new(3, 8), "b");

    assert_eq!(tree.remove(&Interval::new(1, 5)), Some("a"));
    assert_eq!(tree.len(), 1);
    assert_eq!(tree.remove(&Interval::new(1, 5)), None);

    let matches: Vec<_> = tree.query_overlap(&1, &5).collect();
    assert_eq!(matches.len(), 1); // only "b" remains
}

#[test]
fn interval_empty_tree() {
    let tree = IntervalTree::<i32, ()>::new();
    assert!(tree.is_empty());
    assert_eq!(tree.len(), 0);
    assert!(!tree.any_overlaps(&0, &100));
    assert!(tree.query_overlap(&0, &100).next().is_none());
    assert!(tree.first_overlap(&0, &100).is_none());
}

#[test]
fn interval_degenerate_point_intervals() {
    let mut tree = IntervalTree::new();
    for i in 0..10 {
        tree.insert(Interval::new(i, i), i);
    }

    // Each integer is a point interval
    let point_at_5: Vec<_> = tree.query_point(&5).collect();
    assert_eq!(point_at_5.len(), 1);
    assert_eq!(*point_at_5[0].1, 5);
}

#[test]
fn interval_large_fuzz() {
    use std::collections::HashSet;

    let mut tree = IntervalTree::new();
    let intervals: Vec<(i32, i32)> = (0..100).map(|i| (i * 3, i * 3 + 5)).collect();

    for &(lo, hi) in &intervals {
        tree.insert(Interval::new(lo, hi), (lo, hi));
    }

    // Query a range and verify against brute force
    let query_lo = 50;
    let query_hi = 80;
    let expected: HashSet<(i32, i32)> = intervals
        .iter()
        .filter(|&&(lo, hi)| lo <= query_hi && query_lo <= hi)
        .map(|&(lo, hi)| (lo, hi))
        .collect();

    let found: HashSet<(i32, i32)> = tree
        .query_overlap(&query_lo, &query_hi)
        .map(|(iv, _)| (iv.lo, iv.hi))
        .collect();

    assert_eq!(found.len(), expected.len());
    for (lo, hi) in &expected {
        assert!(found.contains(&(*lo, *hi)));
    }
}

#[test]
fn interval_contains_method() {
    let iv = Interval::new(3, 7);
    assert!(iv.contains_point(&3));
    assert!(iv.contains_point(&5));
    assert!(iv.contains_point(&7));
    assert!(!iv.contains_point(&2));
    assert!(!iv.contains_point(&8));
}

#[test]
fn interval_overlaps_method() {
    let a = Interval::new(1, 5);
    let b = Interval::new(3, 8);
    let c = Interval::new(6, 10);

    assert!(a.overlaps(&b));
    assert!(b.overlaps(&a));
    assert!(b.overlaps(&c));
    assert!(!a.overlaps(&c)); // [1,5] and [6,10] don't overlap
}

#[test]
fn interval_ordering() {
    let mut intervals = [
        Interval::new(5, 10),
        Interval::new(1, 3),
        Interval::new(1, 5),
        Interval::new(3, 7),
    ];
    intervals.sort();
    assert_eq!(intervals[0], Interval::new(1, 3));
    assert_eq!(intervals[1], Interval::new(1, 5));
    assert_eq!(intervals[2], Interval::new(3, 7));
    assert_eq!(intervals[3], Interval::new(5, 10));
}

#[test]
fn interval_display() {
    let iv = Interval::new(3, 7);
    assert_eq!(iv.to_string(), "[3, 7]");
}

#[test]
fn interval_debug_tree() {
    let mut tree = IntervalTree::new();
    tree.insert(Interval::new(1, 5), "a");
    let debug = format!("{tree:?}");
    assert!(debug.contains("lo: 1") && debug.contains("hi: 5"));
}

#[test]
fn check_contains_exact_interval() {
    let mut tree = IntervalTree::default();
    tree.insert(Interval::new(1, 5), "a");
    tree.insert(Interval::new(3, 8), "b");

    assert!(tree.contains(&Interval::new(1, 5)));
    assert!(!tree.contains(&Interval::new(2, 4)));
}

#[test]
fn check_any_overlaps_with_exact_interval() {
    let mut tree = IntervalTree::default();
    tree.insert(Interval::new(1, 5), "a");
    tree.insert(Interval::new(3, 8), "b");
    tree.insert(Interval::new(10, 15), "c");
    tree.insert(Interval::new(20, 25), "d");

    assert!(tree.any_overlaps(&1, &5));
    assert!(tree.any_overlaps(&6, &7));
    assert!(!tree.any_overlaps(&100, &120));
}

#[test]
fn test_create_interval_tree_with_global_allocator() {
    let mut tree = IntervalTree::<i32, i32>::new_in(Global);
    tree.insert(Interval::new(1, 5), 10);
    tree.insert(Interval::new(3, 8), 20);
    tree.insert(Interval::new(10, 15), 30);

    assert_eq!(tree.len(), 3);
    assert_eq!(tree.get(&Interval::new(1, 5)), Some(&10));
    assert_eq!(tree.get(&Interval::new(3, 8)), Some(&20));
    assert_eq!(tree.get(&Interval::new(10, 15)), Some(&30));

    assert_eq!(tree.inner_tree().len(), 3);
}
