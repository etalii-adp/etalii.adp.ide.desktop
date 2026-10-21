//! A `.supply` model as what the canvas draws: groups, the seven stages and flows, laid out and marked with the trace
//! through the selection (etalii.adp spec 030, task T076).
//!
//! Ported from the standalone host's `src/diagrams/supply-chain/backend/EtAlii.Adp.Diagram.SupplyChain/
//! SupplyChainElementMapper.cs` and the protobuf payloads of `api/supply-chain.proto` at `8b6ccfc`, with the text the
//! client's `supplyChainModel.ts` and `SupplyChainCanvas.tsx` formatted from them. Groups come first, so their frames
//! are drawn beneath the nodes inside them. The layout is computed over the whole document and only then culled: a
//! group frame is kept when its rectangle overlaps the view, a node when its box does or a flow from a kept node
//! reaches it, and a flow when both its ends were kept. A flow's weight is relative to the heaviest flow in the same
//! unit, and a card's share to the largest quantity in the same unit over the whole document.

use std::collections::{HashMap, HashSet};

use adp_diagram::element::Rect as ViewRect;
use adp_diagram::{DiagramElement, DiagramViewport, ElementKind};
use adp_layout::Rect;

use crate::layout::SupplyChainLayout;
use crate::model::{SupplyChainFlow, SupplyChainGroup, SupplyChainModel, SupplyChainNode, stages};
use crate::trace::SupplyChainTrace;

/// The element types the module puts on the wire, as the standalone host's mapper named them.
pub const PREFIX: &str = "etalii/supply-chain+";
pub const GROUP_TYPE: &str = "etalii/supply-chain+group";
pub const FLOW_TYPE: &str = "etalii/supply-chain+flow";

/// The element type a stage is drawn as.
pub fn node_type_of(stage: &str) -> String {
    format!("{PREFIX}{stage}")
}

fn view_rect(rect: Rect) -> ViewRect {
    ViewRect::new(rect.x, rect.y, rect.width, rect.height)
}

/// Everything drawable, unculled, with no trace: what a selection is looked up in.
pub fn all(model: &SupplyChainModel) -> Vec<DiagramElement> {
    visible(model, DiagramViewport::UNBOUNDED, &SupplyChainTrace::none())
}

/// The elements a view of `viewport` holds, marked with `trace`.
pub fn visible(model: &SupplyChainModel, viewport: DiagramViewport, trace: &SupplyChainTrace) -> Vec<DiagramElement> {
    let layout = SupplyChainLayout::of(model);
    let mut kept: HashSet<&str> = layout
        .nodes
        .iter()
        .filter(|n| viewport.intersects(view_rect(layout.node_boxes[&n.id])))
        .map(|n| n.id.as_str())
        .collect();
    // A visible node pulls in what it trades with, so every flow touching it arrives whole.
    let touching: Vec<&SupplyChainFlow> = layout
        .flows
        .iter()
        .filter(|f| kept.contains(f.from.as_str()) || kept.contains(f.to.as_str()))
        .collect();
    for flow in touching {
        kept.insert(flow.from.as_str());
        kept.insert(flow.to.as_str());
    }

    // A band's width compares like with like: tonnes of ore against tonnes, cars against cars.
    let mut heaviest: HashMap<&str, f64> = HashMap::new();
    for flow in &layout.flows {
        let volume = flow.volume.unwrap_or(0.0);
        let entry = heaviest.entry(flow.unit.as_str()).or_insert(volume);
        *entry = entry.max(volume);
    }
    // A card's bar compares like with like too, and over the whole document rather than the view.
    let mut largest: HashMap<&str, f64> = HashMap::new();
    for node in layout.nodes.iter().filter(|n| n.quantity.is_some()) {
        let quantity = node.quantity.unwrap_or(0.0);
        let entry = largest.entry(node.unit.as_str()).or_insert(quantity);
        *entry = entry.max(quantity);
    }

    let mut elements: Vec<DiagramElement> = layout
        .groups
        .iter()
        .filter(|g| viewport.intersects(view_rect(layout.group_boxes[&g.id])))
        .map(|g| group(&layout, g, trace))
        .collect();
    elements.extend(
        layout
            .nodes
            .iter()
            .filter(|n| kept.contains(n.id.as_str()))
            .map(|n| node(&layout, n, &largest, trace)),
    );
    elements.extend(
        layout
            .flows
            .iter()
            .filter(|f| kept.contains(f.from.as_str()) && kept.contains(f.to.as_str()))
            .map(|f| flow(f, heaviest[f.unit.as_str()], layout.lanes.get(&f.id), trace)),
    );
    elements
}

fn number(value: f64) -> String {
    value.to_string()
}

