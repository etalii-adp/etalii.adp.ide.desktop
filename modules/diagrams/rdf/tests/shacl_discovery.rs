//! The declaration-or-use discovery, one fixture per clause of the recommendation's shape definition, plus the two
//! exclusions that keep the reading honest: literals never, and non-shape subjects never. Ported from the standalone
//! host's `ShaclShapeDiscovery.Tests.cs` at `8b6ccfc`.

mod common;

use adp_diagram_rdf::shacl::discovery::{self, ShaclShape};

const PRELUDE: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n@prefix ex: <http://example.org/> .\n@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n";

fn discover(body: &str) -> Vec<ShaclShape> {
    discovery::discover(&common::model(&format!("{PRELUDE}{body}")))
}

fn single<'a>(shapes: &'a [ShaclShape], key: &str) -> &'a ShaclShape {
    let found: Vec<&ShaclShape> = shapes.iter().filter(|s| s.key == key).collect();
    assert_eq!(found.len(), 1, "{key}");
    found[0]
}

fn keys(shapes: &[ShaclShape]) -> Vec<&str> {
    shapes.iter().map(|s| s.key.as_str()).collect()
}

#[test]
fn a_typed_node_shape_is_discovered() {
    let shapes = discover("ex:PersonShape a sh:NodeShape .\n");
    assert_eq!(keys(&shapes), ["i:http://example.org/PersonShape"]);
    assert!(!shapes[0].is_property_shape);
}

#[test]
fn a_typed_property_shape_is_discovered_and_its_path_makes_it_a_property_shape() {
    let shapes = discover("ex:nameShape a sh:PropertyShape ; sh:path ex:name .\n");
    assert_eq!(keys(&shapes), ["i:http://example.org/nameShape"]);
    assert!(shapes[0].is_property_shape);
}

#[test]
fn a_subject_of_each_target_predicate_is_discovered() {
    let shapes = discover(
        "ex:A sh:targetClass ex:Person .\nex:B sh:targetNode ex:alice .\nex:C sh:targetSubjectsOf ex:knows .\nex:D sh:targetObjectsOf ex:knows .\n",
    );
    assert_eq!(
        keys(&shapes),
        [
            "i:http://example.org/A",
            "i:http://example.org/B",
            "i:http://example.org/C",
            "i:http://example.org/D"
        ]
    );
}

#[test]
fn a_subject_of_a_constraint_parameter_alone_is_discovered() {
    assert_eq!(keys(&discover("ex:S sh:minCount 1 .\n")), ["i:http://example.org/S"]);
}

#[test]
fn objects_of_shape_expecting_parameters_are_discovered_blank_ones_included() {
    let shapes = discover(
        "ex:S a sh:NodeShape ;\n  sh:node ex:Other ;\n  sh:not [ sh:datatype ex:T ] ;\n  sh:property [ sh:path ex:p ] .\n",
    );
    assert!(shapes.iter().any(|s| s.key == "i:http://example.org/Other"));
    let blanks: Vec<&ShaclShape> = shapes.iter().filter(|s| s.key.starts_with("b:")).collect();
    assert_eq!(blanks.len(), 2);
    assert!(blanks.iter().any(|s| s.is_property_shape));
    assert!(blanks.iter().any(|s| !s.is_property_shape));
}

#[test]
fn members_of_combinator_lists_are_discovered() {
    let shapes = discover("ex:S sh:or ( ex:A ex:B [ sh:datatype ex:T ] ) .\n");
    for key in [
        "i:http://example.org/S",
        "i:http://example.org/A",
        "i:http://example.org/B",
    ] {
        assert!(shapes.iter().any(|s| s.key == key), "{key}");
    }
    assert!(shapes.iter().any(|s| s.key.starts_with("b:")));
    // The list's own cons nodes are plumbing, not shapes.
    assert_eq!(shapes.len(), 4);
}

#[test]
fn an_implicit_class_target_is_flagged_directly_and_through_stated_subclassing() {
    let shapes = discover(
        "ex:Person a rdfs:Class, sh:NodeShape .\nex:Special rdfs:subClassOf rdfs:Class .\nex:Employee a ex:Special ;\n  sh:property [ sh:path ex:name ] .\nex:Plain a sh:NodeShape .\n",
    );
    assert!(single(&shapes, "i:http://example.org/Person").implicit_class_target);
    assert!(single(&shapes, "i:http://example.org/Employee").implicit_class_target);
    assert!(!single(&shapes, "i:http://example.org/Plain").implicit_class_target);
}

#[test]
fn non_shape_subjects_are_never_shapes() {
    let shapes = discover(
        "ex:Person a rdfs:Class .\nex:alice a ex:Person ; ex:name \"Alice\" .\nex:S sh:targetClass ex:Person .\n",
    );
    assert_eq!(keys(&shapes), ["i:http://example.org/S"]);
}

#[test]
fn ordering_is_document_order_of_the_first_defining_triple() {
    let shapes = discover(
        "ex:S a sh:NodeShape ; sh:node ex:B .\nex:A sh:targetClass ex:Person .\nex:B sh:property [ sh:path ex:p ] .\n",
    );
    let iris: Vec<&str> = keys(&shapes).into_iter().filter(|k| k.starts_with("i:")).collect();
    assert_eq!(
        iris,
        [
            "i:http://example.org/S",
            "i:http://example.org/B",
            "i:http://example.org/A"
        ]
    );
    assert!(shapes[0].first_triple_index < shapes[1].first_triple_index);
    assert!(
        single(&shapes, "i:http://example.org/B").first_triple_index
            < single(&shapes, "i:http://example.org/A").first_triple_index
    );
}

#[test]
fn two_spellings_of_one_iri_are_one_shape() {
    let shapes = discover("ex:S a sh:NodeShape .\n<http://example.org/S> sh:targetClass ex:Person .\n");
    assert_eq!(keys(&shapes), ["i:http://example.org/S"]);
}

#[test]
fn a_cyclic_combinator_list_yields_what_it_has_and_stops() {
    let shapes = discover(
        "ex:S sh:or _:l1 .\n_:l1 <http://www.w3.org/1999/02/22-rdf-syntax-ns#first> ex:A .\n_:l1 <http://www.w3.org/1999/02/22-rdf-syntax-ns#rest> _:l1 .\n",
    );
    assert!(shapes.iter().any(|s| s.key == "i:http://example.org/S"));
    assert!(shapes.iter().any(|s| s.key == "i:http://example.org/A"));
}
