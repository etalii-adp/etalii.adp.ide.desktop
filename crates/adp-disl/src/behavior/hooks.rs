//! Runs a definition's `after` hooks (DISL section 9.2; the standalone host's `HookRunner`) for a change of an
//! element's attributes - every hook `on: change` whose `for` takes the element and whose `attribute` list meets the
//! changed attributes, with `self` the element after the change and `old` a snapshot before it - and for a relation
//! drawn: every hook `on: connect` whose `for` takes the relation, with `self` the new relation and `old` null.
//! Hooks run in `order` and then declaration order.
//!
//! The host has made the change on the working diagram already and hands in the snapshot it took before; the hooks'
//! own changes are made on it as they run. A hook's change can run further hooks, each at most once per element in
//! the transaction, and no more than `behavior.maxHookDepth` (100) runs in all; past that the transaction is
//! refused. Only `change` and `connect` events, and only `after` hooks, are run here.

use std::collections::{HashMap, HashSet};

use adp_cel::{Map, Value};

use super::actions::ActionRunner;
use super::change::{DislChange, DislTransaction};
use super::ids::IdSource;
use super::operations::variables_of;
use crate::derivation::env::DislEnv;
use crate::derivation::evaluation;
use crate::expressions::contexts;
use crate::json::{self, Json};
use crate::loading::specification::DislSpecification;
use crate::model::element::DislElement;

/// The hooks' changes after `attributes` of `element` changed, from what `old` held: the snapshot taken with
/// [`DislElement::snapshot`] before the change.
pub fn after_change(
    specification: &DislSpecification,
    element: &DislElement,
    old: &DislElement,
    attributes: &[&str],
    ids: &dyn IdSource,
    env: Option<&DislEnv>,
    source: &str,
) -> DislTransaction {
    let changed: Vec<String> = attributes.iter().map(|a| (*a).to_owned()).collect();
    run(specification, element, Some(old), "change", &changed, ids, env, source)
}

/// The hooks' changes after `relation` was drawn (an `on: connect` event): the host has added it to the working
/// diagram, ends and attributes, before calling this.
pub fn after_connect(
    specification: &DislSpecification,
    relation: &DislElement,
    ids: &dyn IdSource,
    env: Option<&DislEnv>,
    source: &str,
) -> Result<DislTransaction, String> {
    if !relation.is_relation() {
        return Err(format!("{relation} is no relation, so it was not drawn."));
    }
    Ok(run(specification, relation, None, "connect", &[], ids, env, source))
}

#[allow(clippy::too_many_arguments)]
fn run(
    specification: &DislSpecification,
    element: &DislElement,
    old: Option<&DislElement>,
    kind: &str,
    attributes: &[String],
    ids: &dyn IdSource,
    env: Option<&DislEnv>,
    source: &str,
) -> DislTransaction {
    let diagram = element.diagram();
    let mut hooks = Hooks {
        specification,
        runner: ActionRunner::new(specification, &diagram, ids, contexts::HOOK),
        ran: HashSet::new(),
        budget: max_depth(specification),
        env,
        source,
    };
    hooks.ran_for(element, old, kind, attributes);
    hooks.runner.transaction()
}

struct Hooks<'a> {
    specification: &'a DislSpecification,
    runner: ActionRunner<'a>,
    ran: HashSet<(String, usize)>,
    budget: i64,
    env: Option<&'a DislEnv>,
    source: &'a str,
}

