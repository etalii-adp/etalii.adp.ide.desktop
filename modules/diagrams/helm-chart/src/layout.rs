//! The computed layout: the chart's anatomy in five bands, left to right, deterministic for a given model (etalii.adp
//! spec 030, task T080).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/helm-chart/backend/EtAlii.Adp.Diagram.HelmChart/
//! HelmLayout.cs`, the layout the definition names as its plugin's (`plugin:net.etalii.adp.helm.anatomyBands`). The
//! metadata band (chart, lock, schema, crds), the values stack (the default layer first), the templates (manifests,
//! partials, notes, test hooks), the dependencies and what `charts/` holds, each a column of boxes of the kind's fixed
//! size, 24 apart, the columns 60 apart; an empty column takes no room. Stored positions win over all of this element
//! by element, their sizes staying the computed ones.

use std::collections::{BTreeMap, HashMap};

use adp_cel::Value;
use adp_diagram::element::Rect;
use adp_disl::{DislDiagram, DislElement};

use crate::mapper::{name_of, path_of};

const LEFT: f64 = 40.0;
const TOP: f64 = 40.0;
const COLUMN_GAP: f64 = 60.0;
const ROW_GAP: f64 = 24.0;

/// The size of a node of DISL type `type_name`.
pub fn size_of(type_name: &str) -> (f64, f64) {
    match type_name {
        "Chart" => (220.0, 110.0),
        "Values" => (200.0, 72.0),
        "Schema" | "Crds" => (200.0, 48.0),
        "Template" | "Partial" => (240.0, 60.0),
        "Dependency" => (210.0, 96.0),
        "Subchart" => (210.0, 84.0),
        "Archive" => (210.0, 52.0),
        "Lock" => (200.0, 56.0),
        _ => (200.0, 60.0),
    }
}

fn rank(node: &DislElement) -> u8 {
    match node.type_name().as_str() {
        "Chart" => 0,
        "Lock" => 1,
        "Schema" => 2,
        "Crds" => 3,
        _ => 4,
    }
}

/// Manifests first, then partials, notes and test hooks, by the template's derived `role`.
fn role_rank(node: &DislElement) -> u8 {
    match node.value_of("role") {
        Value::String(role) => match role.as_str() {
            "manifest" => 0,
            "partial" => 1,
            "notes" => 2,
            "test" => 3,
            _ => 4,
        },
        _ => 4,
    }
}

/// The computed box of every node, in the order they were placed; a second node of an id already placed takes the
/// first one's slot, as a dictionary keeps it.
pub fn compute(diagram: &DislDiagram) -> Vec<(String, Rect)> {
    let nodes = diagram.nodes();
    let of = |types: &[&str]| -> Vec<DislElement> {
        nodes
            .iter()
            .filter(|n| types.contains(&n.type_name().as_str()))
            .cloned()
            .collect()
    };

    let mut metadata = of(&["Chart", "Lock", "Schema", "Crds"]);
    metadata.sort_by_key(rank);
    let mut values = of(&["Values"]);
    values.sort_by(|a, b| {
        let default = |n: &DislElement| u8::from(n.value_of("isDefault") != Value::Bool(true));
        default(a).cmp(&default(b)).then_with(|| path_of(a).cmp(&path_of(b)))
    });
    let mut templates: Vec<DislElement> = nodes.iter().filter(|n| n.is_a("Template")).cloned().collect();
    templates.sort_by(|a, b| {
        role_rank(a)
            .cmp(&role_rank(b))
            .then_with(|| path_of(a).cmp(&path_of(b)))
    });
    let mut dependencies = of(&["Dependency"]);
    dependencies.sort_by_key(name_of);
    let mut vendored: Vec<DislElement> = nodes.iter().filter(|n| n.is_a("Vendored")).cloned().collect();
    vendored.sort_by_key(path_of);

    let mut boxes = Vec::new();
    let mut x = LEFT;
    for column in [metadata, values, templates, dependencies, vendored] {
        x = place_column(&mut boxes, &column, x);
    }
    boxes
}

fn place_column(boxes: &mut Vec<(String, Rect)>, nodes: &[DislElement], x: f64) -> f64 {
    let mut y = TOP;
    let mut widest: f64 = 0.0;
    for node in nodes {
        let (width, height) = size_of(&node.type_name());
        let rect = Rect::new(x, y, width, height);
        let id = node.id();
        match boxes.iter_mut().find(|(placed, _)| *placed == id) {
            Some((_, placed)) => *placed = rect,
            None => boxes.push((id, rect)),
        }
        y += height + ROW_GAP;
        widest = widest.max(width);
    }
    // An empty column takes no room, so a chart without crds or vendored content carries no hole where they would be.
    if widest > 0.0 { x + widest + COLUMN_GAP } else { x }
}

/// Every box the chart has: the computed ones, the stored positions winning element by element, the sizes staying the
/// computed ones; a stale stored id has nothing to place.
pub fn arranged(diagram: &DislDiagram, stored: &BTreeMap<String, (f64, f64)>) -> HashMap<String, Rect> {
    compute(diagram)
        .into_iter()
        .map(|(id, mut rect)| {
            if let Some(&(x, y)) = stored.get(&id) {
                rect.x = x;
                rect.y = y;
            }
            (id, rect)
        })
        .collect()
}
