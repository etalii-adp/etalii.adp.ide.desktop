//! `addTag`, `removeTag`, `renameTag`, `recolourTag`, `moveTag` and `deleteTag` make the bytes the conformance
//! fixtures state (the standalone host's `TagOperations.Tests.cs` at a7fdf03; spec 029 T095, FR-026, R17):
//! `hierarchical-requirement-tags` the Markdown file's, `hierarchical-requirement-tag-list` the registration's, each
//! step run through the module's operation from the surface's ids. An operation that writes both is checked against
//! both. Through the session, a rename or a delete writes both files as one step, which one undo takes back.

mod common;

use std::fs;

use adp_designer_hierarchical_requirement::gestures::DRIFTED;
use adp_designer_hierarchical_requirement::operations::lists::ListFiles;
use adp_designer_hierarchical_requirement::operations::tags::{
    self, ADD_TAG, DELETE_TAG, MOVE_TAG, RECOLOUR_TAG, REMOVE_TAG, RENAME_TAG,
};
use adp_designer_hierarchical_requirement::session::RequirementSession;
use adp_desl::hierarchy::session::HierarchySurfaceSession;
use common::{WebShopFolder, expect, gesture, params, run};

const TAGS: &str = "hierarchical-requirement-tags";
const TAG_LIST: &str = "hierarchical-requirement-tag-list";

/// The registration a step starts from: none, the fixture's input, or the input after its first recolour, which the
/// steps from 3 to 12 start from.
#[derive(Debug, Clone, Copy)]
enum Start {
    None,
    Input,
    Recoloured,
}

/// Every step of the two fixtures that a tag operation makes: the registration it starts from, the gesture, its target
/// and params, then the step of `-tag-list` whose registration it writes and the step of `-tags` whose body it writes,
/// none for a file it does not write.
/// A step: the registration it starts from, the gesture, its target, its params, and the steps it writes.
type Step = (
    Start,
    &'static str,
    &'static str,
    &'static str,
    Option<usize>,
    Option<usize>,
);

const STEPS: [Step; 14] = [
    (Start::None, ADD_TAG, "ITEM-0002", "name=Security", None, Some(0)),
    (Start::None, ADD_TAG, "REQ-0002", "name=security", None, Some(2)),
    (Start::None, ADD_TAG, "REQ-0001", "name=ux", None, Some(4)),
    (Start::None, ADD_TAG, "REQ-0005", "name=security", None, Some(6)),
    (
        Start::Input,
        ADD_TAG,
        "REQ-0004",
        "name=user experience",
        Some(14),
        Some(8),
    ),
    (Start::Input, REMOVE_TAG, "REQ-0004", "name=ux", None, Some(10)),
    (
        Start::Input,
        RENAME_TAG,
        "tag/security",
        "name=infosec",
        Some(16),
        Some(12),
    ),
    (Start::Input, DELETE_TAG, "tag/ux", "", Some(18), Some(14)),
    (Start::Input, RECOLOUR_TAG, "tag/ux", "colour=blue", Some(0), None),
    (
        Start::Recoloured,
        RECOLOUR_TAG,
        "tag/privacy",
        "colour=red",
        Some(3),
        None,
    ),
    (Start::Recoloured, MOVE_TAG, "tag/privacy", "", Some(5), None),
    (
        Start::Recoloured,
        ADD_TAG,
        "REQ-0004",
        "name=user experience",
        Some(7),
        Some(8),
    ),
    (
        Start::Recoloured,
        RENAME_TAG,
        "tag/security",
        "name=infosec",
        Some(9),
        Some(12),
    ),
    (Start::Recoloured, DELETE_TAG, "tag/ux", "", Some(11), Some(14)),
];

fn body() -> Vec<u8> {
    fs::read(common::fixture(TAGS).join("requirements.md")).unwrap()
}

fn files<'a>(body: &'a [u8], registration: Option<&'a [u8]>) -> ListFiles<'a> {
    ListFiles {
        body,
        body_name: "requirements.md",
        registration,
        registration_name: "requirements.adp",
    }
}

fn text(bytes: Option<Vec<u8>>) -> Option<String> {
    bytes.map(|b| String::from_utf8(b).unwrap())
}

#[test]
fn a_gesture_writes_the_fixtures_bytes() {
    let body = body();
    for (start, operation, target, written, registration_step, body_step) in STEPS {
        // Arrange.
        let registration = match start {
            Start::None => None,
            Start::Input => Some(fs::read(common::fixture(TAG_LIST).join("requirements.adp")).unwrap()),
            Start::Recoloured => Some(expect(TAG_LIST, 2).into_bytes()),
        };

        // Act.
        let edit = tags::apply(
            files(&body, registration.as_deref()),
            operation,
            target,
            &params(written),
        )
        .unwrap_or_else(|refusal| panic!("{operation} {target} is refused: {refusal}"));

        // Assert.
        let case = format!("{operation} {target} from {start:?}");
        assert_eq!(body_step.map(|s| expect(TAGS, s)), text(edit.body), "{case}");
        assert_eq!(
            registration_step.map(|s| expect(TAG_LIST, s)),
            text(edit.registration_after.clone()),
            "{case}"
        );
        if edit.registration_after.is_some() {
            assert_eq!(registration, edit.registration_before, "{case}");
        }
    }
}

#[test]
fn a_first_change_of_the_list_without_a_registration_starts_from_the_one_it_creates() {
    // Arrange.
    let body = body();
    let created = fs::read(common::fixture(TAG_LIST).join("requirements.adp")).unwrap();

    // Act.
    let edit = tags::apply(files(&body, None), ADD_TAG, "REQ-0004", &params("name=user experience")).unwrap();

    // Assert: the step starts from the registration createOnFirstPlacement writes, and writes the whole list.
    assert_eq!(Some(created), edit.registration_before);
    assert_eq!(Some(expect(TAG_LIST, 14)), text(edit.registration_after));
}

