//! Where everything goes (etalii.adp spec 030, task T079): ranked so the execution story reads left to right, banded
//! so inventories do not lengthen it, and the same every time. The standalone host's `AnsibleLayout.Tests.cs` and
//! `AnsibleMetrics.Tests.cs` at `8b6ccfc`, with the same expectations.

mod common;

use std::collections::HashMap;
use std::fs;

use adp_diagram::element::Rect;
use adp_diagram_ansible_structure::definition::AnsibleDefinition;
use adp_diagram_ansible_structure::graph::AnsibleGraph;
use adp_diagram_ansible_structure::layout::{self, AnsibleMetrics};
use adp_diagram_ansible_structure::{plugin, reader};
use common::{model, read};

fn layout_of(fixture: &str) -> HashMap<String, Rect> {
    layout::compute(&model(fixture).diagram, &AnsibleMetrics::default())
}

fn sorted(boxes: &HashMap<String, Rect>) -> Vec<(String, Rect)> {
    let mut all: Vec<(String, Rect)> = boxes.iter().map(|(k, v)| (k.clone(), *v)).collect();
    all.sort_by(|a, b| a.0.cmp(&b.0));
    all
}

fn is_band(id: &str) -> bool {
    id.starts_with("inventory:") || id.starts_with("vars:")
}

// ---- determinism -------------------------------------------------------------------------------------------------------

#[test]
fn the_same_graph_yields_the_same_positions_twice() {
    assert_eq!(
        sorted(&layout_of("infrastructure")),
        sorted(&layout_of("infrastructure"))
    );
}

#[test]
fn every_node_is_placed() {
    let graph = AnsibleGraph::derive(&read("infrastructure"));
    let boxes = layout_of("infrastructure");
    assert_eq!(graph.nodes.len(), boxes.len());
    for node in &graph.nodes {
        assert!(boxes.contains_key(&node.id), "{} was not placed.", node.id);
    }
}

// ---- ranks -------------------------------------------------------------------------------------------------------------

#[test]
fn the_execution_story_reads_left_to_right() {
    let boxes = layout_of("infrastructure");
    let entry = boxes["playbook:site.yml"].x;
    let imported = boxes["playbook:webservers.yml"].x;
    let role = boxes["role:nginx"].x;
    let task_file = boxes["taskfile:roles/nginx/tasks/tls.yml"].x;
    assert!(entry < imported, "An entry playbook sits left of what it imports.");
    assert!(imported < role, "A playbook sits left of the roles it uses.");
    assert!(role < task_file, "A role sits left of the task files it includes.");
}

#[test]
fn two_playbooks_at_the_same_depth_share_a_column() {
    let boxes = layout_of("infrastructure");
    assert_eq!(boxes["playbook:webservers.yml"].x, boxes["playbook:dbservers.yml"].x);
    assert_ne!(boxes["playbook:webservers.yml"].y, boxes["playbook:dbservers.yml"].y);
}

#[test]
fn every_role_shares_one_column_so_the_band_of_roles_reads_as_one() {
    let boxes = layout_of("infrastructure");
    let mut columns: Vec<f64> = boxes
        .iter()
        .filter(|(k, _)| k.starts_with("role:"))
        .map(|(_, r)| r.x)
        .collect();
    columns.dedup();
    assert_eq!(columns.len(), 1, "{columns:?}");
}

#[test]
fn a_play_sits_right_of_the_playbook_that_declares_it() {
    let boxes = layout_of("broken");
    let playbook = boxes["playbook:playbooks/deploy.yml"].x;
    for (id, rect) in boxes.iter().filter(|(k, _)| k.starts_with("play:")) {
        assert!(rect.x > playbook, "{id} sits right of its playbook.");
    }
}

// ---- the band ----------------------------------------------------------------------------------------------------------

#[test]
fn inventories_and_variable_folders_sit_beneath_the_flow_rather_than_in_it() {
    let boxes = layout_of("infrastructure");
    let band_top: Vec<f64> = boxes.iter().filter(|(k, _)| is_band(k)).map(|(_, r)| r.y).collect();
    let flow_bottom = boxes
        .iter()
        .filter(|(k, _)| !is_band(k))
        .map(|(_, r)| r.bottom())
        .fold(f64::NEG_INFINITY, f64::max);
    assert!(!band_top.is_empty());
    assert!(
        band_top.iter().all(|y| *y > flow_bottom),
        "The band sits below everything in the flow."
    );
}