impl Hooks<'_> {
    fn ran_for(&mut self, element: &DislElement, before: Option<&DislElement>, happened: &str, changed: &[String]) -> bool {
        for (hook, pointer) in hooks(self.specification, element, happened, changed) {
            let id = json::string_of(hook, "id").map(str::to_owned).unwrap_or_else(|| pointer.clone());
            if !self.ran.insert((id, element.key())) {
                continue;
            }
            self.budget -= 1;
            if self.budget < 0 {
                return self
                    .runner
                    .refuse("The change set off more hooks than behavior.maxHookDepth allows.");
            }

            let mut variables = variables_of(contexts::HOOK, &element.diagram(), self.env);
            variables.insert("self".into(), element.to_value());
            variables.insert("old".into(), before.map(DislElement::to_value).unwrap_or(Value::Null));
            variables.insert("event".into(), Value::map(event(element, before, happened, changed, self.source)));
            if let Some(when) = json::get(hook, "when")
                && !evaluation::holds(
                    self.specification,
                    when,
                    &json::pointer(&pointer, "when"),
                    contexts::HOOK,
                    &variables,
                )
            {
                continue;
            }

            let first = self.runner.changes.len();
            self.runner.before = Some(HashMap::new());
            if let Some(actions) = json::get(hook, "actions")
                && !self.runner.run(actions, &json::pointer(&pointer, "actions"), &mut variables)
            {
                return false;
            }
            let snapshots = self.runner.before.take().unwrap_or_default();

            let sets: Vec<(String, Vec<String>)> = self.runner.changes[first..]
                .iter()
                .filter_map(|change| match change {
                    DislChange::Set {
                        element_id, attributes, ..
                    } => Some((element_id.clone(), attributes.keys().cloned().collect())),
                    _ => None,
                })
                .collect();
            for (element_id, attributes) in sets {
                if let Some(target) = element.diagram().elements().into_iter().find(|e| e.id() == element_id) {
                    let snapshot = snapshots.get(&target.key()).cloned().unwrap_or_else(|| target.snapshot());
                    if !self.ran_for(&target, Some(&snapshot), "change", &attributes) {
                        return false;
                    }
                }
            }
        }
        true
    }
}

fn hooks<'a>(
    specification: &'a DislSpecification,
    element: &DislElement,
    kind: &str,
    changed: &[String],
) -> Vec<(&'a Json, String)> {
    let Some(list) = json::items(json::member(specification.root(), "behavior"), "hooks") else {
        return Vec::new();
    };
    let mut declared: Vec<(&Json, String, i64, usize)> = Vec::new();
    for (index, hook) in list.iter().enumerate() {
        let pointer = json::pointer_at("/behavior/hooks", index);
        if !json::strings_of(hook, "on").iter().any(|o| o == kind) {
            continue;
        }
        if json::string_of(hook, "phase").unwrap_or("after") != "after" {
            continue;
        }
        let types = json::strings_of(hook, "for");
        if !types.is_empty() && !types.iter().any(|t| element.is_a(t)) {
            continue;
        }
        let watched = json::strings_of(hook, "attribute");
        if kind == "change" && !watched.is_empty() && !watched.iter().any(|w| changed.contains(w)) {
            continue;
        }
        let order = json::int_of(hook, "order").unwrap_or(0);
        declared.push((hook, pointer, order, index));
    }
    declared.sort_by_key(|(_, _, order, index)| (*order, *index));
    declared.into_iter().map(|(hook, pointer, _, _)| (hook, pointer)).collect()
}

fn event(element: &DislElement, before: Option<&DislElement>, kind: &str, changed: &[String], source: &str) -> Map {
    let attribute = changed.first();
    let mut event = Map::default();
    event.insert("kind", Value::from(kind));
    event.insert("attribute", attribute.map(|a| Value::String(a.clone())).unwrap_or(Value::Null));
    event.insert(
        "oldValue",
        match (attribute, before) {
            (Some(a), Some(before)) if before.element_type().attributes.contains_key(a) => before.value_of(a),
            _ => Value::Null,
        },
    );
    event.insert(
        "newValue",
        match attribute {
            Some(a) if element.element_type().attributes.contains_key(a) => element.value_of(a),
            _ => Value::Null,
        },
    );
    event.insert("source", Value::from(source));
    event
}

fn max_depth(specification: &DislSpecification) -> i64 {
    json::get(json::member(specification.root(), "behavior"), "maxHookDepth")
        .and_then(Json::as_i64)
        .filter(|d| i32::try_from(*d).is_ok())
        .unwrap_or(100)
}
