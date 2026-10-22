//! The variable-as-node projection and its layout (etalii.adp spec 030, task T084): the standalone host's
//! `SparqlProjection.Tests.cs` and `SparqlLayout.Tests.cs` at `8b6ccfc`, with the same expectations. One node per name
//! through OPTIONAL, UNION and MINUS; the shallowest-scope placement rule with its union lift; subquery collapse;
//! concrete-term merging by form; determinism; the sanity cut; and containment as a tested invariant, for computed
//! layout and for an anchored region.

mod common;

use std::collections::{BTreeMap, HashSet};

use adp_diagram_sparql::layout::{self, NODE_HEIGHT, NODE_WIDTH, SparqlLayoutResult};
use adp_diagram_sparql::projection::{SANITY_BOUND, SparqlNodeKind, SparqlProjectionResult};
use common::{project, project_fixture};

fn node<'a>(result: &'a SparqlProjectionResult, id: &str) -> &'a adp_diagram_sparql::projection::SparqlNode {
    let found: Vec<_> = result.nodes.iter().filter(|n| n.id == id).collect();
    assert_eq!(found.len(), 1, "{id}");
    found[0]
}

#[test]
fn one_variable_one_node_however_many_patterns_and_regions_mention_it() {
    let result = project_fixture("groups.rq");
    let x = node(&result, "var:x");
    assert_eq!(x.kind, SparqlNodeKind::Variable);
    assert_eq!(x.display, "?x");
    assert_eq!(
        result
            .edges
            .iter()
            .filter(|e| e.from_id == "var:x" || e.to_id == "var:x")
            .count(),
        5
    );
    assert_eq!(x.join_count, 5);
    assert!(x.projected);
}

#[test]
fn the_placement_rule_shallowest_scope_with_region_edges_reaching_out() {
    let result = project_fixture("groups.rq");
    assert_eq!(node(&result, "var:x").scope_path, "where");
    assert_eq!(node(&result, "var:y").scope_path, "where/optional.1");
    let optional: Vec<_> = result
        .edges
        .iter()
        .filter(|e| e.scope_path == "where/optional.0")
        .collect();
    assert_eq!(optional.len(), 1);
    assert_eq!(optional[0].to_id, "var:x");
}

#[test]
fn a_union_shared_name_lifts_to_the_scope_that_joins_the_branches() {
    let result = project("PREFIX ex: <http://example.org/> SELECT ?u WHERE { { ?u ex:a 1 } UNION { ?u ex:b 2 } }");
    assert_eq!(node(&result, "var:u").scope_path, "where");
    assert_eq!(result.edges[0].scope_path, "where/union.0/branch.0");
    assert_eq!(result.edges[1].scope_path, "where/union.0/branch.1");
}

#[test]
fn a_minus_region_follows_textual_identity() {
    let result = project_fixture("groups.rq");
    assert_eq!(node(&result, "var:s").scope_path, "where");
    assert_eq!(
        result
            .edges
            .iter()
            .filter(|e| e.scope_path == "where/minus.0" && e.from_id == "var:s")
            .count(),
        1
    );
    let minus: Vec<_> = result.regions.iter().filter(|r| r.kind == "minus").collect();
    assert_eq!(minus.len(), 1);
    assert_eq!(minus[0].label, "MINUS");
}

#[test]
fn regions_nest_as_the_tree_nests_each_element_claimed_by_exactly_one_region() {
    let result = project(
        "PREFIX ex: <http://example.org/> SELECT * WHERE { OPTIONAL { ?a ex:p ?b . OPTIONAL { ?b ex:q ?c } } }",
    );
    let outer = result
        .regions
        .iter()
        .find(|r| r.scope_path == "where/optional.0")
        .unwrap();
    let inner = result
        .regions
        .iter()
        .find(|r| r.scope_path == "where/optional.0/optional.0")
        .unwrap();
    assert_eq!(outer.parent_region_id, "");
    assert_eq!(inner.parent_region_id, outer.id);
    assert_eq!(node(&result, "var:c").scope_path, inner.scope_path);
    assert_eq!(node(&result, "var:b").scope_path, outer.scope_path);
}

