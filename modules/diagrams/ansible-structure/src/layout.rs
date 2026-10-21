//! Where every node goes: ranked left to right so the execution story reads in one direction, with inventories and
//! variable folders banded beneath it (etalii.adp spec 030, task T079).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/ansible-structure/backend/
//! EtAlii.Adp.Diagram.AnsibleStructure/AnsibleLayout.cs` and `_Model/AnsibleMetrics.cs`, the layout the definition
//! names as its plugin's (`layout.algorithms.ranks`, gap 10). Pure: the definition's model in, boxes out. Entry
//! playbooks first (the ones nothing imports), then what they import by longest path, then plays, then roles, then task
//! files; every node of a rank at the rank's width, in the model's order. Inventories and variable folders get no rank
//! and sit in a band beneath the flow.

use std::collections::HashMap;

use adp_diagram::element::Rect;
use adp_disl::DislDiagram;
use adp_layout::cycle_policy::CyclePolicy;

use crate::definition::text;

/// The numbers the layout arranges by (`AnsibleMetrics`), in canvas units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnsibleMetrics {
    pub font_size: f64,
    /// Inside a node box, each side.
    pub horizontal_padding: f64,
    /// Every node is one line tall.
    pub node_height: f64,
    /// So a short name is still something to click on.
    pub minimum_width: f64,
    /// Between one rank's right edge and the next rank's left edge.
    pub rank_gap: f64,
    /// Between two nodes stacked within one rank.
    pub row_gap: f64,
    /// Between the execution flow and the inventory band beneath it.
    pub band_gap: f64,
}

impl Default for AnsibleMetrics {
    fn default() -> Self {
        AnsibleMetrics {
            font_size: 14.0,
            horizontal_padding: 12.0,
            node_height: 32.0,
            minimum_width: 72.0,
            rank_gap: 96.0,
            row_gap: 24.0,
            band_gap: 96.0,
        }
    }
}

impl AnsibleMetrics {
    /// The width a node labelled `text` needs.
    pub fn measure(&self, text: &str) -> f64 {
        self.minimum_width
            .max(adp_layout::text_metric::width_of(text, self.font_size) + 2.0 * self.horizontal_padding)
    }
}

/// The box of every node of `diagram`, by id.
pub fn compute(diagram: &DislDiagram, metrics: &AnsibleMetrics) -> HashMap<String, Rect> {
    let nodes = diagram.nodes();
    let ranks = ranks(diagram);
    let mut boxes = HashMap::new();

    // The flow: everything that has a rank, laid out in columns, in the model's order within a column.
    let mut columns: Vec<(i32, Vec<usize>)> = Vec::new();
    for (index, node) in nodes.iter().enumerate() {
        let Some(&rank) = ranks.get(&node.id()) else {
            continue;
        };
        match columns.iter_mut().find(|(r, _)| *r == rank) {
            Some((_, members)) => members.push(index),
            None => columns.push((rank, vec![index])),
        }
    }
    columns.sort_by_key(|(rank, _)| *rank);

    let mut x = 0.0;
    let mut flow_bottom: f64 = 0.0;
    for (_, members) in &columns {
        let width = members
            .iter()
            .map(|&i| metrics.measure(&name_of(&nodes[i])))
            .fold(f64::NEG_INFINITY, f64::max);
        let mut y = 0.0;
        for &i in members {
            boxes.insert(nodes[i].id(), Rect::new(x, y, width, metrics.node_height));
            y += metrics.node_height + metrics.row_gap;
        }
        flow_bottom = flow_bottom.max(y - metrics.row_gap);
        x += width + metrics.rank_gap;
    }

    // The band: inventories and variable folders, beneath the flow rather than in it.
    let band_y = flow_bottom + metrics.band_gap;
    let mut band_x = 0.0;
    for node in nodes.iter().filter(|n| !ranks.contains_key(&n.id())) {
        let width = metrics.measure(&name_of(node));
        boxes.insert(node.id(), Rect::new(band_x, band_y, width, metrics.node_height));
        band_x += width + metrics.rank_gap;
    }
    boxes
}

/// Which column each node belongs in; inventories and variable folders get none.
fn ranks(diagram: &DislDiagram) -> HashMap<String, i32> {
    let nodes = diagram.nodes();
    let mut ranks: HashMap<String, i32> = HashMap::new();

    // Playbook depth by longest path over imports, each playbook measured by a walk of its own, so a half-written
    // cycle neither hangs the diagram nor depends on which playbook came first.
    let imports: Vec<(String, String)> = diagram
        .relations()
        .iter()
        .filter(|r| r.type_name() == "ImportsPlaybook")
        .filter_map(|r| Some((r.source()?.id(), r.target()?.id())))
        .collect();
    let playbooks: Vec<String> = nodes
        .iter()
        .filter(|n| n.type_name() == "Playbook")
        .map(|n| n.id())
        .collect();
    let depths = adp_layout::layering::longest_path(
        playbooks.iter().cloned().chain(imports.iter().map(|(s, _)| s.clone())),
        imports.iter().map(|(s, t)| (s.as_str(), t.as_str())),
        CyclePolicy::BackEdgeIsZeroPerNode,
    );
    for playbook in &playbooks {
        ranks.insert(playbook.clone(), depths.get(playbook).copied().unwrap_or(0));
    }
    let deepest_playbook = ranks.values().copied().max().unwrap_or(0);

    // A play sits one column right of the playbook that declares it.
    for play in nodes.iter().filter(|n| n.type_name() == "Play") {
        let owner = playbook_id_of(&play.id());
        let depth = ranks.get(&owner).copied().unwrap_or(deepest_playbook);
        ranks.insert(play.id(), depth + 1);
    }
    let deepest = ranks.values().copied().max().unwrap_or(0);
    for role in nodes.iter().filter(|n| n.type_name() == "Role") {
        ranks.insert(role.id(), deepest + 1);
    }
    for file in nodes.iter().filter(|n| n.type_name() == "TaskFile") {
        ranks.insert(file.id(), deepest + 2);
    }
    ranks
}

fn name_of(node: &adp_disl::DislElement) -> String {
    text(&node.value_of("name"))
}

/// The playbook a play's id belongs to: `play:file#2` names `playbook:file`.
fn playbook_id_of(play_id: &str) -> String {
    let rest = play_id.strip_prefix("play:").unwrap_or(play_id);
    let path = rest.rsplit_once('#').map_or(rest, |(path, _)| path);
    format!("playbook:{path}")
}
