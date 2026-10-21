//! What a person can do to a Wardley element, offered and run through the session (etalii.adp spec 030, task T070),
//! ported from the standalone host's `WardleyContextActionProvider.Tests.cs` at `8b6ccfc`. Without a counterpart:
//! `OnlyTheRenamePrompt_IsMarkedForInlineEditing` (the desktop host's input prompt carries no inline-label element;
//! the canvas renames a name in place from F2 on its own), `ItContributesToTheDiagramElementScope_AndNoOther` and
//! `AFileThatIsNotAMap_IsAnsweredWithNothing` (the module is asked only for its own documents, by its tool type).

mod common;

use std::fs;

use adp_context::Prompt;
use adp_diagram::ActionOutcome;
use adp_diagram_wardley_map::actions::{self, decorator_action_id};
use adp_diagram_wardley_map::identities::kind;
use adp_diagram_wardley_map::model::WardleyDecorator;
use common::Project;

const MAP: &str = "title Tea Shop
component Cup of Tea [0.79, 0.61]
component Kettle [0.43, 0.35] (buy)
component Power [0.1, 0.7]
Cup of Tea->Kettle
Kettle+>Power";

fn tea() -> Project {
    Project::new(MAP)
}

fn options(prompt: &Prompt) -> Vec<(String, String)> {
    match prompt {
        Prompt::Choice { options, .. } => options.iter().map(|o| (o.id.clone(), o.label.clone())).collect(),
        other => panic!("not a choice: {other:?}"),
    }
}

// ---- what is offered ------------------------------------------------------------------------------------------------

#[test]
fn a_component_is_offered_the_edits_of_requirement_9_3() {
    let project = tea();
    let ids = project.action_ids_for(&project.id_of("Kettle"));
    for id in [
        actions::RENAME,
        actions::REMOVE,
        actions::SET_EVOLVE,
        actions::TOGGLE_INERTIA,
        actions::LINK,
        actions::ADD_TO_PIPELINE,
        &decorator_action_id(WardleyDecorator::Buy),
    ] {
        assert!(ids.iter().any(|i| i == id), "{id} in {ids:?}");
    }
}

#[test]
fn every_shortcut_is_described_by_the_backend() {
    let project = tea();
    let offered = project.actions_for(&project.id_of("Kettle"));
    let key = |id: &str| {
        offered
            .iter()
            .find(|a| a.id == id)
            .and_then(|a| a.shortcut.as_ref())
            .map(|s| s.key.clone())
    };
    assert_eq!(key(actions::RENAME).as_deref(), Some("F2"));
    assert_eq!(key(actions::REMOVE).as_deref(), Some("Delete"));
}

#[test]
fn a_label_says_which_way_the_toggle_goes() {
    let project = tea();
    let id = decorator_action_id(WardleyDecorator::Buy);
    let label = |name: &str| {
        project
            .actions_for(&project.id_of(name))
            .into_iter()
            .find(|a| a.id == id)
            .unwrap()
            .label
    };
    assert_eq!(label("Kettle"), "Clear buy");
    assert_eq!(label("Power"), "Mark as buy");
}

#[test]
fn an_unlink_is_not_offered_on_an_element_with_no_link() {
    let project = Project::new("component Alone [0.5, 0.5]\n");
    assert!(
        !project
            .action_ids_for(&project.id_of("Alone"))
            .iter()
            .any(|i| i == actions::UNLINK)
    );
}

#[test]
fn stop_evolving_is_only_offered_on_something_that_is_evolving() {
    let project = Project::new("component Kettle [0.4, 0.3]\ncomponent Power [0.1, 0.7]\nevolve Kettle 0.7\n");
    assert!(
        project
            .action_ids_for(&project.id_of("Kettle"))
            .iter()
            .any(|i| i == actions::CLEAR_EVOLVE)
    );
    assert!(
        !project
            .action_ids_for(&project.id_of("Power"))
            .iter()
            .any(|i| i == actions::CLEAR_EVOLVE)
    );
}

#[test]
fn a_legacy_pipeline_is_offered_no_membership() {
    let project = Project::new("component Power [0.1, 0.7]\npipeline Power [0.30, 0.85]\n");
    assert!(
        !project
            .action_ids_for(&project.id_of("Power"))
            .iter()
            .any(|i| i == actions::ADD_TO_PIPELINE)
    );
}

