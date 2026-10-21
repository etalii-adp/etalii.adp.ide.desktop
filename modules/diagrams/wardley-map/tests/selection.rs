//! What a selection of a Wardley element reads as (etalii.adp spec 030, task T070), ported from the standalone host's
//! `WardleyContextSourceResolver.Tests.cs` at `8b6ccfc`, through the session's `select`. Without a counterpart: the
//! context service's own plumbing (`ItAnswersForElementIds_AndNothingElse`, `NothingNestsInsideAnElement`,
//! `AnElementWithNoDiagramAboveIt_IsRejected`, `AClientPathThatDisagreesWithTheElement_IsRejected`,
//! `AFileOfAnotherType_IsRejectedPlainly`, `ADisposedSubscription_StopsAnnouncing`): the desktop session asks the
//! module of its own document by id.

mod common;

use adp_diagram_wardley_map::identities::{SEPARATOR, kind};
use adp_diagram_wardley_map::mapper::{ELEMENT, EVOLUTION_AXIS_ID};
use common::Project;

const MAP: &str = "title Tea Shop
anchor Business [0.95, 0.63]
component Cup of Tea [0.79, 0.61]
component Kettle [0.43, 0.35] (buy)
component Power [0.1, 0.7]
Cup of Tea->Kettle
Kettle+>Power
pipeline Kettle
{
  component Electric Kettle [0.45]
}
note This is a note [0.2, 0.2]
pioneers [0.30, 0.20, 0.55, 0.45]";

fn id_of(project: &Project, kind: &str, key: &str) -> String {
    project
        .identities()
        .into_iter()
        .find(|e| e.kind == kind && e.key == key)
        .unwrap_or_else(|| panic!("no {kind} {key}"))
        .id
}

#[test]
fn a_component_resolves_to_its_own_name_and_path() {
    let project = Project::new(MAP);
    let selected = project
        .session
        .select(project.workspace.documents(), &project.id_of("Kettle"))
        .unwrap();
    assert_eq!(selected.path, ["Kettle"]);
    assert_eq!(selected.text, "Kettle");
}

#[test]
fn a_component_that_is_a_pipeline_parent_says_it_has_children() {
    let project = Project::new(MAP);
    let map = parse(MAP);
    let identities = project.identities();
    let kettle = adp_diagram_wardley_map::descriptions::of(&map, &identities, &project.id_of("Kettle")).unwrap();
    let power = adp_diagram_wardley_map::descriptions::of(&map, &identities, &project.id_of("Power")).unwrap();
    assert!(kettle.has_children);
    assert!(!power.has_children);
}

fn parse(text: &str) -> adp_diagram_wardley_map::model::WardleyMap {
    common::parse_text(text)
}

#[test]
fn a_pipeline_child_is_named_beneath_its_parent() {
    let project = Project::new(MAP);
    let child = id_of(
        &project,
        kind::PIPELINE_CHILD,
        &format!("Kettle{SEPARATOR}Electric Kettle"),
    );
    let selected = project.session.select(project.workspace.documents(), &child).unwrap();
    assert_eq!(selected.path, ["Kettle", "Electric Kettle"]);
    assert_eq!(selected.text, "Electric Kettle");
}

#[test]
fn a_link_is_described_by_what_it_joins() {
    let project = Project::new(MAP);
    let dependency = id_of(
        &project,
        kind::LINK,
        &format!("Cup of Tea{SEPARATOR}Kettle{SEPARATOR}Dependency"),
    );
    let flow = id_of(&project, kind::LINK, &format!("Kettle{SEPARATOR}Power{SEPARATOR}Flow"));
    let documents = project.workspace.documents();
    assert_eq!(
        project.session.select(documents, &dependency).unwrap().text,
        "Cup of Tea -> Kettle"
    );
    assert_eq!(
        project.session.select(documents, &flow).unwrap().text,
        "Kettle +> Power"
    );
}

#[test]
fn a_note_and_a_region_are_selectable_too() {
    let project = Project::new(MAP);
    let note = id_of(&project, kind::NOTE, "This is a note");
    let attitude = id_of(
        &project,
        kind::ATTITUDE,
        &format!("Pioneers{SEPARATOR}0.3{SEPARATOR}0.2"),
    );
    let documents = project.workspace.documents();
    assert_eq!(project.session.select(documents, &note).unwrap().text, "This is a note");
    assert_eq!(project.session.select(documents, &attitude).unwrap().text, "Pioneers");
}

#[test]
fn the_selection_carries_the_elements_own_payload() {
    let project = Project::new(MAP);
    let selected = project
        .session
        .select(project.workspace.documents(), &project.id_of("Kettle"))
        .unwrap();
    assert_eq!(selected.element.element_type, ELEMENT);
    assert_eq!(selected.element.attribute("name"), "Kettle");
    assert_eq!(selected.element.attribute("kind"), "component");
    assert_eq!(selected.element.attribute("visibility"), "0.43");
    assert_eq!(selected.element.attribute("maturity"), "0.35");
    assert_eq!(selected.element.attribute("stage"), "Custom Built");
}

#[test]
fn the_delivered_payload_is_the_one_the_canvas_already_has() {
    let project = Project::new(MAP);
    let cup = project.id_of("Cup of Tea");
    let streamed = project
        .session
        .render(project.workspace.documents())
        .unwrap()
        .into_iter()
        .find(|e| e.id == cup)
        .unwrap();
    let selected = project.session.select(project.workspace.documents(), &cup).unwrap();
    assert_eq!(selected.element, streamed);
}

#[test]
fn an_unknown_element_is_rejected() {
    let project = Project::new(MAP);
    assert!(
        project
            .session
            .select(project.workspace.documents(), "not-an-id")
            .is_none()
    );
}

#[test]
fn the_evolution_axis_is_not_selectable() {
    let project = Project::new(MAP);
    assert!(
        project
            .session
            .select(project.workspace.documents(), EVOLUTION_AXIS_ID)
            .is_none()
    );
}

#[test]
fn a_removed_element_clears_the_selection() {
    let mut project = Project::new(MAP);
    let power = project.id_of("Power");
    let command = adp_diagram_wardley_map::commands::RemoveWardleyElement {
        body_path: project.body.clone(),
        element_id: power.clone(),
    };
    project.execute(&command).unwrap();
    assert!(project.session.select(project.workspace.documents(), &power).is_none());
}

#[test]
fn a_rename_announces_the_new_path_rather_than_clearing_the_selection() {
    let mut project = Project::new(MAP);
    let power = project.id_of("Power");
    let command = adp_diagram_wardley_map::commands::RenameWardleyElement {
        body_path: project.body.clone(),
        element_id: power.clone(),
        new_name: "Mains Power".to_owned(),
    };
    project.execute(&command).unwrap();
    let selected = project.session.select(project.workspace.documents(), &power).unwrap();
    assert_eq!(selected.path, ["Mains Power"]);
}
