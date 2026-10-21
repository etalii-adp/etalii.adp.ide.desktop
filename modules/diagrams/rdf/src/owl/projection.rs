//! The ontology reading's projection (etalii.adp spec 030, task T083): what a model draws when its triples are read as
//! OWL 2 axioms. Ported from the standalone host's `Owl/OwlProjection.cs` and `Owl/OwlGraph.cs` at `8b6ccfc`.
//!
//! Classes, datatypes and individuals draw as nodes while properties draw as edges between their stated domains and
//! ranges (VOWL's vocabulary); a domain-less or range-less property anchors at a materialized `owl:Thing`; types are
//! badges, never edges; blank-node-rooted class expressions draw as expression nodes on structural ids; and everything
//! shown is asserted. The budget cut walks whole units - a named node with the expression nodes attached to it - in
//! document order of first appearance, so a class is never split from its expressions.

use std::collections::{HashMap, HashSet};

use crate::model::{BlankTerm, RdfModel, RdfTerm};
use crate::owl::expression::{self, CANVAS_DEPTH};
use crate::owl::index::OwlModelIndex;
use crate::projection::{self as family, RdfRow};
use crate::vocabulary::{owl, rdf};

/// What kind of node an ontology element draws as (`OwlNodeKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OwlNodeKind {
    Class,
    Datatype,
    Individual,
    Thing,
    OntologyHeader,
    Restriction,
    Operator,
}

/// What kind of axiom an edge draws (`OwlEdgeKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OwlEdgeKind {
    Subclass,
    Equivalent,
    Disjoint,
    ObjectProperty,
    DatatypeProperty,
    Assertion,
    Expression,
}

/// One drawn element of the ontology reading (`OwlNode`).
#[derive(Debug, Clone, PartialEq)]
pub struct OwlNode {
    pub id: String,
    pub kind: OwlNodeKind,
    /// Empty for expression nodes and materialized anchors.
    pub iri: String,
    pub display: String,
    pub badges: Vec<String>,
    pub rows: Vec<RdfRow>,
    pub deprecated: bool,
    pub external: bool,
    /// For an expression node, the id of the named class whose axiom attaches it; empty elsewhere.
    pub owner_id: String,
    /// For an expression node, the blank term rooting its structure.
    pub expression_root: Option<BlankTerm>,
    pub malformed: bool,
    pub elided: bool,
}

/// One drawn axiom or assertion (`OwlEdge`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwlEdge {
    pub id: String,
    pub kind: OwlEdgeKind,
    pub from_id: String,
    pub to_id: String,
    pub label: String,
    pub property_iri: String,
}

/// What the ontology reading draws (`OwlGraphResult`).
#[derive(Debug, Clone, PartialEq)]
pub struct OwlGraph {
    pub nodes: Vec<OwlNode>,
    pub edges: Vec<OwlEdge>,
    pub shown: usize,
    pub total: usize,
}

impl OwlGraph {
    /// Whether the budget cut the graph down.
    pub fn truncated(&self) -> bool {
        self.shown < self.total
    }
}

const QUANTIFIER_PREDICATES: [&str; 9] = [
    owl::SOME_VALUES_FROM,
    owl::ALL_VALUES_FROM,
    owl::HAS_VALUE,
    owl::CARDINALITY,
    owl::MIN_CARDINALITY,
    owl::MAX_CARDINALITY,
    owl::QUALIFIED_CARDINALITY,
    owl::MIN_QUALIFIED_CARDINALITY,
    owl::MAX_QUALIFIED_CARDINALITY,
];

struct Builder<'a> {
    model: &'a RdfModel,
    index: OwlModelIndex,
    nodes: HashMap<String, OwlNode>,
    order: Vec<String>,
    unit_of: HashMap<String, String>,
    edges: Vec<OwlEdge>,
    edge_counts: HashMap<String, usize>,
    /// The blank nodes whose expression is being built, so a structure that contains itself is not built forever.
    building: HashSet<usize>,
}

