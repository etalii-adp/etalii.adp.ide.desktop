//! Drift before an edit (FBL 0.7 section 7.2; the standalone host's `History/OpenBodyDrift.Tests.cs`): an edit planned
//! against bytes that are no longer the body's is refused with the library's one sentence, nothing is written, and
//! the body is read again.

use std::path::Path;
use std::rc::Rc;

use adp_cel::{Map, Value};

use super::{DRIFT_EDIT, OpenBody};
use crate::documents::{self, ProblemSeverity};
use crate::model::FblOptions;
use crate::planning::{ModelChange, PlanResult};

const TIMELINE: &str = "elements:\n  - id: a\n    label: Alpha\n    start: 2026-01-01\n";

fn open(text: &str) -> OpenBody {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/specifications/fbl/timeline.fbl");
    let (problems, loaded) = documents::load_file(&path);
    assert!(
        !problems.iter().any(|p| p.severity == ProblemSeverity::Error),
        "{problems:?}"
    );
    let binding = Rc::new(loaded.unwrap().binding("timeline").unwrap().clone());
    let options = FblOptions {
        file_name: "plan.tml".into(),
        ..FblOptions::default()
    };
    OpenBody::open(text.as_bytes().to_vec(), binding, Rc::new(options))
}

fn label(text: &str) -> ModelChange {
    let mut attributes = Map::default();
    attributes.insert("label", Value::String(text.into()));
    ModelChange::Set {
        id: "a".into(),
        attributes,
    }
}

#[test]
fn an_edit_planned_after_the_body_changed_on_disk_is_refused_and_writes_nothing() {
    // The body as read, and the same file after another writer changed it.
    let mut body = open(TIMELINE);
    let read = body.bytes().to_vec();
    let on_disk = TIMELINE.replace("Alpha", "Gamma").into_bytes();

    let result = body.change_against(&label("Beta"), &on_disk);

    // Refused, nothing of the edit applied, and the body read again as it is on disk (FBL 0.7 sections 7.2, 7.3).
    assert_eq!(result, PlanResult::Refused(DRIFT_EDIT.into()));
    assert_ne!(read, body.bytes());
    assert_eq!(on_disk, body.bytes());
    assert!(!String::from_utf8_lossy(body.bytes()).contains("Beta"));
    assert!(!body.can_undo());
}

#[test]
fn an_outside_change_ends_the_history() {
    let mut body = open(TIMELINE);
    assert!(matches!(body.change(&label("Beta")), PlanResult::Planned(_)));
    assert!(body.can_undo());
    let on_disk = String::from_utf8_lossy(body.bytes())
        .replace("Beta", "Gamma")
        .into_bytes();

    let result = body.change_against(&label("Delta"), &on_disk);

    assert_eq!(result, PlanResult::Refused(DRIFT_EDIT.into()));
    assert!(
        !body.can_undo(),
        "the history ends with an outside change (FBL section 7.3)"
    );
    assert_eq!(
        body.model().find("a").unwrap().attributes.get("label"),
        Some(&Value::String("Gamma".into()))
    );
}

#[test]
fn an_edit_planned_against_the_bytes_on_disk_is_written() {
    let mut body = open(TIMELINE);
    let current = body.bytes().to_vec();

    let result = body.change_against(&label("Beta"), &current);

    assert!(matches!(result, PlanResult::Planned(_)));
    assert!(String::from_utf8_lossy(body.bytes()).contains("Beta"));
}

#[test]
fn the_sentence_is_the_one_fbl_7_2_gives() {
    assert_eq!(
        DRIFT_EDIT,
        "The file has changed since this edit was made, so it was not written."
    );
}
