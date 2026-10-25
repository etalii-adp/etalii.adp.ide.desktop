//! `addAssessment`, `changeAssessment`, `resolveAssessment` and `removeAssessment` make the bytes the conformance
//! fixtures state (the standalone host's `AssessmentOperations.Tests.cs` at a7fdf03; spec 029 T083, T087, FR-015,
//! FR-016, FR-026, R17): each step is the fixture's own, run through the module's operation from the surface's ids, and
//! each starts from the input, since every edit there is followed by its undo. Through the session, each is one step of
//! undo, and an assessment the page does not show is refused at once.

mod common;

use std::collections::BTreeMap;
use std::fs;

use adp_designer_hierarchical_requirement::body::RequirementBody;
use adp_designer_hierarchical_requirement::gestures::DRIFTED;
use adp_designer_hierarchical_requirement::mapper;
use adp_designer_hierarchical_requirement::model::AssessmentKind;
use adp_designer_hierarchical_requirement::operations::assessments::{
    self, ADD_ASSESSMENT, CHANGE_ASSESSMENT, REMOVE_ASSESSMENT, RESOLVE_ASSESSMENT,
};
use adp_designer_hierarchical_requirement::session::RequirementSession;
use adp_desl::hierarchy::model::Gesture;
use adp_desl::hierarchy::session::HierarchySurfaceSession;
use common::{WebShopFolder, outcome};
use serde_json::Value as Json;

/// Every step of the corpus that changes an assessment: the fixture, the step, the gesture and its params.
const STEPS: [(&str, usize, &str, &str, &str); 7] = [
    (
        "hierarchical-requirement-assessments",
        0,
        ADD_ASSESSMENT,
        "REQ-0003",
        "text=Does a refreshed token count as activity?",
    ),
    (
        "hierarchical-requirement-assessments",
        2,
        CHANGE_ASSESSMENT,
        "assessment/0",
        "kind=Remark|text=Is 30 minutes right on a phone?|detail=",
    ),
    (
        "hierarchical-requirement-assessments",
        4,
        RESOLVE_ASSESSMENT,
        "assessment/0",
        "",
    ),
    (
        "hierarchical-requirement-assessments",
        6,
        RESOLVE_ASSESSMENT,
        "assessment/1",
        "",
    ),
    (
        "hierarchical-requirement-assessments",
        8,
        RESOLVE_ASSESSMENT,
        "assessment/2",
        "",
    ),
    (
        "hierarchical-requirement-assessments",
        10,
        REMOVE_ASSESSMENT,
        "assessment/1",
        "",
    ),
    (
        "hierarchical-requirement-assessments/first-assessment",
        0,
        ADD_ASSESSMENT,
        "REQ-0001",
        "kind=question|text=Is 30 minutes right for the mobile app?",
    ),
];

fn params(written: &str) -> BTreeMap<String, String> {
    written
        .split('|')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (name, value) = pair.split_once('=').unwrap();
            (name.to_owned(), value.to_owned())
        })
        .collect()
}

#[test]
fn a_gesture_writes_the_fixtures_bytes() {
    for (fixture, step, operation, target, written) in STEPS {
        // Arrange.
        let folder = common::fixture(fixture);
        let json: Json = serde_json::from_str(&fs::read_to_string(folder.join("fixture.json")).unwrap()).unwrap();
        let input_name = json["input"].as_str().unwrap();
        let input = fs::read(folder.join(input_name)).unwrap();

        // Act.
        let bytes = assessments::apply(&input, input_name, operation, target, &params(written));

        // Assert.
        let bytes = bytes.unwrap_or_else(|refusal| panic!("{fixture} step {step} is refused: {refusal}"));
        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            json["steps"][step]["expect"].as_str().unwrap(),
            "{fixture} step {step}"
        );
    }
}

#[test]
fn an_assessment_of_a_placeholder_names_the_number_the_same_edit_gives_it() {
    // Arrange: the first REQ-NEW of the numbering fixture, which any edit gives a number.
    let input = fs::read(common::fixture("hierarchical-requirement-edits/numbering/web-shop.md")).unwrap();
    let read = RequirementBody::read(input.clone(), "web-shop.md", None);
    let ids = mapper::node_ids(&read.document.entries);
    let at = read
        .document
        .entries
        .iter()
        .position(|e| e.placeholder && !e.is_item())
        .unwrap();

    // Act.
    let bytes = assessments::apply(
        &input,
        "web-shop.md",
        ADD_ASSESSMENT,
        &ids[at],
        &params("kind=Warning|text=Is this one needed?"),
    )
    .unwrap();

    // Assert: the assessment names the number the placeholder now has in the file.
    let read = RequirementBody::read(bytes, "web-shop.md", None).document;
    let added = read.assessments.last().unwrap();
    assert_eq!(
        (AssessmentKind::Warning, "Is this one needed?", false),
        (added.kind, added.text.as_str(), added.resolved)
    );
    assert!(
        read.entries
            .iter()
            .any(|e| e.id == added.about && !e.is_item() && !e.placeholder)
    );
}

