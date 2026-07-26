#![cfg(any(feature = "alloc", feature = "allocator-api", feature = "nightly"))]
#![cfg_attr(feature = "nightly", feature(allocator_api))]

use std::iter::repeat_with;

use augmented_rbtree::{
    Augment, AugmentedRBTreeFactory, InOrderIter, InOrderPruningPolicy, TreeLocation,
};
use itertools::Itertools;
use rand::RngExt;

use crate::helpers::common::test_rng;

mod helpers;

struct MinMaxAugmentation;

impl Augment<i32, i32> for MinMaxAugmentation {
    type Stats = (Option<i32>, Option<i32>);

    fn compute(
        _key: &i32,
        value: &i32,
        left: Option<(&i32, &i32, &Self::Stats)>,
        right: Option<(&i32, &i32, &Self::Stats)>,
    ) -> Self::Stats {
        let mut min = *value;
        let mut max = *value;

        if let Some((_, _, (Some(ls_min), Some(ls_max)))) = left {
            if *ls_min < min {
                min = *ls_min;
            }
            if *ls_max > max {
                max = *ls_max;
            }
        }

        if let Some((_, _, (Some(rs_min), Some(rs_max)))) = right {
            if *rs_min < min {
                min = *rs_min;
            }
            if *rs_max > max {
                max = *rs_max;
            }
        }

        (Some(min), Some(max))
    }
}

struct RangePrunningPolicy {
    min: i32,
    max: i32,
}

impl InOrderPruningPolicy<i32, i32, (Option<i32>, Option<i32>)> for RangePrunningPolicy {
    fn is_match(&self, key: &i32, _value: &i32, _stats: &(Option<i32>, Option<i32>)) -> bool {
        *key >= self.min && *key <= self.max
    }

    fn should_explore_left(
        &self,
        left: (&i32, &i32, &(Option<i32>, Option<i32>)),
        _current: (&i32, &i32, &(Option<i32>, Option<i32>)),
    ) -> bool {
        let (_, _, left_stats) = left;
        // Explore only if the left subtree's maximum key is >= our target minimum bound
        match left_stats.1 {
            Some(left_max) => left_max >= self.min,
            None => false,
        }
    }

    fn should_explore_right(
        &self,
        right: (&i32, &i32, &(Option<i32>, Option<i32>)),
        _current: (&i32, &i32, &(Option<i32>, Option<i32>)),
    ) -> bool {
        let (_, _, right_stats) = right;
        // Explore only if the right subtree's minimum key is <= our target maximum bound
        match right_stats.0 {
            Some(right_min) => right_min <= self.max,
            None => false,
        }
    }
}

#[test]
fn test_search_min_max_augmentation() {
    let mut tree = AugmentedRBTreeFactory::<MinMaxAugmentation>::new_tree();
    let mut rng = test_rng();

    let keys: Vec<i32> = repeat_with(|| rng.random_range(1..200))
        .unique()
        .take(100)
        .collect();
    for &key in &keys {
        tree.insert(key, key);
    }

    let it_search = InOrderIter::from_cursor(
        &tree.nav_cursor(TreeLocation::Root),
        RangePrunningPolicy { min: 80, max: 120 },
    );

    let values = it_search.map(|(_, value, _stats)| *value).collect_vec();
    assert!(values.iter().all(|&v| (80..=120).contains(&v)));
}
