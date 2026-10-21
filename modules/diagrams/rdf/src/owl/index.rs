//! The classification pass under the ontology projection (etalii.adp spec 030, task T083), ported from the standalone
//! host's `Owl/OwlModelIndex.cs` at `8b6ccfc`: which IRIs a file declares or uses as classes, properties, datatypes and
//! individuals, plus the lookups the sweeps need. Classification is per role, so a punned IRI lands in more than one
//! set; everything is asserted, never entailed.

use std::collections::{HashMap, HashSet};

use indexmap::IndexMap;

use crate::model::{BlankTerm, RdfModel, RdfTerm, RdfTriple};
use crate::vocabulary::{owl, rdf};

/// The characteristic words a property type folds into a property's label.
const CHARACTERISTIC_WORDS: [(&str, &str); 7] = [
    (owl::FUNCTIONAL_PROPERTY, "functional"),
    (owl::INVERSE_FUNCTIONAL_PROPERTY, "inverse functional"),
    (owl::TRANSITIVE_PROPERTY, "transitive"),
    (owl::SYMMETRIC_PROPERTY, "symmetric"),
    (owl::ASYMMETRIC_PROPERTY, "asymmetric"),
    (owl::REFLEXIVE_PROPERTY, "reflexive"),
    (owl::IRREFLEXIVE_PROPERTY, "irreflexive"),
];

/// The characteristic types OWL 2 defines as object properties by themselves; `owl:FunctionalProperty` is absent, as
/// it types datatype properties too.
const OBJECT_PROPERTY_TYPES: [&str; 7] = [
    owl::OBJECT_PROPERTY,
    owl::INVERSE_FUNCTIONAL_PROPERTY,
    owl::TRANSITIVE_PROPERTY,
    owl::SYMMETRIC_PROPERTY,
    owl::ASYMMETRIC_PROPERTY,
    owl::REFLEXIVE_PROPERTY,
    owl::IRREFLEXIVE_PROPERTY,
];

/// What one model states as OWL (`OwlModelIndex`).
#[derive(Debug, Clone, Default)]
pub struct OwlModelIndex {
    pub classes: HashSet<String>,
    pub individuals: HashSet<String>,
    pub object_properties: Vec<String>,
    pub data_properties: Vec<String>,
    pub annotation_properties: HashSet<String>,
    pub declared_subjects: HashSet<String>,
    pub deprecated: HashSet<String>,
    pub labels: HashMap<String, String>,
    pub firsts: HashMap<usize, RdfTerm>,
    pub rests: HashMap<usize, RdfTerm>,
    pub disjoint_groups: Vec<Vec<String>>,
    pub ontology_iri: Option<String>,
    types: IndexMap<String, Vec<String>>,
    domains: HashMap<String, Vec<String>>,
    ranges: HashMap<String, Vec<String>>,
    imports: HashMap<String, Vec<String>>,
    inverses: HashMap<String, String>,
    by_blank_subject: IndexMap<usize, Vec<RdfTriple>>,
    declared_datatypes: HashSet<String>,
}

fn add(map: &mut HashMap<String, Vec<String>>, key: &str, value: &str) {
    map.entry(key.to_owned()).or_default().push(value.to_owned());
}

fn add_once(list: &mut Vec<String>, value: &str) {
    if !list.iter().any(|v| v == value) {
        list.push(value.to_owned());
    }
}

