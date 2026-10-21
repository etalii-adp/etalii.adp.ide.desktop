//! The shapes projection: cards for node shapes, rows for property shapes, edges only between shapes, the `sh:class`
//! single-claimant rule, the budget counting cards alone; the computed grid layout; and targets as chips, never
//! elements. Ported from the standalone host's `ShaclProjection.Tests.cs`, `ShaclLayout.Tests.cs` and
//! `ShaclTargetChips.Tests.cs` at `8b6ccfc`.

mod common;

use std::collections::HashSet;

use adp_diagram_rdf::shacl::discovery;
use adp_diagram_rdf::shacl::positions;
use adp_diagram_rdf::shacl::projection::{
    self, DEFAULT_BUDGET, ShaclCard, ShaclProjectionResult, ShaclTargetChip, ShaclTargetKind, id_of,
};

const PRELUDE: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n@prefix ex: <http://example.org/> .\n@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n";

fn project_with(body: &str, budget: usize) -> ShaclProjectionResult {
    projection::project(&common::model(&format!("{PRELUDE}{body}")), budget)
}

fn project(body: &str) -> ShaclProjectionResult {
    project_with(body, DEFAULT_BUDGET)
}

fn card<'a>(projection: &'a ShaclProjectionResult, id: &str) -> &'a ShaclCard {
    projection.cards.iter().find(|c| c.id == id).unwrap()
}

#[test]
fn cards_are_shapes_rows_are_property_shapes_and_plumbing_never_draws() {
    let projection = project(
        "ex:PersonShape a sh:NodeShape ;\n  sh:targetClass ex:Person ;\n  sh:name \"Person\" ;\n  sh:description \"A person profile.\" ;\n  sh:property [ sh:path ex:name ; sh:datatype xsd:string ; sh:minCount 1 ; sh:maxCount 1 ; sh:name \"name\" ] ;\n  sh:property [ sh:path ex:age ; sh:datatype xsd:integer ] .\nex:alice a ex:Person .\n",
    );
    assert_eq!(projection.cards.len(), 1);
    let card = &projection.cards[0];
    assert_eq!(card.id, "res:http://example.org/PersonShape");
    assert_eq!(card.display, "ex:PersonShape");
    assert_eq!(card.name, "Person");
    assert_eq!(card.description, "A person profile.");
    assert_eq!(card.rows.len(), 2);
    let row = &card.rows[0];
    assert_eq!(
        (
            row.path.as_str(),
            row.name.as_str(),
            row.summary.as_str(),
            row.cardinality.as_str()
        ),
        ("ex:name", "name", "xsd:string", "[1..1]")
    );
    let row = &card.rows[1];
    assert_eq!(
        (row.path.as_str(), row.summary.as_str(), row.cardinality.as_str()),
        ("ex:age", "xsd:integer", "[0..*]")
    );
    assert!(card.rows.iter().all(|r| r.blank));
    assert!(projection.edges.is_empty());
}

#[test]
fn flags_are_worn_deactivated_severity_closed() {
    let projection =
        project("ex:S a sh:NodeShape ;\n  sh:deactivated true ;\n  sh:severity sh:Warning ;\n  sh:closed true .\n");
    assert_eq!(projection.cards.len(), 1);
    let card = &projection.cards[0];
    assert!(card.deactivated);
    assert!(card.closed);
    assert_eq!(card.severity, "sh:Warning");
}

#[test]
fn sh_node_draws_an_edge_from_card_and_from_row_alike() {
    let projection = project(
        "ex:A a sh:NodeShape ; sh:node ex:B .\nex:B a sh:NodeShape .\nex:C a sh:NodeShape ;\n  sh:property [ sh:path ex:p ; sh:node ex:B ] .\n",
    );
    assert_eq!(projection.cards.len(), 3);
    assert!(projection.edges.iter().any(|e| e.from_id == "res:http://example.org/A"
        && e.to_id == "res:http://example.org/B"
        && e.kind == "node"
        && e.label == "node"));
    assert!(projection.edges.iter().any(|e| e.from_id == "res:http://example.org/C"
        && e.to_id == "res:http://example.org/B"
        && e.kind == "node"
        && e.label == "ex:p"));
}

