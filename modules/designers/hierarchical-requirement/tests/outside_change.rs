//! A change another program makes to an open document is shown without reopening it (the standalone host's
//! `OutsideChange.Tests.cs` at a7fdf03; spec 029 FR-022, US2 scenario 2, R13): what is collapsed is kept by number, so
//! an item an agent moved is still collapsed; the page's own write and a write of identical bytes send nothing; a file
//! read half written is read again before the page is made read-only; and a change that makes the file unreadable keeps
//! the last page, read-only.

mod common;

use std::cell::Cell;
use std::fs;
use std::rc::Rc;

use adp_designer_hierarchical_requirement::session::{READ_ATTEMPTS, RequirementSession};
use adp_desl::hierarchy::model::causes;
use adp_desl::hierarchy::session::HierarchySurfaceSession;
use common::{WebShopFolder, models, newline_of, outcome, set};

#[test]
fn an_outside_change_is_shown_without_reopening_and_a_moved_item_keeps_its_fold() {
    // Arrange: the web shop open, with ITEM-0003 collapsed by its registration, nested in ITEM-0002.
    let folder = WebShopFolder::new();
    let mut session = RequirementSession::open(folder.body(), Some(folder.registration()));
    let opened = session.model();
    let guest = opened.nodes.iter().find(|n| n.id == "ITEM-0003").unwrap();
    assert_eq!(("ITEM-0002", true), (guest.parent_id.as_str(), guest.collapsed));
    let registration = fs::read(folder.registration()).unwrap();

    // Act: an agent moves ITEM-0003 with its requirements to the top level, at the end, and renames REQ-0001.
    folder.write_as_an_agent(|text| {
        let n = newline_of(text);
        let guest = format!(
            "    - ITEM-0003 Guest checkout{n}      + REQ-0005 A customer can order without an account. state:rejected{n}      + REQ-NEW A guest is offered an account after the order. #customer category:functional{n}"
        );
        let last = format!("  - REQ-0010 Personal data is removed on request within 30 days. #legal #privacy category:non-functional{n}");
        assert!(text.contains(&guest));
        text.replace(&guest, "")
            .replace(
                &last,
                &format!(
                    "{last}- ITEM-0003 Guest checkout{n}  - REQ-0005 A customer can order without an account. state:rejected{n}  - REQ-NEW A guest is offered an account after the order. #customer category:functional{n}"
                ),
            )
            .replace(
                "REQ-0001 A customer finds a product by its name or its description.",
                "REQ-0001 A customer finds a product by its name.",
            )
    });
    session.file_changed(&folder.body());

    // Assert: the page shows it, as a change from outside, the moved item still collapsed and at the top level.
    let shown = models(&session.take_changes());
    assert_eq!(1, shown.len());
    let shown = &shown[0];
    assert_eq!(causes::OUTSIDE, shown.cause);
    assert!(
        shown
            .nodes
            .iter()
            .any(|n| n.title == "A customer finds a product by its name.")
    );
    let moved = shown.nodes.iter().find(|n| n.id == "ITEM-0003").unwrap();
    assert_eq!(("", true), (moved.parent_id.as_str(), moved.collapsed));
    assert!(shown.nodes.iter().find(|n| n.id == "REQ-0002").unwrap().collapsed);
    assert_eq!(registration, fs::read(folder.registration()).unwrap());
}

#[test]
fn the_pages_own_write_and_a_write_of_the_same_bytes_send_nothing_from_outside() {
    // Arrange.
    let folder = WebShopFolder::new();
    let mut session = RequirementSession::open(folder.body(), Some(folder.registration()));

    // Act: a title changed on the page, then both files saved again by another program with the bytes they hold.
    session
        .gesture(set("setTitle", "REQ-0004", "title", "An order is confirmed by e-mail."))
        .unwrap();
    let edited = session.take_changes();
    assert_eq!(Some((true, String::new())), outcome(&edited));
    assert_eq!(
        vec![causes::EDIT],
        models(&edited).iter().map(|m| m.cause.as_str()).collect::<Vec<_>>()
    );
    for path in [folder.body(), folder.registration()] {
        fs::write(&path, fs::read(&path).unwrap()).unwrap();
        session.file_changed(&path);
    }

    // Assert: neither the watcher hearing the page's write nor the identical bytes sent another.
    assert!(session.take_changes().is_empty());
}

#[test]
fn a_file_read_half_written_is_read_again_before_the_page_is_made_read_only() {
    // Arrange: between two reads, the other program finishes its save.
    let folder = WebShopFolder::new();
    let whole = fs::read(folder.body()).unwrap();
    let waits = Rc::new(Cell::new(0));
    let (body, counted) = (folder.body(), waits.clone());
    let mut finished = [&whole[..whole.len() - 1], b" ", &whole[whole.len() - 1..]].concat();
    let mut session =
        RequirementSession::open(folder.body(), Some(folder.registration())).waiting(Box::new(move || {
            counted.set(counted.get() + 1);
            fs::write(&body, std::mem::take(&mut finished)).unwrap();
        }));

    // Act: another program has emptied the file and not yet written it again, as a save that truncates first does.
    fs::write(folder.body(), b"").unwrap();
    session.file_changed(&folder.body());

    // Assert: the page as the file now is, read once more, and never read-only on the way.
    let shown = models(&session.take_changes());
    assert_eq!(1, waits.get());
    assert_eq!(1, shown.len());
    assert_eq!("", shown[0].read_only_reason);
    assert!(!shown[0].nodes.is_empty());
}

#[test]
fn a_change_that_makes_the_file_unreadable_keeps_the_last_page_read_only_until_it_reads_again() {
    // Arrange.
    let folder = WebShopFolder::new();
    let waits = Rc::new(Cell::new(0));
    let counted = waits.clone();
    let mut session = RequirementSession::open(folder.body(), Some(folder.registration()))
        .waiting(Box::new(move || counted.set(counted.get() + 1)));
    let opened = session.model();
    let readable = fs::read(folder.body()).unwrap();

    // Act: another program leaves bytes that are not UTF-8 at the end of the file.
    fs::write(folder.body(), [&readable[..], &[0xFF, 0xFE]].concat()).unwrap();
    session.file_changed(&folder.body());

    // Assert: read the attempts' number of times, then the same boxes, read-only, saying why; and no change is taken
    // while it cannot be read.
    assert_eq!(READ_ATTEMPTS - 1, waits.get());
    let kept = session.model();
    assert_eq!(opened.nodes, kept.nodes);
    assert!(
        kept.read_only_reason.contains("not valid UTF-8"),
        "{}",
        kept.read_only_reason
    );
    assert_eq!(
        Err(kept.read_only_reason.clone()),
        session.gesture(set("setTitle", "REQ-0001", "title", "Lost."))
    );

    // Act and assert: readable again, it is the page again.
    session.take_changes();
    fs::write(folder.body(), &readable).unwrap();
    session.file_changed(&folder.body());
    let again = models(&session.take_changes());
    assert_eq!(1, again.len());
    assert_eq!("", again[0].read_only_reason);
    let named = |m: &adp_desl::hierarchy::model::SurfaceModel| -> Vec<(String, String)> {
        m.nodes.iter().map(|n| (n.id.clone(), n.title.clone())).collect()
    };
    assert_eq!(named(&opened), named(&again[0]));
}