#[test]
fn a_subquery_collapses_to_one_node_with_a_join_edge_per_projected_name() {
    let result = project_fixture("groups.rq");
    let collapsed: Vec<_> = result
        .nodes
        .iter()
        .filter(|n| n.kind == SparqlNodeKind::SubSelect)
        .collect();
    assert_eq!(collapsed.len(), 1);
    assert_eq!(collapsed[0].display, "SELECT ?s (MAX(?v) AS ?best)");
    assert_eq!(collapsed[0].id, "sub:where.0");
    assert!(!result.nodes.iter().any(|n| n.id == "var:v"));
    for to in ["var:s", "var:best"] {
        assert_eq!(
            result
                .edges
                .iter()
                .filter(|e| e.from_id == collapsed[0].id && e.to_id == to)
                .count(),
            1
        );
    }
}

#[test]
fn concrete_terms_merge_by_form_which_asserts_same_term_and_never_a_join() {
    let result = project(
        "PREFIX ex: <http://example.org/> SELECT * WHERE { ?a ex:knows ex:carol . ?b ex:knows ex:carol . ?c ex:age 42 . ?d ex:age 42 }",
    );
    node(&result, "iri:http://example.org/carol");
    assert_eq!(
        result
            .edges
            .iter()
            .filter(|e| e.to_id == "iri:http://example.org/carol")
            .count(),
        2
    );
    let literals: Vec<_> = result
        .nodes
        .iter()
        .filter(|n| n.kind == SparqlNodeKind::Literal)
        .collect();
    assert_eq!(literals.len(), 1);
    assert_eq!(literals[0].display, "42");
    assert_eq!(literals[0].id, "lit:42^integer");
    assert_eq!(result.edges.iter().filter(|e| e.to_id == literals[0].id).count(), 2);
}

#[test]
fn a_property_path_edge_carries_its_label_as_written_and_says_it_is_a_path() {
    let result = project_fixture("constructs.rq");
    let paths: Vec<_> = result.edges.iter().filter(|e| e.is_path).collect();
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].label, "foaf:knows+");
}

#[test]
fn annotations_anchor_to_what_they_constrain_define_or_feed() {
    let result = project_fixture("groups.rq");
    let in_optional: Vec<_> = result
        .annotations
        .iter()
        .filter(|a| a.scope_path == "where/optional.0")
        .collect();
    assert_eq!(in_optional.len(), 1);
    assert_eq!(in_optional[0].attached_to_id, "region:where/optional.0");
    let root_filter = result
        .annotations
        .iter()
        .find(|a| a.kind == "filter" && a.scope_path == "where")
        .unwrap();
    assert_eq!(root_filter.attached_to_id, "");
    assert_eq!(
        result
            .annotations
            .iter()
            .find(|a| a.kind == "bind")
            .unwrap()
            .attached_to_id,
        "var:doubled"
    );
    assert_eq!(
        result
            .annotations
            .iter()
            .find(|a| a.kind == "values")
            .unwrap()
            .attached_to_id,
        "var:x"
    );
}

#[test]
fn the_header_carries_the_form_and_the_frame_rows() {
    let result = project_fixture("constructs.rq");
    assert_eq!(result.header_form, "SELECT DISTINCT");
    assert_eq!(
        result.header_rows,
        [
            "FROM <http://example.org/graphs/people>",
            "GROUP BY ?person ?name",
            "HAVING (COUNT(?friend) > 1)",
            "ORDER BY DESC(?friends) ?name",
            "LIMIT 10",
            "OFFSET 5"
        ]
    );
}

#[test]
fn a_template_draws_as_its_own_region_sharing_the_variable_nodes() {
    let result = project_fixture("construct-form.rq");
    let templates: Vec<_> = result.regions.iter().filter(|r| r.kind == "template").collect();
    assert_eq!(templates.len(), 1);
    assert_eq!(templates[0].label, "CONSTRUCT");
    node(&result, "var:s");
    assert_eq!(
        result
            .edges
            .iter()
            .filter(|e| e.scope_path == "template" && e.label == "ex:mirrored")
            .count(),
        1
    );
}

