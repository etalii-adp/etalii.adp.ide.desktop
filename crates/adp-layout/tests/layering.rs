//! Longest-path layering and its cycle rules (etalii.adp spec 030, FR-018, task T054), ported from the standalone
//! host's `EtAlii.Adp.Layout.Tests/Layering.Tests.cs` at `8b6ccfc`.

use std::collections::{BTreeMap, HashMap};

use adp_layout::CyclePolicy;
use adp_layout::layering::{acyclic_edges, longest_path, longest_path_with_floor};

fn sorted(layers: HashMap<String, i32>) -> BTreeMap<String, i32> {
    layers.into_iter().collect()
}

fn outgoing(entries: &[(&str, &[&str])]) -> HashMap<String, Vec<String>> {
    entries
        .iter()
        .map(|(node, targets)| {
            (
                node.to_string(),
                targets.iter().map(|target| target.to_string()).collect(),
            )
        })
        .collect()
}

#[test]
fn longest_path_puts_a_node_after_its_longest_chain() {
    // Arrange: c waits for a directly and for a through b, so the longer chain decides.
    let nodes = ["a", "b", "c"];
    let edges = [("a", "b"), ("b", "c"), ("a", "c")];

    // Act.
    let layers = longest_path(nodes, edges, CyclePolicy::BackEdgeIsZero);

    // Assert.
    assert_eq!(layers["a"], 0);
    assert_eq!(layers["b"], 1);
    assert_eq!(layers["c"], 2);
}

#[test]
fn longest_path_ignores_an_edge_to_an_unknown_node() {
    // Arrange.
    let nodes = ["a", "b"];
    let edges = [("missing", "a"), ("a", "missing"), ("a", "b")];

    // Act.
    let layers = longest_path(nodes, edges, CyclePolicy::BackEdgeIsZero);

    // Assert.
    assert_eq!(layers.len(), 2);
    assert_eq!(layers["a"], 0);
    assert_eq!(layers["b"], 1);
}

#[test]
fn back_edge_is_zero_counts_the_node_on_the_stack_as_layer_zero() {
    // Arrange: a and b wait for each other. The walk starts at a, reaches b, and meets a on its stack: a counts as zero
    // there, so b is one, and a, waiting for b, is two.
    let nodes = ["a", "b"];
    let edges = [("b", "a"), ("a", "b")];

    // Act.
    let layers = longest_path(nodes, edges, CyclePolicy::BackEdgeIsZero);

    // Assert.
    assert_eq!(layers["a"], 2);
    assert_eq!(layers["b"], 1);
}

#[test]
fn back_edge_is_zero_a_self_loop_moves_its_node_one_layer() {
    // Arrange.
    let nodes = ["a"];
    let edges = [("a", "a")];

    // Act.
    let layers = longest_path(nodes, edges, CyclePolicy::BackEdgeIsZero);

    // Assert.
    assert_eq!(layers["a"], 1);
}

#[test]
fn back_edge_ignored_lets_the_edge_into_the_stack_push_nothing() {
    // Arrange: a and b wait for each other. The walk starts at a, reaches b, and meets a on its stack: that edge is left
    // out, so b is zero, and a, waiting for b, is one.
    let nodes = ["a", "b"];
    let edges = [("b", "a"), ("a", "b")];

    // Act.
    let layers = longest_path(nodes, edges, CyclePolicy::BackEdgeIgnored);

    // Assert.
    assert_eq!(layers["a"], 1);
    assert_eq!(layers["b"], 0);
}

#[test]
fn back_edge_is_zero_per_node_measures_every_cycle_member_from_itself() {
    // Arrange: the same two-node cycle. Each walk starts at its own node, so neither member's layer depends on which was
    // walked first.
    let nodes = ["a", "b"];
    let edges = [("b", "a"), ("a", "b")];

    // Act.
    let layers = longest_path(nodes, edges, CyclePolicy::BackEdgeIsZeroPerNode);

    // Assert.
    assert_eq!(layers["a"], 2);
    assert_eq!(layers["b"], 2);
}

#[test]
fn every_policy_agrees_with_back_edge_is_zero_without_a_cycle() {
    for policy in [
        CyclePolicy::BackEdgeIgnored,
        CyclePolicy::BackEdgeIsZeroPerNode,
        CyclePolicy::Kahn,
    ] {
        // Arrange: a diamond with a tail, so a node is reached along two paths.
        let nodes = ["d", "c", "b", "a", "e"];
        let edges = [("a", "b"), ("a", "c"), ("b", "d"), ("c", "d"), ("d", "e"), ("a", "e")];

        // Act.
        let expected = longest_path(nodes, edges, CyclePolicy::BackEdgeIsZero);
        let actual = longest_path(nodes, edges, policy);

        // Assert.
        assert_eq!(sorted(expected), sorted(actual), "{policy:?}");
    }
}

#[test]
fn kahn_leaves_a_cycle_unreleased_with_what_its_released_predecessors_gave() {
    // Arrange: c comes before a, and a and b wait for each other. c is released and gives a layer one; a still waits for
    // b, so neither is ever released and b keeps zero.
    let nodes = ["c", "a", "b"];
    let edges = [("c", "a"), ("a", "b"), ("b", "a")];

    // Act.
    let layers = longest_path(nodes, edges, CyclePolicy::Kahn);

    // Assert.
    assert_eq!(layers["c"], 0);
    assert_eq!(layers["a"], 1);
    assert_eq!(layers["b"], 0);
}

#[test]
fn a_floor_raises_its_node_and_pushes_what_comes_after() {
    for policy in [
        CyclePolicy::BackEdgeIsZero,
        CyclePolicy::BackEdgeIgnored,
        CyclePolicy::BackEdgeIsZeroPerNode,
        CyclePolicy::Kahn,
    ] {
        // Arrange: b may sit no further left than layer three, so c, after it, is four.
        let nodes = ["a", "b", "c"];
        let edges = [("a", "b"), ("b", "c")];

        // Act.
        let layers = longest_path_with_floor(nodes, edges, policy, |node| if node == "b" { 3 } else { 0 });

        // Assert.
        assert_eq!(layers["a"], 0, "{policy:?}");
        assert_eq!(layers["b"], 3, "{policy:?}");
        assert_eq!(layers["c"], 4, "{policy:?}");
    }
}

#[test]
fn acyclic_edges_leave_out_the_edge_that_closes_a_cycle() {
    // Arrange: a -> b -> c -> a, walked from a, so c -> a is the edge that closes it.
    let nodes = ["a", "b", "c"];
    let outgoing = outgoing(&[("a", &["b"]), ("b", &["c"]), ("c", &["a"])]);

    // Act.
    let forward = acyclic_edges(&nodes, &outgoing);

    // Assert.
    assert_eq!(forward["a"], ["b"]);
    assert_eq!(forward["b"], ["c"]);
    assert!(forward["c"].is_empty());
}

#[test]
fn acyclic_edges_keep_every_edge_of_a_diamond() {
    // Arrange: two paths from a to d; the second reaches d after it is finished.
    let nodes = ["a", "b", "c", "d"];
    let outgoing = outgoing(&[("a", &["b", "c"]), ("b", &["d"]), ("c", &["d"]), ("d", &[])]);

    // Act.
    let forward = acyclic_edges(&nodes, &outgoing);

    // Assert.
    assert_eq!(forward["a"], ["b", "c"]);
    assert_eq!(forward["b"], ["d"]);
    assert_eq!(forward["c"], ["d"]);
    assert!(forward["d"].is_empty());
}
