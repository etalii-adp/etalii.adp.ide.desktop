//! The shapes reading (etalii.adp spec 030, task T083): node shapes as cards, property shapes as rows, targets as
//! chips, shape-to-shape references as edges, and nothing else.
//!
//! Ported from the standalone host's `Shacl/ShaclProjection.cs` and `Shacl/_Model` (`ShaclCard`, `ShaclRow`,
//! `ShaclTargetChip`, `ShaclTargetKind`, `ShaclEdge`, `ShaclProjectionResult`) at `8b6ccfc`. Pure over the model and
//! deterministic: cards in discovery order, rows in source order. The drawn-element budget counts cards only.

use std::collections::{HashMap, HashSet};

use indexmap::IndexMap;

use crate::model::{RdfModel, RdfTerm, RdfTriple};
use crate::projection::display;
use crate::shacl::discovery::{self, ShaclShape, key_of};
use crate::vocabulary::{rdf, shacl};

/// The default drawn-card budget.
pub const DEFAULT_BUDGET: usize = 1000;

/// What a target declaration selects by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaclTargetKind {
    Class,
    Node,
    SubjectsOf,
    ObjectsOf,
    ImplicitClass,
}

/// One target declaration, drawn on its card as a chip.
#[derive(Debug, Clone, PartialEq)]
pub struct ShaclTargetChip {
    pub kind: ShaclTargetKind,
    pub term_display: String,
    pub term_iri: String,
    pub described_in_file: bool,
    pub shape_iri: String,
    /// Empty for the implicit chip, which no triple states.
    pub predicate_iri: String,
}

/// One constraint row on a card.
#[derive(Debug, Clone, PartialEq)]
pub struct ShaclRow {
    pub path: String,
    pub name: String,
    pub summary: String,
    pub cardinality: String,
    pub sparql: bool,
    pub blank: bool,
    pub severity: String,
}

/// One drawn shape: a card.
#[derive(Debug, Clone, PartialEq)]
pub struct ShaclCard {
    pub id: String,
    pub iri: String,
    pub display: String,
    pub blank: bool,
    pub deactivated: bool,
    pub severity: String,
    pub closed: bool,
    pub name: String,
    pub description: String,
    pub targets: Vec<ShaclTargetChip>,
    pub rows: Vec<ShaclRow>,
}

/// One drawn shape-to-shape reference.
#[derive(Debug, Clone, PartialEq)]
pub struct ShaclEdge {
    pub id: String,
    pub from_id: String,
    pub to_id: String,
    pub kind: String,
    pub label: String,
}

/// What the shapes reading draws.
#[derive(Debug, Clone, PartialEq)]
pub struct ShaclProjectionResult {
    pub cards: Vec<ShaclCard>,
    pub edges: Vec<ShaclEdge>,
    pub shown: usize,
    pub total: usize,
}

impl ShaclProjectionResult {
    pub fn truncated(&self) -> bool {
        self.shown < self.total
    }
}

/// The fixed order constraint summaries print their parameters in.
const SUMMARY_ORDER: [&str; 16] = [
    shacl::DATATYPE,
    shacl::CLASS,
    shacl::NODE_KIND,
    "http://www.w3.org/ns/shacl#minLength",
    "http://www.w3.org/ns/shacl#maxLength",
    "http://www.w3.org/ns/shacl#pattern",
    "http://www.w3.org/ns/shacl#minInclusive",
    "http://www.w3.org/ns/shacl#maxInclusive",
    "http://www.w3.org/ns/shacl#minExclusive",
    "http://www.w3.org/ns/shacl#maxExclusive",
    "http://www.w3.org/ns/shacl#uniqueLang",
    shacl::HAS_VALUE,
    shacl::IN,
    shacl::NODE,
    shacl::QUALIFIED_VALUE_SHAPE,
    shacl::SPARQL,
];

/// The element id a shape draws under: `res:{iri}` or `blank:{ordinal}`.
pub fn id_of(shape_key: &str) -> String {
    match shape_key.strip_prefix("i:") {
        Some(iri) => format!("res:{iri}"),
        None => format!("blank:{}", &shape_key[2..]),
    }
}

