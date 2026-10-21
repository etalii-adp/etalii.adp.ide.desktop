//! The XML editor's analysis (etalii.adp spec 030, FR-020, task T032) against the standalone host's XML editor examples
//! at `8b6ccfc` (`src/editors/xml/examples/`), copied into `tests/examples/`.

use adp_editor_xml::{NOT_CHECKED, SchemaKind, Severity, analyse};

fn doc(name: &str) -> Vec<String> {
    vec![name.to_owned()]
}

#[test]
fn a_document_without_a_schema_is_well_formed_and_says_so() {
    let analysis = analyse(include_str!("examples/notes.xml"), &doc("notes.xml"), "UTF-8");
    assert!(analysis.findings.is_empty(), "{:?}", analysis.findings);
    assert_eq!(analysis.status, "No schema");
}

#[test]
fn the_schemas_a_document_names_are_found_where_they_stand() {
    let text = include_str!("examples/library.xml");
    let analysis = analyse(text, &doc("library.xml"), "UTF-8");
    assert_eq!(analysis.schemas.len(), 1);
    assert_eq!(analysis.schemas[0].location, "library.xsd");
    assert_eq!(&text[analysis.schemas[0].span.clone()], "library.xsd");
    assert!(analysis.status.contains("library.xsd"), "{}", analysis.status);

    let text = include_str!("examples/order.xml");
    let analysis = analyse(text, &doc("order.xml"), "UTF-8");
    let named: Vec<(&str, &str)> = analysis
        .schemas
        .iter()
        .map(|s| (s.namespace.as_str(), s.location.as_str()))
        .collect();
    assert_eq!(
        named,
        [
            ("urn:example:orders", "order.xsd"),
            ("urn:example:parties", "party.xsd")
        ]
    );

    let text = include_str!("examples/recipe.xml");
    let analysis = analyse(text, &doc("recipe.xml"), "UTF-8");
    assert_eq!(analysis.schemas[0].kind, SchemaKind::Dtd);
    assert_eq!(analysis.schemas[0].location, "recipe.dtd");
}

#[test]
fn a_mistake_stops_the_reading_with_one_finding_where_it_stopped() {
    let text = "<?xml version=\"1.0\"?>\n<a>\n  <b></c>\n</a>\n";
    let analysis = analyse(text, &doc("x.xml"), "UTF-8");
    assert_eq!(analysis.findings.len(), 1, "{:?}", analysis.findings);
    assert_eq!(analysis.findings[0].severity, Severity::Error);
    assert!(analysis.findings[0].span.start >= text.find("<b>").unwrap());
    assert_eq!(analysis.status, NOT_CHECKED);
}

#[test]
fn a_schema_on_the_web_is_named_and_not_followed() {
    let text = "<a xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:noNamespaceSchemaLocation=\"https://example.com/a.xsd\"/>";
    let analysis = analyse(text, &doc("a.xml"), "UTF-8");
    assert_eq!(analysis.findings.len(), 1);
    assert_eq!(
        analysis.findings[0].message,
        "'https://example.com/a.xsd' is not in the project, so no schema is checked from it."
    );
    assert_eq!(analysis.status, "No schema");
}

#[test]
fn a_declared_encoding_that_differs_from_the_stored_one_is_a_warning() {
    let text = "<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?>\n<a>café</a>\n";
    let analysis = analyse(text, &doc("a.xml"), "UTF-8");
    let warning = analysis
        .findings
        .iter()
        .find(|f| f.severity == Severity::Warning)
        .unwrap();
    assert_eq!(
        warning.message,
        "The declaration says the file is ISO-8859-1, but it is stored as UTF-8."
    );
    assert_eq!(&text[warning.span.clone()], "ISO-8859-1");
    let ascii = "<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?>\n<a>cafe</a>\n";
    assert!(analyse(ascii, &doc("a.xml"), "UTF-8").findings.is_empty());
}
