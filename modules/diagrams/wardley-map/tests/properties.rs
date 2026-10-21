//! What the property grid shows for a Wardley element and what an edit of a row does, through the session
//! (etalii.adp spec 030, task T070), ported from the standalone host's `WardleyContextPropertyProvider.Tests.cs` at
//! `8b6ccfc`. `ItContributesToTheDiagramElementScope_AndRegistersInOneLine` has no counterpart: the module's view is
//! asked for its rows by its session.

mod common;

use std::fs;

use adp_context::{PropertyEditor, PropertyRow};
use adp_diagram_wardley_map::commands::MoveWardleyElement;
use adp_diagram_wardley_map::identities::kind;
use adp_diagram_wardley_map::properties as p;
use common::Project;

const MAP: &str = "title Tea Shop
anchor Business [0.95, 0.63]
component Cup of Tea [0.79, 0.61] label [-30, 12]
component Kettle [0.43, 0.35] (buy) inertia
component Power [0.1, 0.7]
evolve Kettle->Electric Kettle 0.62
Cup of Tea->Kettle
Kettle+>Power; cash
pipeline Kettle
{
  component Electric Kettle [0.63]
}";

fn tea() -> Project {
    Project::new(MAP)
}

fn row<'a>(rows: &'a [PropertyRow], id: &str) -> &'a PropertyRow {
    rows.iter()
        .find(|r| r.id == id)
        .unwrap_or_else(|| panic!("no row {id} in {rows:?}"))
}

fn editable(row: &PropertyRow) -> bool {
    row.read_only_reason.is_empty()
}

fn link_id(project: &Project) -> String {
    project
        .identities()
        .into_iter()
        .find(|e| e.kind == kind::LINK && e.key.contains("Power"))
        .unwrap()
        .id
}

// ---- what is shown --------------------------------------------------------------------------------------------------

#[test]
fn a_component_shows_what_requirement_15_2_asks() {
    let project = tea();
    let rows = project.rows(&project.id_of("Kettle"));
    for (id, value) in [
        (p::NAME, "Kettle"),
        (p::KIND, "component"),
        (p::VISIBILITY, "0.43"),
        (p::MATURITY, "0.35"),
        (p::STAGE, "Custom Built"),
        (p::EVOLVE, "0.62"),
        (p::EVOLVE_NAME, "Electric Kettle"),
        (p::INERTIA, "true"),
        (p::DECORATORS, "buy"),
    ] {
        assert_eq!(row(&rows, id).value, value, "{id}");
    }
}

#[test]
fn kind_and_decorators_are_separate_rows() {
    let project = tea();
    let rows = project.rows(&project.id_of("Kettle"));
    assert_eq!(row(&rows, p::KIND).value, "component");
    assert_eq!(row(&rows, p::DECORATORS).value, "buy");
}

#[test]
fn an_absent_property_is_not_contributed_at_all() {
    let project = tea();
    let kettle = project.rows(&project.id_of("Kettle"));
    let power = project.rows(&project.id_of("Power"));
    assert!(kettle.iter().any(|r| r.id == p::EVOLVE));
    assert!(!power.iter().any(|r| r.id == p::EVOLVE));
    assert!(!power.iter().any(|r| r.id == p::LABEL_OFFSET));
    assert!(
        project
            .rows(&project.id_of("Cup of Tea"))
            .iter()
            .any(|r| r.id == p::LABEL_OFFSET)
    );
}

#[test]
fn inertia_is_always_there_because_absent_is_false() {
    let project = tea();
    let power = project.rows(&project.id_of("Power"));
    let inertia = row(&power, p::INERTIA);
    assert_eq!(inertia.value, "false");
    assert_eq!(inertia.editor, PropertyEditor::Toggle);
}

#[test]
fn the_rows_are_grouped_the_way_requirement_15_9_asks() {
    let project = tea();
    let rows = project.rows(&project.id_of("Kettle"));
    assert_eq!(row(&rows, p::NAME).group, "Identity");
    assert_eq!(row(&rows, p::MATURITY).group, "Position");
    assert_eq!(row(&rows, p::STAGE).group, "Position");
    assert_eq!(row(&rows, p::EVOLVE).group, "Strategy");
    assert_eq!(row(&rows, p::DECORATORS).group, "Strategy");
}

#[test]
fn this_type_needs_no_new_editor() {
    let project = tea();
    let ids = [
        project.id_of("Kettle"),
        project.id_of("Cup of Tea"),
        link_id(&project),
        project.first_of(kind::PIPELINE_CHILD),
    ];
    for id in ids {
        for row in project.rows(&id) {
            assert!(
                matches!(
                    row.editor,
                    PropertyEditor::Line | PropertyEditor::Text | PropertyEditor::Toggle
                ),
                "{}",
                row.id
            );
        }
    }
}