#[test]
fn an_anonymous_node_shape_referenced_by_sh_node_draws_as_a_blank_card() {
    let projection = project("ex:A a sh:NodeShape ; sh:node [ sh:closed true ] .\n");
    assert_eq!(projection.cards.len(), 2);
    let blanks: Vec<&ShaclCard> = projection.cards.iter().filter(|c| c.blank).collect();
    assert_eq!(blanks.len(), 1);
    assert!(blanks[0].closed);
    assert!(
        projection
            .edges
            .iter()
            .any(|e| e.from_id == "res:http://example.org/A" && e.to_id == blanks[0].id)
    );
}

#[test]
fn combinator_operands_edges_for_iri_shapes_inline_row_when_any_operand_is_blank() {
    let projection =
        project("ex:A a sh:NodeShape .\nex:S sh:or ( ex:A [ sh:datatype xsd:date ] ) .\nex:T sh:xone ( ex:A ) .\n");
    assert!(!projection.cards.iter().any(|c| c.blank));
    assert!(
        projection.edges.iter().any(|e| e.from_id == "res:http://example.org/S"
            && e.to_id == "res:http://example.org/A"
            && e.kind == "or")
    );
    assert!(
        projection.edges.iter().any(|e| e.from_id == "res:http://example.org/T"
            && e.to_id == "res:http://example.org/A"
            && e.kind == "xone")
    );
    let s = card(&projection, "res:http://example.org/S");
    assert_eq!(s.rows.len(), 1);
    assert_eq!(s.rows[0].summary, "or(ex:A, { datatype xsd:date })");
    assert!(card(&projection, "res:http://example.org/T").rows.is_empty());
}

#[test]
fn sh_class_edges_only_for_exactly_one_drawn_claimant() {
    let projection = project(
        "ex:S a sh:NodeShape ;\n  sh:property [ sh:path ex:knows ; sh:class ex:Person ] ;\n  sh:property [ sh:path ex:owns ; sh:class ex:Thing ] ;\n  sh:property [ sh:path ex:likes ; sh:class ex:Food ] .\nex:PersonShape sh:targetClass ex:Person .\nex:FoodShapeA sh:targetClass ex:Food .\nex:FoodShapeB sh:targetClass ex:Food .\n",
    );
    let s = card(&projection, "res:http://example.org/S");
    let class_edges: Vec<_> = projection.edges.iter().filter(|e| e.kind == "class").collect();
    assert_eq!(class_edges.len(), 1);
    assert_eq!(
        (
            class_edges[0].from_id.as_str(),
            class_edges[0].to_id.as_str(),
            class_edges[0].label.as_str()
        ),
        (
            "res:http://example.org/S",
            "res:http://example.org/PersonShape",
            "ex:knows"
        )
    );
    let summary = |path: &str| s.rows.iter().find(|r| r.path == path).unwrap().summary.clone();
    assert_eq!(summary("ex:knows"), "");
    assert_eq!(summary("ex:owns"), "class ex:Thing");
    assert_eq!(summary("ex:likes"), "class ex:Food");
}

#[test]
fn an_orphan_iri_property_shape_draws_as_its_own_card_a_referenced_one_rows_instead() {
    let projection = project(
        "ex:nameShape a sh:PropertyShape ; sh:path ex:name .\nex:S a sh:NodeShape ; sh:property ex:refShape .\nex:refShape a sh:PropertyShape ; sh:path ex:ref .\n",
    );
    assert!(
        projection
            .cards
            .iter()
            .any(|c| c.id == "res:http://example.org/nameShape")
    );
    assert!(
        !projection
            .cards
            .iter()
            .any(|c| c.id == "res:http://example.org/refShape")
    );
    let s = card(&projection, "res:http://example.org/S");
    assert_eq!(s.rows.len(), 1);
    assert_eq!(s.rows[0].path, "ex:ref");
    assert!(!s.rows[0].blank);
}

