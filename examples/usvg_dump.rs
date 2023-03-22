//! This example parses an SVG and print its structure as JSON. May be inspected using `fx`.

use std::error::Error;

use serde::Serialize;

#[derive(Serialize)]
enum DumpNode {
    Group {
        id: String,
        mode: String,
        children: Vec<DumpNode>,
    },
    Path {
        id: String,
    },
    Unknown,
}

fn parse_node(node: &usvg::Node) -> DumpNode {
    match *node.borrow() {
        usvg::NodeKind::Group(ref group) => DumpNode::Group {
            id: group.id.clone(),
            mode: format!("{:?}", group.mode),
            children: node.children().map(|child| parse_node(&child)).collect(),
        },
        usvg::NodeKind::Path(ref path) => DumpNode::Path {
            id: path.id.clone(),
        },
        _ => DumpNode::Unknown,
    }
}

fn parse_svg(svg: &str) -> Result<DumpNode, Box<dyn Error>> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default())?;
    let dump_node = parse_node(&tree.root);

    Ok(dump_node)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        println!("Usage:\n\t{} FILE", args[0]);
        return;
    }

    let svg = std::fs::read_to_string(&args[1]).unwrap();

    let dump_node = parse_svg(&svg).unwrap();

    match dump_node {
        DumpNode::Group { children, .. } => {
            let j = serde_json::to_string(&children).unwrap();
            println!("{}", j);
        }
        _ => {}
    }
}
