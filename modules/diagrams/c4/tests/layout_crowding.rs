//! What happens when a lot of elements have to leave one boundary at once (etalii.adp spec 030, task T061): ported
//! from the standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/C4LayoutCrowding.Tests.cs` at
//! `8b6ccfc`. Each outsider leaves by its own shortest route, so several arrive at one place; the sweep that separates
//! them must leave nothing drawn on anything else, keep what is inside the boundary inside it, and draw the same model
//! the same way twice, on synthetic models and on `crowded-component-view.dsl`, the model the lockstep was found on.

use std::path::Path;

use adp_diagram_c4::layout::{self, C4Layout};
use adp_diagram_c4::metrics::C4Metrics;
use adp_diagram_c4::model::C4Workspace;
use adp_diagram_c4::parser;

/// A container view whose system has two containers and `outsiders` other systems the API calls.
fn crowded(outsiders: usize) -> C4Workspace {
    let mut dsl = String::new();
    dsl.push_str("workspace {\n    model {\n");
    dsl.push_str("        subject = softwareSystem \"Subject\" \"The system in scope.\" {\n");
    dsl.push_str("            web = container \"Web\" \"Serves pages.\" \"React\"\n");
    dsl.push_str("            api = container \"Api\" \"Answers calls.\" \"Java\"\n");
    dsl.push_str("        }\n");
    dsl.push_str("        web -> api \"Calls\" \"HTTPS\"\n");
    for index in 0..outsiders {
        dsl.push_str(&format!(
            "        outside{index} = softwareSystem \"Outside {index}\" \"An external system.\"\n"
        ));
        dsl.push_str(&format!("        api -> outside{index} \"Calls\" \"HTTPS\"\n"));
    }
    dsl.push_str("    }\n    views {\n        container subject \"containers\" {\n            include *\n");
    dsl.push_str("        }\n    }\n}\n");
    parser::parse_text(&dsl)
}

fn compute(workspace: &C4Workspace, index: usize) -> C4Layout {
    layout::compute(workspace, &workspace.views[index], &C4Metrics::default(), None)
}

fn first_overlap(layout: &C4Layout) -> Option<(String, String)> {
    let boxes: Vec<_> = layout.boxes.iter().collect();
    for i in 0..boxes.len() {
        for j in i + 1..boxes.len() {
            if boxes[i].1.overlaps(&boxes[j].1) {
                return Some((boxes[i].0.to_owned(), boxes[j].0.to_owned()));
            }
        }
    }
    None
}

fn the_model_that_broke_it() -> C4Workspace {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/crowded-component-view.dsl");
    parser::parse_text(&std::fs::read_to_string(path).unwrap())
}

#[test]
fn however_many_elements_leave_the_boundary_none_end_up_drawn_on_another() {
    for outsiders in [2, 4, 8, 12, 20, 40] {
        let workspace = crowded(outsiders);
        let overlap = first_overlap(&compute(&workspace, 0));
        assert!(overlap.is_none(), "With {outsiders} outsiders, {overlap:?} overlap.");
    }
}

#[test]
fn the_elements_inside_the_boundary_stay_inside_it() {
    let workspace = crowded(20);
    let layout = compute(&workspace, 0);
    assert_eq!(layout.boundaries.len(), 1);
    let boundary = layout.boundaries[0].rect;
    for id in ["web", "api"] {
        let b = layout.boxes.get(id).unwrap();
        assert!(
            b.x >= boundary.x && b.right() <= boundary.right(),
            "'{id}' has left its boundary horizontally."
        );
        assert!(
            b.y >= boundary.y && b.bottom() <= boundary.bottom(),
            "'{id}' has left its boundary vertically."
        );
    }
}

#[test]
fn no_outsider_is_left_sitting_on_the_boundary() {
    let workspace = crowded(20);
    let layout = compute(&workspace, 0);
    assert_eq!(layout.boundaries.len(), 1);
    let boundary = layout.boundaries[0].rect;
    let outside: Vec<_> = layout
        .boxes
        .iter()
        .filter(|(id, _)| id.starts_with("outside"))
        .collect();
    assert!(!outside.is_empty(), "No box has an 'outside' key.");
    for (id, b) in outside {
        assert!(
            !b.overlaps(&boundary),
            "'{id}' is drawn on top of the boundary it was pushed out of."
        );
    }
}

#[test]
fn the_layout_is_stable_so_the_same_model_draws_the_same_twice() {
    let workspace = crowded(20);
    assert_eq!(compute(&workspace, 0).boxes, compute(&workspace, 0).boxes);
}

#[test]
fn on_the_model_this_was_found_on_nothing_is_drawn_on_anything_else() {
    let workspace = the_model_that_broke_it();
    assert!(
        !workspace.views.is_empty(),
        "No views were parsed from crowded-component-view.dsl."
    );
    let mut comparable = 0;
    for (index, view) in workspace.views.iter().enumerate() {
        let layout = compute(&workspace, index);
        if layout.boxes.len() >= 2 {
            comparable += 1;
        }
        let overlap = first_overlap(&layout);
        assert!(overlap.is_none(), "'{}': {overlap:?} overlap.", view.key);
    }
    assert!(comparable > 0, "No view laid out two or more boxes.");
}

#[test]
fn on_that_same_model_no_two_elements_share_a_position() {
    let workspace = the_model_that_broke_it();
    let index = workspace
        .views
        .iter()
        .position(|v| v.key == "OrderServiceComponents")
        .unwrap();
    let positions: Vec<(f64, f64)> = compute(&workspace, index)
        .boxes
        .iter()
        .map(|(_, r)| (r.x, r.y))
        .collect();
    for i in 0..positions.len() {
        for j in i + 1..positions.len() {
            assert_ne!(positions[i], positions[j], "two boxes share a position");
        }
    }
}

#[test]
fn on_that_same_model_the_layout_is_stable() {
    let workspace = the_model_that_broke_it();
    let index = workspace
        .views
        .iter()
        .position(|v| v.key == "OrderServiceComponents")
        .unwrap();
    assert_eq!(compute(&workspace, index).boxes, compute(&workspace, index).boxes);
}