#[test]
fn path_syntax_prints_the_expression_forms() {
    let projection = project(
        "ex:S a sh:NodeShape ;\n  sh:property [ sh:path [ sh:inversePath ex:parent ] ] ;\n  sh:property [ sh:path ( ex:parent ex:member ) ] ;\n  sh:property [ sh:path [ sh:alternativePath ( ex:a ex:b ) ] ] ;\n  sh:property [ sh:path [ sh:zeroOrMorePath ex:next ] ] ;\n  sh:property [ sh:path [ sh:oneOrMorePath ex:next ] ] ;\n  sh:property [ sh:path [ sh:zeroOrOnePath ex:next ] ] .\n",
    );
    let paths: Vec<&str> = projection.cards[0].rows.iter().map(|r| r.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "^ex:parent",
            "ex:parent/ex:member",
            "ex:a|ex:b",
            "ex:next*",
            "ex:next+",
            "ex:next?"
        ]
    );
}

#[test]
fn sparql_constraints_are_opaque_rows() {
    let projection =
        project("ex:S a sh:NodeShape ;\n  sh:sparql [ sh:select \"SELECT $this WHERE { $this ex:p ex:o }\" ] .\n");
    let rows = &projection.cards[0].rows;
    assert_eq!(rows.len(), 1);
    assert!(rows[0].sparql);
    assert_eq!(rows[0].summary, "SPARQL constraint");
}

#[test]
fn the_budget_counts_cards_only_and_cuts_deterministically() {
    let body: String = (0..12)
        .map(|i| format!("ex:S{i} a sh:NodeShape ; sh:property [ sh:path ex:p ] ; sh:targetClass ex:C{i} .\n"))
        .collect();
    let projection = project_with(&body, 5);
    assert_eq!(projection.shown, 5);
    assert_eq!(projection.total, 12);
    assert!(projection.truncated());
    let ids: Vec<&str> = projection.cards.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "res:http://example.org/S0",
            "res:http://example.org/S1",
            "res:http://example.org/S2",
            "res:http://example.org/S3",
            "res:http://example.org/S4"
        ]
    );
    assert!(
        projection
            .cards
            .iter()
            .all(|c| c.rows.len() == 1 && c.targets.len() == 1)
    );
}

#[test]
fn the_projection_is_deterministic() {
    let body = "ex:S a sh:NodeShape ; sh:or ( ex:A [ sh:datatype xsd:date ] ) ;\n  sh:property [ sh:path ex:p ; sh:class ex:Person ] .\nex:A a sh:NodeShape ; sh:targetClass ex:Person .\n";
    assert_eq!(project(body), project(body));
}

// The layout (`ShaclLayout.Tests.cs`).

#[test]
fn every_card_gets_a_position_and_the_same_file_always_opens_the_same_way() {
    let projection = project(
        "ex:A a sh:NodeShape ; sh:property [ sh:path ex:p ] .\nex:B a sh:NodeShape .\nex:C a sh:NodeShape ; sh:node [ sh:closed true ] .\nex:D a sh:NodeShape .\n",
    );
    let laid = positions(&projection);
    assert_eq!(laid.len(), projection.cards.len());
    assert!(projection.cards.iter().all(|c| laid.contains_key(&c.id)));
    assert_eq!(laid, positions(&projection));
    let first = laid[&projection.cards[0].id];
    let second = laid[&projection.cards[1].id];
    assert!(first.0 < second.0);
    assert_eq!(first.1, second.1);
}

#[test]
fn anonymous_cards_are_positioned_too_computed_only() {
    let projection = project("ex:C a sh:NodeShape ; sh:node [ sh:closed true ] .\n");
    let laid = positions(&projection);
    let blank = projection.cards.iter().find(|c| c.blank).unwrap();
    assert!(laid.contains_key(&blank.id));
}

// The target chips (`ShaclTargetChips.Tests.cs`).

