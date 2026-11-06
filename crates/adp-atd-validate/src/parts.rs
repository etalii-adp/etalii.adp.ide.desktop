//! A case's parts as `atd record` writes them and `atd validate` compares them (ATD §2.3): what reading gave, and
//! the findings. Moved from `atd record` (etalii.adp spec 035, T050), so that the two are the same code.

use adp_diagram::problems::{DiagramProblemLocation, DiagramProblemSeverity};
use adp_diagram::{DiagramDocuments, DiagramElement, DiagramSession, ElementKind};
use serde_json::{Map, Value as Json, json};

/// An element's attributes as a reading states them: by name, as text, the empty ones left out.
pub fn attributes(element: &DiagramElement) -> Json {
    Json::Object(
        element
            .attributes
            .iter()
            .filter(|(_, v)| !v.is_empty())
            .map(|(k, v)| (k.clone(), json!(v)))
            .collect(),
    )
}

/// What reading gave: the elements with their parents and attributes, and the relations with their ends.
pub fn reading(elements: &[DiagramElement]) -> (Vec<Json>, Vec<Json>) {
    let (mut nodes, mut relations) = (Vec::new(), Vec::new());
    for element in elements {
        match element.kind {
            ElementKind::Node => nodes.push(json!({
                "id": element.id, "type": element.element_type, "parent": element.parent,
                "attributes": attributes(element),
            })),
            ElementKind::Relation => relations.push(json!({
                "id": element.id, "type": element.element_type,
                "source": element.source.clone().unwrap_or_default(),
                "target": element.target.clone().unwrap_or_default(),
                "attributes": attributes(element),
            })),
        }
    }
    (nodes, relations)
}

/// The findings of the open document, as the errors panel reports them.
pub fn findings(session: &DiagramSession, documents: &DiagramDocuments) -> Vec<Json> {
    session
        .problems(documents)
        .into_iter()
        .map(|problem| {
            let mut finding = Map::new();
            finding.insert("rule".into(), json!(problem.rule_id));
            finding.insert(
                "severity".into(),
                json!(match problem.severity {
                    DiagramProblemSeverity::Error => "error",
                    DiagramProblemSeverity::Warning => "warning",
                    DiagramProblemSeverity::Info => "information",
                }),
            );
            match problem.location {
                Some(DiagramProblemLocation::Element(id)) => {
                    finding.insert("element".into(), json!(id));
                }
                Some(DiagramProblemLocation::Line(line)) => {
                    finding.insert("line".into(), json!(line));
                }
                _ => {}
            }
            finding.insert("message".into(), json!(problem.message));
            Json::Object(finding)
        })
        .collect()
}

/// What reading a designer's document gave (ATD §2.3): its elements in the order `types` lists their types (the
/// surface's), each in reading order, with its id, type, parent and attributes as the model holds them, a list as a
/// list, a bool as a bool, an enum by the value the document writes, and an attribute that holds nothing but states a
/// default with that default.
pub fn designer_reading(document: &adp_diagram::DiagramDocument, types: &[String]) -> Vec<Json> {
    let specification = document.tool().definition.specification();
    let diagram = &document.model().diagram;
    // A type of the tree (`tree:<type>`) ranks with every other type of the tree.
    let rank = |t: &str| {
        let tree = format!("tree:{t}");
        types
            .iter()
            .position(|n| n == t || *n == tree)
            .map(|p| if types[p].starts_with("tree:") { 0 } else { p + 1 })
            .unwrap_or(types.len() + 1)
    };
    let mut nodes: Vec<(usize, usize, adp_disl::DislElement)> = diagram
        .nodes()
        .into_iter()
        .enumerate()
        .map(|(index, node)| (rank(&node.type_name()), index, node))
        .collect();
    nodes.sort_by_key(|(rank, index, _)| (*rank, *index));
    nodes
        .into_iter()
        .map(|(_, _, node)| {
            let element_type = node.element_type();
            let held = node.attributes();
            let mut attributes = Map::new();
            for (name, declared) in &element_type.attributes {
                let value = match held.get(name) {
                    Some(value) if !value.is_null() => value.clone(),
                    _ => match &declared.default {
                        Some(Json::Bool(flag)) => adp_cel::Value::Bool(*flag),
                        Some(Json::Number(n)) if n.is_i64() => adp_cel::Value::Int(n.as_i64().unwrap_or_default()),
                        Some(Json::Number(n)) => adp_cel::Value::Double(n.as_f64().unwrap_or_default()),
                        Some(Json::String(text)) => adp_cel::Value::String(text.clone()),
                        _ => continue,
                    },
                };
                let stored = adp_disl::behavior::write::stored_form(specification, &element_type.name, name, &value);
                let json = match stored {
                    adp_cel::Value::List(items) => {
                        if items.is_empty() {
                            continue;
                        }
                        Json::Array(items.iter().map(|i| json!(adp_diagram::document::text_of(i))).collect())
                    }
                    adp_cel::Value::Bool(flag) => json!(flag),
                    adp_cel::Value::Int(n) => json!(n),
                    adp_cel::Value::Double(n) => json!(n),
                    other => {
                        let text = adp_diagram::document::text_of(&other);
                        if text.is_empty() {
                            continue;
                        }
                        json!(text)
                    }
                };
                attributes.insert(name.clone(), json);
            }
            json!({
                "id": node.id(), "type": node.type_name(), "parent": node.parent().map(|p| p.id()),
                "attributes": attributes,
            })
        })
        .collect()
}

