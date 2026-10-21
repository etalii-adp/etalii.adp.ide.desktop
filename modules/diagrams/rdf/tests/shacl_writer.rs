//! The shapes reading's writers - the one SHACL-owned splice that adds a property row, the gestures that go through
//! the family writer, the exclusive-reachability removal - and the double refusal of a blank-rooted edit by the
//! writer and by the gate independently, in one sentence. Ported from the standalone host's `ShaclWriter.Tests.cs`,
//! `ShaclRemoveShape.Tests.cs` and `ShaclDoubleRefusal.Tests.cs` at `8b6ccfc`.

mod common;

use adp_diagram_rdf::lines::LineDocument;
use adp_diagram_rdf::model::{RdfModel, RdfTerm};
use adp_diagram_rdf::parser;
use adp_diagram_rdf::selection::TRUNCATED_REFUSAL;
use adp_diagram_rdf::shacl::gate;
use adp_diagram_rdf::shacl::projection::{self, DEFAULT_BUDGET};
use adp_diagram_rdf::shacl::refusals::{BLANK_ROOTED, IMPLICIT_TARGET, NO_SUCH_SHAPE, NO_SUCH_TARGET};
use adp_diagram_rdf::shacl::writer::{self, ShaclPropertyShapeOptions};
use adp_diagram_rdf::vocabulary::shacl;
use adp_diagram_rdf::writer as family;

// The standalone host's tests wrote their documents as C# raw string literals in files checked out with CRLF, so a
// raw block's lines end in CRLF and only the suffix the test appended ends otherwise.
const PRELUDE: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\r\n@prefix ex: <http://example.org/> .\r\n@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\r\n";
const PRELUDE_SHORT: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\r\n@prefix ex: <http://example.org/> .\r\n";
const EX: &str = "http://example.org/";

/// A raw block followed by the `"\n"` the test appended: every line break but the last is the block's CRLF.
fn raw(text: &str) -> String {
    if text.ends_with("\r\n") {
        return text.to_owned();
    }
    let body = text.strip_suffix('\n').unwrap_or(text);
    format!("{}\n", body.replace('\n', "\r\n"))
}

fn open_with(prelude: &str, body: &str) -> (LineDocument, RdfModel) {
    let document = LineDocument::parse(&format!("{prelude}{}", raw(body)));
    let model = parser::parse(&document).unwrap();
    (document, model)
}

fn open(body: &str) -> (LineDocument, RdfModel) {
    open_with(PRELUDE, body)
}

fn ex(local: &str) -> String {
    format!("{EX}{local}")
}

fn none() -> ShaclPropertyShapeOptions {
    ShaclPropertyShapeOptions::default()
}

// ShaclWriter.Tests.cs

#[test]
fn append_property_shape_block_splices_one_continuation_matching_the_blocks_indentation() {
    let (mut document, model) = open("ex:PersonShape a sh:NodeShape ;\r\n  sh:targetClass ex:Person .\r\n");
    let refusal = writer::append_property_shape_block(
        &mut document,
        &model,
        &ex("PersonShape"),
        &ex("name"),
        &ShaclPropertyShapeOptions {
            datatype_iri: "http://www.w3.org/2001/XMLSchema#string".to_owned(),
            min_count: Some(1),
            max_count: Some(1),
            name: "name".to_owned(),
        },
    );
    assert_eq!(refusal, "");
    assert_eq!(
        document.text(),
        format!(
            "{PRELUDE}ex:PersonShape a sh:NodeShape ;\r\n  sh:targetClass ex:Person ;\r\n  sh:property [ sh:path ex:name ; sh:datatype xsd:string ; sh:minCount 1 ; sh:maxCount 1 ; sh:name \"name\" ] .\r\n"
        )
    );
}

#[test]
fn append_property_shape_block_on_a_single_line_statement_indents_one_step_in() {
    let (mut document, model) = open("ex:S a sh:NodeShape .\r\n");
    assert_eq!(
        writer::append_property_shape_block(&mut document, &model, &ex("S"), &ex("p"), &none()),
        ""
    );
    assert_eq!(
        document.text(),
        format!("{PRELUDE}ex:S a sh:NodeShape ;\r\n    sh:property [ sh:path ex:p ] .\r\n")
    );
}

