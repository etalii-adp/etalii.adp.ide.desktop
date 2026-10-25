//! The category and state operations make the bytes the conformance fixtures state (the standalone host's
//! `CategoryStateOperations.Tests.cs` at a7fdf03; spec 029 T102, FR-026, FR-030, FR-031):
//! `hierarchical-requirement-category-state` the Markdown file's, its `list` the registration's, each step run through
//! the module's operation from the surface's ids. Of several `category:` words the first counts: a set or a clear
//! changes that one and keeps the others as written. Through the session, a rename or a delete writes both files as
//! one step, which one undo takes back, and a pick writes the Markdown file alone.

mod common;

use std::fs;

use adp_designer_hierarchical_requirement::gestures::DRIFTED;
use adp_designer_hierarchical_requirement::operations::classification;
use adp_designer_hierarchical_requirement::operations::lists::{ListEdit, ListFiles};
use adp_designer_hierarchical_requirement::session::RequirementSession;
use adp_desl::hierarchy::session::HierarchySurfaceSession;
use common::{WebShopFolder, expect, gesture, params, run};

const CATEGORY_STATE: &str = "hierarchical-requirement-category-state";
const LIST: &str = "hierarchical-requirement-category-state/list";

/// Every step of the two fixtures that an operation makes: the step of the body's fixture it starts from (none for
/// the input), whether it starts from the list's registration, the gesture, its target and params, then the step of
/// the body's fixture and of the list's whose bytes it writes, none for a file it does not write.
/// A step: the body it starts from, whether the list's registration is there, the gesture, its target, its params,
/// and the steps it writes.
type Step = (
    Option<usize>,
    bool,
    &'static str,
    &'static str,
    &'static str,
    Option<usize>,
    Option<usize>,
);

const STEPS: [Step; 9] = [
    (
        None,
        false,
        "setCategory",
        "REQ-0002",
        "category=functional",
        Some(0),
        None,
    ),
    (None, false, "setState", "REQ-0001", "state=proposed", Some(2), None),
    (
        None,
        false,
        "setCategory",
        "REQ-0004",
        "category=Functional",
        Some(4),
        None,
    ),
    (None, false, "setState", "REQ-0003", "state=rejected", Some(6), None),
    (None, false, "clearCategory", "REQ-0003", "", Some(8), None),
    (Some(8), false, "clearState", "REQ-0003", "", Some(9), None),
    (None, false, "clearCategory", "REQ-0004", "", Some(12), None),
    (
        None,
        true,
        "renameCategory",
        "category/technical",
        "name=engineering",
        Some(14),
        Some(2),
    ),
    (
        None,
        true,
        "addCategory",
        "document",
        "name=security|colour=red",
        None,
        Some(0),
    ),
];

fn body() -> Vec<u8> {
    fs::read(common::fixture(CATEGORY_STATE).join("requirements.md")).unwrap()
}

fn listed() -> Vec<u8> {
    fs::read(common::fixture(LIST).join("requirements.adp")).unwrap()
}

fn apply(
    body: &[u8],
    registration: Option<&[u8]>,
    operation: &str,
    target: &str,
    written: &str,
) -> Result<ListEdit, String> {
    let files = ListFiles {
        body,
        body_name: "requirements.md",
        registration,
        registration_name: "requirements.adp",
    };
    classification::apply(files, operation, target, &params(written))
}

fn text(bytes: &Option<Vec<u8>>) -> Option<String> {
    bytes.as_ref().map(|b| String::from_utf8(b.clone()).unwrap())
}

#[test]
fn a_gesture_writes_the_fixtures_bytes() {
    for (from, with_list, operation, target, written, body_step, registration_step) in STEPS {
        // Arrange.
        let body = from.map_or_else(body, |step| expect(CATEGORY_STATE, step).into_bytes());
        let registration = with_list.then(listed);

        // Act.
        let edit = apply(&body, registration.as_deref(), operation, target, written)
            .unwrap_or_else(|refusal| panic!("{operation} {target} is refused: {refusal}"));

        // Assert.
        assert_eq!(
            body_step.map(|s| expect(CATEGORY_STATE, s)),
            text(&edit.body),
            "{operation} {target}"
        );
        assert_eq!(
            registration_step.map(|s| expect(LIST, s)),
            text(&edit.registration_after),
            "{operation} {target}"
        );
    }
}

