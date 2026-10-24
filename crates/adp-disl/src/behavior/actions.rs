//! Runs one action list (DISL section 9.4; the standalone host's `ActionRunner`) against a working diagram: in
//! order, each action's expressions seeing every earlier action's effect, every action skipped while its `when` does
//! not hold.
//!
//! The diagram it is given is the transaction's working state, and it is changed as the actions run: a `set` stores
//! its values, a `create` adds its node, a `delete` takes the element out, a `reparent` moves the node with
//! everything beneath it. Model actions become [`DislChange`]s: `set`, `unset`, `create`, `connect`, `retype`,
//! `delete` and `reparent`; `let`, `if` and `forEach` steer the run; `layout`, `plugin`, `select` and `editLabel` are
//! handed back as [`HostAction`]s; `abort` refuses the transaction with its message. Any other action, and an
//! expression that fails, refuses it too: a runtime that cannot do what a definition says does not do part of it.

use std::collections::HashMap;

use adp_cel::{Map, Value};

use super::change::{DislChange, DislTransaction, HostAction};
use super::deletion;
use super::extension::DislActions;
use super::ids::IdSource;
use super::retype;
use crate::derivation::evaluation::{self, Vars};
use crate::json::{self, Json};
use crate::loading::specification::DislSpecification;
use crate::model::diagram::DislDiagram;
use crate::model::element::DislElement;
use crate::model::values;

const KINDS: [&str; 25] = [
    "set",
    "unset",
    "create",
    "connect",
    "delete",
    "move",
    "resize",
    "retype",
    "reparent",
    "reorder",
    "view",
    "let",
    "if",
    "forEach",
    "select",
    "reveal",
    "highlight",
    "editLabel",
    "openForm",
    "notify",
    "layout",
    "abort",
    "call",
    "plugin",
    "extension",
];

pub(crate) struct ActionRunner<'a> {
    specification: &'a DislSpecification,
    diagram: &'a DislDiagram,
    ids: &'a dyn IdSource,
    context: &'static str,
    pub(crate) changes: Vec<DislChange>,
    host: Vec<HostAction>,
    /// When set, each element as it was before the first `set` or `unset` on it, which a change's further hooks
    /// read as `old`.
    pub(crate) before: Option<HashMap<usize, DislElement>>,
    refusal: Option<String>,
}

/// A value as an element stores it: an element by its id, a list item by item.
fn stored(value: Value) -> Value {
    if let Some(element) = DislElement::from_value(&value) {
        return Value::String(element.id());
    }
    match value {
        Value::List(list) => Value::list(list.iter().cloned().map(stored).collect()),
        other => other,
    }
}

fn elements(value: &Value) -> Vec<DislElement> {
    match value {
        Value::List(list) => list.iter().filter_map(DislElement::from_value).collect(),
        other => DislElement::from_value(other).into_iter().collect(),
    }
}

impl<'a> ActionRunner<'a> {
    pub(crate) fn new(
        specification: &'a DislSpecification,
        diagram: &'a DislDiagram,
        ids: &'a dyn IdSource,
        context: &'static str,
    ) -> Self {
        ActionRunner {
            specification,
            diagram,
            ids,
            context,
            changes: Vec::new(),
            host: Vec::new(),
            before: None,
            refusal: None,
        }
    }

    /// The transaction so far.
    pub(crate) fn transaction(&self) -> DislTransaction {
        match &self.refusal {
            Some(refusal) => DislTransaction::refused(refusal.clone()),
            None => DislTransaction {
                changes: self.changes.clone(),
                host_actions: self.host.clone(),
                refusal: None,
            },
        }
    }

    pub(crate) fn specification(&self) -> &DislSpecification {
        self.specification
    }

    pub(crate) fn diagram(&self) -> &DislDiagram {
        self.diagram
    }

    /// Why the transaction is refused, once it is.
    pub(crate) fn refusal(&self) -> Option<String> {
        self.refusal.clone()
    }

    pub(crate) fn refuse(&mut self, reason: impl Into<String>) -> bool {
        if self.refusal.is_none() {
            self.refusal = Some(reason.into());
        }
        false
    }

