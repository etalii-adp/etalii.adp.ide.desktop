//! One open requirement document on one connection, as a hierarchy surface (the standalone host's
//! `HierarchicalRequirementSession.Tests.cs` at a7fdf03, and of `HierarchySurfaceFlow.Tests.cs` what is the session's
//! rather than the host's routing, which etalii.adp spec 036 builds with the shell): the ids its model gives by the
//! contract, the document shown whole writing nothing, collapsing kept in the registration alone and no undo step,
//! and a document of a later DED version opened read-only with the reason.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use adp_designer_hierarchical_requirement::session::RequirementSession;
use adp_desl::hierarchy::model::{Gesture, SurfaceChange, SurfaceModel, causes, roles};
use adp_desl::hierarchy::session::HierarchySurfaceSession;
use common::{WebShopFolder, examples_folder};

/// The example's body, copied into a folder without its registration, as an agent would leave it.
fn copy_body(folder: &Path, change: impl Fn(&str) -> String) -> PathBuf {
    let text = fs::read_to_string(examples_folder().join("web-shop.md")).unwrap();
    let path = folder.join("web-shop.md");
    fs::write(&path, change(&text)).unwrap();
    path
}

/// The line ending the example was written with, so an inserted line keeps it.
fn newline_of(text: &str) -> &'static str {
    if text.contains("\r\n") { "\r\n" } else { "\n" }
}

fn with_a_wish_list(title: &'static str) -> impl Fn(&str) -> String {
    move |text: &str| {
        text.replace(
            "  * REQ-0001 A customer finds",
            &format!("  * REQ-NEW {title}{}  * REQ-0001 A customer finds", newline_of(text)),
        )
    }
}

fn toggle(target: &str) -> Gesture {
    Gesture {
        edit_id: format!("edit-{target}"),
        operation: "toggleCollapsed".into(),
        target_id: target.into(),
        ..Gesture::default()
    }
}

fn collapsed(model: &SurfaceModel, id: &str) -> bool {
    model.nodes.iter().find(|node| node.id == id).unwrap().collapsed
}

#[test]
fn a_numbered_entry_is_named_by_its_number_and_a_placeholder_by_its_line() {
    let folder = tempfile::tempdir().unwrap();
    let body = copy_body(folder.path(), with_a_wish_list("A customer keeps a wish list."));
    let line = fs::read_to_string(&body)
        .unwrap()
        .lines()
        .position(|text| text.contains("REQ-NEW A customer keeps"))
        .unwrap()
        + 1;
    let session = RequirementSession::open(&body, None);

    let model = session.model();

    // The numbered ones by their number, the new one by the line of its bullet.
    assert_eq!("", model.read_only_reason);
    assert_eq!("Web shop", model.title);
    assert!(
        model
            .nodes
            .iter()
            .any(|n| n.id == "ITEM-0001" && n.number == "ITEM-0001" && n.role == roles::LEVEL)
    );
    assert!(
        model
            .nodes
            .iter()
            .any(|n| n.id == "REQ-0001" && n.parent_id == "ITEM-0001" && n.role == roles::ENTRY)
    );
    let added: Vec<_> = model
        .nodes
        .iter()
        .filter(|n| n.title == "A customer keeps a wish list.")
        .collect();
    assert_eq!(1, added.len());
    assert_eq!(format!("line/{line}"), added[0].id);
    assert_eq!("ITEM-0001", added[0].parent_id);
}