#[test]
fn append_property_shape_block_leaves_neighbouring_statements_byte_identical() {
    let (mut document, model) = open(
        "ex:Other a sh:NodeShape ; sh:closed true .\r\nex:S a sh:NodeShape .\r\nex:Third a sh:NodeShape ; sh:targetNode ex:x .\r\n",
    );
    let before = document.text();
    writer::append_property_shape_block(&mut document, &model, &ex("S"), &ex("p"), &none());
    let after = document.text();
    let before_lines: Vec<&str> = before.split('\n').collect();
    let after_lines: Vec<&str> = after.split('\n').collect();
    assert_eq!(before_lines[3], after_lines[3]);
    assert_eq!(before_lines[5], after_lines[6]);
}

#[test]
fn append_property_shape_block_never_invents_a_prefix() {
    let (mut document, model) = open("ex:S a sh:NodeShape .\r\n");
    writer::append_property_shape_block(
        &mut document,
        &model,
        &ex("S"),
        "http://xmlns.com/foaf/0.1/name",
        &none(),
    );
    assert!(document.text().contains("sh:path <http://xmlns.com/foaf/0.1/name>"));
    assert!(!document.text().contains("@prefix foaf:"));
}

#[test]
fn append_property_shape_block_refuses_before_any_splice() {
    for (shape, path) in [("", "http://example.org/p"), ("http://example.org/S", "")] {
        let (mut document, model) = open("ex:S a sh:NodeShape .\r\n");
        let before = document.text();
        assert_ne!(
            writer::append_property_shape_block(&mut document, &model, shape, path, &none()),
            ""
        );
        assert_eq!(document.text(), before);
    }
}

#[test]
fn append_property_shape_block_refuses_an_unknown_shape() {
    let (mut document, model) = open("ex:S a sh:NodeShape .\r\n");
    let before = document.text();
    assert_eq!(
        writer::append_property_shape_block(&mut document, &model, &ex("Missing"), &ex("p"), &none()),
        NO_SUCH_SHAPE
    );
    assert_eq!(document.text(), before);
}

#[test]
fn add_target_and_remove_target_round_trip_through_the_family_writer() {
    let (mut document, model) = open("ex:S a sh:NodeShape .\r\n");
    let before = document.text();
    assert_eq!(
        writer::add_target(
            &mut document,
            &model,
            &ex("S"),
            shacl::TARGET_CLASS,
            &RdfTerm::iri(ex("Person"), "ex:Person")
        ),
        ""
    );
    assert!(document.text().contains("sh:targetClass ex:Person"));
    let reparsed = parser::parse(&document).unwrap();
    assert_eq!(
        writer::remove_target(&mut document, &reparsed, &ex("S"), shacl::TARGET_CLASS, &ex("Person")),
        ""
    );
    assert_eq!(document.text(), before);
}

#[test]
fn remove_target_refuses_what_is_not_there_and_the_implicit_target() {
    let (mut document, model) = open("ex:S a sh:NodeShape .\r\n");
    let before = document.text();
    assert_eq!(
        writer::remove_target(&mut document, &model, &ex("S"), shacl::TARGET_CLASS, &ex("Person")),
        NO_SUCH_TARGET
    );
    assert_eq!(
        writer::remove_target(&mut document, &model, &ex("S"), "", &ex("S")),
        IMPLICIT_TARGET
    );
    assert_eq!(document.text(), before);
}

#[test]
fn create_node_shape_writes_one_type_triple() {
    let (mut document, model) = open("ex:Existing a sh:NodeShape .\r\n");
    assert_eq!(writer::create_node_shape(&mut document, &model, &ex("Fresh")), "");
    assert!(document.text().contains("ex:Fresh a sh:NodeShape ."));
    assert!(document.text().contains("ex:Existing a sh:NodeShape ."));
}

#[test]
fn set_deactivated_adds_and_removes_the_triple_and_is_idempotent() {
    let (mut document, model) = open("ex:S a sh:NodeShape .\r\n");
    let before = document.text();
    assert_eq!(writer::set_deactivated(&mut document, &model, &ex("S"), true), "");
    assert!(
        parser::parse(&document).unwrap().triples.iter().any(
            |t| t.predicate.iri == shacl::DEACTIVATED && t.object.as_literal().is_some_and(|l| l.lexical == "true")
        )
    );
    let after_first = document.text();
    let reparsed = parser::parse(&document).unwrap();
    assert_eq!(writer::set_deactivated(&mut document, &reparsed, &ex("S"), true), "");
    assert_eq!(document.text(), after_first);
    let reparsed = parser::parse(&document).unwrap();
    assert_eq!(writer::set_deactivated(&mut document, &reparsed, &ex("S"), false), "");
    assert_eq!(document.text(), before);
}

