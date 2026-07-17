//! This example demonstrates how to visualize an augmented red-black tree using the `augmented-rbtree` crate.
//! It generates an SVG representation of the tree structure, which can be useful for debugging and understanding the tree's layout.

use crate::visual::dump_tree_to_svg;
use augmented_rbtree::interval_tree::IntervalTree;

#[path = "../tests/shared/visual.rs"]
mod visual;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut tree = IntervalTree::new();

    let intervals = [
        (38, 45, "A"),
        (18, 25, "B"),
        (12, 20, "C"),
        (42, 50, "D"),
        (28, 35, "E"),
        (32, 40, "F"),
        (48, 55, "G"),
        (22, 30, "H"),
        (2, 10, "I"),
        (5, 15, "J"),
    ];

    for i in &intervals {
        tree.insert(
            augmented_rbtree::interval_tree::Interval::new(i.0, i.1),
            i.2,
        );
    }

    // Automatically outputs into your project's assets directory for the README link
    std::fs::create_dir_all("assets")?;
    dump_tree_to_svg(tree.inner_tree(), "assets/visualization_demo.svg", false)?;

    println!("Successfully rendered tree visualization to 'assets/visualization_demo.svg'!");
    Ok(())
}
