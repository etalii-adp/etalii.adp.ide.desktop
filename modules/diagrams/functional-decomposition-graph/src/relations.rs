//! The five relations' rules, the ownership cycles, and whether a link may be written (etalii.adp spec 030, task
//! T072).
//!
//! Ported from the standalone host's `src/diagrams/functional-decomposition-graph/backend/` at `8b6ccfc`:
//! `FdgRelations.cs` (Requirement 5's table, stated once: who may be a relation's source and target, and how many),
//! `FdgOwnership.cs` (cycles among the four ownership relations, Shows exempt, since a navigation loop is legal) and
//! `FdgConnectVerdict.cs` (the type, cardinality and cycle checks, in that order, each refusal naming the check), and
//! the client's `relationOnto`, which picks the one relation a drag from one element's body to another's draws.

use std::collections::HashSet;

use crate::model::{FdgElement, FdgModel, connection_types, element_types};

/// One relation's rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FdgRelation {
    /// The relation id, which is also a connection's `type` in the document.
    pub id: &'static str,
    /// The element types allowed as the source.
    pub sources: &'static [&'static str],
    /// The one element type allowed as the target.
    pub target: &'static str,
    /// At most this many of the relation may arrive at one target.
    pub max_into_target: Option<usize>,
    /// At most this many of the relation may leave one source.
    pub max_from_source: Option<usize>,
    /// Whether the relation is one of the four a cycle is made of.
    pub is_ownership: bool,
}

impl FdgRelation {
    /// Whether the relation admits a link from a `source_type` to a `target_type`. A Comment is in no relation's
    /// sources and is no relation's target, so every link to or from one is refused by construction.
    pub fn admits(&self, source_type: &str, target_type: &str) -> bool {
        self.sources.contains(&source_type) && self.target == target_type
    }
}

use element_types::{ACTION, DATA_ELEMENT, FUNCTION, UI_ELEMENT};

/// The five relations, in the order Requirement 5's table lists them.
pub const ALL: [FdgRelation; 5] = [
    FdgRelation {
        id: connection_types::UI_CHILD,
        sources: &[UI_ELEMENT],
        target: UI_ELEMENT,
        max_into_target: Some(1),
        max_from_source: None,
        is_ownership: true,
    },
    FdgRelation {
        id: connection_types::OWNS_ACTION,
        sources: &[UI_ELEMENT],
        target: ACTION,
        max_into_target: Some(1),
        max_from_source: None,
        is_ownership: true,
    },
    FdgRelation {
        id: connection_types::OWNS_DATA,
        sources: &[UI_ELEMENT, ACTION, DATA_ELEMENT],
        target: DATA_ELEMENT,
        max_into_target: Some(1),
        max_from_source: None,
        is_ownership: true,
    },
    FdgRelation {
        id: connection_types::OWNS_FUNCTION,
        sources: &[UI_ELEMENT, ACTION, DATA_ELEMENT, FUNCTION],
        target: FUNCTION,
        max_into_target: Some(1),
        max_from_source: None,
        is_ownership: true,
    },
    FdgRelation {
        id: connection_types::SHOWS,
        sources: &[ACTION],
        target: UI_ELEMENT,
        max_into_target: None,
        max_from_source: Some(1),
        is_ownership: false,
    },
];

/// What a drag between two elements no relation links answers.
pub const NOTHING_CONNECTS: &str = "Nothing in this diagram connects these two.";

/// The relation with `id`, or none when the document names one that does not exist.
pub fn by_id(id: &str) -> Option<&'static FdgRelation> {
    ALL.iter().find(|r| r.id == id)
}

/// The one relation a link from a `source_type` to a `target_type` can be, as the client's `relationOnto` picks it
/// for a drag from one element's body to another's: there is exactly one for every allowed pair, because each
/// relation is told apart by its target type and sources.
pub fn onto(source_type: &str, target_type: &str) -> Option<&'static FdgRelation> {
    let mut admitting = ALL.iter().filter(|r| r.admits(source_type, target_type));
    let first = admitting.next()?;
    admitting.next().is_none().then_some(first)
}

/// The ownership edges, source to its distinct targets in the order first seen, self-edges left out: a self-edge is
/// `fdg.self-link`'s to report, not an ownership loop.
fn ownership_edges(model: &FdgModel, require_ends: bool, distinct: bool) -> Vec<(String, Vec<String>)> {
    let mut edges: Vec<(String, Vec<String>)> = Vec::new();
    for connection in &model.connections {
        if !by_id(&connection.connection_type).is_some_and(|r| r.is_ownership) {
            continue;
        }
        if require_ends && (connection.from.is_empty() || connection.to.is_empty()) {
            continue;
        }
        if connection.from == connection.to {
            continue;
        }
        let at = match edges.iter().position(|(from, _)| *from == connection.from) {
            Some(at) => at,
            None => {
                edges.push((connection.from.clone(), Vec::new()));
                edges.len() - 1
            }
        };
        let targets = &mut edges[at].1;
        if !distinct || !targets.contains(&connection.to) {
            targets.push(connection.to.clone());
        }
    }
    edges
}

