//! `atd record` for a designer of the `table` kind (etalii.adp spec 076, T067): one conformance case per parity
//! script, its documents kept as its inputs, the model's elements as the designer read them, and after the reading and
//! after each step the findings it gave and the bytes it wrote. A designer's page has no recording painter, so a
//! case states no drawing (ATD §2.3).
//!
//! A script's steps are the table's gestures (`setCell`, `addSort`, …); a case's are the operations of the designer's
//! specification that `knowledge.md`'s table of gestures names for them, on the element they act on, with their
//! parameters. A gesture that no one operation makes is kept as the script has it, and named on stderr
//! ([`super::take_unstated`]).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use adp_cel::Value;
use adp_desl::table::model::TableFindingSeverity;
use adp_desl::table::session::DesignerSession;
use adp_fbl::FblModel;
use adp_parity::Case;
use adp_parity::table::Observed;
use serde_json::{Map, Value as Json, json};

use super::{REFERENCE, UNSTATED, copy_inputs, recorded_with, write, write_json};

/// The Knowledge designer's specification, for the attributes each element type declares, in their order, and their
/// defaults: a reading states an attribute that holds nothing but states a default with that default (ATD §2.3).
fn metamodel() -> &'static Json {
    static METAMODEL: OnceLock<Json> = OnceLock::new();
    METAMODEL.get_or_init(|| {
        let text = include_str!("../../../../vendor/etalii.adp/definitions/designers/knowledge.des");
        let specification: Json = serde_json::from_str(text).unwrap_or_default();
        specification["metamodel"]["types"].clone()
    })
}

/// The types of the table's surface, in the order a reading states them (`surface table`: columns, rows, cells,
/// views); the other types follow in reading order.
const SURFACE: [&str; 4] = ["Property", "Row", "Cell", "View"];

/// What the reading or a step left behind.
struct Seen {
    outcome: Result<(), String>,
    generated: Vec<String>,
    bytes: Option<Vec<u8>>,
    model: FblModel,
    findings: Vec<Json>,
    active_view: String,
}

/// A value as a reading states it: a list as a list, a bool as a bool, a number as a number, the rest as text; none
/// for a value that holds nothing.
fn stated(value: &Value) -> Option<Json> {
    match value {
        Value::Null => None,
        Value::Bool(flag) => Some(json!(flag)),
        Value::Int(n) => Some(json!(n)),
        Value::Double(n) => Some(json!(n)),
        Value::List(items) if items.is_empty() => None,
        Value::List(items) => Some(Json::Array(items.iter().filter_map(stated).collect())),
        Value::String(text) if text.is_empty() => None,
        Value::String(text) => Some(json!(text)),
        other => Some(json!(other.to_string())),
    }
}

/// A value as its attribute's type states it: an XML file holds every value as text, which the model means as a bool
/// or a number where the attribute is one.
fn typed_as(value: Json, declared: &str) -> Json {
    let Some(text) = value.as_str() else { return value };
    match declared {
        "bool" => match text {
            "true" => json!(true),
            "false" => json!(false),
            _ => value,
        },
        "int" | "number" => text
            .parse::<i64>()
            .map(|n| json!(n))
            .or_else(|_| text.parse::<f64>().map(|n| json!(n)))
            .unwrap_or(value),
        _ => value,
    }
}

/// The elements the designer read, in the surface's order, each with the attributes its type declares.
fn elements(model: &FblModel) -> Vec<Json> {
    let types = metamodel();
    let document: Vec<&str> = model
        .elements
        .iter()
        .filter(|e| e.type_name == "Table")
        .map(|e| e.id.as_str())
        .collect();
    let rank = |t: &str| SURFACE.iter().position(|s| *s == t).unwrap_or(SURFACE.len());
    let mut ordered: Vec<(usize, usize, &adp_fbl::FblElement)> = model
        .elements
        .iter()
        .filter(|e| !e.is_relation && e.type_name != "Table")
        .enumerate()
        .map(|(index, e)| (rank(&e.type_name), index, e))
        .collect();
    ordered.sort_by_key(|(rank, index, _)| (*rank, *index));
    ordered
        .into_iter()
        .map(|(_, _, element)| {
            let mut attributes = Map::new();
            let declared = types[&element.type_name]["attributes"]
                .as_object()
                .cloned()
                .unwrap_or_default();
            for (name, declaration) in &declared {
                let value = element
                    .attributes
                    .get(name)
                    .and_then(stated)
                    .map(|v| typed_as(v, declaration["type"].as_str().unwrap_or_default()))
                    .or_else(|| declaration.get("default").cloned());
                if let Some(value) = value {
                    attributes.insert(name.clone(), value);
                }
            }
            let parent = element
                .parent_id
                .as_deref()
                .filter(|p| !document.contains(p))
                .map_or(Json::Null, |p| json!(p));
            json!({"id": element.id, "type": element.type_name, "parent": parent, "attributes": attributes})
        })
        .collect()
}