#[test]
fn what_cannot_be_written_is_refused_and_a_gone_target_is_a_change_made_against_an_older_file() {
    let body = body();
    let refusal = |operation: &str, target: &str, written: &str| {
        tags::apply(files(&body, None), operation, target, &params(written)).err()
    };

    assert_eq!(
        Some("A tag needs a name.".into()),
        refusal(ADD_TAG, "REQ-0001", "name=  ")
    );
    assert_eq!(
        Some("'a#b' cannot be a tag: a tag's name is one word without #.".into()),
        refusal(ADD_TAG, "REQ-0001", "name=a#b")
    );
    assert_eq!(
        Some("'mauve' is not a colour.".into()),
        refusal(RECOLOUR_TAG, "tag/ux", "colour=mauve")
    );
    assert_eq!(
        Some("There is already a tag named UX.".into()),
        refusal(RENAME_TAG, "tag/security", "name=UX")
    );
    assert_eq!(Some(DRIFTED.into()), refusal(ADD_TAG, "REQ-0999", "name=ux"));
    assert_eq!(Some(DRIFTED.into()), refusal(DELETE_TAG, "tag/gone", ""));
    assert_eq!(Some(DRIFTED.into()), refusal(MOVE_TAG, "tag/ux", "after=tag/gone"));
}

#[test]
fn a_tag_the_block_does_not_hold_is_renamed_or_deleted_in_the_body_alone() {
    // Arrange: an agent's tag, ux, that the entries carry and the list's block has not taken in.
    let body = body();
    let registration = expect(TAG_LIST, 2)
        .replace("  ux: blue\n", "")
        .replace("  ux: blue\r\n", "");
    assert!(!registration.contains("ux:"));

    for (operation, written) in [(DELETE_TAG, ""), (RENAME_TAG, "name=usability")] {
        // Act.
        let edit = tags::apply(
            files(&body, Some(registration.as_bytes())),
            operation,
            "tag/ux",
            &params(written),
        )
        .unwrap();

        // Assert: the carriers change, and the block has nothing to change.
        assert!(edit.body.is_some(), "{operation}");
        assert_eq!(None, edit.registration_after, "{operation}");
    }
}

#[test]
fn a_rename_or_a_delete_writes_both_files_as_one_step_that_one_undo_takes_back() {
    for (operation, written, body_has, registration_has) in [
        (RENAME_TAG, "name=infosec", "#infosec", "  infosec: red"),
        (DELETE_TAG, "", "category:functional state:accepted", "  legal: purple"),
    ] {
        // Arrange.
        let folder = WebShopFolder::new();
        let mut session = RequirementSession::open(folder.body(), Some(folder.registration()));
        let body = fs::read(folder.body()).unwrap();
        let registration = fs::read(folder.registration()).unwrap();

        // Act.
        run(&mut session, operation, "tag/security", written);

        // Assert: both files changed, security is gone from both, and one undo gives both back.
        let text = fs::read_to_string(folder.body()).unwrap();
        let kept = fs::read_to_string(folder.registration()).unwrap();
        assert!(
            text.contains(&format!(
                "REQ-0006 A session ends after 30 minutes without activity. {body_has}"
            )),
            "{operation}: {text}"
        );
        assert!(!text.contains("#security"), "{operation}");
        assert!(!kept.contains("  security: red"), "{operation}");
        assert!(kept.contains(registration_has), "{operation}: {kept}");
        session.undo().unwrap();
        assert_eq!(body, fs::read(folder.body()).unwrap(), "{operation}");
        assert_eq!(registration, fs::read(folder.registration()).unwrap(), "{operation}");
        assert!(!session.can_undo(), "{operation}");
    }
}

#[test]
fn a_colour_or_a_place_writes_only_the_registration_as_a_step_of_the_history() {
    // Arrange.
    let folder = WebShopFolder::new();
    let mut session = RequirementSession::open(folder.body(), Some(folder.registration()));
    let body = fs::read(folder.body()).unwrap();
    let registration = fs::read(folder.registration()).unwrap();

    // Act.
    run(&mut session, RECOLOUR_TAG, "tag/search", "colour=green");
    run(&mut session, MOVE_TAG, "tag/privacy", "after=tag/customer");

    // Assert: the Markdown file is as it was (User Story 7 scenario 3), and two undos give the registration back.
    assert_eq!(body, fs::read(folder.body()).unwrap());
    let kept = fs::read_to_string(folder.registration()).unwrap();
    assert!(
        kept.contains("tags:\r\n  customer: blue\r\n  privacy: pink\r\n  search: green\r\n"),
        "{kept}"
    );
    for _ in 0..2 {
        session.undo().unwrap();
    }
    assert_eq!(registration, fs::read(folder.registration()).unwrap());
}

#[test]
fn a_tag_the_page_does_not_show_is_refused_at_once() {
    let folder = WebShopFolder::new();
    let mut session = RequirementSession::open(folder.body(), Some(folder.registration()));

    assert_eq!(
        Err(DRIFTED.to_owned()),
        session.gesture(gesture(DELETE_TAG, "tag/gone", ""))
    );
    assert_eq!(
        Err("'a#b' cannot be a tag: a tag's name is one word without #.".to_owned()),
        session.gesture(gesture(ADD_TAG, "REQ-0001", "name=a#b"))
    );
    assert!(session.take_changes().is_empty());
}
