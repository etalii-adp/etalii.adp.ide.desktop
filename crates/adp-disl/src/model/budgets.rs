//! The budgets of a diagram (DISL section 3.2.1; the standalone host's `DislBudgetReader` and `DislBudgets`): each
//! budget of `language.limits.budgets`, in declaration order, measured over the diagram; the candidates a truncating
//! `count` budget keeps, sorted by its `order`, ties in persistence order, whole `unit`s kept while they fit; and
//! whether it withholds edits.

use std::cmp::Ordering;
use std::collections::HashSet;

use adp_cel::Value;

use super::diagram::DislDiagram;
use super::element::DislElement;
use crate::derivation::env::DislEnv;
use crate::derivation::evaluation::{self, Vars};
use crate::expressions::contexts;
use crate::json::{self, Json};

/// The sentence a withheld edit is refused with when the budget words none.
const DEFAULT_REASON: &str = "The diagram shows only part of this model, so edits through it are withheld.";

/// The code of a gesture refused because a budget withholds it.
pub const WITHHELD: &str = "std.budgetWithheld";

/// The state of one budget over a diagram, as `budget(id)` reads it in CEL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DislBudgetState {
    /// The budget's id.
    pub id: String,
    /// How many candidates are drawn: the kept ones of a truncating budget, every one otherwise.
    pub shown: i64,
    /// The measure: how many candidates a `count` budget has, or what its `cel` gives.
    pub total: i64,
    /// Whether the measure is greater than the budget's `max`.
    pub exceeded: bool,
    /// Whether the budget cuts what is drawn: it truncates and is exceeded.
    pub truncated: bool,
    /// Whether edits are withheld: it is exceeded and withholds something.
    pub withheld: bool,
    /// The candidates it leaves undrawn, by their keys; empty when it does not truncate.
    pub(crate) cut: HashSet<usize>,
}

impl DislBudgetState {
    /// Whether the budget leaves `element` undrawn.
    pub fn cuts(&self, element: &DislElement) -> bool {
        self.cut.contains(&element.key())
    }
}

/// The budgets of the specification, as (id, declaration, pointer), in declaration order.
pub(crate) fn declared(diagram: &DislDiagram) -> Vec<(String, Json, String)> {
    let specification = diagram.specification();
    let limits = json::member(json::member(specification.root(), "language"), "limits");
    match json::get(limits, "budgets").and_then(Json::as_object) {
        Some(budgets) => budgets
            .iter()
            .filter(|(_, budget)| budget.is_object())
            .map(|(id, budget)| {
                (
                    id.clone(),
                    budget.clone(),
                    json::pointer("/language/limits/budgets", id),
                )
            })
            .collect(),
        None => Vec::new(),
    }
}

/// The state of every budget of `diagram`, in declaration order.
pub(crate) fn compute(diagram: &DislDiagram) -> Vec<DislBudgetState> {
    declared(diagram)
        .into_iter()
        .map(|(id, budget, pointer)| state(diagram, id, &budget, &pointer))
        .collect()
}

fn withholds(budget: &Json) -> bool {
    edits_of(budget) != "none"
}

/// What a budget withholds while exceeded: `model`, `all` or `none`.
fn edits_of(budget: &Json) -> &str {
    json::string_of(json::member(budget, "withhold"), "edits").unwrap_or("model")
}

fn state(diagram: &DislDiagram, id: String, budget: &Json, pointer: &str) -> DislBudgetState {
    let specification = diagram.specification();
    let measure = json::member(budget, "measure");
    let max = json::get(budget, "max").and_then(Json::as_i64).unwrap_or(i64::MAX);
    if json::get(measure, "cel").is_some() {
        // A measure that fails, or is not a number, measures nothing: the budget is not exceeded.
        let total = match evaluation::expression(
            &specification,
            measure,
            &json::pointer(pointer, "measure"),
            contexts::BUDGET,
            &variables(diagram, None, 0),
        ) {
            Ok(Value::Int(count)) => count,
            Ok(Value::Double(count)) => count as i64,
            _ => 0,
        };
        let exceeded = total > max;
        return DislBudgetState {
            id,
            shown: total,
            total,
            exceeded,
            truncated: false,
            withheld: exceeded && withholds(budget),
            cut: HashSet::new(),
        };
    }

    let types: Vec<&str> = json::items(measure, "count")
        .into_iter()
        .flatten()
        .filter_map(Json::as_str)
        .collect();
    let candidates: Vec<DislElement> = diagram
        .elements()
        .into_iter()
        .filter(|element| types.iter().any(|t| element.is_a(t)))
        .collect();
    let total = candidates.len() as i64;
    let over = total > max;
    let truncate = json::get(budget, "truncate").and_then(Json::as_bool).unwrap_or(true);
    if !truncate || !over {
        return DislBudgetState {
            id,
            shown: total,
            total,
            exceeded: over,
            truncated: false,
            withheld: over && withholds(budget),
            cut: HashSet::new(),
        };
    }

    let mut sorted: Vec<(usize, &DislElement)> = candidates.iter().enumerate().collect();
    if let Some(order) = json::get(budget, "order") {
        let keys: Vec<Option<Value>> = candidates
            .iter()
            .enumerate()
            .map(|(index, candidate)| {
                evaluation::expression(
                    &specification,
                    order,
                    &json::pointer(pointer, "order"),
                    contexts::BUDGET,
                    &variables(diagram, Some(candidate), index),
                )
                .ok()
            })
            .collect();
        // A stable sort keeps persistence order among equal keys.
        sorted.sort_by(|(a, _), (b, _)| key_order(keys[*a].as_ref(), keys[*b].as_ref()));
    }

    let mut kept: HashSet<usize> = HashSet::new();
    for unit in units(diagram, budget, pointer, &sorted) {
        if kept.len() as i64 + unit.len() as i64 > max {
            break;
        }
        kept.extend(unit);
    }
    let cut: HashSet<usize> = candidates
        .iter()
        .map(DislElement::key)
        .filter(|key| !kept.contains(key))
        .collect();
    DislBudgetState {
        id,
        shown: kept.len() as i64,
        total,
        exceeded: true,
        truncated: true,
        withheld: withholds(budget),
        cut,
    }
}