/// The findings the designer gave, in its order: each by its code, on the element it is about (a cell, a row or a
/// property) and that element's line.
fn findings(session: &adp_designer_knowledge::KnowledgeSession) -> Vec<Json> {
    let model = session.model();
    session
        .baseline()
        .findings
        .iter()
        .map(|f| {
            let mut finding = Map::new();
            finding.insert("code".into(), json!(f.code));
            let severity = match f.severity {
                TableFindingSeverity::Error => "error",
                TableFindingSeverity::Warning => "warning",
                TableFindingSeverity::Info => "info",
            };
            finding.insert("severity".into(), json!(severity));
            let element = match (f.row_id.is_empty(), f.column_id.is_empty()) {
                (false, false) => format!("{}/{}", f.row_id, f.column_id),
                (false, true) => f.row_id.clone(),
                (true, false) => f.column_id.clone(),
                (true, true) => String::new(),
            };
            if !element.is_empty() {
                finding.insert("element".into(), json!(element));
                if let Some(line) = model.elements.iter().find(|e| e.id == element).map(|e| e.line) {
                    finding.insert("line".into(), json!(line));
                }
            }
            finding.insert("message".into(), json!(f.message));
            Json::Object(finding)
        })
        .collect()
}

/// A gesture's references with each `$N` put in, from the ids made so far.
fn resolve(text: &str, generated: &[String]) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find('$') {
        out.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
        match digits
            .parse::<usize>()
            .ok()
            .and_then(|n| generated.get(n.checked_sub(1)?))
        {
            Some(id) => out.push_str(id),
            None => out.push_str(&rest[at..=at + digits.len()]),
        }
        rest = &after[digits.len()..];
    }
    out.push_str(rest);
    out
}

/// What a gesture is in a case's terms.
enum Translated {
    /// An operation, on what it is selected on, with its parameters; or an undo or a redo of so many steps.
    Step(Map<String, Json>, usize),
    /// A gesture no one operation makes: as the script has it, and why.
    Unstated(Map<String, Json>, String),
    /// A step a case does not state (a save).
    Skipped,
}

/// A value of a cell's text as its property's value type takes it.
fn typed(value: &str, value_type: &str) -> Json {
    match value_type {
        "number" => value
            .parse::<i64>()
            .map(|n| json!(n))
            .or_else(|_| value.parse::<f64>().map(|n| json!(n)))
            .unwrap_or_else(|_| json!(value)),
        "checkbox" => match value {
            "true" => json!(true),
            "false" => json!(false),
            other => json!(other),
        },
        _ => json!(value),
    }
}

fn operation(action: &str, select: Option<String>, args: Map<String, Json>) -> Translated {
    let mut step = Map::new();
    step.insert("action".into(), json!(action));
    // An operation on the document selects `document` (ATD §2.3), as the table's toolbar acts on it.
    step.insert(
        "select".into(),
        json!([select.unwrap_or_else(|| "document".to_owned())]),
    );
    if !args.is_empty() {
        step.insert("args".into(), Json::Object(args));
    }
    Translated::Step(step, 1)
}

fn args<const N: usize>(pairs: [(&str, Json); N]) -> Map<String, Json> {
    pairs.into_iter().map(|(k, v)| (k.to_owned(), v)).collect()
}

