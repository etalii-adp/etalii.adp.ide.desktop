//! Builds the DISL model of a format binding's reading (DISL sections 4.11, 11.5 and 14.2; the standalone host's
//! `DislModelBuilder`): the stored elements as the binding read them, typed by the metamodel, then the derived ids
//! and the derived relations.
//!
//! The binding's types are mapped by the specification's `persistence.typeMap` when it has one: a type `as` a
//! metamodel type (with `attributes` renamed, `source` and `target` taking a relation's ends, and `hostAttributes`
//! kept beside the model), `as` `diagram` for diagram attributes, `as` `header` or `unreadable` to keep the element
//! aside. A value is typed by its attribute; one that does not fit stays unset, so it reads as its default. Under
//! the `natural` strategy a type's `prefix` is applied (sections 11.5 and 11.5.1): an element of a type that
//! declares one has that prefix followed by its natural id - the id a node was read with, or for a relation read
//! without an id its ends as read joined by `|` - and an element of a type that declares none keeps the id it was
//! read with. A relation's ends are found by the ids they were read with, and kept as read; a reference attribute
//! is stored as the model id of the node it names, so it resolves under a prefix too.

use std::collections::HashMap;

use adp_cel::{Map, Value};
use adp_fbl::model::codes;
use adp_fbl::{FblElement, FblModel, Finding, FindingSeverity};

use super::diagram::DislDiagram;
use super::element::{DislElement, NewElement};
use super::type_map::{TypeMap, TypeMapping};
use super::values;
use crate::expressions::contexts;
use crate::json;
use crate::loading::metamodel::{DislMetamodel, DislRelationEnd, DislType};
use crate::loading::specification::DislSpecification;

/// Two derived elements with one id, or a derived id equal to a stored one (section 4.11.5).
pub const DERIVED_ID: &str = "std.derivedId";

/// Items of a derived relation type whose ends its declaration does not allow (section 4.11.3).
const DERIVED_ENDS: &str = "std.derivedEnds";

/// A derived type whose `from` or per-item expressions failed (section 4.11.5).
pub const DERIVED_FAILED: &str = "std.derivedFailed";

/// What a model built from a reading holds: the diagram, every finding (the reader's first, in reading order, then
/// the runtime's, section 8.6), and the elements kept aside - the header and the entries the reader could not read.
#[derive(Debug, Clone)]
pub struct DislModel {
    pub diagram: DislDiagram,
    pub findings: Vec<Finding>,
    pub header: Vec<FblElement>,
    pub unreadable: Vec<FblElement>,
    /// Whether the body could not be read at all; the diagram is then empty.
    pub is_unreadable: bool,
}

fn finding(code: &str, message: String) -> Finding {
    Finding {
        code: code.to_owned(),
        severity: FindingSeverity::Warning,
        message,
        location: None,
        detail: None,
    }
}

/// The file a folder subject's plugin read the element in (FBL section 11.2); none for an element of the primary
/// file.
fn file_of(element: &FblElement) -> Option<String> {
    element.file.clone().filter(|f| !f.is_empty())
}

fn line_of(element: &FblElement) -> Option<i32> {
    i32::try_from(element.line).ok()
}

