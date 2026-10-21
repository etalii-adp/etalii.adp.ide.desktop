//! A `.skv` model as the elements a session hands the shell: its nodes as bars and its flows as bands, laid out
//! (etalii.adp spec 030, task T077).
//!
//! Ported from the standalone host's `SankeyElementMapper.cs` at `8b6ccfc`, whose protobuf payloads
//! (`api/sankey.proto`) are the elements' attributes here. Nodes come first, so a flow never arrives before the ends it
//! is drawn between. The layout is computed over the whole document and only then culled: a node is kept when its bar
//! overlaps the view, or when a flow from a kept node reaches it, so no flow arrives with one end missing; and a flow
//! is kept on whether both its ends were, never on a position of its own, since it has none. A node is placed at its
//! bar's centre, as the library draws from the centre.

use std::collections::{HashMap, HashSet};

use adp_diagram::element::Rect;
use adp_diagram::{DiagramElement, DiagramViewport, ElementKind};

use crate::format;
use crate::layout::SankeyLayout;
use crate::model::{FROM_SOURCE, SankeyFlow, SankeyModel, SankeyNode};

/// The type of a node's element.
pub const NODE_TYPE: &str = "etalii/sankey+node";

/// The type of a flow's element.
pub const FLOW_TYPE: &str = "etalii/sankey+flow";

/// Where a node's labels sit when it is in the first column: before the bar.
pub const LEFT_SIDE: &str = "left";

/// Where every other node's labels sit: after the bar.
pub const RIGHT_SIDE: &str = "right";

/// A number as an attribute's text.
fn number(value: f64) -> String {
    format!("{value}")
}

/// Everything drawable, unculled: what a selection is looked up in.
pub fn all(model: &SankeyModel) -> Vec<DiagramElement> {
    visible(model, DiagramViewport::UNBOUNDED)
}

/// The elements a view of `viewport` holds.
pub fn visible(model: &SankeyModel, viewport: DiagramViewport) -> Vec<DiagramElement> {
    let layout = SankeyLayout::of(model);
    let mut kept: HashSet<&str> = layout
        .nodes
        .iter()
        .filter(|node| {
            let bar = layout.boxes[&node.id];
            viewport.intersects(Rect::new(bar.x, bar.y, bar.width, bar.height))
        })
        .map(|node| node.id.as_str())
        .collect();
    // A visible node pulls in what it is joined to, so every flow touching it arrives whole.
    let touching: Vec<&SankeyFlow> = layout
        .flows
        .iter()
        .filter(|flow| kept.contains(flow.from.as_str()) || kept.contains(flow.to.as_str()))
        .collect();
    for flow in touching {
        kept.insert(&flow.from);
        kept.insert(&flow.to);
    }
    let by_id: HashMap<&str, &SankeyNode> = layout.nodes.iter().map(|node| (node.id.as_str(), node)).collect();
    layout
        .nodes
        .iter()
        .filter(|node| kept.contains(node.id.as_str()))
        .map(|node| node_element(model, &layout, node))
        .chain(
            layout
                .flows
                .iter()
                .filter(|flow| kept.contains(flow.from.as_str()) && kept.contains(flow.to.as_str()))
                .map(|flow| flow_element(model, &layout, flow, &by_id)),
        )
        .collect()
}

fn node_element(model: &SankeyModel, layout: &SankeyLayout, node: &SankeyNode) -> DiagramElement {
    let bar = layout.boxes[&node.id];
    let value = layout.values[&node.id];
    let (word, hex) = format::resolve_or_default(&node.color);
    let column = layout.columns[&node.id];
    let format = if node.format.is_empty() {
        &model.settings.format
    } else {
        &node.format
    };
    DiagramElement {
        id: node.id.clone(),
        x: bar.centre_x(),
        y: bar.centre_y(),
        placed: false,
        element_type: NODE_TYPE.to_owned(),
        kind: ElementKind::Node,
        parent: None,
        source: None,
        target: None,
        attributes: vec![
            ("name".to_owned(), node.label().to_owned()),
            ("value".to_owned(), number(value)),
            ("displayValue".to_owned(), format::write(format, value)),
            ("note".to_owned(), node.note.clone()),
            ("color".to_owned(), word),
            ("customColor".to_owned(), hex),
            ("width".to_owned(), number(bar.width)),
            ("height".to_owned(), number(bar.height)),
            ("column".to_owned(), column.to_string()),
            (
                "side".to_owned(),
                if column == 0 { LEFT_SIDE } else { RIGHT_SIDE }.to_owned(),
            ),
        ],
    }
}

fn flow_element(
    model: &SankeyModel,
    layout: &SankeyLayout,
    flow: &SankeyFlow,
    nodes: &HashMap<&str, &SankeyNode>,
) -> DiagramElement {
    let id = flow.id();
    let band = layout.bands[&id];
    let value = flow.amount();
    // A flow states its own colour, or takes the colour of the end the document names.
    let end = nodes[if model.settings.flow_color == FROM_SOURCE {
        flow.from.as_str()
    } else {
        flow.to.as_str()
    }];
    let (end_word, end_hex) = format::resolve_or_default(&end.color);
    let fallback = if end_word.is_empty() { end_hex } else { end_word };
    let (word, hex) = format::resolve(&flow.color, &fallback);
    DiagramElement {
        id,
        // A flow has no position of its own; it follows its ends.
        x: 0.0,
        y: 0.0,
        placed: false,
        element_type: FLOW_TYPE.to_owned(),
        kind: ElementKind::Relation,
        parent: None,
        source: Some(flow.from.clone()),
        target: Some(flow.to.clone()),
        attributes: vec![
            ("value".to_owned(), number(value)),
            ("displayValue".to_owned(), format::write(&model.settings.format, value)),
            ("thickness".to_owned(), number(band.thickness)),
            ("sourceAt".to_owned(), number(band.source_at)),
            ("targetAt".to_owned(), number(band.target_at)),
            ("color".to_owned(), word),
            ("customColor".to_owned(), hex),
            ("backward".to_owned(), band.backward.to_string()),
        ],
    }
}