#[test]
fn collapsing_writes_only_the_registration_creating_it_and_is_kept_for_the_next_opening() {
    // No registration yet.
    let folder = tempfile::tempdir().unwrap();
    let body = copy_body(folder.path(), str::to_owned);
    let registration = body.with_extension("adp");
    let before = fs::read(&body).unwrap();
    {
        let mut session = RequirementSession::open(&body, None);
        assert!(!collapsed(&session.model(), "REQ-0001"));

        let accepted = session.gesture(toggle("REQ-0001"));

        // Accepted, the registration created with the fold and nothing else of the body touched.
        assert_eq!(Ok(()), accepted);
        let written = fs::read_to_string(&registration).unwrap();
        assert!(written.starts_with("etalii/hierarchical-requirement"), "{written}");
        assert!(written.contains("collapsed:"), "{written}");
        assert!(written.contains("REQ-0001: yes"), "{written}");
        assert_eq!(before, fs::read(&body).unwrap());
        assert!(collapsed(&session.model(), "REQ-0001"));
    }

    // The document opened again: collapsed as it was left.
    let mut reopened = RequirementSession::open(&body, None);
    assert!(collapsed(&reopened.model(), "REQ-0001"));

    // Expanded again: the entry gone from the block, the body still untouched.
    assert_eq!(Ok(()), reopened.gesture(toggle("REQ-0001")));
    assert!(!fs::read_to_string(&registration).unwrap().contains("REQ-0001: yes"));
    assert!(!collapsed(&reopened.model(), "REQ-0001"));
    assert_eq!(before, fs::read(&body).unwrap());
}

#[test]
fn an_entry_without_a_number_is_not_collapsed_in_the_document() {
    let folder = tempfile::tempdir().unwrap();
    let body = copy_body(folder.path(), with_a_wish_list("A wish list."));
    let mut session = RequirementSession::open(&body, None);
    let added = session
        .model()
        .nodes
        .into_iter()
        .find(|n| n.title == "A wish list.")
        .unwrap();

    let refused = session.gesture(toggle(&added.id));

    // Refused with the reason, and nothing written.
    assert_eq!(
        Err("REQ-NEW has no number of its own yet, so it is collapsed on this page only.".to_owned()),
        refused
    );
    assert!(!body.with_extension("adp").exists());
    assert!(session.take_changes().is_empty());
}

#[test]
fn a_document_of_a_later_version_opens_read_only_with_the_reason() {
    let folder = tempfile::tempdir().unwrap();
    let body = copy_body(folder.path(), |text| text.replace("<!-- ded: 0.2 ", "<!-- ded: 0.3 "));
    let mut session = RequirementSession::open(&body, None);

    let model = session.model();
    let refused = session.gesture(toggle("REQ-0001"));

    // Shown, with the reason, and nothing it asks is done.
    assert!(model.read_only_reason.contains("0.3"), "{}", model.read_only_reason);
    assert!(!model.nodes.is_empty());
    assert_eq!(Err(model.read_only_reason.clone()), refused);
    assert!(!body.with_extension("adp").exists());
}

#[test]
fn the_document_is_shown_whole_writing_nothing_and_a_fold_is_no_undo_step() {
    let folder = WebShopFolder::new();
    let body_before = fs::read(folder.body()).unwrap();
    let registration_before = fs::read(folder.registration()).unwrap();
    let mut session = RequirementSession::open(folder.body(), Some(folder.registration()));

    // Every item and requirement of the example, the placeholder by its line, with the folds its registration
    // keeps, editable.
    let model = session.model();
    assert_eq!("Web shop", model.title);
    assert_eq!("", model.read_only_reason);
    assert_eq!(causes::OPEN, model.cause);
    assert!(!model.findings.iter().any(|f| f.severity == "error"));
    assert_eq!(6, model.nodes.iter().filter(|n| n.role == roles::LEVEL).count());
    assert_eq!(11, model.nodes.iter().filter(|n| n.role == roles::ENTRY).count());
    let unnumbered: Vec<&str> = model
        .nodes
        .iter()
        .filter(|n| n.id != n.number)
        .map(|n| n.number.as_str())
        .collect();
    assert_eq!(vec!["REQ-NEW"], unnumbered);
    let mut folded: Vec<&str> = model
        .nodes
        .iter()
        .filter(|n| n.collapsed)
        .map(|n| n.id.as_str())
        .collect();
    folded.sort();
    assert_eq!(vec!["ITEM-0003", "REQ-0002"], folded);
    assert_eq!(body_before, fs::read(folder.body()).unwrap());
    assert_eq!(registration_before, fs::read(folder.registration()).unwrap());
    assert!(session.take_changes().is_empty());

    // A fold: written to the registration alone, sent as a new model and then its outcome, and no step of the
    // history.
    assert_eq!(Ok(()), session.gesture(toggle("REQ-0001")));
    let changes = session.take_changes();
    assert_eq!(2, changes.len(), "{changes:?}");
    let SurfaceChange::Model(sent) = &changes[0] else {
        panic!("the model comes first: {changes:?}")
    };
    assert!(collapsed(sent, "REQ-0001"));
    assert_eq!(causes::EDIT, sent.cause);
    let SurfaceChange::Outcome(outcome) = &changes[1] else {
        panic!("the outcome comes second: {changes:?}")
    };
    assert!(outcome.written, "{}", outcome.error);
    assert_eq!("edit-REQ-0001", outcome.edit_id);
    assert!(
        fs::read_to_string(folder.registration())
            .unwrap()
            .contains("REQ-0001: yes")
    );
    assert_eq!(body_before, fs::read(folder.body()).unwrap());
    assert_eq!(Err("There is nothing to undo.".to_owned()), session.undo());
    assert!(
        fs::read_to_string(folder.registration())
            .unwrap()
            .contains("REQ-0001: yes")
    );
}

