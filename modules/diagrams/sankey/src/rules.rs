//! What is wrong with a `.skv` document, without ever refusing to read it (etalii.adp spec 030, task T077).
//!
//! Ported from the standalone host's `SankeyValidator.cs` at `8b6ccfc`. What is lost is an error, what is merely
//! untidy a warning: a node without an id, a flow to a node that is not there, a duplicate or a flow to itself are not
//! drawn; a flow without a value, an unknown colour or a flow running backwards still draw what the author meant.

use std::collections::{HashMap, HashSet};

use adp_diagram::{DiagramProblem, DiagramProblemLocation, DiagramProblemSeverity};

use crate::format::{self, PALETTE};
use crate::layout::SankeyLayout;
use crate::model::SankeyModel;

pub const UNREADABLE_ENTRY: &str = "sankey.unreadable";
pub const MISSING_ID: &str = "sankey.missing-id";
pub const DUPLICATE_ID: &str = "sankey.duplicate-id";
pub const DANGLING_FLOW: &str = "sankey.dangling-flow";
pub const SELF_FLOW: &str = "sankey.self-flow";
pub const MISSING_VALUE: &str = "sankey.missing-value";
pub const NEGATIVE_VALUE: &str = "sankey.negative-value";
pub const UNKNOWN_COLOR: &str = "sankey.unknown-color";
pub const BACKWARD_FLOW: &str = "sankey.backward-flow";

/// One thing wrong with a document, with the zero-based line it was found on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SankeyBreach {
    pub rule_id: &'static str,
    pub message: String,
    pub line: usize,
    pub is_warning: bool,
}

fn breach(rule_id: &'static str, message: String, line: usize, is_warning: bool) -> SankeyBreach {
    SankeyBreach {
        rule_id,
        message,
        line,
        is_warning,
    }
}

/// An entry's id, what it is (`node` or `flow`) and the line it starts on.
type Entry<'a> = (String, &'a str, usize);

/// Everything wrong with a model already read, in the order of the lines it was found on.
pub fn validate(model: &SankeyModel) -> Vec<SankeyBreach> {
    let mut breaches: Vec<SankeyBreach> = model
        .problems
        .iter()
        .map(|problem| breach(UNREADABLE_ENTRY, problem.message.clone(), problem.line, true))
        .collect();
    breaches.extend(model.nodes.iter().filter(|node| node.id.is_empty()).map(|node| {
        breach(
            MISSING_ID,
            "A node has no id and is not drawn.".to_owned(),
            node.range.start,
            false,
        )
    }));

    let entries: Vec<Entry> = model
        .nodes
        .iter()
        .filter(|node| !node.id.is_empty())
        .map(|node| (node.id.clone(), "node", node.range.start))
        .chain(model.flows.iter().map(|flow| (flow.id(), "flow", flow.range.start)))
        .collect();
    // Grouped by id in the order each id first appears, every entry after a group's first reported.
    let mut groups: Vec<(&str, Vec<&Entry>)> = Vec::new();
    let mut group_of: HashMap<&str, usize> = HashMap::new();
    for entry in &entries {
        match group_of.get(entry.0.as_str()) {
            Some(at) => groups[*at].1.push(entry),
            None => {
                group_of.insert(&entry.0, groups.len());
                groups.push((&entry.0, vec![entry]));
            }
        }
    }
    for (_, group) in &groups {
        for (id, what, start) in group.iter().skip(1) {
            let message = if *what == "flow" && id.contains("->") {
                format!(
                    "A flow from `{}` is written twice; only the first is drawn - add their values, or give each an id.",
                    id.replace("->", "` to `")
                )
            } else {
                format!("The id `{id}` is used more than once; only its first entry is drawn.")
            };
            breaches.push(breach(DUPLICATE_ID, message, *start, false));
        }
    }

    let nodes: HashSet<&str> = model
        .nodes
        .iter()
        .map(|node| node.id.as_str())
        .filter(|id| !id.is_empty())
        .collect();
    for flow in &model.flows {
        let line = flow.range.start;
        if !flow.from.is_empty() && flow.from == flow.to {
            breaches.push(breach(
                SELF_FLOW,
                format!("The flow from `{}` runs to itself and is not drawn.", flow.from),
                line,
                false,
            ));
            continue;
        }
        let missing: Vec<&str> = [flow.from.as_str(), flow.to.as_str()]
            .into_iter()
            .filter(|end| !nodes.contains(end))
            .collect();
        for end in &missing {
            let message = if end.is_empty() {
                "A flow is missing an end and is not drawn.".to_owned()
            } else {
                format!("A flow names `{end}`, which is not a node here; it is not drawn.")
            };
            breaches.push(breach(DANGLING_FLOW, message, line, false));
        }
        if !missing.is_empty() {
            continue;
        }
        match flow.value {
            None => breaches.push(breach(
                MISSING_VALUE,
                format!(
                    "The flow from `{}` to `{}` states no value, so it is drawn as a hairline.",
                    flow.from, flow.to
                ),
                line,
                true,
            )),
            Some(value) if value < 0.0 => breaches.push(breach(
                NEGATIVE_VALUE,
                format!(
                    "The flow from `{}` to `{}` has a negative value; it is drawn as zero.",
                    flow.from, flow.to
                ),
                line,
                true,
            )),
            Some(_) => {}
        }
        if !format::is_known(&flow.color) {
            breaches.push(breach(UNKNOWN_COLOR, unknown_color(&flow.color), line, true));
        }
    }
    breaches.extend(
        model
            .nodes
            .iter()
            .filter(|node| !format::is_known(&node.color))
            .map(|node| breach(UNKNOWN_COLOR, unknown_color(&node.color), node.range.start, true)),
    );

    let layout = SankeyLayout::of(model);
    breaches.extend(
        layout
            .flows
            .iter()
            .filter(|flow| layout.bands[&flow.id()].backward)
            .map(|flow| {
                breach(
                    BACKWARD_FLOW,
                    format!(
                        "The flow from `{}` to `{}` runs back to a column at or before its own, so it loops round instead of reading left to right.",
                        flow.from, flow.to
                    ),
                    flow.range.start,
                    true,
                )
            }),
    );

    // A stable sort, as `OrderBy` is.
    breaches.sort_by_key(|breach| breach.line);
    breaches
}

fn unknown_color(color: &str) -> String {
    format!(
        "`{color}` is not a colour this diagram knows - one of {}, or #rrggbb; it is drawn as if it stated none.",
        PALETTE.join(", ")
    )
}

/// The breaches as the problems panel shows them, at their 1-based lines.
pub fn problems(model: &SankeyModel) -> Vec<DiagramProblem> {
    validate(model)
        .into_iter()
        .map(|breach| DiagramProblem {
            severity: if breach.is_warning {
                DiagramProblemSeverity::Warning
            } else {
                DiagramProblemSeverity::Error
            },
            message: breach.message,
            rule_id: breach.rule_id.to_owned(),
            location: Some(DiagramProblemLocation::Line(breach.line as u32 + 1)),
        })
        .collect()
}
