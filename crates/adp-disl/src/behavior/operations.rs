//! Runs a definition's operations (DISL section 9.3; the standalone host's `OperationInterpreter`), a toolbox tool's
//! drop (section 7.1) and a connect gesture released on empty canvas (sections 6.10 and 7.2) as one transaction
//! each, into changes a host writes and actions it carries out.
//!
//! The diagram given is the transaction's working state, changed as the actions run; a host gives each transaction
//! a diagram of its own. An operation that is not available is refused with its reason: the edit gate's (section
//! 9.1), the first of its `unavailable` reasons that applies, or, when only `enabled` is false,
//! `std.notApplicable`. A `plugin` operation is handed back whole.

use adp_cel::{Map, Value};

use super::actions::ActionRunner;
use super::change::{DislChange, DislTransaction, HostAction};
use super::ids::IdSource;
use crate::derivation::env::DislEnv;
use crate::derivation::evaluation::{self, Vars};
use crate::expressions::contexts;
use crate::json::{self, Json};
use crate::loading::specification::DislSpecification;
use crate::model::budgets;
use crate::model::diagram::DislDiagram;
use crate::model::element::DislElement;
use crate::model::values;

/// What an operation is invoked with besides its target (sections 9.3 and 12.3).
#[derive(Debug, Clone, Default)]
pub struct DislInvocation {
    /// The domain point it was invoked at, such as a context menu's on empty canvas: a map with `x` and `y`.
    pub position: Option<Map>,
    /// The selected elements, for an operation `for: "selection"`.
    pub selection: Option<Vec<DislElement>>,
    /// The answered parameters, bound as `p`.
    pub parameters: Option<Map>,
}

/// Every variable of `context` bound to null, but `diagram` and `env`.
pub(crate) fn variables_of(context: &str, diagram: &DislDiagram, env: Option<&DislEnv>) -> Vars {
    let mut variables: Vars = contexts::variables_of(context)
        .iter()
        .map(|n| ((*n).to_owned(), Value::Null))
        .collect();
    variables.insert("diagram".into(), diagram.to_value());
    variables.insert("env".into(), Value::map(env.cloned().unwrap_or_default().to_cel()));
    variables
}

fn operation_of<'a>(specification: &'a DislSpecification, id: &str) -> Option<&'a Json> {
    json::get(
        json::member(json::member(specification.root(), "behavior"), "operations"),
        id,
    )
}

/// One parameter an operation asks for before it runs (section 9.3, `params`): its name, the label its dialog shows,
/// its type, and the value the dialog opens with (its `default`, evaluated on the target).
#[derive(Debug, Clone)]
pub struct DislOperationParameter {
    pub name: String,
    pub label: String,
    pub type_name: String,
    pub default: Value,
}

impl DislOperationParameter {
    /// The text typed for the parameter as the value `p` binds: a number for an `int` or a `double`, a truth value
    /// for a `bool`, else the text itself; none when it does not read as its type.
    pub fn value_of(&self, text: &str) -> Option<Value> {
        let trimmed = text.trim();
        match self.type_name.as_str() {
            "int" | "integer" | "long" => trimmed.parse().ok().map(Value::Int),
            "double" | "number" | "float" => trimmed.parse().ok().map(Value::Double),
            "bool" | "boolean" => match trimmed.to_ascii_lowercase().as_str() {
                "true" => Some(Value::Bool(true)),
                "false" => Some(Value::Bool(false)),
                _ => None,
            },
            _ => Some(Value::from(text)),
        }
    }
}

/// The parameters operation `operation_id` asks for on `element` (none for one `for: "diagram"`), in the order they
/// are declared; empty for an operation that asks for none.
pub fn parameters(
    specification: &DislSpecification,
    operation_id: &str,
    diagram: &DislDiagram,
    element: Option<&DislElement>,
    env: Option<&DislEnv>,
) -> Vec<DislOperationParameter> {
    let Some(operation) = operation_of(specification, operation_id) else {
        return Vec::new();
    };
    let pointer = json::pointer(&json::pointer("/behavior/operations", operation_id), "params");
    let mut variables = variables_of(contexts::OPERATION, diagram, env);
    variables.insert("self".into(), element.map(DislElement::to_value).unwrap_or(Value::Null));
    variables.insert(
        "selection".into(),
        Value::list(element.map(DislElement::to_value).into_iter().collect()),
    );
    variables.insert("position".into(), Value::map(Map::default()));
    variables.insert("p".into(), Value::map(Map::default()));
    json::members(operation, "params")
        .map(|(name, declared)| {
            let at = json::pointer(&pointer, name);
            let default = match json::get(declared, "default") {
                Some(value) if value.is_object() => evaluation::expression(
                    specification,
                    value,
                    &json::pointer(&at, "default"),
                    contexts::OPERATION,
                    &variables,
                )
                .unwrap_or(Value::Null),
                Some(value) => values::to_value(value),
                None => Value::Null,
            };
            let label = match json::get(declared, "label") {
                Some(label) => evaluation::message(
                    specification,
                    label,
                    &json::pointer(&at, "label"),
                    contexts::OPERATION,
                    &variables,
                    name,
                ),
                None => name.clone(),
            };
            DislOperationParameter {
                name: name.clone(),
                label,
                type_name: json::string_of(declared, "type").unwrap_or("string").to_owned(),
                default,
            }
        })
        .collect()
}

