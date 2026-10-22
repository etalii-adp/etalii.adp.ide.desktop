//! The writer's named operations as smallest diffs, and refusals landing before any splice. Ported from the standalone
//! host's `RdfWriter.Tests.cs` at `8b6ccfc`.

use adp_diagram_rdf::lines::LineDocument;
use adp_diagram_rdf::model::{BlankTerm, IriTerm, LiteralTerm, RdfModel, RdfTerm, RdfTriple};
use adp_diagram_rdf::parser;
use adp_diagram_rdf::writer;

const CORPUS: &str = "@prefix ex: <http://example.org/> .\r\n\r\nex:alice ex:knows ex:bob, ex:carol ;\r\n    ex:name \"Alice\" .\r\n\r\nex:bob ex:name \"Bob\" .\r\n";

fn load(text: &str) -> (LineDocument, RdfModel) {
    let document = LineDocument::parse(text);
    let model = parser::parse(&document).unwrap();
    (document, model)
}

fn find(model: &RdfModel, predicate: &str, object: &str) -> RdfTriple {
    let found: Vec<_> = model
        .triples
        .iter()
        .filter(|t| t.predicate.iri == predicate && t.object.is_iri(object))
        .collect();
    assert_eq!(found.len(), 1);
    found[0].clone()
}

fn literal_of(model: &RdfModel, subject: &str, lexical: Option<&str>) -> RdfTriple {
    let found: Vec<_> = model
        .triples
        .iter()
        .filter(|t| {
            t.subject.is_iri(subject)
                && t.object
                    .as_literal()
                    .is_some_and(|l| lexical.is_none_or(|x| l.lexical == x))
        })
        .collect();
    assert_eq!(found.len(), 1);
    found[0].clone()
}

#[test]
fn removing_a_comma_list_continuation_takes_the_object_and_its_preceding_comma_and_nothing_else() {
    let (mut document, model) = load(CORPUS);
    let triple = find(&model, "http://example.org/knows", "http://example.org/carol");

    assert_eq!(writer::remove_triple(&mut document, &model, &triple), "");
    assert_eq!(
        document.text(),
        "@prefix ex: <http://example.org/> .\r\n\r\nex:alice ex:knows ex:bob ;\r\n    ex:name \"Alice\" .\r\n\r\nex:bob ex:name \"Bob\" .\r\n"
    );
}

#[test]
fn removing_a_comma_list_head_takes_the_object_and_its_following_comma_leaving_the_predicate_for_the_rest() {
    let (mut document, model) = load(CORPUS);
    let triple = find(&model, "http://example.org/knows", "http://example.org/bob");

    assert_eq!(writer::remove_triple(&mut document, &model, &triple), "");
    assert_eq!(
        document.text(),
        "@prefix ex: <http://example.org/> .\r\n\r\nex:alice ex:knows ex:carol ;\r\n    ex:name \"Alice\" .\r\n\r\nex:bob ex:name \"Bob\" .\r\n"
    );
}

#[test]
fn removing_a_semicolon_pair_takes_exactly_one_separator_and_the_terminator_survives() {
    let (mut document, model) = load(CORPUS);
    let triple = literal_of(&model, "http://example.org/alice", Some("Alice"));

    assert_eq!(writer::remove_triple(&mut document, &model, &triple), "");
    assert_eq!(
        document.text(),
        "@prefix ex: <http://example.org/> .\r\n\r\nex:alice ex:knows ex:bob, ex:carol .\r\n\r\nex:bob ex:name \"Bob\" .\r\n"
    );
}

#[test]
fn removing_a_statements_only_triple_takes_the_whole_block_and_its_dot() {
    let (mut document, model) = load(CORPUS);
    let triple = literal_of(&model, "http://example.org/bob", Some("Bob"));

    assert_eq!(writer::remove_triple(&mut document, &model, &triple), "");
    assert_eq!(
        document.text(),
        "@prefix ex: <http://example.org/> .\r\n\r\nex:alice ex:knows ex:bob, ex:carol ;\r\n    ex:name \"Alice\" .\r\n\r\n"
    );
}

