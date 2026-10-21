//! What the agent behavior model's module ran itself (etalii.adp spec 030, FR-018, task T074), ported from the
//! standalone host's `AbmContextActionProvider.cs`, `Commands/AbmCommands.cs`, `Commands/ArrangeAbmCommand.cs`,
//! `Commands/ArrangeAbmNodeCommand.cs`, `AbmSession.cs` and `AbmValidator.cs` at `8b6ccfc`.
//!
//! Which actions a node, a drop and a finished parent-line gesture offer, and whether each is available, are the
//! specification's (its context menus with the wire ids of `x-abm`, through `adp-diagram`); so are the deletion's
//! confirmation, the parent line's `moveUnder`, the property rows and the rules. What is here is what the module did
//! in code:
//!
//! - An add creates the node at once, as the definition's `addChild` on a node, or its `addHere` placed where a drop
//!   lands, under the nearest node above it that takes another child; then it asks for the node's label, as the
//!   inline editor did, and the answer renames the node the add made.
//! - Rename and Edit notes ask first, with the node's label or notes, and write what is answered.
//! - Move earlier and Move later move a node one place among its siblings.
//! - Arrange diagram forgets every dragged position: it removes the registration's `layout:` block and never touches
//!   the Markdown.
//! - A drag moves a node's whole row to the height it was dropped at and, when it lands past a sibling's middle,
//!   reorders it among its siblings, writing the Markdown and the registration as one edit.
//! - The findings are the specification's rules, then the reader's `abm.no-behavior`.

use std::cell::RefCell;
use std::path::PathBuf;

use adp_cel::{Map, Value};
use adp_context::Prompt;
use adp_diagram::document::DiagramChange;
use adp_diagram::session::{ActionOutcome, ActionTarget, DiagramSession};
use adp_diagram::workspace::DiagramWorkspace;
use adp_diagram::{
    DiagramDocument, DiagramProblem, DiagramProblemLocation, DiagramProblemSeverity, DislDiagramDefinition,
    DocumentEdit,
};
use adp_disl::behavior::{operations, retype, write};
use adp_disl::derivation::constraints;
use adp_disl::model::builder;
use adp_disl::{DislConstraintOptions, DislEnv, DislInvocation, DislMenuTarget, IdSource};
use adp_fbl::ModelChange;
use adp_fbl::model::FindingSeverity;

use crate::kinds;
use crate::layout::{self, Arrangement, NODE_HEIGHT, NODE_WIDTH};
use crate::lines::LineDocument;
use crate::model::{Model, rule_ids};
use crate::plugin::{GONE, read_model};
use crate::{parser, registration, writer};

/// Rename a node in place.
pub const RENAME: &str = "abm.rename";
/// Remove a node and everything beneath it.
pub const REMOVE: &str = "abm.remove";
/// Move a node one place earlier among its siblings: run sooner.
pub const MOVE_EARLIER: &str = "abm.move-earlier";
/// Move a node one place later among its siblings: run later.
pub const MOVE_LATER: &str = "abm.move-later";
/// Edit a node's notes.
pub const EDIT_NOTES: &str = "abm.edit-notes";
/// Draw a parent line: move the child under the parent.
pub const CONNECT_CHILD: &str = "abm.connect.child";
/// Forget every dragged position, so the whole tree is drawn tidy again.
pub const ARRANGE: &str = "abm.arrange";
/// What the add action of each kind starts with: `abm.add.sequence` and its kin.
pub const ADD_PREFIX: &str = "abm.add.";

/// What an action the target's menu does not offer, or offers unavailable, answers.
pub const NOT_AVAILABLE: &str = "That action is not available for this item.";

/// The ids a node an operation creates has in the transaction's working model: a node's id is its place, which the
/// definition derives once the document is read again (`AbmDefinition.NewIds`).
struct NewIds;

impl IdSource for NewIds {
    fn next(&self, _type_name: &str) -> Result<String, String> {
        Ok("new".to_owned())
    }
}

