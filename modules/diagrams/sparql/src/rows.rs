//! What a selected query element reads as, and its property rows (etalii.adp spec 030, task T084).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/sparql/backend/EtAlii.Adp.Diagram.Sparql/
//! SparqlSelection.cs` (the element-id vocabulary, taken apart in one place), `SparqlContextPropertyProvider.cs`
//! (every row read-only, every row carrying the one reason, naming the text editor) and
//! `SparqlContextActionProvider.cs` (nothing offered, every action refused).

use adp_context::{PropertyEditor, PropertyRow};

use crate::mapper::{HEADER_ID, TRUNCATION_ID};
use crate::model::SparqlQueryModel;
use crate::projection::{self, SparqlNode, SparqlNodeKind, SparqlProjectionResult};

pub const VARIABLE_NAME_PROPERTY: &str = "sparql.variable.name";
pub const VARIABLE_PROJECTED_PROPERTY: &str = "sparql.variable.projected";
pub const VARIABLE_JOIN_COUNT_PROPERTY: &str = "sparql.variable.joins";
pub const VARIABLE_DEFINITION_PROPERTY: &str = "sparql.variable.definition";
pub const TERM_DISPLAY_PROPERTY: &str = "sparql.term.display";
pub const TERM_FULL_PROPERTY: &str = "sparql.term.full";
pub const TERM_ANNOTATION_PROPERTY: &str = "sparql.term.annotation";
pub const REGION_KIND_PROPERTY: &str = "sparql.region.kind";
pub const REGION_LABEL_PROPERTY: &str = "sparql.region.label";
pub const EDGE_LABEL_PROPERTY: &str = "sparql.edge.label";
pub const ANNOTATION_TEXT_PROPERTY: &str = "sparql.annotation.text";
pub const QUERY_FORM_PROPERTY: &str = "sparql.query.form";
pub const QUERY_MODIFIERS_PROPERTY: &str = "sparql.query.modifiers";
pub const SUB_QUERY_TEXT_PROPERTY: &str = "sparql.subquery.text";

const IDENTITY_GROUP: &str = "Identity";
const STRUCTURE_GROUP: &str = "Structure";
const QUERY_GROUP: &str = "Query";

/// The reason every row carries, and what a write is answered with.
pub const READ_ONLY_REASON: &str =
    "This diagram reads the query; edit the .rq file in a text editor and the diagram follows.";

/// What an action arriving anyway is told.
pub const NO_ACTIONS_REASON: &str =
    "A query diagram offers no edits: it reads the .rq file, which is edited in a text editor.";

/// Every row is built here, so none can be created without its reason.
fn row(id: &str, label: &str, value: &str, group: &str) -> PropertyRow {
    let mut row = PropertyRow::new(id, label, PropertyEditor::Line, value);
    row.read_only_reason = READ_ONLY_REASON.to_owned();
    row.group = group.to_owned();
    row
}

fn rows_for(node: &SparqlNode) -> Vec<PropertyRow> {
    match node.kind {
        SparqlNodeKind::Variable | SparqlNodeKind::Anonymous => vec![
            row(
                VARIABLE_NAME_PROPERTY,
                if node.kind == SparqlNodeKind::Anonymous {
                    "Anonymous variable"
                } else {
                    "Variable"
                },
                &node.display,
                IDENTITY_GROUP,
            ),
            row(
                VARIABLE_PROJECTED_PROPERTY,
                "Projected",
                if node.projected { "Yes" } else { "No" },
                IDENTITY_GROUP,
            ),
            // The join count is the fact the whole diagram is built around, said in words.
            row(
                VARIABLE_JOIN_COUNT_PROPERTY,
                "Joins",
                &node.join_count.to_string(),
                STRUCTURE_GROUP,
            ),
            row(
                VARIABLE_DEFINITION_PROPERTY,
                "Defined by",
                &node.defining_expression,
                STRUCTURE_GROUP,
            ),
        ],
        SparqlNodeKind::SubSelect => vec![
            row(TERM_DISPLAY_PROPERTY, "Projection", &node.display, IDENTITY_GROUP),
            row(SUB_QUERY_TEXT_PROPERTY, "Subquery", &node.full, QUERY_GROUP),
        ],
        SparqlNodeKind::Iri | SparqlNodeKind::Literal => {
            let iri = node.kind == SparqlNodeKind::Iri;
            vec![
                row(
                    TERM_DISPLAY_PROPERTY,
                    if iri { "Prefixed name" } else { "Literal" },
                    &node.display,
                    IDENTITY_GROUP,
                ),
                row(
                    TERM_FULL_PROPERTY,
                    if iri { "IRI" } else { "As written" },
                    &node.full,
                    IDENTITY_GROUP,
                ),
                row(TERM_ANNOTATION_PROPERTY, "Type", &node.annotation, IDENTITY_GROUP),
            ]
        }
    }
}

