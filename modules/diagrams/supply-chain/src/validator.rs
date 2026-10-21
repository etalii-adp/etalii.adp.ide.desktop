//! What is wrong with a `.supply` document, each finding on the line it breaks, as the problems panel shows it
//! (etalii.adp spec 030, task T076; `supply-chain.dis`, `constraints`).
//!
//! Ported from the standalone host's `src/diagrams/supply-chain/backend/EtAlii.Adp.Diagram.SupplyChain/
//! SupplyChainValidator.cs` at `8b6ccfc`. A document is never refused for what is wrong with it: what is lost (a flow
//! to a node that is not there, a duplicate id, a flow to itself) is an error, and what is merely untidy (an unreadable
//! line, an unknown group, an empty one) a warning.

use std::collections::HashSet;

use adp_diagram::{DiagramProblem, DiagramProblemLocation, DiagramProblemSeverity};
use indexmap::IndexMap;

use crate::model::SupplyChainModel;

pub const UNREADABLE_ENTRY: &str = "supply-chain.unreadable";
pub const MISSING_ID: &str = "supply-chain.missing-id";
pub const DUPLICATE_ID: &str = "supply-chain.duplicate-id";
pub const DANGLING_FLOW: &str = "supply-chain.dangling-flow";
pub const SELF_FLOW: &str = "supply-chain.self-flow";
pub const UNKNOWN_GROUP: &str = "supply-chain.unknown-group";
pub const EMPTY_GROUP: &str = "supply-chain.empty-group";

/// One thing wrong with a document, on the 0-based line it was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupplyChainBreach {
    pub rule_id: &'static str,
    pub message: String,
    pub line: usize,
    pub is_warning: bool,
}

fn breach(rule_id: &'static str, message: String, line: usize, is_warning: bool) -> SupplyChainBreach {
    SupplyChainBreach {
        rule_id,
        message,
        line,
        is_warning,
    }
}

/// Everything wrong with a model already read, in the order of the lines it is on.
pub fn validate(model: &SupplyChainModel) -> Vec<SupplyChainBreach> {
    let mut breaches: Vec<SupplyChainBreach> = model
        .problems
        .iter()
        .map(|p| breach(UNREADABLE_ENTRY, p.message.clone(), p.line, true))
        .collect();

    let entries: Vec<(&str, &str, usize)> = model
        .groups
        .iter()
        .map(|g| (g.id.as_str(), "group", g.range.start))
        .chain(model.nodes.iter().map(|n| (n.id.as_str(), "node", n.range.start)))
        .chain(model.flows.iter().map(|f| (f.id.as_str(), "flow", f.range.start)))
        .collect();

    breaches.extend(
        entries
            .iter()
            .filter(|(id, _, _)| id.is_empty())
            .map(|(_, what, start)| {
                breach(
                    MISSING_ID,
                    format!("A {what} has no id and is not drawn."),
                    *start,
                    false,
                )
            }),
    );

    let mut by_id: IndexMap<&str, Vec<usize>> = IndexMap::new();
    for (id, _, start) in entries.iter().filter(|(id, _, _)| !id.is_empty()) {
        by_id.entry(id).or_default().push(*start);
    }
    for (id, starts) in &by_id {
        for start in starts.iter().skip(1) {
            breaches.push(breach(
                DUPLICATE_ID,
                format!("The id `{id}` is used more than once; only its first entry is drawn."),
                *start,
                false,
            ));
        }
    }

    let nodes: HashSet<&str> = model.nodes.iter().map(|n| n.id.as_str()).collect();
    for flow in &model.flows {
        if !flow.from.is_empty() && flow.from == flow.to {
            breaches.push(breach(
                SELF_FLOW,
                format!(
                    "The flow `{}` runs from `{}` to itself and is not drawn.",
                    flow.id, flow.from
                ),
                flow.range.start,
                false,
            ));
            continue;
        }
        for end in [&flow.from, &flow.to] {
            if nodes.contains(end.as_str()) {
                continue;
            }
            let message = if end.is_empty() {
                format!("The flow `{}` is missing an end and is not drawn.", flow.id)
            } else {
                format!(
                    "The flow `{}` names `{end}`, which is not a node here; it is not drawn.",
                    flow.id
                )
            };
            breaches.push(breach(DANGLING_FLOW, message, flow.range.start, false));
        }
    }

    let groups: HashSet<&str> = model.groups.iter().map(|g| g.id.as_str()).collect();
    breaches.extend(
        model
            .nodes
            .iter()
            .filter(|n| !n.group.is_empty() && !groups.contains(n.group.as_str()))
            .map(|n| {
                breach(
                    UNKNOWN_GROUP,
                    format!(
                        "The node `{}` is in `{}`, which is not a group here; it is drawn outside every group.",
                        n.id, n.group
                    ),
                    n.range.start,
                    true,
                )
            }),
    );
    breaches.extend(
        model
            .groups
            .iter()
            .filter(|g| !g.id.is_empty() && !g.is_placed() && model.nodes.iter().all(|n| n.group != g.id))
            .map(|g| {
                breach(
                    EMPTY_GROUP,
                    format!(
                        "The group `{}` has no nodes and no `x` and `y`, so it is not drawn.",
                        g.id
                    ),
                    g.range.start,
                    true,
                )
            }),
    );

    // A stable sort by line, as `OrderBy` sorts.
    breaches.sort_by_key(|b| b.line);
    breaches
}

/// The breaches of `model` as the problems panel shows them, each on its 1-based line.
pub fn problems(model: &SupplyChainModel) -> Vec<DiagramProblem> {
    validate(model)
        .into_iter()
        .map(|b| DiagramProblem {
            severity: if b.is_warning {
                DiagramProblemSeverity::Warning
            } else {
                DiagramProblemSeverity::Error
            },
            message: b.message,
            rule_id: b.rule_id.to_owned(),
            location: Some(DiagramProblemLocation::Line(b.line as u32 + 1)),
        })
        .collect()
}