/// The label a new node of `kind` starts with, before the author types their own (`AddAbmNodeCommandHandler`).
pub fn starting_label(kind: &str) -> &'static str {
    match kind {
        kinds::SEQUENCE => "New steps",
        kinds::FALLBACK => "New alternatives",
        kinds::PARALLEL => "New independent work",
        kinds::REPEAT | kinds::GUARD => "New condition",
        kinds::CHECK => "New question",
        kinds::ACTION => "New action",
        kinds::ASK => "New question for the user",
        kinds::DELEGATE => "New task",
        _ => "",
    }
}

/// The add action of `kind`.
pub fn add_action(kind: &str) -> String {
    format!("{ADD_PREFIX}{kind}")
}

/// The node an add made, whose label is still being asked: by the body, the action that made it, and its id.
#[derive(Debug, Default)]
pub struct AbmActions {
    pending: RefCell<Option<(PathBuf, String, String)>>,
}

impl AbmActions {
    /// Runs `action_id` on `target`, or none to leave it to the specification's behaviour.
    pub fn execute(
        &self,
        session: &DiagramSession,
        workspace: &mut DiagramWorkspace,
        target: &ActionTarget,
        action_id: &str,
        _ids: &dyn IdSource,
    ) -> Option<ActionOutcome> {
        let handled = action_id.starts_with(ADD_PREFIX)
            || [RENAME, EDIT_NOTES, MOVE_EARLIER, MOVE_LATER, ARRANGE, CONNECT_CHILD].contains(&action_id);
        if !handled {
            return None;
        }
        let document = workspace.documents().get(session.body_path())?;
        if let Some(refusal) = document.refusal() {
            return Some(ActionOutcome::Refused(refusal));
        }
        if !offered(document, target, action_id) {
            return Some(ActionOutcome::Refused(NOT_AVAILABLE.to_owned()));
        }
        let model = read_model(document.body_bytes());
        let outcome = match (action_id, target) {
            (
                CONNECT_CHILD,
                ActionTarget::Connection {
                    source: Some(parent),
                    target: Some(child),
                },
            ) => connect_child(document, parent, child)
                .and_then(|edit| workspace.execute(edit))
                .map(|()| None),
            (CONNECT_CHILD, _) => {
                Err("A parent line is drawn from the parent to the node that belongs under it.".to_owned())
            }
            (ARRANGE, _) => self
                .arrange(document)
                .and_then(|edit| workspace.execute(edit))
                .map(|()| None),
            (add, ActionTarget::Element(id)) if add.starts_with(ADD_PREFIX) => {
                self.add_child(session, workspace, &model, id, &add[ADD_PREFIX.len()..])
            }
            (add, ActionTarget::Canvas { x, y }) if add.starts_with(ADD_PREFIX) => {
                self.add_here(session, workspace, &model, &add[ADD_PREFIX.len()..], *x, *y)
            }
            (_, ActionTarget::Element(id)) => match model.node(id) {
                None => Err(GONE.to_owned()),
                Some(node) => match action_id {
                    RENAME => Ok(Some(Prompt::Input {
                        title: "Rename".into(),
                        icon: "mdi-pencil-outline".into(),
                        label: "Label".into(),
                        initial_value: node.label.clone(),
                        confirm_label: "Rename".into(),
                    })),
                    // Not the label, so not the inline editor: notes are the agent's extra instructions for this node.
                    EDIT_NOTES => Ok(Some(Prompt::Input {
                        title: "Notes".into(),
                        icon: "mdi-note-text-outline".into(),
                        label: "Notes".into(),
                        initial_value: node.notes.clone(),
                        confirm_label: "Save".into(),
                    })),
                    _ => {
                        let place = model
                            .siblings_of(node)
                            .iter()
                            .position(|s| s.id == node.id)
                            .unwrap_or_default() as i64;
                        let index = if action_id == MOVE_EARLIER {
                            place - 1
                        } else {
                            place + 2
                        };
                        let change = ModelChange::Move {
                            id: node.id.clone(),
                            new_parent_id: node.parent_id.clone(),
                            index,
                        };
                        write_body(document, "Move", change)
                            .and_then(|edit| workspace.execute(edit))
                            .map(|()| None)
                    }
                },
            },
            _ => return None,
        };
        Some(match outcome {
            Ok(None) => ActionOutcome::Done,
            Ok(Some(prompt)) => ActionOutcome::Prompt(prompt),
            Err(refusal) => ActionOutcome::Refused(refusal),
        })
    }

