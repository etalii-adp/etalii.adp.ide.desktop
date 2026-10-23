//! Reading the abstract tree (`atd.schema.json#/$defs/Tree`), which the lowering builds as JSON: spans, names and
//! the walk over every construct.

use serde_json::Value as Json;

/// A span of the tree: a file's byte range and the 1-based positions of its ends, columns in code points.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Span {
    pub file: String,
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

impl Span {
    pub fn of(node: &Json) -> Option<Span> {
        let span = node.get("span")?;
        let int = |key: &str| span.get(key).and_then(Json::as_u64).unwrap_or(0) as usize;
        Some(Span {
            file: span.get("file")?.as_str()?.to_owned(),
            start: int("start"),
            end: int("end"),
            line: int("line"),
            column: int("column"),
            end_line: int("endLine"),
            end_column: int("endColumn"),
        })
    }
}

/// The text of a name (`{text, span}`) or a qualified name.
pub fn text(name: &Json) -> &str {
    name.get("text").and_then(Json::as_str).unwrap_or("")
}

/// The construct's `kind`.
pub fn kind(node: &Json) -> &str {
    node.get("kind").and_then(Json::as_str).unwrap_or("")
}

/// Calls `visit` for every object of the tree, depth first, with the path of keys and indexes to it.
pub fn walk<'a>(node: &'a Json, visit: &mut dyn FnMut(&'a Json)) {
    match node {
        Json::Object(map) => {
            visit(node);
            for value in map.values() {
                walk(value, visit);
            }
        }
        Json::Array(items) => {
            for item in items {
                walk(item, visit);
            }
        }
        _ => {}
    }
}