/// The rows of `element_id` in the projection of `model`; none for an id it does not have.
pub fn rows(projection: &SparqlProjectionResult, element_id: &str) -> Vec<PropertyRow> {
    if element_id == HEADER_ID {
        return vec![
            row(QUERY_FORM_PROPERTY, "Form", &projection.header_form, QUERY_GROUP),
            row(
                QUERY_MODIFIERS_PROPERTY,
                "Modifiers",
                &projection.header_rows.join(" · "),
                QUERY_GROUP,
            ),
        ];
    }
    if element_id.is_empty() {
        return Vec::new();
    }
    if let Some(node) = projection.nodes.iter().find(|n| n.id == element_id) {
        return rows_for(node);
    }
    if let Some(region) = projection.regions.iter().find(|r| r.id == element_id) {
        return vec![
            row(
                REGION_KIND_PROPERTY,
                "Group",
                &region.kind.to_uppercase(),
                STRUCTURE_GROUP,
            ),
            row(REGION_LABEL_PROPERTY, "Constraint", &region.label, STRUCTURE_GROUP),
        ];
    }
    if let Some(edge) = projection.edges.iter().find(|e| e.id == element_id) {
        return vec![row(
            EDGE_LABEL_PROPERTY,
            if edge.is_path { "Property path" } else { "Predicate" },
            &edge.label,
            STRUCTURE_GROUP,
        )];
    }
    if let Some(annotation) = projection.annotations.iter().find(|a| a.id == element_id) {
        return vec![row(
            ANNOTATION_TEXT_PROPERTY,
            &annotation.kind.to_uppercase(),
            &annotation.text,
            STRUCTURE_GROUP,
        )];
    }
    Vec::new()
}

/// How `element_id` reads in a selection path, or none when the id is not in the query; `model` is none for a query
/// that does not parse, whose header still reads as a plain "Query".
pub fn describe(model: Option<&SparqlQueryModel>, element_id: &str) -> Option<String> {
    if element_id == HEADER_ID {
        return Some(match model {
            Some(model) => format!("{} query", projection::project(model).header_form),
            None => "Query".to_owned(),
        });
    }
    let projection = projection::project(model?);
    if element_id.is_empty() {
        return None;
    }
    if let Some(node) = projection.nodes.iter().find(|n| n.id == element_id) {
        return Some(match node.kind {
            SparqlNodeKind::SubSelect => "Subquery".to_owned(),
            SparqlNodeKind::Anonymous => "Anonymous variable".to_owned(),
            _ => node.display.clone(),
        });
    }
    if let Some(region) = projection.regions.iter().find(|r| r.id == element_id) {
        return Some(if region.label.is_empty() {
            format!("{} group", region.kind)
        } else {
            region.label.clone()
        });
    }
    if let Some(edge) = projection.edges.iter().find(|e| e.id == element_id) {
        return Some(edge.label.clone());
    }
    if let Some(annotation) = projection.annotations.iter().find(|a| a.id == element_id) {
        return Some(annotation.text.clone());
    }
    if element_id == TRUNCATION_ID && projection.truncated() {
        return Some(format!("Showing {} of {}", projection.shown, projection.total));
    }
    None
}