#[test]
fn set_literal_rewrites_in_place_through_replace_object_literal_touching_no_neighbouring_byte() {
    let (mut document, model) =
        open("ex:S a sh:NodeShape ;\r\n  sh:name \"Old name\" ;\r\n  sh:description \"Kept exactly.\" .\r\n");
    assert_eq!(
        writer::set_literal(&mut document, &model, &ex("S"), shacl::NAME, "New name"),
        ""
    );
    assert_eq!(
        document.text(),
        format!(
            "{PRELUDE}ex:S a sh:NodeShape ;\r\n  sh:name \"New name\" ;\r\n  sh:description \"Kept exactly.\" .\r\n"
        )
    );
}

#[test]
fn set_literal_adds_the_triple_where_none_is_stated() {
    let (mut document, model) = open("ex:S a sh:NodeShape .\r\n");
    assert_eq!(
        writer::set_literal(&mut document, &model, &ex("S"), shacl::NAME, "Fresh"),
        ""
    );
    assert!(document.text().contains("sh:name \"Fresh\""));
}

#[test]
fn every_gesture_refuses_a_blank_rooted_shape_with_the_one_boundary_sentence() {
    let (mut document, model) = open("[] a sh:NodeShape .\r\n");
    let before = document.text();
    let c = RdfTerm::iri(ex("C"), "ex:C");
    assert_eq!(
        writer::append_property_shape_block(&mut document, &model, "", &ex("p"), &none()),
        BLANK_ROOTED
    );
    assert_eq!(
        writer::add_target(&mut document, &model, "", shacl::TARGET_CLASS, &c),
        BLANK_ROOTED
    );
    assert_eq!(
        writer::remove_target(&mut document, &model, "", shacl::TARGET_CLASS, &ex("C")),
        BLANK_ROOTED
    );
    assert_eq!(writer::set_deactivated(&mut document, &model, "", true), BLANK_ROOTED);
    assert_eq!(
        writer::set_literal(&mut document, &model, "", shacl::NAME, "x"),
        BLANK_ROOTED
    );
    assert_eq!(document.text(), before);
}

// ShaclRemoveShape.Tests.cs

fn removes(body: &str, survivor: &str) {
    let (mut document, model) = open(body);
    assert_eq!(writer::remove_shape_with_subtrees(&mut document, &model, &ex("S")), "");
    assert_eq!(document.text(), format!("{PRELUDE}{}", raw(survivor)));
}

#[test]
fn an_inline_subtree_goes_with_the_statement_that_holds_it() {
    removes(
        "ex:S a sh:NodeShape ;\n  sh:property [ sh:path ex:name ; sh:minCount 1 ] .\nex:Keep a sh:NodeShape ;\n  sh:property [ sh:path ex:other ] .\n",
        "ex:Keep a sh:NodeShape ;\n  sh:property [ sh:path ex:other ] .\n",
    );
}

#[test]
fn a_shape_with_several_property_blocks_sweeps_them_all_without_stale_offsets() {
    removes(
        "ex:S a sh:NodeShape ;\n  sh:property [ sh:path ex:one ; sh:minCount 1 ] ;\n  sh:property [ sh:path ex:two ; sh:datatype xsd:string ] ;\n  sh:property [ sh:path ex:three ] ;\n  sh:property [ sh:path ex:four ; sh:maxCount 2 ] .\nex:Keep a sh:NodeShape ;\n  sh:property [ sh:path ex:kept ; sh:minCount 1 ] .\n",
        "ex:Keep a sh:NodeShape ;\n  sh:property [ sh:path ex:kept ; sh:minCount 1 ] .\n",
    );
}

#[test]
fn several_separately_stated_subtrees_go_without_stale_offsets() {
    removes(
        "ex:S a sh:NodeShape ;\n  sh:property _:a ;\n  sh:property _:b ;\n  sh:property _:c .\n_:a sh:path ex:one ; sh:minCount 1 .\n_:b sh:path ex:two ; sh:datatype xsd:string .\n_:c sh:path ex:three .\nex:Keep a sh:NodeShape .\n",
        "ex:Keep a sh:NodeShape .\n",
    );
}

#[test]
fn an_exclusive_separately_stated_subtree_goes() {
    removes(
        "ex:S a sh:NodeShape ;\n  sh:property _:row .\n_:row sh:path ex:name ;\n  sh:minCount 1 .\nex:Keep a sh:NodeShape .\n",
        "ex:Keep a sh:NodeShape .\n",
    );
}

