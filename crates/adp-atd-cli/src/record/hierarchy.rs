//! `atd record` for a designer of the `hierarchy` kind (etalii.adp spec 035, T167): one conformance case per parity
//! script, its documents kept as its inputs, and after the reading and after each step what the designer's page read,
//! the findings it gave and the bytes it wrote. A designer's page has no recording painter, so a case states no
//! drawing. The desktop host had no hierarchy designer at its reference commit (`fe5feec`), so the reading and the
//! findings are the desktop host's at the commit that records them (ATD §2.3).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use adp_desl::hierarchy::model::SurfaceModel;
use adp_desl::hierarchy::session::HierarchySurfaceSession;
use adp_parity::Case;
use adp_parity::hierarchy::Observed;
use serde_json::{Map, Value as Json, json};

use super::{copy_inputs, recorded_with, write, write_json};

/// What the reading or a step left behind.
struct Seen {
    outcome: Result<(), String>,
    bytes: Option<Vec<u8>>,
    model: SurfaceModel,
}

/// The element type a label's members are, by the label's attribute (`hierarchical-requirement.des`, `surface`).
fn member_type(label: &str) -> String {
    match label {
        "tags" => "Tag".into(),
        "category" => "Category".into(),
        "state" => "State".into(),
        other => other.to_owned(),
    }
}

/// The elements the page read, in its order: the items and requirements, the members of the label lists, the
/// assessments, the stored filters and the widths of the columns, each with the attributes the model names them by.
fn elements(model: &SurfaceModel) -> Vec<Json> {
    let mut out = Vec::new();
    for node in &model.nodes {
        let mut attributes = Map::new();
        attributes.insert("title".into(), json!(node.title));
        if !node.description.is_empty() {
            attributes.insert("description".into(), json!(node.description));
        }
        for label in &node.labels {
            if label.values.is_empty() {
                continue;
            }
            let value = if label.label == "tags" {
                json!(label.values)
            } else {
                json!(label.values[0])
            };
            attributes.insert(label.label.clone(), value);
        }
        attributes.insert("collapsed".into(), json!(node.collapsed));
        let parent = if node.parent_id.is_empty() {
            Json::Null
        } else {
            json!(node.parent_id)
        };
        out.push(json!({"id": node.id, "type": node.type_name, "parent": parent, "attributes": attributes}));
    }
    for list in &model.option_lists {
        for option in &list.options {
            out.push(json!({
                "id": option.id, "type": member_type(&list.label), "parent": null,
                "attributes": {"name": option.name, "colour": option.colour},
            }));
        }
    }
    for note in &model.notes {
        let mut attributes = Map::new();
        attributes.insert("kind".into(), json!(note.kind));
        attributes.insert("about".into(), json!(note.about));
        attributes.insert("text".into(), json!(note.text));
        if !note.detail.is_empty() {
            attributes.insert("detail".into(), json!(note.detail));
        }
        attributes.insert("resolved".into(), json!(note.resolved));
        out.push(json!({"id": note.id, "type": "Assessment", "parent": null, "attributes": attributes}));
    }
    for filter in &model.filters {
        out.push(json!({
            "id": filter.id, "type": "Filter", "parent": null,
            "attributes": {"name": filter.name, "query": filter.query},
        }));
    }
    for width in &model.widths {
        out.push(json!({
            "id": width.id, "type": "Column", "parent": null,
            "attributes": {"level": width.level.to_string(), "width": width.width},
        }));
    }
    out
}

/// The findings the page gave, in its order.
fn findings(model: &SurfaceModel) -> Vec<Json> {
    model
        .findings
        .iter()
        .map(|f| {
            let mut finding = Map::new();
            finding.insert("code".into(), json!(f.code));
            finding.insert("severity".into(), json!(f.severity));
            if !f.node_id.is_empty() {
                finding.insert("element".into(), json!(f.node_id));
            }
            if f.line > 0 {
                finding.insert("line".into(), json!(f.line));
            }
            finding.insert("message".into(), json!(f.message));
            Json::Object(finding)
        })
        .collect()
}

