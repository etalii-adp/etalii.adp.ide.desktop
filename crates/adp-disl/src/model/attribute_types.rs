//! The attribute types a DISL specification gives the types of a binding (the standalone host's
//! `DislAttributeTypes`), in the shapes `FblOptions::attribute_type` and `FblOptions::time_attributes` take, so that
//! FBL 0.5 section 5.2 can read each slot by its type. A binding type is resolved through `persistence.typeMap`: to
//! the metamodel type it maps to, or to the diagram's attributes, with its attributes renamed; a host attribute,
//! which has no type of its own, is a string, read as text; a type mapped to `header` or `unreadable` gives no type.

use std::collections::HashSet;
use std::rc::Rc;

use adp_fbl::FblBinding;
use adp_fbl::model::{AttributeType, Many};

use super::type_map::TypeMap;
use crate::loading::specification::DislSpecification;

/// The type of attribute `attribute` of binding type `type`, or none where the specification gives none.
pub fn of(specification: &DislSpecification) -> AttributeType {
    let specification = specification.clone();
    let map = TypeMap::of(&specification);
    Rc::new(move |type_name: &str, attribute: &str| {
        let mapping = map.mapping(type_name);
        if matches!(mapping.as_type.as_str(), "header" | "unreadable") {
            return None;
        }
        if mapping.host_attributes.contains(attribute) {
            return Some("string".to_owned());
        }
        let name = mapping.renamed(attribute);
        let metamodel = specification.metamodel();
        let attributes = if mapping.as_type == "diagram" {
            Some(&metamodel.diagram_attributes)
        } else {
            metamodel.type_of(&mapping.as_type).map(|t| &t.attributes)
        };
        attributes.and_then(|a| a.get(name)).map(|d| d.type_name.clone())
    })
}

/// Whether attribute `attribute` of binding type `type` holds a list (FBL 0.7 section 4.6), through `persistence.typeMap`.
pub fn many(specification: &DislSpecification) -> Many {
    let specification = specification.clone();
    let map = TypeMap::of(&specification);
    Rc::new(move |type_name: &str, attribute: &str| {
        let mapping = map.mapping(type_name);
        let name = mapping.renamed(attribute);
        let metamodel = specification.metamodel();
        let attributes = if mapping.as_type == "diagram" {
            Some(&metamodel.diagram_attributes)
        } else {
            metamodel.type_of(&mapping.as_type).map(|t| &t.attributes)
        };
        attributes.and_then(|a| a.get(name)).is_some_and(|d| d.many)
    })
}

/// The attributes of `binding` the specification types as a date or a date-time, keyed `rule.attribute`.
pub fn time_attributes(specification: &DislSpecification, binding: &FblBinding) -> HashSet<String> {
    let type_of = of(specification);
    binding
        .all_rules()
        .iter()
        .flat_map(|rule| {
            rule.attributes
                .iter()
                .filter(|(attribute, _)| {
                    matches!(
                        type_of(&rule.type_name, attribute).as_deref(),
                        Some("date" | "datetime")
                    )
                })
                .map(|(attribute, _)| format!("{}.{attribute}", rule.name))
                .collect::<Vec<_>>()
        })
        .collect()
}
