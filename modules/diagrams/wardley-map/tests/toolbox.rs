//! The palette and what dropping each entry writes (etalii.adp spec 030, task T070), ported from the standalone host's
//! `WardleyToolboxProvider.Tests.cs` at `8b6ccfc`. `ItIsKeyedByThisModulesOrigin` has no counterpart: the session asks
//! its view for the palette. A drop runs the entry's action on the map, as the standalone host committed it.

mod common;

use std::collections::HashSet;

use adp_diagram_wardley_map::actions;
use common::Project;

fn empty() -> Project {
    let project = Project::new("title Empty\n");
    assert!(
        project.session.toolbox(project.workspace.documents()).len() >= 8,
        "this guard has stopped finding the palette it guards"
    );
    project
}

/// Drops entry action `action_id` on the map and answers its prompt with `value`.
fn drop(project: &mut Project, action_id: &str, value: &str) -> Result<(), String> {
    project.answer("", action_id, value)
}

#[test]
fn it_offers_what_requirement_13_2_asks() {
    let project = empty();
    let labels: Vec<String> = project
        .session
        .toolbox(project.workspace.documents())
        .into_iter()
        .map(|i| i.label)
        .collect();
    assert_eq!(
        labels,
        [
            "Component",
            "Anchor",
            "Market",
            "Ecosystem",
            "Submap",
            "Pipeline",
            "Note",
            "Annotation"
        ]
    );
}

#[test]
fn there_is_no_link_entry() {
    let project = empty();
    assert!(
        !project
            .session
            .toolbox(project.workspace.documents())
            .iter()
            .any(|i| i.label.to_lowercase().contains("link"))
    );
}

#[test]
fn every_entry_is_described() {
    let project = empty();
    let items = project.session.toolbox(project.workspace.documents());
    for item in &items {
        assert!(!item.id.trim().is_empty(), "{}", item.label);
        assert!(!item.icon.trim().is_empty(), "{}", item.label);
        assert!(!item.description.trim().is_empty(), "{}", item.label);
        assert!(!item.drop_action_id.trim().is_empty(), "{}", item.label);
    }
    let ids: HashSet<&str> = items.iter().map(|i| i.id.as_str()).collect();
    assert_eq!(ids.len(), items.len());
}

#[test]
fn every_drop_action_id_resolves_to_an_action_this_module_actually_contributes() {
    let mut project = empty();
    let mut offered: HashSet<String> = project.action_ids_for("").into_iter().collect();
    drop(&mut project, actions::ADD_COMPONENT, "Kettle").unwrap();
    offered.extend(project.action_ids_for(&project.id_of("Kettle")));
    for item in project.session.toolbox(project.workspace.documents()) {
        assert!(
            offered.contains(&item.drop_action_id),
            "'{}' names an action nothing contributes: {}",
            item.label,
            item.drop_action_id
        );
    }
}

#[test]
fn dropping_a_component_entry_writes_one_component() {
    let mut project = empty();
    drop(&mut project, actions::ADD_COMPONENT, "Kettle").unwrap();
    assert_eq!(project.read(), "title Empty\ncomponent Kettle [0.5, 0.5]\n");
}

#[test]
fn dropping_a_market_entry_writes_a_component_carrying_the_decorator() {
    let mut project = empty();
    drop(&mut project, actions::ADD_MARKET, "Tea Buyers").unwrap();
    assert_eq!(
        project.read(),
        "title Empty\ncomponent Tea Buyers [0.5, 0.5] (market)\n"
    );
}

#[test]
fn dropping_a_market_entry_is_one_undo() {
    let mut project = empty();
    let before = project.read();
    drop(&mut project, actions::ADD_MARKET, "Tea Buyers").unwrap();
    project.undo();
    assert_eq!(project.read(), before);
}

#[test]
fn dropping_an_ecosystem_entry_writes_the_other_decorator() {
    let mut project = empty();
    drop(&mut project, actions::ADD_ECOSYSTEM, "Plugins").unwrap();
    assert!(project.read().contains("(ecosystem)"));
}

#[test]
fn dropping_an_anchor_entry_writes_the_user_need() {
    let mut project = empty();
    drop(&mut project, actions::ADD_ANCHOR, "Business").unwrap();
    assert!(project.read().contains("anchor Business [0.5, 0.5]"));
}

#[test]
fn dropping_a_note_entry_pins_the_text() {
    let mut project = empty();
    drop(&mut project, actions::ADD_NOTE, "Mind the gap").unwrap();
    assert_eq!(project.read(), "title Empty\nnote Mind the gap [0.5, 0.5]\n");
}

#[test]
fn dropping_an_annotation_entry_numbers_it_past_the_highest_already_there() {
    let mut project = Project::new("annotation 1 [0.2, 0.2] first\nannotation 3 [0.3, 0.3] third\n");
    drop(&mut project, actions::ADD_ANNOTATION, "fourth").unwrap();
    assert!(project.read().contains("annotation 4 [0.5, 0.5] fourth"));
}

#[test]
fn dropping_an_annotation_entry_starts_at_one_on_a_map_with_none() {
    let mut project = empty();
    drop(&mut project, actions::ADD_ANNOTATION, "the first thing to say").unwrap();
    assert!(
        project
            .read()
            .contains("annotation 1 [0.5, 0.5] the first thing to say")
    );
}

#[test]
fn a_note_or_annotation_with_nothing_to_say_is_refused() {
    let mut project = empty();
    let refusal = actions::validate(actions::ADD_NOTE, "   ").unwrap_err();
    assert!(refusal.contains("something to say"), "{refusal}");
    assert!(drop(&mut project, actions::ADD_NOTE, "   ").is_err());
    assert_eq!(project.read(), "title Empty\n");
}

#[test]
fn dropping_the_pipeline_entry_asks_for_the_first_component_to_put_in_it() {
    let mut project = empty();
    drop(&mut project, actions::ADD_COMPONENT, "Kettle").unwrap();
    let kettle = project.id_of("Kettle");
    assert!(matches!(
        project.ask(&kettle, actions::ADD_TO_PIPELINE),
        adp_context::Prompt::Input { .. }
    ));
    project
        .answer(&kettle, actions::ADD_TO_PIPELINE, "Electric Kettle")
        .unwrap();
    assert!(project.read().contains("pipeline Kettle"));
}