/// Runs operation `operation_id` on `element` (none for one `for: "diagram"`).
pub fn run(
    specification: &DislSpecification,
    operation_id: &str,
    diagram: &DislDiagram,
    element: Option<&DislElement>,
    ids: &dyn IdSource,
    invocation: Option<&DislInvocation>,
    env: Option<&DislEnv>,
) -> DislTransaction {
    let Some(operation) = operation_of(specification, operation_id) else {
        return DislTransaction::refused(format!("There is no operation '{operation_id}'."));
    };
    let pointer = json::pointer("/behavior/operations", operation_id);
    let invocation = invocation.cloned().unwrap_or_default();
    let mut variables = variables_of(contexts::OPERATION, diagram, env);
    variables.insert("self".into(), element.map(DislElement::to_value).unwrap_or(Value::Null));
    variables.insert(
        "selection".into(),
        Value::list(match &invocation.selection {
            Some(selection) => selection.iter().map(DislElement::to_value).collect(),
            None => element.map(DislElement::to_value).into_iter().collect(),
        }),
    );
    variables.insert("position".into(), Value::map(invocation.position.unwrap_or_default()));
    variables.insert("p".into(), Value::map(invocation.parameters.unwrap_or_default()));

    let is_a = |t: &str| element.is_some_and(|e| e.is_a(t));
    if let Some(reason) = unavailable(
        specification,
        operation,
        &pointer,
        diagram,
        element.is_some().then_some(&is_a as &dyn Fn(&str) -> bool),
        &variables,
        operation_id,
    ) {
        return DislTransaction::refused(reason);
    }
    actions(specification, operation, &pointer, diagram, ids, variables)
}

/// Runs the operation a derived relation type's `edits.connect` names for a new line of `relation_type` from
/// `source` to `target` (section 4.11.4), as one transaction. The relation does not exist yet, so `self` is its two
/// ends and its type - a map with `source`, `target` and `type` - and the operation applies when its `for` names the
/// relation type or one it inherits from.
pub fn connect(
    specification: &DislSpecification,
    relation_type: &str,
    diagram: &DislDiagram,
    source: &DislElement,
    target: &DislElement,
    ids: &dyn IdSource,
    env: Option<&DislEnv>,
) -> DislTransaction {
    let relation = match specification.metamodel().type_of(relation_type) {
        Some(t) if t.is_relation => t.clone(),
        _ => return DislTransaction::refused(format!("There is no relation type '{relation_type}'.")),
    };
    let derived = relation.derived().filter(|d| d.is_object());
    let Some(operation_id) = derived.and_then(|d| json::string_of(json::member(d, "edits"), "connect")) else {
        return DislTransaction::refused(match derived.and_then(|d| json::string_of(d, "reason")) {
            Some(reason) => reason.to_owned(),
            None => format!("A {relation_type} cannot be drawn."),
        });
    };
    let Some(operation) = operation_of(specification, operation_id) else {
        return DislTransaction::refused(format!("There is no operation '{operation_id}'."));
    };
    let pointer = json::pointer("/behavior/operations", operation_id);
    let mut variables = variables_of(contexts::OPERATION, diagram, env);
    let mut self_map = Map::default();
    self_map.insert("source", source.to_value());
    self_map.insert("target", target.to_value());
    self_map.insert("type", Value::from(relation_type));
    variables.insert("self".into(), Value::map(self_map));
    variables.insert("selection".into(), Value::list(Vec::new()));
    variables.insert("position".into(), Value::map(Map::default()));
    variables.insert("p".into(), Value::map(Map::default()));

    let is_a = |t: &str| relation.is_a(t);
    if let Some(refusal) = unavailable(
        specification,
        operation,
        &pointer,
        diagram,
        Some(&is_a),
        &variables,
        operation_id,
    ) {
        return DislTransaction::refused(refusal);
    }
    actions(specification, operation, &pointer, diagram, ids, variables)
}