#[test]
fn ids_are_stable_across_reparses_of_an_unchanged_file() {
    assert_eq!(project_fixture("groups.rq"), project_fixture("groups.rq"));
}

#[test]
fn a_bare_describe_draws_its_target() {
    let result = project("DESCRIBE <http://purl.uniprot.org/embl-cds/AAO89367.1>");
    assert_eq!(result.nodes.len(), 1);
    assert_eq!(result.nodes[0].kind, SparqlNodeKind::Iri);
    assert_eq!(result.nodes[0].full, "http://purl.uniprot.org/embl-cds/AAO89367.1");
    assert_eq!(
        result.header_form,
        "DESCRIBE <http://purl.uniprot.org/embl-cds/AAO89367.1>"
    );
}

#[test]
fn a_describes_prefixed_target_resolves_to_the_same_node_its_patterns_use() {
    let result = project("PREFIX ex: <http://example.org/> DESCRIBE ex:thing WHERE { ?s ex:near ex:thing }");
    node(&result, "iri:http://example.org/thing");
}

#[test]
fn anonymous_variables_get_document_order_ordinals() {
    let result = project("PREFIX ex: <http://example.org/> SELECT * WHERE { [ ex:p ?a ] ex:q [ ex:r ?b ] }");
    let anonymous: Vec<_> = result
        .nodes
        .iter()
        .filter(|n| n.kind == SparqlNodeKind::Anonymous)
        .map(|n| n.id.as_str())
        .collect();
    assert_eq!(anonymous, ["anon:0", "anon:1"]);
}

#[test]
fn a_degenerately_large_query_is_cut_honestly() {
    let mut text = String::from("PREFIX ex: <http://example.org/>\nSELECT * WHERE {\n");
    for i in 0..400 {
        text.push_str(&format!("  ?s{i} ex:p{i} ?o{i} .\n"));
    }
    text.push_str("}\n");
    let result = project(&text);
    assert!(result.truncated());
    assert_eq!(result.shown, SANITY_BOUND);
    assert_eq!(result.total, 1200);
    let kept: HashSet<&str> = result.nodes.iter().map(|n| n.id.as_str()).collect();
    assert!(
        result
            .edges
            .iter()
            .all(|e| kept.contains(e.from_id.as_str()) && kept.contains(e.to_id.as_str()))
    );
}

// -- Layout ------------------------------------------------------------------------------------------------------------

const NESTED_QUERY: &str = "PREFIX ex: <http://example.org/> SELECT * WHERE { ?a ex:p ?b . OPTIONAL { ?b ex:q ?c . OPTIONAL { ?c ex:r ?d } } }";

fn contains(frame: &adp_layout::Rect, x: f64, y: f64, width: f64, height: f64) -> bool {
    x >= frame.x && y >= frame.y && x + width <= frame.x + frame.width && y + height <= frame.y + frame.height
}

/// The invariant itself, asserted for every region against every element it holds.
fn assert_containment(projection: &SparqlProjectionResult, laid_out: &SparqlLayoutResult) {
    for region in &projection.regions {
        let frame = laid_out.region_bounds[&region.id];
        let prefix = format!("{}/", region.scope_path);
        for node in projection
            .nodes
            .iter()
            .filter(|n| n.scope_path == region.scope_path || n.scope_path.starts_with(&prefix))
        {
            let (x, y) = laid_out.node_positions[&node.id];
            assert!(
                contains(&frame, x, y, NODE_WIDTH, NODE_HEIGHT),
                "{} at ({x},{y}) escapes {} {frame:?}",
                node.id,
                region.id
            );
        }
        for nested in projection.regions.iter().filter(|r| r.scope_path.starts_with(&prefix)) {
            let inner = laid_out.region_bounds[&nested.id];
            assert!(
                contains(&frame, inner.x, inner.y, inner.width, inner.height),
                "{} escapes {}",
                nested.id,
                region.id
            );
        }
    }
}

