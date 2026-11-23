//! An editor's description validated against its cases (etalii.adp spec 083): no host runs an editor from its
//! description (ATD §1.2), so validating one is reading its case's document as the description says and comparing
//! what that gives with the case. The document is held as the description's `open` holds a file (`adp-documents`'
//! `TextFile`, from the case's bytes, nothing read from disk), its outline read through its outline binding's reader
//! capability, and its findings given by the host's check source that covers the names its `check with` lists, each at
//! its 1-based line and its column in characters (the description's `findings` setting).

use std::collections::BTreeMap;

use adp_atd_check::Description;
use adp_atd_runtime::RuntimeHost;
use adp_fbl::plugins::{PluginFile, PluginReadRequest};
use serde_json::{Value as Json, json};

use crate::{Case, Difference};

/// The surface of the description, `surface text { … }`.
fn surface(description: &Description) -> Option<&Json> {
    description.members().find(|m| m["kind"] == "textSurface")
}

/// The names of the sources the description's checks name, in its order.
fn check_names(surface: &Json) -> Vec<String> {
    surface["members"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|m| m["kind"] == "check")
        .filter_map(|m| m["source"]["named"].as_str().map(str::to_owned))
        .collect()
}

/// The outline's element type and the capability of the binding that reads it.
fn outline(description: &Description, surface: &Json) -> Option<(String, String)> {
    let item = surface["members"].as_array()?.iter().find(|m| m["kind"] == "outline")?;
    let type_name = item["names"][0]["text"].as_str()?.to_owned();
    let binding = description
        .members()
        .find(|m| m["kind"] == "binding" && m["family"] == "capability")?;
    Some((type_name, binding["capability"]["text"].as_str()?.to_owned()))
}

/// The 1-based line and column, in characters, of byte `offset` of `text` (ATD §3.1), as the case records it.
fn position(text: &str, offset: usize) -> (usize, usize) {
    let before = text.get(..offset.min(text.len())).unwrap_or(text);
    let line = before.matches('\n').count() + 1;
    let start = before.rfind('\n').map_or(0, |at| at + 1);
    (line, before[start..].chars().count() + 1)
}

/// An attribute's value as a case records it: a number as a number, text as text.
fn json_of(value: &adp_cel::Value) -> Json {
    match value {
        adp_cel::Value::Int(n) => json!(n),
        adp_cel::Value::Double(n) => json!(n),
        adp_cel::Value::Bool(b) => json!(b),
        adp_cel::Value::String(text) => json!(text),
        adp_cel::Value::Null => Json::Null,
        other => json!(other.to_string()),
    }
}

/// The differences of `case`, an editor's, on `host`.
pub fn validate(description: &Description, host: &RuntimeHost, case: &Case) -> Vec<Difference> {
    let mut differences = Vec::new();
    let mut differ = |part: &'static str, expected: String, actual: String| {
        differences.push(Difference {
            step: None,
            part,
            expected,
            actual,
            at: None,
        })
    };
    let Some(surface) = surface(description) else {
        differ("reading", "a text surface".into(), "none in the description".into());
        return differences;
    };
    let body = case.fixture["inputs"]["body"].as_str().unwrap_or_default();
    let Some(bytes) = case.files.get(body) else {
        differ(
            "reading",
            format!("the document {body}"),
            "no such file in the case".into(),
        );
        return differences;
    };
    let read = &case.fixture["read"];
    let file = match adp_documents::TextFile::from_bytes(body, bytes) {
        Ok(file) => file,
        Err(refusal) => {
            let expected = read["refused"]
                .as_str()
                .map_or("the document opened".to_owned(), str::to_owned);
            if read["refused"].as_str() != Some(refusal.as_str()) {
                differ("refused", expected, refusal);
            }
            return differences;
        }
    };
    let text = file.content();

    // The outline, through its binding's reader.
    let mut elements = Vec::new();
    if let Some((type_name, capability)) = outline(description, surface) {
        match host.capabilities.reader(&capability) {
            Some(reader) => {
                let result = reader.read(&PluginReadRequest {
                    files: vec![PluginFile::new(body, text.as_bytes().to_vec())],
                    supplementary_entries: Vec::new(),
                });
                for element in result.elements {
                    let attributes: serde_json::Map<String, Json> = element
                        .attributes
                        .iter()
                        .map(|(name, value)| (name.clone(), json_of(value)))
                        .collect();
                    elements.push(json!({"id": element.id, "type": type_name, "attributes": attributes}));
                }
            }
            None => differ(
                "reading",
                format!("the capability {capability}"),
                "missing from this host".into(),
            ),
        }
    }
    let expected_elements = if read["elements"].is_null() {
        json!([])
    } else {
        read["elements"].clone()
    };
    if expected_elements != Json::Array(elements.clone()) {
        differ(
            "reading",
            expected_elements.to_string(),
            Json::Array(elements).to_string(),
        );
    }

    // The findings, from the host's source for the checks the description names.
    let names = check_names(surface);
    let mut findings = Vec::new();
    if !names.is_empty() {
        let wanted: Vec<&str> = names.iter().map(String::as_str).collect();
        match host.capabilities.check_source(&wanted) {
            Some(source) => {
                // The case's other files, as text, by their path in the case: what a schema reference may name.
                let files: BTreeMap<String, String> = case
                    .files
                    .iter()
                    .filter(|(name, _)| *name != "conformance.json")
                    .filter_map(|(name, bytes)| Some((name.clone(), String::from_utf8(bytes.clone()).ok()?)))
                    .collect();
                for finding in source.check(text, body, file.encoding_name(), &files) {
                    let (line, column) = position(text, finding.start);
                    findings.push(json!({
                        "severity": finding.severity, "line": line, "column": column, "message": finding.message,
                    }));
                }
            }
            // A source the host lacks is said once, rather than as every finding it would have given (FR-005).
            None => {
                differ(
                    "findings",
                    format!("the check sources {}", names.join(", ")),
                    "missing from this host".into(),
                );
                return differences;
            }
        }
    }
    let expected_findings = if read["findings"].is_null() {
        json!([])
    } else {
        read["findings"].clone()
    };
    if expected_findings != Json::Array(findings.clone()) {
        differ(
            "findings",
            expected_findings.to_string(),
            Json::Array(findings).to_string(),
        );
    }
    differences
}
