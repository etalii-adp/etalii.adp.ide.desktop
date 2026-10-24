//! The construct of a description that produced a value (ATD §2.3, `ATD9001`): each difference of a case is
//! attributed to the construct a tool engineer would read first to change it.
//!
//! - a gesture's value, to its operation; an action the description names only by its kind (`delete`), to the
//!   `behavior` block; a property set or a node moved, to the binding's rule for the element's type;
//! - a refusal, to the constraint whose message it is, else to the constraint of the gesture's kind (`delete` for a
//!   deletion, `containment` for a move), else to the gesture's construct;
//! - the reading, to the binding's rule for the type of the first element that differs, else to the binding;
//! - a finding, to the constraint that has its code;
//! - a drawing, to the notation's `node` or `edge` for the drawn element's type, else to the notation.

use std::collections::BTreeMap;

use adp_atd_check::Description;
use serde_json::Value as Json;

/// Where a construct is written: its file, as the description names it, and its span there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct At {
    pub file: String,
    /// Byte offsets in the file.
    pub start: usize,
    pub end: usize,
    /// From 1, as the description's spans count them.
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

impl At {
    fn of(node: &Json) -> Option<At> {
        let span = node.get("span")?;
        let number = |key: &str| span[key].as_u64().unwrap_or(0) as usize;
        Some(At {
            file: span["file"].as_str()?.to_owned(),
            start: number("start"),
            end: number("end"),
            line: number("line"),
            column: number("column"),
            end_line: number("endLine"),
            end_column: number("endColumn"),
        })
    }
}

impl std::fmt::Display for At {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}

/// The constructs of a description a difference can be attributed to, by what names them.
#[derive(Debug, Default)]
pub struct Constructs {
    operations: BTreeMap<String, At>,
    behavior: Option<At>,
    binding: Option<At>,
    /// The binding's rules, by the type they read.
    rules: BTreeMap<String, At>,
    /// Constraints and built-in constraints, by their code.
    codes: BTreeMap<String, At>,
    /// Constraints by their literal message.
    messages: BTreeMap<String, At>,
    /// Constraints by their `kind` setting (`delete`, `containment`).
    kinds: BTreeMap<String, At>,
    notation: Option<At>,
    /// The notation's `node` and `edge` entries, by the types they draw.
    drawn: BTreeMap<String, At>,
    /// The description's name of each action, by the wire id that names it in the cases.
    wire_actions: BTreeMap<String, String>,
}

impl Constructs {
    /// The constructs of `description`, with the wire ids its lowered specification `lowered` maps its actions to.
    pub fn of(description: &Description, lowered: &Json) -> Constructs {
        let mut constructs = Constructs::default();
        for file in &description.files {
            if let Some(construct) = file.construct() {
                constructs.walk(construct, false);
            }
        }
        let wire = lowered
            .as_object()
            .into_iter()
            .flatten()
            .find(|(name, block)| name.starts_with("x-") && block.get("actions").is_some());
        if let Some((_, block)) = wire {
            for (name, id) in block["actions"].as_object().into_iter().flatten() {
                if let Some(id) = id.as_str() {
                    constructs.wire_actions.insert(id.to_owned(), name.clone());
                }
            }
        }
        constructs
    }

    fn walk(&mut self, node: &Json, in_binding: bool) {
        let name = node["name"]["text"].as_str();
        let at = At::of(node);
        let mut in_binding = in_binding;
        match (node["kind"].as_str(), at) {
            (Some("operation"), Some(at)) => {
                if let Some(name) = name {
                    self.operations.entry(name.to_owned()).or_insert(at);
                }
            }
            (Some("behavior"), Some(at)) => {
                self.behavior.get_or_insert(at);
            }
            (Some("binding"), Some(at)) => {
                self.binding.get_or_insert(at);
                in_binding = true;
            }
            (Some("rule"), Some(at)) if in_binding => {
                if let Some(type_name) = node["type"]["name"]["text"].as_str() {
                    self.rules.entry(type_name.to_owned()).or_insert(at);
                }
            }
            (Some("constraint" | "builtin"), Some(at)) => {
                let settings = node["settings"].as_array().into_iter().flatten();
                for setting in settings {
                    let value = setting["value"]["literal"].as_str();
                    match (setting["key"].as_str(), value) {
                        (Some("code"), Some(code)) => {
                            self.codes.entry(code.to_owned()).or_insert(at.clone());
                        }
                        (Some("kind"), Some(kind)) => {
                            self.kinds.entry(kind.to_owned()).or_insert(at.clone());
                        }
                        _ => {}
                    }
                }
                if let Some(message) = node["message"]["literal"].as_str() {
                    self.messages.entry(message.to_owned()).or_insert(at);
                }
            }
            (Some("notation"), Some(at)) => {
                self.notation.get_or_insert(at);
            }
            (Some("node" | "edge"), Some(at)) => {
                for drawn in node["names"].as_array().into_iter().flatten() {
                    if let Some(type_name) = drawn["text"].as_str() {
                        self.drawn.entry(type_name.to_owned()).or_insert(at.clone());
                    }
                }
            }
            _ => {}
        }
        for member in node["members"].as_array().into_iter().flatten() {
            self.walk(member, in_binding);
        }
    }

    /// The construct of a gesture: its operation, the `behavior` block for a deletion, else the binding's rule for
    /// the type of the element it changed (`element_type`: the property's, the moved one's, the selection's), else
    /// the binding.
    pub(crate) fn gesture(&self, step: &Json, element_type: Option<&str>) -> Option<At> {
        let action = step.get("action").and_then(Json::as_str);
        let name = action.map(|a| self.wire_actions.get(a).map_or(a, String::as_str));
        name.and_then(|n| self.operations.get(n))
            .or_else(|| (name == Some("delete")).then_some(self.behavior.as_ref()).flatten())
            .cloned()
            .or_else(|| self.rule(element_type))
    }

    /// The construct of a refusal: the constraint whose message is one of `reasons`, else the constraint of the
    /// gesture's kind, else the gesture's construct.
    pub(crate) fn refusal(&self, step: &Json, reasons: &[&str], element_type: Option<&str>) -> Option<At> {
        if let Some(at) = reasons.iter().find_map(|r| self.messages.get(*r)) {
            return Some(at.clone());
        }
        let kind = if step.get("move").is_some() {
            Some("containment")
        } else {
            step.get("action")
                .and_then(Json::as_str)
                .map(|a| self.wire_actions.get(a).map_or(a, String::as_str))
                .filter(|name| *name == "delete")
        };
        kind.and_then(|k| self.kinds.get(k))
            .cloned()
            .or_else(|| self.gesture(step, element_type))
    }

    /// The binding's rule for `type_name`, else the binding.
    pub(crate) fn rule(&self, type_name: Option<&str>) -> Option<At> {
        type_name
            .and_then(|t| self.rules.get(t))
            .or(self.binding.as_ref())
            .cloned()
    }

    /// The constraint that has finding `code`.
    pub(crate) fn code(&self, code: Option<&str>) -> Option<At> {
        code.and_then(|c| self.codes.get(c)).cloned()
    }

    /// The notation's entry for `type_name`, else the notation.
    pub(crate) fn drawn(&self, type_name: Option<&str>) -> Option<At> {
        type_name
            .and_then(|t| self.drawn.get(t))
            .or(self.notation.as_ref())
            .cloned()
    }
}
