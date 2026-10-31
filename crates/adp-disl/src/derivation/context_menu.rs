//! The context menu of a target (DISL section 7.3; the standalone host's `ContextMenuDerivation`): the matching sets
//! of `toolbox.contextMenus` whose `when` holds, in declaration order; each entry once, or once per item of its
//! `forEach`, unless its `visible` fails; and each entry's availability from its own `unavailable` reasons, then its
//! operation's, then `enabled`. Groups follow `group` (DISL 0.3 section 7.3): consecutive entries of one set with an
//! equal name, or with none, form one group, decided by the entries as declared; a group whose entries are all
//! hidden is dropped. `moveUp` and `moveDown` are unavailable at the first and the last place among their siblings,
//! with `behavior.messages`' `std.atStart` and `std.atEnd`.

use adp_cel::{Map, Value};

use super::env::DislEnv;
use super::evaluation::{self, Vars};
use super::wire_ids::WireIdMap;
use crate::expressions::contexts;
use crate::json::{self, Json};
use crate::loading::specification::DislSpecification;
use crate::model::budgets;
use crate::model::diagram::DislDiagram;
use crate::model::element::DislElement;

/// What a context menu is opened on (section 7.3): an element, the empty canvas, or a pending connection.
#[derive(Debug, Clone)]
pub enum DislMenuTarget {
    /// A node or relation: the sets whose `for` names its type or a supertype.
    Element(DislElement),
    /// The empty canvas: the sets `for` `diagram`, `self` bound to the diagram.
    Canvas(DislDiagram),
    /// A finished connect gesture from `source` to `target`, either of which may name nothing: the sets `for`
    /// `connection`, else the connect edit of `relation_type` when it is a derived relation (section 4.11).
    Connection {
        diagram: DislDiagram,
        source: Option<DislElement>,
        target: Option<DislElement>,
        relation_type: Option<String>,
    },
}

impl DislMenuTarget {
    pub fn diagram(&self) -> DislDiagram {
        match self {
            DislMenuTarget::Element(element) => element.diagram(),
            DislMenuTarget::Canvas(diagram) | DislMenuTarget::Connection { diagram, .. } => diagram.clone(),
        }
    }
}

/// A keyboard shortcut as a specification writes it (`Alt+Up`), and its parts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedShortcut {
    /// As written.
    pub text: String,
    /// The key: the text after the last modifier.
    pub key: String,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub meta: bool,
}

impl DerivedShortcut {
    /// Reads `Ctrl`, `Shift`, `Alt` and `Meta` (and `Control`, `Option`, `Cmd`) before the key, joined by `+`.
    pub fn parse(text: &str) -> DerivedShortcut {
        let parts: Vec<&str> = text.split('+').collect();
        let (mut ctrl, mut shift, mut alt, mut meta) = (false, false, false, false);
        for used in 0..parts.len() - 1 {
            match parts[used].trim().to_lowercase().as_str() {
                "ctrl" | "control" => ctrl = true,
                "shift" => shift = true,
                "alt" | "option" => alt = true,
                "meta" | "cmd" | "command" => meta = true,
                _ => {
                    return DerivedShortcut {
                        text: text.to_owned(),
                        key: parts[used..].join("+"),
                        ctrl,
                        shift,
                        alt,
                        meta,
                    };
                }
            }
        }
        DerivedShortcut {
            text: text.to_owned(),
            key: parts[parts.len() - 1].to_owned(),
            ctrl,
            shift,
            alt,
            meta,
        }
    }

    /// The key as a browser's `KeyboardEvent.key` spells it, which is how a host sends and matches a keypress.
    pub fn event_key(&self) -> &str {
        match self.key.as_str() {
            "Space" => " ",
            "Up" => "ArrowUp",
            "Down" => "ArrowDown",
            "Left" => "ArrowLeft",
            "Right" => "ArrowRight",
            "Esc" => "Escape",
            other => other,
        }
    }
}

/// One entry of a context menu as a host shows it, its id mapped by a [`WireIdMap`].
#[derive(Debug, Clone)]
pub struct DerivedMenuEntry {
    /// The action id.
    pub id: String,
    /// The entry's kind (`operation`, `delete`, `editLabel`, …).
    pub kind: String,
    /// The operation it runs.
    pub operation: Option<String>,
    pub label: String,
    pub icon: String,
    pub shortcut: Option<DerivedShortcut>,
    pub available: bool,
    /// Why it cannot run now; empty when available.
    pub unavailable_reason: String,
    /// Its `args`, evaluated, and the name a `forEach` binds; empty for most entries.
    pub arguments: Map,
}