const CHIP_PRELUDE: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n@prefix ex: <http://example.org/> .\n@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n";

fn chips_project(body: &str) -> ShaclProjectionResult {
    projection::project(&common::model(&format!("{CHIP_PRELUDE}{body}")), DEFAULT_BUDGET)
}

/// The whole element universe - card ids and both endpoints of every edge - is exactly the discovered shapes' ids.
fn assert_element_universe_is_shapes_only(body: &str) {
    let model = common::model(&format!("{CHIP_PRELUDE}{body}"));
    let shape_ids: HashSet<String> = discovery::discover(&model).iter().map(|s| id_of(&s.key)).collect();
    let projection = projection::project(&model, DEFAULT_BUDGET);
    assert!(projection.cards.iter().all(|c| shape_ids.contains(&c.id)));
    assert!(
        projection
            .edges
            .iter()
            .all(|e| shape_ids.contains(&e.from_id) && shape_ids.contains(&e.to_id))
    );
}

#[test]
fn an_absent_target_term_yields_a_chip_and_nothing_else() {
    for body in [
        "ex:S sh:targetClass ex:Person .\n",
        "ex:S sh:targetNode ex:alice .\n",
        "ex:S sh:targetSubjectsOf ex:knows .\n",
        "ex:S sh:targetObjectsOf ex:knows .\n",
    ] {
        let projection = chips_project(body);
        assert_eq!(projection.cards.len(), 1);
        assert!(projection.edges.is_empty());
        let targets = &projection.cards[0].targets;
        assert_eq!(targets.len(), 1);
        let chip = &targets[0];
        assert!(!chip.described_in_file);
        assert_ne!(chip.term_iri, "");
        assert_ne!(chip.term_display, "");
        assert_eq!(chip.shape_iri, "http://example.org/S");
        assert!(chip.predicate_iri.starts_with("http://www.w3.org/ns/shacl#target"));
        assert_element_universe_is_shapes_only(body);
    }
}

#[test]
fn a_present_target_term_still_yields_only_a_chip_the_flag_is_the_only_difference() {
    let body = "ex:S sh:targetClass ex:Person .\nex:Person a rdfs:Class .\nex:alice a ex:Person .\n";
    let projection = chips_project(body);
    assert_eq!(projection.cards.len(), 1);
    assert!(projection.edges.is_empty());
    assert_eq!(projection.cards[0].targets.len(), 1);
    assert!(projection.cards[0].targets[0].described_in_file);
    assert_element_universe_is_shapes_only(body);
}

#[test]
fn a_target_naming_another_drawn_shape_still_draws_no_edge() {
    let body = "ex:A sh:targetClass ex:B .\nex:B a sh:NodeShape .\n";
    let projection = chips_project(body);
    assert_eq!(projection.cards.len(), 2);
    assert!(projection.edges.is_empty());
    let chips = &card(&projection, "res:http://example.org/A").targets;
    assert_eq!(chips.len(), 1);
    assert!(chips[0].described_in_file);
    assert_eq!(chips[0].term_iri, "http://example.org/B");
    assert_element_universe_is_shapes_only(body);
}

#[test]
fn a_mixed_target_matrix_every_chip_complete_and_addressable() {
    let body = "ex:S a sh:NodeShape ;\n  sh:targetClass ex:Present, ex:Absent ;\n  sh:targetNode \"a literal node\" ;\n  sh:targetSubjectsOf ex:knows .\nex:Present a rdfs:Class .\n";
    let projection = chips_project(body);
    assert_eq!(projection.cards.len(), 1);
    assert!(projection.edges.is_empty());
    let chips = &projection.cards[0].targets;
    assert_eq!(chips.len(), 4);
    let has = |f: &dyn Fn(&ShaclTargetChip) -> bool| chips.iter().any(f);
    assert!(has(&|c| c.kind == ShaclTargetKind::Class
        && c.term_iri == "http://example.org/Present"
        && c.described_in_file));
    assert!(has(&|c| c.kind == ShaclTargetKind::Class
        && c.term_iri == "http://example.org/Absent"
        && !c.described_in_file));
    assert!(has(&|c| c.kind == ShaclTargetKind::Node
        && c.term_display == "a literal node"
        && c.term_iri.is_empty()
        && !c.described_in_file));
    assert!(has(
        &|c| c.kind == ShaclTargetKind::SubjectsOf && c.term_iri == "http://example.org/knows"
    ));
    assert!(
        chips
            .iter()
            .all(|c| c.shape_iri == "http://example.org/S" && !c.predicate_iri.is_empty())
    );
    assert_element_universe_is_shapes_only(body);
}