#[test]
fn the_band_is_one_row() {
    let boxes = layout_of("infrastructure");
    let mut rows: Vec<f64> = boxes.iter().filter(|(k, _)| is_band(k)).map(|(_, r)| r.y).collect();
    rows.dedup();
    assert_eq!(rows.len(), 1);
}

// ---- no overlap --------------------------------------------------------------------------------------------------------

#[test]
fn no_two_nodes_overlap() {
    for fixture in ["infrastructure", "broken"] {
        let boxes = sorted(&layout_of(fixture));
        for (i, (a, ra)) in boxes.iter().enumerate() {
            for (b, rb) in &boxes[i + 1..] {
                let overlaps = ra.x < rb.right() && rb.x < ra.right() && ra.y < rb.bottom() && rb.y < ra.bottom();
                assert!(!overlaps, "{fixture}: {a} overlaps {b}.");
            }
        }
    }
}

#[test]
fn a_long_node_name_widens_its_whole_column_rather_than_overlapping_its_neighbour() {
    let boxes = layout_of("infrastructure");
    let nginx = boxes["role:nginx"];
    assert!(
        nginx.width >= AnsibleMetrics::default().measure("postgres"),
        "The column fits its widest member."
    );
    assert_eq!(boxes["role:postgres"].width, nginx.width);
}

/// The boxes the standalone host's stream placed the parity fixture's nodes at, from the recorder's `--elements`.
#[test]
fn the_boxes_are_the_standalone_hosts_to_the_last_bit() {
    let boxes = layout_of("infrastructure");
    for (id, x, y) in [
        ("playbook:site.yml", 0.0, 0.0),
        ("playbook:dbservers.yml", 181.600_000_000_000_02, 0.0),
        ("playbook:webservers.yml", 181.600_000_000_000_02, 56.0),
        ("role:common", 409.400_000_000_000_03, 0.0),
        ("role:postgres", 409.400_000_000_000_03, 112.0),
        ("taskfile:roles/nginx/tasks/tls.yml", 591.0, 0.0),
        ("inventory:inventories/production", 0.0, 240.0),
        ("vars:inventories/production/group_vars", 197.0, 240.0),
        ("vars:inventories/production/host_vars", 394.0, 240.0),
        ("inventory:inventories/staging", 583.3, 240.0),
        ("vars:inventories/staging/group_vars", 757.199_999_999_999_9, 240.0),
    ] {
        assert_eq!((boxes[id].x, boxes[id].y), (x, y), "{id}");
    }
}

// ---- degenerate input --------------------------------------------------------------------------------------------------

#[test]
fn an_empty_graph_lays_out_to_nothing_rather_than_failing() {
    assert!(layout_of("unconventional").is_empty());
}

#[test]
fn an_import_cycle_terminates() {
    let scratch = tempfile::tempdir().unwrap();
    fs::write(scratch.path().join("a.yml"), "---\n- import_playbook: b.yml\n").unwrap();
    fs::write(scratch.path().join("b.yml"), "---\n- import_playbook: a.yml\n").unwrap();
    let project = reader::read_folder(scratch.path(), &AnsibleDefinition::get().binding);
    let boxes = layout::compute(&plugin::model_of(&project).diagram, &AnsibleMetrics::default());
    assert_eq!(boxes.len(), 2);
    assert_eq!(boxes["playbook:a.yml"].x, boxes["playbook:b.yml"].x);
}

// ---- metrics -----------------------------------------------------------------------------------------------------------

/// A text's width is the shared metric's width with this module's own sizing around it, against the shared metric's
/// golden fixture.
#[test]
fn a_texts_width_is_the_shared_metrics_width_with_this_modules_own_sizing_around_it() {
    let metrics = AnsibleMetrics::default();
    let fixture: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../crates/adp-layout/tests/fixtures/text-metric.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let cases: Vec<(String, f64)> = fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| (c["fontSize"].as_f64().unwrap() - metrics.font_size).abs() < 1e-6)
        .map(|c| (c["text"].as_str().unwrap().to_owned(), c["width"].as_f64().unwrap()))
        .collect();
    assert!(!cases.is_empty());
    for (text, width) in cases {
        let expected = metrics.minimum_width.max(width + 2.0 * metrics.horizontal_padding);
        assert!((metrics.measure(&text) - expected).abs() < 1e-9, "\"{text}\"");
    }
}