#[test]
fn a_subtree_a_surviving_shape_also_references_stays_byte_identical() {
    removes(
        "ex:S a sh:NodeShape ;\n  sh:property _:shared .\nex:Keep a sh:NodeShape ;\n  sh:property _:shared .\n_:shared sh:path ex:name ;\n  sh:minCount 1 .\n",
        "ex:Keep a sh:NodeShape ;\n  sh:property _:shared .\n_:shared sh:path ex:name ;\n  sh:minCount 1 .\n",
    );
}

#[test]
fn a_diamond_two_paths_from_the_removed_shape_and_no_other_referrer_goes_exactly_once() {
    removes(
        "ex:S a sh:NodeShape ;\n  sh:property _:twice ;\n  sh:node _:twice .\n_:twice sh:path ex:name .\nex:Keep a sh:NodeShape .\n",
        "ex:Keep a sh:NodeShape .\n",
    );
}

#[test]
fn a_blank_a_non_shape_subject_also_references_stays() {
    removes(
        "ex:S a sh:NodeShape ;\n  sh:property _:b .\n_:b sh:path ex:name .\nex:someData ex:mentions _:b .\n",
        "_:b sh:path ex:name .\nex:someData ex:mentions _:b .\n",
    );
}

#[test]
fn three_deep_nesting_is_removed_bottom_up() {
    removes(
        "ex:S a sh:NodeShape ;\n  sh:node _:one .\n_:one sh:node _:two .\n_:two sh:node _:three .\n_:three sh:path ex:deep .\nex:Keep a sh:NodeShape .\n",
        "ex:Keep a sh:NodeShape .\n",
    );
}

#[test]
fn a_deep_subtree_shared_further_down_keeps_everything_from_that_point_down() {
    removes(
        "ex:S a sh:NodeShape ;\n  sh:node _:one .\n_:one sh:node _:two .\n_:two sh:node _:three .\n_:three sh:path ex:deep .\nex:Keep a sh:NodeShape ;\n  sh:node _:two .\n",
        "_:two sh:node _:three .\n_:three sh:path ex:deep .\nex:Keep a sh:NodeShape ;\n  sh:node _:two .\n",
    );
}

#[test]
fn the_count_is_stated_before_anything_runs_and_matches_what_goes() {
    let (mut document, model) = open(
        "ex:S a sh:NodeShape ;\n  sh:property _:row .\n_:row sh:path ex:name ;\n  sh:minCount 1 .\nex:Keep a sh:NodeShape .\n",
    );
    let announced = writer::count_shape_removal(&model, &ex("S"));
    let before = model.triples.len();
    assert_eq!(writer::remove_shape_with_subtrees(&mut document, &model, &ex("S")), "");
    assert_eq!(announced, 4);
    assert_eq!(parser::parse(&document).unwrap().triples.len(), before - announced);
}

#[test]
fn the_count_excludes_a_shared_subtree() {
    let (_, model) = open(
        "ex:S a sh:NodeShape ;\n  sh:property _:shared .\nex:Keep a sh:NodeShape ;\n  sh:property _:shared .\n_:shared sh:path ex:name ;\n  sh:minCount 1 .\n",
    );
    assert_eq!(writer::count_shape_removal(&model, &ex("S")), 2);
}

#[test]
fn removing_refuses_blank_rooted_and_unknown_shapes_leaving_the_file_untouched() {
    let (mut document, model) = open("ex:S a sh:NodeShape .\n");
    let before = document.text();
    assert_eq!(
        writer::remove_shape_with_subtrees(&mut document, &model, ""),
        BLANK_ROOTED
    );
    assert_eq!(
        writer::remove_shape_with_subtrees(&mut document, &model, &ex("Missing")),
        NO_SUCH_SHAPE
    );
    assert_eq!(document.text(), before);
}

#[test]
fn an_inline_subtree_shared_by_label_stays_when_another_shape_names_it() {
    removes(
        "ex:Keep a sh:NodeShape ;\n  sh:property _:shared .\n_:shared sh:path ex:name .\nex:S a sh:NodeShape ;\n  sh:property _:shared .\n",
        "ex:Keep a sh:NodeShape ;\n  sh:property _:shared .\n_:shared sh:path ex:name .\n",
    );
}

// ShaclDoubleRefusal.Tests.cs