/// One group of a context menu: consecutive entries with one `group` name (none when they state none).
#[derive(Debug, Clone)]
pub struct DerivedMenuGroup {
    pub name: Option<String>,
    pub entries: Vec<DerivedMenuEntry>,
}

/// The context menu of `target`.
pub fn derive(
    specification: &DislSpecification,
    target: &DislMenuTarget,
    env: &DislEnv,
    ids: &WireIdMap,
) -> Vec<DerivedMenuGroup> {
    let mut groups = Vec::new();
    let cel = Value::map(env.to_cel());
    let diagram = target.diagram();
    let (context, variables, types) = match target {
        DislMenuTarget::Element(element) => (
            contexts::ELEMENT,
            evaluation::vars([
                ("self", element.to_value()),
                ("diagram", diagram.to_value()),
                ("env", cel.clone()),
            ]),
            element.element_type().linearisation.clone(),
        ),
        DislMenuTarget::Connection { source, target, .. } => (
            contexts::CONNECTION,
            evaluation::vars([
                (
                    "source",
                    source.as_ref().map(DislElement::to_value).unwrap_or(Value::Null),
                ),
                (
                    "target",
                    target.as_ref().map(DislElement::to_value).unwrap_or(Value::Null),
                ),
                ("sourceAnchor", Value::Null),
                ("position", Value::Null),
                ("diagram", diagram.to_value()),
                ("env", cel.clone()),
            ]),
            vec!["connection".to_owned()],
        ),
        DislMenuTarget::Canvas(_) => (
            contexts::ELEMENT,
            evaluation::vars([
                ("self", diagram.to_value()),
                ("diagram", diagram.to_value()),
                ("env", cel.clone()),
            ]),
            vec!["diagram".to_owned()],
        ),
    };

    let mut matched = false;
    if let Some(sets) = json::items(json::member(specification.root(), "toolbox"), "contextMenus") {
        for (index, set) in sets.iter().enumerate() {
            let pointer = json::pointer_at("/toolbox/contextMenus", index);
            if !applies(set, target) {
                continue;
            }
            matched = true;
            if let Some(when) = json::get(set, "when")
                && !evaluation::holds(
                    specification,
                    when,
                    &json::pointer(&pointer, "when"),
                    context,
                    &variables,
                )
            {
                continue;
            }
            let menu = Menu {
                specification,
                context,
                types: &types,
                target,
                ids,
            };
            menu.set(set, &pointer, &variables, &mut groups);
        }
    }

    if !matched
        && let DislMenuTarget::Connection {
            relation_type: Some(relation),
            source,
            target,
            ..
        } = target
    {
        connect_edit(specification, &diagram, source, target, relation, cel, ids, &mut groups);
    }
    groups
}

/// The entry of the operation whose action id is `action_id`, for a gesture that names an action `target`'s menu does not
/// offer: an operation the toolbox drops or the wire names, run on what its `for` admits (a canvas for one with none),
/// with the reason it is unavailable. An operation the menu holds behind its `visible` has an entry only while it has
/// such a reason, since a gesture is told why; otherwise it is not offered, and there is no entry.
pub fn operation_entry(
    specification: &DislSpecification,
    target: &DislMenuTarget,
    env: &DislEnv,
    ids: &WireIdMap,
    action_id: &str,
) -> Option<DerivedMenuEntry> {
    let diagram = target.diagram();
    let (types, self_value) = match target {
        DislMenuTarget::Element(element) => (element.element_type().linearisation.clone(), element.to_value()),
        DislMenuTarget::Canvas(_) => (Vec::new(), diagram.to_value()),
        DislMenuTarget::Connection { .. } => return None,
    };
    let operations = json::member(json::member(specification.root(), "behavior"), "operations");
    let (name, operation) = operations
        .as_object()?
        .iter()
        .find(|(name, _)| ids.action_id(&types, "operation", Some(name), &Map::default()) == action_id)?;
    let admitted = json::strings_of(operation, "for");
    let admits = match target {
        DislMenuTarget::Element(element) => admitted.iter().any(|n| element.is_a(n)),
        _ => admitted.is_empty(),
    };
    if !admits {
        return None;
    }
    let variables = evaluation::vars([
        ("self", self_value),
        ("selection", Value::list(Vec::new())),
        ("p", Value::map(Map::default())),
        ("diagram", diagram.to_value()),
        ("env", Value::map(env.to_cel())),
        ("position", Value::Null),
    ]);
    let at = operation_pointer(name);
    let reason = json::get(operation, "unavailable").and_then(|reasons| {
        evaluation::first_reason(
            specification,
            reasons,
            &json::pointer(&at, "unavailable"),
            contexts::OPERATION,
            &variables,
        )
    });
    let held_back = json::member(json::member(specification.root(), "toolbox"), "contextMenus")
        .as_array()
        .into_iter()
        .flatten()
        .filter(|set| applies(set, target))
        .flat_map(|set| json::member(set, "tools").as_array().into_iter().flatten())
        .any(|tool| json::string_of(tool, "operation") == Some(name.as_str()));
    if held_back && reason.is_none() {
        return None;
    }
    let label = match json::get(operation, "label") {
        Some(message) => evaluation::message(
            specification,
            message,
            &json::pointer(&at, "label"),
            contexts::OPERATION,
            &variables,
            name,
        ),
        None => name.clone(),
    };
    Some(DerivedMenuEntry {
        id: action_id.to_owned(),
        kind: "operation".into(),
        operation: Some(name.clone()),
        label,
        icon: json::string_of(operation, "icon").unwrap_or("").to_owned(),
        shortcut: json::string_of(operation, "shortcut").map(DerivedShortcut::parse),
        available: reason.is_none(),
        unavailable_reason: reason.unwrap_or_default(),
        arguments: Map::default(),
    })
}

