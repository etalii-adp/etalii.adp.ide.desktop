//! One open map through the session: its baseline, its viewport, a drag and a change from outside (etalii.adp spec
//! 030, task T070), ported from the standalone host's `WardleySession.Tests.cs` at `8b6ccfc`. Without a counterpart:
//! `Baseline_IncludesAGroupForAPipeline` (the desktop host draws no pipeline box and folds none, so the module hands
//! its pipeline groups to no one; `mapper::group` keeps them and `tests/mapper.rs` checks them),
//! `MoveElementToAsync_RefusesOnAReadOnlyMap` (a desktop session always has its project's history) and
//! `DisposeAsync_UnsubscribesFromTheStore` (a session hears of a change only when it is asked).

mod common;

use std::fs;

use adp_diagram::{DiagramDelta, DiagramElement, DiagramViewport, DocumentChange};
use adp_diagram_wardley_map::mapper::{ELEMENT, EVOLUTION_AXIS, LINK};
use adp_diagram_wardley_map::view::PARENT_REFUSAL;
use common::Project;

fn added(deltas: &[DiagramDelta]) -> Vec<DiagramElement> {
    deltas
        .iter()
        .filter_map(|d| match d {
            DiagramDelta::Add(elements) => Some(elements.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

fn removed(deltas: &[DiagramDelta]) -> Vec<String> {
    deltas
        .iter()
        .filter_map(|d| match d {
            DiagramDelta::Remove(ids) => Some(ids.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

fn baseline(project: &mut Project) -> Vec<DiagramDelta> {
    project.session.baseline(project.workspace.documents()).unwrap()
}

fn update_view(project: &mut Project, viewport: DiagramViewport) -> Vec<DiagramDelta> {
    project
        .session
        .update_view(project.workspace.documents(), viewport)
        .unwrap()
}

#[test]
fn baseline_sends_the_whole_map_as_one_add() {
    let mut project = Project::new("title Tea\ncomponent Cup of Tea [0.79, 0.61]\nanchor Customer [0.95, 0.63]\n");
    let deltas = baseline(&mut project);
    assert_eq!(deltas.len(), 1);
    // Two elements and the evolution axis.
    assert_eq!(added(&deltas).len(), 3);
}

#[test]
fn baseline_of_an_empty_map_is_just_the_axis() {
    let mut project = Project::new("title Empty\n");
    let elements = added(&baseline(&mut project));
    assert_eq!(elements.len(), 1);
    assert_eq!(elements[0].element_type, EVOLUTION_AXIS);
}

#[test]
fn update_view_with_an_unbounded_viewport_sends_nothing() {
    let mut project = Project::new("component Alpha [0.5, 0.5]\n");
    baseline(&mut project);
    assert!(update_view(&mut project, DiagramViewport::UNBOUNDED).is_empty());
}

#[test]
fn update_view_answering_a_viewport_change_removes_what_left_then_adds_what_appeared() {
    // Alpha sits near (0.1, 0.1) and Beta near (0.9, 0.9).
    let mut project = Project::new("component Alpha [0.9, 0.1]\ncomponent Beta [0.1, 0.9]\n");
    baseline(&mut project);
    let beta = project.id_of("Beta");
    let narrowed = update_view(&mut project, DiagramViewport::new(0.0, 0.0, 0.4, 0.4));
    let moved = update_view(&mut project, DiagramViewport::new(0.6, 0.6, 1.0, 1.0));
    assert!(added(&narrowed).is_empty());
    assert_eq!(removed(&narrowed), std::slice::from_ref(&beta));
    let came = added(&moved);
    assert_eq!(came.len(), 1);
    assert_eq!(came[0].id, beta);
    assert_eq!(removed(&moved).len(), 1);
    // Remove comes before add.
    assert!(matches!(moved[0], DiagramDelta::Remove(_)));
    assert!(matches!(moved[1], DiagramDelta::Add(_)));
}

#[test]
fn a_document_change_under_a_narrowed_viewport_does_not_resend_the_culled_elements() {
    let mut project = Project::new("component Alpha [0.9, 0.1]\ncomponent Beta [0.1, 0.9]\n");
    baseline(&mut project);
    update_view(&mut project, DiagramViewport::new(0.0, 0.0, 0.4, 0.4));
    fs::write(
        &project.body,
        "component Alpha [0.85, 0.15]\ncomponent Beta [0.1, 0.9]\n",
    )
    .unwrap();
    let body = project.body.clone();
    project.workspace.reconcile(&body);
    let change = project
        .session
        .on_document_changed(project.workspace.documents(), &body);
    let DocumentChange::Changed(deltas) = change else {
        panic!("{change:?}");
    };
    let arrived = added(&deltas);
    assert!(!arrived.is_empty());
    assert!(arrived.iter().all(|e| e.x <= 0.5), "{arrived:?}");
}

#[test]
fn update_view_keeps_the_axis_and_a_link_whose_endpoint_is_in_view() {
    let mut project = Project::new("component Alpha [0.9, 0.1]\ncomponent Beta [0.1, 0.9]\nAlpha->Beta\n");
    baseline(&mut project);
    let deltas = update_view(&mut project, DiagramViewport::new(0.6, 0.6, 1.0, 1.0));
    assert!(removed(&deltas).is_empty());
    let held = project.session.render(project.workspace.documents()).unwrap();
    assert!(held.iter().any(|e| e.element_type == EVOLUTION_AXIS));
    assert!(held.iter().any(|e| e.element_type == LINK));
    assert_eq!(held.iter().filter(|e| e.element_type == ELEMENT).count(), 2);
}

#[test]
fn move_element_is_refused_because_dragging_is_not_reparenting() {
    let mut project = Project::new("component Alpha [0.5, 0.5]\n");
    let refusal = project
        .session
        .move_element(&mut project.workspace, "any", Some("other"), 0)
        .unwrap_err();
    assert_eq!(refusal, PARENT_REFUSAL);
    assert_eq!(
        refusal,
        "Dragging a component changes where it sits on the map, not what contains it."
    );
}

#[test]
fn move_element_to_writes_the_documents_own_axes_back_into_the_file() {
    let mut project = Project::new("component Alpha [0.90, 0.10]\n");
    let alpha = project.id_of("Alpha");
    // Canvas (0.8, 0.4) is maturity 0.8, visibility 0.6.
    project
        .session
        .move_element_to(&mut project.workspace, &alpha, 0.8, 0.4)
        .unwrap();
    assert_eq!(project.read(), "component Alpha [0.6, 0.8]\n");
}

#[test]
fn move_element_to_changes_only_the_line_it_touches() {
    let mut project = Project::new(
        "// A map with things worth keeping.\ntitle Tea shop\ncomponent Alpha [0.90, 0.10] // why it sits here\ncomponent Beta [0.50, 0.50]\nAlpha->Beta",
    );
    let alpha = project.id_of("Alpha");
    project
        .session
        .move_element_to(&mut project.workspace, &alpha, 0.2, 0.7)
        .unwrap();
    let after = project.read();
    assert!(after.contains("// A map with things worth keeping."));
    assert!(
        after.contains("component Alpha [0.3, 0.2] // why it sits here"),
        "{after}"
    );
    assert!(after.contains("component Beta [0.50, 0.50]"));
    assert!(after.contains("Alpha->Beta"));
}

#[test]
fn move_element_to_clamps_a_point_outside_the_map() {
    let mut project = Project::new("component Alpha [0.5, 0.5]\n");
    let alpha = project.id_of("Alpha");
    project
        .session
        .move_element_to(&mut project.workspace, &alpha, 1.7, -0.4)
        .unwrap();
    assert_eq!(project.read(), "component Alpha [1, 1]\n");
}

#[test]
fn move_element_to_is_undoable_and_the_undo_restores_the_file_exactly() {
    let text = "component Alpha [0.90, 0.10] // kept\n";
    let mut project = Project::new(text);
    let alpha = project.id_of("Alpha");
    project
        .session
        .move_element_to(&mut project.workspace, &alpha, 0.8, 0.4)
        .unwrap();
    let moved = project.read();
    project.undo();
    assert_ne!(moved, text);
    assert_eq!(project.read(), text);
}

#[test]
fn move_element_to_reports_a_refusal_for_an_element_that_is_not_there() {
    let mut project = Project::new("component Alpha [0.5, 0.5]\n");
    assert_eq!(
        project
            .session
            .move_element_to(&mut project.workspace, "not-an-element", 0.5, 0.5)
            .unwrap_err(),
        "That element is no longer on this map."
    );
}

#[test]
fn move_element_to_moves_a_pipeline_child_along_its_evolution_axis_only() {
    let mut project =
        Project::new("component Kettle [0.43, 0.35]\npipeline Kettle\n{\n  component Electric [0.63]\n}\n");
    let child = project.first_of(adp_diagram_wardley_map::identities::kind::PIPELINE_CHILD);
    project
        .session
        .move_element_to(&mut project.workspace, &child, 0.9, 0.9)
        .unwrap();
    let after = project.read();
    assert!(after.contains("component Electric [0.9]"));
    assert!(after.contains("component Kettle [0.43, 0.35]"));
}

#[test]
fn document_changed_sends_the_difference_to_the_connection() {
    let mut project = Project::new("component Alpha [0.9, 0.1]\n");
    baseline(&mut project);
    fs::write(&project.body, "component Alpha [0.2, 0.8]\n").unwrap();
    let body = project.body.clone();
    project.workspace.reconcile(&body);
    let change = project
        .session
        .on_document_changed(project.workspace.documents(), &body);
    let DocumentChange::Changed(deltas) = change else {
        panic!("{change:?}");
    };
    // One add carrying the element in its new state, never a remove and an add.
    assert_eq!(added(&deltas).len(), 1);
    assert!(removed(&deltas).is_empty());
}

#[test]
fn document_changed_ignores_another_maps_change() {
    let mut project = Project::with_files(
        "component Alpha [0.9, 0.1]\n",
        &[("other.owm", "component Beta [0.5, 0.5]\n")],
    );
    baseline(&mut project);
    let other = project.root.join("other.owm");
    let change = project
        .session
        .on_document_changed(project.workspace.documents(), &other);
    assert!(matches!(change, DocumentChange::OtherDocument), "{change:?}");
}