#[test]
fn removing_a_resource_takes_every_triple_it_touches_bottom_up() {
    let (mut document, model) = load(CORPUS);

    assert_eq!(
        writer::remove_resource(&mut document, &model, "http://example.org/bob"),
        ""
    );
    assert_eq!(
        document.text(),
        "@prefix ex: <http://example.org/> .\r\n\r\nex:alice ex:knows ex:carol ;\r\n    ex:name \"Alice\" .\r\n\r\n"
    );
}

#[test]
fn renaming_a_term_rewrites_every_occurrence_stranding_no_reference() {
    let (mut document, model) = load(CORPUS);

    assert_eq!(
        writer::rename_term(
            &mut document,
            &model,
            "http://example.org/bob",
            "http://example.org/robert"
        ),
        ""
    );
    assert_eq!(
        document.text(),
        "@prefix ex: <http://example.org/> .\r\n\r\nex:alice ex:knows ex:robert, ex:carol ;\r\n    ex:name \"Alice\" .\r\n\r\nex:robert ex:name \"Bob\" .\r\n"
    );
}

#[test]
fn renaming_onto_an_existing_term_is_refused_before_any_splice() {
    let (mut document, model) = load(CORPUS);

    let refusal = writer::rename_term(
        &mut document,
        &model,
        "http://example.org/bob",
        "http://example.org/alice",
    );
    assert!(refusal.contains("already names"), "{refusal}");
    assert_eq!(document.text(), CORPUS);
}

#[test]
fn replacing_a_literal_object_touches_no_neighbouring_byte() {
    let (mut document, model) = load(CORPUS);
    let triple = literal_of(&model, "http://example.org/alice", None);

    assert_eq!(
        writer::replace_object_literal(&mut document, &model, &triple, "Alicia", Some("en"), None),
        ""
    );
    assert_eq!(
        document.text(),
        "@prefix ex: <http://example.org/> .\r\n\r\nex:alice ex:knows ex:bob, ex:carol ;\r\n    ex:name \"Alicia\"@en .\r\n\r\nex:bob ex:name \"Bob\" .\r\n"
    );
}

/// Not in the standalone host's tests: a language tag's token spans its `@`, so the literal it ends is replaced to its
/// last byte (found by the `w3c/skos/geographic-names` parity script, where `@en` became `@enn`).
#[test]
fn replacing_a_language_tagged_literal_replaces_the_whole_tag() {
    let text = "@prefix ex: <http://example.org/> .\r\n\r\nex:alice ex:name \"Alice\"@de, \"Alice\"@en .\r\n";
    let (mut document, model) = load(text);
    let triple = model
        .triples
        .iter()
        .find(|t| {
            t.object
                .as_literal()
                .is_some_and(|l| l.language.as_deref() == Some("en"))
        })
        .unwrap()
        .clone();
    assert_eq!(
        writer::replace_object_literal(&mut document, &model, &triple, "Alicia", Some("en"), None),
        ""
    );
    assert_eq!(
        document.text(),
        "@prefix ex: <http://example.org/> .\r\n\r\nex:alice ex:name \"Alice\"@de, \"Alicia\"@en .\r\n"
    );
}

#[test]
fn replacing_a_non_literal_object_is_refused() {
    let (mut document, model) = load(CORPUS);
    let triple = find(&model, "http://example.org/knows", "http://example.org/bob");

    let refusal = writer::replace_object_literal(&mut document, &model, &triple, "nope", None, None);
    assert!(refusal.contains("not a literal"), "{refusal}");
    assert_eq!(document.text(), CORPUS);
}

