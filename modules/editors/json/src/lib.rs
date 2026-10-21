//! The JSON editor's analysis (etalii.adp spec 030, FR-020, task T031), after the standalone host's JSON editor at
//! `8b6ccfc` (json-editor, `client/jsonAnalysis.ts`): a `.json` file is checked for the mistakes of JSON itself,
//! checked against the JSON Schema its root `$schema` names, and completed from that schema.
//!
//! - **Findings**: a syntax error, a comment or a trailing comma (error); a repeated property name (warning); a break
//!   of the schema (error); a schema that cannot be used, with the reason (information, once, on the `$schema` line).
//! - **The schema** is read from the project only: relative to the document's folder, never above the project's root,
//!   never from the network; a `$ref` inside it by the same rules. A document whose `$schema` is a JSON Schema
//!   dialect's own address is a schema, checked against that dialect.
//! - **The status line** says the schema's title, else its reference; "No schema"; why nothing is checked; or "Too
//!   large to check" above 1,000,000 characters.
//! - **Completion** offers, where a property name may begin, the schema's properties not yet present, required ones
//!   first; and where a value may begin, the values the schema limits it to.
//!
//! The standalone host words its findings as `vscode-json-languageservice` does; the wording here is this crate's own.

pub mod parse;

use std::ops::Range;
use std::sync::Arc;

use jsonschema::{Retrieve, Uri};
use serde_json::Value;

use parse::{Node, parse};

/// Above this many characters nothing is checked, as in the standalone host.
pub const LARGEST_CHECKED_TEXT: usize = 1_000_000;

const PROJECT_SCHEME: &str = "adp-project:";
const DIALECTS: &[&str] = &[
    "https://json-schema.org/draft-04/schema",
    "https://json-schema.org/draft-06/schema",
    "https://json-schema.org/draft-07/schema",
    "https://json-schema.org/draft/2019-09/schema",
    "https://json-schema.org/draft/2020-12/schema",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// One finding, by byte span in the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub span: Range<usize>,
    pub severity: Severity,
    pub message: String,
}

/// Reads a file of the project by its path segments, or `None` when it cannot be read.
pub type ReadFile = Arc<dyn Fn(&[String]) -> Option<String> + Send + Sync>;

/// What the editor knows about the text.
#[derive(Debug, Clone, PartialEq)]
pub struct Analysis {
    pub findings: Vec<Finding>,
    pub status: String,
    /// The schema that applies, for completion.
    pub schema: Option<Value>,
}

/// Where a schema reference leads (the standalone host's `resolveSchemaReference`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaTarget {
    /// A file of the project, by its path segments.
    Project(Vec<String>),
    /// A schema ADP ships, by the address a document names it with. ADP ships none yet; the rule is kept so the
    /// standalone host's shared cases hold here too.
    Bundled(String),
    /// No schema, and why.
    None(String),
}

/// Resolves a reference against the document's path (file name last), never leaving the project.
pub fn resolve_schema_reference(reference: &str, document: &[String]) -> SchemaTarget {
    resolve_schema_reference_with(reference, document, &[])
}

/// As [`resolve_schema_reference`], with the addresses of the schemas ADP ships.
pub fn resolve_schema_reference_with(reference: &str, document: &[String], bundled: &[&str]) -> SchemaTarget {
    let trimmed = reference.trim();
    if trimmed.is_empty() {
        return SchemaTarget::None("is empty".into());
    }
    let address = trimmed.split('#').next().unwrap_or_default();
    if bundled.contains(&address) {
        return SchemaTarget::Bundled(address.to_owned());
    }
    let has_scheme = address.split_once(':').is_some_and(|(s, _)| {
        s.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
            && s.chars().all(|c| c.is_ascii_alphanumeric() || "+.-".contains(c))
    });
    if has_scheme {
        let lower = address.to_ascii_lowercase();
        return SchemaTarget::None(if lower.starts_with("http:") || lower.starts_with("https:") {
            "is a web address; ADP reads schemas from the project".into()
        } else {
            "is not a file of the project".into()
        });
    }
    if address.starts_with('/') {
        return SchemaTarget::None("is an absolute path; ADP reads a schema relative to the document".into());
    }
    let segments: Vec<&str> = address.split('/').collect();
    if matches!(segments.last(), Some(&"" | &"." | &"..")) {
        return SchemaTarget::None("names a folder, not a file".into());
    }
    let mut resolved: Vec<String> = document[..document.len().saturating_sub(1)].to_vec();
    for segment in segments {
        match segment {
            "" | "." => {}
            ".." => {
                if resolved.pop().is_none() {
                    return SchemaTarget::None("leaves the project".into());
                }
            }
            other => resolved.push(other.to_owned()),
        }
    }
    SchemaTarget::Project(resolved)
}