/// A table step as a case's step, on the model as it stood before it.
fn translate(step: &Json, before: &Seen, after: &Seen, generated: &[String]) -> Translated {
    for key in ["undo", "redo"] {
        if let Some(n) = step.get(key) {
            let mut undo = Map::new();
            undo.insert(key.into(), json!(true));
            return Translated::Step(undo, n.as_u64().unwrap_or(1).max(1) as usize);
        }
    }
    if let Some(view) = step.get("view").and_then(Json::as_str) {
        return operation("switchView", Some(resolve(view, generated)), Map::new());
    }
    let Some(gesture) = step.get("table") else {
        return Translated::Skipped;
    };
    let text = |name: &str| gesture.get(name).and_then(Json::as_str).map(|t| resolve(t, generated));
    let setting = |name: &str| gesture["settings"].get(name).and_then(Json::as_str).map(str::to_owned);
    let values: Vec<String> = gesture["values"]
        .as_array()
        .map(|v| {
            v.iter()
                .filter_map(Json::as_str)
                .map(|t| resolve(t, generated))
                .collect()
        })
        .unwrap_or_default();
    let kind = gesture["kind"].as_str().unwrap_or_default();
    let column = text("column").unwrap_or_default();
    let view = text("view")
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| before.active_view.clone());
    let element = |id: &str| before.model.elements.iter().find(|e| e.id == id);
    let attribute = |id: &str, name: &str| {
        element(id)
            .and_then(|e| e.attributes.get(name))
            .map(adp_diagram::document::text_of)
            .unwrap_or_default()
    };
    let unstated = |reason: &str| {
        let mut fields = Map::new();
        fields.insert("action".into(), json!(kind));
        let mut kept = gesture.as_object().cloned().unwrap_or_default();
        kept.remove("kind");
        if !kept.is_empty() {
            fields.insert("args".into(), Json::Object(kept));
        }
        Translated::Unstated(fields, format!("`{kind}` {reason}"))
    };
    match kind {
        "setCell" => {
            let row = text("row").unwrap_or_default();
            let cell = format!("{row}/{column}");
            let value_type = attribute(&column, "valueType");
            let property = json!(column);
            match value_type.as_str() {
                "text" | "number" | "checkbox" | "date" | "dateTime" | "time" | "selection" => {
                    if let Some(name) = setting("newOption").filter(|n| !n.trim().is_empty()) {
                        if value_type != "selection" {
                            return unstated(
                                "with a new option on a property of several is an option and an item added",
                            );
                        }
                        // The colour the host picked for the new option is the gesture's, given to the operation.
                        let made = after
                            .model
                            .elements
                            .iter()
                            .find(|e| e.type_name == "Option" && after.generated.contains(&e.id));
                        let mut given = args([("property", property), ("name", json!(name))]);
                        if let Some(colour) = made.and_then(|o| o.attributes.get("colour")).and_then(stated) {
                            given.insert("colour".into(), colour);
                        }
                        return operation("setNewOption", Some(row), given);
                    }
                    let Some(value) = values.first() else {
                        return match element(&cell) {
                            Some(_) => operation("clearCell", Some(cell), Map::new()),
                            None => unstated("of no value on an empty cell makes no operation"),
                        };
                    };
                    let action = match value_type.as_str() {
                        "text" => "setText",
                        "number" => "setNumber",
                        "checkbox" => "setChecked",
                        "date" => "setDate",
                        "dateTime" => "setDateTime",
                        "time" => "setTime",
                        _ => "setOption",
                    };
                    operation(
                        action,
                        Some(row),
                        args([("property", property), ("value", typed(value, &value_type))]),
                    )
                }
                "multipleSelection" | "relation" => {
                    let key = if value_type == "relation" { "row" } else { "option" };
                    let held: Vec<String> = before
                        .model
                        .elements
                        .iter()
                        .filter(|e| e.type_name == "CellItem" && e.parent_id.as_deref() == Some(cell.as_str()))
                        .map(|e| adp_diagram::document::text_of(e.attributes.get(key).unwrap_or(&Value::Null)))
                        .collect();
                    if values.len() == held.len() + 1 && values[..held.len()] == held[..] {
                        let added = json!(values[held.len()]);
                        return operation("addItem", Some(row), args([("property", property), (key, added)]));
                    }
                    if values.len() + 1 == held.len()
                        && let Some(gone) = held.iter().find(|h| !values.contains(h))
                        && held.iter().filter(|h| *h != gone).eq(values.iter())
                    {
                        return operation("removeItem", Some(format!("{cell}/{gone}")), Map::new());
                    }
                    if values.is_empty() && element(&cell).is_some() {
                        return operation("clearCell", Some(cell), Map::new());
                    }
                    unstated("of several values that are not one added or removed makes several operations")
                }
                _ => unstated("on a property of no known type makes no operation"),
            }
        }
        "addRow" => {
            if setting("group").is_some_and(|g| !g.is_empty()) {
                return unstated("in a group is a row added and its grouping value set, two operations");
            }
            operation("addRow", None, Map::new())
        }
        "deleteRows" => {
            let mut rows: Vec<String> = text("row").into_iter().filter(|r| !r.is_empty()).collect();
            rows.extend(values.iter().cloned());
            if rows.len() != 1 {
                return unstated("of other than one row is not stated with one selection");
            }
            operation("deleteRows", rows.pop(), Map::new())
        }
        "addSort" => operation(
            "addSort",
            Some(view),
            args([
                ("property", json!(column)),
                (
                    "direction",
                    json!(setting("direction").unwrap_or_else(|| "ascending".into())),
                ),
            ]),
        ),
        "setSort" => {
            let sort = format!("{view}/sorts/{column}");
            if element(&sort).is_none() {
                return unstated(
                    "of a property the view does not sort by adds a sort, which `setSortDirection` does not",
                );
            }
            let direction = setting("direction").unwrap_or_else(|| "ascending".into());
            operation("setSortDirection", Some(sort), args([("direction", json!(direction))]))
        }
        "removeSort" => operation("removeSort", Some(format!("{view}/sorts/{column}")), Map::new()),
        "addFilter" => {
            let operator = setting("comparison").unwrap_or_default();
            operation(
                "addCondition",
                Some(view),
                args([("property", json!(column)), ("operator", json!(operator))]),
            )
        }
        "setFilter" => {
            let condition = format!("{view}/filter/{}", text("target").unwrap_or_default());
            let Some(held) = element(&condition) else {
                return unstated("of a condition the view does not have makes no operation");
            };
            let operator = setting("comparison").unwrap_or_else(|| attribute(&condition, "operator"));
            let property = held
                .attributes
                .get("property")
                .map(adp_diagram::document::text_of)
                .unwrap_or_default();
            let mut given = args([("operator", json!(operator))]);
            if let Some(value) = values.first().filter(|v| !v.is_empty()) {
                let value_type = attribute(&property, "valueType");
                let key = match value_type.as_str() {
                    "checkbox" => "checked",
                    "selection" | "multipleSelection" => "option",
                    "text" | "number" | "date" | "dateTime" | "time" => value_type.as_str(),
                    _ => return unstated("on a property of no comparable type makes no operation"),
                };
                given.insert(key.into(), typed(value, &value_type));
            }
            operation("setCondition", Some(condition), given)
        }
        "setHideEmptyGroups" => operation(
            "hideEmptyGroups",
            Some(view),
            args([("hide", json!(setting("hide").as_deref() == Some("true")))]),
        ),
        "toggleGroup" => {
            let key = text("target").unwrap_or_default();
            if setting("collapsed").as_deref() == Some("true") {
                operation(
                    "setGroup",
                    Some(view),
                    args([("slot", json!("collapsed")), ("key", json!(key))]),
                )
            } else {
                operation("unsetGroup", Some(format!("{view}/collapsed/{key}")), Map::new())
            }
        }
        "hideColumn" | "showColumn" => operation(
            "setColumn",
            Some(view),
            args([("property", json!(column)), ("visible", json!(kind == "showColumn"))]),
        ),
        "resizeColumn" => match setting("width").and_then(|w| w.trim().parse::<i64>().ok()) {
            Some(width) => operation(
                "setColumn",
                Some(view),
                args([("property", json!(column)), ("width", json!(width))]),
            ),
            // A width that is no number is given as the gesture gave it, for the operation to refuse.
            None => operation(
                "setColumn",
                Some(view),
                args([
                    ("property", json!(column)),
                    ("width", json!(setting("width").unwrap_or_default())),
                ]),
            ),
        },
        "addView" => operation("addView", None, Map::new()),
        "renameView" => match values.first() {
            Some(name) => operation("renameView", Some(view), args([("name", json!(name))])),
            None => unstated("without a name makes no operation"),
        },
        "deleteView" => operation("deleteView", Some(view), Map::new()),
        "addOption" => match values.first() {
            Some(name) => operation("addOption", Some(column), args([("name", json!(name))])),
            None => unstated("without a name makes no operation"),
        },
        "renameOption" => operation(
            "renameOption",
            text("target"),
            args([("name", json!(values.first().cloned().unwrap_or_default()))]),
        ),
        "recolourOption" => operation(
            "recolourOption",
            text("target"),
            args([("colour", json!(setting("colour").unwrap_or_default()))]),
        ),
        "deleteOption" => operation("deleteOption", text("target"), Map::new()),
        "addColumn" => {
            let mut given = Map::new();
            if let Some(name) = values.first() {
                given.insert("name".into(), json!(name));
            }
            given.insert(
                "valueType".into(),
                json!(setting("type").unwrap_or_else(|| "text".into())),
            );
            operation("addProperty", None, given)
        }
        "renameColumn" => operation(
            "renameProperty",
            Some(column),
            args([("name", json!(values.first().cloned().unwrap_or_default()))]),
        ),
        "deleteColumn" => operation("deleteProperty", Some(column), Map::new()),
        "setColumnType" => operation(
            "changePropertyType",
            Some(column),
            args([("valueType", json!(setting("type").unwrap_or_default()))]),
        ),
        "setColumnParent" if setting("parent").as_deref() == Some("true") => {
            operation("setParentRelation", Some(column), Map::new())
        }
        "groupBy" if column.is_empty() => operation("ungroup", Some(view), Map::new()),
        "groupBy" => operation("groupBy", Some(view), args([("property", json!(column))])),
        "setColumnCalculation" => operation(
            "calculate",
            Some(view),
            args([
                ("property", json!(column)),
                ("calculation", json!(setting("calculate").unwrap_or_default())),
            ]),
        ),
        _ => unstated("has no one operation of `knowledge.md`'s table of gestures"),
    }
}

