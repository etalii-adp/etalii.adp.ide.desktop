//! The keys that add an entry directly below the selected one, with no dialog (the standalone host's
//! `ContextActions.Tests.cs` at a7fdf03; spec 029 T074, FR-012, US4 scenario 3; etalii.adp spec 036, T057): offered
//! for an entry of a requirement document with their shortcuts, found by those shortcuts, each one step of the
//! document's history; and nothing offered for another tool's Markdown file.

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use adp_designer_hierarchical_requirement::context_actions::{self, ADD_BELOW};
use adp_designer_hierarchical_requirement::operations::structure;
use adp_designer_hierarchical_requirement::session::RequirementSession;
use adp_desl::hierarchy::model::Gesture;
use adp_desl::hierarchy::session::HierarchySurfaceSession;
use common::{WebShopFolder, read};

/// Runs the action `id`'s entry offers for `key`, Shift held or not, as the page does: its operation sent for it.
fn press(session: &mut RequirementSession, body: &Path, id: &str, shift: bool) {
    let offered = context_actions::discover(body, id);
    let action = context_actions::by_shortcut(&offered, "Enter", shift).expect("an action takes the key");
    let gesture = Gesture {
        edit_id: format!("edit-{}-{id}", action.id),
        operation: action.operation.clone(),
        target_id: id.into(),
        params: BTreeMap::new(),
    };
    assert_eq!(Ok(()), session.gesture(gesture));
    session.take_changes();
}

fn children(body: &Path, parent: &str) -> Vec<String> {
    read(body)
        .document
        .entries
        .iter()
        .filter(|e| e.parent_id.as_deref() == Some(parent))
        .map(|e| e.id.clone())
        .collect()
}

#[test]
fn an_item_offers_an_item_below_and_a_requirement_and_a_requirement_a_requirement_below_each_with_its_shortcut() {
    let folder = WebShopFolder::new();

    let item = context_actions::discover(&folder.body(), "ITEM-0003");
    let requirement = context_actions::discover(&folder.body(), "REQ-0005");

    // Enter below, Shift+Enter a requirement in the item; Ctrl or Cmd is held for both.
    let shown = |actions: &[adp_desl::hierarchy::model::SurfaceAction]| -> Vec<(String, bool, String, bool)> {
        actions
            .iter()
            .map(|a| (a.label.clone(), a.unavailable.is_empty(), a.key.clone(), a.shift))
            .collect()
    };
    assert_eq!(
        vec![
            ("New item below".to_owned(), true, "Enter".to_owned(), false),
            ("New requirement".to_owned(), true, "Enter".to_owned(), true),
        ],
        shown(&item)
    );
    assert_eq!(
        vec![("New requirement below".to_owned(), true, "Enter".to_owned(), false)],
        shown(&requirement)
    );
    assert_eq!(ADD_BELOW, requirement[0].id);
}

#[test]
fn each_shortcut_adds_where_the_specification_says_as_one_step_of_the_history() {
    let folder = WebShopFolder::new();
    let body = folder.body();
    let before = fs::read(&body).unwrap();
    let mut session = RequirementSession::open(&body, Some(folder.registration()));

    // An item below ITEM-0003, then a requirement in ITEM-0006, then one below REQ-0009.
    press(&mut session, &body, "ITEM-0003", false);
    press(&mut session, &body, "ITEM-0006", true);
    press(&mut session, &body, "REQ-0009", false);

    // ITEM-0008 after ITEM-0003 in ITEM-0002; REQ-0014 after REQ-0010, the last requirement of ITEM-0006 (the
    // placeholder took REQ-0013); REQ-0015 directly below REQ-0009.
    assert_eq!(
        vec!["REQ-0003", "REQ-0004", "ITEM-0003", "ITEM-0008"],
        children(&body, "ITEM-0002")
    );
    assert_eq!(
        vec!["REQ-0009", "REQ-0015", "REQ-0010", "REQ-0014"],
        children(&body, "ITEM-0006")
    );
    let entries = read(&body).document.entries;
    let title = |id: &str| entries.iter().find(|e| e.id == id).unwrap().title.clone();
    assert_eq!(
        ("New item".to_owned(), "New requirement".to_owned()),
        (title("ITEM-0008"), title("REQ-0015"))
    );

    // And each was one step: three undos give the file back.
    for _ in 0..3 {
        assert_eq!(Ok(()), session.undo());
    }
    assert_eq!(before, fs::read(&body).unwrap());
}

#[test]
fn a_requirement_in_no_item_is_offered_nothing_below_with_the_reason_and_another_tools_file_nothing_at_all() {
    let folder = tempfile::tempdir().unwrap();
    let alone = folder.path().join("alone.md");
    fs::write(
        &alone,
        "<!-- ded: 0.2 designer: etalii/hierarchical-requirement -->\r\n# Alone\r\n\r\n## Requirements\r\n\r\n- REQ-0001 Stands in no item.\r\n",
    )
    .unwrap();
    let notes = folder.path().join("notes.md");
    fs::write(&notes, "# Notes\r\n\r\n- REQ-0001 A bullet that is no requirement.\r\n").unwrap();

    let offered = context_actions::discover(&alone, "REQ-0001");
    let other = context_actions::discover(&notes, "REQ-0001");

    let below = offered
        .iter()
        .find(|a| a.id == ADD_BELOW)
        .expect("the action is offered");
    assert_eq!(structure::NO_ITEM, below.unavailable);
    assert!(context_actions::by_shortcut(&offered, "Enter", false).is_none());
    assert!(other.is_empty());
}

#[test]
fn a_document_that_cannot_change_offers_nothing() {
    let folder = WebShopFolder::new();
    folder.write_as_an_agent(|text| text.replace("<!-- ded: 0.2 ", "<!-- ded: 0.3 "));

    assert!(context_actions::discover(&folder.body(), "ITEM-0003").is_empty());
}