    /// Answers the prompt `action_id` raised on `target` with `value`: the label of the node an add made, a node's new
    /// label, or its notes.
    pub fn answer(
        &self,
        session: &DiagramSession,
        workspace: &mut DiagramWorkspace,
        target: &ActionTarget,
        action_id: &str,
        value: &str,
    ) -> Option<ActionOutcome> {
        let (node_id, attribute) = if action_id.starts_with(ADD_PREFIX) {
            let pending = self.pending.borrow_mut().take();
            match pending {
                Some((body, action, id)) if body == session.body_path() && action == action_id => (id, "label"),
                _ => return Some(ActionOutcome::Refused(GONE.to_owned())),
            }
        } else {
            let ActionTarget::Element(id) = target else {
                return None;
            };
            match action_id {
                RENAME => (id.clone(), "label"),
                EDIT_NOTES => (id.clone(), "notes"),
                _ => return None,
            }
        };
        let document = workspace.documents().get(session.body_path())?;
        if read_model(document.body_bytes()).node(&node_id).is_none() {
            return Some(ActionOutcome::Refused(GONE.to_owned()));
        }
        let change = ModelChange::Set {
            id: node_id,
            attributes: [(attribute, Value::String(value.to_owned()))].into_iter().collect(),
        };
        let label = if attribute == "label" { "Rename" } else { "Notes" };
        Some(
            match write_body(document, label, change).and_then(|edit| workspace.execute(edit)) {
                Ok(()) => ActionOutcome::Done,
                Err(refusal) => ActionOutcome::Refused(refusal),
            },
        )
    }

    /// A node of `kind` added as the last child of `parent_id`, by the definition's `addChild`; then its label asked.
    fn add_child(
        &self,
        session: &DiagramSession,
        workspace: &mut DiagramWorkspace,
        model: &Model,
        parent_id: &str,
        kind: &str,
    ) -> Result<Option<Prompt>, String> {
        let Some(kind) = kinds::ALL.iter().find(|k| k.id == kind) else {
            return Err(format!("A behavior model has no `{kind}` node."));
        };
        let parent = model.node(parent_id).ok_or(GONE)?;
        let document = workspace.documents().get(session.body_path()).ok_or(GONE)?;
        let specification = document.tool().definition.specification();
        let working = builder::from(document.fbl(), specification).diagram;
        let element = working.element_by_id(parent_id).ok_or(GONE)?;
        let transaction = operations::run(
            specification,
            "addChild",
            &working,
            Some(&element),
            &NewIds,
            Some(&invocation(kind.type_name)),
            Some(&DislEnv::default()),
        );
        if let Some(refusal) = transaction.refusal {
            return Err(refusal);
        }
        let changes: Vec<DiagramChange> = transaction
            .changes
            .iter()
            .map(|c| DiagramChange::Body(write::to_fbl(specification, c)))
            .collect();
        let edit = document.plan("Add", &changes)?;
        let new_id = format!("{}.{}", parent.id, parent.child_ids.len() + 1);
        workspace.execute(edit)?;
        Ok(Some(self.ask_label(session, kind.id, new_id)))
    }

