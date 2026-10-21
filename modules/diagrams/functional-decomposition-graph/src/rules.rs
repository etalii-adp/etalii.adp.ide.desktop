//! What a document gets wrong, as the Errors and Warnings panel lists it (etalii.adp spec 030, task T072).
//!
//! Ported from the standalone host's `src/diagrams/functional-decomposition-graph/backend/` at `8b6ccfc`:
//! `FdgRuleSet.cs` (Requirement 5.5: a document that breaks the rules still opens, and every breach is reported, each
//! checked on its own, in rule order then document order) and `FdgValidator.cs` (the parser's problems and the rule
//! set together; an unreadable entry is a warning, since its lines are kept and the document still draws, every other
//! breach an error, each located by its line counted from one).

use std::collections::HashMap;

use adp_diagram::{DiagramProblem, DiagramProblemLocation, DiagramProblemSeverity};

use crate::lines::{LineDocument, shortest};
use crate::model::{FdgElement, FdgModel, geometry};
use crate::parser;
use crate::relations;

/// The rule ids the design's table names.
pub mod rule_ids {
    pub const FORBIDDEN_LINK: &str = "fdg.forbidden-link";
    pub const SELF_LINK: &str = "fdg.self-link";
    pub const SECOND_PARENT: &str = "fdg.second-parent";
    pub const SECOND_SHOWS: &str = "fdg.second-shows";
    pub const OWNERSHIP_CYCLE: &str = "fdg.ownership-cycle";
    pub const DANGLING_REFERENCE: &str = "fdg.dangling-reference";
    pub const DUPLICATE_ID: &str = "fdg.duplicate-id";
    pub const UNREADABLE_ENTRY: &str = "fdg.unreadable-entry";
}

/// One breach, named by rule and by the elements involved, at its zero-based line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FdgBreach {
    pub rule_id: &'static str,
    pub message: String,
    pub elements: Vec<String>,
    pub line: usize,
}

fn breach(rule_id: &'static str, message: String, elements: Vec<String>, line: usize) -> FdgBreach {
    FdgBreach {
        rule_id,
        message,
        elements,
        line,
    }
}