#[test]
fn no_contributed_reason_is_blank() {
    let project = tea();
    let ids = [
        project.id_of("Kettle"),
        project.id_of("Cup of Tea"),
        project.id_of("Power"),
        link_id(&project),
        project.first_of(kind::PIPELINE_CHILD),
        project.first_of(kind::PIPELINE),
    ];
    for id in ids {
        for row in project.rows(&id) {
            assert!(
                row.read_only_reason.is_empty() || !row.read_only_reason.trim().is_empty(),
                "'{}' carries a whitespace-only reason",
                row.id
            );
        }
    }
}

#[test]
fn the_editable_rows_are_exactly_the_authors_own_claims() {
    let project = tea();
    let rows = project.rows(&project.id_of("Kettle"));
    for id in [
        p::NAME,
        p::VISIBILITY,
        p::MATURITY,
        p::EVOLVE,
        p::INERTIA,
        p::DECORATORS,
    ] {
        assert!(editable(row(&rows, id)), "{id}");
    }
    assert!(!editable(row(&rows, p::STAGE)));
}

#[test]
fn the_derived_stage_refuses_a_write_with_a_reason_naming_maturity() {
    let mut project = tea();
    let kettle = project.id_of("Kettle");
    let rows = project.rows(&kettle);
    assert!(
        row(&rows, p::STAGE)
            .read_only_reason
            .contains("Change Maturity instead")
    );
    let refusal = project.set(&kettle, p::STAGE, "Commodity").unwrap_err();
    assert!(refusal.contains("Maturity"), "{refusal}");
    assert_eq!(project.read(), MAP);
}

#[test]
fn a_link_shows_its_ends_its_kind_and_its_context() {
    let project = tea();
    let rows = project.rows(&link_id(&project));
    assert_eq!(row(&rows, p::LINK_SOURCE).value, "Kettle");
    assert_eq!(row(&rows, p::LINK_TARGET).value, "Power");
    assert_eq!(row(&rows, p::LINK_KIND).value, "flow");
    assert_eq!(row(&rows, p::LINK_CONTEXT).value, "cash");
    assert!(editable(row(&rows, p::LINK_CONTEXT)));
    assert!(!editable(row(&rows, p::LINK_SOURCE)));
}

#[test]
fn a_pipeline_childs_visibility_is_shown_as_its_parents_with_a_reason_saying_so() {
    let project = tea();
    let rows = project.rows(&project.first_of(kind::PIPELINE_CHILD));
    let visibility = row(&rows, p::VISIBILITY);
    assert_eq!(visibility.value, "0.43");
    assert!(!editable(visibility));
    assert!(
        visibility
            .read_only_reason
            .contains("takes its visibility from its parent")
    );
    assert!(visibility.read_only_reason.contains("Kettle"));
    assert!(editable(row(&rows, p::MATURITY)));
}

#[test]
fn a_pipeline_shows_its_children_and_their_positions() {
    let project = tea();
    let rows = project.rows(&project.first_of(kind::PIPELINE));
    assert_eq!(row(&rows, p::NAME).value, "Kettle");
    assert_eq!(row(&rows, p::VISIBILITY).value, "0.43");
    assert_eq!(row(&rows, p::PIPELINE_CHILDREN).value, "Electric Kettle (0.63)");
}

#[test]
fn in_read_only_mode_every_row_is_still_contributed_and_none_is_editable() {
    let mut project = tea();
    let kettle = project.id_of("Kettle");
    let ids = |rows: &[PropertyRow]| rows.iter().map(|r| r.id.clone()).collect::<Vec<_>>();
    let before = ids(&project.rows(&kettle));
    let mut permissions = fs::metadata(&project.body).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&project.body, permissions.clone()).unwrap();
    let rows = project.rows(&kettle);
    let refused = project.set(&kettle, p::MATURITY, "0.9");
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);
    fs::set_permissions(&project.body, permissions).unwrap();
    assert_eq!(ids(&rows), before);
    assert!(rows.iter().all(|r| !editable(r)));
    assert!(refused.is_err());
}

// ---- what editing one does ------------------------------------------------------------------------------------------

#[test]
fn editing_maturity_and_dragging_to_the_same_place_are_one_command_and_one_undo() {
    let mut project = tea();
    let kettle = project.id_of("Kettle");
    project.set(&kettle, p::MATURITY, "0.42").unwrap();
    let after_typing = project.read();
    project.undo();
    let after_undo = project.read();
    let drag = MoveWardleyElement {
        body_path: project.body.clone(),
        element_id: kettle,
        visibility: 0.43,
        maturity: 0.42,
    };
    project.execute(&drag).unwrap();
    assert_eq!(after_undo, MAP);
    assert_eq!(project.read(), after_typing);
}