/// The model of `reading` under `specification`.
pub fn from(reading: &FblModel, specification: &DislSpecification) -> DislModel {
    let diagram = DislDiagram::new(specification);
    let mut findings = reading.findings.clone();
    let mut header = Vec::new();
    let mut unreadable = Vec::new();
    if reading.unreadable {
        return DislModel {
            diagram,
            findings,
            header,
            unreadable,
            is_unreadable: true,
        };
    }

    let map = TypeMap::of(specification);
    let metamodel = specification.metamodel();
    let mut pending: Vec<(usize, TypeMapping, std::rc::Rc<DislType>)> = Vec::new();

    for (index, element) in reading.elements.iter().enumerate() {
        let mapping = map.mapping(&element.type_name);
        match mapping.as_type.as_str() {
            "header" => {
                header.push(element.clone());
                continue;
            }
            "diagram" => {
                for (name, raw) in element.attributes.iter() {
                    let target = mapping.renamed(name);
                    if let Some(attribute) = metamodel.diagram_attributes.get(target)
                        && let Some(value) = values::from_read(raw, attribute, metamodel)
                    {
                        diagram.set_attribute(target, value);
                    }
                }
                continue;
            }
            _ => {}
        }
        match metamodel.type_of(&mapping.as_type) {
            Some(t) if mapping.as_type != "unreadable" && !t.is_abstract() => {
                pending.push((index, mapping.clone(), t.clone()));
            }
            _ => unreadable.push(element.clone()),
        }
    }

    let mut builder = Builder {
        reading,
        specification,
        metamodel,
        diagram: &diagram,
        pending: &pending,
        built: HashMap::new(),
        by_read_id: HashMap::new(),
    };

    // Nodes in reading order, each after its parent; then relations, whose ends are nodes.
    for (position, (_, _, t)) in pending.iter().enumerate() {
        if !t.is_relation {
            builder.node(position);
        }
    }
    for (index, mapping, t) in pending.iter().filter(|(_, _, t)| t.is_relation) {
        let element = &reading.elements[*index];
        let (attributes, host, ends) = attributes(element, mapping, t, metamodel);
        let source_id = ends.get("source").cloned().flatten().or_else(|| element.source.clone());
        let target_id = ends.get("target").cloned().flatten().or_else(|| element.target.clone());
        let mut new = NewElement::new(
            &t.name,
            &natural_id(specification, t, element, source_id.as_deref(), target_id.as_deref()),
        );
        new.attributes = attributes;
        new.source = builder.end(source_id.as_deref());
        new.target = builder.end(target_id.as_deref());
        new.source_id = source_id;
        new.target_id = target_id;
        new.host_attributes = host;
        new.line = line_of(element);
        new.file = file_of(element);
        new.id_is_stored = element.id_is_stored;
        if let Ok(made) = DislElement::make(&diagram, &new, t.clone()) {
            let _ = diagram.add_made(made);
        }
    }

    // A reference attribute holds the id its target was read with, which under a prefix is not the target's model
    // id: it is stored as the model id of the first node read with it (section 11.5.4), and one that names nothing
    // stays as read.
    let model_id = |id: &str| {
        builder
            .by_read_id
            .get(id)
            .map(DislElement::id)
            .unwrap_or_else(|| id.to_owned())
    };
    for element in diagram.elements() {
        let element_type = element.element_type();
        for attribute in element_type.attributes.values() {
            if metamodel.type_of(&attribute.type_name).is_none() {
                continue;
            }
            let Some(stored) = element.attributes().get(&attribute.name).cloned() else {
                continue;
            };
            let value = match &stored {
                Value::String(id) => Value::String(model_id(id)),
                Value::List(ids) => Value::list(
                    ids.iter()
                        .map(|id| match id {
                            Value::String(text) => Value::String(model_id(text)),
                            other => other.clone(),
                        })
                        .collect(),
                ),
                _ => stored.clone(),
            };
            let _ = element.store(&attribute.name, value);
        }
    }

    findings.extend(complete(&diagram));
    DislModel {
        diagram,
        findings,
        header,
        unreadable,
        is_unreadable: false,
    }
}

struct Builder<'a> {
    reading: &'a FblModel,
    specification: &'a DislSpecification,
    metamodel: &'a DislMetamodel,
    diagram: &'a DislDiagram,
    pending: &'a [(usize, TypeMapping, std::rc::Rc<DislType>)],
    built: HashMap<usize, DislElement>,
    by_read_id: HashMap<String, DislElement>,
}