    /// Runs `actions`, found at `pointer`; false once the transaction is refused.
    pub(crate) fn run(&mut self, actions: &Json, pointer: &str, variables: &mut Vars) -> bool {
        let Some(list) = actions.as_array() else {
            return true;
        };
        for (index, action) in list.iter().enumerate() {
            let at = json::pointer_at(pointer, index);
            if !action.is_object() {
                return self.refuse(format!("The action at {at} is not an object."));
            }
            if let Some(when) = json::get(action, "when")
                && !evaluation::holds(
                    self.specification,
                    when,
                    &json::pointer(&at, "when"),
                    self.context,
                    variables,
                )
            {
                continue;
            }
            if !self.action(action, &at, variables) {
                return false;
            }
        }
        true
    }

    fn action(&mut self, action: &Json, at: &str, variables: &mut Vars) -> bool {
        let Some(kind) = KINDS.iter().copied().find(|k| json::has(action, k)) else {
            return self.refuse(format!("The action at {at} is none this runtime knows."));
        };
        let body = json::member(action, kind);
        let body_at = json::pointer(at, kind);
        match kind {
            "set" => {
                let Some(target) = self.target(action, at, variables) else {
                    return false;
                };
                let mut values = Map::default();
                if let Some(members) = body.as_object() {
                    for (name, value) in members {
                        let Some(value) = self.value(value, &json::pointer(&body_at, name), variables) else {
                            return false;
                        };
                        values.insert(name.clone(), stored(value));
                    }
                }
                self.set(&target, values)
            }
            "unset" => {
                let Some(target) = self.target(action, at, variables) else {
                    return false;
                };
                let values = json::strings(body).into_iter().map(|n| (n, Value::Null)).collect();
                self.set(&target, values)
            }
            "create" => self.create(action, body, &body_at, variables),
            "connect" => self.connect(action, body, &body_at, variables),
            "retype" => self.retype(body, &body_at, variables),
            "delete" => {
                let Some(value) = self.value(body, &body_at, variables) else {
                    return false;
                };
                for element in elements(&value) {
                    let deletion = deletion::changes(self.specification, &element, false);
                    if let Some(refusal) = deletion.refusal {
                        return self.refuse(refusal);
                    }
                    for change in deletion.changes {
                        if let DislChange::Remove { element_id } = &change
                            && let Some(gone) = self.diagram.elements().into_iter().find(|e| e.id() == *element_id)
                        {
                            self.diagram.remove(&gone);
                        }
                        self.changes.push(change);
                    }
                }
                true
            }
            "reparent" => {
                let self_value = variables.get("self").cloned().unwrap_or(Value::Null);
                let Some(target) = self.member(body, &body_at, "target", variables, self_value) else {
                    return false;
                };
                let Some(parent) = self.member(body, &body_at, "parent", variables, Value::Null) else {
                    return false;
                };
                let Some(slot) = self.member(body, &body_at, "slot", variables, Value::Null) else {
                    return false;
                };
                let Some(after) = self.member(body, &body_at, "after", variables, Value::Null) else {
                    return false;
                };
                let Some(before) = self.member(body, &body_at, "before", variables, Value::Null) else {
                    return false;
                };
                let Some(element) = DislElement::from_value(&target) else {
                    return self.refuse(format!("The reparent at {at} has no element to move."));
                };
                let new_parent = DislElement::from_value(&parent);
                let siblings = self.diagram.siblings_under(new_parent.as_ref());
                let (after, before) = (DislElement::from_value(&after), DislElement::from_value(&before));
                let mut index: i64 = -1;
                if let Some(previous) = &after
                    && let Some(found) = siblings.iter().position(|s| s == previous)
                {
                    index = found as i64 + 1;
                } else if let Some(next) = &before {
                    index = siblings.iter().position(|s| s == next).map(|p| p as i64).unwrap_or(-1);
                }
                if index < 0 && (after.is_some() || before.is_some()) {
                    return self.refuse(format!(
                        "The reparent at {at} places the element beside one that is not beneath its new parent."
                    ));
                }
                if let Err(e) = self
                    .diagram
                    .reparent(&element, new_parent.as_ref(), slot.as_str(), index)
                {
                    return self.refuse(e);
                }
                self.changes.push(DislChange::Reparent {
                    element_id: element.id(),
                    parent_id: new_parent.map(|p| p.id()),
                    index,
                });
                true
            }
            "let" => {
                if let Some(members) = body.as_object() {
                    for (name, value) in members {
                        let Some(value) = self.value(value, &json::pointer(&body_at, name), variables) else {
                            return false;
                        };
                        variables.insert(name.clone(), value);
                    }
                }
                true
            }
            "if" => {
                if evaluation::holds(self.specification, body, &body_at, self.context, variables) {
                    match json::get(action, "then") {
                        Some(then) => self.run(then, &json::pointer(at, "then"), variables),
                        None => true,
                    }
                } else {
                    match json::get(action, "else") {
                        Some(otherwise) => self.run(otherwise, &json::pointer(at, "else"), variables),
                        None => true,
                    }
                }
            }
            "forEach" => {
                let Some(list) = self.value(body, &body_at, variables) else {
                    return false;
                };
                let Value::List(items) = list else {
                    return self.refuse(format!("The forEach at {at} is not over a list."));
                };
                let name = json::string_of(action, "as").unwrap_or("item").to_owned();
                for item in items.iter() {
                    variables.insert(name.clone(), item.clone());
                    if let Some(steps) = json::get(action, "do")
                        && !self.run(steps, &json::pointer(at, "do"), variables)
                    {
                        return false;
                    }
                }
                true
            }
            "select" => {
                let Some(value) = self.value(body, &body_at, variables) else {
                    return false;
                };
                self.host.push(HostAction::Select {
                    element_ids: elements(&value).iter().map(DislElement::id).collect(),
                });
                true
            }
            "editLabel" => {
                let self_value = variables.get("self").cloned().unwrap_or(Value::Null);
                let Some(target) = self.member(body, &body_at, "target", variables, self_value) else {
                    return false;
                };
                let Some(element) = DislElement::from_value(&target) else {
                    return self.refuse(format!("The editLabel at {at} has no element to edit."));
                };
                self.host.push(HostAction::EditLabel {
                    element_id: element.id(),
                    label: json::string_of(body, "label").map(str::to_owned),
                });
                true
            }
            "view" => {
                let self_value = variables.get("self").cloned().unwrap_or(Value::Null);
                let target = match json::get(action, "target") {
                    Some(target) => match self.value(target, &json::pointer(at, "target"), variables) {
                        Some(value) => value,
                        None => return false,
                    },
                    None => self_value,
                };
                let Some(element) = DislElement::from_value(&target) else {
                    return self.refuse(format!("The view at {at} has no element to view."));
                };
                let mut values = Map::default();
                if let Some(members) = body.as_object() {
                    for (name, value) in members {
                        let Some(value) = self.value(value, &json::pointer(&body_at, name), variables) else {
                            return false;
                        };
                        values.insert(name.clone(), value);
                    }
                }
                self.host.push(HostAction::View {
                    element_id: element.id(),
                    values,
                });
                true
            }
            "layout" => {
                self.host.push(HostAction::Layout {
                    algorithm: json::string_of(body, "algorithm").unwrap_or("").to_owned(),
                });
                true
            }
            "plugin" => {
                let mut arguments = Map::default();
                for (name, argument) in json::members(action, "args") {
                    let Some(value) = self.value(argument, &json::pointer(&json::pointer(at, "args"), name), variables)
                    else {
                        return false;
                    };
                    arguments.insert(name.clone(), value);
                }
                self.host.push(HostAction::Plugin {
                    name: body.as_str().unwrap_or("").to_owned(),
                    arguments,
                });
                true
            }
            "abort" => {
                let Some(message) = self.member(
                    body,
                    &body_at,
                    "message",
                    variables,
                    Value::from("The change was refused."),
                ) else {
                    return false;
                };
                self.refuse(evaluation::text_of(&message))
            }
            "extension" => {
                let name = body.as_str().unwrap_or_default().to_owned();
                let Some(extension) = self.specification.action_extension() else {
                    return self.refuse(format!("This runtime cannot run the extension `{name}` ({at})."));
                };
                let result = extension.run(&name, &mut DislActions::new(self, variables, at.to_owned()));
                match result {
                    Ok(()) => self.refusal.is_none(),
                    Err(refusal) => self.refuse(refusal),
                }
            }
            _ => self.refuse(format!("This runtime cannot run a `{kind}` action yet ({at}).")),
        }
    }