/// Analyses a JSON text; `document` is its path in the project, its file name last.
pub fn analyse(text: &str, document: &[String], read: ReadFile) -> Analysis {
    if text.len() > LARGEST_CHECKED_TEXT {
        return Analysis {
            findings: Vec::new(),
            status: "Too large to check".into(),
            schema: None,
        };
    }
    let parsed = parse(text);
    let mut findings: Vec<Finding> = parsed
        .mistakes
        .iter()
        .map(|m| Finding {
            span: m.span.clone(),
            severity: if m.warning { Severity::Warning } else { Severity::Error },
            message: m.message.clone(),
        })
        .collect();
    let Some(root) = parsed.root else {
        return Analysis {
            findings,
            status: "No schema".into(),
            schema: None,
        };
    };
    let Some((reference, key_span)) = schema_property(&root) else {
        return Analysis {
            findings,
            status: "No schema".into(),
            schema: None,
        };
    };
    let instance = root.to_value();
    let check = check_against_schema(&reference, document, read, instance, true);
    for (pointer, message) in check.breaks {
        findings.push(schema_finding(&root, &pointer, message));
    }
    if let Some(reason) = check.refusal {
        findings.push(Finding {
            span: key_span,
            severity: Severity::Info,
            message: reason,
        });
    }
    Analysis {
        findings,
        status: check.status,
        schema: check.schema,
    }
}

/// What checking a value against the schema a reference names found.
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaCheck {
    /// The status line: the schema's title, else its reference, or "Schema not used".
    pub status: String,
    /// Each break of the schema, by the JSON pointer of the value it is about.
    pub breaks: Vec<(String, String)>,
    /// Why no schema was used, as a sentence, when one was named but could not be.
    pub refusal: Option<String>,
    /// The schema that applied, for completion.
    pub schema: Option<Value>,
}

/// Checks `instance` against the schema `reference` names, read from the project relative to `document` (its file
/// name last) by the rules above. With `dialect_is_schema`, a dialect's own address makes the instance a schema, checked
/// against that dialect, as a JSON file's `$schema` does; the root `$schema` property is not part of the data unless
/// the schema declares it. The YAML editor checks its documents with this too.
pub fn check_against_schema(
    reference: &str,
    document: &[String],
    read: ReadFile,
    instance: Value,
    dialect_is_schema: bool,
) -> SchemaCheck {
    let normalised = normalise(reference);
    if dialect_is_schema && DIALECTS.contains(&normalised.as_str()) {
        let breaks = match jsonschema::meta::validate(&instance) {
            Err(error) => vec![(error.instance_path().to_string(), error.to_string())],
            Ok(()) => Vec::new(),
        };
        let status = instance
            .get("title")
            .and_then(Value::as_str)
            .map_or_else(|| reference.to_owned(), str::to_owned);
        return SchemaCheck {
            status,
            breaks,
            refusal: None,
            schema: None,
        };
    }
    let refuse = |reason: String| SchemaCheck {
        status: "Schema not used".into(),
        breaks: Vec::new(),
        refusal: Some(reason),
        schema: None,
    };
    if normalised.starts_with("https://json-schema.org/") {
        return refuse(format!(
            "The schema '{reference}' is a JSON Schema dialect ADP does not carry."
        ));
    }
    let path = match resolve_schema_reference(reference, document) {
        SchemaTarget::None(why) => return refuse(format!("The schema '{reference}' {why}.")),
        SchemaTarget::Bundled(address) => return refuse(format!("The schema '{address}' is not shipped with ADP.")),
        SchemaTarget::Project(path) => path,
    };
    let Some(schema_text) = read(&path) else {
        return refuse(format!("The schema '{reference}' could not be read."));
    };
    let schema: Value = match serde_json::from_str(&schema_text) {
        Ok(schema) => schema,
        Err(e) => return refuse(format!("The schema '{reference}' is not valid JSON: {e}.")),
    };
    let base = format!("{PROJECT_SCHEME}/{}", path.join("/"));
    let validator = match jsonschema::options()
        .with_base_uri(base)
        .with_retriever(ProjectRetriever { read: read.clone() })
        .build(&schema)
    {
        Ok(validator) => validator,
        Err(e) => return refuse(format!("The schema '{reference}' cannot be used: {e}.")),
    };
    // The root `$schema` names the schema and is not part of the data, as VS Code's JSON service treats it, unless
    // the schema itself declares the property.
    let mut instance = instance;
    let declared = schema.get("properties").and_then(|p| p.get("$schema")).is_some();
    if let (false, Some(object)) = (declared, instance.as_object_mut()) {
        object.remove("$schema");
    }
    let breaks = validator
        .iter_errors(&instance)
        .map(|e| (e.instance_path().to_string(), e.to_string()))
        .collect();
    let status = schema
        .get("title")
        .and_then(Value::as_str)
        .map_or_else(|| reference.to_owned(), str::to_owned);
    SchemaCheck {
        status,
        breaks,
        refusal: None,
        schema: Some(schema),
    }
}