/// The findings of a designer's open document (ATD §2.3): each by its `code`, with the element it is on and that
/// element's line, or the line alone.
pub fn designer_findings(session: &DiagramSession, documents: &DiagramDocuments) -> Vec<Json> {
    // The specification's findings state their line beside their element: a mention is at its own line (DESL 6.5).
    if let Some(findings) = session.findings(documents) {
        let document = documents.get(session.body_path());
        return findings
            .into_iter()
            .map(|found| {
                let mut finding = Map::new();
                finding.insert("code".into(), json!(found.code));
                finding.insert("severity".into(), json!(found.severity));
                let element = found.element_ids.first().filter(|id| !id.is_empty());
                let line = found
                    .line
                    .or_else(|| element.and_then(|id| document.and_then(|d| d.element(id)).and_then(|e| e.line())));
                if let Some(id) = element {
                    finding.insert("element".into(), json!(id));
                }
                if let Some(line) = line {
                    finding.insert("line".into(), json!(line));
                }
                finding.insert("message".into(), json!(found.message));
                Json::Object(finding)
            })
            .chain(document.into_iter().flat_map(stale_settings))
            .collect();
    }
    let document = documents.get(session.body_path());
    session
        .problems(documents)
        .into_iter()
        .map(|problem| {
            let mut finding = Map::new();
            finding.insert("code".into(), json!(problem.rule_id));
            finding.insert(
                "severity".into(),
                json!(match problem.severity {
                    DiagramProblemSeverity::Error => "error",
                    DiagramProblemSeverity::Warning => "warning",
                    DiagramProblemSeverity::Info => "information",
                }),
            );
            match problem.location {
                Some(DiagramProblemLocation::Element(id)) => {
                    let line = document.and_then(|d| d.element(&id)).and_then(|e| e.line());
                    finding.insert("element".into(), json!(id));
                    if let Some(line) = line {
                        finding.insert("line".into(), json!(line));
                    }
                }
                Some(DiagramProblemLocation::Line(line)) => {
                    finding.insert("line".into(), json!(line));
                }
                _ => {}
            }
            finding.insert("message".into(), json!(problem.message));
            Json::Object(finding)
        })
        .collect()
}

/// The entries of the registration's blocks keyed by ids that name no element of the document (FBL section 8.1),
/// after the document's own findings, as a designer reports them.
fn stale_settings(document: &adp_diagram::document::DiagramDocument) -> Vec<Json> {
    let Some(registration) = document.registration() else {
        return Vec::new();
    };
    // Read with the blocks the binding declares, which the reading's registration need not know.
    let declarations = &document.tool().binding.registration.blocks;
    let declared: Vec<String> = declarations.iter().map(|d| d.name.clone()).collect();
    let registration = match document.registration_bytes() {
        Some(bytes) => adp_fbl::RegistrationDocument::read_declared(bytes.to_vec(), &declared),
        None => registration.clone(),
    };
    let ids: std::collections::HashSet<String> = document
        .model()
        .diagram
        .elements()
        .iter()
        .map(|e| e.read_id().unwrap_or_else(|| e.id()))
        .collect();
    let name = document
        .registration_path()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    registration
        .stale_entries(&ids, &name, declarations)
        .into_iter()
        .map(|found| {
            let mut finding = Map::new();
            finding.insert("code".into(), json!(found.code));
            finding.insert(
                "severity".into(),
                json!(match format!("{:?}", found.severity).as_str() {
                    "Error" => "error",
                    "Warning" => "warning",
                    _ => "information",
                }),
            );
            if let Some(location) = &found.location {
                finding.insert("line".into(), json!(location.line));
            }
            finding.insert("message".into(), json!(found.message));
            Json::Object(finding)
        })
        .collect()
}
