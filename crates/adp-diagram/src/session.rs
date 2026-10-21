//! One open view of a diagram: what it shows through its viewport, and the gestures, property edits and actions it
//! turns into edits on the project's history (etalii.adp spec 030, FR-015, FR-017, FR-018; contracts/core-shell.md,
//! `open`, `set_viewport`, `move_elements`, `properties`, `set_property`, `toolbox`, `execute`).
//!
//! Ported from the standalone host's `EtAlii.Adp.Diagram/IDiagramSession.cs` and the frames of
//! `EtAlii.Adp.Diagram.Modules` at `8b6ccfc`: `DocumentDiagramSession.cs` (the baseline, a viewport report, the
//! document change, the move gestures and their refusals), `DiagramPropertyProvider.cs` (the rows and a new value as a
//! command), `DiagramActionProvider.cs` with `DiagramAction.cs` (the menu, the run and the deletion's confirmation)
//! and `DiagramElementSourceResolver.cs` with `DiagramSnapshotSourceResolver.cs` (what a selected element shows). The
//! standalone host's modules each said what their elements are; here every DISL and FBL tool type is drawn the same
//! way, from its specification, its binding and its registration, so one session serves them all. What a
//! notation looks like is the scene's (`adp-scene`), not the session's.
//!
//! A session holds no document: the project's [`DiagramDocuments`] does, shared by every view of a body, and every
//! edit goes through the project's history in [`DiagramWorkspace`], so a gesture is one undo away like every other
//! edit, and a change to several elements is one undo.

use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_cel::{Map, Value};
use adp_context::{ActionGroup, Prompt, PropertyRow};
use adp_disl::behavior::{deletion, operations};
use adp_disl::derivation::{constraints, form_commit, gestures};
use adp_disl::model::builder;
use adp_disl::{
    DerivedMenuEntry, DislChange, DislConstraintOptions, DislElement, DislEnv, DislInvocation, DislMenuTarget,
    DislReaderFinding, DislTransaction, HostAction, IdSource,
};
use adp_fbl::model::codes;

use crate::change_handler::{DiagramDocumentChangeHandler, DocumentChange};
use crate::definition::ToolboxItem;
use crate::disl_definition::DislDiagramDefinition;
use crate::document::{DiagramChange, DiagramDocument, DiagramDocuments, NOT_OPEN};
use crate::element::{DiagramDelta, DiagramElement, DiagramViewport, ElementKind, Rect};
use crate::module::DiagramModule;
use crate::problems::{DiagramProblem, DiagramProblemLocation, DiagramProblemSeverity};
use crate::workspace::DiagramWorkspace;

/// What a drag answers when the diagram may not be edited.
pub const READ_ONLY: &str = "This diagram is read-only.";

/// What a placement of something that has no position of its own answers.
pub const NOT_MOVABLE: &str = "Only a node can be moved; a relation is drawn between its ends.";

/// What an element that is gone answers.
pub const GONE: &str = "That is no longer in this diagram.";

/// The size a node is taken to have when the viewport decides what is in view: a scene knows its notation's sizes,
/// the session does not, so a node counts as in view when a box of this size at its position touches the viewport.
pub const NODE_EXTENT: f64 = 200.0;

/// What a context menu is opened on.
#[derive(Debug, Clone, PartialEq)]
pub enum ActionTarget {
    /// An element, by its id.
    Element(String),
    /// The empty canvas, at a point in the diagram's units.
    Canvas { x: f64, y: f64 },
    /// A finished connect gesture between two elements, either of which may be none.
    Connection {
        source: Option<String>,
        target: Option<String>,
    },
}

/// The answer to an action: it happened, it was refused with a sentence, or the person is asked first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionOutcome {
    Done,
    Refused(String),
    Prompt(Prompt),
}

/// What a selected element shows: the path the selection names it by, and its text.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectedElement {
    pub path: Vec<String>,
    pub text: String,
    pub element: DiagramElement,
}

/// One open view of a diagram.
pub struct DiagramSession {
    body_path: PathBuf,
    viewport: DiagramViewport,
    changes: DiagramDocumentChangeHandler,
    env: DislEnv,
}

impl DiagramSession {
    /// A view of the open document whose body is at `body_path`.
    pub fn new(body_path: impl Into<PathBuf>) -> Self {
        let body_path = body_path.into();
        DiagramSession {
            changes: DiagramDocumentChangeHandler::new(body_path.clone()),
            body_path,
            viewport: DiagramViewport::UNBOUNDED,
            env: DislEnv::default(),
        }
    }

    pub fn body_path(&self) -> &Path {
        &self.body_path
    }

    pub fn viewport(&self) -> DiagramViewport {
        self.viewport
    }

