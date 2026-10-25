//! The operations on assessments of `hierarchical-requirement.des` (the standalone host's `AssessmentOperations` at
//! a7fdf03; FR-026): `addAssessment` on an entry, and `changeAssessment`, `resolveAssessment` and `removeAssessment` on
//! an assessment, with the numbering every edit writes (spec 029 R17). What they write is what the conformance fixtures
//! state, byte for byte: an add at the end of Assessments, the section appended when the file has none, and a resolve
//! one character where the line has a checkbox.

use std::collections::BTreeMap;

use adp_cel::{Map, Value};
use adp_fbl::ModelChange;

use super::body_edit::BodyEdit;
use crate::body::RequirementBody;
use crate::gestures::DRIFTED;
use crate::mapper;
use crate::model::AssessmentKind;

pub const ADD_ASSESSMENT: &str = "addAssessment";
pub const CHANGE_ASSESSMENT: &str = "changeAssessment";
pub const RESOLVE_ASSESSMENT: &str = "resolveAssessment";
pub const REMOVE_ASSESSMENT: &str = "removeAssessment";

const ASSESSMENT: &str = "Assessment";

/// Whether `operation` is one of these.
pub fn handles(operation: &str) -> bool {
    [ADD_ASSESSMENT, CHANGE_ASSESSMENT, RESOLVE_ASSESSMENT, REMOVE_ASSESSMENT].contains(&operation)
}

/// Whether `operation` names an assessment rather than an entry.
pub fn targets_an_assessment(operation: &str) -> bool {
    [CHANGE_ASSESSMENT, RESOLVE_ASSESSMENT, REMOVE_ASSESSMENT].contains(&operation)
}

/// Whether `params` can be written by `operation`: the sentence saying why not, or empty. The text is the rest of its
/// bullet's line, so it is one line and says something; the kind is one of the four.
pub fn refusal(operation: &str, params: &BTreeMap<String, String>) -> String {
    if operation != ADD_ASSESSMENT && operation != CHANGE_ASSESSMENT {
        return String::new();
    }
    if let Some(kind) = params.get("kind")
        && AssessmentKind::parse(kind).is_none()
    {
        return format!("'{kind}' is not a kind of assessment: a question, remark, warning or idea.");
    }
    if operation == CHANGE_ASSESSMENT && !params.contains_key("kind") {
        return format!("'{operation}' needs its kind.");
    }
    let text = params.get("text").map_or("", String::as_str);
    if text.trim().is_empty() {
        return "An assessment needs a text.".into();
    }
    if text.contains(['\r', '\n']) {
        return "The text of an assessment is one line; its detail holds the lines below it.".into();
    }
    String::new()
}

/// The bytes of the body `body`, named `file_name`, after `operation` on `target_id`: the surface's id of an entry for
/// an add, the assessment's id for the others; or the sentence saying why it cannot be, in which case nothing changed.
/// A target the file no longer has is refused as a change made against an older file.
pub fn apply(
    body: &[u8],
    file_name: &str,
    operation: &str,
    target_id: &str,
    params: &BTreeMap<String, String>,
) -> Result<Vec<u8>, String> {
    let refused = refusal(operation, params);
    if !refused.is_empty() {
        return Err(refused);
    }
    let mut read = RequirementBody::read(body.to_vec(), file_name, None);
    if !read.document.read_only_reason.is_empty() {
        return Err(read.document.read_only_reason.clone());
    }
    let elements: Vec<String> = read
        .body
        .model()
        .elements
        .into_iter()
        .filter(|e| e.type_name == ASSESSMENT)
        .map(|e| e.id)
        .collect();
    let text = |name: &str| Value::String(params.get(name).cloned().unwrap_or_default());

    if operation == ADD_ASSESSMENT {
        let ids = mapper::node_ids(&read.document.entries);
        let Some(at) = ids.iter().position(|id| id == target_id) else {
            return Err(DRIFTED.into());
        };
        // An assessment names its entry by number; a placeholder's is the one this same edit writes for it.
        let about = read.document.entries[at].id.clone();
        let kind = params.get("kind").map_or("Question", String::as_str);
        let attributes: Map = [
            ("kind", Value::String(stored(kind).into())),
            ("about", Value::String(about)),
            ("text", text("text")),
            ("resolved", Value::Bool(false)),
        ]
        .into_iter()
        .collect();
        let mut edit = BodyEdit::new(&mut read);
        edit.change(ModelChange::Add {
            type_name: ASSESSMENT.into(),
            id: Some(format!("assessment/{}", elements.len())),
            attributes,
            parent_id: None,
            index: None,
            slot: None,
        });
        return edit.write();
    }

    let Some(place) = read.document.assessments.iter().position(|a| a.id == target_id) else {
        return Err(DRIFTED.into());
    };
    let resolved = read.document.assessments[place].resolved;
    let element = elements[place].clone();
    let mut edit = BodyEdit::new(&mut read);
    match operation {
        CHANGE_ASSESSMENT => {
            let kind = params.get("kind").map_or("", String::as_str);
            edit.set(&element, "kind", Value::String(stored(kind).into()));
            edit.set(&element, "text", text("text"));
            edit.set(&element, "detail", text("detail"));
        }
        RESOLVE_ASSESSMENT => edit.set(&element, "resolved", Value::Bool(!resolved)),
        _ => edit.change(ModelChange::Remove { id: element }),
    }
    edit.write()
}

/// A kind as the file writes it, its stored form: `Question` for question in any case.
fn stored(kind: &str) -> &'static str {
    AssessmentKind::parse(kind).map_or("Question", AssessmentKind::name)
}
