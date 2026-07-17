use augmented_rbtree::{Allocator, AugmentedRBTree, Color};
use std::fmt::Write;
use std::fs::File;
use std::path::Path;

pub(crate) fn dump_tree_to_svg<K, V, G, S, A: Allocator, P: AsRef<Path>>(
    tree: &AugmentedRBTree<K, V, G, A>,
    path: P,
    show_nulls: bool,
) -> std::io::Result<()>
where
    K: std::fmt::Display + Ord,
    V: std::fmt::Display,
    G: augmented_rbtree::Augment<K, V, Stats = S>,
    S: std::fmt::Display,
{
    let mut dot = String::new();
    dot.push_str("digraph RBT {\n");
    dot.push_str("    node [fontname=\"Arial\", shape=Mrecord, style=filled, color=\"white\", fontcolor=white];\n");

    let mut nil_counter = 0;

    tree.visit_topology(|key, color, left, right| {
        let color_str = match color {
            Color::Red => "red",
            Color::Black => "black",
        };

        // Fetch the value for the current key
        let (value_str, stats_str) = if let Some((val, stats)) = tree.get_value_stats(key) {
            (format!("{val}"), format!("{stats}"))
        } else {
            ("None".to_string(), "None".to_string())
        };

        // Escape special Graphviz characters including double quotes inside labels
        let escaped_key = format!("{key}")
            .replace('"', "\\\"")
            .replace('|', "\\|")
            .replace('{', "\\{")
            .replace('}', "\\}");
        let escaped_val = value_str
            .replace('"', "\\\"")
            .replace('|', "\\|")
            .replace('{', "\\{")
            .replace('}', "\\}");
        let escaped_stat = stats_str
            .replace('"', "\\\"")
            .replace('|', "\\|")
            .replace('{', "\\{")
            .replace('}', "\\}");

        let label = format!("{{ {escaped_key} | {{ {escaped_val} | {escaped_stat} }} }}");

        // Safely escape the key string to use it as a Node Identifier
        let id_key = format!("{key}").replace('"', "\\\"");

        // Wrap the node ID in double quotes to handle spaces/brackets securely
        let _ = writeln!(
            dot,
            "    \"{id_key}\" [label=\"{label}\", fillcolor={color_str}, color=\"white\"];"
        );

        if let Some(l_key) = left {
            let id_l_key = format!("{l_key}").replace('"', "\\\"");
            let _ = writeln!(dot, "    \"{id_key}\" -> \"{id_l_key}\";");
        } else if show_nulls {
            nil_counter += 1;
            let nil_name = format!("nil_{nil_counter}");
            let _ = writeln!(
                dot,
                "    {nil_name} [label=\"NIL\", shape=box, fillcolor=black, color=white, fontcolor=white];"
            );
            let _ = writeln!(dot, "    \"{id_key}\" -> {nil_name};");
        }

        if let Some(r_key) = right {
            let id_r_key = format!("{r_key}").replace('"', "\\\"");
            let _ = writeln!(dot, "    \"{id_key}\" -> \"{id_r_key}\";");
        } else if show_nulls {
            nil_counter += 1;
            let nil_name = format!("nil_{nil_counter}");
            let _ = writeln!(
                dot,
                "    {nil_name} [label=\"NIL\", shape=box, fillcolor=black, color=white, fontcolor=white];"
            );
            let _ = writeln!(dot, "    \"{id_key}\" -> {nil_name};");
        }
    });

    dot.push_str("}\n");

    let parsed_graph = graphviz_rust::parse(&dot)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    let svg_bytes = graphviz_rust::exec(
        parsed_graph,
        &mut graphviz_rust::printer::PrinterContext::default(),
        vec![graphviz_rust::cmd::CommandArg::Format(
            graphviz_rust::cmd::Format::Svg,
        )],
    )
    .map_err(std::io::Error::other)?;

    let mut file = File::create(path)?;
    std::io::Write::write_all(&mut file, svg_bytes.as_ref())?;

    Ok(())
}