#[test]
fn adding_a_triple_to_an_existing_subject_continues_its_block_with_its_indentation() {
    let (mut document, model) = load(CORPUS);

    let refusal = writer::add_triple(
        &mut document,
        &model,
        "http://example.org/bob",
        "http://example.org/knows",
        &RdfTerm::Iri(IriTerm::new("http://example.org/alice", "ex:alice")),
    );
    assert_eq!(refusal, "");
    assert_eq!(
        document.text(),
        "@prefix ex: <http://example.org/> .\r\n\r\nex:alice ex:knows ex:bob, ex:carol ;\r\n    ex:name \"Alice\" .\r\n\r\nex:bob ex:name \"Bob\" ;\r\n    ex:knows ex:alice .\r\n"
    );
}

#[test]
fn adding_a_triple_for_a_new_subject_appends_a_statement_with_declared_prefixes_never_invented() {
    let (mut document, model) = load(CORPUS);

    let mut dave = LiteralTerm::new("Dave", None, None);
    dave.as_written = "\"Dave\"".to_owned();
    let refusal = writer::add_triple(
        &mut document,
        &model,
        "http://example.org/dave",
        "http://xmlns.com/foaf/0.1/name",
        &RdfTerm::Literal(dave),
    );
    assert_eq!(refusal, "");
    assert_eq!(
        document.text(),
        format!("{CORPUS}\r\nex:dave <http://xmlns.com/foaf/0.1/name> \"Dave\" .\r\n")
    );
}

#[test]
fn adding_a_prefix_lands_beside_the_declaration_run_and_redeclaration_is_refused() {
    let (mut document, model) = load(CORPUS);

    let added = writer::add_prefix(&mut document, &model, "foaf", "http://xmlns.com/foaf/0.1/");
    let redeclared = writer::add_prefix(&mut document, &model, "ex", "http://elsewhere.example/");

    assert_eq!(added, "");
    assert!(redeclared.contains("already declared"), "{redeclared}");
    assert!(
        document
            .text()
            .starts_with("@prefix ex: <http://example.org/> .\r\n@prefix foaf: <http://xmlns.com/foaf/0.1/> .\r\n")
    );
}

#[test]
fn blank_node_rooted_edits_are_refused_with_the_identity_boundary_sentence() {
    let text = "@prefix ex: <http://example.org/> .\r\n_:someone ex:name \"Nobody\" .\r\n";
    let (mut document, model) = load(text);
    assert_eq!(model.triples.len(), 1);
    let triple = model.triples[0].clone();

    let remove = writer::remove_triple(&mut document, &model, &triple);
    let add = writer::add_triple(
        &mut document,
        &model,
        "http://example.org/alice",
        "http://example.org/knows",
        &RdfTerm::Blank(BlankTerm {
            label: None,
            ordinal: 99,
        }),
    );

    assert!(remove.contains("blank node"), "{remove}");
    assert!(add.contains("blank node"), "{add}");
    assert_eq!(document.text(), text);
}

#[test]
fn n_triples_flow_through_the_same_operations_degenerately() {
    let (mut document, model) = load(
        "<http://example.org/a> <http://example.org/p> <http://example.org/b> .\n<http://example.org/a> <http://example.org/q> \"x\" .\n<http://example.org/c> <http://example.org/p> <http://example.org/a> .\n",
    );
    let triple = model
        .triples
        .iter()
        .find(|t| t.object.as_literal().is_some())
        .unwrap()
        .clone();

    assert_eq!(writer::remove_triple(&mut document, &model, &triple), "");
    assert_eq!(
        document.text(),
        "<http://example.org/a> <http://example.org/p> <http://example.org/b> .\n<http://example.org/c> <http://example.org/p> <http://example.org/a> .\n"
    );
}

#[test]
fn every_refusal_leaves_the_document_byte_identical() {
    let (mut document, model) = load(CORPUS);

    let missing = writer::remove_resource(&mut document, &model, "http://example.org/nobody");
    let same_name = writer::rename_term(
        &mut document,
        &model,
        "http://example.org/bob",
        "http://example.org/bob",
    );

    assert!(missing.contains("nothing to remove"), "{missing}");
    assert!(same_name.contains("nothing to rename"), "{same_name}");
    assert_eq!(document.text(), CORPUS);
}
