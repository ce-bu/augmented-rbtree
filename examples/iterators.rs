use augmented_rbtree::{AugmentedRBTree, Unit};

fn main() {
    // Create a new tree
    let mut tree = AugmentedRBTree::<i32, String, Unit>::new();

    // Insert some values
    tree.insert(5, "five".to_string());
    tree.insert(3, "three".to_string());
    tree.insert(7, "seven".to_string());
    tree.insert(1, "one".to_string());
    tree.insert(9, "nine".to_string());

    println!("Tree size: {}", tree.len());

    // Iterate immutably
    println!("\n=== Immutable iteration (iter) ===");
    for (k, v, _s) in &tree {
        println!("Key: {k}, Value: {v}");
    }

    // Iterate over keys only
    println!("\n=== Keys iteration ===");
    for k in tree.keys() {
        println!("Key: {k}");
    }

    // Iterate over values only
    println!("\n=== Values iteration ===");
    for v in tree.values() {
        println!("Value: {v}");
    }

    // Mutable iteration
    println!("\n=== Mutable iteration (iter_mut) ===");
    for (k, mut v, _s) in &mut tree {
        println!("Modifying value at key {k}");
        v.push_str(" (modified)");
    }

    println!("\n=== After modification ===");
    for (k, v, _s) in &tree {
        println!("Key: {k}, Value: {v}");
    }

    // Values mutable iteration
    println!("\n=== Values mutable iteration ===");
    for mut v in tree.values_mut() {
        v.push('!');
    }

    println!("\n=== After values_mut ===");
    for v in tree.values() {
        println!("Value: {v}");
    }

    // Reverse iteration
    println!("\n=== Reverse iteration ===");
    for (k, v, _s) in tree.iter().rev() {
        println!("Key: {k}, Value: {v}");
    }

    // Consuming iteration (into_iter)
    println!("\n=== Consuming iteration (into_iter) ===");
    for (k, v) in tree {
        println!("Consumed - Key: {k}, Value: {v}");
    }

    // Tree is now consumed, can't use it anymore
    println!("\nTree has been consumed!");
}
