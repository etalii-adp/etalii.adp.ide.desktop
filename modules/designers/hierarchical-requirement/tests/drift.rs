//! An edit made on the page against a file another program has written since is refused (the standalone host's
//! `Drift.Tests.cs` at a7fdf03; spec 029 FR-024, R10, R12): with R10's sentence, nothing written, and the page read
//! again so it shows the file as it is. An edit of an entry the file no longer has is refused the same way, at once.

mod common;

use std::fs;

use adp_designer_hierarchical_requirement::gestures::DRIFTED;
use adp_designer_hierarchical_requirement::session::RequirementSession;
use adp_desl::hierarchy::model::causes;
use adp_desl::hierarchy::session::HierarchySurfaceSession;
use common::{WebShopFolder, models, outcome, set};

#[test]
fn an_edit_made_against_a_file_an_agent_has_since_written_is_refused_nothing_is_written_and_the_new_page_is_sent() {
    // Arrange: the agent's write lands before the watcher has told the session, which is the race the check exists
    // for.
    let folder = WebShopFolder::new();
    let mut session = RequirementSession::open(folder.body(), Some(folder.registration()));
    folder.write_as_an_agent(|text| {
        text.replace(
            "REQ-0009 Order data is kept for seven years",
            "REQ-0009 Order data is kept for ten years",
        )
    });
    let agents = fs::read(folder.body()).unwrap();

    // Act.
    let taken = session.gesture(set("setTitle", "REQ-0004", "title", "An order is confirmed by e-mail."));

    // Assert: taken, then refused with R10's sentence; the file is the agent's; the page shows the agent's change.
    assert_eq!(Ok(()), taken);
    let changes = session.take_changes();
    assert_eq!(Some((false, DRIFTED.to_owned())), outcome(&changes));
    assert_eq!(agents, fs::read(folder.body()).unwrap());
    let shown = models(&changes);
    let shown = shown.last().unwrap();
    assert_eq!(causes::OUTSIDE, shown.cause);
    assert!(
        shown
            .nodes
            .iter()
            .any(|n| n.title.starts_with("Order data is kept for ten years"))
    );
    assert!(
        shown
            .nodes
            .iter()
            .all(|n| n.title != "An order is confirmed by e-mail.")
    );
    assert!(!session.can_undo());
}

#[test]
fn an_edit_of_an_entry_the_file_no_longer_has_is_refused_at_once() {
    // Arrange: an agent removed REQ-0007, and the page has shown it.
    let folder = WebShopFolder::new();
    let mut session = RequirementSession::open(folder.body(), Some(folder.registration()));
    folder.write_as_an_agent(|text| {
        text.split('\n')
            .filter(|line| !line.contains("REQ-0007"))
            .collect::<Vec<_>>()
            .join("\n")
    });
    session.file_changed(&folder.body());
    assert!(session.model().nodes.iter().all(|n| n.id != "REQ-0007"));
    session.take_changes();
    let before = fs::read(folder.body()).unwrap();

    // Act: the text a person kept, committed against the number that is gone.
    let refused = session.gesture(set(
        "setTitle",
        "REQ-0007",
        "title",
        "Customers sign in with a passkey.",
    ));

    // Assert.
    assert_eq!(Err(DRIFTED.to_owned()), refused);
    assert_eq!(before, fs::read(folder.body()).unwrap());
    assert!(session.take_changes().is_empty());
}