#[test]
fn the_writer_alone_refuses_every_blank_rooted_mutation_and_splices_nothing() {
    let (mut document, model) = open_with(PRELUDE_SHORT, "[] a sh:NodeShape ; sh:property [ sh:path ex:p ] .\n");
    let before = document.text();
    let c = RdfTerm::iri(ex("C"), "ex:C");
    assert_eq!(
        writer::append_property_shape_block(&mut document, &model, "", &ex("q"), &none()),
        BLANK_ROOTED
    );
    assert_eq!(
        writer::remove_shape_with_subtrees(&mut document, &model, ""),
        BLANK_ROOTED
    );
    assert_eq!(writer::set_deactivated(&mut document, &model, "", true), BLANK_ROOTED);
    assert_eq!(
        writer::set_literal(&mut document, &model, "", shacl::NAME, "x"),
        BLANK_ROOTED
    );
    assert_eq!(
        writer::add_target(&mut document, &model, "", shacl::TARGET_CLASS, &c),
        BLANK_ROOTED
    );
    assert_eq!(
        writer::remove_target(&mut document, &model, "", shacl::TARGET_CLASS, &ex("C")),
        BLANK_ROOTED
    );
    assert_eq!(document.text(), before);
}

#[test]
fn the_gate_alone_refuses_an_anonymous_shape_without_touching_the_writer_or_the_document() {
    let (document, model) = open_with(PRELUDE_SHORT, "[] a sh:NodeShape ; sh:targetClass ex:Person .\n");
    let before = document.text();
    let blank_id = projection::project(&model, DEFAULT_BUDGET)
        .cards
        .into_iter()
        .find(|c| c.blank)
        .unwrap()
        .id;
    let decision = gate(&model, &blank_id, false);
    assert!(decision.applies);
    assert!(!decision.available);
    assert_eq!(decision.reason, BLANK_ROOTED);
    assert_eq!(decision.shape_iri, "");
    assert_eq!(document.text(), before);
}

#[test]
fn the_gate_alone_allows_an_iri_named_shape() {
    let (_, model) = open_with(PRELUDE_SHORT, "ex:S a sh:NodeShape .\n");
    let decision = gate(&model, "res:http://example.org/S", false);
    assert!(decision.applies);
    assert!(decision.available);
    assert_eq!(decision.reason, "");
    assert_eq!(decision.shape_iri, "http://example.org/S");
}

#[test]
fn both_layers_say_the_identical_sentence() {
    let (mut document, model) = open_with(PRELUDE_SHORT, "[] a sh:NodeShape ; sh:property [ sh:path ex:p ] .\n");
    let blank_id = projection::project(&model, DEFAULT_BUDGET)
        .cards
        .into_iter()
        .find(|c| c.blank)
        .unwrap()
        .id;
    let from_the_gate = gate(&model, &blank_id, false).reason;
    let from_the_writer = writer::remove_shape_with_subtrees(&mut document, &model, "");
    assert_eq!(from_the_gate, from_the_writer);
    assert_eq!(from_the_gate, BLANK_ROOTED);
}

#[test]
fn the_family_writers_public_remove_triple_still_refuses_a_blank_involving_triple() {
    let (mut document, model) = open_with(PRELUDE_SHORT, "ex:S a sh:NodeShape ; sh:property [ sh:path ex:p ] .\n");
    let before = document.text();
    let blank_involving = model
        .triples
        .iter()
        .find(|t| t.subject.is_blank() || t.object.is_blank())
        .unwrap()
        .clone();
    let refusal = family::remove_triple(&mut document, &model, &blank_involving);
    assert_ne!(refusal, "");
    assert!(refusal.to_lowercase().contains("blank node"));
    assert_eq!(document.text(), before);
}

#[test]
fn the_gate_says_nothing_about_a_term_this_file_does_not_state_to_be_a_shape() {
    let (_, model) = open_with(
        PRELUDE_SHORT,
        "ex:S a sh:NodeShape ; sh:targetClass ex:Person .\nex:alice a ex:Person ; ex:name \"Alice\" .\n",
    );
    assert!(!gate(&model, "res:http://example.org/alice", false).applies);
    assert!(!gate(&model, "res:http://example.org/Person", false).applies);
    assert!(gate(&model, "res:http://example.org/S", false).applies);
}

#[test]
fn the_gate_withholds_under_truncation_with_the_family_sentence() {
    let (_, model) = open_with(PRELUDE_SHORT, "ex:S a sh:NodeShape .\n");
    let decision = gate(&model, "res:http://example.org/S", true);
    assert!(decision.applies);
    assert!(!decision.available);
    assert_eq!(decision.reason, TRUNCATED_REFUSAL);
}

#[test]
fn the_gate_ignores_ids_that_are_not_this_familys() {
    let (_, model) = open_with(PRELUDE_SHORT, "ex:S a sh:NodeShape .\n");
    assert!(!gate(&model, "", false).applies);
    assert!(!gate(&model, "shacl-edge:a|node|b", false).applies);
}