impl Builder<'_> {
    fn node(&mut self, position: usize) -> Option<DislElement> {
        if let Some(done) = self.built.get(&position) {
            return Some(done.clone());
        }
        let (index, mapping, t) = &self.pending[position];
        let element = &self.reading.elements[*index];
        let parent = element.parent_id.as_ref().and_then(|parent_id| {
            let owner = self
                .pending
                .iter()
                .position(|(i, _, _)| self.reading.elements[*i].id == *parent_id)?;
            if self.pending[owner].2.is_relation {
                None
            } else {
                self.node(owner)
            }
        });
        let (attributes, host, _) = attributes(element, mapping, t, self.metamodel);
        let mut new = NewElement::new(&t.name, &natural_id(self.specification, t, element, None, None));
        new.attributes = attributes;
        new.slot = if parent.is_some() {
            element.parent_slot.clone()
        } else {
            None
        };
        new.parent = parent;
        new.host_attributes = host;
        new.line = line_of(element);
        new.file = file_of(element);
        new.id_is_stored = element.id_is_stored;
        let made = DislElement::make(self.diagram, &new, t.clone()).ok()?;
        let node = self.diagram.add_made(made).ok()?;
        self.built.insert(position, node.clone());
        self.by_read_id
            .entry(element.id.clone())
            .or_insert_with(|| node.clone());
        Some(node)
    }

    /// A relation end names a stored id (an FBL reference by id) or an FBL element's id, which is the node's as it
    /// was read; the first element with it keeps it (section 11.5.4).
    fn end(&self, id: Option<&str>) -> Option<DislElement> {
        let id = id?;
        self.by_read_id
            .get(id)
            .cloned()
            .or_else(|| self.diagram.element_by_id(id))
    }
}

/// The id of `element`: under the `natural` strategy, the prefix its type declares followed by its natural id
/// (section 11.5.1) - a relation read without an id is named by its ends as read, joined by `|` - and otherwise, or
/// without a prefix, the id it was read with.
fn natural_id(
    specification: &DislSpecification,
    t: &DislType,
    element: &FblElement,
    source_id: Option<&str>,
    target_id: Option<&str>,
) -> String {
    let rule = specification.id_rule_of(&t.name);
    let (true, Some(prefix)) = (rule.strategy == "natural", rule.prefix) else {
        return element.id.clone();
    };
    if t.is_relation && !element.id_is_stored {
        format!("{prefix}{}|{}", source_id.unwrap_or(""), target_id.unwrap_or(""))
    } else {
        format!("{prefix}{}", element.id)
    }
}

/// A binding element's attributes: the metamodel's typed, the ends named, the rest kept as read.
fn attributes(
    element: &FblElement,
    mapping: &TypeMapping,
    t: &DislType,
    metamodel: &DislMetamodel,
) -> (Map, Map, HashMap<String, Option<String>>) {
    let mut attributes = Map::default();
    let mut host = Map::default();
    let mut ends = HashMap::new();
    for (name, raw) in element.attributes.iter() {
        let target = mapping.renamed(name);
        if t.is_relation && matches!(target, "source" | "target") {
            let id = match raw {
                Value::Null => None,
                Value::String(s) => Some(s.clone()),
                other => Some(other.to_string()),
            };
            ends.insert(target.to_owned(), id);
        } else if let (false, Some(attribute)) = (mapping.host_attributes.contains(name), t.attributes.get(target)) {
            if let Some(value) = values::from_read(raw, attribute, metamodel) {
                attributes.insert(target, value);
            }
        } else {
            host.insert(name.clone(), raw.clone());
        }
    }
    (attributes, host, ends)
}

/// Completes a diagram whose stored elements are in place (section 14.2): the derived ids of its nodes, ancestors
/// first, then its derived relations, then the derived ids of its stored relations. The runtime's findings, in the
/// order they arose.
pub fn complete(diagram: &DislDiagram) -> Vec<Finding> {
    let mut findings = Vec::new();
    derived_ids(diagram, &diagram.nodes(), &mut findings);
    derived_relations(diagram, &mut findings);
    let stored: Vec<DislElement> = diagram.relations().into_iter().filter(|r| !r.is_derived()).collect();
    derived_ids(diagram, &stored, &mut findings);
    findings
}

