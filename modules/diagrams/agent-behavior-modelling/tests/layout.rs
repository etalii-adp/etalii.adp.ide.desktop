//! The tidy tree, its dragged rows and Arrange diagram (etalii.adp spec 030, task T074), ported from the standalone
//! host's `AbmLayout.Tests.cs`, `AbmArrangement.Tests.cs` and `AbmSession.Tests.cs` at `8b6ccfc`: the order is the
//! meaning, a drag moves a row and may reorder it, both files written as one edit and one undo, and Arrange diagram
//! forgets every dragged position without touching the Markdown.

mod common;

use adp_diagram::session::ActionTarget;
use adp_diagram_agent_behavior_modelling::actions::ARRANGE;
use adp_diagram_agent_behavior_modelling::layout::{
    self, HORIZONTAL_GAP, MINIMUM_GAP, NODE_HEIGHT, NODE_WIDTH, Positions, VERTICAL_GAP,
};
use adp_diagram_agent_behavior_modelling::registration;
use common::{EXAMPLES, TestProject, example, parse};

const CLOSE: f64 = 1e-6;

#[test]
fn children_run_left_to_right_in_document_order_under_a_centred_parent() {
    let model = parse(
        "## Behavior\n- **Do in order:** A\n  - **Do:** B\n  - **Do in order:** C\n    - **Do:** D\n    - **Do:** E\n  - **Do:** F\n",
    );
    let p = layout::compute(&model);
    assert!(p["1.1"].0 < p["1.2"].0 && p["1.2"].0 < p["1.3"].0);
    assert!(p["1.2.1"].0 < p["1.2.2"].0);
    assert_eq!(p["1"].1 + NODE_HEIGHT + VERTICAL_GAP, p["1.1"].1);
    assert!(((p["1.1"].0 + p["1.3"].0) / 2.0 - p["1"].0).abs() < CLOSE);
    assert!(((p["1.2.1"].0 + p["1.2.2"].0) / 2.0 - p["1.2"].0).abs() < CLOSE);
}

#[test]
fn a_row_takes_its_stored_height_and_the_rows_beneath_it_follow() {
    let model = parse("## Behavior\n- **Do in order:** A\n  - **Do:** B\n  - **Do in order:** C\n    - **Do:** D\n");
    let computed = layout::compute(&model);
    // The second child's row is stored 300 lower than computed; its x is ignored.
    let stored: Positions = [("1.2".to_owned(), (-5000.0, computed["1.2"].1 + 300.0))]
        .into_iter()
        .collect();
    let p = layout::arrange(&model, &stored);
    assert_eq!(computed["1"], p["1"]);
    for id in ["1.1", "1.2", "1.2.1"] {
        assert_eq!((computed[id].0, computed[id].1 + 300.0), p[id], "{id}");
    }
}

#[test]
fn a_row_is_never_drawn_above_its_parent() {
    let model = parse("## Behavior\n- **Do in order:** A\n  - **Do:** B\n");
    let stored: Positions = [("1.1".to_owned(), (0.0, -1000.0))].into_iter().collect();
    let p = layout::arrange(&model, &stored);
    assert_eq!(p["1"].1 + NODE_HEIGHT + MINIMUM_GAP, p["1.1"].1);
}

#[test]
fn no_two_nodes_overlap_in_any_example() {
    for name in EXAMPLES {
        let positions: Vec<(f64, f64)> = layout::compute(&parse(&example(name))).into_values().collect();
        for (i, a) in positions.iter().enumerate() {
            for b in &positions[i + 1..] {
                let overlap = (a.0 - b.0).abs() < NODE_WIDTH && (a.1 - b.1).abs() < NODE_HEIGHT;
                assert!(!overlap, "{name}: two nodes overlap at {a:?} and {b:?}");
            }
        }
    }
}

#[test]
fn a_leaf_between_two_bushes_tucks_in_so_the_bushes_children_sit_side_by_side() {
    // P and Q each have three children, and the leaf L sits between them: outlines let Q's children follow P's.
    let model = parse(concat!(
        "## Behavior\n- **Do in order:** R\n",
        "  - **Do in order:** P\n    - **Do:** P1\n    - **Do:** P2\n    - **Do:** P3\n",
        "  - **Do:** L\n",
        "  - **Do in order:** Q\n    - **Do:** Q1\n    - **Do:** Q2\n    - **Do:** Q3\n",
    ));
    let p = layout::compute(&model);
    assert!(p["1.1"].0 < p["1.2"].0 && p["1.2"].0 < p["1.3"].0);
    assert!((p["1.3.1"].0 - p["1.1.3"].0 - (NODE_WIDTH + HORIZONTAL_GAP)).abs() < CLOSE);
    assert!(((p["1.1"].0 + p["1.3"].0) / 2.0 - p["1"].0).abs() < CLOSE);
}

