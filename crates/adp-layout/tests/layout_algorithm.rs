//! The layout seam (etalii.adp spec 030, FR-018, task T054), ported from the standalone host's
//! `src/client/src/canvas/library/layout/layoutAlgorithm.test.tsx` at `8b6ccfc`, its "the layout seam" part. The
//! part that mounts the canvas (the switcher, the active mode drawn, a drag under an automatic mode) belongs to the
//! canvas's own tests (tasks T055, T056).

use adp_layout::{
    LAYOUT_ALGORITHMS, LayoutConnection, LayoutDefinition, LayoutElement, LayoutInput, LayoutMode, TreeDirection,
    layout_algorithm_for,
};

fn element(id: &str, x: f64, y: f64, parent_id: Option<&str>) -> LayoutElement {
    LayoutElement {
        parent_id: parent_id.map(str::to_string),
        ..LayoutElement::new(id, x, y, 100.0, 40.0)
    }
}

fn tree(direction: TreeDirection) -> LayoutDefinition {
    LayoutDefinition {
        tree_direction: Some(direction),
        ..LayoutDefinition::with_modes(&[LayoutMode::Tree])
    }
}

fn mode(mode: LayoutMode) -> &'static dyn adp_layout::LayoutAlgorithm {
    layout_algorithm_for(mode).unwrap()
}

#[test]
fn manual_returns_the_models_own_positions_untouched_literally_no_second_store() {
    // None, not a copied map: there is nothing to drift from the model (Requirement 8.3).
    let input = LayoutInput {
        elements: vec![element("a", 7.0, 9.0, None)],
        connections: vec![],
    };

    assert!(
        mode(LayoutMode::Manual)
            .place(&input, &LayoutDefinition::with_modes(&[LayoutMode::Manual]), None)
            .is_none()
    );
}

#[test]
fn tree_places_children_beyond_their_parent_along_the_direction_siblings_apart_on_the_cross_axis() {
    let input = LayoutInput {
        elements: vec![
            element("root", 0.0, 0.0, None),
            element("one", 0.0, 0.0, Some("root")),
            element("two", 0.0, 0.0, Some("root")),
        ],
        connections: vec![],
    };

    let positions = mode(LayoutMode::Tree)
        .place(&input, &tree(TreeDirection::LeftToRight), None)
        .unwrap();

    let (root, one, two) = (positions["root"], positions["one"], positions["two"]);
    assert!(one.x > root.x);
    assert_eq!(two.x, one.x);
    assert!(two.y > one.y);
    // The parent sits centred on its children's span.
    assert!((root.y - (one.y + two.y) / 2.0).abs() < 5e-6);
}

#[test]
fn tree_grows_down_when_the_definition_says_top_down_and_hangs_on_connections_when_no_parent_id_does() {
    let input = LayoutInput {
        elements: vec![element("root", 0.0, 0.0, None), element("leaf", 0.0, 0.0, None)],
        connections: vec![LayoutConnection::new("root", "leaf")],
    };

    let positions = mode(LayoutMode::Tree)
        .place(&input, &tree(TreeDirection::TopDown), None)
        .unwrap();

    assert!(positions["leaf"].y > positions["root"].y);
    assert_eq!(positions["leaf"].x, positions["root"].x);
}

#[test]
fn a_further_algorithm_is_an_addition_the_registry_answers_for_its_members_and_nothing_else() {
    // Named-member canaries (Requirement 10.2).
    assert_eq!(mode(LayoutMode::Manual).mode(), LayoutMode::Manual);
    assert_eq!(mode(LayoutMode::Tree).mode(), LayoutMode::Tree);
    // Declared in the schema, with no algorithm yet: the canvas falls back to manual rather than inventing placements.
    assert!(layout_algorithm_for(LayoutMode::HorizontalFlow).is_none());
    assert_eq!(mode(LayoutMode::RowPacked).mode(), LayoutMode::RowPacked);
    assert_eq!(mode(LayoutMode::TieredForce).mode(), LayoutMode::TieredForce);
    assert_eq!(LAYOUT_ALGORITHMS.len(), 4);
}

#[test]
fn every_mode_reads_and_writes_by_the_name_the_definitions_use() {
    // The names the standalone host's `LayoutMode` union lists; not a test of the standalone host's, but the port's
    // string form is what a definition is read with.
    let names = [
        "manual",
        "horizontal-flow",
        "vertical-flow",
        "tree",
        "layered-graph",
        "row-packed",
        "tiered-force",
    ];
    for (mode, name) in LayoutMode::ALL.into_iter().zip(names) {
        assert_eq!(mode.name(), name);
        assert_eq!(LayoutMode::from_name(name), Some(mode));
    }
    assert_eq!(LayoutMode::from_name("cloud"), None);
}
