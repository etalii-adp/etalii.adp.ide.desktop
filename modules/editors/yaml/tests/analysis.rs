//! The YAML editor's analysis (etalii.adp spec 030, FR-020, task T033) against the standalone host's YAML editor
//! examples at `8b6ccfc` (`src/editors/yaml/examples/`), copied into `tests/examples/`.

use std::collections::HashMap;
use std::sync::Arc;

use adp_editor_yaml::{Severity, analyse, split_documents};

const SCHEMA: &str = include_str!("examples/service.schema.json");

fn project() -> adp_editor_json::ReadFile {
    let files: HashMap<String, String> = [("service.schema.json".to_owned(), SCHEMA.to_owned())].into();
    Arc::new(move |path: &[String]| files.get(&path.join("/")).cloned())
}

fn doc(name: &str) -> Vec<String> {
    vec![name.to_owned()]
}

#[test]
fn the_service_example_is_clean_and_names_its_schema() {
    let analysis = analyse(include_str!("examples/service.yaml"), &doc("service.yaml"), project());
    assert!(analysis.findings.is_empty(), "{:?}", analysis.findings);
    assert_ne!(analysis.status, "No schema");
}

#[test]
fn the_broken_service_example_breaks_its_schema_in_several_places() {
    let text = include_str!("examples/service-broken.yaml");
    let analysis = analyse(text, &doc("service-broken.yaml"), project());
    let errors: Vec<&str> = analysis
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .map(|f| &text[f.span.clone()])
        .collect();
    assert!(errors.len() >= 4, "{:?}", analysis.findings);
    assert!(errors.iter().any(|e| e.contains("replicas")), "{errors:?}");
}

#[test]
fn several_documents_are_read_each_with_its_own_schema_line() {
    let text = include_str!("examples/several-documents.yaml");
    let parts = split_documents(text);
    assert_eq!(parts.len(), 3, "{parts:?}");
    assert_eq!(parts[0].schema_reference.as_deref(), Some("service.schema.json"));
    assert_eq!(parts[1].schema_reference.as_deref(), Some("service.schema.json"));
    assert_eq!(parts[2].schema_reference, None);
    let analysis = analyse(text, &doc("several-documents.yaml"), project());
    assert!(analysis.findings.is_empty(), "{:?}", analysis.findings);
}

#[test]
fn anchors_aliases_and_merge_keys_are_read_as_data() {
    let analysis = analyse(include_str!("examples/anchors.yaml"), &doc("anchors.yaml"), project());
    assert!(analysis.findings.is_empty(), "{:?}", analysis.findings);
    assert_eq!(analysis.status, "No schema");
}

#[test]
fn the_mistakes_of_yaml_itself_are_found() {
    let text = "a: 1\na: 2\nb: *nowhere\nc:\n\t- x\n";
    let analysis = analyse(text, &doc("x.yaml"), project());
    let messages: Vec<&str> = analysis.findings.iter().map(|f| f.message.as_str()).collect();
    assert!(
        messages.contains(&"Tabs are not allowed as indentation; indent with spaces."),
        "{messages:?}"
    );
}

#[test]
fn a_repeated_key_and_an_unknown_alias_are_errors() {
    let analysis = analyse("a: 1\na: 2\n", &doc("x.yaml"), project());
    assert!(
        analysis.findings.iter().any(|f| f.message == "Map keys must be unique"),
        "{:?}",
        analysis.findings
    );
    let analysis = analyse("b: *nowhere\n", &doc("x.yaml"), project());
    assert!(
        analysis.findings.iter().any(|f| f.severity == Severity::Error),
        "{:?}",
        analysis.findings
    );
}

#[test]
fn a_schema_on_the_web_is_refused_on_its_line() {
    let text = "# yaml-language-server: $schema=https://example.com/s.json\na: 1\n";
    let analysis = analyse(text, &doc("x.yaml"), project());
    let info = analysis.findings.iter().find(|f| f.severity == Severity::Info).unwrap();
    assert!(info.message.contains("is a web address"), "{}", info.message);
    assert!(text[info.span.clone()].starts_with("# yaml-language-server"));
}
