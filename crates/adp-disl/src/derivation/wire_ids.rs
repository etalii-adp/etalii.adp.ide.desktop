//! A host's wire ids for a specification's names (the standalone host's `WireIdMap`), read from a top-level
//! `x-<prefix>` block (`x-abm`, `x-ghg`): what the client and the backend already call a tool, an action and a
//! property row, which DISL's own names cannot always be.
//!
//! The block's shape, every part optional: `tools`, per toolbox tool id, `{ "id": …, "drop": … }`, its palette id
//! and the action a drop runs; `actions`, per operation name or entry kind, the action id, where a key
//! `"<Type>/<operation>"` or `"<Type>/<kind>"` overrides it for entries offered on that type or its subtypes, and a
//! `<name>` in an id is replaced by the entry's binding of that name, mapped through `types` when it is a type name
//! there; `properties`, per form item key, the row id; `types`, per type name, the host's kind. A name the block
//! does not map is its own id, so a specification without a block keeps DISL's names.

use std::collections::HashMap;
use std::sync::LazyLock;

use adp_cel::{Map, Value};
use regex::Regex;

use crate::json::{self, Json};
use crate::loading::specification::DislSpecification;

static PLACEHOLDER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("<([A-Za-z_][A-Za-z0-9_]*)>").expect("the placeholder pattern compiles"));

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WireIdMap {
    tools: HashMap<String, (Option<String>, Option<String>)>,
    actions: HashMap<String, String>,
    properties: HashMap<String, String>,
    /// The host's kind of each type name (`types`).
    pub types: HashMap<String, String>,
}

fn strings(block: &Json, name: &str) -> HashMap<String, String> {
    json::members(block, name)
        .filter_map(|(k, v)| v.as_str().map(|v| (k.clone(), v.to_owned())))
        .collect()
}

impl WireIdMap {
    /// The map that maps nothing: every name is its own id.
    pub fn none() -> WireIdMap {
        WireIdMap::default()
    }

    /// Whether the map maps nothing.
    pub fn is_none(&self) -> bool {
        *self == WireIdMap::default()
    }

    /// The map in `specification`'s top-level `key` (`x-ghg`); the map that maps nothing when it has none.
    pub fn of(specification: &DislSpecification, key: &str) -> WireIdMap {
        let Some(block) = json::get(specification.root(), key).filter(|b| b.is_object()) else {
            return WireIdMap::none();
        };
        let tools = json::members(block, "tools")
            .map(|(name, tool)| {
                (
                    name.clone(),
                    (
                        json::string_of(tool, "id").map(str::to_owned),
                        json::string_of(tool, "drop").map(str::to_owned),
                    ),
                )
            })
            .collect();
        WireIdMap {
            tools,
            actions: strings(block, "actions"),
            properties: strings(block, "properties"),
            types: strings(block, "types"),
        }
    }

    /// The palette id of toolbox tool `tool`.
    pub fn tool_id(&self, tool: &str) -> String {
        self.tools
            .get(tool)
            .and_then(|(id, _)| id.clone())
            .unwrap_or_else(|| tool.to_owned())
    }

    /// The action a drop of toolbox tool `tool` runs, or none when the block names none.
    pub fn drop_action_id(&self, tool: &str) -> Option<String> {
        self.tools.get(tool).and_then(|(_, drop)| drop.clone())
    }

    /// The id of a context entry of `kind` running `operation`, offered on an element whose type linearisation is
    /// `types` (most specific first), with `bindings` for the `<name>`s in it.
    pub fn action_id(&self, types: &[String], kind: &str, operation: Option<&str>, bindings: &Map) -> String {
        for t in types {
            if let Some(operation) = operation
                && let Some(by_operation) = self.actions.get(&format!("{t}/{operation}"))
            {
                return self.substitute(by_operation, bindings);
            }
            if let Some(by_kind) = self.actions.get(&format!("{t}/{kind}")) {
                return self.substitute(by_kind, bindings);
            }
        }
        if let Some(operation) = operation
            && let Some(mapped) = self.actions.get(operation)
        {
            return self.substitute(mapped, bindings);
        }
        match self.actions.get(kind) {
            Some(for_kind) => self.substitute(for_kind, bindings),
            None => operation.unwrap_or(kind).to_owned(),
        }
    }

    /// The row id of a form item known by `key`.
    pub fn property_id(&self, key: &str) -> String {
        self.properties.get(key).cloned().unwrap_or_else(|| key.to_owned())
    }

    fn substitute(&self, id: &str, bindings: &Map) -> String {
        PLACEHOLDER
            .replace_all(id, |captures: &regex::Captures<'_>| match bindings.get(&captures[1]) {
                Some(Value::String(text)) => self.types.get(text).cloned().unwrap_or_else(|| text.clone()),
                _ => captures[0].to_owned(),
            })
            .into_owned()
    }
}
