//! A double-click on the canvas (`parity/inventory.md` P-028), held to the standalone host at `8b6ccfc`: it runs what
//! the element's notation names in its `doubleClick` (DISL section 6.9), `editLabel` when it names none, through the
//! entry of that kind in the element's context menu (`compileNotation.ts`, the `activate` gesture); `none` runs
//! nothing. A module that draws its view itself runs what its canvas declares for the gesture: the Sankey diagram's,
//! the functional decomposition graph's and the supply chain's rename (`SankeyCanvas.tsx`, `FdgCanvas.tsx`,
//! `SupplyChainCanvas.tsx`), and the agent activity diagram's `editLabel` (`aadDefinition.ts`, compiled from its
//! notation). The mind map's `none` is in `walk_tests.rs`, beside the walk's other findings.

use super::Harness;

/// The inline editor a double-click opened: on `element`, with `text`.
fn renames(harness: &Harness, element: &str, text: &str) {
    let editor = harness.tab.editor.as_ref().unwrap_or_else(|| {
        panic!(
            "a double-click on {element} opens the inline editor: {:?}",
            harness.tab.notice
        )
    });
    assert_eq!(editor.element, element);
    assert_eq!(editor.text, text, "with what {element} is called");
}

/// Commits `text` in the open inline editor.
fn commit(harness: &mut Harness, text: &str) {
    harness.tab.editor.as_mut().expect("the inline editor is open").text = text.into();
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.commit_editor(host.workspace(root));
}

/// Nothing opened and nothing written.
fn does_nothing(harness: &Harness, element: &str) {
    assert!(
        harness.tab.editor.is_none(),
        "a double-click on {element} opens no editor"
    );
    assert!(harness.tab.prompt.is_none(), "and asks nothing");
    assert_eq!(harness.written(), harness.original, "and writes nothing");
}

/// `editLabel`, as the dependency graph's node names it: the label is edited in place and one undo puts it back.
#[test]
fn a_double_click_on_a_dependency_graph_node_renames_it_in_place() {
    let mut harness = Harness::dependency_graph();
    harness.double_click("api-gateway");
    let text = harness.tab.editor.as_ref().map(|e| e.text.clone()).unwrap_or_default();
    renames(&harness, "api-gateway", &text);
    assert!(!text.is_empty(), "the editor opens on the node's label");
    commit(&mut harness, "Edge router");
    assert!(
        harness.written().contains("label: Edge router"),
        "{:?}",
        harness.tab.notice
    );
    assert!(harness.workspace().undo().is_done());
    assert_eq!(harness.written(), harness.original);
}

/// The causal loop diagram's variable names `editLabel`, its loop `none`: a double-click renames the one and does
/// nothing to the other.
#[test]
fn a_double_click_renames_a_causal_loop_variable_and_does_nothing_to_a_loop() {
    let mut harness = super::causal_loop::reference();
    let a_loop = harness
        .tab
        .scene()
        .expect("a scene")
        .primitives
        .iter()
        .find_map(|p| match p {
            adp_scene::scene::Primitive::Shape(s) => match &s.owner {
                adp_scene::scene::Owner::Element(id) if id.starts_with("loop:") => Some(id.clone()),
                _ => None,
            },
            _ => None,
        })
        .expect("a loop is drawn");
    harness.double_click(&a_loop);
    does_nothing(&harness, &a_loop);

    harness.double_click("variable:deaths");
    renames(&harness, "variable:deaths", "Deaths");
    commit(&mut harness, "Mortality");
    assert!(
        harness.written().contains("variable deaths \"Mortality\""),
        "{:?}",
        harness.tab.notice
    );
    assert!(harness.workspace().undo().is_done());
    assert_eq!(harness.written(), harness.original);
}

/// A hype cycle's trend names no `doubleClick`, which is `editLabel`: its menu's Rename runs.
#[test]
fn a_double_click_on_a_hype_cycle_note_that_names_no_double_click_renames_it() {
    let mut harness = Harness::hype_cycle();
    harness.double_click("note-1");
    assert!(
        harness.tab.editor.as_ref().is_some_and(|e| e.element == "note-1"),
        "the default is editLabel: {:?}",
        harness.tab.notice
    );
}

/// The agent behaviour model's nodes name `editLabel`.
#[test]
fn a_double_click_on_an_agent_behavior_node_renames_it() {
    let mut harness = super::abm::research_assistant();
    harness.double_click("1.2.1");
    assert!(
        harness.tab.editor.as_ref().is_some_and(|e| e.element == "1.2.1"),
        "{:?}",
        harness.tab.notice
    );
}

/// The Sankey diagram's node runs its rename (`SankeyCanvas.tsx`: `activate` invokes `sankey.rename`).
#[test]
fn a_double_click_on_a_sankey_node_renames_it() {
    let mut harness = super::sankey::company();
    harness.double_click("cafes");
    renames(&harness, "cafes", "Cafés");
    commit(&mut harness, "Coffee bars");
    assert!(
        harness.written().contains("name: Coffee bars"),
        "{:?}",
        harness.tab.notice
    );
    assert!(harness.workspace().undo().is_done());
    assert_eq!(harness.written(), harness.original);
}

/// The functional decomposition graph's element runs its rename (`FdgCanvas.tsx`).
#[test]
fn a_double_click_on_a_functional_decomposition_element_renames_it() {
    let mut harness = super::fdg::field_service();
    harness.double_click("planning");
    renames(&harness, "planning", "Planning");
    commit(&mut harness, "Day plan");
    assert!(harness.written().contains("name: Day plan"), "{:?}", harness.tab.notice);
    assert!(harness.workspace().undo().is_done());
    assert_eq!(harness.written(), harness.original);
}

/// The supply chain's stage and group run its rename (`SupplyChainCanvas.tsx`).
#[test]
fn a_double_click_on_a_supply_chain_stage_or_group_renames_it() {
    let mut harness = super::supply_chain::automotive();
    harness.double_click("atacama-lithium");
    renames(&harness, "atacama-lithium", "Atacama lithium brine");
    let mut harness = super::supply_chain::automotive();
    harness.double_click("europe");
    renames(&harness, "europe", "Europe");
}

/// The agent activity diagram's elements name no `doubleClick`, which is `editLabel` (`aadDefinition.ts`).
#[test]
fn a_double_click_on_an_agent_activity_element_renames_it() {
    let mut harness = super::agent_activity::two_projects();
    harness.double_click("s-conversion");
    assert!(
        harness.tab.editor.as_ref().is_some_and(|e| e.element == "s-conversion"),
        "{:?}",
        harness.tab.notice
    );
}
