//! The parser over the construct list, and the span discipline the writers splice by: the line range each triple
//! occupies and, on a shared line, the exact fragment. Ported from the standalone host's `RdfParser.Tests.cs` at
//! `8b6ccfc`.

mod common;

use adp_diagram_rdf::lines::{LineDocument, LineRange};
use adp_diagram_rdf::model::{IriTerm, RdfModel, RdfTerm};
use adp_diagram_rdf::parser::{self, ParseError};
use adp_diagram_rdf::vocabulary::rdf;

fn parse(text: &str) -> RdfModel {
    parser::parse(&LineDocument::parse(text)).unwrap()
}

fn refuse(text: &str) -> ParseError {
    parser::parse(&LineDocument::parse(text)).unwrap_err()
}

#[test]
fn the_construct_corpus_parses_with_every_construct_accounted_for() {
    let model = parse(&common::fixture("constructs.ttl"));

    let prefixes: Vec<&str> = model.prefixes.iter().map(|p| p.prefix.as_str()).collect();
    assert_eq!(prefixes, ["ex", "foaf", "dc"]);
    assert_eq!(model.base_iri.as_deref(), Some("http://example.org/base/"));

    let r#type = model.triples.iter().find(|t| t.predicate.iri == rdf::TYPE).unwrap();
    assert_eq!(r#type.predicate.as_written, "a");
    assert_eq!(r#type.object.as_iri(), Some("http://xmlns.com/foaf/0.1/Person"));

    let alice = RdfTerm::Iri(IriTerm::new("http://example.org/alice", "ex:alice"));
    let knows = model
        .triples
        .iter()
        .filter(|t| t.subject == alice && t.predicate.iri == "http://xmlns.com/foaf/0.1/knows")
        .count();
    assert_eq!(knows, 2);

    let literal_of = |predicate: &str| {
        model
            .triples
            .iter()
            .find(|t| t.predicate.iri == predicate)
            .unwrap()
            .object
            .as_literal()
            .unwrap()
            .clone()
    };
    assert_eq!(
        literal_of("http://xmlns.com/foaf/0.1/age").datatype.as_deref(),
        Some(rdf::XSD_INTEGER)
    );
    assert_eq!(
        literal_of("http://xmlns.com/foaf/0.1/score").datatype.as_deref(),
        Some(rdf::XSD_DOUBLE)
    );
    let tagged = literal_of("http://purl.org/dc/elements/1.1/description");
    assert_eq!(tagged.language.as_deref(), Some("en"));
    assert_eq!(tagged.lexical, "A description\nwith a break");

    let carol: Vec<_> = model
        .triples
        .iter()
        .filter(|t| t.subject.as_blank().is_some_and(|b| b.label.as_deref() == Some("c")))
        .collect();
    assert_eq!(carol.len(), 2);
    assert_eq!(carol[0].subject, carol[1].subject);

    assert!(model.triples.iter().any(|t| t.predicate.iri == rdf::FIRST));
    assert!(
        model
            .triples
            .iter()
            .any(|t| t.predicate.iri == rdf::REST && t.object.is_iri(rdf::NIL))
    );

    let dave = model
        .triples
        .iter()
        .find(|t| t.object.as_literal().is_some_and(|l| l.lexical == "Dave"))
        .unwrap();
    assert!(dave.subject.is_blank());

    assert!(
        model
            .triples
            .iter()
            .any(|t| t.subject.is_iri("http://example.org/base/relative"))
    );
    assert!(
        model
            .triples
            .iter()
            .any(|t| t.object.is_iri("http://example.org/base/"))
    );

    let title = literal_of("http://purl.org/dc/elements/1.1/title");
    assert_eq!(title.lexical, "A long\r\nstring with \"quotes\" inside");

    assert!(model.triples.iter().any(|t| {
        t.object
            .as_literal()
            .is_some_and(|l| l.lexical == "true" && l.datatype.as_deref() == Some(rdf::XSD_BOOLEAN))
    }));
}

#[test]
fn n_triples_parse_through_the_same_door() {
    let model = parse(&common::fixture("simple.nt"));

    assert_eq!(model.triples.len(), 4);
    for t in &model.triples {
        assert_eq!(t.span.start_line, t.span.end_line);
        assert_eq!(t.statement, LineRange::new(t.span.start_line, t.span.start_line));
        assert!(!t.span.owns_lines());
    }
    assert!(
        model
            .triples
            .iter()
            .any(|t| t.subject.as_blank().is_some_and(|b| b.label.as_deref() == Some("b0")))
    );
}

#[test]
fn a_single_line_statement_records_its_fragment_beside_its_subject() {
    let model = parse("@prefix ex: <http://example.org/> .\r\nex:s ex:p ex:o .\r\n");

    assert_eq!(model.triples.len(), 1);
    let triple = &model.triples[0];
    assert_eq!(triple.span.start_line, 1);
    assert_eq!(triple.span.end_line, 1);
    assert_eq!(triple.span.fragment_start, Some(5));
    assert_eq!(triple.span.fragment_end, Some(14));
    assert_eq!(triple.statement, LineRange::new(1, 1));
}

#[test]
fn a_predicate_list_continuation_on_its_own_line_owns_that_line() {
    let model = parse("@prefix ex: <http://example.org/> .\r\nex:s ex:p1 ex:o1 ;\r\n     ex:p2 ex:o2 .\r\n");

    assert_eq!(model.triples.len(), 2);
    assert!(!model.triples[0].span.owns_lines());
    assert_eq!(model.triples[0].span.fragment_start, Some(5));
    assert!(model.triples[1].span.owns_lines());
    assert_eq!(model.triples[1].span.start_line, 2);
    assert_eq!(model.triples[1].span.end_line, 2);
    assert_eq!(model.triples[0].statement, LineRange::new(1, 2));
    assert_eq!(model.triples[1].statement, LineRange::new(1, 2));
}

#[test]
fn an_object_list_on_one_line_gives_each_triple_its_own_fragment() {
    let model = parse("@prefix ex: <http://example.org/> .\r\nex:s ex:p ex:o1, ex:o2 .\r\n");

    assert_eq!(model.triples.len(), 2);
    let fragment = |i: usize| (model.triples[i].span.fragment_start, model.triples[i].span.fragment_end);
    assert_eq!(fragment(0), (Some(5), Some(15)));
    assert_eq!(fragment(1), (Some(17), Some(22)));
}

#[test]
fn a_triple_spanning_lines_carries_no_fragment() {
    let model = parse("@prefix ex: <http://example.org/> .\r\nex:s ex:p \"\"\"one\r\ntwo\"\"\" .\r\n");

    assert_eq!(model.triples.len(), 1);
    let triple = &model.triples[0];
    assert_eq!(triple.span.start_line, 1);
    assert_eq!(triple.span.end_line, 2);
    assert!(triple.span.owns_lines());
}

#[test]
fn blank_node_ordinals_are_deterministic_across_parses() {
    let text = common::fixture("constructs.ttl");
    let blanks = |model: &RdfModel| -> Vec<(Option<String>, usize)> {
        model
            .triples
            .iter()
            .filter_map(|t| t.subject.as_blank())
            .map(|b| (b.label.clone(), b.ordinal))
            .collect()
    };
    assert_eq!(blanks(&parse(&text)), blanks(&parse(&text)));
}

#[test]
fn a_carriage_return_inside_a_short_string_is_refused_naming_its_line() {
    let error = refuse("@prefix ex: <http://example.org/> .\r\nex:s ex:p \"a\rb\" .\r\n");

    assert_eq!(error.line, 2);
    assert!(error.message.contains("end of its line"), "{}", error.message);
}

#[test]
fn an_escape_turtle_does_not_know_is_refused_naming_its_line() {
    for (literal, message) in [
        ("\"a\\qb\"", "'\\q' is not a string escape"),
        ("\"\\u12zz\"", "'\\u12zz' is not an escape"),
    ] {
        let error = refuse(&format!("@prefix ex: <http://example.org/> .\nex:s ex:p {literal} .\n"));
        assert_eq!(error.line, 2);
        assert!(error.message.starts_with(message), "{}", error.message);
    }
}

#[test]
fn an_undeclared_prefix_is_refused_naming_its_line() {
    let error = refuse("@prefix ex: <http://example.org/> .\r\nex:s nope:p ex:o .\r\n");

    assert_eq!(error.line, 2);
    assert!(error.message.contains("nope"));
}

#[test]
fn sparql_style_directives_work_without_their_dot() {
    let model = parse("PREFIX ex: <http://example.org/>\r\nBASE <http://example.org/base/>\r\nex:s ex:p <thing> .\r\n");

    assert_eq!(model.triples.len(), 1);
    assert_eq!(model.triples[0].object.as_iri(), Some("http://example.org/base/thing"));
}

#[test]
fn without_a_base_a_relative_iri_stays_as_written() {
    let model = parse("@prefix ex: <http://example.org/> .\r\nex:s ex:p <thing> .\r\n");

    assert_eq!(model.triples.len(), 1);
    assert_eq!(model.triples[0].object.as_iri(), Some("thing"));
}

#[test]
fn a_collection_expands_to_its_cons_pairs() {
    let model = parse("@prefix ex: <http://example.org/> .\r\nex:s ex:p ( ex:a ex:b ) .\r\n");

    assert_eq!(model.triples.len(), 5);
    assert_eq!(
        model.triples.iter().filter(|t| t.predicate.iri == rdf::FIRST).count(),
        2
    );
    assert_eq!(model.triples.iter().filter(|t| t.predicate.iri == rdf::REST).count(), 2);
    assert_eq!(model.triples.iter().filter(|t| t.object.is_iri(rdf::NIL)).count(), 1);
}