fn next<'a>(edges: &'a [(String, Vec<String>)], node: &str) -> Option<&'a [String]> {
    edges.iter().find(|(from, _)| from == node).map(|(_, to)| to.as_slice())
}

/// Every ownership cycle in the model, each as its members in order, rotated so its smallest id leads, and each
/// reported once however many of its members it was found from.
pub fn cycles_in(model: &FdgModel) -> Vec<Vec<String>> {
    let edges = ownership_edges(model, true, true);
    let mut walk = Walk {
        edges: &edges,
        visited: HashSet::new(),
        on_path: HashSet::new(),
        path: Vec::new(),
        found: Vec::new(),
    };
    for (start, _) in &edges {
        walk.walk(start);
    }
    walk.found.into_iter().map(|(_, cycle)| cycle).collect()
}

struct Walk<'a> {
    edges: &'a [(String, Vec<String>)],
    visited: HashSet<String>,
    on_path: HashSet<String>,
    path: Vec<String>,
    found: Vec<(String, Vec<String>)>,
}

impl Walk<'_> {
    fn walk(&mut self, node: &str) {
        if self.on_path.contains(node) {
            if let Some(at) = self.path.iter().position(|p| p == node) {
                let cycle = normalise(&self.path[at..]);
                let key = cycle.join("\u{0}");
                if !self.found.iter().any(|(k, _)| *k == key) {
                    self.found.push((key, cycle));
                }
            }
            return;
        }
        if !self.visited.insert(node.to_owned()) {
            return;
        }
        self.on_path.insert(node.to_owned());
        self.path.push(node.to_owned());
        let edges = self.edges;
        if let Some(children) = next(edges, node) {
            for child in children {
                self.walk(child);
            }
        }
        self.path.pop();
        self.on_path.remove(node);
    }
}

/// One cycle rotated so its smallest id, by ordinal comparison, comes first.
fn normalise(cycle: &[String]) -> Vec<String> {
    let mut lowest = 0;
    for (i, member) in cycle.iter().enumerate().skip(1) {
        if member.as_str() < cycle[lowest].as_str() {
            lowest = i;
        }
    }
    cycle[lowest..].iter().chain(&cycle[..lowest]).cloned().collect()
}

/// Whether an ownership link `from` → `to` would close a cycle: whether `from` is already reachable from `to` along
/// ownership links.
pub fn would_close(model: &FdgModel, from: &str, to: &str) -> bool {
    if from == to {
        return false;
    }
    let edges = ownership_edges(model, false, false);
    let mut seen: HashSet<String> = HashSet::from([to.to_owned()]);
    let mut pending = vec![to.to_owned()];
    while let Some(node) = pending.pop() {
        if node == from {
            return true;
        }
        if let Some(children) = next(&edges, &node) {
            for child in children {
                if seen.insert(child.clone()) {
                    pending.push(child.clone());
                }
            }
        }
    }
    false
}

fn name_of(element: &FdgElement) -> &str {
    if element.name.is_empty() {
        &element.id
    } else {
        &element.name
    }
}

/// The refusal for linking `from_id` to `to_id` by `relation_type`, or none when it may be written: the type check,
/// then the cardinality check, then the cycle check (Shows exempt), against the document as it is now.
pub fn refusal_for(model: &FdgModel, relation_type: &str, from_id: &str, to_id: &str) -> Option<String> {
    let Some(relation) = by_id(relation_type) else {
        return Some(format!("This notation has no `{relation_type}` relation."));
    };
    let (Some(from), Some(to)) = (model.element_of(from_id), model.element_of(to_id)) else {
        return Some("That link names an element that is no longer in this graph.".to_owned());
    };
    if from.id == to.id {
        return Some("Refused by the type check: no relation links an element to itself.".to_owned());
    }
    if !relation.admits(&from.element_type, &to.element_type) {
        return Some(format!(
            "Refused by the type check: `{}` does not link a {} to a {}.",
            relation.id, from.element_type, to.element_type
        ));
    }
    if let Some(into_target) = relation.max_into_target
        && model
            .connections
            .iter()
            .filter(|c| c.connection_type == relation.id && c.to == to.id)
            .count()
            >= into_target
    {
        return Some(format!(
            "Refused by the cardinality check: `{}` already has its `{}` parent, and may have {into_target}.",
            name_of(to),
            relation.id
        ));
    }
    if let Some(from_source) = relation.max_from_source
        && model
            .connections
            .iter()
            .filter(|c| c.connection_type == relation.id && c.from == from.id)
            .count()
            >= from_source
    {
        return Some(format!(
            "Refused by the cardinality check: `{}` already has its `{}` link, and may have {from_source}.",
            name_of(from),
            relation.id
        ));
    }
    if relation.is_ownership && would_close(model, &from.id, &to.id) {
        return Some(format!(
            "Refused by the cycle check: `{}` already owns `{}`, so this link would make an ownership loop.",
            name_of(to),
            name_of(from)
        ));
    }
    None
}