impl OwlModelIndex {
    /// The index of `model` (`OwlModelIndex.Build`).
    pub fn build(model: &RdfModel) -> OwlModelIndex {
        let mut index = OwlModelIndex::default();

        // First pass: declarations, plumbing and per-subject facts.
        for triple in &model.triples {
            if let RdfTerm::Blank(blank) = &triple.subject {
                index
                    .by_blank_subject
                    .entry(blank.ordinal)
                    .or_default()
                    .push(triple.clone());
                match triple.predicate.iri.as_str() {
                    rdf::FIRST => {
                        index.firsts.insert(blank.ordinal, triple.object.clone());
                    }
                    rdf::REST => {
                        index.rests.insert(blank.ordinal, triple.object.clone());
                    }
                    _ => {}
                }
                continue;
            }
            let Some(subject) = triple.subject.as_iri() else {
                continue;
            };
            index.declared_subjects.insert(subject.to_owned());
            let object_iri = triple.object.as_iri();
            match triple.predicate.iri.as_str() {
                rdf::TYPE => {
                    if let Some(type_iri) = object_iri {
                        index
                            .types
                            .entry(subject.to_owned())
                            .or_default()
                            .push(type_iri.to_owned());
                        index.classify(subject, type_iri);
                    }
                }
                rdf::LABEL => {
                    if let Some(label) = triple.object.as_literal()
                        && !index.labels.contains_key(subject)
                    {
                        index.labels.insert(subject.to_owned(), label.lexical.clone());
                    }
                }
                owl::DEPRECATED => {
                    if triple.object.as_literal().is_some_and(|l| l.lexical == "true") {
                        index.deprecated.insert(subject.to_owned());
                    }
                }
                owl::DOMAIN => {
                    if let Some(domain) = object_iri {
                        add(&mut index.domains, subject, domain);
                        index.classes.insert(domain.to_owned());
                    }
                }
                owl::RANGE => {
                    if let Some(range) = object_iri {
                        add(&mut index.ranges, subject, range);
                    }
                }
                owl::SUB_CLASS_OF => {
                    index.classes.insert(subject.to_owned());
                    if let Some(parent) = object_iri {
                        index.classes.insert(parent.to_owned());
                    }
                }
                owl::EQUIVALENT_CLASS | owl::DISJOINT_WITH => {
                    if let Some(other) = object_iri {
                        index.classes.insert(subject.to_owned());
                        index.classes.insert(other.to_owned());
                    }
                }
                owl::INVERSE_OF => {
                    if let Some(inverse) = object_iri {
                        index.inverses.insert(subject.to_owned(), inverse.to_owned());
                    }
                }
                owl::IMPORTS => {
                    if let Some(import) = object_iri {
                        add(&mut index.imports, subject, import);
                    }
                }
                _ => {}
            }
        }

        // Second pass: individuals typed by this file's classes, and the AllDisjointClasses groups.
        let individuals: Vec<String> = index
            .types
            .iter()
            .filter(|(_, types)| types.iter().any(|t| !owl::is_built_in(t) && index.classes.contains(t)))
            .map(|(subject, _)| subject.clone())
            .collect();
        index.individuals.extend(individuals);

        let mut groups = Vec::new();
        let mut members_classes = Vec::new();
        for triples in index.by_blank_subject.values() {
            let is_group = triples
                .iter()
                .any(|t| t.predicate.iri == rdf::TYPE && t.object.is_iri(owl::ALL_DISJOINT_CLASSES));
            if !is_group {
                continue;
            }
            let Some(members_triple) = triples.iter().find(|t| t.predicate.iri == owl::MEMBERS) else {
                continue;
            };
            let mut members = Vec::new();
            let mut cursor = members_triple.object.clone();
            while let RdfTerm::Blank(cell) = &cursor {
                let Some(member) = index.firsts.get(&cell.ordinal) else {
                    break;
                };
                if let Some(member_iri) = member.as_iri() {
                    members.push(member_iri.to_owned());
                    members_classes.push(member_iri.to_owned());
                }
                cursor = index
                    .rests
                    .get(&cell.ordinal)
                    .cloned()
                    .unwrap_or_else(|| RdfTerm::iri(rdf::NIL, "rdf:nil"));
            }
            groups.push(members);
        }
        index.classes.extend(members_classes);
        index.disjoint_groups = groups;
        index
    }

    /// The asserted types of `iri`, in document order.
    pub fn types_of(&self, iri: &str) -> &[String] {
        self.types.get(iri).map_or(&[], Vec::as_slice)
    }

    pub fn domains_of(&self, property: &str) -> &[String] {
        self.domains.get(property).map_or(&[], Vec::as_slice)
    }

    pub fn ranges_of(&self, property: &str) -> &[String] {
        self.ranges.get(property).map_or(&[], Vec::as_slice)
    }

    pub fn imports_of(&self, iri: &str) -> &[String] {
        self.imports.get(iri).map_or(&[], Vec::as_slice)
    }

    pub fn inverse_of(&self, property: &str) -> Option<&str> {
        self.inverses.get(property).map(String::as_str)
    }

    /// The characteristic words of `property`, in the order its types are asserted.
    pub fn characteristics_of(&self, property: &str) -> Vec<&'static str> {
        self.types_of(property)
            .iter()
            .filter_map(|t| CHARACTERISTIC_WORDS.iter().find(|(iri, _)| iri == t).map(|(_, w)| *w))
            .collect()
    }

    /// The triples whose subject is the blank node `subject`, in document order.
    pub fn triples_of(&self, subject: &BlankTerm) -> &[RdfTriple] {
        self.by_blank_subject.get(&subject.ordinal).map_or(&[], Vec::as_slice)
    }

    /// Whether `iri` names a datatype in this file's terms: XSD's, `rdfs:Literal`, `rdf:langString`, or one declared
    /// `rdfs:Datatype`.
    pub fn is_datatype(&self, iri: &str) -> bool {
        iri.starts_with(owl::XSD_NAMESPACE)
            || iri == owl::RDFS_LITERAL
            || iri == rdf::LANG_STRING
            || self.declared_datatypes.contains(iri)
    }

    fn classify(&mut self, subject: &str, type_iri: &str) {
        match type_iri {
            owl::CLASS | owl::RDFS_CLASS => {
                self.classes.insert(subject.to_owned());
            }
            owl::DATATYPE_PROPERTY => add_once(&mut self.data_properties, subject),
            owl::ANNOTATION_PROPERTY => {
                self.annotation_properties.insert(subject.to_owned());
            }
            owl::NAMED_INDIVIDUAL => {
                self.individuals.insert(subject.to_owned());
            }
            owl::ONTOLOGY => {
                if self.ontology_iri.is_none() {
                    self.ontology_iri = Some(subject.to_owned());
                }
            }
            owl::RDFS_DATATYPE => {
                self.declared_datatypes.insert(subject.to_owned());
            }
            _ => {
                if OBJECT_PROPERTY_TYPES.contains(&type_iri) {
                    add_once(&mut self.object_properties, subject);
                }
            }
        }
    }
}
