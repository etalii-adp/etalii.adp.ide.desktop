//! What every derivation shares (the standalone host's `DislEvaluation`): evaluating a specification's expressions,
//! its Messages and its Reasons (DISL sections 2.3 and 2.5), and the text of a value as a form shows it.
//!
//! Programs are compiled once per specification: an expression the loader compiled is taken from
//! [`DislSpecification::expression_at`]; one it does not walk, inside an `x-` key, is compiled on first use and kept
//! beside the specification for as long as it lives. An expression that fails gives no value, which each caller
//! turns into the position's fallback: a Message its default text, a condition false.

use std::collections::HashMap;

use adp_cel::Value;

use crate::expressions::contexts;
use crate::json::{self, Json};
use crate::loading::specification::DislSpecification;
use crate::model::element::DislElement;

/// The variables of one evaluation.
pub type Vars = HashMap<String, Value>;

/// Variables from pairs.
pub(crate) fn vars<const N: usize>(pairs: [(&str, Value); N]) -> Vars {
    pairs.into_iter().map(|(k, v)| (k.to_owned(), v)).collect()
}

/// The value of `source` compiled in `context`, for an expression the loader does not walk; a variable beyond the
/// context's (an option's `item`) is declared as bound.
fn of(specification: &DislSpecification, context: &str, source: &str, variables: &Vars) -> Result<Value, String> {
    let declared = contexts::variables_of(context);
    let mut bound: Vec<&str> = variables
        .keys()
        .map(String::as_str)
        .filter(|n| !declared.contains(n))
        .collect();
    bound.sort_unstable();
    let key = (context.to_owned(), source.to_owned(), bound.join(","));
    let program = {
        let mut compiled = specification.0.compiled.borrow_mut();
        compiled
            .entry(key)
            .or_insert_with(|| specification.environment(context, &bound).compile(source))
            .clone()
    };
    program?.evaluate(variables)
}

/// The value of an Expression property (section 2.5 a) at `pointer`: a bare string or the `cel` of an object,
/// compiled by the loader, or in `context` when it is not.
pub(crate) fn expression(
    specification: &DislSpecification,
    json: &Json,
    pointer: &str,
    context: &str,
    variables: &Vars,
) -> Result<Value, String> {
    let (source, at) = match json {
        Json::String(s) => (s.as_str(), pointer.to_owned()),
        Json::Object(_) => match json::string_of(json, "cel") {
            Some(cel) => (cel, json::pointer(pointer, "cel")),
            None => return Err(format!("{pointer} is not an expression.")),
        },
        _ => return Err(format!("{pointer} is not an expression.")),
    };
    match specification.expression_at(&at) {
        Some(compiled) => compiled.program.evaluate(variables),
        None => of(specification, context, source, variables),
    }
}

/// Whether the condition at `pointer` holds; a condition that fails does not.
pub(crate) fn holds(
    specification: &DislSpecification,
    json: &Json,
    pointer: &str,
    context: &str,
    variables: &Vars,
) -> bool {
    matches!(
        expression(specification, json, pointer, context, variables),
        Ok(Value::Bool(true))
    )
}

/// The text of a Message (section 2.3): a string as it is, a locale map's English or first text, the value of a
/// `{cel}` object; `fallback` when it is absent or fails.
pub(crate) fn message(
    specification: &DislSpecification,
    json: &Json,
    pointer: &str,
    context: &str,
    variables: &Vars,
    fallback: &str,
) -> String {
    match json {
        Json::String(s) => s.clone(),
        Json::Object(members) if members.contains_key("cel") => {
            match expression(specification, json, pointer, context, variables) {
                Ok(Value::String(text)) => text,
                Ok(Value::Map(locales)) => localized(
                    locales
                        .iter()
                        .map(|(k, v)| (k.as_str(), v.as_str().map(str::to_owned)))
                        .collect(),
                )
                .unwrap_or_else(|| fallback.to_owned()),
                _ => fallback.to_owned(),
            }
        }
        Json::Object(members) => localized(
            members
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str().map(str::to_owned)))
                .collect(),
        )
        .unwrap_or_else(|| fallback.to_owned()),
        _ => fallback.to_owned(),
    }
}

/// The first Reason of `reasons` that applies (section 2.3): a Message always does, an object while its `when`
/// holds, a `{reason: id}` while the named reason's does.
pub(crate) fn first_reason(
    specification: &DislSpecification,
    reasons: &Json,
    pointer: &str,
    context: &str,
    variables: &Vars,
) -> Option<String> {
    let list = reasons.as_array()?;
    list.iter().enumerate().find_map(|(index, reason)| {
        self::reason(
            specification,
            reason,
            &json::pointer_at(pointer, index),
            context,
            variables,
        )
    })
}

/// The text of one Reason (section 2.3) when it applies, as [`first_reason`] judges each of a list.
pub(crate) fn one_reason(
    specification: &DislSpecification,
    reason: &Json,
    pointer: &str,
    context: &str,
    variables: &Vars,
) -> Option<String> {
    self::reason(specification, reason, pointer, context, variables)
}

fn reason(
    specification: &DislSpecification,
    reason: &Json,
    pointer: &str,
    context: &str,
    variables: &Vars,
) -> Option<String> {
    if let Some(id) = json::string_of(reason, "reason") {
        // A named reason is declared once under behavior.reasons, in the element context.
        let declared = json::get(
            json::member(json::member(specification.root(), "behavior"), "reasons"),
            id,
        )?;
        return self::reason(
            specification,
            declared,
            &json::pointer("/behavior/reasons", id),
            contexts::ELEMENT,
            variables,
        );
    }
    if let Some(message) = json::get(reason, "message") {
        if let Some(when) = json::get(reason, "when")
            && !holds(specification, when, &json::pointer(pointer, "when"), context, variables)
        {
            return None;
        }
        return Some(self::message(
            specification,
            message,
            &json::pointer(pointer, "message"),
            context,
            variables,
            "",
        ));
    }
    Some(self::message(specification, reason, pointer, context, variables, ""))
}

/// The text `behavior.messages` gives for `key` (section 9.1), else `fallback`.
pub(crate) fn standard_message(
    specification: &DislSpecification,
    key: &str,
    variables: &Vars,
    fallback: &str,
) -> String {
    match json::get(
        json::member(json::member(specification.root(), "behavior"), "messages"),
        key,
    ) {
        Some(message) => self::message(
            specification,
            message,
            &json::pointer("/behavior/messages", key),
            contexts::ELEMENT,
            variables,
            fallback,
        ),
        None => fallback.to_owned(),
    }
}

/// A CEL value as text: a string as it is, an element by its id, any other value as CEL's `string()` writes it.
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

fn localized(locales: Vec<(&str, Option<String>)>) -> Option<String> {
    let list: Vec<(&str, String)> = locales.into_iter().filter_map(|(t, s)| s.map(|s| (t, s))).collect();
    list.iter()
        .find(|(t, _)| *t == "en")
        .or_else(|| list.iter().find(|(t, _)| t.starts_with("en-")))
        .or_else(|| list.first())
        .map(|(_, s)| s.clone())
}