/// What `model` draws as shapes, cut to `budget` cards.
pub fn project(model: &RdfModel, budget: usize) -> ShaclProjectionResult {
    let shapes = discovery::discover(model);
    let context = Context::build(model, &shapes);
    let card_shapes: Vec<&ShaclShape> = shapes.iter().filter(|s| is_card(s, &context)).collect();
    let drawn: Vec<&ShaclShape> = card_shapes.iter().take(budget).copied().collect();
    let drawn_ids: HashSet<String> = drawn.iter().map(|s| id_of(&s.key)).collect();
    let claimants = claimants_of(&drawn, &context);
    let mut cards = Vec::with_capacity(drawn.len());
    let mut edges = Vec::new();
    for shape in &drawn {
        cards.push(build_card(model, shape, &context, &claimants, &mut edges));
    }
    let mut seen = HashSet::new();
    let unique: Vec<ShaclEdge> = edges
        .into_iter()
        .filter(|e| drawn_ids.contains(&e.from_id) && drawn_ids.contains(&e.to_id))
        .filter(|e| seen.insert(e.id.clone()))
        .collect();
    ShaclProjectionResult {
        shown: cards.len(),
        cards,
        edges: unique,
        total: card_shapes.len(),
    }
}

fn is_card(shape: &ShaclShape, context: &Context<'_>) -> bool {
    if shape.is_property_shape {
        return matches!(shape.term, RdfTerm::Iri(_)) && !context.property_referenced.contains(&shape.key);
    }
    matches!(shape.term, RdfTerm::Iri(_))
        || context.node_referenced.contains(&shape.key)
        || !context.operand_referenced.contains(&shape.key)
}

fn claimants_of(drawn: &[&ShaclShape], context: &Context<'_>) -> HashMap<String, Option<String>> {
    let mut claimants: HashMap<String, Option<String>> = HashMap::new();
    for shape in drawn {
        for class in context.targeted_classes(shape) {
            let value = if claimants.contains_key(&class) {
                None
            } else {
                Some(id_of(&shape.key))
            };
            claimants.insert(class, value);
        }
    }
    claimants
}

fn build_card(
    model: &RdfModel,
    shape: &ShaclShape,
    context: &Context<'_>,
    claimants: &HashMap<String, Option<String>>,
    edges: &mut Vec<ShaclEdge>,
) -> ShaclCard {
    let id = id_of(&shape.key);
    let iri = shape.term.as_iri().unwrap_or_default().to_owned();
    let card_display = match &shape.term {
        RdfTerm::Iri(term) => display(model, &term.iri),
        RdfTerm::Blank(blank) if blank.label.as_deref().is_some_and(|l| !l.is_empty()) => {
            format!("_:{}", blank.label.as_deref().unwrap_or_default())
        }
        _ => "anonymous shape".to_owned(),
    };
    let mut chips = Vec::new();
    let mut rows = Vec::new();
    let mut severity = String::new();
    let mut deactivated = false;
    let mut closed = false;
    let mut name = String::new();
    let mut description = String::new();
    for (triple, _) in context.triples_of(&shape.key) {
        let predicate = triple.predicate.iri.as_str();
        match predicate {
            shacl::TARGET_CLASS | shacl::TARGET_NODE | shacl::TARGET_SUBJECTS_OF | shacl::TARGET_OBJECTS_OF => {
                chips.push(chip(model, context, &iri, predicate, &triple.object));
            }
            shacl::DEACTIVATED if is_true(&triple.object) => deactivated = true,
            shacl::CLOSED if is_true(&triple.object) => closed = true,
            shacl::SEVERITY => {
                if let Some(term) = triple.object.as_iri().filter(|o| *o != shacl::VIOLATION) {
                    severity = display(model, term);
                }
            }
            shacl::NAME if name.is_empty() => {
                if let Some(literal) = triple.object.as_literal() {
                    name = literal.lexical.clone();
                }
            }
            shacl::DESCRIPTION if description.is_empty() => {
                if let Some(literal) = triple.object.as_literal() {
                    description = literal.lexical.clone();
                }
            }
            shacl::PROPERTY => rows.push(property_row(model, context, &triple.object, &id, claimants, edges)),
            shacl::NODE => reference(edges, &id, &triple.object, "node", "node"),
            shacl::AND | shacl::OR | shacl::XONE => {
                combinator_row(model, context, &triple.object, &id, kind_of(predicate), &mut rows, edges);
            }
            shacl::NOT => {
                if matches!(triple.object, RdfTerm::Iri(_)) {
                    reference(edges, &id, &triple.object, "not", "not");
                } else {
                    rows.push(ShaclRow {
                        path: String::new(),
                        name: String::new(),
                        summary: format!("not({})", inline_summary(model, context, &triple.object, 0)),
                        cardinality: String::new(),
                        sparql: false,
                        blank: true,
                        severity: String::new(),
                    });
                }
            }
            shacl::SPARQL => rows.push(ShaclRow {
                path: String::new(),
                name: String::new(),
                summary: "SPARQL constraint".to_owned(),
                cardinality: String::new(),
                sparql: true,
                blank: triple.object.is_blank(),
                severity: String::new(),
            }),
            _ => {}
        }
    }
    if shape.implicit_class_target {
        chips.push(ShaclTargetChip {
            kind: ShaclTargetKind::ImplicitClass,
            term_display: card_display.clone(),
            term_iri: iri.clone(),
            described_in_file: true,
            shape_iri: iri.clone(),
            predicate_iri: String::new(),
        });
    }
    ShaclCard {
        id,
        iri,
        display: card_display,
        blank: shape.term.is_blank(),
        deactivated,
        severity,
        closed,
        name,
        description,
        targets: chips,
        rows,
    }
}

