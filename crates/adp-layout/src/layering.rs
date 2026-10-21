//! Layers of a directed graph, for the layouts that draw one column per layer (etalii.adp spec 030, FR-018), ported
//! from the standalone host's `EtAlii.Adp.Layout/Layering.cs` at `8b6ccfc`.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::cycle_policy::CyclePolicy;

/// Every node's layer, with no floor: see [`longest_path_with_floor`].
pub fn longest_path<N, F, T>(
    nodes: impl IntoIterator<Item = N>,
    edges: impl IntoIterator<Item = (F, T)>,
    policy: CyclePolicy,
) -> HashMap<String, i32>
where
    N: AsRef<str>,
    F: AsRef<str>,
    T: AsRef<str>,
{
    longest_path_with_floor(nodes, edges, policy, |_| 0)
}

/// Every node's layer: the length of the longest path of edges leading into it, so every edge points to a later layer
/// and a node with nothing before it sits in layer zero.
///
/// `nodes` are given in the order a tie is resolved: earlier nodes are walked first. `edges` run from the node that
/// comes first to the node that waits for it; an edge naming a node outside `nodes` is ignored. `policy` says what an
/// edge closing a cycle does, and `floor` is the lowest layer each node may take, where something other than its edges
/// places it. The result holds the layer of each node.
pub fn longest_path_with_floor<N, F, T>(
    nodes: impl IntoIterator<Item = N>,
    edges: impl IntoIterator<Item = (F, T)>,
    policy: CyclePolicy,
    floor: impl Fn(&str) -> i32,
) -> HashMap<String, i32>
where
    N: AsRef<str>,
    F: AsRef<str>,
    T: AsRef<str>,
{
    let order: Vec<String> = nodes.into_iter().map(|node| node.as_ref().to_string()).collect();
    let mut keys = Vec::new();
    let mut before: HashMap<String, Vec<String>> = HashMap::new();
    for node in &order {
        if !before.contains_key(node) {
            before.insert(node.clone(), Vec::new());
            keys.push(node.clone());
        }
    }

    for (from, to) in edges {
        let (from, to) = (from.as_ref(), to.as_ref());
        if before.contains_key(from)
            && let Some(dependencies) = before.get_mut(to)
        {
            dependencies.push(from.to_string());
        }
    }

    let graph = Graph {
        before: &before,
        floor: &floor,
    };
    match policy {
        CyclePolicy::BackEdgeIsZero => depth_first(&order, &graph, 0),
        CyclePolicy::BackEdgeIgnored => depth_first(&order, &graph, -1),
        CyclePolicy::BackEdgeIsZeroPerNode => keys
            .iter()
            .map(|node| (node.clone(), unremembered(node, &graph, &mut HashSet::new())))
            .collect(),
        CyclePolicy::Kahn => kahn(&keys, &graph),
    }
}

/// The edges that close no cycle, found by a depth-first walk in the order of `nodes`: an edge into a node still on the
/// walk is left out, every other edge is kept.
///
/// `outgoing` holds each node's targets, in the order they are walked; every node has an entry. The result holds each
/// node's kept targets.
///
/// # Panics
///
/// When a node or a target reached has no entry in `outgoing`, as the standalone host throws there.
pub fn acyclic_edges<N: AsRef<str>>(
    nodes: &[N],
    outgoing: &HashMap<String, Vec<String>>,
) -> HashMap<String, Vec<String>> {
    let mut forward: HashMap<String, Vec<String>> = nodes
        .iter()
        .map(|node| (node.as_ref().to_string(), Vec::new()))
        .collect();
    // 1 is a node still on the walk, 2 a node finished.
    let mut state: HashMap<String, u8> = HashMap::new();

    for start in nodes {
        let start = start.as_ref();
        if state.contains_key(start) {
            continue;
        }

        // An explicit stack: a long chain must not overflow the call stack.
        let mut stack: Vec<(String, usize)> = vec![(start.to_string(), 0)];
        state.insert(start.to_string(), 1);
        while let Some((id, next)) = stack.pop() {
            let targets = &outgoing[&id];
            if next >= targets.len() {
                state.insert(id, 2);
                continue;
            }

            let target = targets[next].clone();
            stack.push((id.clone(), next + 1));
            match state.get(&target) {
                None => {
                    forward.entry(id).or_default().push(target.clone());
                    state.insert(target.clone(), 1);
                    stack.push((target, 0));
                }
                Some(2) => forward.entry(id).or_default().push(target),
                // 1 is a node still on the walk: this edge closes a cycle and is left out.
                Some(_) => {}
            }
        }
    }

    forward
}