#[test]
fn the_map_itself_is_offered_the_adds() {
    let project = tea();
    assert_eq!(
        project.action_ids_for(""),
        [
            actions::ADD_COMPONENT,
            actions::ADD_ANCHOR,
            actions::ADD_SUBMAP,
            actions::ADD_MARKET,
            actions::ADD_ECOSYSTEM,
            actions::ADD_NOTE,
            actions::ADD_ANNOTATION,
        ]
    );
}

#[test]
fn a_read_only_file_is_offered_nothing() {
    let project = tea();
    let kettle = project.id_of("Kettle");
    let mut permissions = fs::metadata(&project.body).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&project.body, permissions.clone()).unwrap();
    let ids = project.action_ids_for(&kettle);
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);
    fs::set_permissions(&project.body, permissions).unwrap();
    assert!(ids.is_empty(), "{ids:?}");
}

#[test]
fn a_note_is_offered_nothing_because_no_command_edits_one_yet() {
    let project = Project::new("component Alpha [0.5, 0.5]\nnote Mind the gap [0.2, 0.2]\n");
    assert!(project.action_ids_for(&project.first_of(kind::NOTE)).is_empty());
}

// ---- what they do ---------------------------------------------------------------------------------------------------

#[test]
fn toggle_inertia_edits_the_file_through_the_history() {
    let mut project = tea();
    let power = project.id_of("Power");
    assert_eq!(project.run(&power, actions::TOGGLE_INERTIA), ActionOutcome::Done);
    assert!(project.read().contains("component Power [0.1, 0.7] inertia"));
}

#[test]
fn toggle_inertia_is_undone_by_the_projects_history() {
    let mut project = tea();
    let before = project.read();
    let power = project.id_of("Power");
    project.run(&power, actions::TOGGLE_INERTIA);
    project.undo();
    assert_eq!(project.read(), before);
}

#[test]
fn a_decorator_toggle_adds_and_then_removes_the_same_word() {
    let mut project = tea();
    let kettle = project.id_of("Kettle");
    let id = decorator_action_id(WardleyDecorator::Buy);
    project.run(&kettle, &id);
    let cleared = project.read();
    project.run(&kettle, &id);
    assert!(!cleared.contains("(buy)"));
    assert!(project.read().contains("(buy)"));
}

#[test]
fn rename_asks_first_and_then_commits_the_command() {
    let mut project = tea();
    let kettle = project.id_of("Kettle");
    match project.ask(&kettle, actions::RENAME) {
        Prompt::Input { initial_value, .. } => assert_eq!(initial_value, "Kettle"),
        other => panic!("{other:?}"),
    }
    project.answer(&kettle, actions::RENAME, "Boiler").unwrap();
    let text = project.read();
    assert!(text.contains("component Boiler [0.43, 0.35]"));
    assert!(text.contains("Cup of Tea->Boiler"));
}