    fn attributes(&mut self, body: &Json, at: &str, variables: &Vars) -> Option<Map> {
        let mut attributes = Map::default();
        for (name, value) in json::members(body, "attributes") {
            let value = self.value(value, &json::pointer(&json::pointer(at, "attributes"), name), variables)?;
            if !value.is_null() {
                attributes.insert(name.clone(), stored(value));
            }
        }
        Some(attributes)
    }

    fn create(&mut self, action: &Json, body: &Json, at: &str, variables: &mut Vars) -> bool {
        let Some(type_value) = self.member(body, at, "type", variables, Value::Null) else {
            return false;
        };
        let type_name = match type_value
            .as_str()
            .and_then(|t| self.specification.metamodel().type_of(t))
        {
            Some(t) if !t.is_relation && !t.is_abstract() => t.name.clone(),
            _ => return self.refuse(format!("The create at {at} names no node type it can create.")),
        };
        let Some(parent) = self.member(body, at, "parent", variables, Value::Null) else {
            return false;
        };
        let Some(slot) = self.member(body, at, "slot", variables, Value::Null) else {
            return false;
        };
        let Some(point) = self.member(body, at, "at", variables, Value::Null) else {
            return false;
        };
        let Some(attributes) = self.attributes(body, at, variables) else {
            return false;
        };
        let Some(after) = self.member(body, at, "after", variables, Value::Null) else {
            return false;
        };
        let Some(before) = self.member(body, at, "before", variables, Value::Null) else {
            return false;
        };
        let (after, before) = (DislElement::from_value(&after), DislElement::from_value(&before));
        // Placed beside a sibling (section 9.4), the new node goes under the sibling's parent unless one is given.
        let parent =
            DislElement::from_value(&parent).or_else(|| after.as_ref().or(before.as_ref()).and_then(|s| s.parent()));
        let siblings = self.diagram.siblings_under(parent.as_ref());
        let index = match (&after, &before) {
            (Some(after), _) => match siblings.iter().position(|s| s == after) {
                Some(found) => Some(found as i64 + 1),
                None => {
                    return self.refuse(format!(
                        "The create at {at} is placed after a node that is not its sibling."
                    ));
                }
            },
            (None, Some(before)) => match siblings.iter().position(|s| s == before) {
                Some(found) => Some(found as i64),
                None => {
                    return self.refuse(format!(
                        "The create at {at} is placed before a node that is not its sibling."
                    ));
                }
            },
            (None, None) => None,
        };
        let created = self.ids.next(&type_name).and_then(|id| match &parent {
            Some(parent) => self
                .diagram
                .add_child(&type_name, &id, attributes.clone(), parent, slot.as_str()),
            None => self.diagram.add_node(&type_name, &id, attributes.clone()),
        });
        let created = match created {
            Ok(created) => created,
            Err(e) => return self.refuse(e),
        };
        // Added last; moved to its place among the siblings when it has one before the end.
        if let Some(index) = index
            && (index as usize) < siblings.len()
            && let Err(e) = self.diagram.reparent(&created, parent.as_ref(), slot.as_str(), index)
        {
            return self.refuse(e);
        }
        self.changes.push(DislChange::Create {
            type_name,
            id: created.id(),
            attributes,
            parent_id: parent.map(|p| p.id()),
            at: point,
            index,
        });
        if let Some(name) = json::string_of(action, "as") {
            variables.insert(name.to_owned(), created.to_value());
        }
        true
    }

