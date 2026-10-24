//! The expressions of a lowered specification as DISL writes them (ATD §6.6): `( e )` is `{"cel": e}`, except in a
//! property the schema types as an *Expression* or a *GeomExpr*, where it is the string `e`. The lowering writes the
//! object form everywhere; this pass walks the specification beside DISL's schema and writes the string where the
//! schema says so.

use std::sync::OnceLock;

use serde_json::Value as Json;

fn schema() -> &'static Json {
    static SCHEMA: OnceLock<Json> = OnceLock::new();
    SCHEMA.get_or_init(|| serde_json::from_str(adp_atd_check::embedded::DISL_SCHEMA).expect("DISL's schema is JSON"))
}

/// The `$defs` entry a `$ref` names, by name.
fn def_name(node: &Json) -> Option<&str> {
    node.get("$ref")?.as_str()?.strip_prefix("#/$defs/")
}

fn resolve(node: &Json) -> &Json {
    let mut current = node;
    for _ in 0..32 {
        match def_name(current) {
            Some(name) => current = &schema()["$defs"][name],
            None => break,
        }
    }
    current
}

/// Whether `node` is, through its references, DISL's *Expression* or *GeomExpr*.
fn is_expression(node: &Json) -> bool {
    let mut current = node;
    for _ in 0..32 {
        match def_name(current) {
            Some("Expression" | "GeomExpr") => return true,
            Some(name) => current = &schema()["$defs"][name],
            None => return false,
        }
    }
    false
}

/// The schemas of the property `key` of an object `node` admits.
fn property_schemas(node: &'static Json, key: &str, out: &mut Vec<&'static Json>, depth: usize) {
    if depth > 12 {
        return;
    }
    let node = resolve(node);
    if let Some(property) = node.get("properties").and_then(|p| p.get(key)) {
        out.push(property);
    }
    if let Some(patterns) = node.get("patternProperties").and_then(Json::as_object) {
        for (pattern, property) in patterns {
            if regex_lite_match(pattern, key) {
                out.push(property);
            }
        }
    }
    if let Some(additional) = node.get("additionalProperties").filter(|a| a.is_object()) {
        out.push(additional);
    }
    for composite in ["allOf", "anyOf", "oneOf"] {
        for branch in node.get(composite).and_then(Json::as_array).into_iter().flatten() {
            property_schemas(branch, key, out, depth + 1);
        }
    }
    for key_branch in ["then", "else"] {
        if let Some(branch) = node.get(key_branch) {
            property_schemas(branch, key, out, depth + 1);
        }
    }
}

/// The schemas of the items of an array `node` admits.
fn item_schemas(node: &'static Json, out: &mut Vec<&'static Json>, depth: usize) {
    if depth > 12 {
        return;
    }
    let node = resolve(node);
    if let Some(items) = node.get("items").filter(|i| i.is_object()) {
        out.push(items);
    }
    for composite in ["allOf", "anyOf", "oneOf"] {
        for branch in node.get(composite).and_then(Json::as_array).into_iter().flatten() {
            item_schemas(branch, out, depth + 1);
        }
    }
}

/// Whether a `patternProperties` pattern admits `key`. DISL's are prefixes (`^x-`); a pattern of another form is
/// taken to admit nothing, so the expressions under such a key keep the form they are written in.
fn regex_lite_match(pattern: &str, key: &str) -> bool {
    match pattern.strip_prefix('^') {
        Some(prefix) if !prefix.contains(['[', '(', '*', '+', '?', '.', '\\', '$']) => key.starts_with(prefix),
        _ => false,
    }
}

/// Rewrites `{"cel": e}` to `"e"` wherever DISL's schema types the position as an expression.
pub fn as_disl_writes(specification: &mut Json) {
    let root: &'static Json = schema()["$defs"].get("Specification").unwrap_or(&Json::Null);
    walk(specification, &[root], 0);
}

fn walk(value: &mut Json, schemas: &[&'static Json], depth: usize) {
    if depth > 64 || schemas.is_empty() {
        return;
    }
    if schemas.iter().any(|s| is_expression(s))
        && let Some(cel) = value
            .as_object()
            .filter(|o| o.len() == 1)
            .and_then(|o| o.get("cel"))
            .cloned()
        && cel.is_string()
    {
        *value = cel;
        return;
    }
    match value {
        Json::Object(object) => {
            for (key, child) in object.iter_mut() {
                let mut child_schemas = Vec::new();
                for schema in schemas {
                    property_schemas(schema, key, &mut child_schemas, 0);
                }
                walk(child, &child_schemas, depth + 1);
            }
        }
        Json::Array(items) => {
            let mut child_schemas = Vec::new();
            for schema in schemas {
                item_schemas(schema, &mut child_schemas, 0);
            }
            for item in items {
                walk(item, &child_schemas, depth + 1);
            }
        }
        _ => {}
    }
}