    /// A node of `kind` dropped at (`x`, `y`), by the definition's `addHere`, placed under the nearest node above the
    /// drop that takes another child, among its children by how far left it landed; then its label asked.
    fn add_here(
        &self,
        session: &DiagramSession,
        workspace: &mut DiagramWorkspace,
        model: &Model,
        kind: &str,
        x: f64,
        y: f64,
    ) -> Result<Option<Prompt>, String> {
        let Some(kind) = kinds::ALL.iter().find(|k| k.id == kind) else {
            return Err(format!("A behavior model has no `{kind}` node."));
        };
        let document = workspace.documents().get(session.body_path()).ok_or(GONE)?;
        let stored = registration::read(document.registration_bytes().unwrap_or_default());
        let (parent, index) = layout::place_drop(model, &stored, x, y)?;
        let specification = document.tool().definition.specification();
        let working = builder::from(document.fbl(), specification).diagram;
        let transaction = operations::run(
            specification,
            "addHere",
            &working,
            None,
            &NewIds,
            Some(&invocation(kind.type_name)),
            Some(&DislEnv::default()),
        );
        if let Some(refusal) = transaction.refusal {
            return Err(refusal);
        }
        let changes: Vec<DiagramChange> = transaction
            .changes
            .iter()
            .map(|c| match write::to_fbl(specification, c) {
                ModelChange::Add {
                    type_name,
                    id,
                    attributes,
                    slot,
                    ..
                } => ModelChange::Add {
                    type_name,
                    id,
                    attributes,
                    parent_id: parent.clone(),
                    index: Some(index as i64),
                    slot,
                },
                other => other,
            })
            .map(DiagramChange::Body)
            .collect();
        let edit = document.plan("Add", &changes)?;
        let new_id = match &parent {
            Some(parent) => format!("{parent}.{}", index + 1),
            None => (index + 1).to_string(),
        };
        workspace.execute(edit)?;
        Ok(Some(self.ask_label(session, kind.id, new_id)))
    }

    /// The question for the label of the node `new_id` an add of `kind` made, remembered until it is answered.
    fn ask_label(&self, session: &DiagramSession, kind: &str, new_id: String) -> Prompt {
        *self.pending.borrow_mut() = Some((session.body_path().to_path_buf(), add_action(kind), new_id));
        Prompt::Input {
            title: "Rename".into(),
            icon: "mdi-pencil-outline".into(),
            label: "Label".into(),
            initial_value: starting_label(kind).into(),
            confirm_label: "Rename".into(),
        }
    }

    /// Arrange diagram: the registration without its `layout:` block, as one edit; the Markdown is never touched.
    fn arrange(&self, document: &DiagramDocument) -> Result<DocumentEdit, String> {
        let Some(bytes) = document.registration_bytes() else {
            return Err(
                "This behavior model was opened without a registration, so it has no dragged positions to forget."
                    .to_owned(),
            );
        };
        if registration::read(bytes).is_empty() {
            return Err("This behavior model is already arranged.".to_owned());
        }
        document.rewrite_registration(
            "Arrange diagram",
            &registration::with_block(bytes, &layout::Positions::new()),
        )
    }
}

/// What a drag of the nodes of `moves` writes, each point the centre it was dropped at: its row moved to that height
/// and, when it landed past a sibling's middle, its place among its siblings changed, the Markdown and the registration
/// written as one edit; nothing for a drop that changes neither.
pub fn drag(document: &DiagramDocument, moves: &[(String, f64, f64)]) -> Result<DocumentEdit, String> {
    let Some(before) = document.registration_bytes() else {
        return Err(
            "This behavior model was opened without a registration, so there is nowhere to keep a position.".to_owned(),
        );
    };
    let mut edit = document.rewrite_registration("Move", before)?;
    let Some((id, x, y)) = moves.first() else {
        return Ok(edit);
    };
    let (x, y) = (x - NODE_WIDTH / 2.0, y - NODE_HEIGHT / 2.0);
    let model = read_model(document.body_bytes());
    let node = model
        .node(id)
        .ok_or("That is not something this behavior model can move.")?;
    let stored = registration::read(before);
    let arrangement = Arrangement::of(&model, &stored, node, x, y);
    if arrangement.is_nothing() {
        return Ok(edit);
    }
    let sibling_count = model.siblings_of(node).len();
    let parent_id = node.parent_id.clone();
    let mut after = model.clone();
    let mut renamed = stored.clone();
    if arrangement.from != arrangement.to {
        let change = ModelChange::Move {
            id: node.id.clone(),
            new_parent_id: parent_id.clone(),
            index: arrangement.move_index(),
        };
        let mut lines = LineDocument::parse(&String::from_utf8_lossy(document.body_bytes()));
        let parent = parent_id.as_deref().and_then(|p| model.node(p));
        writer::move_node(&mut lines, &model, node, parent, arrangement.move_index())?;
        after = parser::parse(&lines);
        renamed = arrangement.renamed(parent_id.as_deref(), sibling_count, &stored);
        edit = write_body(document, "Move", change)?;
    }

    // Every node in the row and beneath it is written out, at where it is drawn now plus the row's move: so a row
    // below one that only hung from its parent keeps its place under it.
    let arranged = layout::arrange(&after, &renamed);
    let row = match parent_id.as_deref().and_then(|p| after.node(p)) {
        Some(parent) => after.children_of(parent),
        None => after.roots(),
    };
    let mut positions = renamed.clone();
    for moved in &after.nodes {
        if row.iter().any(|member| Model::is_within(&moved.id, &member.id)) {
            let (ax, ay) = arranged[&moved.id];
            positions.insert(moved.id.clone(), (ax, ay + arrangement.dy));
        }
    }
    let positioned = document.rewrite_registration("Move", &registration::with_block(before, &positions))?;
    let mut steps = positioned.steps;
    steps.extend(edit.steps);
    Ok(DocumentEdit {
        label: "Move".into(),
        body_path: edit.body_path,
        steps,
    })
}

