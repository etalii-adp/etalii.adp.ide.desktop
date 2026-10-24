//! A statement of the script as the DISL action it is (ATD §12.3): an operation or a hook whose statements are all
//! DISL's runs as DISL's actions, and the script's host performs each action of a script through the same action.
//! Where an argument comes from is the caller's: the CEL as written, or the value the script evaluated.

use std::collections::BTreeMap;

use serde_json::{Map, Value as Json, json};

use super::values::{expression, text, value};

/// Where an action's arguments come from.
pub trait Arguments {
    /// The DISL expression of argument `key` (an entry of it for `assignments`, `settings` and `args`), whose node
    /// in the tree is `node`.
    fn argument(&mut self, key: &str, entry: Option<&str>, node: &Json) -> Json;
}

/// The arguments as they are written.
pub struct Written;

impl Arguments for Written {
    fn argument(&mut self, _key: &str, _entry: Option<&str>, node: &Json) -> Json {
        expression(node)
    }
}

/// The parameters of each operation, in order, for `call`.
pub type Parameters = BTreeMap<String, Vec<String>>;

/// The DISL actions of `statements`, or none when one of them is the script's alone (a loop with its limit, a
/// mutable local, `break`, `continue`, a `return` with a value or not last).
pub fn block(statements: &[Json], parameters: &Parameters, arguments: &mut dyn Arguments) -> Option<Vec<Json>> {
    let mut actions = Vec::new();
    for (index, statement) in statements.iter().enumerate() {
        if statement["kind"] == "return" && statement.get("value").is_none() && index + 1 == statements.len() {
            break;
        }
        actions.push(action(statement, parameters, arguments)?);
    }
    Some(actions)
}

