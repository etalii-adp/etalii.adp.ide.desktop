//! The property rows of a selected element of the family (etalii.adp spec 030, task T083), contributed as data the
//! panel renders without understanding.
//!
//! Ported from the standalone host's `RdfContextPropertyProvider.cs` at `8b6ccfc`. An element the file asserts to be
//! a concept, scheme or collection gets the scheme reading's grid; a shape on a shapes diagram the shapes reading's;
//! every other resource the family's rows - `rdfs:label` and `rdfs:comment` editable, a first write stating the triple
//! and a later one rewriting its literal in place - with the ontology's axioms beneath where it has any.

use adp_context::{PropertyEditor, PropertyRow};

use crate::commands::{ObjectValue, RdfEdit};
use crate::family::Target;
use crate::model::{RdfModel, RdfTerm};
use crate::projection::display;
use crate::store::RdfDocument;
use crate::vocabulary::{owl, rdf};
use crate::{owl as owl_reading, selection, shacl, skos};

/// The resource's full IRI, read-only: renaming rewrites references, so the menu owns it.
pub const IRI: &str = "rdf.iri";
/// The resource's types, read-only: types are stated by triples, edited as triples.
pub const TYPES: &str = "rdf.types";
/// The resource's `rdfs:label`, editable.
pub const LABEL: &str = "rdf.label";
/// The resource's `rdfs:comment`, editable.
pub const COMMENT: &str = "rdf.comment";
/// An edge's predicate, read-only.
const PREDICATE: &str = "rdf.predicate";

const IDENTITY_GROUP: &str = "Identity";
const DOCUMENTATION_GROUP: &str = "Documentation";
const RENAME_VIA_MENU: &str = "Rename through the context menu, so every reference follows the name.";
const TYPES_ARE_TRIPLES: &str = "Types are stated by triples; add or remove them as statements.";
const BLANK_BOUNDARY: &str =
    "A blank node's identity does not survive a reparse, so nothing about it can be edited from the diagram.";
const AXIOM_GROUP: &str = "Axioms";
const AXIOMS_ARE_TRIPLES: &str = "Axioms are stated by triples; draw or remove them as edges and statements.";

fn row(
    id: impl Into<String>,
    label: &str,
    value: impl Into<String>,
    read_only_reason: &str,
    group: &str,
) -> PropertyRow {
    let mut row = PropertyRow::new(id, label, PropertyEditor::Line, value);
    row.read_only_reason = read_only_reason.to_owned();
    row.group = group.to_owned();
    row
}

fn named(model: &RdfModel, element_id: &str) -> Option<String> {
    selection::resource_of(model, element_id).or_else(|| owl_reading::selection::individual_iri_of(model, element_id))
}

/// The rows of `target` (`DescribeAsync`).
pub fn describe(document: &RdfDocument, target: &Target) -> Vec<PropertyRow> {
    if let Some(rows) = skos::properties::describe(document, target) {
        return rows;
    }
    if let Some(rows) = shacl::properties::describe(document, target) {
        return rows;
    }
    let model = &document.model;
    let element_id = target.element_id;
    if let Some(iri) = named(model, element_id) {
        let truncated = selection::is_truncated(model);
        let editable = if truncated { selection::TRUNCATED_REFUSAL } else { "" };
        let types: Vec<String> = model
            .triples
            .iter()
            .filter(|t| t.subject.is_iri(&iri) && t.predicate.iri == rdf::TYPE)
            .filter_map(|t| t.object.as_iri())
            .map(|o| display(model, o))
            .collect();
        let mut rows = vec![
            row(IRI, "IRI", iri.clone(), RENAME_VIA_MENU, IDENTITY_GROUP),
            row(TYPES, "Types", types.join(", "), TYPES_ARE_TRIPLES, IDENTITY_GROUP),
            row(
                LABEL,
                "Label",
                literal_of(model, &iri, rdf::LABEL),
                editable,
                DOCUMENTATION_GROUP,
            ),
            row(
                COMMENT,
                "Comment",
                literal_of(model, &iri, rdf::COMMENT),
                editable,
                DOCUMENTATION_GROUP,
            ),
        ];
        rows.extend(owl_rows(model, &iri));
        return rows;
    }
    if let Some(text) = owl_reading::selection::expression_text(model, element_id) {
        return vec![row(
            "owl.expression",
            "Expression",
            text,
            owl_reading::selection::EXPRESSION_REFUSAL,
            IDENTITY_GROUP,
        )];
    }
    if let Some(property) = owl_reading::selection::property_edge_iri_of(model, element_id) {
        let mut rows = vec![row(IRI, "IRI", property.clone(), RENAME_VIA_MENU, IDENTITY_GROUP)];
        rows.extend(owl_rows(model, &property));
        return rows;
    }
    if let Some(edge) = selection::edge_of(model, element_id) {
        return vec![row(
            PREDICATE,
            "Predicate",
            display(model, &edge.predicate.iri),
            "A statement's predicate is the statement; remove and restate to change it.",
            IDENTITY_GROUP,
        )];
    }
    if selection::is_blank(element_id)
        && let Some(text) = selection::describe(model, element_id)
    {
        return vec![row(IRI, "Node", text, BLANK_BOUNDARY, IDENTITY_GROUP)];
    }
    Vec::new()
}

