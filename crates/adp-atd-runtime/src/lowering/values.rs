//! The JSON value of a value (ATD §6.6): what a setting is lowered to.

use serde_json::{Map, Value as Json, json};

/// A value of the tree as JSON: a literal as itself, `( e )` as `{"cel": e}`, `@p`, `$n` and `#n` as DISL's
/// references, a settings map as an object with its keys in written order and a list as an array.
pub fn value(value: &Json) -> Json {
    if let Some(literal) = value.get("literal") {
        return literal.clone();
    }
    if let Some(cel) = value.get("cel") {
        return json!({ "cel": cel });
    }
    for key in ["attribute", "token", "param"] {
        if let Some(name) = value.get(key) {
            return json!({ key: name });
        }
    }
    if let Some(entries) = value.get("settings").and_then(Json::as_array) {
        return Json::Object(settings(entries));
    }
    if let Some(items) = value.get("list").and_then(Json::as_array) {
        return Json::Array(items.iter().map(self::value).collect());
    }
    Json::Null
}

/// A list of settings (`{key, value}`) as an object.
pub fn settings(entries: &[Json]) -> Map<String, Json> {
    let mut object = Map::new();
    for entry in entries {
        if let Some(key) = entry["key"].as_str() {
            object.insert(key.to_owned(), value(&entry["value"]));
        }
    }
    object
}

/// The settings a construct lists in `settings`, and those among its `members` (`kind: setting`), as one object.
pub fn settings_of(node: &Json) -> Map<String, Json> {
    let mut object = settings(node["settings"].as_array().map(Vec::as_slice).unwrap_or_default());
    for member in node["members"].as_array().into_iter().flatten() {
        if member["kind"] == "setting" {
            let setting = &member["setting"];
            if let Some(key) = setting["key"].as_str() {
                object.insert(key.to_owned(), value(&setting["value"]));
            }
        }
    }
    object
}

/// An expression a position takes bare (ATD §6.2), as DISL's *Expression*: the CEL text.
pub fn expression(node: &Json) -> Json {
    match node.get("cel").and_then(Json::as_str) {
        Some(cel) => Json::String(cel.to_owned()),
        None => value(node),
    }
}

/// A name of the tree.
pub fn text(node: &Json) -> String {
    node["text"].as_str().unwrap_or_default().to_owned()
}

/// A list of names, as names.
pub fn names(node: &Json) -> Vec<String> {
    node.as_array().into_iter().flatten().map(text).collect()
}

/// A type reference (`string`, `Node`, `list<string>`) as DISL writes a type: `list(string)`.
pub fn type_name(node: &Json) -> String {
    let name = text(&node["name"]);
    let arguments: Vec<String> = node["arguments"]
        .as_array()
        .into_iter()
        .flatten()
        .map(type_name)
        .collect();
    if arguments.is_empty() {
        name
    } else {
        format!("{name}({})", arguments.join(", "))
    }
}