impl Builder<'_> {
    fn display(&self, iri: &str) -> String {
        expression::display(&self.index, self.model, iri)
    }

    fn ensure_nodes_for(&mut self, term: &RdfTerm) {
        let Some(iri) = term.as_iri() else {
            return;
        };
        if self.index.classes.contains(iri) || iri == owl::THING {
            let kind = if iri == owl::THING || iri == owl::NOTHING {
                OwlNodeKind::Thing
            } else {
                OwlNodeKind::Class
            };
            self.ensure(&format!("res:{iri}"), kind, iri);
        }
        if self.index.is_datatype(iri) {
            self.ensure_datatype(iri);
        }
        if self.index.individuals.contains(iri) {
            let id = self.individual_id(iri);
            self.ensure(&id, OwlNodeKind::Individual, iri);
        }
        if self.index.ontology_iri.as_deref() == Some(iri) {
            self.ensure(&format!("res:{iri}"), OwlNodeKind::OntologyHeader, iri);
        }
    }

    fn ensure(&mut self, id: &str, kind: OwlNodeKind, iri: &str) {
        if self.nodes.contains_key(id) {
            return;
        }
        // Datatype nodes skip the external dimming: their kind already says they are borrowed vocabulary.
        let external = matches!(kind, OwlNodeKind::Class | OwlNodeKind::Individual)
            && !self.index.declared_subjects.contains(iri)
            && !owl::is_built_in(iri);
        let node = OwlNode {
            id: id.to_owned(),
            kind,
            iri: iri.to_owned(),
            display: self.display(iri),
            badges: Vec::new(),
            rows: Vec::new(),
            deprecated: self.index.deprecated.contains(iri),
            external,
            owner_id: String::new(),
            expression_root: None,
            malformed: false,
            elided: false,
        };
        self.nodes.insert(id.to_owned(), node);
        self.order.push(id.to_owned());
    }

    fn ensure_datatype(&mut self, iri: &str) -> String {
        let id = format!("res:{iri}");
        self.ensure(&id, OwlNodeKind::Datatype, iri);
        id
    }

    fn ensure_anchor(&mut self, id: &str, kind: OwlNodeKind, display: &str) -> String {
        if !self.nodes.contains_key(id) {
            self.nodes.insert(
                id.to_owned(),
                OwlNode {
                    id: id.to_owned(),
                    kind,
                    iri: String::new(),
                    display: display.to_owned(),
                    badges: Vec::new(),
                    rows: Vec::new(),
                    deprecated: false,
                    external: false,
                    owner_id: String::new(),
                    expression_root: None,
                    malformed: false,
                    elided: false,
                },
            );
            self.order.push(id.to_owned());
        }
        id.to_owned()
    }

    fn endpoint_ids(&mut self, stated: Vec<String>, property: &str, side: &str) -> Vec<String> {
        let mut ids: Vec<String> = stated
            .iter()
            .map(|iri| format!("res:{iri}"))
            .filter(|id| self.nodes.contains_key(id))
            .collect();
        if ids.is_empty() {
            // The VOWL anchor: a property without a stated end attaches to its own small owl:Thing.
            ids.push(self.ensure_anchor(&format!("thing:{property}|{side}"), OwlNodeKind::Thing, "Thing"));
        }
        ids
    }

    fn individual_id(&self, iri: &str) -> String {
        if self.index.classes.contains(iri) {
            format!("ind:{iri}")
        } else {
            format!("res:{iri}")
        }
    }

    fn primary_id(&self, iri: &str) -> Option<String> {
        if self.index.ontology_iri.as_deref() == Some(iri) || self.index.classes.contains(iri) {
            return Some(format!("res:{iri}"));
        }
        if self.index.individuals.contains(iri) {
            return Some(self.individual_id(iri));
        }
        let id = format!("res:{iri}");
        self.nodes.contains_key(&id).then_some(id)
    }

    fn property_label(&self, property: &str) -> String {
        let mut words: Vec<String> = self
            .index
            .characteristics_of(property)
            .into_iter()
            .map(str::to_owned)
            .collect();
        if let Some(inverse) = self.index.inverse_of(property) {
            words.push(format!("inverse of {}", self.display(inverse)));
        }
        if words.is_empty() {
            self.display(property)
        } else {
            format!("{} ({})", self.display(property), words.join(", "))
        }
    }

    fn edge(&mut self, kind: OwlEdgeKind, from_id: &str, to_id: &str, label: &str, property_iri: &str) {
        if !self.nodes.contains_key(from_id) || !self.nodes.contains_key(to_id) {
            return;
        }
        let key = format!("{from_id}|{property_iri}|{to_id}");
        let count = self.edge_counts.entry(key.clone()).or_insert(0);
        let id = if *count == 0 {
            format!("edge:{key}")
        } else {
            format!("edge:{key}|{count}")
        };
        *count += 1;
        self.edges.push(OwlEdge {
            id,
            kind,
            from_id: from_id.to_owned(),
            to_id: to_id.to_owned(),
            label: label.to_owned(),
            property_iri: property_iri.to_owned(),
        });
    }

    fn malform(&mut self, id: &str) {
        if let Some(node) = self.nodes.get_mut(id) {
            node.malformed = true;
        }
    }

    fn build_expression(&mut self, id: &str, owner_id: &str, root: &BlankTerm) {
        let about = self.index.triples_of(root).to_vec();
        let is_operator = about.iter().any(|t| {
            matches!(
                t.predicate.iri.as_str(),
                owl::UNION_OF | owl::INTERSECTION_OF | owl::COMPLEMENT_OF | owl::ONE_OF
            )
        });
        let has_on_property = about.iter().any(|t| t.predicate.iri == owl::ON_PROPERTY);
        let quantifiers = about
            .iter()
            .filter(|t| QUANTIFIER_PREDICATES.contains(&t.predicate.iri.as_str()))
            .count();
        let malformed = !is_operator && (!has_on_property || quantifiers != 1);

        let kind = if is_operator {
            OwlNodeKind::Operator
        } else {
            OwlNodeKind::Restriction
        };
        let rendered = expression::render_with(
            &RdfTerm::Blank(root.clone()),
            &self.index,
            self.model,
            Some(CANVAS_DEPTH),
        );
        self.nodes.insert(
            id.to_owned(),
            OwlNode {
                id: id.to_owned(),
                kind,
                iri: String::new(),
                display: rendered.text,
                badges: Vec::new(),
                rows: Vec::new(),
                deprecated: false,
                external: false,
                owner_id: owner_id.to_owned(),
                expression_root: Some(root.clone()),
                malformed: malformed || rendered.cyclic,
                elided: rendered.elided,
            },
        );
        self.order.push(id.to_owned());
        self.unit_of.insert(id.to_owned(), owner_id.to_owned());
        self.building.insert(root.ordinal);

        let mut child_index = 0;
        for triple in &about {
            match triple.predicate.iri.as_str() {
                owl::SOME_VALUES_FROM
                | owl::ALL_VALUES_FROM
                | owl::ON_CLASS
                | owl::ON_DATA_RANGE
                | owl::HAS_VALUE
                | owl::COMPLEMENT_OF => self.reference(id, owner_id, &triple.object, &mut child_index),
                owl::UNION_OF | owl::INTERSECTION_OF | owl::ONE_OF => {
                    let mut cursor = triple.object.clone();
                    while let RdfTerm::Blank(cell) = &cursor {
                        let Some(member) = self.index.firsts.get(&cell.ordinal).cloned() else {
                            break;
                        };
                        self.reference(id, owner_id, &member, &mut child_index);
                        cursor = self
                            .index
                            .rests
                            .get(&cell.ordinal)
                            .cloned()
                            .unwrap_or_else(|| RdfTerm::iri(rdf::NIL, "rdf:nil"));
                    }
                    // A list that neither reaches rdf:nil nor follows the cons shape is broken.
                    match &cursor {
                        RdfTerm::Blank(dangling) => {
                            if !self.index.firsts.contains_key(&dangling.ordinal) {
                                self.malform(id);
                            }
                        }
                        RdfTerm::Iri(iri) if iri.iri == rdf::NIL => {}
                        _ => self.malform(id),
                    }
                }
                _ => {}
            }
        }
        self.building.remove(&root.ordinal);
    }

    fn reference(&mut self, id: &str, owner_id: &str, target: &RdfTerm, child_index: &mut usize) {
        match target {
            RdfTerm::Iri(iri)
                if !owl::is_built_in(&iri.iri)
                    || self.index.is_datatype(&iri.iri)
                    || iri.iri == owl::THING
                    || iri.iri == owl::NOTHING =>
            {
                // A named filler: draw the reference to its class, datatype or individual node.
                self.ensure_nodes_for(target);
                let res = format!("res:{}", iri.iri);
                if !self.nodes.contains_key(&res) {
                    let kind = if self.index.is_datatype(&iri.iri) {
                        OwlNodeKind::Datatype
                    } else {
                        OwlNodeKind::Class
                    };
                    self.ensure(&res, kind, &iri.iri);
                }
                self.edge(OwlEdgeKind::Expression, id, &res, "", "");
            }
            RdfTerm::Blank(child) => {
                // A nested expression: its own node on a child-suffixed structural id. A structure that contains
                // itself is drawn once (the standalone host recursed without end on one; its renderer marks it).
                let child_id = format!("{id}/{child_index}");
                *child_index += 1;
                if self.building.contains(&child.ordinal) {
                    return;
                }
                self.build_expression(&child_id, owner_id, child);
                self.edge(OwlEdgeKind::Expression, id, &child_id, "", "");
            }
            _ => {}
        }
    }
}

