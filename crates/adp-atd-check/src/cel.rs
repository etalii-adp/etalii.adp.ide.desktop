//! CEL in a description (ATD §6.3, §14.5): the functions an expression may call, and the static type of an
//! expression where it can be known. An expression whose type cannot be known is `dyn` and never a finding.

use std::collections::BTreeMap;

use serde_json::Value as Json;

use crate::symbols::{BUILT_IN_FIELDS, Symbols};

/// A CEL type, as far as the checker infers it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    Bool,
    Int,
    Uint,
    Double,
    String,
    Bytes,
    Null,
    List(Box<Ty>),
    Map,
    Element(String),
    Dyn,
}

impl Ty {
    pub fn name(&self) -> String {
        match self {
            Ty::Bool => "bool".into(),
            Ty::Int => "int".into(),
            Ty::Uint => "uint".into(),
            Ty::Double => "double".into(),
            Ty::String => "string".into(),
            Ty::Bytes => "bytes".into(),
            Ty::Null => "null".into(),
            Ty::List(inner) => format!("list<{}>", inner.name()),
            Ty::Map => "map".into(),
            Ty::Element(name) => name.clone(),
            Ty::Dyn => "dyn".into(),
        }
    }
}

/// The global functions of CEL's standard environment and extensions, and of DISL §12.4, DESL §9.4 and FBL §2.4.
pub const LIBRARY: &[&str] = &[
    "size",
    "int",
    "uint",
    "double",
    "string",
    "bytes",
    "bool",
    "type",
    "dyn",
    "duration",
    "timestamp",
    "matches",
    "has",
    "range",
    "snap",
    "snapFloor",
    "snapCeil",
    "snapTo",
    "clamp",
    "min",
    "max",
    "ordinal",
    "enumLabel",
    "enumColor",
    "enumIcon",
    "token",
    "color",
    "formatNumber",
    "formatDate",
    "formatDuration",
    "date",
    "workingDays",
    "addWorkingDays",
    "isWorkingTime",
    "daysBetween",
    "distance",
    "lower",
    "upper",
    "matchesGlob",
    "budget",
    "filterValue",
    "axisRange",
    "textWidth",
    "textHeight",
    "yearMonth",
    "formatYearMonth",
    "parseYearMonth",
    "precisionOf",
    "optional",
    "list",
    "map",
    "abs",
    "ceil",
    "floor",
    "round",
    "greatest",
    "least",
    "sqrt",
    "trunc",
    // ATD §12.8: a host attribute the binding read, by name, and the id a relation's end was read with.
    "hostAttribute",
    "endId",
];

/// Functions that read what a script may not (ATD §12.5): a call of one is `ATD7001`, not an unknown name.
pub const FORBIDDEN: &[&str] = &[
    "now",
    "today",
    "clock",
    "time",
    "random",
    "rand",
    "randomInt",
    "uuid",
    "uuidv4",
    "newid",
    "env",
    "getenv",
    "readFile",
    "file",
    "fetch",
    "http",
    "sleep",
    "host",
    "exec",
];

/// What an expression's names mean where it stands: its variables and their types.
#[derive(Debug, Clone, Default)]
pub struct Scope {
    pub variables: BTreeMap<String, Ty>,
}

impl Scope {
    pub fn with(mut self, name: &str, ty: Ty) -> Self {
        self.variables.insert(name.to_owned(), ty);
        self
    }
}

/// The type of an attribute's declared type.
pub fn type_of_ref(type_ref: &Json, symbols: &Symbols) -> Ty {
    let name = type_ref["name"]["text"].as_str().unwrap_or("");
    match name {
        "string" | "text" | "uri" | "url" | "color" | "path" | "date" | "datetime" | "time" | "timestamp"
        | "duration" | "expression" | "timezone" => Ty::String,
        "int" | "integer" | "yearMonth" => Ty::Int,
        "uint" => Ty::Uint,
        "number" | "double" | "float" => Ty::Double,
        "bool" | "boolean" => Ty::Bool,
        "bytes" | "binary" => Ty::Bytes,
        "list" => {
            let inner = type_ref["arguments"]
                .get(0)
                .map(|a| type_of_ref(a, symbols))
                .unwrap_or(Ty::Dyn);
            Ty::List(Box::new(inner))
        }
        "map" => Ty::Map,
        other if symbols.enums.contains_key(other) => Ty::String,
        other if symbols.is_type(other) => Ty::Element(other.to_owned()),
        _ => Ty::Dyn,
    }
}

