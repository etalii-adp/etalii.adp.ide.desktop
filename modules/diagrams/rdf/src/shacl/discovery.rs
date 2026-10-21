//! Which terms of a document are shapes, by the recommendation's declaration-or-use definition (etalii.adp spec 030,
//! task T083), ported from the standalone host's `Shacl/ShaclShapeDiscovery.cs` and `Shacl/_Model/ShaclShape.cs` at
//! `8b6ccfc`. Shapes come back in document order of the triple that first made each one a shape.

use std::collections::{HashMap, HashSet};

use crate::model::{RdfModel, RdfTerm, RdfTriple};
use crate::vocabulary::{rdf, shacl};

/// One discovered shape (`ShaclShape`).
#[derive(Debug, Clone, PartialEq)]
pub struct ShaclShape {
    pub term: RdfTerm,
    /// `i:{iri}` or `b:{ordinal}`.
    pub key: String,
    pub first_triple_index: usize,
    pub is_property_shape: bool,
    pub implicit_class_target: bool,
}

/// The identity key of a term, or none for a literal, which is never a shape.
pub fn key_of(term: &RdfTerm) -> Option<String> {
    match term {
        RdfTerm::Iri(iri) => Some(format!("i:{}", iri.iri)),
        RdfTerm::Blank(blank) => Some(format!("b:{}", blank.ordinal)),
        RdfTerm::Literal(_) => None,
    }
}

/// Every shape of `model`, in document order of what made it one.
pub fn discover(model: &RdfModel) -> Vec<ShaclShape> {
    let mut found: HashMap<String, (RdfTerm, usize)> = HashMap::new();
    let mark = |found: &mut HashMap<String, (RdfTerm, usize)>, term: &RdfTerm, index: usize| {
        let Some(key) = key_of(term) else {
            return;
        };
        match found.get(&key) {
            Some((_, existing)) if *existing <= index => {}
            _ => {
                found.insert(key, (term.clone(), index));
            }
        }
    };
    for (index, triple) in model.triples.iter().enumerate() {
        let predicate = triple.predicate.iri.as_str();
        if predicate == rdf::TYPE
            && triple
                .object
                .as_iri()
                .is_some_and(|o| o == shacl::NODE_SHAPE || o == shacl::PROPERTY_SHAPE)
        {
            mark(&mut found, &triple.subject, index);
            continue;
        }
        if shacl::TARGET_PREDICATES.contains(&predicate) || shacl::is_constraint_parameter(predicate) {
            mark(&mut found, &triple.subject, index);
        }
        if shacl::SHAPE_EXPECTING_PREDICATES.contains(&predicate) {
            mark(&mut found, &triple.object, index);
        }
        if shacl::SHAPE_LIST_PREDICATES.contains(&predicate) {
            for (member, member_index) in list_members(model, &triple.object) {
                mark(&mut found, &member, member_index);
            }
        }
    }
    let path_subjects: HashSet<String> = model
        .triples
        .iter()
        .filter(|t| t.predicate.iri == shacl::PATH)
        .filter_map(|t| key_of(&t.subject))
        .collect();
    let class_subjects = class_subjects(model);
    let mut shapes: Vec<ShaclShape> = found
        .into_iter()
        .map(|(key, (term, index))| ShaclShape {
            implicit_class_target: matches!(term, RdfTerm::Iri(_)) && class_subjects.contains(&key),
            is_property_shape: path_subjects.contains(&key),
            term,
            key,
            first_triple_index: index,
        })
        .collect();
    // Two shapes made by one triple (a list's members share none, a subject and its object do) keep the order the
    // standalone host's dictionary gave them: the subject first.
    shapes.sort_by(|a, b| {
        a.first_triple_index
            .cmp(&b.first_triple_index)
            .then_with(|| subject_first(model, a, b))
    });
    shapes
}

fn subject_first(model: &RdfModel, a: &ShaclShape, b: &ShaclShape) -> std::cmp::Ordering {
    let triple = &model.triples[a.first_triple_index];
    let a_subject = key_of(&triple.subject).as_deref() == Some(a.key.as_str());
    let b_subject = key_of(&triple.subject).as_deref() == Some(b.key.as_str());
    b_subject.cmp(&a_subject)
}

/// The members of an RDF list with the index of each one's `rdf:first` triple; a malformed or cyclic list yields what
/// it has.
fn list_members(model: &RdfModel, head: &RdfTerm) -> Vec<(RdfTerm, usize)> {
    let mut members = Vec::new();
    let mut visited = HashSet::new();
    let mut current = head.clone();
    while !current.is_iri(rdf::NIL) {
        let Some(key) = key_of(&current) else {
            break;
        };
        if !visited.insert(key.clone()) {
            break;
        }
        let first = first_triple(model, &key, rdf::FIRST);
        let rest = first_triple(model, &key, rdf::REST);
        if let Some((triple, index)) = first {
            members.push((triple.object.clone(), index));
        }
        let Some((rest, _)) = rest else {
            break;
        };
        current = rest.object.clone();
    }
    members
}

fn first_triple<'a>(model: &'a RdfModel, subject_key: &str, predicate: &str) -> Option<(&'a RdfTriple, usize)> {
    model
        .triples
        .iter()
        .enumerate()
        .find(|(_, t)| t.predicate.iri == predicate && key_of(&t.subject).as_deref() == Some(subject_key))
        .map(|(index, t)| (t, index))
}

/// Every subject the file states to be a class: typed `rdfs:Class`, or typed with a class the file's own
/// `rdfs:subClassOf` triples reach `rdfs:Class` from.
fn class_subjects(model: &RdfModel) -> HashSet<String> {
    let mut classes: HashSet<String> = HashSet::from([shacl::RDFS_CLASS.to_owned()]);
    loop {
        let mut grew = false;
        for triple in &model.triples {
            if triple.predicate.iri == shacl::RDFS_SUB_CLASS_OF
                && let (Some(subject), Some(parent)) = (triple.subject.as_iri(), triple.object.as_iri())
                && classes.contains(parent)
                && classes.insert(subject.to_owned())
            {
                grew = true;
            }
        }
        if !grew {
            break;
        }
    }
    model
        .triples
        .iter()
        .filter(|t| t.predicate.iri == rdf::TYPE && t.object.as_iri().is_some_and(|o| classes.contains(o)))
        .filter_map(|t| key_of(&t.subject))
        .collect()
}
