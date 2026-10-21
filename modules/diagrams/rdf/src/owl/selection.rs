//! The ontology reading's additions to the family's selection vocabulary (etalii.adp spec 030, task T083), ported from
//! the standalone host's `Owl/OwlSelection.cs` at `8b6ccfc`: `ind:{iri}` for a punned individual role, `expr:`
//! structural ids, and the derived edges - a property edge between a domain and a range is no one triple, so the
//! family's selection alone cannot name it.

use crate::model::{BlankTerm, RdfModel, RdfTerm};
use crate::owl::expression;
use crate::owl::projection::{self, OwlNode};
use crate::projection::DEFAULT_BUDGET;
use crate::vocabulary::{owl, rdf};

/// The blank-node identity boundary's sentence, answered wherever an expression refuses an edit.
pub const EXPRESSION_REFUSAL: &str = "That is an anonymous class expression, whose identity does not survive an edit to the file, so nothing about it can be edited from the diagram. Edit the expression as text.";

/// The duplicate-subclass sentence, answered before any splice.
pub const DUPLICATE_SUBCLASS_REFUSAL: &str =
    "That subclass relation is already asserted; drawing it again would state nothing new.";

/// The IRI an `ind:` id names, when the model states it as a subject (`IndividualIriOf`).
pub fn individual_iri_of(model: &RdfModel, element_id: &str) -> Option<String> {
    let iri = element_id.strip_prefix("ind:")?;
    model
        .triples
        .iter()
        .any(|t| t.subject.is_iri(iri))
        .then(|| iri.to_owned())
}

fn is_expression(element_id: &str) -> bool {
    element_id.starts_with("expr:")
}

/// The text a drawn ontology node or edge resolves to, the projection being the authority because a derived edge
/// exists only there (`OwlSelection.Describe`).
pub fn describe(model: &RdfModel, element_id: &str) -> Option<String> {
    let owl_shaped = is_expression(element_id)
        || ["ind:", "thing:", "dt:", "edge:"]
            .iter()
            .any(|prefix| element_id.starts_with(prefix));
    if !owl_shaped {
        return None;
    }
    let graph = projection::project(model, DEFAULT_BUDGET);
    if let Some(node) = graph.nodes.iter().find(|n| n.id == element_id) {
        return Some(node.display.clone());
    }
    let edge = graph.edges.iter().find(|e| e.id == element_id)?;
    let display_of = |id: &str| {
        graph
            .nodes
            .iter()
            .find(|n| n.id == id)
            .map(|n| n.display.clone())
            .unwrap_or_default()
    };
    Some(format!("{} → {}", display_of(&edge.from_id), display_of(&edge.to_id)))
}

/// The expression node an `expr:` id names (`OwlSelection.ExpressionOf`).
pub fn expression_of(model: &RdfModel, element_id: &str) -> Option<OwlNode> {
    if !is_expression(element_id) {
        return None;
    }
    projection::project(model, DEFAULT_BUDGET)
        .nodes
        .into_iter()
        .find(|n| n.id == element_id && n.expression_root.is_some())
}

/// The full, uncapped Manchester rendering of the expression an `expr:` id names.
pub fn expression_text(model: &RdfModel, element_id: &str) -> Option<String> {
    let root = expression_of(model, element_id)?.expression_root?;
    Some(render_expression(model, &root))
}

/// The uncapped Manchester rendering of the expression rooted at the blank node `root` (`ExpressionRenderer.Render`).
pub fn render_expression(model: &RdfModel, root: &BlankTerm) -> String {
    expression::render(&RdfTerm::Blank(root.clone()), model, None).text
}

/// The property IRI a derived property edge names: `edge:{from}|{propertyIri}|{to}` where the middle part is a
/// declared object or datatype property (`PropertyEdgeIriOf`).
pub fn property_edge_iri_of(model: &RdfModel, element_id: &str) -> Option<String> {
    let rest = element_id.strip_prefix("edge:")?;
    let parts: Vec<&str> = rest.split('|').collect();
    if parts.len() < 3 {
        return None;
    }
    let candidate = parts[1];
    let declared = model.triples.iter().any(|t| {
        t.subject.is_iri(candidate)
            && t.predicate.iri == rdf::TYPE
            && (t.object.is_iri(owl::OBJECT_PROPERTY) || t.object.is_iri(owl::DATATYPE_PROPERTY))
    });
    declared.then(|| candidate.to_owned())
}

/// Whether `iri` is used as a class in `model`: what the subclass gesture asks of both ends (`IsClass`).
pub fn is_class(model: &RdfModel, iri: &str) -> bool {
    model.triples.iter().any(|t| {
        (t.subject.is_iri(iri)
            && ((t.predicate.iri == rdf::TYPE && (t.object.is_iri(owl::CLASS) || t.object.is_iri(owl::RDFS_CLASS)))
                || t.predicate.iri == owl::SUB_CLASS_OF))
            || (t.object.is_iri(iri)
                && matches!(t.predicate.iri.as_str(), owl::SUB_CLASS_OF | owl::DOMAIN | owl::RANGE))
    })
}

/// Whether the document carries the ontology marker (`IsOntologyDocument`).
pub fn is_ontology_document(model: &RdfModel) -> bool {
    model
        .triples
        .iter()
        .any(|t| t.predicate.iri == rdf::TYPE && t.object.is_iri(owl::ONTOLOGY))
}