#[test]
fn a_document_without_a_registration_opens_by_its_envelope() {
    // The body alone, as an agent writes it.
    let folder = tempfile::tempdir().unwrap();
    let body = folder.path().join("agent.md");
    fs::copy(examples_folder().join("web-shop.md"), &body).unwrap();

    let session = RequirementSession::open(&body, None);

    // The page, and still no registration beside it.
    assert_eq!("Web shop", session.model().title);
    assert!(!session.model().nodes.is_empty());
    assert!(!folder.path().join("agent.adp").exists());
}

#[test]
fn an_unknown_operation_and_one_not_built_yet_are_refused_with_the_reason() {
    let folder = WebShopFolder::new();
    let mut session = RequirementSession::open(folder.body(), Some(folder.registration()));
    let gesture = |operation: &str| Gesture {
        operation: operation.into(),
        target_id: "REQ-0001".into(),
        ..Gesture::default()
    };

    assert_eq!(
        Err("'addLink' is not an operation of the Hierarchical Requirement Designer.".to_owned()),
        session.gesture(gesture("addLink"))
    );
    assert_eq!(
        Err("'setTitle' cannot be done here yet.".to_owned()),
        session.gesture(gesture("setTitle"))
    );
    assert!(session.take_changes().is_empty());
}

#[test]
fn a_change_from_outside_is_read_again_and_one_of_nothing_sends_nothing() {
    let folder = WebShopFolder::new();
    let mut session = RequirementSession::open(folder.body(), Some(folder.registration()));

    // The same bytes written again are the same page.
    fs::write(folder.body(), fs::read(folder.body()).unwrap()).unwrap();
    session.file_changed(&folder.body());
    assert!(session.take_changes().is_empty());

    // Another program renames an item.
    let text = fs::read_to_string(folder.body()).unwrap();
    fs::write(folder.body(), text.replacen("ITEM-0001 ", "ITEM-0001 The ", 1)).unwrap();
    session.file_changed(&folder.body());

    let changes = session.take_changes();
    assert_eq!(1, changes.len());
    let SurfaceChange::Model(model) = &changes[0] else {
        panic!("{changes:?}")
    };
    assert_eq!(causes::OUTSIDE, model.cause);
    let item = model.nodes.iter().find(|n| n.id == "ITEM-0001").unwrap();
    assert!(item.title.starts_with("The "), "{}", item.title);
}

#[test]
fn a_file_that_stops_being_readable_keeps_the_last_page_read_only() {
    let folder = WebShopFolder::new();
    let mut session = RequirementSession::open(folder.body(), Some(folder.registration()));
    let nodes = session.model().nodes.len();

    // The envelope taken off: the file is no requirement document while it lacks it.
    let text = fs::read_to_string(folder.body()).unwrap();
    let without = text.split_once('\n').unwrap().1.to_owned();
    fs::write(folder.body(), &without).unwrap();
    session.file_changed(&folder.body());

    let model = session.model();
    assert_ne!("", model.read_only_reason);
    assert_eq!(nodes, model.nodes.len());

    // And read again when it can be.
    fs::write(folder.body(), text).unwrap();
    session.file_changed(&folder.body());
    assert_eq!("", session.model().read_only_reason);
}
