//! The scheme reading's projection (etalii.adp spec 030, task T083): what a model states as a vocabulary, by assertion
//! only, ported from the standalone host's `Skos/SkosProjection.cs` and `Skos/_Model` at `8b6ccfc`.
//!
//! A concept is whatever the file types as `skos:Concept`; membership is `inScheme`, `topConceptOf` and
//! `hasTopConcept` as written, and a concept asserted into no scheme goes to the unfiled band. One hierarchy edge
//! exists per pair wherever `broader` or `narrower` is asserted in either direction; nothing is completed. The budget
//! cut is hierarchy-aware: schemes by IRI, each scheme's tops and their descendants breadth-first by layer, a scheme's
//! unreached members, then unfiled concepts, then collections.

use std::collections::{HashMap, HashSet};

use indexmap::IndexMap;

use crate::model::{RdfModel, RdfTerm, RdfTriple};
use crate::projection::cons_cells;
use crate::skos::labels::{SkosLabel, SkosLabelSource};
use crate::vocabulary::{rdf, skos};

/// The three edge languages of the scheme reading (`SkosEdgeKind`); the payload carries it as its number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SkosEdgeKind {
    Hierarchy = 0,
    Related = 1,
    Mapping = 2,
}

/// One concept scheme (`SkosScheme`).
#[derive(Debug, Clone, PartialEq)]
pub struct SkosScheme {
    pub id: String,
    pub iri: String,
    pub labels: Vec<SkosLabel>,
    pub top_concept_ids: Vec<String>,
}

/// One drawn concept (`SkosConcept`).
#[derive(Debug, Clone, PartialEq)]
pub struct SkosConcept {
    pub id: String,
    pub iri: String,
    pub labels: Vec<SkosLabel>,
    pub notations: Vec<String>,
    pub scheme_iris: Vec<String>,
    pub blank: bool,
}

/// One collection (`SkosCollection`).
#[derive(Debug, Clone, PartialEq)]
pub struct SkosCollection {
    pub id: String,
    pub iri: String,
    pub labels: Vec<SkosLabel>,
    pub ordered: bool,
    pub member_ids: Vec<String>,
}

/// One drawn edge (`SkosEdge`), with every asserted triple it projects.
#[derive(Debug, Clone, PartialEq)]
pub struct SkosEdge {
    pub id: String,
    pub from_id: String,
    pub to_id: String,
    pub kind: SkosEdgeKind,
    pub predicate_iri: String,
    pub triples: Vec<RdfTriple>,
    pub asserted_both_ways: bool,
}

/// What the scheme reading draws (`SkosProjectionResult`).
#[derive(Debug, Clone, PartialEq)]
pub struct SkosProjectionResult {
    pub schemes: Vec<SkosScheme>,
    pub concepts: Vec<SkosConcept>,
    pub collections: Vec<SkosCollection>,
    pub edges: Vec<SkosEdge>,
    pub out_of_file_mappings: Vec<RdfTriple>,
    pub shown: usize,
    pub total: usize,
}

impl SkosProjectionResult {
    /// Whether the budget cut the vocabulary down.
    pub fn truncated(&self) -> bool {
        self.shown < self.total
    }
}

/// The element id a term draws under, or none for a literal (`SkosProjection.ElementId`).
pub fn element_id(term: &RdfTerm) -> Option<String> {
    match term {
        RdfTerm::Iri(iri) => Some(format!("res:{}", iri.iri)),
        RdfTerm::Blank(blank) => Some(format!("blank:{}", blank.ordinal)),
        RdfTerm::Literal(_) => None,
    }
}

fn iri_of(id: &str) -> String {
    id.strip_prefix("res:").unwrap_or_default().to_owned()
}

/// The ids of every subject the file types as `type_iri` (`SkosProjection.TypedIds`).
pub fn typed_ids(model: &RdfModel, type_iri: &str) -> HashSet<String> {
    model
        .triples
        .iter()
        .filter(|t| t.predicate.iri == rdf::TYPE && t.object.is_iri(type_iri))
        .filter_map(|t| element_id(&t.subject))
        .collect()
}

