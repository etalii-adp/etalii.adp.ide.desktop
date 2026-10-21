//! The chain through whatever is selected: everything that supplies it, directly or not, and everything it supplies,
//! the highlight a supply chain is read by (etalii.adp spec 030, task T076; `supply-chain.dis`, the plugin function
//! `traceOf`).
//!
//! Ported from the standalone host's `src/diagrams/supply-chain/backend/EtAlii.Adp.Diagram.SupplyChain/
//! SupplyChainTrace.cs` at `8b6ccfc`. Upstream and downstream are walked separately, each along the flows in one
//! direction only, since following goods says exactly what depends on what. A node traces from itself; a flow traces
//! upstream from its supplier and downstream from its consumer, and is itself selected; a group traces from all of
//! its members at once.

use std::collections::{HashMap, HashSet, VecDeque};

use adp_diagram::DiagramElement;

use crate::layout::SupplyChainLayout;
use crate::mapper;

/// The marking of something selected.
pub const SELECTED: &str = "selected";
/// The marking of something that supplies the selection.
pub const UPSTREAM: &str = "upstream";
/// The marking of something the selection supplies.
pub const DOWNSTREAM: &str = "downstream";
/// The marking of a group with a member on the traced chain.
pub const RELATED: &str = "related";
/// The marking of everything off the chain.
pub const DIMMED: &str = "dimmed";

/// The marks of one selection, or none.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SupplyChainTrace {
    marks: HashMap<String, &'static str>,
    active: bool,
}

impl SupplyChainTrace {
    /// No selection in this diagram: nothing is marked at all.
    pub fn none() -> SupplyChainTrace {
        SupplyChainTrace::default()
    }

    /// Whether anything in this diagram is selected, so that everything is marked.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// The marking of the entry with `id`: empty when nothing is traced, dimmed when it is off the chain.
    pub fn of(&self, id: &str) -> &'static str {
        if !self.active {
            return "";
        }
        self.marks.get(id).copied().unwrap_or(DIMMED)
    }

    /// The trace through `selected` in a laid-out document, or none when it names nothing drawn.
    pub fn through(layout: &SupplyChainLayout, selected: Option<&str>) -> SupplyChainTrace {
        SupplyChainTrace::over(&TraceGraph::of(layout), selected)
    }

    /// The trace through `selected` over what `graph` draws, or none when it names nothing drawn.
    pub fn over(graph: &TraceGraph, selected: Option<&str>) -> SupplyChainTrace {
        let Some(selected) = selected.filter(|s| !s.is_empty()) else {
            return SupplyChainTrace::none();
        };
        let is_node = graph.nodes.iter().any(|(id, _)| id == selected);
        let is_group = graph.groups.iter().any(|id| id == selected);
        let mut marks: HashMap<String, &'static str> = HashMap::new();
        let (up_from, down_from): (Vec<String>, Vec<String>) = if is_node {
            (vec![selected.to_owned()], vec![selected.to_owned()])
        } else if let Some((_, from, to)) = graph.flows.iter().find(|(id, _, _)| id == selected) {
            marks.insert(from.clone(), UPSTREAM);
            marks.insert(to.clone(), DOWNSTREAM);
            (vec![from.clone()], vec![to.clone()])
        } else if is_group {
            let members: Vec<String> = graph
                .nodes
                .iter()
                .filter(|(_, group)| group == selected)
                .map(|(id, _)| id.clone())
                .collect();
            (members.clone(), members)
        } else {
            return SupplyChainTrace::none();
        };

        walk(&up_from, &graph.flows, true, UPSTREAM, &mut marks);
        walk(&down_from, &graph.flows, false, DOWNSTREAM, &mut marks);

        // What the walk started from is the selection, whatever a walk marked it on the way round.
        if is_node || is_group {
            for id in &up_from {
                marks.insert(id.clone(), SELECTED);
            }
        }
        marks.insert(selected.to_owned(), SELECTED);

        for group in &graph.groups {
            if !marks.contains_key(group)
                && graph
                    .nodes
                    .iter()
                    .any(|(id, member_of)| member_of == group && marks.contains_key(id))
            {
                marks.insert(group.clone(), RELATED);
            }
        }
        SupplyChainTrace { marks, active: true }
    }
}

/// What a trace walks: the drawn nodes with the group each states, the drawn flows by id and ends, the drawn groups.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TraceGraph {
    pub nodes: Vec<(String, String)>,
    pub flows: Vec<(String, String, String)>,
    pub groups: Vec<String>,
}

impl TraceGraph {
    /// What `layout` draws.
    pub fn of(layout: &SupplyChainLayout) -> TraceGraph {
        TraceGraph {
            nodes: layout.nodes.iter().map(|n| (n.id.clone(), n.group.clone())).collect(),
            flows: layout
                .flows
                .iter()
                .map(|f| (f.id.clone(), f.from.clone(), f.to.clone()))
                .collect(),
            groups: layout.groups.iter().map(|g| g.id.clone()).collect(),
        }
    }

    /// What the canvas was given ([`crate::mapper`]): its cards with the group each states, its flows and its frames,
    /// so a canvas that knows the selection traces it as the standalone host's session did.
    pub fn of_elements(elements: &[DiagramElement]) -> TraceGraph {
        let mut graph = TraceGraph::default();
        for element in elements {
            if element.element_type == mapper::GROUP_TYPE {
                graph.groups.push(element.id.clone());
            } else if element.element_type == mapper::FLOW_TYPE {
                graph.flows.push((
                    element.id.clone(),
                    element.source.clone().unwrap_or_default(),
                    element.target.clone().unwrap_or_default(),
                ));
            } else if element.element_type.starts_with(mapper::PREFIX) {
                graph
                    .nodes
                    .push((element.id.clone(), element.attribute("group").to_owned()));
            }
        }
        graph
    }
}

/// Marks every flow and node reached from `starts` along the flows, against them when `upstream`. A node reached both
/// ways, goods that come back round, keeps the first marking it got.
fn walk(
    starts: &[String],
    flows: &[(String, String, String)],
    upstream: bool,
    mark: &'static str,
    marks: &mut HashMap<String, &'static str>,
) {
    let mut seen: HashSet<String> = starts.iter().cloned().collect();
    let mut queue: VecDeque<String> = starts.iter().cloned().collect();
    while let Some(id) = queue.pop_front() {
        for (flow, from, to) in flows {
            let (along, reached) = if upstream { (to, from) } else { (from, to) };
            if *along != id {
                continue;
            }
            marks.entry(flow.clone()).or_insert(mark);
            marks.entry(reached.to_owned()).or_insert(mark);
            if seen.insert(reached.to_owned()) {
                queue.push_back(reached.to_owned());
            }
        }
    }
}