#[test]
fn remove_asks_for_confirmation_because_it_takes_more_than_the_statement_in_front_of_the_user() {
    let mut project = tea();
    let kettle = project.id_of("Kettle");
    match project.ask(&kettle, actions::REMOVE) {
        Prompt::Confirm { danger, message, .. } => {
            assert!(danger);
            assert!(message.contains("every link"), "{message}");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn link_to_offers_every_other_element_and_links_the_chosen_one() {
    let mut project = tea();
    let cup = project.id_of("Cup of Tea");
    let offered = options(&project.ask(&cup, actions::LINK));
    let labels: Vec<&str> = offered.iter().map(|(_, l)| l.as_str()).collect();
    assert_eq!(labels, ["Kettle", "Power"]);
    let power = offered.iter().find(|(_, l)| l == "Power").unwrap().0.clone();
    project.answer(&cup, actions::LINK, &power).unwrap();
    assert!(project.read().contains("Cup of Tea->Power"));
}

#[test]
fn flow_to_writes_the_other_arrow() {
    let mut project = tea();
    let cup = project.id_of("Cup of Tea");
    let offered = options(&project.ask(&cup, actions::FLOW));
    let power = offered.iter().find(|(_, l)| l == "Power").unwrap().0.clone();
    project.answer(&cup, actions::FLOW, &power).unwrap();
    assert!(project.read().contains("Cup of Tea+>Power"));
}

#[test]
fn unlink_with_one_link_does_not_ask_which_one() {
    let mut project = Project::new("component Alpha [0.5, 0.5]\ncomponent Beta [0.2, 0.2]\nAlpha->Beta\n");
    let alpha = project.id_of("Alpha");
    assert_eq!(project.run(&alpha, actions::UNLINK), ActionOutcome::Done);
    assert_eq!(
        project.read(),
        "component Alpha [0.5, 0.5]\ncomponent Beta [0.2, 0.2]\n"
    );
}

#[test]
fn unlink_with_several_links_asks_which_one_and_removes_only_that_one() {
    let mut project = tea();
    let kettle = project.id_of("Kettle");
    let offered = options(&project.ask(&kettle, actions::UNLINK));
    let labels: Vec<&str> = offered.iter().map(|(_, l)| l.as_str()).collect();
    assert_eq!(labels, ["Cup of Tea -> Kettle", "Kettle +> Power"]);
    let chosen = offered.iter().find(|(_, l)| l == "Kettle +> Power").unwrap().0.clone();
    project.answer(&kettle, actions::UNLINK, &chosen).unwrap();
    let text = project.read();
    assert!(text.contains("Cup of Tea->Kettle"));
    assert!(!text.contains("Kettle+>Power"));
}

#[test]
fn a_link_itself_is_selectable_and_removable() {
    let mut project = tea();
    let link = project.first_of(kind::LINK);
    assert_eq!(project.action_ids_for(&link), [actions::UNLINK]);
    assert_eq!(project.run(&link, actions::UNLINK), ActionOutcome::Done);
    assert!(!project.read().contains("Cup of Tea->Kettle"));
}

#[test]
fn evolve_refuses_a_number_off_the_scale_and_accepts_one_on_it() {
    let refusal = actions::validate(actions::SET_EVOLVE, "1.4").unwrap_err();
    assert!(refusal.contains("between 0 and 1"), "{refusal}");
    assert!(actions::validate(actions::SET_EVOLVE, "0.8").is_ok());
}

#[test]
fn evolve_starts_from_where_the_element_is_when_it_is_not_evolving_yet() {
    let mut project = tea();
    let kettle = project.id_of("Kettle");
    match project.ask(&kettle, actions::SET_EVOLVE) {
        Prompt::Input { initial_value, .. } => assert_eq!(initial_value, "0.35"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn add_to_pipeline_commits_the_membership_command() {
    let mut project = tea();
    let kettle = project.id_of("Kettle");
    project
        .answer(&kettle, actions::ADD_TO_PIPELINE, "Electric Kettle")
        .unwrap();
    assert!(project.read().contains("  component Electric Kettle [0.5]"));
}

#[test]
fn a_pipeline_child_is_offered_only_its_removal() {
    let mut project =
        Project::new("component Kettle [0.4, 0.3]\npipeline Kettle\n{\n  component Electric Kettle [0.63]\n}\n");
    let child = project.first_of(kind::PIPELINE_CHILD);
    assert_eq!(project.action_ids_for(&child), [actions::REMOVE_FROM_PIPELINE]);
    assert_eq!(project.run(&child, actions::REMOVE_FROM_PIPELINE), ActionOutcome::Done);
    assert_eq!(project.read(), "component Kettle [0.4, 0.3]\npipeline Kettle\n{\n}\n");
}

#[test]
fn an_element_that_is_gone_is_refused_with_a_message_rather_than_throwing() {
    let mut project = tea();
    assert_eq!(
        project.run("no-such-id", actions::RENAME),
        ActionOutcome::Refused("That element is no longer on this map.".to_owned())
    );
}

#[test]
fn evolve_from_the_menu_keeps_the_name_it_arrives_under() {
    let mut project = Project::new("component Kettle [0.43, 0.35]\nevolve Kettle->Electric Kettle 0.62\n");
    let kettle = project.id_of("Kettle");
    project.answer(&kettle, actions::SET_EVOLVE, "0.8").unwrap();
    assert_eq!(
        project.read(),
        "component Kettle [0.43, 0.35]\nevolve Kettle->Electric Kettle 0.8\n"
    );
}

#[test]
fn a_document_that_could_not_be_read_is_refused_by_the_menus_and_the_grid_and_writes_nothing() {
    // A folder where the body should be: it exists, and it cannot be read.
    let mut project = Project::with_body_folder();
    let kettle = "any-id";
    assert!(project.actions_for(kettle).is_empty());
    assert!(project.actions_for("").is_empty());
    match project.run(kettle, actions::TOGGLE_INERTIA) {
        ActionOutcome::Refused(refusal) => assert!(refusal.contains("could not be read"), "{refusal}"),
        other => panic!("{other:?}"),
    }
    let refusal = project.answer(kettle, actions::RENAME, "Boiler").unwrap_err();
    assert!(refusal.contains("could not be read"), "{refusal}");
    let refusal = project.set(kettle, "wardley.name", "Boiler").unwrap_err();
    assert!(refusal.contains("could not be read"), "{refusal}");
    assert!(project.body.is_dir());
}