fn group(layout: &SupplyChainLayout, group: &SupplyChainGroup, trace: &SupplyChainTrace) -> DiagramElement {
    let frame = layout.group_boxes[&group.id];
    let members = layout.nodes.iter().filter(|n| n.group == group.id).count();
    DiagramElement {
        x: frame.center_x(),
        y: frame.center_y(),
        placed: true,
        attributes: vec![
            ("name".into(), display_name(&group.name, &group.id)),
            ("width".into(), number(frame.width)),
            ("height".into(), number(frame.height)),
            ("members".into(), members.to_string()),
            (
                "membersText".into(),
                if members == 1 {
                    "1 node".to_owned()
                } else {
                    format!("{members} nodes")
                },
            ),
            ("trace".into(), trace.of(&group.id).to_owned()),
        ],
        ..DiagramElement::node(&group.id, GROUP_TYPE)
    }
}

fn display_name(name: &str, id: &str) -> String {
    if name.is_empty() { id } else { name }.to_owned()
}

fn node(
    layout: &SupplyChainLayout,
    node: &SupplyChainNode,
    largest: &HashMap<&str, f64>,
    trace: &SupplyChainTrace,
) -> DiagramElement {
    // The document holds the top-left; the canvas draws from the centre.
    let box_ = layout.node_boxes[&node.id];
    let share = match (node.quantity, largest.get(node.unit.as_str())) {
        (Some(quantity), Some(most)) if *most > 0.0 => (quantity / most).clamp(0.0, 1.0),
        _ => 0.0,
    };
    let name = display_name(&node.name, &node.id);
    let stage = stages::display(&node.node_type);
    let amount = match node.quantity {
        Some(quantity) => format_amount(quantity),
        None => "—".to_owned(),
    };
    let tooltip = match node.quantity {
        Some(_) if !node.unit.is_empty() => format!("{stage}: {name} — {amount} {}", node.unit),
        Some(_) => format!("{stage}: {name} — {amount}"),
        None => format!("{stage}: {name}"),
    };
    DiagramElement {
        x: box_.center_x(),
        y: box_.center_y(),
        placed: node.is_placed(),
        attributes: vec![
            ("name".into(), name),
            ("group".into(), node.group.clone()),
            (
                "groupId".into(),
                if layout.group_boxes.contains_key(&node.group) {
                    node.group.clone()
                } else {
                    String::new()
                },
            ),
            ("quantity".into(), number(node.quantity.unwrap_or(0.0))),
            ("hasQuantity".into(), node.quantity.is_some().to_string()),
            ("amount".into(), amount),
            ("unit".into(), node.unit.clone()),
            ("width".into(), number(box_.width)),
            ("height".into(), number(box_.height)),
            ("trace".into(), trace.of(&node.id).to_owned()),
            ("stage".into(), stage.clone()),
            ("stageTitle".into(), stage.to_uppercase()),
            ("share".into(), number(share)),
            ("tooltip".into(), tooltip),
        ],
        ..DiagramElement::node(&node.id, node_type_of(stages::normalize(&node.node_type)))
    }
}

fn flow(
    flow: &SupplyChainFlow,
    heaviest: f64,
    lanes: Option<&Vec<(f64, f64)>>,
    trace: &SupplyChainTrace,
) -> DiagramElement {
    let weight = match flow.volume {
        Some(volume) if heaviest > 0.0 => (volume / heaviest).clamp(0.0, 1.0),
        _ => 0.0,
    };
    let volume = match flow.volume {
        Some(volume) if flow.unit.is_empty() => format_amount(volume),
        Some(volume) => format!("{} {}", format_amount(volume), flow.unit),
        None => String::new(),
    };
    let lanes = lanes
        .map(|lanes| {
            lanes
                .iter()
                .map(|(x, y)| format!("{x},{y}"))
                .collect::<Vec<_>>()
                .join(";")
        })
        .unwrap_or_default();
    // A flow has no position of its own; it follows its ends.
    DiagramElement {
        kind: ElementKind::Relation,
        source: Some(flow.from.clone()),
        target: Some(flow.to.clone()),
        attributes: vec![
            ("product".into(), flow.product.clone()),
            ("volume".into(), number(flow.volume.unwrap_or(0.0))),
            ("hasVolume".into(), flow.volume.is_some().to_string()),
            ("volumeText".into(), volume),
            ("unit".into(), flow.unit.clone()),
            ("weight".into(), number(weight)),
            ("trace".into(), trace.of(&flow.id).to_owned()),
            ("lanes".into(), lanes),
        ],
        ..DiagramElement::node(&flow.id, FLOW_TYPE)
    }
}

/// A quantity or a volume as the canvas shows it (`supplyChainModel.ts`, `formatAmount`; `supply-chain.dis`, the
/// function `formatAmount`): grouped thousands, two decimals at most, three below one.
pub fn format_amount(value: f64) -> String {
    let pattern = if value != 0.0 && value.abs() < 1.0 {
        "#,##0.###"
    } else {
        "#,##0.##"
    };
    adp_disl::format_number(value, pattern).unwrap_or_else(|_| value.to_string())
}