/// A parent line drawn from `parent_id` to `child_id`: the derived `Child` relation's `connect` edit, the definition's
/// `moveUnder`, whose aborts refuse a line that cannot move the child in the writer's words
/// (`ConnectAbmChildCommandHandler`).
fn connect_child(document: &DiagramDocument, parent_id: &str, child_id: &str) -> Result<DocumentEdit, String> {
    let specification = document.tool().definition.specification();
    let working = builder::from(document.fbl(), specification).diagram;
    let child = working.element_by_id(child_id).ok_or(GONE)?;
    let parent = working
        .element_by_id(parent_id)
        .ok_or("The node it was moved under is no longer in this behavior model.")?;
    let transaction = operations::connect(
        specification,
        "Child",
        &working,
        &parent,
        &child,
        &NewIds,
        Some(&DislEnv::default()),
    );
    if let Some(refusal) = transaction.refusal {
        return Err(refusal);
    }
    let changes: Vec<DiagramChange> = transaction.changes.into_iter().map(DiagramChange::Disl).collect();
    document.plan("Connect", &changes)
}

/// The kind a value of the Kind row names: its menu name (`Retry` for a Retry) or its id.
fn kind_named(value: &str) -> Option<&'static kinds::Kind> {
    kinds::ALL.iter().find(|kind| {
        let choice = if kind.id == kinds::RETRY { "Retry" } else { kind.keyword };
        choice == value || kind.id == value
    })
}

/// What setting row `property_id` of node `element_id` to `value` writes, as the module's property provider did
/// (`AbmContextPropertyProvider.Change`): the Kind by a kind's menu name, through the definition's `behavior.retype`
/// when it changes; the label; a Retry's attempts, at least 1; the notes.
pub fn set_property(
    document: &DiagramDocument,
    element_id: &str,
    property_id: &str,
    value: &str,
) -> Result<Vec<DiagramChange>, String> {
    let model = read_model(document.body_bytes());
    let node = model.node(element_id).ok_or(GONE)?;
    let set = |attribute: &str| {
        vec![DiagramChange::Body(ModelChange::Set {
            id: node.id.clone(),
            attributes: [(attribute, Value::String(value.to_owned()))].into_iter().collect(),
        })]
    };
    match property_id {
        "abm.kind" => {
            let kind = kind_named(value).ok_or_else(|| format!("'{value}' is not a kind of node."))?;
            set_kind(document, node, kind, 0)
        }
        "abm.label" => Ok(set("label")),
        "abm.attempts" => match value.trim().parse::<i32>() {
            Ok(attempts) if attempts >= 1 => set_kind(document, node, kinds::of(kinds::RETRY), attempts),
            _ => Err(format!(
                "'{value}' is not a number of attempts; a Retry allows at least 1."
            )),
        },
        "abm.notes" => Ok(set("notes")),
        other => Err(format!("'{other}' cannot be edited on this selection.")),
    }
}