/// Records one table designer's script as the case in `folder`, which is made anew.
pub(super) fn record_case(case: &Case, folder: &Path, description: &str) -> Result<(), String> {
    let (body, inputs) = copy_inputs(case, folder)?;
    let body_path = Path::new(&body).to_owned();
    let mut seen: Vec<Seen> = Vec::new();
    let verdict = adp_parity::replay_table_observed(case, &mut |observed: &Observed| {
        seen.push(Seen {
            outcome: observed.outcome.clone(),
            generated: observed.generated.clone(),
            bytes: fs::read(observed.project.join(&body_path)).ok(),
            model: observed.session.model().clone(),
            findings: findings(observed.session),
            active_view: observed.session.active_view_id().to_owned(),
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
    let generated: Vec<String> = seen.iter().flat_map(|s| s.generated.iter().cloned()).collect();

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
    for (index, step) in case.script.steps.iter().enumerate() {
        let (before, after) = (&seen[index], &seen[index + 1]);
        let (mut gesture, repeat) = match translate(step, before, after, &generated) {
            Translated::Step(fields, repeat) => (fields, repeat),
            Translated::Unstated(fields, reason) => {
                let name = &case.name;
                UNSTATED.with(|u| {
                    u.borrow_mut()
                        .push(format!("{name}, script step {}: {reason}", index + 1))
                });
                (fields, 1)
            }
            Translated::Skipped => continue,
        };
        if !after.generated.is_empty() {
            gesture.insert("ids".into(), json!(after.generated));
        }
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
        gesture.insert("findings".into(), json!(after.findings));
        steps.push(Json::Object(gesture));
    }

    let desktop = |part: &str| {
        json!({
            "host": "desktop", "repository": "etalii.adp.ide.desktop", "commit": REFERENCE,
            "recordedWith": recorded_with(), "source": format!("atd record ({part})"),
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
            "Recorded by `atd record {}` from {source}/{}.json of etalii.adp.ide.desktop; never edited by hand. A designer's page states no drawing; the table's gestures are written as the operations `knowledge.md`'s table of gestures names, and the steps state the body's bytes.",
            case.script.tool, case.name
        ),
        "tool": case.script.tool,
        "description": description,
        "inputs": inputs,
        "read": {"elements": elements(&opened.model), "findings": opened.findings},
        "steps": steps,
        "provenance": {
            "bytes": bytes,
            "reading": desktop("reading"),
            "findings": desktop("findings"),
        },
    });
    write_json(&folder.join("conformance.json"), &fixture)
}
