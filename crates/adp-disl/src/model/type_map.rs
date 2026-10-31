//! The specification's `persistence.typeMap` (DISL 0.3; the standalone host's `DislTypeMap`), or the identity
//! mapping: how one binding type becomes part of the model - the metamodel type it is read as, its attributes
//! renamed, and those kept for the host - read from the binding to the model by the model builder, and back by the
//! writer.

use std::collections::HashSet;

use indexmap::IndexMap;

use crate::json;
use crate::loading::specification::DislSpecification;

/// How one binding type becomes part of the model.
#[derive(Debug, Clone)]
pub(crate) struct TypeMapping {
    /// The metamodel type, or `header`, `diagram` or `unreadable`.
    pub(crate) as_type: String,
    /// The binding's attribute names renamed to the metamodel's.
    pub(crate) attributes: IndexMap<String, String>,
    /// The binding's attributes that are the host's, not the model's.
    pub(crate) host_attributes: HashSet<String>,
}

impl TypeMapping {
    /// The metamodel's name for the binding's attribute `name`.
    pub(crate) fn renamed<'a>(&'a self, name: &'a str) -> &'a str {
        self.attributes.get(name).map(String::as_str).unwrap_or(name)
    }

    /// The binding's name for the metamodel's attribute `name`.
    pub(crate) fn binding_name<'a>(&'a self, name: &'a str) -> &'a str {
        self.attributes
            .iter()
            .find(|(_, v)| *v == name)
            .map(|(k, _)| k.as_str())
            .unwrap_or(name)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TypeMap {
    mappings: IndexMap<String, TypeMapping>,
}

impl TypeMap {
    pub(crate) fn of(specification: &DislSpecification) -> TypeMap {
        let mut mappings = IndexMap::new();
        let map = json::member(json::member(specification.root(), "persistence"), "typeMap");
        if let Some(entries) = map.as_object() {
            for (name, entry) in entries {
                let attributes = json::members(entry, "attributes")
                    .filter_map(|(k, v)| v.as_str().map(|v| (k.clone(), v.to_owned())))
                    .collect();
                mappings.insert(
                    name.clone(),
                    TypeMapping {
                        as_type: json::string_of(entry, "as").unwrap_or(name).to_owned(),
                        attributes,
                        host_attributes: json::strings_of(entry, "hostAttributes").into_iter().collect(),
                    },
                );
            }
        }
        TypeMap { mappings }
    }

    /// The mapping of binding type `type_name`.
    pub(crate) fn mapping(&self, type_name: &str) -> TypeMapping {
        self.mappings.get(type_name).cloned().unwrap_or_else(|| TypeMapping {
            as_type: type_name.to_owned(),
            attributes: IndexMap::new(),
            host_attributes: HashSet::new(),
        })
    }

    /// The binding type mapped as `diagram` whose `attributes` name diagram attribute `attribute`.
    pub(crate) fn diagram_type_holding(&self, attribute: &str) -> Option<String> {
        self.mappings
            .iter()
            .find(|(_, m)| m.as_type == "diagram" && m.attributes.values().any(|a| a == attribute))
            .map(|(k, _)| k.clone())
    }

    /// The binding types mapped as `diagram` that the type map lists before `type_name`.
    pub(crate) fn diagram_types_before(&self, type_name: &str) -> Vec<String> {
        self.mappings
            .iter()
            .take_while(|(k, _)| *k != type_name)
            .filter(|(_, m)| m.as_type == "diagram")
            .map(|(k, _)| k.clone())
            .collect()
    }

    /// The binding type metamodel type `type_name` is written as: the first mapped to it, else its own name.
    pub(crate) fn binding_type_of(&self, type_name: &str) -> String {
        self.mappings
            .iter()
            .find(|(_, m)| m.as_type == type_name)
            .map(|(k, _)| k.clone())
            .unwrap_or_else(|| type_name.to_owned())
    }

    /// The binding's name for `attribute` of metamodel type `type_name`.
    pub(crate) fn binding_attribute_of(&self, type_name: &str, attribute: &str) -> String {
        self.mapping(&self.binding_type_of(type_name))
            .binding_name(attribute)
            .to_owned()
    }
}