/// The units of the sorted candidates, as their keys, in the order of each one's first candidate; each candidate its
/// own without a `unit`.
fn units(diagram: &DislDiagram, budget: &Json, pointer: &str, sorted: &[(usize, &DislElement)]) -> Vec<Vec<usize>> {
    let Some(unit) = json::get(budget, "unit") else {
        return sorted.iter().map(|(_, candidate)| vec![candidate.key()]).collect();
    };
    let specification = diagram.specification();
    let mut units: Vec<(String, Vec<usize>)> = Vec::new();
    for (index, candidate) in sorted {
        let key = evaluation::expression(
            &specification,
            unit,
            &json::pointer(pointer, "unit"),
            contexts::BUDGET,
            &variables(diagram, Some(candidate), *index),
        )
        .map(|value| evaluation::text_of(&value))
        .unwrap_or_default();
        match units.iter_mut().find(|(known, _)| *known == key) {
            Some((_, members)) => members.push(candidate.key()),
            None => units.push((key, vec![candidate.key()])),
        }
    }
    units.into_iter().map(|(_, members)| members).collect()
}

fn variables(diagram: &DislDiagram, candidate: Option<&DislElement>, index: usize) -> Vars {
    evaluation::vars([
        ("self", candidate.map_or(Value::Null, DislElement::to_value)),
        ("index", Value::Int(index as i64)),
        ("diagram", diagram.to_value()),
        ("env", Value::map(DislEnv::default().to_cel())),
    ])
}

/// Sort keys compared as CEL orders them: a list element by element, a shorter list first when it is a prefix; a key
/// that cannot be compared (a failed one among them) after every key that can.
fn key_order(a: Option<&Value>, b: Option<&Value>) -> Ordering {
    match (a, b) {
        (Some(Value::List(left)), Some(Value::List(right))) => {
            for (x, y) in left.iter().zip(right.iter()) {
                let order = key_order(Some(x), Some(y));
                if order != Ordering::Equal {
                    return order;
                }
            }
            left.len().cmp(&right.len())
        }
        _ => {
            let comparable = |v: Option<&Value>| {
                matches!(
                    v,
                    Some(Value::String(_) | Value::Int(_) | Value::Double(_) | Value::Bool(_))
                )
            };
            match (comparable(a), comparable(b)) {
                (true, true) => {}
                (left, right) => return right.cmp(&left),
            }
            let (x, y) = (a.expect("comparable"), b.expect("comparable"));
            let rank = |v: &Value| match v {
                Value::Bool(_) => 0,
                Value::String(_) => 2,
                _ => 1,
            };
            if rank(x) != rank(y) {
                return rank(x).cmp(&rank(y));
            }
            adp_cel::compare(x, y).unwrap_or(Ordering::Equal)
        }
    }
}

/// The reason an edit is withheld (section 3.2.1): that of the first exceeded budget that withholds it, every budget
/// withholding `model` or `all` withholding a change to the model and only `all` one to stored view data; none when
/// no budget does.
pub fn withheld_reason(diagram: &DislDiagram, view_data: bool) -> Option<String> {
    let states = diagram.budgets();
    for ((_, budget, pointer), state) in declared(diagram).iter().zip(states.iter()) {
        if !state.withheld || (view_data && edits_of(budget) != "all") {
            continue;
        }
        let reason = json::get(json::member(budget, "withhold"), "reason").and_then(|reason| {
            let variables = evaluation::vars([
                ("self", diagram.to_value()),
                ("diagram", diagram.to_value()),
                ("env", Value::map(DislEnv::default().to_cel())),
            ]);
            let at = json::pointer(&json::pointer(pointer, "withhold"), "reason");
            evaluation::one_reason(&diagram.specification(), reason, &at, contexts::ELEMENT, &variables)
        });
        return Some(
            reason
                .filter(|text| !text.is_empty())
                .unwrap_or_else(|| DEFAULT_REASON.to_owned()),
        );
    }
    None
}

/// What an operation changes (section 3.2.1): `model` when it runs a plugin or any action that changes the model,
/// `view` when it only lays out or writes view data, and none when it changes neither.
pub(crate) fn changes(operation: &Json) -> Option<&'static str> {
    if json::get(operation, "plugin").is_some() {
        return Some("model");
    }
    actions_change(json::member(operation, "actions"))
}

fn actions_change(actions: &Json) -> Option<&'static str> {
    let mut changes = None;
    for action in actions.as_array().into_iter().flatten() {
        let kind = if ["let", "select", "abort"]
            .iter()
            .any(|k| json::get(action, k).is_some())
        {
            None
        } else if ["layout", "view"].iter().any(|k| json::get(action, k).is_some()) {
            Some("view")
        } else if json::get(action, "if").is_some() {
            let then = actions_change(json::member(action, "then"));
            let otherwise = actions_change(json::member(action, "else"));
            if then == Some("model") || otherwise == Some("model") {
                Some("model")
            } else {
                then.or(otherwise)
            }
        } else if json::get(action, "forEach").is_some() {
            actions_change(json::member(action, "do"))
        } else {
            Some("model")
        };
        if kind == Some("model") {
            return kind;
        }
        changes = changes.or(kind);
    }
    changes
}
