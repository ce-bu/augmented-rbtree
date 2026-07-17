#[path = "../shared/visual.rs"]
mod visual;

use augmented_rbtree::{Allocator, AugmentedRBTree};
use std::path::PathBuf;

fn get_test_svg_path(suffix: Option<&str>) -> PathBuf {
    let thread = std::thread::current();
    let test_name = thread.name().unwrap_or("test_fallback");
    let safe_name = test_name.replace("::", "_");
    let filename = if let Some(suffix) = suffix {
        format!("{safe_name}_{suffix}.svg")
    } else {
        format!("{safe_name}.svg")
    };

    // Rout into a dedicated visual folder under the workspace target block
    let mut base_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    base_path.push("target");
    base_path.push("tree_visualizations");

    // Create the physical folder inside CI runner right before writing
    let _ = std::fs::create_dir_all(&base_path);

    base_path.push(filename);
    base_path
}

#[allow(dead_code)]
pub(crate) fn dump_tree<K, V, G, S, A: Allocator>(
    tree: &AugmentedRBTree<K, V, G, A>,
    name: Option<&str>,
    show_nulls: bool,
) -> std::io::Result<()>
where
    K: std::fmt::Display + Ord,
    V: std::fmt::Display,
    G: augmented_rbtree::Augment<K, V, Stats = S>,
    S: std::fmt::Display,
{
    let path = get_test_svg_path(name);
    visual::dump_tree_to_svg(tree, path, show_nulls)
}
