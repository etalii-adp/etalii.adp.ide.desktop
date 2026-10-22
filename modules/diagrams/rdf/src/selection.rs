//! What one of the family's element ids names inside a document (etalii.adp spec 030, task T083): the shared
//! vocabulary the description, the menus and the property rows all answer from, so they never disagree about what is
//! selected.
//!
//! Ported from the standalone host's `RdfSelection.cs` and `RdfGestures.cs` at `8b6ccfc`. The shapes: `res:{iri}` for
//! an IRI-named resource, `blank:{ordinal}` for a blank node, and `edge:{fromId}|{predicateIri}|{toId}` for a drawn
//! triple; the readings add `ind:`, `expr:`, `thing:`, `dt:` and `shacl-edge:` ids.

use crate::model::{RdfModel, RdfTriple};
use crate::projection::{self, display};

/// The sentence every editing surface answers with under truncation.
pub const TRUNCATED_REFUSAL: &str = "The diagram shows only the first part of this file under the drawn-element budget, so edits through it are withheld - an edit through a partial view could touch what the view does not show. Edit the file as text instead.";

/// The truncation banner's element id.
pub const TRUNCATION_ID: &str = "truncation";

/// What moving an edge or the banner answers.
pub const NOT_MOVABLE: &str = "That element is not something this diagram can move.";

/// What moving a blank node answers.
pub const BLANK_NOT_MOVABLE: &str = "That is a blank node, whose identity does not survive a reparse, so a stored position could not be trusted. Name it with an IRI to arrange it.";

/// The IRI a `res:` id names, when the model states it.
pub fn resource_of(model: &RdfModel, element_id: &str) -> Option<String> {
    let iri = element_id.strip_prefix("res:")?;
    model.names(iri).then(|| iri.to_owned())
}

/// Whether the id names a blank node.
pub fn is_blank(element_id: &str) -> bool {
    element_id.starts_with("blank:")
}

/// The drawn triple an `edge:` id between two IRI-named resources names, when the model still states it.
pub fn edge_of<'a>(model: &'a RdfModel, element_id: &str) -> Option<&'a RdfTriple> {
    let rest = element_id.strip_prefix("edge:")?;
    let parts: Vec<&str> = rest.split('|').collect();
    if parts.len() < 3 {
        return None;
    }
    let from = parts[0].strip_prefix("res:")?;
    let to = parts[2].strip_prefix("res:")?;
    let predicate = parts[1];
    model
        .triples
        .iter()
        .find(|t| t.subject.is_iri(from) && t.predicate.iri == predicate && t.object.is_iri(to))
}

/// Every triple the IRI is subject or object of.
pub fn touching<'a>(model: &'a RdfModel, iri: &str) -> Vec<&'a RdfTriple> {
    model
        .triples
        .iter()
        .filter(|t| t.subject.is_iri(iri) || t.object.is_iri(iri))
        .collect()
}

/// Whether the drawn view is budget-cut, in which case every edit is withheld.
pub fn is_truncated(model: &RdfModel) -> bool {
    projection::project(model, projection::DEFAULT_BUDGET).truncated()
}

/// The text an id resolves to, or none when the model holds nothing by that id (`RdfSelection.Describe`).
pub fn describe(model: &RdfModel, element_id: &str) -> Option<String> {
    if let Some(iri) = resource_of(model, element_id) {
        return Some(display(model, &iri));
    }
    if let Some(ordinal) = element_id.strip_prefix("blank:").and_then(|o| o.parse::<usize>().ok()) {
        return model
            .triples
            .iter()
            .filter_map(|t| t.subject.as_blank())
            .find(|b| b.ordinal == ordinal)
            .map(projection::blank_display);
    }
    if let Some(edge) = edge_of(model, element_id) {
        let from = display(model, edge.subject.as_iri().unwrap_or_default());
        let to = display(model, edge.object.as_iri().unwrap_or_default());
        return Some(format!("{from} → {to}"));
    }
    if element_id == TRUNCATION_ID && is_truncated(model) {
        return Some("Truncated view".to_owned());
    }
    None
}

/// The id of a placement gesture at a canvas point (`RdfNewPlacement.IdFor`).
pub fn placement_id(x: f64, y: f64) -> String {
    format!("new:{x},{y}")
}

/// The point a placement gesture `new:{x},{y}` names: exactly two numbers after the prefix (`GestureIds.TryParsePlacement`).
pub fn placement_of(element_id: &str) -> Option<(f64, f64)> {
    let rest = element_id.strip_prefix("new:")?;
    let parts: Vec<&str> = rest.split(',').collect();
    let [x, y] = parts.as_slice() else {
        return None;
    };
    Some((number(x)?, number(y)?))
}

/// A number as .NET's `double.TryParse` with `NumberStyles.Float` reads it, in the invariant culture.
fn number(text: &str) -> Option<f64> {
    let trimmed = text.trim();
    let lower = trimmed.to_ascii_lowercase();
    if trimmed.is_empty() || lower.contains("inf") || lower.contains("nan") {
        return None;
    }
    trimmed.parse().ok()
}

/// Whether `element_id` is a placement gesture (`RdfNewPlacement.IsPlacement`).
pub fn is_placement(element_id: &str) -> bool {
    placement_of(element_id).is_some()
}

/// The ends of a relation gesture `rel:{from}->{to}`, neither empty (`GestureIds.TryParseRelation`).
pub fn relation_of(element_id: &str) -> Option<(String, String)> {
    let rest = element_id.strip_prefix("rel:")?;
    let (from, to) = rest.split_once("->")?;
    (!from.is_empty() && !to.is_empty()).then(|| (from.to_owned(), to.to_owned()))
}