#[test]
fn a_first_change_of_a_list_without_a_registration_starts_from_the_one_it_creates() {
    // Act.
    let edit = apply(&body(), None, "addCategory", "document", "name=security|colour=red").unwrap();

    // Assert: the step starts from the registration createOnFirstPlacement writes, and writes the whole list, legal
    // included.
    assert_eq!(Some(listed()), edit.registration_before);
    assert_eq!(Some(expect(LIST, 0)), text(&edit.registration_after));
    assert_eq!(None, edit.body);
}

#[test]
fn a_name_of_two_words_is_written_with_a_dash_on_the_carriers_and_in_the_list() {
    // Act.
    let edit = apply(&body(), None, "renameCategory", "category/technical", "name= built in ").unwrap();

    // Assert: p.name.trim().replace(' ', '-'), as the operation writes it.
    assert!(text(&edit.body).unwrap().contains("category:built-in"));
    assert!(text(&edit.registration_after).unwrap().contains("built-in:"));
}

/// The keys of `block` in the registration `edit` writes, in their order.
fn keys(edit: &ListEdit, block: &str) -> Vec<String> {
    let written = text(&edit.registration_after).unwrap();
    let (_, rest) = written.split_once(&format!("{block}:\r\n")).unwrap();
    rest.split("\r\n")
        .take_while(|line| line.starts_with("  "))
        .map(|line| line.trim().split(':').next().unwrap().to_owned())
        .collect()
}

#[test]
fn a_recolour_or_a_move_writes_only_the_list() {
    // Arrange.
    let (body, registration) = (body(), listed());

    // Act: each from the registration without the list, which its first change writes whole.
    let recoloured = apply(
        &body,
        Some(&registration),
        "recolourState",
        "state/proposed",
        "colour=orange",
    )
    .unwrap();
    let first = apply(
        &body,
        Some(&registration),
        "moveCategory",
        "category/technical",
        "after=",
    )
    .unwrap();
    let after = apply(
        &body,
        Some(&registration),
        "moveCategory",
        "category/functional",
        "after=category/technical",
    )
    .unwrap();

    // Assert.
    assert!(
        text(&recoloured.registration_after)
            .unwrap()
            .contains("  proposed: orange\r\n")
    );
    assert_eq!(
        ["technical", "functional", "non-functional", "legal"],
        keys(&first, "categories").as_slice()
    );
    assert_eq!(
        ["non-functional", "technical", "functional", "legal"],
        keys(&after, "categories").as_slice()
    );
    assert!([recoloured, first, after].iter().all(|edit| edit.body.is_none()));
}

#[test]
fn what_cannot_be_written_is_refused_and_a_gone_target_is_a_change_made_against_an_older_file() {
    let body = body();
    let refusal = |operation: &str, target: &str, written: &str| apply(&body, None, operation, target, written).err();

    assert_eq!(
        Some("A category needs a name.".into()),
        refusal("addCategory", "document", "name= ")
    );
    assert_eq!(
        Some("'mauve' is not a colour.".into()),
        refusal("recolourState", "state/proposed", "colour=mauve")
    );
    assert_eq!(
        Some("There is already a category named LEGAL.".into()),
        refusal("renameCategory", "category/technical", "name=LEGAL")
    );
    assert_eq!(
        Some("There is already a state named done.".into()),
        refusal("addState", "document", "name=done")
    );
    assert_eq!(
        Some("'urgent' is not a state of this document.".into()),
        refusal("setState", "REQ-0001", "state=urgent")
    );
    assert_eq!(
        Some("'a\tb' cannot be a category: a category's name is one word.".into()),
        refusal("addCategory", "document", "name=a\tb")
    );
    assert_eq!(Some("Pick a state.".into()), refusal("setState", "REQ-0001", "state= "));
    assert_eq!(
        Some(DRIFTED.into()),
        refusal("setCategory", "ITEM-0001", "category=legal")
    );
    assert_eq!(Some(DRIFTED.into()), refusal("deleteState", "state/gone", ""));
    assert_eq!(
        Some(DRIFTED.into()),
        refusal("moveCategory", "category/legal", "after=category/gone")
    );
}