/// The operation's `plugin` handed back whole, its `args` as written, or its actions run.
fn actions(
    specification: &DislSpecification,
    operation: &Json,
    pointer: &str,
    diagram: &DislDiagram,
    ids: &dyn IdSource,
    mut variables: Vars,
) -> DislTransaction {
    if let Some(plugin) = json::get(operation, "plugin") {
        let name = match plugin {
            Json::String(s) => s.clone(),
            other => json::string_of(other, "name").unwrap_or("").to_owned(),
        };
        let arguments = json::members(plugin, "args")
            .map(|(k, v)| (k.clone(), values::to_value(v)))
            .collect();
        return DislTransaction {
            changes: Vec::new(),
            host_actions: vec![HostAction::Plugin { name, arguments }],
            refusal: None,
        };
    }
    let mut runner = ActionRunner::new(specification, diagram, ids, contexts::OPERATION);
    if let Some(list) = json::get(operation, "actions") {
        runner.run(list, &json::pointer(pointer, "actions"), &mut variables);
    }
    runner.transaction()
}

fn initial_attributes(
    specification: &DislSpecification,
    owner: &Json,
    initial_at: &str,
    created: &crate::DislType,
    variables: &Vars,
) -> Result<Map, String> {
    let mut attributes = Map::default();
    for (name, value) in json::members(owner, "initial") {
        let Some(attribute) = created.attributes.get(name) else {
            return Err(format!("{} has no attribute '{name}'.", created.name));
        };
        let value = if value.is_object() {
            evaluation::expression(
                specification,
                value,
                &json::pointer(initial_at, name),
                contexts::CREATE,
                variables,
            )?
        } else {
            values::from_json(value, attribute, specification.metamodel()).unwrap_or(Value::Null)
        };
        if !value.is_null() {
            attributes.insert(name.clone(), value);
        }
    }
    Ok(attributes)
}

/// Drops toolbox tool `tool_id` at `position` (section 7.1): a `create` of its type with its `initial` values, and
/// its `after` (`select` or `editLabel`) handed back.
pub fn drop(
    specification: &DislSpecification,
    tool_id: &str,
    diagram: &DislDiagram,
    position: &Map,
    ids: &dyn IdSource,
    env: Option<&DislEnv>,
) -> DislTransaction {
    let Some((tool, pointer)) = tool(specification, tool_id) else {
        return DislTransaction::refused(format!("There is no tool '{tool_id}'."));
    };
    let created = match json::string_of(tool, "creates").and_then(|c| specification.metamodel().type_of(c)) {
        Some(t) if !t.is_relation && !t.is_abstract() => t.clone(),
        _ => return DislTransaction::refused(format!("Tool '{tool_id}' creates no node.")),
    };
    let mut variables = variables_of(contexts::CREATE, diagram, env);
    variables.insert("position".into(), Value::map(position.clone()));
    variables.insert("elementType".into(), Value::String(created.name.clone()));
    variables.insert("tool".into(), Value::from(tool_id));

    let attributes = match initial_attributes(
        specification,
        tool,
        &json::pointer(&pointer, "initial"),
        &created,
        &variables,
    ) {
        Ok(attributes) => attributes,
        Err(e) => return DislTransaction::refused(e),
    };
    let element = match ids
        .next(&created.name)
        .and_then(|id| diagram.add_node(&created.name, &id, attributes.clone()))
    {
        Ok(element) => element,
        Err(e) => return DislTransaction::refused(e),
    };
    let after = match json::string_of(tool, "after") {
        Some("editLabel") => vec![HostAction::EditLabel {
            element_id: element.id(),
            label: None,
        }],
        Some("select") | None => vec![HostAction::Select {
            element_ids: vec![element.id()],
        }],
        _ => Vec::new(),
    };
    DislTransaction {
        changes: vec![DislChange::Create {
            type_name: created.name.clone(),
            id: element.id(),
            attributes,
            parent_id: None,
            at: Value::map(position.clone()),
            index: None,
        }],
        host_actions: after,
        refusal: None,
    }
}

