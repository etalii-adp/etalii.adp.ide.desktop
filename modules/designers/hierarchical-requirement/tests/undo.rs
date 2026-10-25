//! Undo of an edit made on the page (the standalone host's `Undo.Tests.cs` at a7fdf03; spec 029 FR-025, US2 scenario
//! 3, R10, R12): it gives the file's bytes back exactly and redo makes the edit again, each shown as an edit; the
//! watcher hearing that write sends nothing from outside; and once another program has written the file, undo gives
//! the history's own message once, writes nothing, and the edits before that are gone from the history.

mod common;

use std::fs;

use adp_designer_hierarchical_requirement::session::RequirementSession;
use adp_desl::hierarchy::model::causes;
use adp_desl::hierarchy::session::HierarchySurfaceSession;
use common::{WebShopFolder, models, outcome, set};

const OUTDATED: &str =
    "web-shop.md was changed by another program, so the edits made to it before that can no longer be undone.";

#[test]
fn undo_gives_the_bytes_back_and_redo_makes_the_edit_again_each_shown_as_an_edit() {
    // Arrange: a description changed on the page.
    let folder = WebShopFolder::new();
    let mut session = RequirementSession::open(folder.body(), Some(folder.registration()));
    let before = fs::read(folder.body()).unwrap();
    session
        .gesture(set(
            "setDescription",
            "REQ-0007",
            "description",
            "A forgotten password is reset by e-mail.",
        ))
        .unwrap();
    assert_eq!(Some((true, String::new())), outcome(&session.take_changes()));
    let after = fs::read(folder.body()).unwrap();
    assert_ne!(before, after);

    // Act and assert: undone, the bytes are those from before, and the page shows it as an edit.
    session.undo().unwrap();
    assert_eq!(before, fs::read(folder.body()).unwrap());
    let undone = models(&session.take_changes());
    let undone = undone.last().unwrap();
    assert_eq!(causes::EDIT, undone.cause);
    assert_eq!(
        "",
        undone.nodes.iter().find(|n| n.id == "REQ-0007").unwrap().description
    );

    // Act and assert: redone, the edit's bytes again.
    session.redo().unwrap();
    assert_eq!(after, fs::read(folder.body()).unwrap());
    assert_eq!(causes::EDIT, models(&session.take_changes()).last().unwrap().cause);
}

#[test]
fn the_watcher_hearing_an_undos_write_sends_nothing_from_outside() {
    // Arrange: a title changed on the page and undone, the page shown as the undo left the file.
    let folder = WebShopFolder::new();
    let mut session = RequirementSession::open(folder.body(), Some(folder.registration()));
    session
        .gesture(set(
            "setTitle",
            "REQ-0009",
            "title",
            "Order data is kept for ten years.",
        ))
        .unwrap();
    session.undo().unwrap();
    session.take_changes();

    // Act: the watcher hears the undo's write after the session has read it.
    session.file_changed(&folder.body());

    // Assert: nothing is sent, so a field open on the page is not offered the choice a change from outside gets.
    assert!(session.take_changes().is_empty());
}

#[test]
fn undo_past_an_outside_change_gives_the_outdated_message_once_and_writes_nothing() {
    // Arrange: two titles changed on the page, then the file written by an agent.
    let folder = WebShopFolder::new();
    let mut session = RequirementSession::open(folder.body(), Some(folder.registration()));
    for (target, title) in [
        ("REQ-0001", "A customer finds a product by its name."),
        ("REQ-0004", "An order is confirmed by e-mail."),
    ] {
        session.gesture(set("setTitle", target, "title", title)).unwrap();
        assert_eq!(Some((true, String::new())), outcome(&session.take_changes()));
    }
    folder.write_as_an_agent(|text| {
        text.replace("Order data is kept for seven years", "Order data is kept for ten years")
    });
    let agents = fs::read(folder.body()).unwrap();

    // Act.
    let undo = session.undo();

    // Assert: the history's own message, nothing written, and nothing left to undo, so it is said once.
    assert_eq!(Err(OUTDATED.to_owned()), undo);
    assert_eq!(agents, fs::read(folder.body()).unwrap());
    assert!(!session.can_undo());
    assert_eq!(Err("There is nothing to undo.".to_owned()), session.undo());
    assert_eq!(agents, fs::read(folder.body()).unwrap());
}