/// Whether the model asserts any SKOS-XL label triple (`SkosProjection.HasXlLabels`).
pub fn has_xl_labels(model: &RdfModel) -> bool {
    model
        .triples
        .iter()
        .any(|t| skos::XL_LABEL_PROPERTIES.contains(&t.predicate.iri.as_str()))
}

fn add<V>(map: &mut HashMap<String, Vec<V>>, key: &str, value: V) {
    map.entry(key.to_owned()).or_default().push(value);
}

type Pairs = IndexMap<(String, String), (Vec<RdfTriple>, bool)>;

fn pair(map: &mut Pairs, key: (String, String), triple: &RdfTriple) {
    match map.get_mut(&key) {
        Some((triples, both)) => {
            triples.push(triple.clone());
            let predicates: HashSet<&str> = triples.iter().map(|t| t.predicate.iri.as_str()).collect();
            let subjects: HashSet<Option<String>> = triples.iter().map(|t| element_id(&t.subject)).collect();
            *both = *both || predicates.len() > 1 || subjects.len() > 1;
        }
        None => {
            map.insert(key, (vec![triple.clone()], false));
        }
    }
}

fn distinct(items: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    items.iter().filter(|i| seen.insert(i.as_str())).cloned().collect()
}

fn sorted<'a>(items: impl IntoIterator<Item = &'a String>) -> Vec<String> {
    let mut items: Vec<String> = items.into_iter().cloned().collect();
    items.sort();
    items
}