    /// A `connect` (section 9.4): a relation of its `type` from its `source` to its `target`, both elements of the
    /// working diagram, with its `attributes`, under a new id; `as` names it.
    fn connect(&mut self, action: &Json, body: &Json, at: &str, variables: &mut Vars) -> bool {
        let Some(type_value) = self.member(body, at, "type", variables, Value::Null) else {
            return false;
        };
        let type_name = match type_value
            .as_str()
            .and_then(|t| self.specification.metamodel().type_of(t))
        {
            Some(t) if t.is_relation && !t.is_abstract() => t.name.clone(),
            _ => return self.refuse(format!("The connect at {at} names no relation type it can create.")),
        };
        let Some(source) = self.member(body, at, "source", variables, Value::Null) else {
            return false;
        };
        let Some(target) = self.member(body, at, "target", variables, Value::Null) else {
            return false;
        };
        let (Some(from), Some(to)) = (DislElement::from_value(&source), DislElement::from_value(&target)) else {
            return self.refuse(format!("The connect at {at} does not name two elements to connect."));
        };
        let Some(attributes) = self.attributes(body, at, variables) else {
            return false;
        };
        let created = self.ids.next(&type_name).and_then(|id| {
            self.diagram
                .add_relation(&type_name, &id, Some(&from), Some(&to), attributes.clone())
        });
        let created = match created {
            Ok(created) => created,
            Err(e) => return self.refuse(e),
        };
        self.changes.push(DislChange::Connect {
            type_name,
            id: created.id(),
            source_id: from.id(),
            target_id: to.id(),
            attributes,
        });
        if let Some(name) = json::string_of(action, "as") {
            variables.insert(name.to_owned(), created.to_value());
        }
        true
    }