/// What may be written at a byte offset: property names not yet present (required first), or a value's allowed values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completion {
    pub label: String,
    /// The text to insert, well-formed where it lands.
    pub insert: String,
    pub detail: String,
}

pub fn completions(text: &str, offset: usize, schema: &Value) -> Vec<Completion> {
    let Some(root) = parse(text).root else {
        return Vec::new();
    };
    let Some((path, place)) = locate(&root, offset, Vec::new()) else {
        return Vec::new();
    };
    let Some(sub) = subschema(schema, schema, &path) else {
        return Vec::new();
    };
    match place {
        Place::Name(present) => {
            let Some(properties) = sub.get("properties").and_then(Value::as_object) else {
                return Vec::new();
            };
            let required: Vec<&str> = sub
                .get("required")
                .and_then(Value::as_array)
                .map(|r| r.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            let mut names: Vec<(&String, &Value)> =
                properties.iter().filter(|(name, _)| !present.contains(name)).collect();
            names.sort_by_key(|(name, _)| !required.contains(&name.as_str()));
            names
                .into_iter()
                .map(|(name, property)| Completion {
                    label: name.clone(),
                    insert: format!("\"{name}\": "),
                    detail: describe(resolve(schema, property)),
                })
                .collect()
        }
        Place::Value => {
            let sub = resolve(schema, sub);
            let mut values: Vec<Value> = sub.get("enum").and_then(Value::as_array).cloned().unwrap_or_default();
            if let Some(constant) = sub.get("const") {
                values.push(constant.clone());
            }
            if values.is_empty() && sub.get("type").and_then(Value::as_str) == Some("boolean") {
                values = vec![Value::Bool(true), Value::Bool(false)];
            }
            values
                .into_iter()
                .map(|v| Completion {
                    label: v.to_string(),
                    insert: v.to_string(),
                    detail: describe(sub),
                })
                .collect()
        }
    }
}

enum Place {
    /// Where a property name may begin, with the names already present.
    Name(Vec<String>),
    Value,
}

/// The path of keys to the innermost object or value around the offset, and what may begin there.
fn locate(node: &Node, offset: usize, path: Vec<String>) -> Option<(Vec<String>, Place)> {
    let span = node.span();
    if offset <= span.start || offset > span.end {
        return None;
    }
    match node {
        Node::Object { members, .. } => {
            let names = || members.iter().map(|m| m.key.clone()).collect();
            for member in members {
                let mut inner = path.clone();
                inner.push(member.key.clone());
                match &member.value {
                    Some(value) => {
                        if let Some(found) = locate(value, offset, inner.clone()) {
                            return Some(found);
                        }
                        if offset > member.key_span.end && offset <= value.span().start {
                            return Some((inner, Place::Value));
                        }
                    }
                    None if offset > member.key_span.end => return Some((inner, Place::Value)),
                    None => {}
                }
                if offset > member.key_span.start && offset < member.key_span.end {
                    return Some((path, Place::Name(names())));
                }
            }
            Some((path, Place::Name(names())))
        }
        Node::Array { items, .. } => items.iter().enumerate().find_map(|(index, item)| {
            let mut inner = path.clone();
            inner.push(index.to_string());
            locate(item, offset, inner)
        }),
        _ => Some((path, Place::Value)),
    }
}

/// The schema for a path of keys, following `properties`, `items` and local `$ref`s.
fn subschema<'s>(root: &'s Value, schema: &'s Value, path: &[String]) -> Option<&'s Value> {
    let mut current = resolve(root, schema);
    for key in path {
        current = resolve(root, current);
        current = match key.parse::<usize>() {
            Ok(_) if current.get("items").is_some() => current.get("items")?,
            _ => current
                .get("properties")
                .and_then(|p| p.get(key))
                .or_else(|| current.get("additionalProperties").filter(|a| a.is_object()))?,
        };
    }
    Some(resolve(root, current))
}

/// Follows a local `$ref` (`#/...`) to what it points at.
fn resolve<'s>(root: &'s Value, schema: &'s Value) -> &'s Value {
    let mut current = schema;
    for _ in 0..16 {
        match current
            .get("$ref")
            .and_then(Value::as_str)
            .and_then(|r| r.strip_prefix('#'))
        {
            Some(pointer) => match root.pointer(pointer) {
                Some(target) => current = target,
                None => break,
            },
            None => break,
        }
    }
    current
}

fn describe(schema: &Value) -> String {
    schema
        .get("description")
        .or_else(|| schema.get("title"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// The root object's `$schema` string, with the property key's span.
fn schema_property(root: &Node) -> Option<(String, Range<usize>)> {
    let Node::Object { members, .. } = root else {
        return None;
    };
    members.iter().find_map(|m| match (&m.key[..], &m.value) {
        ("$schema", Some(Node::String { value, .. })) => Some((value.clone(), m.key_span.clone())),
        _ => None,
    })
}

/// `http://json-schema.org/draft-07/schema#` and its https form are one address.
fn normalise(reference: &str) -> String {
    let trimmed = reference.trim().trim_end_matches('#');
    match trimmed.strip_prefix("http://json-schema.org/") {
        Some(rest) => format!("https://json-schema.org/{rest}"),
        None => trimmed.to_owned(),
    }
}

/// A schema break: on the key of the property it is about when there is one; for a whole object, on the property
/// the message names ("'region' was unexpected") or else on its opening brace, as VS Code marks it; otherwise on the
/// value.
fn schema_finding(root: &Node, pointer: &str, message: String) -> Finding {
    let span = match root.at_pointer(pointer) {
        Some((Node::Object { span, members }, _)) => message
            .split('\'')
            .skip(1)
            .step_by(2)
            .find_map(|name| members.iter().find(|m| m.key == name).map(|m| m.key_span.clone()))
            .unwrap_or(span.start..span.start + 1),
        Some((Node::Array { span, .. }, _)) => span.start..span.start + 1,
        Some((_, Some(key))) => key,
        Some((node, None)) => node.span(),
        None => root.span().start..root.span().start + 1,
    };
    Finding {
        span,
        severity: Severity::Error,
        message,
    }
}

/// Reads a `$ref` to another schema from the project, by the same rules as the document's `$schema`.
struct ProjectRetriever {
    read: ReadFile,
}

impl Retrieve for ProjectRetriever {
    fn retrieve(&self, uri: &Uri<String>) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let text = uri.as_str();
        let Some(path) = text.strip_prefix(PROJECT_SCHEME) else {
            return Err(format!("'{text}' is not a file of the project").into());
        };
        let segments: Vec<String> = path.trim_start_matches('/').split('/').map(str::to_owned).collect();
        let content = (self.read)(&segments).ok_or_else(|| format!("'{path}' could not be read"))?;
        Ok(serde_json::from_str(&content)?)
    }
}
