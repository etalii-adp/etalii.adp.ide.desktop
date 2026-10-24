//! A description's bindings as an FBL document (ATD §11; research R16): each `binding` as FBL's `Binding`, its rules
//! as `ElementRule` and `RelationRule`, and their settings as FBL's properties (ATD §6.6), so that `adp-fbl` reads and
//! writes the description's bodies as it reads and writes those of today's tools.

use serde_json::{Map, Value as Json, json};

use super::values::{settings_of, text, value};

/// The FBL version the lowered document states.
const FBL: &str = "0.1";

/// The FBL document of the bindings among `members`, none when there are none.
pub fn lower(members: &[&Json]) -> Option<Json> {
    let mut bindings = Map::new();
    for binding in members.iter().filter(|m| m["kind"] == "binding") {
        bindings.insert(text(&binding["name"]), Json::Object(lower_binding(binding)));
    }
    if bindings.is_empty() {
        return None;
    }
    Some(json!({
        "$schema": "https://etalii.net/adp/fbl/schema/0.1/fbl.schema.json#/$defs/Document",
        "fbl": FBL,
        "bindings": bindings,
    }))
}

fn lower_binding(binding: &Json) -> Map<String, Json> {
    let mut lowered = Map::new();
    let mut elements = Vec::new();
    let mut relations = Vec::new();
    let mut body = Map::new();
    body.insert("kind".into(), json!("file"));
    if let Some(family) = binding["family"].as_str() {
        body.insert("family".into(), json!(family));
    }
    for member in binding["members"].as_array().into_iter().flatten() {
        match member["kind"].as_str().unwrap_or_default() {
            "label" => {
                lowered.insert("title".into(), value(&member["value"]));
            }
            "claims" | "text" | "header" | "registration" => {
                let key = member["kind"].as_str().unwrap_or_default().to_owned();
                let mut own = settings_of(member);
                if let Some(files) = member.get("files") {
                    own.insert("files".into(), files.clone());
                }
                lowered.insert(key, Json::Object(own));
            }
            "body" => body.extend(settings_of(member)),
            "rule" => {
                let rule = rule(member);
                if member["ruleKind"] == "relation" {
                    relations.push(rule);
                } else {
                    elements.push(rule);
                }
            }
            "block" | "section" => {
                let section = if member["kind"] == "block" {
                    "blocks"
                } else {
                    "sections"
                };
                let mut lowered_rule = settings_of(member);
                lowered_rule.insert("name".into(), json!(text(&member["name"])));
                for key in ["line", "heading"] {
                    if let Some(v) = member.get(key) {
                        lowered_rule.insert(key.into(), v.clone());
                    }
                }
                let entry = lowered.entry(section).or_insert_with(|| json!([]));
                if let Some(list) = entry.as_array_mut() {
                    list.push(Json::Object(lowered_rule));
                }
            }
            "template" => {
                lowered.insert("template".into(), json!({ "text": value(&member["value"]) }));
            }
            "setting" => {
                let key = member["setting"]["key"].as_str().unwrap_or_default().to_owned();
                lowered.insert(key, value(&member["setting"]["value"]));
            }
            _ => {}
        }
    }
    let mut ordered = Map::new();
    if let Some(title) = lowered.shift_remove("title") {
        ordered.insert("title".into(), title);
    }
    if let Some(claims) = lowered.shift_remove("claims") {
        ordered.insert("claims".into(), claims);
    }
    ordered.insert("body".into(), Json::Object(body));
    ordered.insert("reader".into(), json!("declared"));
    ordered.extend(lowered);
    if !elements.is_empty() {
        ordered.insert("elements".into(), Json::Array(elements));
    }
    if !relations.is_empty() {
        ordered.insert("relations".into(), Json::Array(relations));
    }
    // The template last, as FBL writes it.
    if let Some(template) = ordered.shift_remove("template") {
        ordered.insert("template".into(), template);
    }
    ordered
}

/// An element or relation rule (ATD §11.3).
fn rule(rule: &Json) -> Json {
    let mut lowered = Map::new();
    if let Some(name) = rule.get("as") {
        lowered.insert("name".into(), json!(text(name)));
    }
    lowered.insert("type".into(), json!(text(&rule["type"]["name"])));
    for key in ["at", "line"] {
        if let Some(v) = rule.get(key) {
            lowered.insert(key.into(), v.clone());
        }
    }
    let mut attributes = Map::new();
    for member in rule["members"].as_array().into_iter().flatten() {
        match member["kind"].as_str().unwrap_or_default() {
            "parent" => {
                let mut parent = Map::new();
                if let Some(rules) = member.get("rules") {
                    parent.insert("rules".into(), json!(super::values::names(rules)));
                }
                if let Some(slot) = member.get("slot") {
                    parent.insert("slot".into(), json!(text(slot)));
                }
                parent.extend(settings_of(member));
                lowered.insert("parent".into(), Json::Object(parent));
            }
            "id" => {
                let mut id = settings_of(member);
                if let Some(from) = member.get("from") {
                    id.insert("from".into(), Json::Object(slot(from)));
                }
                lowered.insert("id".into(), Json::Object(id));
            }
            "source" | "target" => {
                let mut end = member.get("from").map(slot).unwrap_or_default();
                end.extend(settings_of(member));
                lowered.insert(
                    member["kind"].as_str().unwrap_or_default().to_owned(),
                    Json::Object(end),
                );
            }
            "attribute" => {
                let mut attribute = member.get("from").map(slot).unwrap_or_default();
                attribute.extend(settings_of(member));
                if let Some(references) = member.get("references") {
                    attribute.insert(
                        "reference".into(),
                        json!({ "rules": super::values::names(&references["rules"]), "by": text(&references["by"]) }),
                    );
                }
                attributes.insert(text(&member["name"]), Json::Object(attribute));
            }
            "insert" | "remove" => {
                let key = member["kind"].as_str().unwrap_or_default().to_owned();
                lowered.insert(key, Json::Object(settings_of(member)));
            }
            "setting" => {
                let key = member["setting"]["key"].as_str().unwrap_or_default().to_owned();
                lowered.insert(key, value(&member["setting"]["value"]));
            }
            _ => {}
        }
    }
    if !attributes.is_empty() {
        // FBL writes a rule's attributes after its id and parent, before what it inserts and removes.
        let mut ordered = Map::new();
        let mut attributes = Some(Json::Object(attributes));
        for (key, value) in lowered {
            if matches!(key.as_str(), "insert" | "remove" | "readOnly")
                && let Some(attributes) = attributes.take()
            {
                ordered.insert("attributes".into(), attributes);
            }
            ordered.insert(key, value);
        }
        if let Some(attributes) = attributes {
            ordered.insert("attributes".into(), attributes);
        }
        return Json::Object(ordered);
    }
    Json::Object(lowered)
}

/// Where a value is read from (`attribute "TEXT"`, `text`, `parent "ID"`) as FBL writes it: the source's name, with
/// its argument, or `true` when it takes none.
fn slot(from: &Json) -> Map<String, Json> {
    let kind = from["kind"].as_str().unwrap_or_default().to_owned();
    let argument = match from.get("argument") {
        Some(argument) if argument.get("cel").is_some() => argument["cel"].clone(),
        Some(argument) => argument.clone(),
        None => json!(true),
    };
    let mut lowered = Map::new();
    lowered.insert(kind, argument);
    lowered
}
