//! Collapse and expand (etalii.adp spec 030, FR-018, task T054): the fold state ported from the standalone host's
//! `MindmapConnectionView` at `8b6ccfc`. The standalone host tests it only through the mind map's session and mapper
//! (`MindmapContextSourceResolver.Tests.cs`, `MindmapElementMapper.Tests.cs`); these are the same behaviours on the
//! state alone.

use std::collections::HashMap;

use adp_layout::FoldState;

/// root > a > a1 > a11, and root > b.
fn parents() -> HashMap<&'static str, &'static str> {
    HashMap::from([("a", "root"), ("a1", "a"), ("a11", "a1"), ("b", "root")])
}

#[test]
fn a_toggle_folds_an_expanded_element_and_expands_a_folded_one() {
    let mut state = FoldState::new();

    assert!(state.toggle("a"));
    assert!(state.is_folded("a"));
    assert!(!state.toggle("a"));
    assert!(!state.is_folded("a"));
}

#[test]
fn the_folds_a_file_states_are_where_a_viewer_starts() {
    let state = FoldState::seeded_from(["a", "b"]);

    assert!(state.is_folded("a"));
    assert!(state.is_folded("b"));
    assert!(!state.is_folded("root"));
}

#[test]
fn the_folded_ancestors_of_an_element_are_listed_outermost_first() {
    let parents = parents();
    let state = FoldState::seeded_from(["a1", "root", "b"]);

    let ancestors = state.folded_ancestors_of("a11", |id| parents.get(id).copied());

    assert_eq!(ancestors, ["root", "a1"]);
}

#[test]
fn opening_each_folded_ancestor_through_the_toggle_leaves_none() {
    let parents = parents();
    let mut state = FoldState::seeded_from(["root", "a1"]);

    for ancestor in state.folded_ancestors_of("a11", |id| parents.get(id).copied()) {
        state.toggle(&ancestor);
    }

    assert!(
        state
            .folded_ancestors_of("a11", |id| parents.get(id).copied())
            .is_empty()
    );
}

#[test]
fn what_a_folded_element_holds_is_hidden_and_the_folded_element_itself_is_not() {
    let parents = parents();
    let mut state = FoldState::new();
    state.toggle("a"); // collapse "a", hiding "a1" and "a11"

    let hidden = |id: &str| state.is_hidden(id, |id| parents.get(id).copied());

    assert!(!hidden("a"));
    assert!(hidden("a1"));
    assert!(hidden("a11"));
    assert!(!hidden("b"));
    assert!(!hidden("root"));
}
