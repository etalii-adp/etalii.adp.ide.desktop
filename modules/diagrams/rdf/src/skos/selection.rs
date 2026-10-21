//! What the family's element ids mean under the scheme reading (etalii.adp spec 030, task T083), ported from the
//! standalone host's `Skos/SkosSelection.cs` at `8b6ccfc`: answered from the file's own assertions, so the one family
//! provider serves every reading without knowing which registration a selection came through.

use crate::model::{RdfModel, RdfTriple};
use crate::selection::resource_of;
use crate::skos::projection::SkosEdgeKind;
use crate::vocabulary::{rdf, skos};

fn is_typed(model: &RdfModel, iri: &str, type_iri: &str) -> bool {
    model
        .triples
        .iter()
        .any(|t| t.subject.is_iri(iri) && t.predicate.iri == rdf::TYPE && t.object.is_iri(type_iri))
}

/// The IRI of an element the file asserts to be a `skos:Concept` (`ConceptOf`).
pub fn concept_of(model: &RdfModel, element_id: &str) -> Option<String> {
    resource_of(model, element_id).filter(|iri| is_typed(model, iri, skos::CONCEPT))
}

/// The IRI of an element the file asserts to be a scheme or collection (`SchemeOrCollectionOf`).
pub fn scheme_or_collection_of(model: &RdfModel, element_id: &str) -> Option<String> {
    resource_of(model, element_id).filter(|iri| {
        is_typed(model, iri, skos::CONCEPT_SCHEME)
            || is_typed(model, iri, skos::COLLECTION)
            || is_typed(model, iri, skos::ORDERED_COLLECTION)
    })
}

fn is_scheme_declaration(t: &RdfTriple) -> bool {
    t.predicate.iri == rdf::TYPE && t.object.is_iri(skos::CONCEPT_SCHEME)
}

/// Whether the file asserts any concept scheme (`HasScheme`).
pub fn has_scheme(model: &RdfModel) -> bool {
    model.triples.iter().any(is_scheme_declaration)
}

/// The first asserted scheme IRI, in document order (`FirstScheme`).
pub fn first_scheme(model: &RdfModel) -> Option<String> {
    model
        .triples
        .iter()
        .filter(|t| is_scheme_declaration(t))
        .find_map(|t| t.subject.as_iri().map(str::to_owned))
}

/// A hierarchy or related pair a skos edge id names (`PairOf`).
#[derive(Debug, Clone, PartialEq)]
pub struct SkosPair {
    pub a_iri: String,
    pub b_iri: String,
    pub kind: SkosEdgeKind,
    pub triples: Vec<RdfTriple>,
}

fn matches(t: &RdfTriple, subject: &str, predicate: &str, object: &str) -> bool {
    t.subject.is_iri(subject) && t.predicate.iri == predicate && t.object.is_iri(object)
}

/// The hierarchy or related edge a skos edge id names, when the drawn pair still has asserted triples, resolved by
/// values in either direction (`PairOf`).
pub fn pair_of(model: &RdfModel, element_id: &str) -> Option<SkosPair> {
    let rest = element_id.strip_prefix("edge:")?;
    let parts: Vec<&str> = rest.split('|').collect();
    let [a, predicate, b] = parts.as_slice() else {
        return None;
    };
    let a = a.strip_prefix("res:")?;
    let b = b.strip_prefix("res:")?;
    let kind = match *predicate {
        skos::BROADER => SkosEdgeKind::Hierarchy,
        skos::RELATED => SkosEdgeKind::Related,
        _ => return None,
    };
    let triples: Vec<RdfTriple> = model
        .triples
        .iter()
        .filter(|t| match kind {
            SkosEdgeKind::Hierarchy => matches(t, a, skos::BROADER, b) || matches(t, b, skos::NARROWER, a),
            _ => matches(t, a, skos::RELATED, b) || matches(t, b, skos::RELATED, a),
        })
        .cloned()
        .collect();
    (!triples.is_empty()).then(|| SkosPair {
        a_iri: a.to_owned(),
        b_iri: b.to_owned(),
        kind,
        triples,
    })
}

/// Whether the resource's labels are stated through SKOS-XL (`HasXlLabels`).
pub fn has_xl_labels(model: &RdfModel, iri: &str) -> bool {
    model
        .triples
        .iter()
        .any(|t| t.subject.is_iri(iri) && skos::XL_LABEL_PROPERTIES.contains(&t.predicate.iri.as_str()))
}
