//! A designer's description lowered onto the diagrams' runtime for validation (etalii.adp spec 076, T056, research R17):
//! a designer has the parts of a diagram, a model, operations of nearly the same actions, constraints, persistence and a
//! binding, so the DISL specification its description lowers to runs through what runs a diagram, with DESL's names
//! given as DISL's: `document` is the diagram, `document.elementsOfType` is `diagram.nodesOfType`, and an attribute
//! DISL §2.2 reserves keeps its name on a type marked `x-desl`. A designer's surface is not drawn.

use regex::Regex;
use serde_json::Value as Json;
use std::sync::LazyLock;

static DOCUMENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bdocument\.(?:elementsOfType\b)?").expect("the pattern compiles"));

/// `specification`, lowered from a designer's description, as DISL reads it.
pub fn as_disl(specification: &mut Json) {
    rename(specification);
    moves_as_reparents(specification);
    // A binding type mapped onto the document maps onto the diagram (DESL section 5.2, DISL section 11.2).
    if let Some(map) = specification
        .pointer_mut("/persistence/typeMap")
        .and_then(Json::as_object_mut)
    {
        for entry in map.values_mut() {
            if entry["as"] == "document" {
                entry["as"] = Json::String("diagram".into());
            }
        }
    }
    if let Some(types) = specification
        .pointer_mut("/metamodel/types")
        .and_then(Json::as_object_mut)
    {
        for t in types.values_mut() {
            if let Some(t) = t.as_object_mut() {
                t.insert("x-desl".into(), Json::Bool(true));
            }
        }
    }
}

/// The statements the script runs (a loop with its limit, ATD §12), with DESL's `elementsOfType` as DISL's.
pub fn as_disl_script(scripts: &mut serde_json::Map<String, Json>) {
    scripts.values_mut().for_each(rename_in_script);
}

/// Every text of `json` with `elementsOfType` as `nodesOfType`: the script names the diagram `document` itself.
fn rename_in_script(json: &mut Json) {
    match json {
        Json::String(text) if text.contains("elementsOfType") => *text = text.replace("elementsOfType", "nodesOfType"),
        Json::Array(items) => items.iter_mut().for_each(rename_in_script),
        Json::Object(map) => map.values_mut().for_each(rename_in_script),
        _ => {}
    }
}

/// Every text of `json` with DESL's `document.` as DISL's `diagram.`, and `elementsOfType` as `nodesOfType`.
fn rename(json: &mut Json) {
    match json {
        Json::String(text) if text.contains("document.") => {
            *text = DOCUMENT
                .replace_all(text, |c: &regex::Captures<'_>| {
                    if c[0].ends_with("elementsOfType") {
                        "diagram.nodesOfType".to_owned()
                    } else {
                        "diagram.".to_owned()
                    }
                })
                .into_owned();
        }
        Json::Array(items) => items.iter_mut().for_each(rename),
        Json::Object(map) => {
            // An action on the document and a constraint of the document are DISL's on the diagram.
            for key in ["target", "scope"] {
                if map
                    .get(key)
                    .is_some_and(|v| v == "document" || *v == serde_json::json!(["document"]))
                {
                    map.insert(key.into(), Json::String("diagram".into()));
                }
            }
            map.values_mut().for_each(rename)
        }
        _ => {}
    }
}

/// DESL's `move` into a parent (DESL section 9.4) as DISL's `reparent`: a designer's elements have no position.
fn moves_as_reparents(json: &mut Json) {
    match json {
        Json::Object(map) => {
            if let Some(body) = map.get("move")
                && body.get("parent").is_some()
                && let Some(body) = map.remove("move")
            {
                map.insert("reparent".into(), body);
            }
            map.values_mut().for_each(moves_as_reparents);
        }
        Json::Array(items) => items.iter_mut().for_each(moves_as_reparents),
        _ => {}
    }
}