/// A parity script's step as a conformance step's gesture: a page gesture is the operation it names, on its target,
/// with its parameters; an edit outside the host is `outside`; the explorer's add is its action. `None` for a step a
/// case does not state; the number is how many steps it stands for (an undo or a redo of several).
fn gesture(step: &Json) -> Option<(Map<String, Json>, usize)> {
    let mut gesture = Map::new();
    if let Some(page) = step.get("hierarchy") {
        gesture.insert("action".into(), page.get("operation").cloned().unwrap_or_default());
        if let Some(target) = page.get("target").and_then(Json::as_str).filter(|t| !t.is_empty()) {
            gesture.insert("select".into(), json!([target]));
        }
        if let Some(params) = page
            .get("params")
            .filter(|p| p.as_object().is_some_and(|o| !o.is_empty()))
        {
            gesture.insert("args".into(), params.clone());
        }
        return Some((gesture, 1));
    }
    if let Some(outside) = step.get("outside") {
        gesture.insert("outside".into(), outside.clone());
        return Some((gesture, 1));
    }
    if let Some(action) = step.get("action") {
        gesture.insert("action".into(), action.clone());
        if let Some(args) = step
            .get("args")
            .filter(|a| a.as_object().is_some_and(|o| !o.is_empty()))
        {
            gesture.insert("args".into(), args.clone());
        }
        return Some((gesture, 1));
    }
    for key in ["undo", "redo"] {
        if let Some(n) = step.get(key) {
            gesture.insert(key.into(), json!(true));
            return Some((gesture, n.as_u64().unwrap_or(1).max(1) as usize));
        }
    }
    None
}

/// Records one hierarchy designer's script as the case in `folder`, which is made anew.
pub(super) fn record_case(case: &Case, folder: &Path, description: &str) -> Result<(), String> {
    let (body, inputs) = copy_inputs(case, folder)?;
    let body_path = Path::new(&body).to_owned();
    let mut seen: Vec<Seen> = Vec::new();
    let verdict = adp_parity::replay_hierarchy_observed(case, &mut |observed: &Observed| {
        seen.push(Seen {
            outcome: observed.outcome.clone(),
            bytes: fs::read(observed.project.join(&body_path)).ok(),
            model: observed.session.model(),
        });
    })?;
    if !verdict.matches() {
        return Err(format!(
            "{}: the replay does not end in the golden's bytes:\n{verdict}",
            case.name
        ));
    }
    let Some(opened) = seen.first() else {
        return Err(format!("{}: the document did not open", case.name));
    };

    // The bytes each step leaves, each distinct text written once: the inputs' own name for the bytes the case
    // started with, `after-<n>` for the others.
    let extension = body_path
        .extension()
        .map(|e| e.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut texts: BTreeMap<Vec<u8>, String> = BTreeMap::new();
    if let Some(bytes) = &opened.bytes {
        texts.insert(bytes.clone(), body.clone());
    }
    let mut steps: Vec<Json> = Vec::new();
    for (step, after) in case.script.steps.iter().zip(seen.iter().skip(1)) {
        let Some((mut gesture, repeat)) = gesture(step) else {
            continue;
        };
        if let Err(reason) = &after.outcome {
            gesture.insert("refused".into(), json!({"reason": reason}));
        }
        // Several undos or redos at once are that many steps; what they leave is stated on the last.
        for _ in 1..repeat {
            steps.push(Json::Object(gesture.clone()));
        }
        if let Some(bytes) = &after.bytes {
            let next = texts.len();
            let name = texts
                .entry(bytes.clone())
                .or_insert_with(|| format!("after-{next}.{extension}"))
                .clone();
            if !folder.join(&name).exists() {
                write(&folder.join(&name), bytes)?;
            }
            gesture.insert("expectFile".into(), json!(name));
        }
        gesture.insert("findings".into(), json!(findings(&after.model)));
        steps.push(Json::Object(gesture));
    }

    let commit = recorded_with();
    let desktop = |part: &str| {
        json!({
            "host": "desktop", "repository": "etalii.adp.ide.desktop", "commit": commit,
            "recordedWith": commit, "source": format!("atd record ({part})"),
        })
    };
    let standalone = case.recorded.standalone_commit.chars().take(7).collect::<String>();
    let bytes = if case.minted {
        desktop("bytes")
    } else {
        json!({
            "host": "standalone", "repository": "etalii.adp.ide.standalone", "commit": standalone,
            "source": format!("etalii.adp.ide.desktop/parity/goldens/{}/", case.name),
        })
    };
    let source = if case.minted { "parity/cases" } else { "parity/scripts" };
    let fixture = json!({
        "$schema": "https://etalii.net/adp/atd/schema/0.1/atd.schema.json#/$defs/ConformanceFixture",
        "doc": format!(
            "Recorded by `atd record {}` from {source}/{}.json of etalii.adp.ide.desktop; never edited by hand. A designer's page states no drawing, and the steps state the body's bytes, not the registration's.",
            case.script.tool, case.name
        ),
        "tool": case.script.tool,
        "description": description,
        "inputs": inputs,
        "read": {"elements": elements(&opened.model), "findings": findings(&opened.model)},
        "steps": steps,
        "provenance": {
            "bytes": bytes,
            "reading": desktop("reading"),
            "findings": desktop("findings"),
        },
    });
    write_json(&folder.join("conformance.json"), &fixture)
}