    /// What the connection was last given.
    pub fn delivered(&self) -> &[DiagramElement] {
        self.changes.delivered()
    }

    fn document<'a>(&self, documents: &'a DiagramDocuments) -> Result<&'a DiagramDocument, String> {
        documents.get(&self.body_path).ok_or_else(|| NOT_OPEN.to_owned())
    }

    /// The module of the tool type this view shows, when it has one.
    fn module(&self, documents: &DiagramDocuments) -> Option<Rc<dyn DiagramModule>> {
        self.document(documents).ok()?.tool().module.clone()
    }

    /// The elements in view: every node whose box touches the viewport, every relation both of whose ends are in
    /// view, in the model's order.
    pub fn render(&self, documents: &DiagramDocuments) -> Result<Vec<DiagramElement>, String> {
        let elements = self.document(documents)?.elements();
        if self.viewport == DiagramViewport::UNBOUNDED {
            return Ok(elements);
        }
        let shown: std::collections::HashSet<&str> = elements
            .iter()
            .filter(|e| {
                e.kind == ElementKind::Node && self.viewport.intersects(Rect::new(e.x, e.y, NODE_EXTENT, NODE_EXTENT))
            })
            .map(|e| e.id.as_str())
            .collect();
        Ok(elements
            .iter()
            .filter(|e| match e.kind {
                ElementKind::Node => shown.contains(e.id.as_str()),
                ElementKind::Relation => {
                    e.source.as_deref().is_some_and(|s| shown.contains(s))
                        && e.target.as_deref().is_some_and(|t| shown.contains(t))
                }
            })
            .cloned()
            .collect())
    }

    /// The baseline: one add of everything in view, empty for an empty diagram, so the shell knows the open has
    /// been answered.
    pub fn baseline(&mut self, documents: &DiagramDocuments) -> Result<Vec<DiagramDelta>, String> {
        let elements = self.render(documents)?;
        Ok(vec![DiagramDelta::Add(self.changes.deliver(elements).to_vec())])
    }

    /// Narrows or widens what the view holds; the deltas that bring it into line.
    pub fn update_view(
        &mut self,
        documents: &DiagramDocuments,
        viewport: DiagramViewport,
    ) -> Result<Vec<DiagramDelta>, String> {
        self.viewport = viewport;
        let elements = self.render(documents)?;
        Ok(self.changes.refresh(elements))
    }

    /// The view's reaction to the body at `path` changing, an edit's or one from outside.
    pub fn on_document_changed(&mut self, documents: &DiagramDocuments, path: &Path) -> DocumentChange {
        let rendered = self.render(documents);
        self.changes
            .on_document_changed(path, || rendered.map_err(io::Error::other))
    }

    /// What a selection of `element_id` shows, or none when nothing has that id: its label, else its name, else its
    /// id; a relation without one by its ends.
    pub fn select(&self, documents: &DiagramDocuments, element_id: &str) -> Option<SelectedElement> {
        let element = self
            .document(documents)
            .ok()?
            .elements()
            .into_iter()
            .find(|e| e.id == element_id)?;
        let named = ["label", "name", "title"]
            .iter()
            .map(|a| element.attribute(a))
            .find(|t| !t.is_empty())
            .map(str::to_owned);
        let text = named.unwrap_or_else(|| match (&element.source, &element.target) {
            (Some(source), Some(target)) if element.kind == ElementKind::Relation => format!("{source} → {target}"),
            _ => element.id.clone(),
        });
        Some(SelectedElement {
            path: vec![text.clone()],
            text,
            element,
        })
    }

    /// The palette.
    pub fn toolbox(&self, documents: &DiagramDocuments) -> Vec<ToolboxItem> {
        self.document(documents)
            .map(|d| d.tool().definition.toolbox().to_vec())
            .unwrap_or_default()
    }

    /// The findings of the diagram's constraints, each with its severity and where it is.
    pub fn problems(&self, documents: &DiagramDocuments) -> Vec<DiagramProblem> {
        let Ok(document) = self.document(documents) else {
            return Vec::new();
        };
        // What the reader could not read: a body that does not parse is the specification's `std.unparseable`, and a
        // body whose reader plugin the host lacks is a problem of its own.
        let reader_findings = document
            .fbl()
            .findings
            .iter()
            .filter(|f| f.code == codes::UNPARSEABLE)
            .map(|f| DislReaderFinding::not_parsed(&f.message, f.location.as_ref().map(|l| l.line as i32), None))
            .collect();
        let missing = document
            .fbl()
            .findings
            .iter()
            .filter(|f| f.code == codes::PLUGIN_MISSING)
            .map(|f| DiagramProblem {
                severity: DiagramProblemSeverity::Error,
                location: f
                    .location
                    .as_ref()
                    .map(|l| DiagramProblemLocation::Line(l.line.max(1) as u32)),
                message: f.message.clone(),
                rule_id: f.code.clone(),
            })
            .collect::<Vec<_>>();
        let options = DislConstraintOptions {
            env: Some(self.env.clone()),
            reader_findings,
            ..DislConstraintOptions::default()
        };
        let mut problems = missing;
        problems.extend(
            constraints::evaluate(
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
            }),
        );
        problems
    }

    // ---- gestures ------------------------------------------------------------------------------------------------

    /// Moves an element to a place on the canvas: the authored position stored in the registration, as one command.
    pub fn move_element_to(
        &self,
        workspace: &mut DiagramWorkspace,
        element_id: &str,
        x: f64,
        y: f64,
    ) -> Result<(), String> {
        self.move_elements(workspace, &[(element_id.to_owned(), x, y)])
    }

    /// Moves several elements at once, as one command and one undo. Each move is checked against the
    /// specification's placement constraints first; the first refusal moves nothing. Where the tool type's module
    /// has coordinates of its own, it says what a move writes.
    pub fn move_elements(&self, workspace: &mut DiagramWorkspace, moves: &[(String, f64, f64)]) -> Result<(), String> {
        let document = self.document(workspace.documents())?;
        let positions = document.positions();
        let specification = document.tool().definition.specification();
        let mut changes = Vec::with_capacity(moves.len());
        for (id, x, y) in moves {
            let element = document.element(id).ok_or_else(|| GONE.to_owned())?;
            if element.is_relation() {
                return Err(NOT_MOVABLE.to_owned());
            }
            let (old_x, old_y) = positions.get(id).copied().unwrap_or_default();
            if (old_x, old_y) == (*x, *y) && positions.contains_key(id) {
                continue;
            }
            if let Some(placed) = document
                .tool()
                .module
                .as_ref()
                .and_then(|m| m.placement(&element, *x, *y))
            {
                let placed = placed?;
                if placed.attributes.is_empty() {
                    continue;
                }
                let refusals = gestures::placement(
                    specification,
                    &element,
                    "move",
                    placed.old_bounds,
                    placed.new_bounds,
                    None,
                    Some(&self.env),
                );
                if let Some(refusal) = refusals.into_iter().next() {
                    return Err(refusal.message);
                }
                changes.push(DiagramChange::Disl(DislChange::Set {
                    element_id: element.id(),
                    type_name: element.type_name(),
                    attributes: placed.attributes,
                }));
                continue;
            }
            let refusals = gestures::placement(
                specification,
                &element,
                "move",
                bounds(old_x, old_y),
                bounds(*x, *y),
                None,
                Some(&self.env),
            );
            if let Some(refusal) = refusals.into_iter().next() {
                return Err(refusal.message);
            }
            changes.push(document.placement_change(&element, *x, *y));
        }
        let label = if moves.len() == 1 {
            "Move".to_owned()
        } else {
            format!("Move {} elements", moves.len())
        };
        let edit = document.plan(&label, &changes)?;
        workspace.execute(edit)
    }

    /// Moves an element under a new parent, at `index` among its children (negative appends), for a diagram whose
    /// elements form a tree.
    pub fn move_element(
        &self,
        workspace: &mut DiagramWorkspace,
        element_id: &str,
        new_parent_id: Option<&str>,
        index: i64,
    ) -> Result<(), String> {
        let document = self.document(workspace.documents())?;
        let element = document.element(element_id).ok_or_else(|| GONE.to_owned())?;
        let parent = match new_parent_id {
            Some(id) => Some(document.element(id).ok_or_else(|| GONE.to_owned())?),
            None => None,
        };
        let refusals = gestures::placement(
            document.tool().definition.specification(),
            &element,
            "reparent",
            Map::default(),
            Map::default(),
            parent.as_ref(),
            Some(&self.env),
        );
        if let Some(refusal) = refusals.into_iter().next() {
            return Err(refusal.message);
        }
        let change = DislChange::Reparent {
            element_id: element_id.to_owned(),
            parent_id: new_parent_id.map(str::to_owned),
            index,
        };
        let edit = document.plan("Move", &[DiagramChange::Disl(change)])?;
        workspace.execute(edit)
    }

    /// Drops toolbox tool `tool_id` on the canvas at a point: what the specification's tool creates there, its point
    /// stored in the registration in the same step.
    pub fn drop_tool(
        &self,
        workspace: &mut DiagramWorkspace,
        tool_id: &str,
        x: f64,
        y: f64,
        ids: &dyn IdSource,
    ) -> Result<(), String> {
        let document = self.document(workspace.documents())?;
        let specification = document.tool().definition.specification();
        let name = tool_name(document, tool_id).unwrap_or_else(|| tool_id.to_owned());
        let working = builder::from(document.fbl(), specification).diagram;
        let position = position_at(document, x, y);
        let transaction = operations::drop(specification, &name, &working, &position, ids, Some(&self.env));
        self.write(workspace, "Add", transaction)
    }

    // ---- properties ----------------------------------------------------------------------------------------------

    /// The property rows of the selected elements: one element's form, or for several the rows they share by id,
    /// each with the value they share, empty where they differ, and read-only where any of them is.
    pub fn properties(&self, documents: &DiagramDocuments, element_ids: &[&str]) -> Vec<PropertyRow> {
        let Ok(document) = self.document(documents) else {
            return Vec::new();
        };
        let definition = &document.tool().definition;
        let mut shared: Option<Vec<PropertyRow>> = None;
        for id in element_ids {
            let Some(element) = document.element(id) else {
                return Vec::new();
            };
            let rows = definition.rows(&element, &self.env);
            shared = Some(match shared {
                None => rows,
                Some(previous) => previous
                    .into_iter()
                    .filter_map(|mut row| {
                        let other = rows.iter().find(|r| r.id == row.id)?;
                        if other.value != row.value {
                            row.value = String::new();
                        }
                        if row.read_only_reason.is_empty() {
                            row.read_only_reason = other.read_only_reason.clone();
                        }
                        Some(row)
                    })
                    .collect(),
            });
        }
        shared.unwrap_or_default()
    }

    /// Sets property `property_id` of every selected element to `value`, as one command and one undo; a value
    /// every element already holds writes nothing, and the first refusal, the specification's or the tool type's
    /// module's, writes nothing at all.
    pub fn set_property(
        &self,
        workspace: &mut DiagramWorkspace,
        element_ids: &[&str],
        property_id: &str,
        value: &str,
    ) -> Result<(), String> {
        let document = self.document(workspace.documents())?;
        let definition = &document.tool().definition;
        let row = definition.row_key(property_id);
        let specification = definition.specification();
        let mut changes = Vec::new();
        for id in element_ids {
            let element = document.element(id).ok_or_else(|| GONE.to_owned())?;
            if let Some(shown) = definition
                .rows(&element, &self.env)
                .iter()
                .find(|r| r.id == property_id)
            {
                shown.validate(value)?;
            }
            let transaction = form_commit::row(specification, &element, &row, value, &self.env, "inspector");
            if let Some(refusal) = transaction.refusal {
                return Err(refusal);
            }
            let written = match &document.tool().module {
                Some(module) => module.written(&document.model().diagram, transaction.changes)?,
                None => transaction.changes,
            };
            changes.extend(written.into_iter().map(DiagramChange::Disl));
        }
        let label = if element_ids.len() == 1 {
            "Change".to_owned()
        } else {
            format!("Change {} elements", element_ids.len())
        };
        let edit = document.plan(&label, &changes)?;
        workspace.execute(edit)
    }

    // ---- actions -------------------------------------------------------------------------------------------------

    fn menu_target(document: &DiagramDocument, target: &ActionTarget) -> Option<DislMenuTarget> {
        let diagram = document.model().diagram.clone();
        Some(match target {
            ActionTarget::Element(id) => DislMenuTarget::Element(document.element(id)?),
            ActionTarget::Canvas { .. } => DislMenuTarget::Canvas(diagram),
            ActionTarget::Connection { source, target } => DislMenuTarget::Connection {
                source: source.as_deref().and_then(|id| document.element(id)),
                target: target.as_deref().and_then(|id| document.element(id)),
                diagram,
                relation_type: None,
            },
        })
    }

    /// The context menu of `target`, as the shell shows it.
    pub fn actions(&self, documents: &DiagramDocuments, target: &ActionTarget) -> Vec<ActionGroup> {
        let Ok(document) = self.document(documents) else {
            return Vec::new();
        };
        match Self::menu_target(document, target) {
            Some(menu) => document.tool().definition.menus(&menu, &self.env),
            None => Vec::new(),
        }
    }

    fn entry(&self, document: &DiagramDocument, target: &ActionTarget, action_id: &str) -> Option<DerivedMenuEntry> {
        let menu = Self::menu_target(document, target)?;
        document
            .tool()
            .definition
            .menu_entries(&menu, &self.env)
            .into_iter()
            .flatten()
            .find(|e| e.id == action_id)
    }

    /// Runs action `action_id` of `target`'s menu: an operation is run by the specification's behaviour and written
    /// as one command, a deletion asks its confirmation first when the specification states one. Executing an action
    /// only finds the actions its target offers.
    pub fn execute(
        &self,
        workspace: &mut DiagramWorkspace,
        target: &ActionTarget,
        action_id: &str,
        ids: &dyn IdSource,
    ) -> ActionOutcome {
        if let Some(module) = self.module(workspace.documents())
            && let Some(outcome) = module.execute(self, workspace, target, action_id, ids)
        {
            return outcome;
        }
        let document = match self.document(workspace.documents()) {
            Ok(document) => document,
            Err(refusal) => return ActionOutcome::Refused(refusal),
        };
        if let Some(refusal) = document.refusal() {
            return ActionOutcome::Refused(refusal);
        }
        let Some(entry) = self.entry(document, target, action_id) else {
            return ActionOutcome::Refused(format!("'{action_id}' does not apply to this selection."));
        };
        if !entry.available {
            return ActionOutcome::Refused(entry.unavailable_reason);
        }
        let specification = document.tool().definition.specification();
        match (entry.kind.as_str(), target) {
            ("delete", ActionTarget::Element(id)) => {
                let Some(element) = document.element(id) else {
                    return ActionOutcome::Refused(GONE.to_owned());
                };
                match deletion::confirmation(specification, &element, None, Some(&self.env)) {
                    Some(confirmation) => ActionOutcome::Prompt(Prompt::Confirm {
                        title: confirmation.title,
                        icon: "mdi-delete-outline".into(),
                        message: confirmation.message,
                        confirm_label: confirmation.confirm_label,
                        danger: confirmation.danger,
                    }),
                    None => self.remove(workspace, id),
                }
            }
            ("editLabel", ActionTarget::Element(id)) => {
                let Some(element) = document.element(id) else {
                    return ActionOutcome::Refused(GONE.to_owned());
                };
                let label = label_id(document, &element, &entry);
                ActionOutcome::Prompt(Prompt::Input {
                    title: entry.label.clone(),
                    icon: entry.icon.clone(),
                    label: "Label".into(),
                    initial_value: form_commit::edit_text(specification, &element, &label, &self.env)
                        .unwrap_or_default(),
                    confirm_label: entry.label.clone(),
                })
            }
            ("connect", ActionTarget::Connection { source, target }) => {
                let (Some(source), Some(target)) = (source, target) else {
                    return ActionOutcome::Refused(format!(
                        "'{}' needs both ends: a relation released on empty canvas has no point here to create its other end at.",
                        entry.label
                    ));
                };
                let Some(via) = connect_via(specification) else {
                    return ActionOutcome::Refused(format!("'{}' names no relation type.", entry.label));
                };
                match self.connect(workspace, &via, source, target, ids) {
                    Ok(()) => ActionOutcome::Done,
                    Err(refusal) => ActionOutcome::Refused(refusal),
                }
            }
            (_, _) if entry.operation.is_some() => {
                let operation = entry.operation.clone().unwrap_or_default();
                let working = builder::from(document.fbl(), specification).diagram;
                let element = match target {
                    ActionTarget::Element(id) => working.element_by_id(id),
                    _ => None,
                };
                let parameters =
                    operations::parameters(specification, &operation, &working, element.as_ref(), Some(&self.env));
                match parameters.as_slice() {
                    [] => {}
                    [parameter] => {
                        let title = entry.label.trim_end_matches(['\u{2026}', '.']).trim_end().to_owned();
                        return ActionOutcome::Prompt(Prompt::Input {
                            title: title.clone(),
                            icon: entry.icon.clone(),
                            label: parameter.label.clone(),
                            initial_value: crate::document::text_of(&parameter.default),
                            confirm_label: title,
                        });
                    }
                    _ => {
                        return ActionOutcome::Refused(format!(
                            "'{}' asks for more than one value, which this host cannot ask for yet.",
                            entry.label
                        ));
                    }
                }
                match self.run_operation(workspace, target, &entry, &operation, None, ids) {
                    Ok(()) => ActionOutcome::Done,
                    Err(refusal) => ActionOutcome::Refused(refusal),
                }
            }
            _ => ActionOutcome::Refused(format!("'{}' cannot be run here yet.", entry.label)),
        }
    }

    /// Answers the prompt action `action_id` of `target` raised: the deletion it asked to confirm, the label typed
    /// for an inline rename, or the value an operation asked for (its one parameter, DISL section 9.3).
    pub fn answer(
        &self,
        workspace: &mut DiagramWorkspace,
        target: &ActionTarget,
        action_id: &str,
        value: &str,
    ) -> ActionOutcome {
        if let Some(module) = self.module(workspace.documents())
            && let Some(outcome) = module.answer(self, workspace, target, action_id, value)
        {
            return outcome;
        }
        let Ok(document) = self.document(workspace.documents()) else {
            return ActionOutcome::Refused(NOT_OPEN.to_owned());
        };
        let entry = self.entry(document, target, action_id);
        match (entry, target) {
            (Some(entry), ActionTarget::Element(id)) if entry.kind == "delete" => self.remove(workspace, id),
            (Some(entry), _) if entry.operation.is_some() => {
                let operation = entry.operation.clone().unwrap_or_default();
                let specification = document.tool().definition.specification();
                let working = builder::from(document.fbl(), specification).diagram;
                let element = match target {
                    ActionTarget::Element(id) => working.element_by_id(id),
                    _ => None,
                };
                let parameters =
                    operations::parameters(specification, &operation, &working, element.as_ref(), Some(&self.env));
                let [parameter] = parameters.as_slice() else {
                    return ActionOutcome::Refused(format!("'{action_id}' asks for nothing."));
                };
                let Some(answered) = parameter.value_of(value) else {
                    return ActionOutcome::Refused(format!("'{value}' is not a value of {}.", parameter.label));
                };
                let parameters: Map = [(parameter.name.clone(), answered)].into_iter().collect();
                // An answer carries no ids: an operation that asks first creates nothing it would need one for.
                let ids = adp_disl::behavior::ids::fixed(&[]);
                match self.run_operation(workspace, target, &entry, &operation, Some(parameters), &ids) {
                    Ok(()) => ActionOutcome::Done,
                    Err(refusal) => ActionOutcome::Refused(refusal),
                }
            }
            (Some(entry), ActionTarget::Element(id)) if entry.kind == "editLabel" => {
                let label = match document.element(id) {
                    Some(element) => label_id(document, &element, &entry),
                    None => return ActionOutcome::Refused(GONE.to_owned()),
                };
                match self.edit_label(workspace, id, &label, value) {
                    Ok(()) => ActionOutcome::Done,
                    Err(refusal) => ActionOutcome::Refused(refusal),
                }
            }
            _ => ActionOutcome::Refused(format!("'{action_id}' does not apply to this selection.")),
        }
    }

    /// The text the inline editor of `element_id`'s label `label` opens on, or none when it has no such label.
    pub fn label_text(&self, documents: &DiagramDocuments, element_id: &str, label: &str) -> Option<String> {
        let document = self.document(documents).ok()?;
        let element = document.element(element_id)?;
        form_commit::edit_text(document.tool().definition.specification(), &element, label, &self.env)
    }

    /// Commits the inline edit `text` of `element_id`'s label `label`, as the label's `parse` writes it, else to the
    /// attribute it binds, or the one the tool type's module names for a computed label, as one command.
    pub fn edit_label(
        &self,
        workspace: &mut DiagramWorkspace,
        element_id: &str,
        label: &str,
        text: &str,
    ) -> Result<(), String> {
        let document = self.document(workspace.documents())?;
        let element = document.element(element_id).ok_or_else(|| GONE.to_owned())?;
        let specification = document.tool().definition.specification();
        let transaction = match document
            .tool()
            .module
            .as_ref()
            .and_then(|m| m.label_attribute(&element, label))
        {
            Some(attribute) => form_commit::attribute(specification, &element, &attribute, text, &self.env),
            None => form_commit::label(specification, &element, label, text, &self.env),
        };
        self.write(workspace, "Rename", transaction)
    }

    /// The connect gesture: a relation of `relation_type` from `source` to `target`, under the id `ids` gives; for a
    /// derived relation type, the operation its `edits.connect` names (DISL section 4.11.4). The specification's
    /// connect constraints are checked first (section 8.4); the first refusal draws nothing.
    pub fn connect(
        &self,
        workspace: &mut DiagramWorkspace,
        relation_type: &str,
        source: &str,
        target: &str,
        ids: &dyn IdSource,
    ) -> Result<(), String> {
        let document = self.document(workspace.documents())?;
        let specification = document.tool().definition.specification();
        let relation = specification
            .metamodel()
            .type_of(relation_type)
            .filter(|t| t.is_relation)
            .ok_or_else(|| format!("There is no relation type '{relation_type}'."))?
            .clone();
        let (Some(from), Some(to)) = (document.element(source), document.element(target)) else {
            return Err("Both ends of a relation must be elements of this diagram.".to_owned());
        };
        if let Some(refusal) = gestures::connect(specification, relation_type, &from, &to, Some(&self.env))
            .into_iter()
            .next()
        {
            return Err(refusal.message);
        }
        let transaction = if relation.derived().is_some() {
            let working = builder::from(document.fbl(), specification).diagram;
            let (Some(from), Some(to)) = (working.element_by_id(source), working.element_by_id(target)) else {
                return Err(GONE.to_owned());
            };
            operations::connect(specification, relation_type, &working, &from, &to, ids, Some(&self.env))
        } else {
            DislTransaction {
                changes: vec![DislChange::Connect {
                    type_name: relation_type.to_owned(),
                    id: ids.next(relation_type)?,
                    source_id: from.id(),
                    target_id: to.id(),
                    attributes: Map::default(),
                }],
                ..DislTransaction::default()
            }
        };
        self.write(workspace, "Connect", transaction)
    }

    /// The connect gesture released on empty canvas (DISL sections 6.10 and 7.2): the relation's missing target
    /// made at (`x`, `y`) in the diagram's units by the tool that creates it, and the relation from `source` to it, as
    /// one command; refused where no tool makes such a target.
    #[allow(clippy::too_many_arguments)]
    pub fn connect_to_new(
        &self,
        workspace: &mut DiagramWorkspace,
        relation_type: &str,
        source: &str,
        x: f64,
        y: f64,
        ids: &dyn IdSource,
    ) -> Result<(), String> {
        let document = self.document(workspace.documents())?;
        let specification = document.tool().definition.specification();
        let working = builder::from(document.fbl(), specification).diagram;
        let existing = working.element_by_id(source).ok_or_else(|| GONE.to_owned())?;
        let transaction = operations::connect_to_new(
            specification,
            relation_type,
            &working,
            &existing,
            "target",
            &bounds(x, y),
            ids,
            Some(&self.env),
        );
        self.write(workspace, "Connect", transaction)
    }

    /// The relation types a connect gesture from `source` may draw to `target`, or towards empty canvas when there
    /// is none, in the metamodel's order: those whose source end takes the source's type and whose target end takes
    /// the target's, abstract types left out.
    pub fn relation_types_between(
        &self,
        documents: &DiagramDocuments,
        source: &str,
        target: Option<&str>,
    ) -> Vec<String> {
        let Ok(document) = self.document(documents) else {
            return Vec::new();
        };
        let Some(from) = document.element(source) else {
            return Vec::new();
        };
        let to = target.and_then(|id| document.element(id));
        if target.is_some() && to.is_none() {
            return Vec::new();
        }
        let takes = |end: Option<&adp_disl::DislRelationEnd>, element: &DislElement| match end {
            None => true,
            Some(end) => {
                (end.types.is_empty() || end.types.iter().any(|t| element.is_a(t)))
                    && !end.exclude.iter().any(|t| element.is_a(t))
            }
        };
        document
            .tool()
            .definition
            .specification()
            .metamodel()
            .relations
            .values()
            .filter(|relation| !relation.is_abstract())
            .filter(|relation| takes(relation.source.as_ref(), &from))
            .filter(|relation| to.as_ref().is_none_or(|to| takes(relation.target.as_ref(), to)))
            .map(|relation| relation.name.clone())
            .collect()
    }

    /// Removes an element; the binding's cascade takes what goes with it, as the standalone host's removal
    /// commands let it.
    fn remove(&self, workspace: &mut DiagramWorkspace, element_id: &str) -> ActionOutcome {
        let result = self.document(workspace.documents()).and_then(|document| {
            document.plan(
                "Delete",
                &[DiagramChange::Disl(DislChange::Remove {
                    element_id: element_id.to_owned(),
                })],
            )
        });
        match result.and_then(|edit| workspace.execute(edit)) {
            Ok(()) => ActionOutcome::Done,
            Err(refusal) => ActionOutcome::Refused(refusal),
        }
    }

    /// Writes what a transaction made, as one command, or its refusal.
    fn write(&self, workspace: &mut DiagramWorkspace, label: &str, transaction: DislTransaction) -> Result<(), String> {
        if let Some(refusal) = transaction.refusal {
            return Err(refusal);
        }
        let document = self.document(workspace.documents())?;
        let changes = match &document.tool().module {
            Some(module) => module.written(&document.model().diagram, transaction.changes)?,
            None => transaction.changes,
        };
        let changes: Vec<DiagramChange> = changes.into_iter().map(DiagramChange::Disl).collect();
        let edit = document.plan(label, &changes)?;
        workspace.execute(edit)
    }

    /// Runs operation `operation` of `entry` on `target` with its answered `parameters`, the plugin layouts its
    /// `layout` actions name run by the tool type's module, and writes it as one command.
    fn run_operation(
        &self,
        workspace: &mut DiagramWorkspace,
        target: &ActionTarget,
        entry: &DerivedMenuEntry,
        operation: &str,
        parameters: Option<Map>,
        ids: &dyn IdSource,
    ) -> Result<(), String> {
        let document = self.document(workspace.documents())?;
        let specification = document.tool().definition.specification();
        let working = builder::from(document.fbl(), specification).diagram;
        let element = match target {
            ActionTarget::Element(id) => working.element_by_id(id),
            _ => None,
        };
        let invocation = DislInvocation {
            position: match target {
                ActionTarget::Canvas { x, y } => Some(position_at(document, *x, *y)),
                _ => None,
            },
            parameters,
            ..DislInvocation::default()
        };
        let mut transaction = operations::run(
            specification,
            operation,
            &working,
            element.as_ref(),
            ids,
            Some(&invocation),
            Some(&self.env),
        );
        if transaction.refusal.is_none()
            && let Some(module) = &document.tool().module
        {
            for action in &transaction.host_actions {
                let HostAction::Layout { algorithm } = action else {
                    continue;
                };
                let Some(plugin) = plugin_layout(specification, algorithm) else {
                    continue;
                };
                match module.layout(specification, &plugin, &working) {
                    Some(Ok(changes)) => transaction.changes.extend(changes),
                    Some(Err(refusal)) => return Err(refusal),
                    None => {}
                }
            }
        }
        self.write(workspace, &entry.label, transaction)
    }
}