/// The static type of a CEL node in a scope.
pub fn infer(node: &Json, scope: &Scope, symbols: &Symbols) -> Ty {
    if let Some(kind) = node.get("type").and_then(Json::as_str)
        && node.get("literal").is_some()
    {
        return match kind {
            "int" => Ty::Int,
            "uint" => Ty::Uint,
            "double" => Ty::Double,
            "string" => Ty::String,
            "bytes" => Ty::Bytes,
            "bool" => Ty::Bool,
            _ => Ty::Null,
        };
    }
    if let Some(name) = node.get("ident").and_then(Json::as_str) {
        return scope.variables.get(name).cloned().unwrap_or(Ty::Dyn);
    }
    if let Some(field) = node.get("select").and_then(Json::as_str) {
        if node.get("optional").is_some() {
            return Ty::Dyn;
        }
        return match infer(&node["operand"], scope, symbols) {
            Ty::Element(name) => match field {
                "id" | "type" | "kind" => Ty::String,
                "descendants" | "ancestors" | "incoming" | "outgoing" | "ports" => Ty::List(Box::new(Ty::Dyn)),
                "parent" => symbols.container_of(&name).map(Ty::Element).unwrap_or(Ty::Dyn),
                "children" => match symbols.contained_by(&name).as_slice() {
                    [only] => Ty::List(Box::new(Ty::Element(only.clone()))),
                    _ => Ty::List(Box::new(Ty::Dyn)),
                },
                "source" | "target" | "owner" | "view" | "diagram" | "sourcePort" | "targetPort" => Ty::Dyn,
                _ => symbols
                    .attributes_of(&name)
                    .get(field)
                    .map(|a| type_of_ref(&a["type"], symbols))
                    .unwrap_or(Ty::Dyn),
            },
            _ => Ty::Dyn,
        };
    }
    if let Some(items) = node.get("list").and_then(Json::as_array) {
        let first = items.first().map(|i| infer(i, scope, symbols)).unwrap_or(Ty::Dyn);
        let same = items.iter().all(|i| infer(i, scope, symbols) == first);
        return Ty::List(Box::new(if same { first } else { Ty::Dyn }));
    }
    if node.get("map").is_some() {
        return Ty::Map;
    }
    let Some(call) = node.get("call").and_then(Json::as_str) else {
        return Ty::Dyn;
    };
    let args: Vec<&Json> = node["args"].as_array().map(|a| a.iter().collect()).unwrap_or_default();
    let arg = |i: usize| args.get(i).map(|a| infer(a, scope, symbols)).unwrap_or(Ty::Dyn);
    let numeric = |a: Ty, b: Ty| {
        if a == b && matches!(a, Ty::Int | Ty::Uint | Ty::Double) {
            a
        } else {
            Ty::Dyn
        }
    };
    if let Some(target) = node.get("target") {
        let receiver = infer(target, scope, symbols);
        return match call {
            "size" | "indexOf" | "lastIndexOf" | "positionIn" | "count" => Ty::Int,
            "contains" | "startsWith" | "endsWith" | "matches" | "exists" | "all" | "exists_one" | "isUnique"
            | "hasValue" => Ty::Bool,
            "trim" | "lowerAscii" | "upperAscii" | "replace" | "substring" | "join" | "format" | "charAt" | "quote" => {
                Ty::String
            }
            "map" => Ty::List(Box::new(Ty::Dyn)),
            "filter" | "sort" | "sortBy" | "distinct" | "reverse" => match receiver {
                Ty::List(_) => receiver,
                _ => Ty::Dyn,
            },
            _ => Ty::Dyn,
        };
    }
    match call {
        "_==_" | "_!=_" | "_<_" | "_<=_" | "_>_" | "_>=_" | "_&&_" | "_||_" | "!_" | "@in" | "has" | "matches" => {
            Ty::Bool
        }
        "_+_" => match (arg(0), arg(1)) {
            (Ty::String, Ty::String) => Ty::String,
            (Ty::List(a), Ty::List(b)) => Ty::List(if a == b { a } else { Box::new(Ty::Dyn) }),
            (a, b) => numeric(a, b),
        },
        "_-_" | "_*_" | "_/_" | "_%_" => numeric(arg(0), arg(1)),
        "-_" => match arg(0) {
            t @ (Ty::Int | Ty::Double) => t,
            _ => Ty::Dyn,
        },
        "_?_:_" => {
            let (a, b) = (arg(1), arg(2));
            if a == b { a } else { Ty::Dyn }
        }
        "_[_]" => match arg(0) {
            Ty::List(inner) => *inner,
            _ => Ty::Dyn,
        },
        "size" | "int" | "ordinal" | "workingDays" | "daysBetween" | "yearMonth" => Ty::Int,
        "uint" => Ty::Uint,
        "double" | "textWidth" | "textHeight" | "distance" | "snap" | "snapTo" => Ty::Double,
        "string" | "enumLabel" | "token" | "formatNumber" | "formatDate" | "formatDuration" | "lower" | "upper" => {
            Ty::String
        }
        "bool" | "isWorkingTime" | "matchesGlob" => Ty::Bool,
        "range" => Ty::List(Box::new(Ty::Int)),
        other => match symbols.functions.get(other) {
            Some(function) => function
                .get("returns")
                .map(|r| type_of_ref(r, symbols))
                .unwrap_or(Ty::Dyn),
            None => Ty::Dyn,
        },
    }
}

