#![cfg(any(feature = "alloc", feature = "allocator-api", feature = "nightly"))]
#![cfg_attr(feature = "nightly", feature(allocator_api))]

use augmented_rbtree::AugmentedRBTree;

use crate::helpers::{common::UnitforTest, dumper::dump_tree};

mod helpers;

// dump_tree(&tree, Some("before"), true);
// println!("Black height before deletion: {}", tree.black_height());

// dump_tree(&tree, Some("after"), true);
// println!("Black height before deletion: {}", tree.black_height());