/// What `model` states, cut to `budget` drawn elements (`SkosProjection.Project`).
pub fn project(model: &RdfModel, budget: usize) -> SkosProjectionResult {
    let concept_ids = typed_ids(model, skos::CONCEPT);
    let scheme_ids = typed_ids(model, skos::CONCEPT_SCHEME);
    let ordered_collection_ids = typed_ids(model, skos::ORDERED_COLLECTION);
    let collection_ids: HashSet<String> = typed_ids(model, skos::COLLECTION)
        .into_iter()
        .chain(ordered_collection_ids.iter().cloned())
        .collect();

    let mut labels: HashMap<String, Vec<SkosLabel>> = HashMap::new();
    let mut notations: HashMap<String, Vec<String>> = HashMap::new();
    let mut memberships: HashMap<String, Vec<String>> = HashMap::new();
    let mut tops_by_scheme: HashMap<String, Vec<String>> = HashMap::new();
    let mut hierarchy_pairs: Pairs = IndexMap::new();
    let mut related_pairs: Pairs = IndexMap::new();
    let mut mapping_edges: Vec<SkosEdge> = Vec::new();
    let mut out_of_file_mappings: Vec<RdfTriple> = Vec::new();
    let mut members_by_collection: HashMap<String, Vec<String>> = HashMap::new();
    let (firsts, rests) = cons_cells(model);

    for triple in &model.triples {
        let subject_id = element_id(&triple.subject);
        let object_id = element_id(&triple.object);
        let predicate = triple.predicate.iri.as_str();
        match (predicate, &subject_id, &object_id) {
            (skos::PREF_LABEL | skos::ALT_LABEL | skos::HIDDEN_LABEL, Some(subject), _)
                if triple.object.as_literal().is_some() =>
            {
                let literal = triple.object.as_literal().expect("checked above");
                let source = match predicate {
                    skos::PREF_LABEL => SkosLabelSource::Preferred,
                    skos::ALT_LABEL => SkosLabelSource::Alternate,
                    _ => SkosLabelSource::Hidden,
                };
                add(
                    &mut labels,
                    subject,
                    SkosLabel {
                        source,
                        text: literal.lexical.clone(),
                        language: literal.language.as_ref().map(|l| l.to_lowercase()),
                        triple: Some(triple.clone()),
                    },
                );
            }
            (skos::NOTATION, Some(subject), _) if triple.object.as_literal().is_some() => {
                let literal = triple.object.as_literal().expect("checked above");
                add(&mut notations, subject, literal.lexical.clone());
            }
            (skos::IN_SCHEME, Some(subject), Some(object)) => add(&mut memberships, subject, object.clone()),
            (skos::TOP_CONCEPT_OF, Some(subject), Some(object)) => {
                add(&mut memberships, subject, object.clone());
                add(&mut tops_by_scheme, object, subject.clone());
            }
            (skos::HAS_TOP_CONCEPT, Some(subject), Some(object)) => {
                add(&mut memberships, object, subject.clone());
                add(&mut tops_by_scheme, subject, object.clone());
            }
            (skos::BROADER, Some(subject), Some(object)) => {
                pair(&mut hierarchy_pairs, (object.clone(), subject.clone()), triple)
            }
            (skos::NARROWER, Some(subject), Some(object)) => {
                pair(&mut hierarchy_pairs, (subject.clone(), object.clone()), triple)
            }
            (skos::RELATED, Some(subject), Some(object)) => {
                let key = if subject <= object {
                    (subject.clone(), object.clone())
                } else {
                    (object.clone(), subject.clone())
                };
                pair(&mut related_pairs, key, triple);
            }
            (skos::MEMBER, Some(subject), Some(object)) => {
                if !ordered_collection_ids.contains(subject) {
                    add(&mut members_by_collection, subject, object.clone());
                }
            }
            (skos::MEMBER_LIST, Some(subject), _) => {
                let mut cursor = triple.object.clone();
                while let RdfTerm::Blank(cell) = &cursor {
                    let Some(member) = firsts.get(cell) else {
                        break;
                    };
                    if let Some(member_id) = element_id(member) {
                        add(&mut members_by_collection, subject, member_id);
                    }
                    cursor = rests
                        .get(cell)
                        .cloned()
                        .unwrap_or_else(|| RdfTerm::iri(rdf::NIL, "rdf:nil"));
                }
            }
            (_, Some(subject), _) if skos::MAPPINGS.contains(&predicate) => match &object_id {
                Some(object) if concept_ids.contains(subject) && concept_ids.contains(object) => {
                    mapping_edges.push(SkosEdge {
                        id: format!("edge:{subject}|{predicate}|{object}"),
                        from_id: subject.clone(),
                        to_id: object.clone(),
                        kind: SkosEdgeKind::Mapping,
                        predicate_iri: predicate.to_owned(),
                        triples: vec![triple.clone()],
                        asserted_both_ways: false,
                    });
                }
                _ => out_of_file_mappings.push(triple.clone()),
            },
            _ => {}
        }
    }

    // The budget order, walked over the drawable element set.
    let mut children_of: HashMap<String, Vec<String>> = HashMap::new();
    for (broader, narrower) in hierarchy_pairs.keys() {
        add(&mut children_of, broader, narrower.clone());
    }

    let mut order: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let take = |id: &str, order: &mut Vec<String>, seen: &mut HashSet<String>| {
        if (concept_ids.contains(id) || scheme_ids.contains(id) || collection_ids.contains(id))
            && seen.insert(id.to_owned())
        {
            order.push(id.to_owned());
        }
    };

    for scheme_id in sorted(&scheme_ids) {
        take(&scheme_id, &mut order, &mut seen);
        let members: HashSet<&String> = concept_ids
            .iter()
            .filter(|id| memberships.get(*id).is_some_and(|m| m.contains(&scheme_id)))
            .collect();
        let tops = tops_by_scheme.get(&scheme_id).map(|t| distinct(t)).unwrap_or_default();
        let broaderless = members.iter().filter(|id| {
            !hierarchy_pairs
                .keys()
                .any(|(broader, narrower)| narrower == **id && members.contains(broader))
        });
        let mut layer = distinct(
            &tops
                .iter()
                .cloned()
                .chain(broaderless.map(|id| (*id).clone()))
                .collect::<Vec<_>>(),
        );
        layer.sort();
        while !layer.is_empty() {
            let mut next = Vec::new();
            for id in &layer {
                if members.contains(id) || tops.contains(id) {
                    take(id, &mut order, &mut seen);
                }
                if let Some(children) = children_of.get(id) {
                    next.extend(
                        children
                            .iter()
                            .filter(|child| members.contains(child) && !seen.contains(*child))
                            .cloned(),
                    );
                }
            }
            layer = distinct(&next);
            layer.sort();
        }
        for id in sorted(members.iter().copied()) {
            take(&id, &mut order, &mut seen);
        }
    }
    for id in sorted(&concept_ids) {
        take(&id, &mut order, &mut seen);
    }
    for id in sorted(&collection_ids) {
        take(&id, &mut order, &mut seen);
    }

    let total = order.len();
    let kept: HashSet<&String> = order.iter().take(budget).collect();
    let labels_of = |id: &str| labels.get(id).cloned().unwrap_or_default();

    let schemes = order
        .iter()
        .filter(|id| kept.contains(id) && scheme_ids.contains(*id))
        .map(|id| SkosScheme {
            id: id.clone(),
            iri: iri_of(id),
            labels: labels_of(id),
            top_concept_ids: tops_by_scheme.get(id).map(|t| distinct(t)).unwrap_or_default(),
        })
        .collect();
    let concepts = order
        .iter()
        .filter(|id| kept.contains(id) && concept_ids.contains(*id))
        .map(|id| SkosConcept {
            id: id.clone(),
            iri: iri_of(id),
            labels: labels_of(id),
            notations: notations.get(id).cloned().unwrap_or_default(),
            scheme_iris: distinct(
                &memberships
                    .get(id)
                    .map(|m| {
                        m.iter()
                            .filter(|s| scheme_ids.contains(*s))
                            .cloned()
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default(),
            )
            .iter()
            .map(|s| iri_of(s))
            .collect(),
            blank: id.starts_with("blank:"),
        })
        .collect();
    let collections = order
        .iter()
        .filter(|id| kept.contains(id) && collection_ids.contains(*id))
        .map(|id| SkosCollection {
            id: id.clone(),
            iri: iri_of(id),
            labels: labels_of(id),
            ordered: ordered_collection_ids.contains(id),
            member_ids: members_by_collection
                .get(id)
                .map(|m| m.iter().filter(|member| kept.contains(member)).cloned().collect())
                .unwrap_or_default(),
        })
        .collect();

    let mut edges = Vec::new();
    for ((broader, narrower), (triples, both)) in &hierarchy_pairs {
        if kept.contains(broader) && kept.contains(narrower) {
            edges.push(SkosEdge {
                id: format!("edge:{narrower}|{}|{broader}", skos::BROADER),
                from_id: broader.clone(),
                to_id: narrower.clone(),
                kind: SkosEdgeKind::Hierarchy,
                predicate_iri: skos::BROADER.to_owned(),
                triples: triples.clone(),
                asserted_both_ways: *both,
            });
        }
    }
    for ((a, b), (triples, both)) in &related_pairs {
        if kept.contains(a) && kept.contains(b) {
            edges.push(SkosEdge {
                id: format!("edge:{a}|{}|{b}", skos::RELATED),
                from_id: a.clone(),
                to_id: b.clone(),
                kind: SkosEdgeKind::Related,
                predicate_iri: skos::RELATED.to_owned(),
                triples: triples.clone(),
                asserted_both_ways: *both,
            });
        }
    }
    edges.extend(
        mapping_edges
            .into_iter()
            .filter(|e| kept.contains(&e.from_id) && kept.contains(&e.to_id)),
    );
    edges.sort_by(|a, b| a.id.cmp(&b.id));

    SkosProjectionResult {
        schemes,
        concepts,
        collections,
        edges,
        out_of_file_mappings,
        shown: kept.len(),
        total,
    }
}
