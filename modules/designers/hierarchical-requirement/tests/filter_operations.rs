//! `storeFilter`, `renameFilter` and `removeFilter` make the registration's bytes the conformance fixture
//! `hierarchical-requirement-filters` states and write nothing to the Markdown file (the standalone host's
//! `FilterOperations.Tests.cs` at a7fdf03; spec 029 T103, FR-026, FR-048); what FBL refuses is refused, and nothing is
//! written. On the page, each is a step of the history that writes the registration alone.

mod common;

use std::fs;

use adp_designer_hierarchical_requirement::gestures::DRIFTED;
use adp_designer_hierarchical_requirement::operations::filters::{self, REMOVE_FILTER, RENAME_FILTER, STORE_FILTER};
use adp_designer_hierarchical_requirement::operations::lists::{ListEdit, ListFiles};
use adp_designer_hierarchical_requirement::session::RequirementSession;
use adp_desl::hierarchy::session::HierarchySurfaceSession;
use common::{WebShopFolder, expect, gesture, params, run};

const FILTERS: &str = "hierarchical-requirement-filters";

/// Every step of the fixture that writes: the step whose registration it starts from (none for the input), the
/// gesture, its target and params, and the step whose registration it writes, none for the input.
/// A step: the registration it starts from, the gesture, its target, its params and the registration it writes.
type Step = (Option<usize>, &'static str, &'static str, &'static str, Option<usize>);

const STEPS: [Step; 6] = [
    (
        None,
        STORE_FILTER,
        "document",
        "name= Open security |query= #security state:proposed sort:state ",
        Some(0),
    ),
    (
        Some(0),
        STORE_FILTER,
        "document",
        "name=Keyboard and forms|query=#ux item:inter* sort:title",
        Some(1),
    ),
    (
        Some(1),
        RENAME_FILTER,
        "filter/Keyboard and forms",
        "name=Interface",
        Some(2),
    ),
    (
        Some(1),
        STORE_FILTER,
        "document",
        "name=Security: open|query=#security state:proposed",
        Some(6),
    ),
    (Some(1), REMOVE_FILTER, "filter/Open security", "", Some(8)),
    (Some(0), REMOVE_FILTER, "filter/Open security", "", None),
];

/// The registration as the fixture's step leaves it, or its input for none.
fn registration(step: Option<usize>) -> Vec<u8> {
    match step {
        None => fs::read(common::fixture(FILTERS).join("requirements.adp")).unwrap(),
        Some(step) => expect(FILTERS, step).into_bytes(),
    }
}

fn apply(registration: &[u8], operation: &str, target: &str, written: &str) -> Result<ListEdit, String> {
    let body = fs::read(common::fixture(FILTERS).join("requirements.md")).unwrap();
    let files = ListFiles {
        body: &body,
        body_name: "requirements.md",
        registration: Some(registration),
        registration_name: "requirements.adp",
    };
    filters::apply(files, operation, target, &params(written))
}

#[test]
fn a_gesture_writes_the_fixtures_bytes() {
    for (from, operation, target, written, step) in STEPS {
        // Arrange.
        let registration = registration(from);

        // Act.
        let edit = apply(&registration, operation, target, written)
            .unwrap_or_else(|refusal| panic!("{operation} {target} is refused: {refusal}"));

        // Assert.
        assert_eq!(None, edit.body, "{operation} {target}");
        assert_eq!(
            Some(&registration),
            edit.registration_before.as_ref(),
            "{operation} {target}"
        );
        assert_eq!(
            String::from_utf8(self::registration(step)).unwrap(),
            String::from_utf8(edit.registration_after.unwrap()).unwrap(),
            "{operation} {target}"
        );
    }
}

#[test]
fn what_fbl_refuses_writes_nothing() {
    for (operation, target, written) in [
        (RENAME_FILTER, "filter/Keyboard and forms", "name=Open security"),
        (STORE_FILTER, "document", "name=Accepted|query=state: accepted"),
    ] {
        assert!(
            apply(&registration(Some(1)), operation, target, written).is_err(),
            "{operation}"
        );
    }
}

#[test]
fn what_cannot_be_written_is_refused_and_a_gone_filter_is_a_change_made_against_an_older_file() {
    let registration = registration(Some(0));
    let refusal = |operation: &str, target: &str, written: &str| apply(&registration, operation, target, written).err();

    assert_eq!(
        Some("A filter needs a name.".into()),
        refusal(STORE_FILTER, "document", "name= |query=#ux")
    );
    assert_eq!(
        Some("There is no filter to store.".into()),
        refusal(STORE_FILTER, "document", "name=Empty|query= ")
    );
    assert_eq!(Some(DRIFTED.into()), refusal(REMOVE_FILTER, "filter/Gone", ""));
    assert_eq!(
        Some(DRIFTED.into()),
        refusal(RENAME_FILTER, "filter/open security", "name=Open")
    );
}

#[test]
fn storing_renaming_and_removing_are_steps_of_the_history_in_the_registration_alone() {
    // Arrange.
    let folder = WebShopFolder::new();
    let mut session = RequirementSession::open(folder.body(), Some(folder.registration()));
    let body = fs::read(folder.body()).unwrap();
    let registration = fs::read(folder.registration()).unwrap();

    // Act.
    run(
        &mut session,
        STORE_FILTER,
        "document",
        "name=Done|query=state:done sort:title",
    );
    let stored = fs::read_to_string(folder.registration()).unwrap();
    run(&mut session, RENAME_FILTER, "filter/Backend", "name=Server");
    run(&mut session, REMOVE_FILTER, "filter/Open security", "");
    let changed = fs::read_to_string(folder.registration()).unwrap();

    // Assert.
    assert!(
        stored.ends_with("  Backend: item:back* sort:number\r\n  Done: state:done sort:title\r\n"),
        "{stored}"
    );
    assert!(
        changed.ends_with("filters:\r\n  Server: item:back* sort:number\r\n  Done: state:done sort:title\r\n"),
        "{changed}"
    );
    assert_eq!(body, fs::read(folder.body()).unwrap());
    for _ in 0..3 {
        session.undo().unwrap();
    }
    assert_eq!(registration, fs::read(folder.registration()).unwrap());
}

#[test]
fn a_filter_the_page_does_not_show_is_refused_at_once() {
    let folder = WebShopFolder::new();
    let mut session = RequirementSession::open(folder.body(), Some(folder.registration()));

    assert_eq!(
        Err(DRIFTED.to_owned()),
        session.gesture(gesture(REMOVE_FILTER, "filter/Gone", ""))
    );
}