fn is_true(term: &RdfTerm) -> bool {
    term.as_literal().is_some_and(|l| l.lexical == "true")
}

fn reference(edges: &mut Vec<ShaclEdge>, from_id: &str, target: &RdfTerm, kind: &str, label: &str) {
    if let Some(key) = key_of(target) {
        let to_id = id_of(&key);
        edges.push(ShaclEdge {
            id: format!("shacl-edge:{from_id}|{kind}|{to_id}"),
            from_id: from_id.to_owned(),
            to_id,
            kind: kind.to_owned(),
            label: label.to_owned(),
        });
    }
}

fn chip(model: &RdfModel, context: &Context<'_>, shape_iri: &str, predicate: &str, target: &RdfTerm) -> ShaclTargetChip {
    let kind = match predicate {
        shacl::TARGET_CLASS => ShaclTargetKind::Class,
        shacl::TARGET_NODE => ShaclTargetKind::Node,
        shacl::TARGET_SUBJECTS_OF => ShaclTargetKind::SubjectsOf,
        _ => ShaclTargetKind::ObjectsOf,
    };
    let (term_display, term_iri) = match target {
        RdfTerm::Iri(iri) => (display(model, &iri.iri), iri.iri.clone()),
        RdfTerm::Literal(literal) => (literal.lexical.clone(), String::new()),
        RdfTerm::Blank(blank) => (
            format!(
                "_:{}",
                blank.label.clone().unwrap_or_else(|| blank.ordinal.to_string())
            ),
            String::new(),
        ),
    };
    let described = key_of(target).is_some_and(|k| context.subject_keys.contains(&k));
    ShaclTargetChip {
        kind,
        term_display,
        term_iri,
        described_in_file: described,
        shape_iri: shape_iri.to_owned(),
        predicate_iri: predicate.to_owned(),
    }
}