/// Groups `items` by `key`, in the order each key is first seen, as LINQ's `GroupBy`.
fn group_by<'a, T>(items: impl IntoIterator<Item = &'a T>, key: impl Fn(&T) -> &str) -> Vec<(String, Vec<&'a T>)>
where
    T: 'a,
{
    let mut groups: Vec<(String, Vec<&T>)> = Vec::new();
    for item in items {
        let k = key(item);
        match groups.iter_mut().find(|(g, _)| g == k) {
            Some((_, members)) => members.push(item),
            None => groups.push((k.to_owned(), vec![item])),
        }
    }
    groups
}

/// Every breach in the model, in rule order then document order.
pub fn breaches(model: &FdgModel) -> Vec<FdgBreach> {
    let mut by_id: HashMap<&str, &FdgElement> = HashMap::new();
    for element in model.elements.iter().filter(|e| !e.id.is_empty()) {
        by_id.entry(element.id.as_str()).or_insert(element);
    }
    let mut all = duplicate_ids(model);
    all.extend(unreadable_entries(model));
    all.extend(links(model, &by_id));
    all.extend(cardinality(model));
    all.extend(cycles(model));
    all
}

fn duplicate_ids(model: &FdgModel) -> Vec<FdgBreach> {
    let ids: Vec<(&str, usize)> = model
        .elements
        .iter()
        .map(|e| (e.id.as_str(), e.range.start))
        .chain(model.connections.iter().map(|c| (c.id.as_str(), c.range.start)))
        .filter(|(id, _)| !id.is_empty())
        .collect();
    group_by(&ids, |(id, _)| id)
        .into_iter()
        .filter(|(_, group)| group.len() > 1)
        .map(|(id, group)| {
            breach(
                rule_ids::DUPLICATE_ID,
                format!("`{id}` is declared {} times; an id names one entry.", group.len()),
                vec![id.clone()],
                group.last().map(|(_, line)| *line).unwrap_or_default(),
            )
        })
        .collect()
}

/// What the parser passed over, and the sizes a document states below their minimum: one thing to an author.
fn unreadable_entries(model: &FdgModel) -> Vec<FdgBreach> {
    let mut found: Vec<FdgBreach> = model
        .problems
        .iter()
        .map(|p| breach(rule_ids::UNREADABLE_ENTRY, p.message.clone(), Vec::new(), p.line))
        .collect();
    for element in &model.elements {
        if element.width < geometry::MINIMUM_WIDTH {
            found.push(breach(
                rule_ids::UNREADABLE_ENTRY,
                format!(
                    "`{}` is {} wide; the minimum is {}.",
                    element.id,
                    shortest(element.width),
                    shortest(geometry::MINIMUM_WIDTH)
                ),
                vec![element.id.clone()],
                element.range.start,
            ));
        }
        if element.is_comment() && element.drawn_height() < geometry::MINIMUM_COMMENT_HEIGHT {
            found.push(breach(
                rule_ids::UNREADABLE_ENTRY,
                format!(
                    "`{}` is {} tall; a Comment's minimum is {}.",
                    element.id,
                    shortest(element.drawn_height()),
                    shortest(geometry::MINIMUM_COMMENT_HEIGHT)
                ),
                vec![element.id.clone()],
                element.range.start,
            ));
        }
    }
    found
}

fn links(model: &FdgModel, by_id: &HashMap<&str, &FdgElement>) -> Vec<FdgBreach> {
    let mut found = Vec::new();
    for connection in &model.connections {
        let (source, target) = (by_id.get(connection.from.as_str()), by_id.get(connection.to.as_str()));
        let (Some(source), Some(target)) = (source, target) else {
            let missing = if source.is_none() {
                &connection.from
            } else {
                &connection.to
            };
            found.push(breach(
                rule_ids::DANGLING_REFERENCE,
                format!(
                    "`{}` names `{missing}`, which is not an element in this document.",
                    connection.id
                ),
                vec![connection.id.clone(), missing.clone()],
                connection.range.start,
            ));
            continue;
        };
        if connection.from == connection.to {
            found.push(breach(
                rule_ids::SELF_LINK,
                format!(
                    "`{}` links `{}` to itself; no relation allows that.",
                    connection.id, connection.from
                ),
                vec![connection.from.clone()],
                connection.range.start,
            ));
            continue;
        }
        let Some(relation) = relations::by_id(&connection.connection_type) else {
            found.push(breach(
                rule_ids::FORBIDDEN_LINK,
                format!(
                    "`{}` names the relation `{}`, which this notation does not have.",
                    connection.id, connection.connection_type
                ),
                vec![connection.id.clone()],
                connection.range.start,
            ));
            continue;
        };
        if !relation.admits(&source.element_type, &target.element_type) {
            found.push(breach(
                rule_ids::FORBIDDEN_LINK,
                format!(
                    "`{}` does not admit a link from a {} to a {}.",
                    relation.id, source.element_type, target.element_type
                ),
                vec![connection.from.clone(), connection.to.clone()],
                connection.range.start,
            ));
        }
    }
    found
}

fn cardinality(model: &FdgModel) -> Vec<FdgBreach> {
    let mut found = Vec::new();
    for relation in &relations::ALL {
        let links: Vec<_> = model
            .connections
            .iter()
            .filter(|c| c.connection_type == relation.id)
            .collect();
        if let Some(into_target) = relation.max_into_target {
            for (to, group) in group_by(links.iter().copied(), |c| &c.to) {
                if to.is_empty() || group.len() <= into_target {
                    continue;
                }
                let mut elements = vec![to.clone()];
                elements.extend(group.iter().map(|c| c.from.clone()));
                found.push(breach(
                    rule_ids::SECOND_PARENT,
                    format!(
                        "`{to}` has {} `{}` parents; it may have {into_target}.",
                        group.len(),
                        relation.id
                    ),
                    elements,
                    group.last().map(|c| c.range.start).unwrap_or_default(),
                ));
            }
        }
        let Some(from_source) = relation.max_from_source else {
            continue;
        };
        for (from, group) in group_by(links.iter().copied(), |c| &c.from) {
            if from.is_empty() || group.len() <= from_source {
                continue;
            }
            let mut elements = vec![from.clone()];
            elements.extend(group.iter().map(|c| c.to.clone()));
            found.push(breach(
                rule_ids::SECOND_SHOWS,
                format!(
                    "`{from}` has {} `{}` links; it may have {from_source}.",
                    group.len(),
                    relation.id
                ),
                elements,
                group.last().map(|c| c.range.start).unwrap_or_default(),
            ));
        }
    }
    found
}

fn cycles(model: &FdgModel) -> Vec<FdgBreach> {
    relations::cycles_in(model)
        .into_iter()
        .map(|cycle| {
            let line = model.element_of(&cycle[0]).map(|e| e.range.start).unwrap_or_default();
            breach(
                rule_ids::OWNERSHIP_CYCLE,
                format!("Ownership loops: {} -> {}.", cycle.join(" -> "), cycle[0]),
                cycle,
                line,
            )
        })
        .collect()
}

/// Everything wrong with the document, parse problems and rule breaches together.
pub fn validate(document: &LineDocument) -> Vec<FdgBreach> {
    breaches(&parser::parse(document))
}

/// One breach as the panel reads it: an unreadable entry a warning, every other breach an error, at its line counted
/// from one.
pub fn to_problem(breach: &FdgBreach) -> DiagramProblem {
    DiagramProblem {
        severity: if breach.rule_id == rule_ids::UNREADABLE_ENTRY {
            DiagramProblemSeverity::Warning
        } else {
            DiagramProblemSeverity::Error
        },
        message: breach.message.clone(),
        rule_id: breach.rule_id.to_owned(),
        location: Some(DiagramProblemLocation::Line(breach.line as u32 + 1)),
    }
}

/// The problems of the document `text`, as the panel lists them (`FdgValidator.ValidateAsync`).
pub fn problems_of(text: &str) -> Vec<DiagramProblem> {
    validate(&LineDocument::parse(text)).iter().map(to_problem).collect()
}
