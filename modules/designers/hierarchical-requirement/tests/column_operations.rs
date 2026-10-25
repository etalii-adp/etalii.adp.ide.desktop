//! The widths of the columns of items (the standalone host's `ColumnOperations.Tests.cs` at a7fdf03; DESL 0.2 section
//! 7.7.1; etalii.adp spec 036, T050): `addColumnWidth`, `setColumnWidth` and `resetColumnWidth` write the
//! registration's `widths` block alone, each a step of the history, and the page is sent the widths the registration
//! holds.

mod common;

use std::fs;

use adp_designer_hierarchical_requirement::body::RequirementBody;
use adp_designer_hierarchical_requirement::gestures::DRIFTED;
use adp_designer_hierarchical_requirement::model::Column;
use adp_designer_hierarchical_requirement::operations::columns::{
    self, ADD_COLUMN_WIDTH, RESET_COLUMN_WIDTH, SET_COLUMN_WIDTH,
};
use adp_designer_hierarchical_requirement::operations::lists::ListFiles;
use adp_designer_hierarchical_requirement::session::RequirementSession;
use adp_desl::hierarchy::model::ColumnWidth;
use adp_desl::hierarchy::session::HierarchySurfaceSession;
use common::{WebShopFolder, gesture, outcome, params, run};

#[test]
fn setting_changing_and_resetting_are_steps_of_the_history_in_the_registration_alone_and_the_page_is_sent_the_widths() {
    // Arrange.
    let folder = WebShopFolder::new();
    let mut session = RequirementSession::open(folder.body(), Some(folder.registration()));
    let body = fs::read(folder.body()).unwrap();
    let registration = fs::read(folder.registration()).unwrap();

    // Act.
    run(&mut session, ADD_COLUMN_WIDTH, "document", "level=1|width=360");
    run(&mut session, SET_COLUMN_WIDTH, "column/1", "width=420");
    run(&mut session, ADD_COLUMN_WIDTH, "document", "level=2|width=300");
    let set = fs::read_to_string(folder.registration()).unwrap();
    run(&mut session, RESET_COLUMN_WIDTH, "column/1", "");
    let shown = session.model().widths;

    // Assert.
    assert!(
        set.ends_with(
            "filters:\r\n  Open security: #security @proposed sort:state\r\n  Backend: item:back* sort:number\r\nwidths:\r\n  1: 420\r\n  2: 300\r\n"
        ),
        "{set}"
    );
    let reset = fs::read_to_string(folder.registration()).unwrap();
    assert!(reset.ends_with("widths:\r\n  2: 300\r\n"), "{reset}");
    assert_eq!(
        vec![ColumnWidth {
            id: "column/2".into(),
            level: 2,
            width: 300
        }],
        shown
    );
    assert_eq!(body, fs::read(folder.body()).unwrap());
    for _ in 0..4 {
        session.undo().unwrap();
    }
    assert_eq!(registration, fs::read(folder.registration()).unwrap());
}

#[test]
fn what_cannot_be_written_is_refused_at_once() {
    for (operation, target, written, expected) in [
        (
            ADD_COLUMN_WIDTH,
            "document",
            "level=0|width=300",
            "A column is named by its depth, a whole number from 1.",
        ),
        (
            ADD_COLUMN_WIDTH,
            "document",
            "width=300",
            "A column is named by its depth, a whole number from 1.",
        ),
        (
            ADD_COLUMN_WIDTH,
            "document",
            "level=1|width=119",
            "A column is 120 to 2000 pixels wide.",
        ),
        (
            SET_COLUMN_WIDTH,
            "column/1",
            "width=2001",
            "A column is 120 to 2000 pixels wide.",
        ),
        (
            SET_COLUMN_WIDTH,
            "column/1",
            "width=wide",
            "A column is 120 to 2000 pixels wide.",
        ),
        (
            SET_COLUMN_WIDTH,
            "column/1",
            "width=-300",
            "A column is 120 to 2000 pixels wide.",
        ),
        (
            SET_COLUMN_WIDTH,
            "column/1",
            "width=+300",
            "A column is 120 to 2000 pixels wide.",
        ),
        (SET_COLUMN_WIDTH, "column/3", "width=300", DRIFTED),
        (RESET_COLUMN_WIDTH, "column/3", "", DRIFTED),
    ] {
        // Arrange: the first column has a width; the third has none.
        let folder = WebShopFolder::new();
        let mut session = RequirementSession::open(folder.body(), Some(folder.registration()));
        run(&mut session, ADD_COLUMN_WIDTH, "document", "level=1|width=360");
        let before = fs::read(folder.registration()).unwrap();

        // Act.
        let refusal = session.gesture(gesture(operation, target, written));

        // Assert.
        assert_eq!(Err(expected.to_owned()), refusal, "{operation} {target} {written}");
        assert_eq!(None, outcome(&session.take_changes()), "{operation} {target} {written}");
        assert_eq!(
            before,
            fs::read(folder.registration()).unwrap(),
            "{operation} {target} {written}"
        );
    }
}

#[test]
fn the_boundary_widths_are_written() {
    // Arrange.
    let folder = WebShopFolder::new();
    let mut session = RequirementSession::open(folder.body(), Some(folder.registration()));

    // Act.
    run(&mut session, ADD_COLUMN_WIDTH, "document", "level=1|width=120");
    run(&mut session, ADD_COLUMN_WIDTH, "document", "level=3|width=2000");

    // Assert.
    let written = fs::read_to_string(folder.registration()).unwrap();
    assert!(written.ends_with("widths:\r\n  1: 120\r\n  3: 2000\r\n"), "{written}");
}

#[test]
fn a_first_width_for_a_column_that_has_one_is_a_change_made_against_an_older_file() {
    // Arrange.
    let folder = WebShopFolder::new();
    let body = fs::read(folder.body()).unwrap();
    let mut registration = fs::read(folder.registration()).unwrap();
    registration.extend_from_slice(b"widths:\r\n  1: 360\r\n");
    let files = ListFiles {
        body: &body,
        body_name: "web-shop.md",
        registration: Some(&registration),
        registration_name: "web-shop.adp",
    };

    // Act.
    let edit = columns::apply(files, ADD_COLUMN_WIDTH, "document", &params("level=1|width=400"));

    // Assert.
    assert_eq!(Err(DRIFTED.to_owned()), edit);
}

#[test]
fn a_width_that_is_not_a_number_in_range_is_read_as_a_column_with_the_hosts_width() {
    // Arrange.
    let folder = WebShopFolder::new();
    let body = fs::read(folder.body()).unwrap();
    let mut registration = fs::read(folder.registration()).unwrap();
    registration.extend_from_slice(b"widths:\r\n  1: 400\r\n  2: wide\r\n  3: 50\r\n  0: 300\r\n  4: 2000\r\n");

    // Act.
    let document = RequirementBody::read(body, "web-shop.md", Some((registration, "web-shop.adp"))).document;

    // Assert.
    assert_eq!(
        vec![Column { level: 1, width: 400 }, Column { level: 4, width: 2000 }],
        document.columns
    );
}
