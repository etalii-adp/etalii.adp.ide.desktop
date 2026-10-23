//! The declarations of a description, by namespace (ATD §14.2).

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value as Json;

use crate::description::Description;

/// An element or relation type.
#[derive(Debug, Clone)]
pub struct TypeDecl {
    pub node: Json,
    pub relation: bool,
    pub extends: Vec<String>,
    /// Its own attributes, by name.
    pub attributes: BTreeMap<String, Json>,
}

/// A binding and its rules, blocks and sections by name.
#[derive(Debug, Clone)]
pub struct BindingDecl {
    pub node: Json,
    pub rules: BTreeMap<String, Json>,
    pub blocks: BTreeSet<String>,
}

/// Everything a description declares.
#[derive(Debug, Clone, Default)]
pub struct Symbols {
    pub types: BTreeMap<String, TypeDecl>,
    pub enums: BTreeMap<String, Json>,
    pub datatypes: BTreeMap<String, Json>,
    pub document: BTreeMap<String, Json>,
    pub functions: BTreeMap<String, Json>,
    pub operations: BTreeMap<String, Json>,
    pub reasons: BTreeMap<String, Json>,
    pub forms: BTreeMap<String, Json>,
    pub styles: BTreeSet<String>,
    pub algorithms: BTreeSet<String>,
    pub bindings: BTreeMap<String, BindingDecl>,
    /// The capabilities the description requires, by name.
    pub required: BTreeMap<String, Json>,
    /// Every declaration by namespace, for duplicates.
    pub declared: Vec<(&'static str, String, Json)>,
}

/// Built-in fields of every element (DISL §2.2's reserved names), readable in CEL.
pub const BUILT_IN_FIELDS: &[&str] = &[
    "id",
    "type",
    "kind",
    "parent",
    "children",
    "descendants",
    "ancestors",
    "incoming",
    "outgoing",
    "source",
    "target",
    "sourcePort",
    "targetPort",
    "ports",
    "owner",
    "view",
    "diagram",
];

impl Symbols {
    /// The attributes of a type, its supertypes' included, by name.
    pub fn attributes_of(&self, name: &str) -> BTreeMap<String, Json> {
        let mut out = BTreeMap::new();
        let mut seen = BTreeSet::new();
        let mut todo = vec![name.to_owned()];
        while let Some(current) = todo.pop() {
            if !seen.insert(current.clone()) {
                continue;
            }
            if let Some(decl) = self.types.get(&current) {
                for (k, v) in &decl.attributes {
                    out.entry(k.clone()).or_insert_with(|| v.clone());
                }
                todo.extend(decl.extends.iter().cloned());
            }
        }
        out
    }

    pub fn is_type(&self, name: &str) -> bool {
        self.types.contains_key(name)
    }
}

fn text(name: &Json) -> String {
    name["text"].as_str().unwrap_or("").to_owned()
}

/// Collects the declarations of every file of a description.
pub fn collect(description: &Description) -> Symbols {
    let mut symbols = Symbols::default();
    for member in description.members() {
        collect_member(member, &mut symbols);
    }
    symbols
}

fn collect_member(member: &Json, symbols: &mut Symbols) {
    match member["kind"].as_str().unwrap_or("") {
        "model" => {
            for item in member["members"].as_array().into_iter().flatten() {
                match item["kind"].as_str().unwrap_or("") {
                    "element" | "relation" => {
                        let name = text(&item["name"]);
                        let attributes = item["members"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter(|m| m["kind"] == "attribute")
                            .map(|m| (text(&m["name"]), m.clone()))
                            .collect();
                        let extends = item["extends"].as_array().into_iter().flatten().map(text).collect();
                        symbols.declared.push(("type", name.clone(), item["name"].clone()));
                        symbols.types.insert(
                            name,
                            TypeDecl {
                                node: item.clone(),
                                relation: item["kind"] == "relation",
                                extends,
                                attributes,
                            },
                        );
                    }
                    "enum" => {
                        let name = text(&item["name"]);
                        symbols.declared.push(("type", name.clone(), item["name"].clone()));
                        symbols.enums.insert(name, item.clone());
                    }
                    "datatype" => {
                        let name = text(&item["name"]);
                        symbols.declared.push(("type", name.clone(), item["name"].clone()));
                        symbols.datatypes.insert(name, item.clone());
                    }
                    "document" => {
                        for attribute in item["members"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter(|m| m["kind"] == "attribute")
                        {
                            symbols.document.insert(text(&attribute["name"]), attribute.clone());
                        }
                    }
                    _ => {}
                }
            }
        }
        "fn" => {
            let name = text(&member["name"]);
            symbols
                .declared
                .push(("function", name.clone(), member["name"].clone()));
            symbols.functions.insert(name, member.clone());
        }
        "operation" => {
            let name = text(&member["name"]);
            symbols
                .declared
                .push(("operation", name.clone(), member["name"].clone()));
            symbols.operations.insert(name, member.clone());
        }
        "reason" => {
            let name = text(&member["name"]);
            symbols.declared.push(("reason", name.clone(), member["name"].clone()));
            symbols.reasons.insert(name, member.clone());
        }
        "form" => {
            let name = text(&member["name"]);
            symbols.declared.push(("form", name.clone(), member["name"].clone()));
            symbols.forms.insert(name, member.clone());
        }
        "constraint" => symbols
            .declared
            .push(("constraint", text(&member["name"]), member["name"].clone())),
        "constraints" => {
            for item in member["members"].as_array().into_iter().flatten() {
                collect_member(item, symbols);
            }
        }
        "notation" => {
            for item in member["members"].as_array().into_iter().flatten() {
                if item["kind"] == "style" {
                    symbols.styles.insert(text(&item["name"]));
                }
            }
        }
        "layout" => {
            for item in member["members"].as_array().into_iter().flatten() {
                if item["kind"] == "algorithm" {
                    symbols.algorithms.insert(text(&item["name"]));
                }
            }
        }
        "requires" => {
            symbols.required.insert(text(&member["capability"]), member.clone());
        }
        "binding" => {
            let name = text(&member["name"]);
            let mut rules = BTreeMap::new();
            let mut blocks = BTreeSet::new();
            for item in member["members"].as_array().into_iter().flatten() {
                match item["kind"].as_str().unwrap_or("") {
                    "rule" => {
                        let rule_name = match item.get("as") {
                            Some(alias) => text(alias),
                            None => text(&item["type"]["name"]),
                        };
                        rules.insert(rule_name, item.clone());
                    }
                    "block" | "section" => {
                        blocks.insert(text(&item["name"]));
                    }
                    _ => {}
                }
            }
            symbols.declared.push(("binding", name.clone(), member["name"].clone()));
            symbols.bindings.insert(
                name,
                BindingDecl {
                    node: member.clone(),
                    rules,
                    blocks,
                },
            );
        }
        _ => {}
    }
}