/// The ids of the `derived` strategy (section 11.5.2), in the identity context, in the order given; an id computed
/// twice is reported as `std.duplicateId` unless the specification switches that built-in off (section 8.1).
fn derived_ids(diagram: &DislDiagram, elements: &[DislElement], findings: &mut Vec<Finding>) {
    let specification = diagram.specification();
    let mut programs: HashMap<String, Result<adp_cel::Program, String>> = HashMap::new();
    let mut computed: HashMap<String, DislElement> = HashMap::new();
    let reports_duplicates = specification.is_built_in_enabled(codes::DUPLICATE_ID);
    for element in elements {
        let rule = specification.id_rule_of(&element.type_name());
        let (true, Some(expression)) = (rule.strategy == "derived", rule.expression) else {
            continue;
        };
        let program = programs
            .entry(expression.clone())
            .or_insert_with(|| specification.environment(contexts::IDENTITY, &[]).compile(&expression))
            .clone();
        let variables: HashMap<String, Value> = [
            ("self".to_owned(), element.to_value()),
            ("diagram".to_owned(), diagram.to_value()),
            ("env".to_owned(), Value::Null),
        ]
        .into_iter()
        .collect();
        let value = program.and_then(|p| p.evaluate(&variables));
        let id = match value {
            Ok(Value::String(id)) => id,
            other => {
                let why = match other {
                    Err(message) => message,
                    _ => "it did not give a string".to_owned(),
                };
                findings.push(finding(
                    codes::MISSING_ID,
                    format!("The id of this {} could not be computed: {why}", element.type_name()),
                ));
                element.set_id_is_stored(false);
                continue;
            }
        };
        element.set_id(&id);
        element.set_id_is_stored(false);
        if computed.contains_key(&id) {
            if reports_duplicates {
                findings.push(finding(
                    codes::DUPLICATE_ID,
                    format!("The id '{id}' is computed for two elements; the first keeps it."),
                ));
            }
        } else {
            computed.insert(id, element.clone());
        }
    }
}

fn evaluate(
    specification: &DislSpecification,
    pointer: &str,
    variables: &HashMap<String, Value>,
) -> Result<Value, String> {
    match specification.expression_at(pointer) {
        Some(expression) => expression.program.evaluate(variables),
        None => Err(format!("No expression was compiled at {pointer}.")),
    }
}

fn allows(end: Option<&DislRelationEnd>, value: &Value) -> Option<DislElement> {
    let element = DislElement::from_value(value)?;
    if element.is_relation() {
        return None;
    }
    let types_ok = end.is_none_or(|e| e.types.is_empty() || e.types.iter().any(|t| element.is_a(t)));
    let not_excluded = end.is_none_or(|e| !e.exclude.iter().any(|t| element.is_a(t)));
    (types_ok && not_excluded).then_some(element)
}