/// Every global call of an expression (no receiver), with its node.
/// The fields of an element every element has besides `BUILT_IN_FIELDS`: whether it is derived, and its sources
/// (DISL §12.2).
const ELEMENT_MEMBERS: &[&str] = &["derived", "sources"];

/// The selects of an expression on an element of a known type that names a field the type does not have, each with
/// the field and the type (ATD §14.2, `ATD2001`). An operand whose type cannot be known is never one.
pub fn unknown_members(node: &Json, scope: &Scope, symbols: &Symbols, out: &mut Vec<(Json, String, String)>) {
    if let Some(field) = node.get("select").and_then(Json::as_str)
        && node.get("optional").is_none()
        && let Ty::Element(name) = infer(&node["operand"], scope, symbols)
        && symbols.is_type(&name)
        && !BUILT_IN_FIELDS.contains(&field)
        && !ELEMENT_MEMBERS.contains(&field)
        && !symbols.attributes_of(&name).contains_key(field)
    {
        // The finding is at the field: the end of the select, as long as the field's name.
        let mut at = node.clone();
        let span = &mut at["span"];
        let width = field.chars().count() as u64;
        if let (Some(end), Some(end_column)) = (span["end"].as_u64(), span["endColumn"].as_u64()) {
            span["start"] = Json::from(end.saturating_sub(field.len() as u64));
            span["line"] = span["endLine"].clone();
            span["column"] = Json::from(end_column.saturating_sub(width));
        }
        out.push((at, field.to_owned(), name));
    }
    // `a && b` reads `b` only where `a` holds, so an `x.isA('T')` among `a`'s conjuncts makes `x` a `T` in `b`
    // (ATD §14.2): a form for a base type may show a subtype's attribute behind that guard.
    if node.get("call").and_then(Json::as_str) == Some("_&&_")
        && let Some([left, right]) = node["args"].as_array().map(Vec::as_slice)
    {
        unknown_members(left, scope, symbols, out);
        let mut narrowed = scope.clone();
        type_guards(left, symbols, &mut narrowed);
        unknown_members(right, &narrowed, symbols, out);
        return;
    }
    // `c ? a : b` reads `a` only where `c` holds, so an `x.isA('T')` among `c`'s conjuncts makes `x` a `T` in `a`.
    if node.get("call").and_then(Json::as_str) == Some("_?_:_")
        && let Some([condition, held, otherwise]) = node["args"].as_array().map(Vec::as_slice)
    {
        unknown_members(condition, scope, symbols, out);
        let mut narrowed = scope.clone();
        type_guards(condition, symbols, &mut narrowed);
        unknown_members(held, &narrowed, symbols, out);
        unknown_members(otherwise, scope, symbols, out);
        return;
    }
    match node {
        Json::Object(map) => {
            for (key, value) in map {
                if key != "span" {
                    unknown_members(value, scope, symbols, out);
                }
            }
        }
        Json::Array(items) => items.iter().for_each(|item| unknown_members(item, scope, symbols, out)),
        _ => {}
    }
}

/// The variables a condition proves to be of a type: each `x.isA('T')` among its `&&` conjuncts, `T` a type.
fn type_guards(node: &Json, symbols: &Symbols, scope: &mut Scope) {
    match node.get("call").and_then(Json::as_str) {
        Some("_&&_") => {
            for arg in node["args"].as_array().into_iter().flatten() {
                type_guards(arg, symbols, scope);
            }
        }
        Some("isA") => {
            if let Some(variable) = node["target"].get("ident").and_then(Json::as_str)
                && let Some(name) = node["args"]
                    .get(0)
                    .and_then(|a| a.get("literal"))
                    .and_then(Json::as_str)
                && symbols.is_type(name)
            {
                scope
                    .variables
                    .insert(variable.to_owned(), Ty::Element(name.to_owned()));
            }
        }
        _ => {}
    }
}

pub fn global_calls<'a>(node: &'a Json, out: &mut Vec<&'a Json>) {
    if let Some(call) = node.get("call").and_then(Json::as_str)
        && node.get("target").is_none()
        && !call.starts_with('_')
        && !call.starts_with('!')
        && !call.starts_with('-')
        && !call.starts_with('@')
    {
        out.push(node);
    }
    match node {
        Json::Object(map) => {
            for (key, value) in map {
                if key != "span" {
                    global_calls(value, out);
                }
            }
        }
        Json::Array(items) => {
            for item in items {
                global_calls(item, out);
            }
        }
        _ => {}
    }
}