/// A node's kind changed (`SetAbmNodeKindCommandHandler`): to another kind, the definition's `behavior.retype`, a
/// Retry's attempts from its `attributeMapping`; a Retry's attempts, and a kind to itself, rewrite the keyword in place.
fn set_kind(
    document: &DiagramDocument,
    node: &crate::model::Node,
    kind: &kinds::Kind,
    retry_count: i32,
) -> Result<Vec<DiagramChange>, String> {
    if node.kind != kind.id && retry_count < 1 {
        let specification = document.tool().definition.specification();
        let element = document.element(&node.id).ok_or(GONE)?;
        let transaction = retype::change(specification, &element, kind.type_name, Some(&DislEnv::default()));
        if let Some(refusal) = transaction.refusal {
            return Err(refusal);
        }
        return Ok(transaction.changes.into_iter().map(DiagramChange::Disl).collect());
    }
    let attempts = if kind.id == kinds::RETRY && retry_count < 1 {
        node.retry_count.max(kinds::DEFAULT_RETRY_COUNT)
    } else {
        retry_count
    };
    Ok(vec![DiagramChange::Body(ModelChange::Retype {
        id: node.id.clone(),
        type_name: kind.type_name.to_owned(),
        attributes: [("attempts", Value::Int(i64::from(attempts)))].into_iter().collect(),
    })])
}

/// The findings of `document`: the specification's rules, evaluated from `options`, then what the reader reports.
pub fn problems(document: &DiagramDocument, options: DislConstraintOptions) -> Vec<DiagramProblem> {
    let mut problems: Vec<DiagramProblem> = constraints::evaluate(
        document.tool().definition.specification(),
        &document.model().diagram,
        &options,
    )
    .into_iter()
    .map(|finding| DiagramProblem {
        severity: DislDiagramDefinition::severity_of(&finding),
        location: match (finding.element_ids.first(), finding.line) {
            (Some(id), _) => Some(DiagramProblemLocation::Element(id.clone())),
            (None, Some(line)) => Some(DiagramProblemLocation::Line(line.max(1) as u32)),
            (None, None) => None,
        },
        message: finding.message,
        rule_id: finding.code,
    })
    .collect();
    problems.extend(
        document
            .fbl()
            .findings
            .iter()
            .filter(|f| f.code == rule_ids::NO_BEHAVIOR)
            .map(|f| DiagramProblem {
                severity: match f.severity {
                    FindingSeverity::Error => DiagramProblemSeverity::Error,
                    FindingSeverity::Info => DiagramProblemSeverity::Info,
                    _ => DiagramProblemSeverity::Warning,
                },
                message: f.message.clone(),
                rule_id: f.code.clone(),
                location: f
                    .location
                    .as_ref()
                    .map(|l| DiagramProblemLocation::Line(l.line.max(1) as u32)),
            }),
    );
    problems
}

/// Whether the menu of `target` offers `action_id`, available.
fn offered(document: &DiagramDocument, target: &ActionTarget, action_id: &str) -> bool {
    let diagram = document.model().diagram.clone();
    let menu = match target {
        ActionTarget::Element(id) => match document.element(id) {
            Some(element) => DislMenuTarget::Element(element),
            None => return false,
        },
        ActionTarget::Canvas { .. } => DislMenuTarget::Canvas(diagram),
        ActionTarget::Connection { source, target } => DislMenuTarget::Connection {
            source: source.as_deref().and_then(|id| document.element(id)),
            target: target.as_deref().and_then(|id| document.element(id)),
            diagram,
            relation_type: Some("Child".to_owned()),
        },
    };
    document
        .tool()
        .definition
        .menu_entries(&menu, &DislEnv::default())
        .into_iter()
        .flatten()
        .any(|e| e.id == action_id && e.available)
}

/// The invocation of an add of a node of the specification's type `type_name`.
fn invocation(type_name: &str) -> DislInvocation {
    DislInvocation {
        parameters: Some(
            [("kind", Value::String(type_name.to_owned()))]
                .into_iter()
                .collect::<Map>(),
        ),
        ..DislInvocation::default()
    }
}

/// The edit, labelled `label`, that writes `change` to the Markdown.
fn write_body(document: &DiagramDocument, label: &str, change: ModelChange) -> Result<DocumentEdit, String> {
    document.plan(label, &[DiagramChange::Body(change)])
}
