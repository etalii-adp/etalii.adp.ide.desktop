//! The SPARQL query's persistence plugin (`reader.sparql`; etalii.adp spec 035, T132): every entry a stored record with
//! the span it is written at, and a text it cannot read one `std.unparseable` at the line and column it stopped at.

use adp_diagram_sparql::plugin::SparqlQueryPlugin;
use adp_fbl::plugins::{PersistencePlugin, PluginFile, PluginReadRequest};

fn read(text: &str) -> adp_fbl::plugins::PluginReadResult {
    SparqlQueryPlugin.read(&PluginReadRequest {
        files: vec![PluginFile::new("", text.as_bytes().to_vec())],
        supplementary_entries: Vec::new(),
    })
}

#[test]
fn every_entry_is_a_record_whose_span_slices_to_what_it_says() {
    let text = "PREFIX ex: <http://example.org/>\nSELECT ?s WHERE {\n  ?s ex:p \"é\" .\n  OPTIONAL { ?s ex:q ?o }\n  FILTER(?o > 1)\n}\n";
    let read = read(text);
    assert!(!read.unreadable);
    let ids: Vec<&str> = read.elements.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "query",
            "prefix-0",
            "pattern-0",
            "where/optional.0",
            "pattern-1",
            "where/constraint.0"
        ]
    );
    let slice = |id: &str| {
        let element = read.elements.iter().find(|e| e.id == id).unwrap();
        (&text[element.own_span.start..element.own_span.end], element.line)
    };
    assert_eq!(slice("prefix-0"), ("PREFIX ex: <http://example.org/>", 1));
    assert_eq!(slice("pattern-0"), ("?s ex:p \"é\"", 3));
    assert_eq!(slice("where/optional.0"), ("OPTIONAL { ?s ex:q ?o }", 4));
    assert_eq!(slice("where/constraint.0"), ("FILTER(?o > 1)", 5));
    let optional = read.elements.iter().find(|e| e.id == "pattern-1").unwrap();
    assert_eq!(optional.parent_id, None, "a pattern is no child record");
}

#[test]
fn a_text_it_cannot_read_is_one_finding_at_its_line_and_column() {
    let read = read("SELECT ?s WHERE {\n  ?s ?p\n");
    assert!(read.unreadable);
    assert!(read.elements.is_empty());
    assert_eq!(read.findings.len(), 1);
    assert_eq!(read.findings[0].code, "std.unparseable");
    let location = read.findings[0].location.as_ref().unwrap();
    assert!(location.line >= 2 && location.column >= 1, "{location:?}");
}
