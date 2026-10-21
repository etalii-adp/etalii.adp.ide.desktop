//! The JSON editor's analysis (etalii.adp spec 030, FR-020, task T031) against the standalone host's JSON editor
//! examples at `8b6ccfc` (`src/editors/json/examples/`), copied into `tests/examples/`.

use std::collections::HashMap;
use std::sync::Arc;

use adp_editor_json::{SchemaTarget, Severity, analyse, completions, resolve_schema_reference};

fn project(files: &[(&str, &str)]) -> adp_editor_json::ReadFile {
    let files: HashMap<String, String> = files.iter().map(|(p, t)| (p.to_string(), t.to_string())).collect();
    Arc::new(move |path: &[String]| files.get(&path.join("/")).cloned())
}

fn doc(path: &str) -> Vec<String> {
    path.split('/').map(str::to_owned).collect()
}

const SCHEMA: &str = include_str!("examples/service.schema.json");
const SERVICE: &str = include_str!("examples/service.json");
const BROKEN: &str = include_str!("examples/service-broken.json");

#[test]
fn the_service_example_is_clean_and_names_its_schema() {
    let read = project(&[("examples/service.schema.json", SCHEMA)]);
    let analysis = analyse(SERVICE, &doc("examples/service.json"), read);
    assert!(analysis.findings.is_empty(), "{:?}", analysis.findings);
    assert_ne!(analysis.status, "No schema");
    assert!(analysis.schema.is_some());
}

#[test]
fn the_broken_service_example_has_findings_against_its_schema() {
    let read = project(&[("examples/service.schema.json", SCHEMA)]);
    let analysis = analyse(BROKEN, &doc("examples/service-broken.json"), read);
    assert!(
        analysis.findings.iter().any(|f| f.severity == Severity::Error),
        "{:?}",
        analysis.findings
    );
}

#[test]
fn json_mistakes_are_found_without_a_schema() {
    let text = "{\n  // a comment\n  \"a\": 1,\n  \"a\": 2,\n  \"b\": [1, 2,],\n}\n";
    let analysis = analyse(text, &doc("x.json"), project(&[]));
    let messages: Vec<&str> = analysis.findings.iter().map(|f| f.message.as_str()).collect();
    assert!(messages.contains(&"Comments are not permitted in JSON."));
    assert!(messages.contains(&"Duplicate object key."));
    assert_eq!(messages.iter().filter(|m| **m == "Trailing comma.").count(), 2);
    assert_eq!(analysis.status, "No schema");
    let duplicate = analysis
        .findings
        .iter()
        .find(|f| f.message == "Duplicate object key.")
        .unwrap();
    assert_eq!(duplicate.severity, Severity::Warning);
}

#[test]
fn a_schema_outside_the_project_or_on_the_web_is_refused_with_its_reason() {
    for (reference, reason) in [
        (
            "https://example.com/s.json",
            "is a web address; ADP reads schemas from the project",
        ),
        ("../../s.json", "leaves the project"),
        (
            "/etc/s.json",
            "is an absolute path; ADP reads a schema relative to the document",
        ),
        ("schemas/", "names a folder, not a file"),
    ] {
        let text = format!("{{ \"$schema\": \"{reference}\" }}");
        let analysis = analyse(&text, &doc("a/x.json"), project(&[]));
        assert_eq!(analysis.status, "Schema not used");
        let info = &analysis.findings[0];
        assert_eq!(info.severity, Severity::Info);
        assert!(info.message.ends_with(&format!("{reason}.")), "{}", info.message);
        assert_eq!(&text[info.span.clone()], "\"$schema\"");
    }
    assert_eq!(
        resolve_schema_reference("../s.json#/x", &doc("a/x.json")),
        SchemaTarget::Project(doc("s.json"))
    );
}

#[test]
fn a_schema_document_is_checked_against_its_dialect() {
    let text = "{ \"$schema\": \"https://json-schema.org/draft/2020-12/schema\", \"title\": \"Thing\", \"type\": 3 }";
    let analysis = analyse(text, &doc("thing.schema.json"), project(&[]));
    assert_eq!(analysis.status, "Thing");
    assert_eq!(analysis.findings.len(), 1);
}

#[test]
fn too_large_a_text_is_not_checked() {
    let text = format!("[{}0]", "0,".repeat(600_000));
    assert_eq!(
        analyse(&text, &doc("big.json"), project(&[])).status,
        "Too large to check"
    );
}

#[test]
fn completion_offers_missing_properties_required_first_and_enum_values() {
    let schema: serde_json::Value = serde_json::json!({
        "type": "object",
        "required": ["name"],
        "properties": {
            "port": { "type": "integer", "description": "The port." },
            "name": { "type": "string" },
            "mode": { "$ref": "#/$defs/Mode" }
        },
        "$defs": { "Mode": { "enum": ["fast", "safe"] } }
    });
    let text = "{ \"port\": 1,  }";
    let names: Vec<String> = completions(text, 13, &schema).into_iter().map(|c| c.label).collect();
    assert_eq!(names, ["name", "mode"]);
    let text = "{ \"mode\":  }";
    let values: Vec<String> = completions(text, 10, &schema).into_iter().map(|c| c.insert).collect();
    assert_eq!(values, ["\"fast\"", "\"safe\""]);
}
