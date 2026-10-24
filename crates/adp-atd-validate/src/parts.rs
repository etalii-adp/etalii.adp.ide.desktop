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