/// Runs a connect gesture of `relation_type` released on empty canvas (sections 6.10 and 7.2): the missing end - the
/// `new_end`, `source` or `target` - created at `position` from the `createSource` or `createTarget` of the tool the
/// edge's `connect.tool` names, else of a toolbox tool that creates the relation type, and the relation between it
/// and `existing`, as one transaction: a create, then a connect. A tool that states no such end, or `"ask"`, is
/// refused: the runtime has no menu of targets to offer.
#[allow(clippy::too_many_arguments)]
pub fn connect_to_new(
    specification: &DislSpecification,
    relation_type: &str,
    diagram: &DislDiagram,
    existing: &DislElement,
    new_end: &str,
    position: &Map,
    ids: &dyn IdSource,
    env: Option<&DislEnv>,
) -> DislTransaction {
    assert!(
        matches!(new_end, "source" | "target"),
        "The new end is the source or the target."
    );
    if !specification
        .metamodel()
        .type_of(relation_type)
        .is_some_and(|t| t.is_relation && !t.is_abstract())
    {
        return DislTransaction::refused(format!("There is no relation type '{relation_type}'."));
    }
    let Some((tool, pointer)) = connect_tool(specification, relation_type) else {
        return DislTransaction::refused(format!("No tool creates a {relation_type} on empty canvas."));
    };
    let key = if new_end == "target" {
        "createTarget"
    } else {
        "createSource"
    };
    let end = match json::get(tool, key) {
        Some(end) if end.as_str() != Some("ask") => end,
        _ => return DislTransaction::refused(format!("A {relation_type} cannot end on empty canvas.")),
    };
    let type_name = match end {
        Json::String(s) => Some(s.as_str()),
        other => json::string_of(other, "type"),
    };
    let created = match type_name.and_then(|t| specification.metamodel().type_of(t)) {
        Some(t) if !t.is_relation && !t.is_abstract() => t.clone(),
        _ => {
            return DislTransaction::refused(format!(
                "The {key} of {relation_type} names no node type it can create."
            ));
        }
    };
    let mut variables = variables_of(contexts::CREATE, diagram, env);
    variables.insert("position".into(), Value::map(position.clone()));
    variables.insert("elementType".into(), Value::String(created.name.clone()));
    variables.insert(
        "tool".into(),
        json::string_of(tool, "id").map(Value::from).unwrap_or(Value::Null),
    );
    let initial_at = json::pointer(&json::pointer(&pointer, key), "initial");
    let attributes = match initial_attributes(specification, end, &initial_at, &created, &variables) {
        Ok(attributes) => attributes,
        Err(e) => return DislTransaction::refused(e),
    };

    let made = ids
        .next(&created.name)
        .and_then(|id| diagram.add_node(&created.name, &id, attributes.clone()))
        .and_then(|node| {
            let (source, target) = if new_end == "target" {
                (existing.clone(), node.clone())
            } else {
                (node.clone(), existing.clone())
            };
            let relation = ids.next(relation_type).and_then(|id| {
                diagram.add_relation(relation_type, &id, Some(&source), Some(&target), Map::default())
            })?;
            Ok((node, relation, source, target))
        });
    match made {
        Ok((node, relation, source, target)) => DislTransaction {
            changes: vec![
                DislChange::Create {
                    type_name: created.name.clone(),
                    id: node.id(),
                    attributes,
                    parent_id: None,
                    at: Value::map(position.clone()),
                    index: None,
                },
                DislChange::Connect {
                    type_name: relation_type.to_owned(),
                    id: relation.id(),
                    source_id: source.id(),
                    target_id: target.id(),
                    attributes: Map::default(),
                },
            ],
            host_actions: Vec::new(),
            refusal: None,
        },
        Err(e) => DislTransaction::refused(e),
    }
}

/// Whether operation `operation_id` is for `element` (none for the diagram): its `for` names the element's type or
/// one it inherits from, or the diagram, the selection, or nothing. It asks nothing about the operation's state, so
/// a host can tell an entry that is not offered here from one that is refused now.
pub fn applies_to(specification: &DislSpecification, operation_id: &str, element: Option<&DislElement>) -> bool {
    let Some(operation) = operation_of(specification, operation_id) else {
        return false;
    };
    let targets = json::strings_of(operation, "for");
    targets.is_empty()
        || targets.iter().any(|t| t == "diagram" || t == "selection")
        || element.is_some_and(|e| targets.iter().any(|t| e.is_a(t)))
}