fn property_row(
    model: &RdfModel,
    context: &Context<'_>,
    property_shape: &RdfTerm,
    card_id: &str,
    claimants: &HashMap<String, Option<String>>,
    edges: &mut Vec<ShaclEdge>,
) -> ShaclRow {
    let Some(key) = key_of(property_shape) else {
        return ShaclRow {
            path: String::new(),
            name: String::new(),
            summary: String::new(),
            cardinality: String::new(),
            sparql: false,
            blank: true,
            severity: String::new(),
        };
    };
    let path = context
        .first_object(&key, shacl::PATH)
        .map(|p| path_syntax(model, context, p, false))
        .unwrap_or_default();
    let name = context.first_literal(&key, shacl::NAME);
    let min = context.first_literal(&key, shacl::MIN_COUNT);
    let max = context.first_literal(&key, shacl::MAX_COUNT);
    let cardinality = format!(
        "[{}..{}]",
        if min.is_empty() { "0" } else { &min },
        if max.is_empty() { "*" } else { &max }
    );
    let severity = context
        .first_object(&key, shacl::SEVERITY)
        .and_then(RdfTerm::as_iri)
        .filter(|s| *s != shacl::VIOLATION)
        .map(|s| display(model, s))
        .unwrap_or_default();
    let mut parts = Vec::new();
    for parameter in SUMMARY_ORDER {
        for value in context.objects(&key, parameter) {
            let part = summary_part(model, context, card_id, &path, parameter, value, claimants, edges);
            if !part.is_empty() {
                parts.push(part);
            }
        }
    }
    for combinator in [shacl::AND, shacl::OR, shacl::XONE, shacl::NOT] {
        for value in context.objects(&key, combinator) {
            parts.push(format!(
                "{}({})",
                kind_of(combinator),
                operand_list(model, context, combinator, value)
            ));
        }
    }
    let sparql = !context.objects(&key, shacl::SPARQL).is_empty();
    ShaclRow {
        path,
        name,
        summary: parts.join(", "),
        cardinality,
        sparql,
        blank: property_shape.is_blank(),
        severity,
    }
}

#[allow(clippy::too_many_arguments)]
fn summary_part(
    model: &RdfModel,
    context: &Context<'_>,
    card_id: &str,
    row_path: &str,
    parameter: &str,
    value: &RdfTerm,
    claimants: &HashMap<String, Option<String>>,
    edges: &mut Vec<ShaclEdge>,
) -> String {
    match (parameter, value) {
        (shacl::CLASS, RdfTerm::Iri(class)) => {
            if let Some(Some(claimant)) = claimants.get(&class.iri)
                && claimant != card_id
            {
                edges.push(ShaclEdge {
                    id: format!("shacl-edge:{card_id}|class|{claimant}"),
                    from_id: card_id.to_owned(),
                    to_id: claimant.clone(),
                    kind: "class".to_owned(),
                    label: row_path.to_owned(),
                });
                return String::new();
            }
            format!("class {}", display(model, &class.iri))
        }
        (shacl::NODE, RdfTerm::Iri(_)) => {
            if let Some(node_key) = key_of(value) {
                let to = id_of(&node_key);
                edges.push(ShaclEdge {
                    id: format!("shacl-edge:{card_id}|node|{to}"),
                    from_id: card_id.to_owned(),
                    to_id: to,
                    kind: "node".to_owned(),
                    label: row_path.to_owned(),
                });
            }
            String::new()
        }
        (shacl::NODE, _) => format!("node {}", inline_summary(model, context, value, 0)),
        (shacl::DATATYPE, RdfTerm::Iri(datatype)) => display(model, &datatype.iri),
        (shacl::NODE_KIND, RdfTerm::Iri(kind)) => match kind.iri.strip_prefix(shacl::NAMESPACE) {
            Some(local) => local.to_owned(),
            None => display(model, &kind.iri),
        },
        (shacl::IN, _) => format!("in ({})", context.list_member_displays(model, value)),
        (shacl::HAS_VALUE, _) => format!("has {}", term_display(model, value)),
        (shacl::QUALIFIED_VALUE_SHAPE, _) => format!(
            "qualified {}",
            if matches!(value, RdfTerm::Iri(_)) {
                term_display(model, value)
            } else {
                inline_summary(model, context, value, 0)
            }
        ),
        (shacl::SPARQL, _) => String::new(),
        _ => {
            let local = &parameter[shacl::NAMESPACE.len()..];
            format!("{local} {}", term_display(model, value))
        }
    }
}