#[test]
fn the_implicit_class_target_is_a_chip_with_no_address_nothing_to_remove() {
    let projection = chips_project("ex:Person a rdfs:Class, sh:NodeShape .\n");
    assert_eq!(projection.cards.len(), 1);
    let chips = &projection.cards[0].targets;
    assert_eq!(chips.len(), 1);
    assert_eq!(chips[0].kind, ShaclTargetKind::ImplicitClass);
    assert!(chips[0].described_in_file);
    assert_eq!(chips[0].term_iri, "http://example.org/Person");
    assert_eq!(chips[0].predicate_iri, "");
}

#[test]
fn a_chip_on_an_anonymous_shape_has_no_shape_address_the_boundary_owns_it() {
    let projection = chips_project("[] sh:targetClass ex:Person ; sh:closed true .\n");
    assert_eq!(projection.cards.len(), 1);
    assert!(projection.cards[0].blank);
    let chips = &projection.cards[0].targets;
    assert_eq!(chips.len(), 1);
    assert_eq!(chips[0].shape_iri, "");
    assert_eq!(chips[0].predicate_iri, "http://www.w3.org/ns/shacl#targetClass");
}

// The mapper (`ShaclElementMapper`): a card's chips and rows travel in its payload, every field in field order.

#[test]
fn the_mapper_carries_chips_and_rows_in_the_cards_payload_and_edges_between_cards() {
    use adp_diagram::ElementKind;
    use adp_diagram_rdf::payload;
    use adp_diagram_rdf::shacl::{EDGE_TYPE, SHAPE_TYPE, mapper};

    let projection = project(
        "ex:A a sh:NodeShape ; sh:targetClass ex:Person ; sh:node ex:B ;\n  sh:property [ sh:path ex:name ; sh:minCount 1 ] .\nex:B a sh:NodeShape .\n",
    );
    let elements = mapper::elements(&projection, &positions(&projection));
    assert_eq!(elements.len(), 3);
    let a = &elements[0];
    assert_eq!(
        (a.id.as_str(), a.element_type.as_str()),
        ("res:http://example.org/A", SHAPE_TYPE)
    );
    assert_eq!(payload::text(a, "iri"), "http://example.org/A");
    assert_eq!(payload::text(a, "display"), "ex:A");
    assert!(!payload::flag(a, "blank"));
    let targets = payload::records(a, "targets");
    assert_eq!(targets.len(), 1);
    assert_eq!(payload::field(&targets[0], "kind"), "SHACL_TARGET_KIND_PROTO_CLASS");
    assert_eq!(payload::field(&targets[0], "termDisplay"), "ex:Person");
    assert_eq!(payload::field(&targets[0], "describedInFile"), "false");
    let rows = payload::records(a, "rows");
    assert_eq!(rows.len(), 1);
    assert_eq!(payload::field(&rows[0], "path"), "ex:name");
    assert_eq!(payload::field(&rows[0], "cardinality"), "[1..*]");
    let edge = &elements[2];
    assert_eq!(edge.element_type, EDGE_TYPE);
    assert_eq!(edge.kind, ElementKind::Relation);
    assert_eq!(payload::text(edge, "kind"), "node");
    assert_eq!(edge.source.as_deref(), Some("res:http://example.org/A"));
    assert_eq!(edge.target.as_deref(), Some("res:http://example.org/B"));
}