    /// A `retype` (section 9.4): its `target`, by default `self`, made the type `to` names, as `behavior.retype`
    /// allows. The element changes type in the working diagram, so the actions after it see it as the new type.
    fn retype(&mut self, body: &Json, at: &str, variables: &mut Vars) -> bool {
        let self_value = variables.get("self").cloned().unwrap_or(Value::Null);
        let Some(target) = self.member(body, at, "target", variables, self_value) else {
            return false;
        };
        let Some(to) = self.member(body, at, "to", variables, Value::Null) else {
            return false;
        };
        let Some(element) = DislElement::from_value(&target) else {
            return self.refuse(format!("The retype at {at} has no element to change."));
        };
        let Some(type_name) = to.as_str() else {
            return self.refuse(format!("The retype at {at} names no type."));
        };
        let transaction = retype::change(self.specification, &element, type_name, None);
        if let Some(refusal) = transaction.refusal {
            return self.refuse(refusal);
        }
        let Some(change @ DislChange::Retype { .. }) = transaction.changes.into_iter().next() else {
            return self.refuse(format!("The retype at {at} changed nothing."));
        };
        if let DislChange::Retype { attributes, .. } = &change {
            self.snapshot(&element);
            if let Some(t) = self.specification.metamodel().type_of(type_name) {
                element.retype(t.clone(), attributes);
            }
        }
        self.changes.push(change);
        true
    }

    fn snapshot(&mut self, element: &DislElement) {
        if let Some(before) = &mut self.before {
            before.entry(element.key()).or_insert_with(|| element.snapshot());
        }
    }

    fn set(&mut self, target: &DislElement, values: Map) -> bool {
        self.snapshot(target);
        for (name, value) in values.iter() {
            if let Err(e) = target.store(name, value.clone()) {
                return self.refuse(e);
            }
        }
        if !values.is_empty() {
            self.changes.push(DislChange::Set {
                element_id: target.id(),
                type_name: target.type_name(),
                attributes: values,
            });
        }
        true
    }

    fn target(&mut self, action: &Json, at: &str, variables: &Vars) -> Option<DislElement> {
        let mut value = variables.get("self").cloned().unwrap_or(Value::Null);
        if let Some(declared) = json::get(action, "target") {
            value = self.value(declared, &json::pointer(at, "target"), variables)?;
        }
        match DislElement::from_value(&value) {
            Some(element) => Some(element),
            None => {
                self.refuse(format!("The action at {at} has no element to change."));
                None
            }
        }
    }

    fn member(&mut self, body: &Json, at: &str, name: &str, variables: &Vars, fallback: Value) -> Option<Value> {
        match json::get(body, name) {
            Some(json) if body.is_object() => self.value(json, &json::pointer(at, name), variables),
            _ => Some(fallback),
        }
    }

    fn value(&mut self, json: &Json, at: &str, variables: &Vars) -> Option<Value> {
        let value = if json.is_string() || json.is_object() {
            evaluation::expression(self.specification, json, at, self.context, variables)
        } else {
            Ok(values::to_value(json))
        };
        match value {
            Ok(value) => Some(value),
            Err(message) => {
                self.refuse(format!("{at}: {message}"));
                None
            }
        }
    }
}