/// The edit setting `property_id` of `target` to `value` makes, or why none (`SetAsync`).
pub fn command_for(document: &RdfDocument, target: &Target, property_id: &str, value: &str) -> Result<RdfEdit, String> {
    let model = &document.model;
    if selection::is_truncated(model) {
        return Err(selection::TRUNCATED_REFUSAL.to_owned());
    }
    edit_for(document, target, property_id, value)
        .ok_or_else(|| format!("'{property_id}' cannot be edited on this selection."))
}

fn edit_for(document: &RdfDocument, target: &Target, property_id: &str, value: &str) -> Option<RdfEdit> {
    if let Some(edit) = skos::properties::command_for(document, target, property_id, value) {
        return Some(edit);
    }
    if let Some(edit) = shacl::properties::command_for(document, target, property_id, value) {
        return Some(edit);
    }
    let predicate = match property_id {
        LABEL => rdf::LABEL,
        COMMENT => rdf::COMMENT,
        _ => return None,
    };
    let model = &document.model;
    let iri = named(model, target.element_id)?;
    // A first write states the triple; a later one rewrites exactly the literal token.
    let existing = model
        .triples
        .iter()
        .find(|t| t.subject.is_iri(&iri) && t.predicate.iri == predicate && matches!(t.object, RdfTerm::Literal(_)));
    let Some(literal) = existing.and_then(|t| t.object.as_literal()) else {
        return Some(RdfEdit::AddTriple {
            subject: iri,
            predicate: predicate.to_owned(),
            object: ObjectValue::literal(value, "", ""),
        });
    };
    let language = literal.language.clone().unwrap_or_default();
    let datatype = literal.datatype.clone().unwrap_or_default();
    Some(RdfEdit::ReplaceLiteral {
        subject: iri,
        predicate: predicate.to_owned(),
        old: ObjectValue::literal(literal.lexical.clone(), language.clone(), datatype.clone()),
        new_lexical: value.to_owned(),
        new_language: language,
        new_datatype: datatype,
    })
}

/// The ontology rows a named term carries beyond the family's: asserted superclasses, equivalents and disjoints, a
/// property's domain, range and inverse - expressions rendered uncapped - and its characteristics; none for a term
/// with no OWL axioms (`OwlRowsFor`).
fn owl_rows(model: &RdfModel, iri: &str) -> Vec<PropertyRow> {
    let mut rows = Vec::new();
    let mut index = 0;
    for triple in &model.triples {
        if !triple.subject.is_iri(iri) {
            continue;
        }
        let name = match triple.predicate.iri.as_str() {
            owl::SUB_CLASS_OF => "Subclass of",
            owl::EQUIVALENT_CLASS => "Equivalent to",
            owl::DISJOINT_WITH => "Disjoint with",
            owl::DOMAIN => "Domain",
            owl::RANGE => "Range",
            owl::INVERSE_OF => "Inverse of",
            _ => continue,
        };
        let (value, reason) = match &triple.object {
            RdfTerm::Iri(named) => (display(model, &named.iri), AXIOMS_ARE_TRIPLES),
            RdfTerm::Blank(root) => (
                owl_reading::selection::render_expression(model, root),
                owl_reading::selection::EXPRESSION_REFUSAL,
            ),
            RdfTerm::Literal(_) => (String::new(), AXIOMS_ARE_TRIPLES),
        };
        rows.push(row(format!("owl.axiom.{index}"), name, value, reason, AXIOM_GROUP));
        index += 1;
    }
    for triple in &model.triples {
        if !triple.subject.is_iri(iri) || triple.predicate.iri != rdf::TYPE {
            continue;
        }
        let word = match triple.object.as_iri() {
            Some(owl::FUNCTIONAL_PROPERTY) => "functional",
            Some(owl::INVERSE_FUNCTIONAL_PROPERTY) => "inverse functional",
            Some(owl::TRANSITIVE_PROPERTY) => "transitive",
            Some(owl::SYMMETRIC_PROPERTY) => "symmetric",
            Some(owl::ASYMMETRIC_PROPERTY) => "asymmetric",
            Some(owl::REFLEXIVE_PROPERTY) => "reflexive",
            Some(owl::IRREFLEXIVE_PROPERTY) => "irreflexive",
            _ => continue,
        };
        rows.push(row(
            format!("owl.characteristic.{index}"),
            "Characteristic",
            word,
            AXIOMS_ARE_TRIPLES,
            AXIOM_GROUP,
        ));
        index += 1;
    }
    rows
}

/// The lexical form of the first literal `iri` states for `predicate`, empty when none.
fn literal_of(model: &RdfModel, iri: &str, predicate: &str) -> String {
    model
        .triples
        .iter()
        .filter(|t| t.subject.is_iri(iri) && t.predicate.iri == predicate)
        .find_map(|t| t.object.as_literal())
        .map(|l| l.lexical.clone())
        .unwrap_or_default()
}