#[test]
fn computed_layout_keeps_every_regions_contents_inside_its_bounds_nested_two_deep() {
    let projection = project(NESTED_QUERY);
    let laid_out = layout::compute(&projection, &BTreeMap::new());
    assert_eq!(projection.regions.len(), 2);
    assert_containment(&projection, &laid_out);
}

#[test]
fn the_group_corpus_holds_containment_across_every_region_kind() {
    let projection = project_fixture("groups.rq");
    let laid_out = layout::compute(&projection, &BTreeMap::new());
    assert_containment(&projection, &laid_out);
    assert!(
        projection
            .nodes
            .iter()
            .all(|n| laid_out.node_positions.contains_key(&n.id))
    );
    assert!(
        projection
            .regions
            .iter()
            .all(|r| laid_out.region_bounds.contains_key(&r.id))
    );
}

#[test]
fn an_authored_region_anchor_moves_the_frame_with_its_contents_following() {
    let projection = project(NESTED_QUERY);
    let computed = layout::compute(&projection, &BTreeMap::new());
    let outer = projection
        .regions
        .iter()
        .find(|r| r.scope_path == "where/optional.0")
        .unwrap();
    let laid_out = layout::compute(&projection, &BTreeMap::from([(outer.id.clone(), (900.0, 700.0))]));
    let frame = laid_out.region_bounds[&outer.id];
    assert_eq!((frame.x, frame.y), (900.0, 700.0));
    assert_containment(&projection, &laid_out);
    let before = computed.node_positions["var:c"];
    let after = laid_out.node_positions["var:c"];
    assert_eq!(after.0 - before.0, 900.0 - computed.region_bounds[&outer.id].x);
    assert_eq!(after.1 - before.1, 700.0 - computed.region_bounds[&outer.id].y);
}

#[test]
fn an_authored_node_position_overrides_its_computed_place_and_beats_a_region_anchor() {
    let projection = project(NESTED_QUERY);
    let outer = projection
        .regions
        .iter()
        .find(|r| r.scope_path == "where/optional.0")
        .unwrap();
    let laid_out = layout::compute(
        &projection,
        &BTreeMap::from([("var:c".to_owned(), (1500.0, 40.0)), (outer.id.clone(), (900.0, 700.0))]),
    );
    assert_eq!(laid_out.node_positions["var:c"], (1500.0, 40.0));
}

#[test]
fn an_anonymous_node_always_takes_its_computed_place() {
    let projection = project("PREFIX ex: <http://example.org/> SELECT * WHERE { [ ex:p ?a ] ex:q ?b }");
    let computed = layout::compute(&projection, &BTreeMap::new());
    let laid_out = layout::compute(&projection, &BTreeMap::from([("anon:0".to_owned(), (999.0, 999.0))]));
    assert_eq!(computed.node_positions["anon:0"], laid_out.node_positions["anon:0"]);
}

#[test]
fn same_input_same_geometry() {
    let projection = project_fixture("groups.rq");
    assert_eq!(
        layout::compute(&projection, &BTreeMap::new()),
        layout::compute(&projection, &BTreeMap::new())
    );
}

#[test]
fn the_root_grid_starts_below_the_header_band_three_to_a_row() {
    // `SparqlLayout`'s constants: the root at (40, 90), boxes 170 by 64, 60 apart across and 48 down.
    let projection = project("PREFIX ex: <http://example.org/> SELECT * WHERE { ?a ex:p ?b . ?c ex:p ?d }");
    let laid_out = layout::compute(&projection, &BTreeMap::new());
    let ids: Vec<&str> = projection.nodes.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(ids, ["var:a", "var:b", "var:c", "var:d"]);
    assert_eq!(laid_out.node_positions["var:a"], (40.0, 90.0));
    assert_eq!(laid_out.node_positions["var:b"], (270.0, 90.0));
    assert_eq!(laid_out.node_positions["var:c"], (500.0, 90.0));
    assert_eq!(laid_out.node_positions["var:d"], (40.0, 202.0));
}