#[test]
fn what_cannot_be_written_is_refused_and_a_gone_target_is_a_change_made_against_an_older_file() {
    let input = fs::read(common::fixture("hierarchical-requirement-assessments/web-shop.md")).unwrap();
    let apply = |operation: &str, target: &str, written: &str| {
        assessments::apply(&input, "web-shop.md", operation, target, &params(written))
    };

    assert_eq!(
        Err("An assessment needs a text.".to_owned()),
        apply(ADD_ASSESSMENT, "REQ-0001", "text= ")
    );
    assert_eq!(
        Err("The text of an assessment is one line; its detail holds the lines below it.".to_owned()),
        apply(CHANGE_ASSESSMENT, "assessment/0", "kind=Question|text=One\nTwo")
    );
    assert_eq!(
        Err("'Doubt' is not a kind of assessment: a question, remark, warning or idea.".to_owned()),
        apply(ADD_ASSESSMENT, "REQ-0001", "kind=Doubt|text=Hm")
    );
    assert_eq!(
        Err("'changeAssessment' needs its kind.".to_owned()),
        apply(CHANGE_ASSESSMENT, "assessment/0", "text=Hm")
    );
    assert_eq!(Err(DRIFTED.to_owned()), apply(RESOLVE_ASSESSMENT, "assessment/9", ""));
    assert_eq!(Err(DRIFTED.to_owned()), apply(ADD_ASSESSMENT, "REQ-0999", "text=Gone"));
}

fn gesture(operation: &str, target: &str, written: &str) -> Gesture {
    Gesture {
        edit_id: format!("edit-{operation}-{target}"),
        operation: operation.into(),
        target_id: target.into(),
        params: params(written),
    }
}

#[test]
fn each_gesture_is_written_as_one_step_of_the_history() {
    // Arrange.
    let folder = WebShopFolder::new();
    let mut session = RequirementSession::open(folder.body(), Some(folder.registration()));
    let before = fs::read(folder.body()).unwrap();

    // Act: a question added to REQ-0004, the example's first assessment resolved, its third changed, its second removed.
    for (operation, target, written) in [
        (
            ADD_ASSESSMENT,
            "REQ-0004",
            "kind=Question|text=Is a second e-mail sent?",
        ),
        (RESOLVE_ASSESSMENT, "assessment/0", ""),
        (
            CHANGE_ASSESSMENT,
            "assessment/2",
            "kind=Warning|text=Two customers asked for Klarna.|detail=Both in October.",
        ),
        (REMOVE_ASSESSMENT, "assessment/1", ""),
    ] {
        session.gesture(gesture(operation, target, written)).unwrap();
        assert_eq!(
            Some((true, String::new())),
            outcome(&session.take_changes()),
            "{operation}"
        );
    }

    // Assert: the file says each, and four undos give the file back.
    let text = fs::read_to_string(folder.body()).unwrap();
    assert!(
        text.contains("- [ ] Question REQ-0004: Is a second e-mail sent?"),
        "{text}"
    );
    assert!(
        text.contains("- [x] Question REQ-0006: Is 30 minutes right for the mobile app?"),
        "{text}"
    );
    assert!(
        text.contains("- [ ] Warning REQ-0003: Two customers asked for Klarna.\r\n  Both in October."),
        "{text}"
    );
    assert!(!text.contains("Overlaps with the payment provider"), "{text}");
    for _ in 0..4 {
        session.undo().unwrap();
    }
    assert_eq!(before, fs::read(folder.body()).unwrap());
}

#[test]
fn an_assessment_the_page_does_not_show_is_refused_at_once() {
    let folder = WebShopFolder::new();
    let mut session = RequirementSession::open(folder.body(), Some(folder.registration()));

    assert_eq!(
        Err(DRIFTED.to_owned()),
        session.gesture(gesture(RESOLVE_ASSESSMENT, "assessment/9", ""))
    );
}
