use augmented_rbtree::{Augment, AugmentedRBTree, augmentations::SumAugmentation};

/// Counter augmentation that tracks the size of each subtree
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Counter;

impl Augment<i32, &'static str> for Counter {
    type Stats = usize;

    fn identity() -> Self::Stats {
        0
    }

    fn compute(
        _key: &i32,
        _value: &&'static str,
        left: Option<(&i32, &&'static str, &Self::Stats)>,
        right: Option<(&i32, &&'static str, &Self::Stats)>,
    ) -> Self::Stats {
        let left_count = left.map_or(0, |(_, _, count)| *count);
        let right_count = right.map_or(0, |(_, _, count)| *count);
        1 + left_count + right_count
    }
}

fn example_counter() {
    let mut tree = AugmentedRBTree::<i32, &str, Counter>::new();

    // Insert some values
    tree.insert(5, "five");
    tree.insert(3, "three");
    tree.insert(7, "seven");
    tree.insert(1, "one");
    tree.insert(9, "nine");
    tree.insert(4, "four");
    tree.insert(6, "six");

    println!("Tree with Counter augmentation:");
    println!("Each node stores the count of nodes in its subtree\n");

    // Iterate and show the augmentation data (subtree size) for each node
    println!("Key | Value | Subtree Size");
    println!("----+-------+-------------");
    for (key, value, subtree_size) in &tree {
        println!("{key:3} | {value:5} | {subtree_size:12}");
    }

    println!("\nNote: The root node (with the largest subtree size) contains");
    println!("the total count of all nodes in the tree.");

    // Find the node with the largest subtree (should be the root)
    let max_subtree = tree
        .iter()
        .max_by_key(|(_, _, size)| *size)
        .map(|(k, v, s)| (*k, *v, *s));

    if let Some((key, value, size)) = max_subtree {
        println!("\nNode with largest subtree (root): key={key}, value={value}, size={size}");
    }

    // Demonstrate consuming iteration with stats
    println!("\n=== Consuming iteration (showing stats) ===");
    for (key, value) in tree {
        println!("Key: {key}, Value: {value}");
    }
}

fn example_sum() {
    let mut tree = AugmentedRBTree::<i32, i32, SumAugmentation>::new();
    tree.insert(2, 2);
    tree.insert(1, 1);
    tree.insert(3, 3);

    let stats: Vec<_> = tree.stats().collect();
    println!("exapmple_sum {stats:?}");
}
fn main() {
    example_counter();
    example_sum();
}