/// The DISL action of one statement.
pub fn action(statement: &Json, parameters: &Parameters, arguments: &mut dyn Arguments) -> Option<Json> {
    let kind = statement["kind"].as_str()?;
    let mut body = Map::new();
    let put = |body: &mut Map<String, Json>, name: &str, value: Option<Json>| {
        if let Some(value) = value {
            body.insert(name.to_owned(), value);
        }
    };
    let action = match kind {
        "let" => json!({ "let": { text(&statement["name"]): arg(statement, arguments, "value")? } }),
        "if" => {
            let condition = arg(statement, arguments, "condition")?;
            let then = branch(&statement["then"], parameters, arguments)?;
            let mut action = json!({ "if": condition, "then": then });
            if !statement["else"].is_null() {
                action["else"] = Json::Array(branch(&statement["else"], parameters, arguments)?);
            }
            action
        }
        "fail" => json!({ "abort": { "message": arg(statement, arguments, "value")? } }),
        "create" | "connect" => {
            let type_name = format!("'{}'", text(&statement["type"]["name"]));
            body.insert("type".into(), Json::String(type_name));
            if kind == "connect" {
                put(&mut body, "source", arg(statement, arguments, "source"));
                put(&mut body, "target", arg(statement, arguments, "to"));
            } else {
                for key in ["parent", "after", "before", "at", "size"] {
                    put(&mut body, key, arg(statement, arguments, key));
                }
                if let Some(slot) = statement.get("slot") {
                    body.insert("slot".into(), Json::String(format!("'{}'", text(slot))));
                }
                if let Some(place) = statement.get("place") {
                    body.insert("place".into(), Json::String(text(place)));
                }
            }
            let attributes = entries(statement, "assignments", arguments);
            if !attributes.is_empty() {
                body.insert("attributes".into(), Json::Object(attributes));
            }
            let mut action = json!({ kind: body });
            if let Some(name) = statement.get("as") {
                action["as"] = Json::String(text(name));
            }
            action
        }
        "set" | "unset" => {
            let path: Vec<String> = statement["path"].as_array()?.iter().map(text).collect();
            let (target, attribute) = match &path[..] {
                [attribute] => (None, attribute.clone()),
                [target, attribute] => (Some(target.clone()), attribute.clone()),
                _ => return None,
            };
            let mut action = if kind == "set" {
                json!({ "set": { attribute: arg(statement, arguments, "value")? } })
            } else {
                json!({ "unset": [attribute] })
            };
            // `self` is DISL's default target.
            if let Some(target) = target.filter(|t| t != "self") {
                action["target"] = arguments.argument("target", None, &json!({ "cel": target }));
            }
            action
        }
        "delete" | "open" | "select" | "reveal" => json!({ kind: arg(statement, arguments, "target")? }),
        "highlight" => {
            let mut action = json!({ "highlight": arg(statement, arguments, "target")? });
            if let Some(duration) = setting(statement, "durationMs") {
                action["durationMs"] = duration;
            }
            action
        }
        "move" | "resize" | "reparent" | "reorder" => {
            for key in ["target", "x", "y", "width", "height", "parent", "after", "before", "by"] {
                put(&mut body, key, arg(statement, arguments, key));
            }
            if let Some(slot) = statement.get("slot") {
                body.insert("slot".into(), Json::String(format!("'{}'", text(slot))));
            }
            json!({ kind: body })
        }
        "retype" => {
            put(&mut body, "target", arg(statement, arguments, "target"));
            body.insert(
                "to".into(),
                Json::String(format!("'{}'", text(&statement["to"]["name"]))),
            );
            json!({ "retype": body })
        }
        "view" => {
            let mut action = json!({ "view": entries(statement, "assignments", arguments) });
            if let Some(target) = arg(statement, arguments, "target").filter(|t| t != "self") {
                action["target"] = target;
            }
            action
        }
        "editLabel" => {
            put(&mut body, "target", arg(statement, arguments, "target"));
            if let Some(label) = setting(statement, "label") {
                body.insert("label".into(), label);
            }
            json!({ "editLabel": body })
        }
        "openForm" => {
            put(&mut body, "target", arg(statement, arguments, "target"));
            body.insert("form".into(), Json::String(text(&statement["form"])));
            json!({ "openForm": body })
        }
        "notify" => {
            body.insert("message".into(), arg(statement, arguments, "value")?);
            if let Some(severity) = statement.get("severity") {
                body.insert("severity".into(), Json::String(text(severity)));
            }
            json!({ "notify": body })
        }
        "layout" => {
            body.insert("algorithm".into(), Json::String(text(&statement["name"])));
            put(&mut body, "scope", arg(statement, arguments, "on"));
            json!({ "layout": body })
        }
        "call" => {
            let operation = text(&statement["operation"]);
            let names = parameters.get(&operation)?;
            let mut args = Map::new();
            for (index, entry) in statement["args"].as_array().into_iter().flatten().enumerate() {
                let name = names.get(index)?;
                args.insert(
                    name.clone(),
                    arguments.argument("args", Some(&index.to_string()), &entry["value"]),
                );
            }
            json!({ "call": operation, "args": args })
        }
        _ => return None,
    };
    Some(action)
}

/// The DISL expression of the argument `key` of `statement`, when it has one.
fn arg(statement: &Json, arguments: &mut dyn Arguments, key: &str) -> Option<Json> {
    statement
        .get(key)
        .filter(|n| n.get("cel").is_some())
        .map(|n| arguments.argument(key, None, n))
}

/// A branch of an `if`: a block, or an `else if`.
fn branch(node: &Json, parameters: &Parameters, arguments: &mut dyn Arguments) -> Option<Vec<Json>> {
    match node {
        Json::Array(statements) => block(statements, parameters, arguments),
        Json::Object(_) => Some(vec![action(node, parameters, arguments)?]),
        _ => Some(Vec::new()),
    }
}

/// An action's `assignments` as DISL's attribute map.
fn entries(statement: &Json, key: &str, arguments: &mut dyn Arguments) -> Map<String, Json> {
    let mut map = Map::new();
    for entry in statement[key].as_array().into_iter().flatten() {
        let name = entry["key"].as_str().unwrap_or_default();
        map.insert(name.to_owned(), arguments.argument(key, Some(name), &entry["value"]));
    }
    map
}

/// A literal setting of an action (`{ label: text }`).
fn setting(statement: &Json, key: &str) -> Option<Json> {
    statement["settings"]
        .as_array()?
        .iter()
        .find(|s| s["key"] == key)
        .map(|s| value(&s["value"]))
}