fn combinator_row(
    model: &RdfModel,
    context: &Context<'_>,
    list_head: &RdfTerm,
    card_id: &str,
    kind: &str,
    rows: &mut Vec<ShaclRow>,
    edges: &mut Vec<ShaclEdge>,
) {
    let mut has_blank_operand = false;
    for member in context.list_members(list_head) {
        if let (RdfTerm::Iri(_), Some(key)) = (member, key_of(member)) {
            let to = id_of(&key);
            edges.push(ShaclEdge {
                id: format!("shacl-edge:{card_id}|{kind}|{to}"),
                from_id: card_id.to_owned(),
                to_id: to,
                kind: kind.to_owned(),
                label: kind.to_owned(),
            });
        } else {
            has_blank_operand = true;
        }
    }
    if has_blank_operand {
        rows.push(ShaclRow {
            path: String::new(),
            name: String::new(),
            summary: format!("{kind}({})", operand_list(model, context, shacl::AND, list_head)),
            cardinality: String::new(),
            sparql: false,
            blank: true,
            severity: String::new(),
        });
    }
}

fn operand_list(model: &RdfModel, context: &Context<'_>, combinator: &str, value: &RdfTerm) -> String {
    let operands: Vec<&RdfTerm> = if combinator == shacl::NOT {
        vec![value]
    } else {
        context.list_members(value)
    };
    operands
        .iter()
        .map(|operand| match operand {
            RdfTerm::Iri(iri) => display(model, &iri.iri),
            _ => inline_summary(model, context, operand, 0),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// A blank operand's one-level constraint summary; anything deeper elides to an ellipsis.
fn inline_summary(model: &RdfModel, context: &Context<'_>, operand: &RdfTerm, depth: usize) -> String {
    if depth > 0 {
        return "…".to_owned();
    }
    let Some(key) = key_of(operand) else {
        return "…".to_owned();
    };
    let mut parts = Vec::new();
    for (triple, _) in context.triples_of(&key) {
        let predicate = triple.predicate.iri.as_str();
        if predicate == rdf::TYPE {
            continue;
        }
        let Some(local) = predicate.strip_prefix(shacl::NAMESPACE) else {
            continue;
        };
        let value = match &triple.object {
            RdfTerm::Iri(_) | RdfTerm::Literal(_) => term_display(model, &triple.object),
            RdfTerm::Blank(_) => "…".to_owned(),
        };
        parts.push(format!("{local} {value}"));
    }
    format!("{{ {} }}", parts.join(", "))
}

/// A `sh:path` value printed as SHACL path syntax.
fn path_syntax(model: &RdfModel, context: &Context<'_>, path: &RdfTerm, nested: bool) -> String {
    if let RdfTerm::Iri(iri) = path {
        return display(model, &iri.iri);
    }
    let Some(key) = key_of(path) else {
        return String::new();
    };
    if let Some(inverse) = context.first_object(&key, shacl::INVERSE_PATH) {
        return format!("^{}", path_syntax(model, context, inverse, true));
    }
    if let Some(alternatives) = context.first_object(&key, shacl::ALTERNATIVE_PATH) {
        let joined = context
            .list_members(alternatives)
            .iter()
            .map(|m| path_syntax(model, context, m, true))
            .collect::<Vec<_>>()
            .join("|");
        return if nested { format!("({joined})") } else { joined };
    }
    if let Some(inner) = context.first_object(&key, shacl::ZERO_OR_MORE_PATH) {
        return format!("{}*", path_syntax(model, context, inner, true));
    }
    if let Some(inner) = context.first_object(&key, shacl::ONE_OR_MORE_PATH) {
        return format!("{}+", path_syntax(model, context, inner, true));
    }
    if let Some(inner) = context.first_object(&key, shacl::ZERO_OR_ONE_PATH) {
        return format!("{}?", path_syntax(model, context, inner, true));
    }
    if context.first_object(&key, rdf::FIRST).is_some() {
        let joined = context
            .list_members(path)
            .iter()
            .map(|m| path_syntax(model, context, m, true))
            .collect::<Vec<_>>()
            .join("/");
        return if nested { format!("({joined})") } else { joined };
    }
    String::new()
}

fn term_display(model: &RdfModel, term: &RdfTerm) -> String {
    match term {
        RdfTerm::Iri(iri) => display(model, &iri.iri),
        RdfTerm::Literal(literal) => literal.lexical.clone(),
        RdfTerm::Blank(_) => "…".to_owned(),
    }
}

fn kind_of(combinator: &str) -> &str {
    &combinator[shacl::NAMESPACE.len()..]
}

/// The one-pass indexes the projection reads (`ProjectionContext`).
pub struct Context<'a> {
    by_subject: IndexMap<String, Vec<(&'a RdfTriple, usize)>>,
    pub subject_keys: HashSet<String>,
    property_referenced: HashSet<String>,
    node_referenced: HashSet<String>,
    operand_referenced: HashSet<String>,
}

impl<'a> Context<'a> {
    pub fn build(model: &'a RdfModel, shapes: &[ShaclShape]) -> Self {
        let mut context = Context {
            by_subject: IndexMap::new(),
            subject_keys: HashSet::new(),
            property_referenced: HashSet::new(),
            node_referenced: HashSet::new(),
            operand_referenced: HashSet::new(),
        };
        for (index, triple) in model.triples.iter().enumerate() {
            if let Some(subject_key) = key_of(&triple.subject) {
                context.subject_keys.insert(subject_key.clone());
                context.by_subject.entry(subject_key).or_default().push((triple, index));
            }
            let Some(object_key) = key_of(&triple.object) else {
                continue;
            };
            match triple.predicate.iri.as_str() {
                shacl::PROPERTY => {
                    context.property_referenced.insert(object_key);
                }
                shacl::NODE => {
                    context.node_referenced.insert(object_key);
                }
                shacl::NOT | shacl::QUALIFIED_VALUE_SHAPE => {
                    context.operand_referenced.insert(object_key);
                }
                _ => {}
            }
        }
        let mut operands = Vec::new();
        for shape in shapes {
            for (triple, _) in context.triples_of(&shape.key) {
                if !shacl::SHAPE_LIST_PREDICATES.contains(&triple.predicate.iri.as_str()) {
                    continue;
                }
                for member in context.list_members(&triple.object) {
                    if let Some(key) = key_of(member) {
                        operands.push(key);
                    }
                }
            }
        }
        context.operand_referenced.extend(operands);
        context
    }

    pub fn triples_of(&self, subject_key: &str) -> &[(&'a RdfTriple, usize)] {
        self.by_subject.get(subject_key).map_or(&[], Vec::as_slice)
    }

    pub fn objects(&self, subject_key: &str, predicate: &str) -> Vec<&'a RdfTerm> {
        self.triples_of(subject_key)
            .iter()
            .filter(|(t, _)| t.predicate.iri == predicate)
            .map(|(t, _)| &t.object)
            .collect()
    }

    pub fn first_object(&self, subject_key: &str, predicate: &str) -> Option<&'a RdfTerm> {
        self.triples_of(subject_key)
            .iter()
            .find(|(t, _)| t.predicate.iri == predicate)
            .map(|(t, _)| &t.object)
    }

    pub fn first_literal(&self, subject_key: &str, predicate: &str) -> String {
        self.first_object(subject_key, predicate)
            .and_then(RdfTerm::as_literal)
            .map(|l| l.lexical.clone())
            .unwrap_or_default()
    }

    pub fn list_members(&self, head: &RdfTerm) -> Vec<&'a RdfTerm> {
        let mut members = Vec::new();
        let mut visited = HashSet::new();
        let mut current: Option<RdfTerm> = Some(head.clone());
        while let Some(term) = current.take() {
            if term.is_iri(rdf::NIL) {
                break;
            }
            let Some(key) = key_of(&term) else {
                break;
            };
            if !visited.insert(key.clone()) {
                break;
            }
            if let Some(member) = self.first_object(&key, rdf::FIRST) {
                members.push(member);
            }
            let Some(rest) = self.first_object(&key, rdf::REST) else {
                break;
            };
            current = Some(rest.clone());
        }
        members
    }

    fn list_member_displays(&self, model: &RdfModel, head: &RdfTerm) -> String {
        self.list_members(head)
            .iter()
            .map(|m| term_display(model, m))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn targeted_classes(&self, shape: &ShaclShape) -> Vec<String> {
        let mut classes: Vec<String> = self
            .triples_of(&shape.key)
            .iter()
            .filter(|(t, _)| t.predicate.iri == shacl::TARGET_CLASS)
            .filter_map(|(t, _)| t.object.as_iri().map(str::to_owned))
            .collect();
        if shape.implicit_class_target
            && let Some(iri) = shape.term.as_iri()
        {
            classes.push(iri.to_owned());
        }
        classes
    }
}