/// The label an `editLabel` entry edits: its `label` argument, else the first label of the element's notation.
fn label_id(document: &DiagramDocument, element: &DislElement, entry: &DerivedMenuEntry) -> String {
    if let Some(Value::String(label)) = entry.arguments.get("label") {
        return label.clone();
    }
    let notation = document.tool().definition.specification().root().get("notation");
    ["nodes", "edges"]
        .iter()
        .filter_map(|kind| notation?.get(kind)?.get(element.type_name())?.get("labels")?.as_array())
        .flatten()
        .find_map(|label| label.get("id")?.as_str().map(str::to_owned))
        .unwrap_or_else(|| "label".to_owned())
}

/// The name of the specification's tool whose wire id is `tool_id`.
fn tool_name(document: &DiagramDocument, tool_id: &str) -> Option<String> {
    let ids = document.tool().definition.ids();
    let toolbox = document.tool().definition.specification().root().get("toolbox")?;
    toolbox
        .get("groups")?
        .as_array()?
        .iter()
        .filter_map(|g| g.get("tools")?.as_array())
        .flatten()
        .filter_map(|t| t.get("id")?.as_str())
        .find(|name| ids.tool_id(name) == tool_id)
        .map(str::to_owned)
}

/// The `position` an operation or a tool run at the canvas point (`x`, `y`) reads: the module's, else the point.
fn position_at(document: &DiagramDocument, x: f64, y: f64) -> Map {
    document
        .tool()
        .module
        .as_ref()
        .and_then(|m| m.position_at(x, y))
        .unwrap_or_else(|| bounds(x, y))
}

