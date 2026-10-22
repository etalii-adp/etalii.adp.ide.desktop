//! The data graph's projection and its computed layout: pure, deterministic, type-banded, untyped last. Ported from
//! the standalone host's `RdfProjection.Tests.cs` and `RdfLayout.Tests.cs` at `8b6ccfc`.

mod common;

use adp_diagram_rdf::projection::{self, RdfProjection};
use adp_diagram_rdf::vocabulary::rdf;

fn project(text: &str) -> RdfProjection {
    projection::project(&common::model(text), usize::MAX)
}

#[test]
fn the_construct_corpus_projects_the_adopted_positions() {
    let projection = project(&common::fixture("constructs.ttl"));

    let node = |id: &str| {
        let found: Vec<_> = projection.nodes.iter().filter(|n| n.id == id).collect();
        assert_eq!(found.len(), 1, "{id}");
        found[0]
    };
    let alice = node("res:http://example.org/alice");
    assert_eq!(alice.display, "ex:alice");
    assert_eq!(alice.types, ["foaf:Person"]);
    assert!(!projection.edges.iter().any(|e| e.predicate_iri == rdf::TYPE));

    assert!(
        alice
            .rows
            .iter()
            .any(|r| r.predicate == "foaf:name" && r.value == "Alice" && r.annotation.is_empty())
    );
    assert!(
        alice
            .rows
            .iter()
            .any(|r| r.value.starts_with("A description") && r.annotation == "@en")
    );
    assert!(
        projection
            .edges
            .iter()
            .any(|e| e.from_id == "res:http://example.org/alice"
                && e.to_id == "res:http://example.org/bob"
                && e.predicate == "foaf:knows")
    );

    let carol = "res:http://example.org/carol";
    assert!(
        projection
            .edges
            .iter()
            .any(|e| e.from_id == carol && e.predicate == "foaf:interests [1]")
    );
    assert!(
        projection
            .edges
            .iter()
            .any(|e| e.from_id == carol && e.predicate == "foaf:interests [2]")
    );
    assert!(
        node(carol)
            .rows
            .iter()
            .any(|r| r.predicate == "foaf:interests [3]" && r.value == "4.5")
    );

    assert!(projection.nodes.iter().any(|n| n.blank && n.display == "_:c"));
    for n in projection.nodes.iter().filter(|n| n.blank) {
        assert!(n.id.starts_with("blank:"), "{}", n.id);
    }

    let dave = projection
        .nodes
        .iter()
        .find(|n| n.rows.iter().any(|r| r.value == "Dave"))
        .unwrap();
    assert!(dave.blank);

    assert!(!projection.truncated());
    assert_eq!(projection.nodes.len(), projection.shown);
}

#[test]
fn the_budget_cuts_in_document_order_deterministically() {
    let mut text = String::from("@prefix ex: <http://example.org/> .\r\n");
    for i in 0..30 {
        text.push_str(&format!("ex:s{i:03} ex:knows ex:s{:03} .\r\n", i + 1));
    }
    let model = common::model(&text);

    let first = projection::project(&model, 10);
    let again = projection::project(&model, 10);

    assert!(first.truncated());
    assert_eq!(first.shown, 10);
    assert_eq!(first.total, 31);
    assert_eq!(first.nodes[0].id, "res:http://example.org/s000");
    assert_eq!(first.nodes[9].id, "res:http://example.org/s009");
    assert!(first.edges.iter().all(|e| e.to_id != "res:http://example.org/s010"));
    let ids = |p: &RdfProjection| p.nodes.iter().map(|n| n.id.clone()).collect::<Vec<_>>();
    assert_eq!(ids(&first), ids(&again));
}

#[test]
fn a_display_name_falls_back_from_prefixed_name_to_local_name() {
    let projection =
        project("@prefix ex: <http://example.org/> .\r\nex:a ex:p <http://xmlns.com/foaf/0.1/Person> .\r\n");

    assert!(projection.nodes.iter().any(|n| n.display == "Person"));
}

const BANDED: &str = "@prefix ex: <http://example.org/> .\r\nex:zebra a ex:Animal .\r\nex:apple a ex:Fruit .\r\nex:bear a ex:Animal .\r\nex:loose ex:notes \"untyped\" .\r\n";

#[test]
fn bands_group_by_primary_type_untyped_last() {
    let positions = projection::positions(&project(BANDED));
    let y = |local: &str| positions[&format!("res:http://example.org/{local}")].1;

    assert_eq!(y("zebra"), y("bear"));
    assert!(y("zebra") < y("apple"));
    assert!(y("apple") < y("loose"));
}

#[test]
fn the_layout_is_deterministic() {
    let projection = project(BANDED);

    assert_eq!(projection::positions(&projection), projection::positions(&projection));
}

#[test]
fn every_drawn_node_gets_a_position() {
    let projection = project(BANDED);
    let positions = projection::positions(&projection);

    assert!(!projection.nodes.is_empty());
    for node in &projection.nodes {
        assert!(positions.contains_key(&node.id), "{}", node.id);
    }
}