#[test]
fn a_drag_down_moves_the_whole_row_and_everything_beneath_it_and_is_undone() {
    let mut project = TestProject::example("bug-fixer");
    let markdown = project.body();
    let computed = layout::compute(&project.model());
    // 1.2 dropped 100 lower, and a little across but not past a neighbour.
    project
        .drop_at("1.2", computed["1.2"].0 + 20.0, computed["1.2"].1 + 100.0)
        .unwrap();
    let drawn = project.drawn();
    assert_eq!(computed["1"], drawn["1"]);
    for node in project.model().nodes.iter().filter(|n| n.id != "1") {
        assert_eq!(
            (computed[&node.id].0, computed[&node.id].1 + 100.0),
            drawn[&node.id],
            "{}",
            node.id
        );
    }
    assert_eq!(project.body(), markdown, "the Markdown did not change by a byte");
    project.undo();
    assert!(registration::read(project.registration().as_bytes()).is_empty());
}

#[test]
fn a_drag_across_past_a_sibling_swaps_them_in_the_markdown_their_rows_follow_and_one_undo_puts_both_files_back() {
    let mut project = TestProject::example("bug-fixer");
    let computed = layout::compute(&project.model());
    // The first child's own children are dragged 40 lower first.
    project
        .drop_at("1.1.1", computed["1.1.1"].0, computed["1.1.1"].1 + 40.0)
        .unwrap();
    let (markdown, stored) = (project.body(), project.registration());

    // "Understand the report" dropped just right of "Write a test"'s middle.
    project
        .drop_at("1.1", computed["1.2"].0 + 10.0, computed["1.1"].1)
        .unwrap();
    let model = project.model();
    assert_eq!(
        (
            model.node("1.1").unwrap().kind.as_str(),
            model.node("1.2").unwrap().kind.as_str()
        ),
        ("action", "fallback")
    );
    let text = project.body();
    assert!(text.find("Write a test").unwrap() < text.find("Understand the report").unwrap());
    // The dragged node's children kept the height they were given, under their new ids.
    let drawn = project.drawn();
    assert_eq!(drawn["1.2"].1 + NODE_HEIGHT + VERTICAL_GAP + 40.0, drawn["1.2.1"].1);
    assert_eq!(drawn["1.2.1"].1, drawn["1.2.2"].1);
    assert!(drawn["1.1"].0 < drawn["1.2"].0);

    project.undo();
    assert_eq!(project.body(), markdown);
    assert_eq!(project.registration(), stored);
    assert_eq!(project.model().node("1.1").unwrap().kind, "fallback");
    project.redo();
    assert_eq!(project.model().node("1.1").unwrap().kind, "action");
}

#[test]
fn a_drop_where_it_was_writes_nothing() {
    let mut project = TestProject::example("bug-fixer");
    let computed = layout::compute(&project.model());
    project
        .drop_at("1.3", computed["1.3"].0 - 30.0, computed["1.3"].1)
        .unwrap();
    assert!(!project.can_undo());
    assert!(registration::read(project.registration().as_bytes()).is_empty());
}

#[test]
fn a_row_dragged_above_its_parent_stops_just_below_it() {
    let mut project = TestProject::example("bug-fixer");
    let computed = layout::compute(&project.model());
    project
        .drop_at("1.2", computed["1.2"].0, computed["1"].1 - 500.0)
        .unwrap();
    assert_eq!(computed["1"].1 + NODE_HEIGHT + MINIMUM_GAP, project.drawn()["1.2"].1);
}

#[test]
fn a_drag_onto_another_node_moves_it_under_that_node_on_the_history() {
    let mut project = TestProject::example("bug-fixer");
    let children = project.model().node("1.1").unwrap().child_ids.len();
    // The test-writing step, dropped as the first child of the step before it.
    project
        .session
        .move_element(&mut project.workspace, "1.2", Some("1.1"), 0)
        .unwrap();
    assert_eq!(project.model().node("1.1").unwrap().child_ids.len(), children + 1);
    assert!(project.can_undo());
}

#[test]
fn arranging_forgets_every_dragged_position_and_its_undo_and_redo_are_exact() {
    let mut project = TestProject::with(
        &example("bug-fixer"),
        "etalii/agent-behavior-modelling\r\nlayout:\r\n  1: 900 40\r\n  1.1: -300 500\r\n",
    );
    let (dragged, markdown) = (project.registration(), project.body());
    assert_eq!(registration::read(dragged.as_bytes()).len(), 2);
    let arranged = project.execute(ActionTarget::Canvas { x: 0.0, y: 0.0 }, ARRANGE);
    assert_eq!(arranged, adp_diagram::ActionOutcome::Done);
    assert_eq!(project.registration(), "etalii/agent-behavior-modelling\r\n");
    assert_eq!(project.body(), markdown);
    project.undo();
    assert_eq!(project.registration(), dragged);
    project.redo();
    assert!(registration::read(project.registration().as_bytes()).is_empty());
}

#[test]
fn arranging_with_nothing_dragged_is_refused_and_writes_nothing() {
    let mut project = TestProject::example("bug-fixer");
    let before = project.registration();
    match project.on("1", ARRANGE) {
        adp_diagram::ActionOutcome::Refused(refusal) => assert!(refusal.contains("already arranged"), "{refusal}"),
        other => panic!("{other:?}"),
    }
    assert_eq!(project.registration(), before);
}
