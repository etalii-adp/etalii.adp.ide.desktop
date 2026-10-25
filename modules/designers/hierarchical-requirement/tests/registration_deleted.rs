//! Deleting a requirement document's registration loses nothing an agent reads (the standalone host's
//! `RegistrationDeleted.Tests.cs` at a7fdf03): the document stays, an open page goes on with the lists' default
//! colours and order, reopening creates no registration until a setting changes, and with one created again every
//! tag, category, state, reference and assessment is there. Only colours, order, what was collapsed and stored
//! filters are gone.
//!
//! The standalone host's tests change a tag's colour for the setting that creates the registration again, and an
//! entry's title for the edit that does not; until etalii.adp spec 036 builds those operations, the setting here is a
//! fold, the one this module has, and the edit of the text is left to the tests that build it.

mod common;

use std::fs;

use adp_designer_hierarchical_requirement::definition;
use adp_designer_hierarchical_requirement::session::RequirementSession;
use adp_desl::hierarchy::model::{Gesture, SurfaceChange, SurfaceModel};
use adp_desl::hierarchy::session::HierarchySurfaceSession;
use common::WebShopFolder;

/// What the Markdown file holds of the page: each node's tags, category and state, and its references; the
/// assessments; and the names of each list's members, whose order and colours are the registration's.
fn kept(model: &SurfaceModel) -> String {
    let nodes: Vec<String> = model
        .nodes
        .iter()
        .map(|node| {
            let labels: Vec<String> = node
                .labels
                .iter()
                .map(|values| format!("{}={}", values.label, values.values.join(",")))
                .collect();
            let references: Vec<&str> = node.references.iter().map(|r| r.target_id.as_str()).collect();
            format!("{} {} {}", node.id, labels.join(" "), references.join(","))
        })
        .collect();
    let notes: Vec<String> = model
        .notes
        .iter()
        .map(|n| format!("{} {} {} {} {}", n.id, n.kind, n.about, n.text, n.resolved))
        .collect();
    let lists: Vec<String> = model
        .option_lists
        .iter()
        .map(|list| {
            let mut names: Vec<&str> = list.options.iter().map(|o| o.name.as_str()).collect();
            names.sort();
            format!("{}: {}", list.label, names.join(","))
        })
        .collect();
    [nodes.join("\n"), notes.join("\n"), lists.join("\n")].join("\n\n")
}

/// The tags the page's entries carry, once each in any case, sorted.
fn carried(model: &SurfaceModel) -> Vec<String> {
    let mut tags: Vec<String> = model
        .nodes
        .iter()
        .flat_map(|n| &n.labels)
        .filter(|v| v.label == "tags")
        .flat_map(|v| &v.values)
        .map(|t| t.to_lowercase())
        .collect();
    tags.sort();
    tags.dedup();
    tags
}

fn colours(model: &SurfaceModel, label: &str) -> Vec<(String, String)> {
    model
        .option_lists
        .iter()
        .find(|l| l.label == label)
        .unwrap()
        .options
        .iter()
        .filter(|o| !o.implicit)
        .map(|o| (o.name.clone(), o.colour.clone()))
        .collect()
}

fn toggle(target: &str) -> Gesture {
    Gesture {
        edit_id: "fold".into(),
        operation: "toggleCollapsed".into(),
        target_id: target.into(),
        ..Gesture::default()
    }
}

fn run(session: &mut RequirementSession, gesture: Gesture) {
    assert_eq!(Ok(()), session.gesture(gesture));
    let changes = session.take_changes();
    let written = changes
        .iter()
        .any(|c| matches!(c, SurfaceChange::Outcome(o) if o.written));
    assert!(written, "{changes:?}");
}

#[test]
fn an_open_page_goes_on_with_the_default_colours_and_order_and_the_document_stays() {
    let folder = WebShopFolder::new();
    let mut session = RequirementSession::open(folder.body(), Some(folder.registration()));
    let body = fs::read(folder.body()).unwrap();
    let before = session.model();

    fs::remove_file(folder.registration()).unwrap();
    session.file_changed(&folder.registration());

    // The page is sent the document without a registration, and the body is untouched.
    let changes = session.take_changes();
    let Some(SurfaceChange::Model(after)) = changes.last() else {
        panic!("no model was sent: {changes:?}")
    };
    assert!(after.filters.is_empty());
    let tags = &after.option_lists.iter().find(|l| l.label == "tags").unwrap().options;
    let mut names: Vec<String> = tags.iter().map(|t| t.name.to_lowercase()).collect();
    names.sort();
    assert_eq!(carried(&before), names);
    assert!(tags.iter().all(|t| t.colour == "default" && t.implicit));
    assert_eq!(definition::initial("categories"), colours(after, "category"));
    assert_eq!(definition::initial("states"), colours(after, "state"));
    assert!(!after.nodes.iter().any(|n| n.collapsed));
    assert_eq!(kept(&before), kept(after));
    assert_eq!(body, fs::read(folder.body()).unwrap());
    assert!(!folder.registration().exists());
}

#[test]
fn reopening_creates_no_registration_until_a_setting_changes() {
    let folder = WebShopFolder::new();
    fs::remove_file(folder.registration()).unwrap();

    let mut session = RequirementSession::open(folder.body(), None);
    let created_by_opening = folder.registration().exists();
    run(&mut session, toggle("ITEM-0001"));

    assert!(!created_by_opening);
    assert!(folder.registration().exists());
    assert!(
        fs::read_to_string(folder.registration())
            .unwrap()
            .contains("  ITEM-0001: yes\r\n")
    );
}

#[test]
fn deleted_and_created_again_the_document_has_every_tag_category_state_reference_and_assessment() {
    let folder = WebShopFolder::new();
    let before = kept(&RequirementSession::open(folder.body(), Some(folder.registration())).model());

    // Deleted, then created again by the first setting of a page opened without it.
    fs::remove_file(folder.registration()).unwrap();
    let mut session = RequirementSession::open(folder.body(), None);
    run(&mut session, toggle("ITEM-0001"));
    let reopened = RequirementSession::open(folder.body(), Some(folder.registration()));

    // The same document.
    assert!(folder.registration().exists());
    assert_eq!(before, kept(&reopened.model()));
}