fn unavailable(
    specification: &DislSpecification,
    operation: &Json,
    pointer: &str,
    diagram: &DislDiagram,
    is_a: Option<&dyn Fn(&str) -> bool>,
    variables: &Vars,
    operation_id: &str,
) -> Option<String> {
    let targets = json::strings_of(operation, "for");
    let applies = targets.is_empty()
        || targets.iter().any(|t| t == "diagram" || t == "selection")
        || is_a.is_some_and(|is_a| targets.iter().any(|t| is_a(t)));
    if !applies {
        return Some(not_applicable(specification, diagram, operation_id));
    }
    let behavior = json::member(specification.root(), "behavior");
    if let Some(gate) = json::get(behavior, "editGate") {
        let gated = evaluation::vars([
            ("self", diagram.to_value()),
            ("diagram", diagram.to_value()),
            ("env", variables.get("env").cloned().unwrap_or(Value::Null)),
        ]);
        if let Some(reason) =
            evaluation::first_reason(specification, gate, "/behavior/editGate", contexts::ELEMENT, &gated)
        {
            return Some(reason);
        }
    }
    if let Some(changes) = budgets::changes(operation)
        && let Some(withheld) = budgets::withheld_reason(diagram, changes == "view")
    {
        return Some(withheld);
    }
    if let Some(reasons) = json::get(operation, "unavailable")
        && let Some(reason) = evaluation::first_reason(
            specification,
            reasons,
            &json::pointer(pointer, "unavailable"),
            contexts::OPERATION,
            variables,
        )
    {
        return Some(reason);
    }
    match json::get(operation, "enabled") {
        Some(enabled)
            if !evaluation::holds(
                specification,
                enabled,
                &json::pointer(pointer, "enabled"),
                contexts::OPERATION,
                variables,
            ) =>
        {
            Some(not_applicable(specification, diagram, operation_id))
        }
        _ => None,
    }
}

fn not_applicable(specification: &DislSpecification, diagram: &DislDiagram, operation_id: &str) -> String {
    let mut variables = variables_of(contexts::ELEMENT, diagram, None);
    variables.insert("self".into(), diagram.to_value());
    variables.insert("operationId".into(), Value::from(operation_id));
    evaluation::standard_message(
        specification,
        "std.notApplicable",
        &variables,
        "That does not apply to this selection.",
    )
}

/// The tool `tool_id` as the palette declares it, inline in a group or from the toolbox's library, with its pointer.
fn tool<'a>(specification: &'a DislSpecification, tool_id: &str) -> Option<(&'a Json, String)> {
    let toolbox = json::get(specification.root(), "toolbox")?;
    let groups = json::get(toolbox, "groups")?;
    in_groups(toolbox, groups, "/toolbox/groups", tool_id)
}

fn in_groups<'a>(toolbox: &'a Json, list: &'a Json, at: &str, tool_id: &str) -> Option<(&'a Json, String)> {
    for (index, group) in list.as_array()?.iter().enumerate() {
        let group_at = json::pointer_at(at, index);
        for (position, entry) in json::items(group, "tools").into_iter().flatten().enumerate() {
            let entry_at = json::pointer_at(&json::pointer(&group_at, "tools"), position);
            if entry.as_str() == Some(tool_id)
                && let Some(shared) = json::get(json::member(toolbox, "tools"), tool_id)
            {
                return Some((shared, json::pointer("/toolbox/tools", tool_id)));
            }
            if entry.is_object() && json::string_of(entry, "id") == Some(tool_id) {
                return Some((entry, entry_at));
            }
        }
        if let Some(nested) = json::get(group, "groups")
            && let Some(found) = in_groups(toolbox, nested, &json::pointer(&group_at, "groups"), tool_id)
        {
            return Some(found);
        }
    }
    None
}

/// The tool a connect gesture of `relation_type` runs: the one its edge notation's `connect.tool` names (section
/// 6.10), else the first library tool (`toolbox.tools`) that creates it.
fn connect_tool<'a>(specification: &'a DislSpecification, relation_type: &str) -> Option<(&'a Json, String)> {
    let library = json::get(json::member(specification.root(), "toolbox"), "tools")?.as_object()?;
    let edge = json::get(
        json::member(json::member(specification.root(), "notation"), "edges"),
        relation_type,
    );
    let named = edge.and_then(|e| json::string_of(json::member(e, "connect"), "tool"));
    library.iter().find_map(|(name, tool)| {
        let chosen = match named {
            Some(named) => name == named,
            None => json::string_of(tool, "creates") == Some(relation_type),
        };
        chosen.then(|| (tool, json::pointer("/toolbox/tools", name)))
    })
}