/// The derived relation types (section 4.11.3), in the order of `metamodel.relations`.
fn derived_relations(diagram: &DislDiagram, findings: &mut Vec<Finding>) {
    let specification = diagram.specification();
    for t in specification.metamodel().relations.values() {
        let Some(derived) = t.derived().filter(|d| d.is_object()) else {
            continue;
        };
        let pointer = json::pointer(&json::pointer("/metamodel/relations", &t.name), "derived");
        if json::has(derived, "key") {
            findings.push(finding(
                DERIVED_FAILED,
                format!(
                    "{} groups its items by key, which this runtime does not compute yet.",
                    t.name
                ),
            ));
            continue;
        }

        let base: HashMap<String, Value> = [
            ("diagram".to_owned(), diagram.to_value()),
            ("env".to_owned(), Value::Null),
        ]
        .into_iter()
        .collect();
        let items = match evaluate(&specification, &json::pointer(&pointer, "from"), &base) {
            Ok(Value::List(items)) => items,
            _ => {
                findings.push(finding(
                    DERIVED_FAILED,
                    format!("The items of {} could not be computed, so there are none.", t.name),
                ));
                continue;
            }
        };

        let mut failures: Vec<String> = Vec::new();
        let mut fail = |message: String| {
            if !failures.contains(&message) {
                failures.push(message);
            }
        };
        let mut bad_ends: Vec<(&str, usize)> = Vec::new();
        let mut bad_end = |end: &'static str| match bad_ends.iter_mut().find(|(e, _)| *e == end) {
            Some((_, count)) => *count += 1,
            None => bad_ends.push((end, 1)),
        };
        let mut repeats: HashMap<String, usize> = HashMap::new();
        for (index, item) in items.iter().enumerate() {
            let mut variables = base.clone();
            variables.insert("item".to_owned(), item.clone());
            variables.insert("group".to_owned(), Value::list(vec![item.clone()]));
            variables.insert("index".to_owned(), Value::Int(index as i64));
            let source = evaluate(&specification, &json::pointer(&pointer, "source"), &variables);
            let target = evaluate(&specification, &json::pointer(&pointer, "target"), &variables);
            let (source, target) = match (source, target) {
                (Err(e), _) | (_, Err(e)) => {
                    fail(e);
                    continue;
                }
                (Ok(s), Ok(t)) => (s, t),
            };
            let Some(from) = allows(t.source.as_ref(), &source) else {
                bad_end("source");
                continue;
            };
            let Some(to) = allows(t.target.as_ref(), &target) else {
                bad_end("target");
                continue;
            };

            let id = if json::has(derived, "id") {
                match evaluate(&specification, &json::pointer(&pointer, "id"), &variables) {
                    Ok(Value::String(written)) => written,
                    _ => {
                        fail("the id of an item is not a string".to_owned());
                        continue;
                    }
                }
            } else {
                let plain = format!("{}:{}->{}", t.name, from.id(), to.id());
                let seen = repeats.entry(plain.clone()).or_insert(0);
                *seen += 1;
                if *seen == 1 { plain } else { format!("{plain}#{seen}") }
            };

            let mut sources: Vec<DislElement> = DislElement::from_value(item).into_iter().collect();
            if json::has(derived, "sources") {
                match evaluate(&specification, &json::pointer(&pointer, "sources"), &variables) {
                    Ok(Value::List(listed)) => sources = listed.iter().filter_map(DislElement::from_value).collect(),
                    _ => {
                        fail("the sources of an item are not a list".to_owned());
                        continue;
                    }
                }
            }

            let mut attributes = Map::default();
            for (name, _) in json::members(derived, "attributes") {
                let value = evaluate(
                    &specification,
                    &json::pointer(&json::pointer(&pointer, "attributes"), name),
                    &variables,
                )
                .unwrap_or(Value::Null);
                attributes.insert(name.clone(), value);
            }

            if let Some(taken) = diagram.element_by_id(&id) {
                findings.push(finding(
                    DERIVED_ID,
                    format!("{} '{id}' has the id of {taken}; it is not drawn.", t.name),
                ));
                continue;
            }
            let mut new = NewElement::new(&t.name, &id);
            new.attributes = attributes;
            new.source_id = Some(from.id());
            new.target_id = Some(to.id());
            new.source = Some(from);
            new.target = Some(to);
            new.is_derived = true;
            new.sources = sources;
            new.id_is_stored = false;
            match DislElement::make(diagram, &new, t.clone()) {
                Ok(made) => {
                    let _ = diagram.add_made(made);
                }
                Err(e) => fail(e),
            }
        }

        for failure in failures {
            findings.push(finding(
                DERIVED_FAILED,
                format!("Items of {} were dropped: {failure}", t.name),
            ));
        }
        for (end, count) in bad_ends {
            findings.push(finding(
                DERIVED_ENDS,
                format!(
                    "{count} item(s) of {} were dropped: their {end} is not one its declaration allows.",
                    t.name
                ),
            ));
        }
    }
}