#[test]
fn editing_visibility_leaves_maturity_where_it_was() {
    let mut project = tea();
    project.set(&project.id_of("Kettle"), p::VISIBILITY, "0.5").unwrap();
    assert!(project.read().contains("component Kettle [0.5, 0.35] (buy) inertia"));
}

#[test]
fn a_rejected_coordinate_leaves_the_document_unchanged() {
    let mut project = tea();
    let kettle = project.id_of("Kettle");
    let off_scale = project.set(&kettle, p::MATURITY, "1.4").unwrap_err();
    assert!(off_scale.contains("off the map"), "{off_scale}");
    assert!(project.set(&kettle, p::MATURITY, "quite evolved").is_err());
    assert_eq!(project.read(), MAP);
}

#[test]
fn a_rename_through_the_grid_rewrites_every_reference() {
    let mut project = tea();
    project.set(&project.id_of("Kettle"), p::NAME, "Boiler").unwrap();
    let text = project.read();
    assert!(text.contains("component Boiler [0.43, 0.35]"));
    assert!(text.contains("Cup of Tea->Boiler"));
    assert!(text.contains("evolve Boiler->Electric Kettle"));
}

#[test]
fn a_rename_to_a_taken_name_is_refused_with_the_reason() {
    let mut project = tea();
    let refusal = project.set(&project.id_of("Kettle"), p::NAME, "Power").unwrap_err();
    assert!(refusal.contains("already has an element"), "{refusal}");
    assert_eq!(project.read(), MAP);
}

#[test]
fn editing_the_decorators_row_is_one_undo() {
    let mut project = tea();
    project
        .set(&project.id_of("Kettle"), p::DECORATORS, "market, outsource")
        .unwrap();
    let after = project.read();
    project.undo();
    assert!(after.contains("(market)"));
    assert!(after.contains("(outsource)"));
    assert!(!after.contains("(buy)"));
    assert_eq!(project.read(), MAP);
}

#[test]
fn a_misspelled_decorator_is_refused_rather_than_quietly_dropped() {
    let mut project = tea();
    let refusal = project
        .set(&project.id_of("Kettle"), p::DECORATORS, "buy, outsorce")
        .unwrap_err();
    assert!(refusal.contains("outsorce"), "{refusal}");
    assert!(refusal.contains("market"), "{refusal}");
    assert_eq!(project.read(), MAP);
}

#[test]
fn clearing_the_decorators_row_takes_them_all_off() {
    let mut project = tea();
    project.set(&project.id_of("Kettle"), p::DECORATORS, "").unwrap();
    assert!(!project.read().contains("(buy)"));
}

#[test]
fn toggling_inertia_writes_and_removes_the_word() {
    let mut project = tea();
    let kettle = project.id_of("Kettle");
    project.set(&kettle, p::INERTIA, "false").unwrap();
    let off = project.read();
    project.set(&kettle, p::INERTIA, "true").unwrap();
    assert!(!off.contains("inertia"));
    assert!(project.read().contains("inertia"));
}

#[test]
fn setting_inertia_to_what_it_is_keeps_it_rather_than_flipping_it() {
    let mut project = tea();
    project.set(&project.id_of("Kettle"), p::INERTIA, "true").unwrap();
    assert!(project.read().contains("inertia"));
}

#[test]
fn emptying_the_evolve_row_clears_the_statement() {
    let mut project = tea();
    project.set(&project.id_of("Kettle"), p::EVOLVE, "").unwrap();
    assert!(!project.read().contains("evolve Kettle"));
}

#[test]
fn editing_the_evolve_target_keeps_the_name_it_arrives_under() {
    let mut project = tea();
    project.set(&project.id_of("Kettle"), p::EVOLVE, "0.8").unwrap();
    assert!(project.read().contains("evolve Kettle->Electric Kettle 0.8"));
}

#[test]
fn editing_a_links_context_rewrites_the_one_statement() {
    let mut project = tea();
    project.set(&link_id(&project), p::LINK_CONTEXT, "money").unwrap();
    assert!(project.read().contains("Kettle+>Power; money"));
}

#[test]
fn editing_a_pipeline_childs_maturity_moves_only_its_own_number() {
    let mut project = tea();
    project
        .set(&project.first_of(kind::PIPELINE_CHILD), p::MATURITY, "0.71")
        .unwrap();
    assert!(project.read().contains("  component Electric Kettle [0.71]"));
}

#[test]
fn an_element_that_is_gone_is_refused_with_a_message() {
    let mut project = tea();
    assert_eq!(
        project.set("no-such-id", p::NAME, "Anything").unwrap_err(),
        "That element is no longer on this map."
    );
}

#[test]
fn a_property_this_element_does_not_have_is_refused_rather_than_ignored() {
    let mut project = tea();
    assert!(project.set(&link_id(&project), p::INERTIA, "true").is_err());
    assert_eq!(project.read(), MAP);
}