/// What `model` draws as an ontology, cut to `budget` nodes (`OwlProjection.Project`).
pub fn project(model: &RdfModel, budget: usize) -> OwlGraph {
    let mut b = Builder {
        model,
        index: OwlModelIndex::build(model),
        nodes: HashMap::new(),
        order: Vec::new(),
        unit_of: HashMap::new(),
        edges: Vec::new(),
        edge_counts: HashMap::new(),
        building: HashSet::new(),
    };

    // Named nodes, in document order of first appearance.
    for triple in &model.triples {
        b.ensure_nodes_for(&triple.subject);
        b.ensure_nodes_for(&triple.object);
    }

    // Rows.
    for triple in &model.triples {
        let (Some(subject), Some(literal)) = (triple.subject.as_iri(), triple.object.as_literal()) else {
            continue;
        };
        if triple.predicate.iri == owl::DEPRECATED {
            continue; // The flag, not a row.
        }
        if let Some(id) = b.primary_id(subject) {
            let row = RdfRow {
                predicate: family::display(model, &triple.predicate.iri),
                value: literal.lexical.clone(),
                annotation: family::annotation(model, literal),
            };
            if let Some(node) = b.nodes.get_mut(&id) {
                node.rows.push(row);
            }
        }
    }

    // Badges.
    let individuals: Vec<String> = b.index.individuals.iter().cloned().collect();
    for individual in individuals {
        let prefix = if b.index.classes.contains(&individual) {
            "ind"
        } else {
            "res"
        };
        let id = format!("{prefix}:{individual}");
        if b.nodes.contains_key(&id) {
            let badges: Vec<String> = b
                .index
                .types_of(&individual)
                .iter()
                .filter(|t| !owl::is_built_in(t))
                .map(|t| b.display(t))
                .collect();
            if let Some(node) = b.nodes.get_mut(&id) {
                node.badges = badges;
            }
        }
    }

    if let Some(ontology_iri) = b.index.ontology_iri.clone() {
        let id = format!("res:{ontology_iri}");
        let imports: Vec<RdfRow> = b
            .index
            .imports_of(&ontology_iri)
            .iter()
            .map(|import| RdfRow {
                predicate: "owl:imports".to_owned(),
                value: import.clone(),
                annotation: String::new(),
            })
            .collect();
        if let Some(header) = b.nodes.get_mut(&id) {
            header.rows.extend(imports);
        }
    }

    // Property edges.
    for property in b.index.object_properties.clone() {
        let label = b.property_label(&property);
        let froms = b.endpoint_ids(b.index.domains_of(&property).to_vec(), &property, "domain");
        let ranges: Vec<String> = b
            .index
            .ranges_of(&property)
            .iter()
            .filter(|r| !b.index.is_datatype(r))
            .cloned()
            .collect();
        let tos = b.endpoint_ids(ranges, &property, "range");
        for from in &froms {
            for to in &tos {
                b.edge(OwlEdgeKind::ObjectProperty, from, to, &label, &property);
            }
        }
    }

    for property in b.index.data_properties.clone() {
        let label = b.property_label(&property);
        let froms = b.endpoint_ids(b.index.domains_of(&property).to_vec(), &property, "domain");
        let ranges: Vec<String> = b
            .index
            .ranges_of(&property)
            .iter()
            .filter(|r| b.index.is_datatype(r))
            .cloned()
            .collect();
        let tos: Vec<String> = if ranges.is_empty() {
            vec![b.ensure_anchor(&format!("dt:{property}"), OwlNodeKind::Datatype, "Literal")]
        } else {
            ranges.iter().map(|r| b.ensure_datatype(r)).collect()
        };
        for from in &froms {
            for to in &tos {
                b.edge(OwlEdgeKind::DatatypeProperty, from, to, &label, &property);
            }
        }
    }

    // Axiom edges and assertions, in document order.
    for triple in &model.triples {
        let Some(subject) = triple.subject.as_iri() else {
            continue;
        };
        let predicate = triple.predicate.iri.as_str();
        let object = triple.object.as_iri();
        match (predicate, object) {
            (owl::SUB_CLASS_OF, Some(to)) => b.edge(
                OwlEdgeKind::Subclass,
                &format!("res:{subject}"),
                &format!("res:{to}"),
                "",
                predicate,
            ),
            (owl::EQUIVALENT_CLASS, Some(to)) if b.index.classes.contains(subject) => b.edge(
                OwlEdgeKind::Equivalent,
                &format!("res:{subject}"),
                &format!("res:{to}"),
                "",
                predicate,
            ),
            (owl::DISJOINT_WITH, Some(to)) => b.edge(
                OwlEdgeKind::Disjoint,
                &format!("res:{subject}"),
                &format!("res:{to}"),
                "",
                predicate,
            ),
            _ => {
                if let Some(target) = object
                    && b.index.individuals.contains(subject)
                    && !owl::is_built_in(predicate)
                    && b.nodes.contains_key(&b.individual_id(target))
                {
                    let from = b.individual_id(subject);
                    let to = b.individual_id(target);
                    let label = b.display(predicate);
                    b.edge(OwlEdgeKind::Assertion, &from, &to, &label, predicate);
                }
            }
        }
    }

    // owl:AllDisjointClasses groups: pairwise disjointness edges over the members list.
    for group in b.index.disjoint_groups.clone() {
        for i in 0..group.len() {
            for j in i + 1..group.len() {
                b.edge(
                    OwlEdgeKind::Disjoint,
                    &format!("res:{}", group[i]),
                    &format!("res:{}", group[j]),
                    "",
                    owl::ALL_DISJOINT_CLASSES,
                );
            }
        }
    }

    // Class expressions, on structural ids: deterministic within a parse, unstable across edits.
    let mut ordinals: HashMap<String, usize> = HashMap::new();
    for triple in &model.triples {
        let Some(owner) = triple.subject.as_iri() else {
            continue;
        };
        let predicate = triple.predicate.iri.as_str();
        if !matches!(
            predicate,
            owl::SUB_CLASS_OF | owl::EQUIVALENT_CLASS | owl::DISJOINT_WITH
        ) {
            continue;
        }
        let ordinal_key = format!("{owner}|{predicate}");
        let slot = ordinals.entry(ordinal_key).or_insert(0);
        let ordinal = *slot;
        *slot += 1;
        let RdfTerm::Blank(root) = &triple.object else {
            continue;
        };
        let owner_id = format!("res:{owner}");
        let expression_id = format!("expr:{owner}|{predicate}|{ordinal}");
        let kind = match predicate {
            owl::EQUIVALENT_CLASS => OwlEdgeKind::Equivalent,
            owl::DISJOINT_WITH => OwlEdgeKind::Disjoint,
            _ => OwlEdgeKind::Subclass,
        };
        b.build_expression(&expression_id, &owner_id, root);
        b.edge(kind, &owner_id, &expression_id, "", predicate);
    }

    // The cut: whole units in document order, and only the edges both of whose ends survived.
    let mut unit_roots: Vec<String> = Vec::new();
    let mut unit_members: HashMap<String, Vec<String>> = HashMap::new();
    for id in &b.order {
        let root = b.unit_of.get(id).cloned().unwrap_or_else(|| id.clone());
        let members = unit_members.entry(root.clone()).or_insert_with(|| {
            unit_roots.push(root.clone());
            Vec::new()
        });
        members.push(id.clone());
    }
    let mut kept: Vec<String> = Vec::new();
    for root in &unit_roots {
        let members = &unit_members[root];
        if kept.len() + members.len() > budget {
            break;
        }
        kept.extend(members.iter().cloned());
    }
    let kept_set: HashSet<&String> = kept.iter().collect();
    let edges = b
        .edges
        .iter()
        .filter(|e| kept_set.contains(&e.from_id) && kept_set.contains(&e.to_id))
        .cloned()
        .collect();
    let total = b.order.len();
    let nodes: Vec<OwlNode> = kept.iter().map(|id| b.nodes[id].clone()).collect();
    OwlGraph {
        shown: kept.len(),
        nodes,
        edges,
        total,
    }
}