#[test]
fn a_name_only_a_requirement_carries_is_a_member_to_pick() {
    // Arrange: REQ-0005 carries a category the registration's block does not hold.
    let body = String::from_utf8(body())
        .unwrap()
        .replace("category:Technical", "category:Hardware");
    let registration = expect(LIST, 0);
    assert!(registration.contains("categories:") && !registration.contains("Hardware"));

    // Act.
    let edit = apply(
        body.as_bytes(),
        Some(registration.as_bytes()),
        "setCategory",
        "REQ-0002",
        "category=hardware",
    )
    .unwrap();

    // Assert: picked with the spelling the requirement wrote.
    assert!(
        text(&edit.body)
            .unwrap()
            .contains("REQ-0002 An order is confirmed by e-mail. #mail category:Hardware")
    );
}

#[test]
fn a_rename_or_a_delete_writes_both_files_as_one_step_that_one_undo_takes_back() {
    for (operation, written, body_has, registration_has) in [
        (
            "renameCategory",
            "name=behaviour",
            "category:behaviour state:accepted",
            "  behaviour: blue",
        ),
        ("deleteCategory", "", "state:accepted", "  non-functional: purple"),
    ] {
        // Arrange.
        let folder = WebShopFolder::new();
        let mut session = RequirementSession::open(folder.body(), Some(folder.registration()));
        let body = fs::read(folder.body()).unwrap();
        let registration = fs::read(folder.registration()).unwrap();

        // Act.
        run(&mut session, operation, "category/functional", written);

        // Assert: both files changed, functional is gone from both - the file's own example of the notation aside - and
        // one undo gives both back.
        let text = fs::read_to_string(folder.body()).unwrap();
        let kept = fs::read_to_string(folder.registration()).unwrap();
        assert!(
            text.contains(&format!(
                "REQ-0006 A session ends after 30 minutes without activity. #security {body_has}"
            )),
            "{operation}: {text}"
        );
        assert!(!text.contains("REQ-0003 A customer pays by card, iDEAL or PayPal. category:functional"));
        assert!(!kept.contains("  functional: blue"), "{operation}");
        assert!(kept.contains(registration_has), "{operation}: {kept}");
        session.undo().unwrap();
        assert_eq!(body, fs::read(folder.body()).unwrap(), "{operation}");
        assert_eq!(registration, fs::read(folder.registration()).unwrap(), "{operation}");
        assert!(!session.can_undo(), "{operation}");
    }
}

#[test]
fn a_pick_writes_only_the_markdown_file_with_the_lists_spelling() {
    // Arrange.
    let folder = WebShopFolder::new();
    let mut session = RequirementSession::open(folder.body(), Some(folder.registration()));
    let registration = fs::read(folder.registration()).unwrap();

    // Act.
    run(&mut session, "setState", "REQ-0006", "state=DONE");

    // Assert.
    assert!(fs::read_to_string(folder.body()).unwrap().contains(
        "REQ-0006 A session ends after 30 minutes without activity. #security category:functional state:done"
    ));
    assert_eq!(registration, fs::read(folder.registration()).unwrap());
}

#[test]
fn a_member_the_page_does_not_show_or_a_pick_of_nothing_is_refused_at_once() {
    let folder = WebShopFolder::new();
    let mut session = RequirementSession::open(folder.body(), Some(folder.registration()));

    assert_eq!(
        Err(DRIFTED.to_owned()),
        session.gesture(gesture("deleteState", "state/gone", ""))
    );
    assert_eq!(
        Err("Pick a state.".to_owned()),
        session.gesture(gesture("setState", "REQ-0001", "state= "))
    );
}