/// The edges into each node and the floor each node may not go below.
struct Graph<'a, Floor: Fn(&str) -> i32> {
    before: &'a HashMap<String, Vec<String>>,
    floor: &'a Floor,
}

/// A depth-first walk that remembers nothing, so a node reached along two paths is measured twice, each time against
/// the stack of the path that reached it.
fn unremembered<Floor: Fn(&str) -> i32>(node: &str, graph: &Graph<'_, Floor>, resolving: &mut HashSet<String>) -> i32 {
    if !resolving.insert(node.to_string()) {
        return 0;
    }

    let mut depth = (graph.floor)(node);
    for dependency in &graph.before[node] {
        depth = depth.max(unremembered(dependency, graph, resolving) + 1);
    }

    resolving.remove(node);
    depth
}

/// A memoised depth-first walk; `back_edge` is the depth a node already on the stack counts as, so 0 lets the edge
/// into it push one layer and -1 lets it push none.
fn depth_first<Floor: Fn(&str) -> i32>(
    order: &[String],
    graph: &Graph<'_, Floor>,
    back_edge: i32,
) -> HashMap<String, i32> {
    struct Walk<'g, 'a, Floor: Fn(&str) -> i32> {
        graph: &'g Graph<'a, Floor>,
        back_edge: i32,
        layers: HashMap<String, i32>,
        resolving: HashSet<String>,
    }

    impl<Floor: Fn(&str) -> i32> Walk<'_, '_, Floor> {
        fn depth(&mut self, node: &str) -> i32 {
            if let Some(&known) = self.layers.get(node) {
                return known;
            }

            if !self.resolving.insert(node.to_string()) {
                return self.back_edge;
            }

            let mut depth = (self.graph.floor)(node);
            for dependency in &self.graph.before[node] {
                depth = depth.max(self.depth(dependency) + 1);
            }

            self.resolving.remove(node);
            self.layers.insert(node.to_string(), depth);
            depth
        }
    }

    let mut walk = Walk {
        graph,
        back_edge,
        layers: HashMap::new(),
        resolving: HashSet::new(),
    };
    for node in order {
        walk.depth(node);
    }

    walk.layers
}

/// Kahn's algorithm: a node is placed once everything before it is, so a node on a cycle, or after one, is never
/// released and keeps what its released predecessors gave it.
fn kahn<Floor: Fn(&str) -> i32>(keys: &[String], graph: &Graph<'_, Floor>) -> HashMap<String, i32> {
    let mut after: HashMap<&str, Vec<&str>> = keys.iter().map(|node| (node.as_str(), Vec::new())).collect();
    let mut waiting: HashMap<&str, usize> = keys
        .iter()
        .map(|node| (node.as_str(), graph.before[node].len()))
        .collect();
    for node in keys {
        for dependency in &graph.before[node] {
            after
                .get_mut(dependency.as_str())
                .expect("every dependency is a node")
                .push(node.as_str());
        }
    }

    let mut layers: HashMap<String, i32> = keys.iter().map(|node| (node.clone(), (graph.floor)(node))).collect();
    let mut ready: VecDeque<&str> = keys
        .iter()
        .map(String::as_str)
        .filter(|node| waiting[node] == 0)
        .collect();
    while let Some(node) = ready.pop_front() {
        let layer = layers[node];
        for &next in &after[node] {
            let entry = layers.get_mut(next).expect("every node has a layer");
            *entry = (*entry).max(layer + 1);
            let count = waiting.get_mut(next).expect("every node waits for a count");
            *count -= 1;
            if *count == 0 {
                ready.push_back(next);
            }
        }
    }

    layers
}