fn applies(set: &Json, target: &DislMenuTarget) -> bool {
    let declared = json::has(set, "for");
    let names = json::strings_of(set, "for");
    match target {
        DislMenuTarget::Element(element) => {
            !declared
                || names
                    .iter()
                    .any(|n| n != "diagram" && n != "connection" && element.is_a(n))
        }
        DislMenuTarget::Canvas(_) => names.iter().any(|n| n == "diagram"),
        DislMenuTarget::Connection { .. } => names.iter().any(|n| n == "connection"),
    }
}

fn operation<'a>(specification: &'a DislSpecification, name: Option<&str>) -> Option<&'a Json> {
    let name = name?;
    json::get(
        json::member(json::member(specification.root(), "behavior"), "operations"),
        name,
    )
    .filter(|o| o.is_object())
}

fn operation_pointer(name: &str) -> String {
    json::pointer("/behavior/operations", name)
}

struct Menu<'a> {
    specification: &'a DislSpecification,
    context: &'static str,
    types: &'a [String],
    target: &'a DislMenuTarget,
    ids: &'a WireIdMap,
}

impl Menu<'_> {
    fn set(&self, set: &Json, pointer: &str, variables: &Vars, groups: &mut Vec<DerivedMenuGroup>) {
        let Some(tools) = json::items(set, "tools") else {
            return;
        };
        let mut current: Option<Vec<DerivedMenuEntry>> = None;
        let mut current_name: Option<String> = None;
        for (index, tool) in tools.iter().enumerate() {
            let at = json::pointer_at(&json::pointer(pointer, "tools"), index);
            let name = json::string_of(tool, "group").map(str::to_owned);
            if current.is_none() || name != current_name {
                if let Some(entries) = current.take()
                    && !entries.is_empty()
                {
                    groups.push(DerivedMenuGroup {
                        name: current_name.clone(),
                        entries,
                    });
                }
                current = Some(Vec::new());
                current_name = name;
            }
            let entries = self.entries(tool, &at, variables);
            current.as_mut().expect("a group is open").extend(entries);
        }
        if let Some(entries) = current
            && !entries.is_empty()
        {
            groups.push(DerivedMenuGroup {
                name: current_name,
                entries,
            });
        }
    }

    fn entries(&self, tool: &Json, pointer: &str, variables: &Vars) -> Vec<DerivedMenuEntry> {
        let Some(for_each) = json::get(tool, "forEach") else {
            return self
                .entry(tool, pointer, variables, Map::default())
                .into_iter()
                .collect();
        };
        // An entry per item, in list order, with item (or its as name) and index bound (section 7.3); an empty or
        // failing list gives none.
        let Ok(Value::List(items)) = evaluation::expression(
            self.specification,
            for_each,
            &json::pointer(pointer, "forEach"),
            self.context,
            variables,
        ) else {
            return Vec::new();
        };
        let named = json::string_of(tool, "as");
        let mut entries = Vec::new();
        for (index, item) in items.iter().enumerate() {
            let mut bindings = Map::default();
            bindings.insert("item", item.clone());
            bindings.insert("index", Value::Int(index as i64));
            if let Some(named) = named {
                bindings.insert(named, item.clone());
            }
            entries.extend(self.entry(tool, pointer, variables, bindings));
        }
        entries
    }

    fn entry(&self, tool: &Json, pointer: &str, outer: &Vars, bindings: Map) -> Option<DerivedMenuEntry> {
        let specification = self.specification;
        let mut variables = outer.clone();
        for (name, value) in bindings.iter() {
            variables.insert(name.clone(), value.clone());
        }
        if let Some(visible) = json::get(tool, "visible")
            && !evaluation::holds(
                specification,
                visible,
                &json::pointer(pointer, "visible"),
                self.context,
                &variables,
            )
        {
            return None;
        }

        let kind = json::string_of(tool, "kind").unwrap_or("operation");
        let operation_name = json::string_of(tool, "operation");
        let operation = operation(specification, operation_name);

        let mut arguments = bindings.clone();
        for (name, argument) in json::members(tool, "args") {
            let value = evaluation::expression(
                specification,
                argument,
                &json::pointer(&json::pointer(pointer, "args"), name),
                self.context,
                &variables,
            )
            .unwrap_or(Value::Null);
            arguments.insert(name.clone(), value);
        }

        let fallback = match (operation.and_then(|o| json::get(o, "label")), operation_name) {
            (Some(label), Some(name)) => evaluation::message(
                specification,
                label,
                &json::pointer(&operation_pointer(name), "label"),
                contexts::OPERATION,
                &self.operational(&variables, &arguments),
                kind,
            ),
            _ => match json::string_of(tool, "via")
                .and_then(|via| specification.metamodel().type_of(via).map(|t| (via, t)))
            {
                Some((via, t)) => json::string_of(&t.json, "label").unwrap_or(via).to_owned(),
                None => kind.to_owned(),
            },
        };
        let label = match json::get(tool, "label") {
            Some(message) => evaluation::message(
                specification,
                message,
                &json::pointer(pointer, "label"),
                self.context,
                &variables,
                &fallback,
            ),
            None => fallback,
        };
        let icon = json::string_of(tool, "icon")
            .or_else(|| operation.and_then(|o| json::string_of(o, "icon")))
            .unwrap_or("")
            .to_owned();
        let shortcut =
            json::string_of(tool, "shortcut").or_else(|| operation.and_then(|o| json::string_of(o, "shortcut")));

        let reason = self.unavailable(tool, pointer, &variables, kind, operation, operation_name, &arguments);
        let available = reason.is_none()
            && json::get(tool, "enabled").is_none_or(|enabled| {
                evaluation::holds(
                    specification,
                    enabled,
                    &json::pointer(pointer, "enabled"),
                    self.context,
                    &variables,
                )
            });

        Some(DerivedMenuEntry {
            id: self.ids.action_id(self.types, kind, operation_name, &arguments),
            kind: kind.to_owned(),
            operation: operation_name.map(str::to_owned),
            label,
            icon,
            shortcut: shortcut.map(DerivedShortcut::parse),
            available,
            unavailable_reason: reason.unwrap_or_default(),
            arguments,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn unavailable(
        &self,
        tool: &Json,
        pointer: &str,
        variables: &Vars,
        kind: &str,
        operation: Option<&Json>,
        operation_name: Option<&str>,
        arguments: &Map,
    ) -> Option<String> {
        let specification = self.specification;
        if let Some(own) = json::get(tool, "unavailable")
            && let Some(reason) = evaluation::first_reason(
                specification,
                own,
                &json::pointer(pointer, "unavailable"),
                self.context,
                variables,
            )
        {
            return Some(reason);
        }
        if let (Some(inherited), Some(name)) = (operation.and_then(|o| json::get(o, "unavailable")), operation_name)
            && let Some(reason) = evaluation::first_reason(
                specification,
                inherited,
                &json::pointer(&operation_pointer(name), "unavailable"),
                contexts::OPERATION,
                &self.operational(variables, arguments),
            )
        {
            return Some(reason);
        }
        let changes = if matches!(
            kind,
            "delete" | "duplicate" | "create-connected" | "create-child" | "connect"
        ) {
            Some("model")
        } else {
            operation.and_then(budgets::changes)
        };
        if let Some(changes) = changes
            && let Some(diagram) = variables.get("diagram").and_then(DislDiagram::from_value)
            && let Some(withheld) = budgets::withheld_reason(&diagram, changes == "view")
        {
            return Some(withheld);
        }
        if matches!(kind, "moveUp" | "moveDown")
            && let DislMenuTarget::Element(element) = self.target
        {
            let siblings = match element.parent() {
                Some(parent) => parent.children(),
                None => element
                    .diagram()
                    .nodes()
                    .into_iter()
                    .filter(|n| n.parent().is_none())
                    .collect(),
            };
            let place = siblings
                .iter()
                .position(|s| s == element)
                .map(|p| p as i64)
                .unwrap_or(-1);
            if kind == "moveUp" && place <= 0 {
                return Some(evaluation::standard_message(
                    specification,
                    "std.atStart",
                    variables,
                    "It is already the first.",
                ));
            }
            if kind == "moveDown" && place >= siblings.len() as i64 - 1 {
                return Some(evaluation::standard_message(
                    specification,
                    "std.atEnd",
                    variables,
                    "It is already the last.",
                ));
            }
        }
        None
    }

    /// The operation context's variables (section 12.3) for an entry: its target as `self` and its selection, its
    /// arguments as `p`.
    fn operational(&self, variables: &Vars, arguments: &Map) -> Vars {
        let diagram = self.target.diagram();
        let self_value = match self.target {
            DislMenuTarget::Element(element) => element.to_value(),
            DislMenuTarget::Canvas(_) => diagram.to_value(),
            DislMenuTarget::Connection { .. } => Value::Null,
        };
        let selection = match self.target {
            DislMenuTarget::Element(element) => Value::list(vec![element.to_value()]),
            _ => Value::list(Vec::new()),
        };
        let mut operational = variables.clone();
        operational.insert("self".into(), self_value);
        operational.insert("selection".into(), selection);
        operational.insert("p".into(), Value::map(arguments.clone()));
        operational.insert("diagram".into(), diagram.to_value());
        operational.insert("position".into(), Value::Null);
        operational
    }
}

/// The menu of a connection on a derived relation type with a `connect` edit (section 4.11): its operation, as one
/// entry.
#[allow(clippy::too_many_arguments)]
fn connect_edit(
    specification: &DislSpecification,
    diagram: &DislDiagram,
    source: &Option<DislElement>,
    target: &Option<DislElement>,
    relation: &str,
    env: Value,
    ids: &WireIdMap,
    groups: &mut Vec<DerivedMenuGroup>,
) {
    let Some(derived) = specification
        .metamodel()
        .type_of(relation)
        .and_then(|t| t.derived().cloned())
        .filter(|d| d.is_object())
    else {
        return;
    };
    let Some(operation_name) = json::string_of(json::member(&derived, "edits"), "connect") else {
        return;
    };
    let Some(operation) = operation(specification, Some(operation_name)) else {
        return;
    };

    // The relation does not exist yet: self is its two ends, as the operation's reasons read them.
    let mut self_map = Map::default();
    self_map.insert(
        "source",
        source.as_ref().map(DislElement::to_value).unwrap_or(Value::Null),
    );
    self_map.insert(
        "target",
        target.as_ref().map(DislElement::to_value).unwrap_or(Value::Null),
    );
    self_map.insert("type", Value::from(relation));
    let variables = evaluation::vars([
        ("self", Value::map(self_map)),
        ("selection", Value::list(Vec::new())),
        ("p", Value::map(Map::default())),
        ("diagram", diagram.to_value()),
        ("env", env),
        ("position", Value::Null),
    ]);
    let at = operation_pointer(operation_name);
    let label = match json::get(operation, "label") {
        Some(message) => evaluation::message(
            specification,
            message,
            &json::pointer(&at, "label"),
            contexts::OPERATION,
            &variables,
            operation_name,
        ),
        None => operation_name.to_owned(),
    };
    let reason = json::get(operation, "unavailable").and_then(|reasons| {
        evaluation::first_reason(
            specification,
            reasons,
            &json::pointer(&at, "unavailable"),
            contexts::OPERATION,
            &variables,
        )
    });
    groups.push(DerivedMenuGroup {
        name: None,
        entries: vec![DerivedMenuEntry {
            id: ids.action_id(
                &[relation.to_owned(), "connection".to_owned()],
                "connect",
                Some(operation_name),
                &Map::default(),
            ),
            kind: "operation".into(),
            operation: Some(operation_name.to_owned()),
            label,
            icon: json::string_of(operation, "icon").unwrap_or("").to_owned(),
            shortcut: json::string_of(operation, "shortcut").map(DerivedShortcut::parse),
            available: reason.is_none(),
            unavailable_reason: reason.unwrap_or_default(),
            arguments: Map::default(),
        }],
    });
}
