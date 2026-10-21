//! A Bindable value (DISL section 2.6) as a definition keeps it, and its evaluation for one element (in place of the
//! standalone host's `canvas/library/definition/binding.ts` at 8b6ccfc).
//!
//! The standalone's binding is a path into a payload its backend computed, composed by templates and parts, because
//! its library has no CEL and must not call anything. Here the runtime evaluates the specification's own expression,
//! so a binding is what the specification wrote: a literal, an attribute of `self`, or CEL. A binding that fails to
//! evaluate gives nothing, as a payload path that does not resolve does in the standalone, and what it was bound to is
//! not drawn.

use adp_cel::Value;
use adp_disl::DislElement;
use adp_disl::expressions::contexts;
use serde_json::Value as Json;

use crate::evaluation::{Evaluator, Vars};

/// What a value is bound to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Binding {
    /// A value written outright.
    Literal(String),
    /// An attribute of `self` (`{ "attribute": "name" }`).
    Attribute(String),
    /// A CEL expression (`{ "cel": "..." }`), as written.
    Cel(String),
}

impl Binding {
    /// The binding a Bindable states; none for a value that is neither a literal, an attribute nor CEL.
    pub fn of(value: &Json) -> Option<Binding> {
        match value {
            Json::String(s) => Some(Binding::Literal(s.clone())),
            Json::Number(n) => Some(Binding::Literal(n.to_string())),
            Json::Object(o) => {
                if let Some(attribute) = o.get("attribute").and_then(Json::as_str) {
                    return Some(Binding::Attribute(attribute.to_owned()));
                }
                o.get("cel")
                    .and_then(Json::as_str)
                    .map(|cel| Binding::Cel(cel.to_owned()))
            }
            _ => None,
        }
    }

    /// The bound value, evaluated in `context` (the element context unless a list's item is being read).
    pub fn value(&self, evaluator: &Evaluator, context: &str, vars: &Vars) -> Option<Value> {
        match self {
            Binding::Literal(text) => Some(Value::String(text.clone())),
            Binding::Attribute(name) => {
                let element = vars.get("self").and_then(DislElement::from_value)?;
                Some(element.value_of(name))
            }
            Binding::Cel(source) => evaluator.evaluate(context, source, vars).ok(),
        }
    }

    /// The bound value as text: a string as it is, an element by its id, anything else as CEL writes it; nothing for
    /// a binding that fails or gives null.
    pub fn text(&self, evaluator: &Evaluator, context: &str, vars: &Vars) -> Option<String> {
        match self.value(evaluator, context, vars)? {
            Value::Null => None,
            value => Some(text_of(&value)),
        }
    }

    /// The bound value as text in the element context.
    pub fn element_text(&self, evaluator: &Evaluator, vars: &Vars) -> Option<String> {
        self.text(evaluator, contexts::ELEMENT, vars)
    }

    /// The bound value as a number in the element context.
    pub fn number(&self, evaluator: &Evaluator, vars: &Vars) -> Option<f64> {
        match self.value(evaluator, contexts::ELEMENT, vars)? {
            Value::String(text) => text.trim().parse().ok(),
            value => value.as_number(),
        }
    }
}

/// A CEL value as text: a string as it is, an element by its id, any other value as CEL's `string()` writes it (the
/// runtime's `text_of`).
pub fn text_of(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        other => match DislElement::from_value(other) {
            Some(element) => element.id(),
            None => other.to_string(),
        },
    }
}

/// Whether a CEL condition holds in `context`; one that fails does not.
pub fn holds(evaluator: &Evaluator, condition: Option<&str>, context: &str, vars: &Vars) -> bool {
    match condition {
        None => true,
        Some(source) => matches!(evaluator.evaluate(context, source, vars), Ok(Value::Bool(true))),
    }
}

/// A DISL `visible` (a boolean, a CEL string, or a `{cel}` object) as a condition: none where it always holds, and
/// `false` where it never does.
pub fn visibility(visible: Option<&Json>) -> Option<String> {
    match visible? {
        Json::Bool(true) => None,
        Json::Bool(false) => Some("false".to_owned()),
        Json::String(s) => Some(s.trim().to_owned()),
        Json::Object(o) => o.get("cel").and_then(Json::as_str).map(|s| s.trim().to_owned()),
        _ => None,
    }
}