/// The plugin a layout algorithm runs (`layout.algorithms.<name>.algorithm` of `plugin:<plugin>`), or none for one
/// the runtime implements.
fn plugin_layout(specification: &adp_disl::DislSpecification, algorithm: &str) -> Option<String> {
    let declared = specification
        .root()
        .get("layout")?
        .get("algorithms")?
        .get(algorithm)?
        .get("algorithm")?
        .as_str()?;
    declared.strip_prefix("plugin:").map(str::to_owned)
}

/// The relation type a finished connect gesture's menu entry draws: the `via` of the `connect` entry of the menu
/// `for: "connection"`, else the one relation type that is not abstract.
fn connect_via(specification: &adp_disl::DislSpecification) -> Option<String> {
    let root = specification.root();
    let menus = root.get("toolbox")?.get("contextMenus")?.as_array()?;
    let named = menus
        .iter()
        .filter(|menu| {
            menu.get("for")
                .and_then(|f| f.as_array())
                .is_some_and(|f| f.iter().any(|t| t.as_str() == Some("connection")))
        })
        .filter_map(|menu| menu.get("tools")?.as_array())
        .flatten()
        .filter(|tool| tool.get("kind").and_then(|k| k.as_str()) == Some("connect"))
        .find_map(|tool| tool.get("via")?.as_str().map(str::to_owned));
    named.or_else(|| {
        let mut relations = specification
            .metamodel()
            .types
            .values()
            .filter(|t| t.is_relation && !t.is_abstract());
        match (relations.next(), relations.next()) {
            (Some(only), None) => Some(only.name.clone()),
            _ => None,
        }
    })
}

fn bounds(x: f64, y: f64) -> Map {
    [("x", Value::Double(x)), ("y", Value::Double(y))].into_iter().collect()
}

impl From<&DislElement> for ActionTarget {
    fn from(element: &DislElement) -> Self {
        ActionTarget::Element(element.id())
    }
}
