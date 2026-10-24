//! A [`DislChange`] as the FBL change that writes it (the standalone host's `DislWrite`): the metamodel's types and
//! attribute names mapped back through `persistence.typeMap`, each value in its stored form - a `yearMonth` as
//! `±YYYY-MM`, an enum value as its stored form, an element as its id - and every id as the binding reads it.
//!
//! A reparent is written as an FBL move and a retype as an FBL retype, which only a persistence plugin writes. An id
//! is written as it was read: under the `natural` strategy an element's id is the prefix its type declares followed
//! by the id the binding read (section 11.5.1), so the prefix is taken off again wherever an id is written. A
//! connect is written as an add of the relation, with its ends in the attributes `persistence.typeMap` maps to
//! `source` and `target`, or as `source` and `target` where it maps none.

use adp_cel::{Map, Value};
use adp_fbl::ModelChange;

use super::change::DislChange;
use crate::expressions::year_month;
use crate::loading::specification::DislSpecification;
use crate::model::element::DislElement;
use crate::model::type_map::TypeMap;

/// The FBL change that writes `change`.
pub fn to_fbl(specification: &DislSpecification, change: &DislChange) -> ModelChange {
    let map = TypeMap::of(specification);
    match change {
        DislChange::Create {
            type_name,
            id,
            attributes,
            parent_id,
            index,
            ..
        } => ModelChange::Add {
            type_name: map.binding_type_of(type_name),
            id: Some(read_id(specification, type_name, id)),
            attributes: written(specification, &map, type_name, attributes),
            parent_id: parent_id.as_ref().map(|p| read_id_of_any(specification, p)),
            index: *index,
            slot: None,
        },
        DislChange::Connect {
            type_name,
            id,
            source_id,
            target_id,
            attributes,
        } => {
            let mut written = written(specification, &map, type_name, attributes);
            written.insert(
                map.binding_attribute_of(type_name, "source"),
                Value::String(read_id_of_any(specification, source_id)),
            );
            written.insert(
                map.binding_attribute_of(type_name, "target"),
                Value::String(read_id_of_any(specification, target_id)),
            );
            ModelChange::Add {
                type_name: map.binding_type_of(type_name),
                id: Some(read_id(specification, type_name, id)),
                attributes: written,
                parent_id: None,
                index: None,
                slot: None,
            }
        }
        DislChange::Set {
            element_id,
            type_name,
            attributes,
        } => ModelChange::Set {
            id: read_id(specification, type_name, element_id),
            attributes: written(specification, &map, type_name, attributes),
        },
        DislChange::Remove { element_id } => ModelChange::Remove {
            id: read_id_of_any(specification, element_id),
        },
        DislChange::Reparent {
            element_id,
            parent_id,
            index,
        } => ModelChange::Move {
            id: read_id_of_any(specification, element_id),
            new_parent_id: parent_id.as_ref().map(|p| read_id_of_any(specification, p)),
            index: *index,
        },
        DislChange::Retype {
            element_id,
            type_name,
            attributes,
        } => ModelChange::Retype {
            id: read_id_of_any(specification, element_id),
            type_name: map.binding_type_of(type_name),
            attributes: written(specification, &map, type_name, attributes),
        },
    }
}

/// The stored form of `value` for `attribute` of `type_name`.
pub fn stored_form(specification: &DislSpecification, type_name: &str, attribute: &str, value: &Value) -> Value {
    match specification
        .metamodel()
        .type_of(type_name)
        .and_then(|t| t.attributes.get(attribute))
    {
        Some(definition) => stored(specification, &definition.type_name, value),
        None => value.clone(),
    }
}

/// The id `id`, an element of `type_name`, was read with: without the prefix the type's `natural` id rule declares.
fn read_id(specification: &DislSpecification, type_name: &str, id: &str) -> String {
    let rule = specification.id_rule_of(type_name);
    match rule.prefix {
        Some(prefix) if rule.strategy == "natural" && !prefix.is_empty() && id.starts_with(&prefix) => {
            id[prefix.len()..].to_owned()
        }
        _ => id.to_owned(),
    }
}

/// The id `id` was read with, for an element whose type the change does not say: without the longest prefix a
/// type's `natural` id rule declares that it starts with.
fn read_id_of_any(specification: &DislSpecification, id: &str) -> String {
    let metamodel = specification.metamodel();
    let prefix = metamodel
        .types
        .keys()
        .chain(metamodel.relations.keys())
        .map(|t| specification.id_rule_of(t))
        .filter(|r| r.strategy == "natural")
        .filter_map(|r| r.prefix)
        .filter(|p| !p.is_empty() && id.starts_with(p.as_str()))
        .max_by_key(String::len);
    match prefix {
        Some(prefix) => id[prefix.len()..].to_owned(),
        None => id.to_owned(),
    }
}

fn written(specification: &DislSpecification, map: &TypeMap, type_name: &str, attributes: &Map) -> Map {
    attributes
        .iter()
        .map(|(name, value)| {
            (
                map.binding_attribute_of(type_name, name),
                stored_form(specification, type_name, name, value),
            )
        })
        .collect()
}

fn stored(specification: &DislSpecification, type_name: &str, value: &Value) -> Value {
    if let Some(element) = DislElement::from_value(value) {
        return Value::String(read_id(specification, &element.type_name(), &element.id()));
    }
    let metamodel = specification.metamodel();
    match value {
        Value::Null => Value::Null,
        Value::List(list) => Value::list(list.iter().map(|i| stored(specification, type_name, i)).collect()),
        Value::String(id) if metamodel.type_of(type_name).is_some() => {
            Value::String(read_id(specification, type_name, id))
        }
        Value::Int(month) if type_name == "yearMonth" => {
            Value::String(year_month::format(*month, "uuuu-MM").unwrap_or_default())
        }
        Value::String(key) => match metamodel.enums.get(type_name) {
            Some(enumeration) => Value::String(
                enumeration
                    .value_of(key)
                    .map(|v| v.stored.clone())
                    .unwrap_or_else(|| key.clone()),
            ),
            None => value.clone(),
        },
        other => other.clone(),
    }
}
