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
//!
//! A view of a tool type whose module reads, draws and edits its documents itself ([`DiagramSession::of_module`])
//! hands every question and gesture to the module's [`ModuleView`]; the delivery of what it shows is the session's.

use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_cel::{Map, Value};
use adp_context::{ActionGroup, Prompt, PropertyRow};
use adp_disl::behavior::{deletion, hooks, operations, write};
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
use crate::module::{DiagramModule, ModuleView};
use crate::problems::{DiagramProblem, DiagramProblemLocation, DiagramProblemSeverity};
use crate::sizing::{Resize, ResizedEdge, SizeSource};
use crate::workspace::DiagramWorkspace;

/// What a drag answers when the diagram may not be edited.
pub const READ_ONLY: &str = "This diagram is read-only.";

/// What a placement of something that has no position of its own answers.
pub const NOT_MOVABLE: &str = "Only a node can be moved; a relation is drawn between its ends.";

/// What a resize of a relation answers.
pub const NOT_RESIZABLE: &str = "Only a node can be resized; a relation is drawn between its ends.";

/// What a resize of a node whose notation keeps its size nowhere answers.
pub const NOT_SIZED: &str = "This diagram keeps no size for it, so it cannot be resized.";

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
    /// The module's own view, for a tool type whose module reads, draws and edits its documents itself.
    view: Option<Rc<dyn ModuleView>>,
    /// What the last operation handed back for the host to carry out (`select`, `editLabel`, …).
    host_actions: std::cell::RefCell<Vec<HostAction>>,
}

/// What a gesture a module's own view does not offer answers.
pub const NOT_OFFERED: &str = "This diagram does not offer that here.";

impl DiagramSession {
    /// A view of the open document whose body is at `body_path`.
    pub fn new(body_path: impl Into<PathBuf>) -> Self {
        let body_path = body_path.into();
        DiagramSession {
            changes: DiagramDocumentChangeHandler::new(body_path.clone()),
            body_path,
            viewport: DiagramViewport::UNBOUNDED,
            env: DislEnv::default(),
            view: None,
            host_actions: Default::default(),
        }
    }

    /// The view in `specification`'s default viewpoint, which `env.viewpoint` reads (DISL section 12.4): the one it
    /// marks `default`, else its first; empty for a specification without viewpoints.
    pub fn in_default_viewpoint(mut self, specification: &adp_disl::DislSpecification) -> Self {
        self.env.viewpoint = specification.default_viewpoint();
        self
    }

    /// Whether the view's viewpoint selects what it shows (DISL section 3.5): whether `specification` declares it.
    pub fn uses_viewpoint(&self, specification: &adp_disl::DislSpecification) -> bool {
        specification
            .root()
            .get("viewpoints")
            .is_some_and(|all| all.get(self.viewpoint()).is_some())
    }

    /// Shows the view in viewpoint `viewpoint` (DISL section 11), which `env.viewpoint` reads: a menu or a form that
    /// asks for one viewpoint is offered in it alone.
    pub fn set_viewpoint(&mut self, viewpoint: &str) {
        self.env.viewpoint = viewpoint.to_owned();
    }

    /// The viewpoint the view is in; empty for a specification without viewpoints.
    pub fn viewpoint(&self) -> &str {
        &self.env.viewpoint
    }

    /// A view of the document whose body is at `body_path`, read, drawn and edited by its module's `view`.
    pub fn of_module(body_path: impl Into<PathBuf>, view: Rc<dyn ModuleView>) -> Self {
        DiagramSession {
            view: Some(view),
            ..DiagramSession::new(body_path)
        }
    }

    /// The module's own view, when the module reads, draws and edits this document itself.
    pub fn module_view(&self) -> Option<&Rc<dyn ModuleView>> {
        self.view.as_ref()
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
        if let Some(view) = &self.view {
            return view.render(documents, self.viewport);
        }
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
        if let Some(view) = &self.view {
            let (path, text) = view.describe(documents, element_id)?;
            let element = view
                .render(documents, DiagramViewport::UNBOUNDED)
                .ok()?
                .into_iter()
                .find(|e| e.id == element_id)
                .unwrap_or_else(|| DiagramElement::node(element_id, ""));
            return Some(SelectedElement { path, text, element });
        }
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
        if let Some(view) = &self.view {
            return view.toolbox();
        }
        self.document(documents)
            .map(|d| d.tool().definition.toolbox().to_vec())
            .unwrap_or_default()
    }

    /// The findings of the diagram's constraints, each with its severity and where it is.
    pub fn problems(&self, documents: &DiagramDocuments) -> Vec<DiagramProblem> {
        if let Some(view) = &self.view {
            return view.problems(documents);
        }
        let Ok(document) = self.document(documents) else {
            return Vec::new();
        };
        // `diagram.file` names the file the document is opened through (DISL section 12.2): its registration, else
        // its body.
        let file = document.registration_path().unwrap_or(document.body_path());
        document
            .model()
            .diagram
            .set_file(file.file_name().and_then(|n| n.to_str()));
        // What the reader could not read: a body that does not parse is the specification's `std.unparseable`, and a
        // body whose reader plugin the host lacks is a problem of its own.
        let mut reader_findings: Vec<DislReaderFinding> = document
            .fbl()
            .findings
            .iter()
            .filter(|f| f.code == codes::UNPARSEABLE)
            .map(|f| DislReaderFinding::not_parsed(&f.message, f.location.as_ref().map(|l| l.line as i32), None))
            .collect();
        if let Some(module) = &document.tool().module {
            reader_findings.extend(module.reader_findings(document));
        }
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
        if let Some(found) = document
            .tool()
            .module
            .as_ref()
            .and_then(|m| m.problems(document, options.clone()))
        {
            problems.extend(found);
            return problems;
        }
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
        if let Some(view) = &self.view {
            return view.move_elements(workspace, moves);
        }
        let document = self.document(workspace.documents())?;
        if let Some(edit) = document
            .tool()
            .module
            .as_ref()
            .and_then(|m| m.move_elements(document, moves))
        {
            return workspace.execute(edit?);
        }
        let positions = document.positions();
        let specification = document.tool().definition.specification();
        let mut changes = Vec::with_capacity(moves.len());
        for (id, x, y) in moves {
            let element = document.element(id).ok_or_else(|| GONE.to_owned())?;
            if element.is_relation() {
                // What the description says of moving one, else the host's own sentence (DISL section 8.4).
                let bounds = Map::default;
                let stated = gestures::placement(
                    specification,
                    &element,
                    "move",
                    bounds(),
                    bounds(),
                    None,
                    Some(&self.env),
                );
                return Err(stated
                    .into_iter()
                    .next()
                    .map_or_else(|| NOT_MOVABLE.to_owned(), |r| r.message));
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
            // A move does not change what contains the element (DISL section 8.8), so its new parent is its parent.
            let refusals = gestures::placement(
                specification,
                &element,
                "move",
                bounds(old_x, old_y),
                bounds(*x, *y),
                element.parent().as_ref(),
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

    /// Resizes a node by one of its edges, as one command and one undo (DISL sections 5.8 and 6.9): the dragged
    /// axis's size written where its notation keeps it, and, for a left or a top edge, the position the far edge
    /// keeps the node at, in the same step. The specification's placement constraints judge the gesture `resize`
    /// first; a node whose size is kept nowhere is refused with [`NOT_SIZED`]. Where the tool type's module has
    /// coordinates of its own, it says what a resize writes.
    pub fn resize_element(
        &self,
        workspace: &mut DiagramWorkspace,
        element_id: &str,
        resize: &Resize,
    ) -> Result<(), String> {
        let document = self.document(workspace.documents())?;
        if let Some(refusal) = document.refusal() {
            return Err(refusal);
        }
        let element = document.element(element_id).ok_or_else(|| GONE.to_owned())?;
        if element.is_relation() {
            return Err(NOT_RESIZABLE.to_owned());
        }
        let specification = document.tool().definition.specification();
        let judged = |old_bounds: Map, new_bounds: Map| -> Result<(), String> {
            match gestures::placement(
                specification,
                &element,
                "resize",
                old_bounds,
                new_bounds,
                None,
                Some(&self.env),
            )
            .into_iter()
            .next()
            {
                Some(refusal) => Err(refusal.message),
                None => Ok(()),
            }
        };
        if let Some(placed) = document.tool().module.as_ref().and_then(|m| m.resize(&element, resize)) {
            let placed = placed?;
            if placed.attributes.is_empty() {
                return Ok(());
            }
            judged(placed.old_bounds, placed.new_bounds)?;
            let change = DiagramChange::Disl(DislChange::Set {
                element_id: element.id(),
                type_name: element.type_name(),
                attributes: placed.attributes,
            });
            let edit = document.plan("Resize", &[change])?;
            return workspace.execute(edit);
        }

        // A module that places the node by coordinates of its own and says nothing of its size keeps it nowhere the
        // session can write as a number.
        if document
            .tool()
            .module
            .as_ref()
            .is_some_and(|m| m.position(&element).is_some())
        {
            return Err(NOT_SIZED.to_owned());
        }
        let sources = document.size_sources(&element);
        let horizontal = resize.edge.is_horizontal();
        let Some(source) = (if horizontal { &sources.width } else { &sources.height }) else {
            return Err(NOT_SIZED.to_owned());
        };
        let (near, far, extent) = if horizontal {
            (resize.x, resize.right, resize.width)
        } else {
            (resize.y, resize.bottom, resize.height)
        };
        let written = match source {
            SizeSource::Size(_) => extent,
            SizeSource::Far(_) => far,
            SizeSource::Span(_) => far - near,
        };
        let attribute = source.attribute();
        let mut attributes = Map::default();
        let was = element.value_of(attribute).as_number();
        let value = document.typed(&element, attribute, written);
        if was != value.as_number() {
            attributes.insert(attribute, value);
        }
        let positions = document.positions();
        let (old_x, old_y) = positions.get(element_id).copied().unwrap_or_default();
        let moved =
            matches!(resize.edge, ResizedEdge::Left | ResizedEdge::Top) && (old_x, old_y) != (resize.x, resize.y);
        if attributes.is_empty() && !moved {
            return Ok(());
        }
        // The bounds the constraints judge: the far edge only where the length is in the axis's units.
        let in_axis_units = !matches!(source, SizeSource::Size(_));
        let extent_of = |x: f64, y: f64, length: Option<f64>| -> Map {
            let mut map = bounds(x, y);
            if let Some(length) = length {
                let (far, size) = if horizontal { ("x2", "width") } else { ("y2", "height") };
                let start = if horizontal { x } else { y };
                if in_axis_units {
                    map.insert(far, Value::Double(start + length));
                }
                map.insert(size, Value::Double(length));
            }
            map
        };
        let old_length = was.map(|was| match source {
            SizeSource::Far(_) => was - if horizontal { old_x } else { old_y },
            _ => was,
        });
        judged(
            extent_of(old_x, old_y, old_length),
            extent_of(
                resize.x,
                resize.y,
                Some(
                    written
                        - if matches!(source, SizeSource::Far(_)) {
                            near
                        } else {
                            0.0
                        },
                ),
            ),
        )?;
        let mut changes = Vec::new();
        if moved {
            changes.push(document.placement_change(&element, resize.x, resize.y));
        }
        if !attributes.is_empty() {
            changes.push(DiagramChange::Disl(DislChange::Set {
                element_id: element.id(),
                type_name: element.type_name(),
                attributes,
            }));
        }
        let edit = document.plan("Resize", &changes)?;
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
        if let Some(view) = &self.view {
            return view.move_element(workspace, element_id, new_parent_id, index);
        }
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

    /// Whether a drag of `element_id` proposes a new parent for it rather than a place
    /// ([`DiagramDocument::drag_reparents`]).
    pub fn drag_reparents(&self, documents: &DiagramDocuments, element_id: &str) -> bool {
        self.view.is_none()
            && self
                .document(documents)
                .ok()
                .and_then(|d| Some((d, d.element(element_id)?)))
                .is_some_and(|(document, element)| document.drag_reparents(&element))
    }

    /// Makes each element of `moves` the last child of the parent it names, as one command and one undo; the first
    /// refusal of the specification's containment constraints moves none of them (`moveElement`, a drag dropped on a
    /// node).
    pub fn reparent_elements(
        &self,
        workspace: &mut DiagramWorkspace,
        moves: &[(String, String)],
    ) -> Result<(), String> {
        if let Some(view) = &self.view {
            for (id, parent) in moves {
                view.move_element(workspace, id, Some(parent), -1)?;
            }
            return Ok(());
        }
        let document = self.document(workspace.documents())?;
        let specification = document.tool().definition.specification();
        let mut changes = Vec::with_capacity(moves.len());
        for (id, parent_id) in moves {
            let element = document.element(id).ok_or_else(|| GONE.to_owned())?;
            let parent = document.element(parent_id).ok_or_else(|| GONE.to_owned())?;
            let refusals = gestures::placement(
                specification,
                &element,
                "reparent",
                Map::default(),
                Map::default(),
                Some(&parent),
                Some(&self.env),
            );
            if let Some(refusal) = refusals.into_iter().next() {
                return Err(refusal.message);
            }
            changes.push(DiagramChange::Disl(DislChange::Reparent {
                element_id: id.clone(),
                parent_id: Some(parent_id.clone()),
                index: -1,
            }));
        }
        if changes.is_empty() {
            return Ok(());
        }
        let label = if changes.len() == 1 {
            "Move".to_owned()
        } else {
            format!("Move {} elements", changes.len())
        };
        let edit = document.plan(&label, &changes)?;
        workspace.execute(edit)
    }

    /// Whether action `action_id` of `target`'s menu is its deletion (an entry of kind `delete`).
    pub fn is_deletion(&self, documents: &DiagramDocuments, target: &ActionTarget, action_id: &str) -> bool {
        self.view.is_none()
            && self
                .document(documents)
                .ok()
                .and_then(|document| self.entry(document, target, action_id))
                .is_some_and(|entry| entry.kind == "delete")
    }

    /// Deletes every element of `element_ids` as one command and one undo, or none of them (the multi-select
    /// specification's Requirements 6.1, 6.3 and 7.3): each must offer its deletion `action_id`, and one inside
    /// another that goes goes with it. Unless `confirmed`, it asks first when the specification would ask before
    /// deleting any of them.
    pub fn delete_elements(
        &self,
        workspace: &mut DiagramWorkspace,
        element_ids: &[String],
        action_id: &str,
        confirmed: bool,
    ) -> ActionOutcome {
        if self.view.is_some() {
            return ActionOutcome::Refused(NOT_OFFERED.to_owned());
        }
        let document = match self.document(workspace.documents()) {
            Ok(document) => document,
            Err(refusal) => return ActionOutcome::Refused(refusal),
        };
        if let Some(refusal) = document.refusal() {
            return ActionOutcome::Refused(refusal);
        }
        let specification = document.tool().definition.specification();
        let mut elements = Vec::with_capacity(element_ids.len());
        for id in element_ids {
            let Some(element) = document.element(id) else {
                return ActionOutcome::Refused(GONE.to_owned());
            };
            match self.entry(document, &ActionTarget::Element(id.clone()), action_id) {
                Some(entry) if entry.kind == "delete" && entry.available => {}
                Some(entry) if !entry.available => return ActionOutcome::Refused(entry.unavailable_reason),
                _ => {
                    let name = self
                        .select(workspace.documents(), id)
                        .map(|s| s.text)
                        .unwrap_or_else(|| id.clone());
                    return ActionOutcome::Refused(format!("'{name}' cannot be deleted, so none of the selection is."));
                }
            }
            if let Some(refusal) = gestures::delete(specification, &element, Some(&self.env))
                .into_iter()
                .next()
            {
                return ActionOutcome::Refused(refusal.message);
            }
            elements.push(element);
        }
        // What goes with an ancestor that goes is not removed a second time.
        let removed: Vec<&DislElement> = elements
            .iter()
            .filter(|element| {
                !element
                    .ancestors()
                    .iter()
                    .any(|ancestor| element_ids.contains(&ancestor.id()))
            })
            .collect();
        if !confirmed
            && removed
                .iter()
                .any(|element| deletion::confirmation(specification, element, None, Some(&self.env)).is_some())
        {
            let count = removed.len();
            return ActionOutcome::Prompt(Prompt::Confirm {
                title: format!("Delete {count} elements?"),
                icon: "mdi-delete-outline".into(),
                message: format!("Delete the {count} selected elements and everything under them? You can undo this."),
                confirm_label: "Delete".into(),
                danger: true,
            });
        }
        let changes: Vec<DiagramChange> = removed
            .iter()
            .map(|element| {
                DiagramChange::Disl(DislChange::Remove {
                    element_id: element.id(),
                })
            })
            .collect();
        let label = if changes.len() == 1 {
            "Delete".to_owned()
        } else {
            format!("Delete {} elements", changes.len())
        };
        match document.plan(&label, &changes).and_then(|edit| workspace.execute(edit)) {
            Ok(()) => ActionOutcome::Done,
            Err(refusal) => ActionOutcome::Refused(refusal),
        }
    }

    /// Drops toolbox tool `tool_id` on the element `element_id`: a tool that runs an operation runs it on the element
    /// it lands on, as the entry of the element's menu that runs that operation (`onElementDropped`: the mind map's
    /// Node adds a child under the node it is dropped on). The action run and its outcome; none for a tool that
    /// creates what it drops, or a view its module draws.
    pub fn drop_tool_on(
        &self,
        workspace: &mut DiagramWorkspace,
        tool_id: &str,
        element_id: &str,
        ids: &dyn IdSource,
    ) -> Option<(String, ActionOutcome)> {
        let operation = self.tool_operation(workspace.documents(), tool_id)?;
        let document = self.document(workspace.documents()).ok()?;
        let target = ActionTarget::Element(element_id.to_owned());
        document.bind_views();
        let menu = Self::menu_target(document, &target)?;
        let Some(entry) = document
            .tool()
            .definition
            .menu_entries(&menu, &self.env)
            .into_iter()
            .flatten()
            .find(|e| e.operation.as_deref() == Some(operation.as_str()))
        else {
            return Some((
                String::new(),
                ActionOutcome::Refused(format!("'{tool_id}' does not apply there.")),
            ));
        };
        let outcome = self.execute(workspace, &target, &entry.id, ids);
        Some((entry.id, outcome))
    }

    /// The operation toolbox tool `tool_id` runs, for a tool of kind `operation`; none for any other.
    pub fn tool_operation(&self, documents: &DiagramDocuments, tool_id: &str) -> Option<String> {
        if self.view.is_some() {
            return None;
        }
        let document = self.document(documents).ok()?;
        let name = tool_name(document, tool_id).unwrap_or_else(|| tool_id.to_owned());
        let toolbox = document.tool().definition.specification().root().get("toolbox")?;
        let tool = toolbox
            .get("groups")?
            .as_array()?
            .iter()
            .filter_map(|g| g.get("tools")?.as_array())
            .flatten()
            .find(|t| t.get("id").and_then(|i| i.as_str()) == Some(name.as_str()))?;
        if tool.get("kind").and_then(|k| k.as_str()) != Some("operation") {
            return None;
        }
        Some(tool.get("operation")?.as_str()?.to_owned())
    }

    /// Drops toolbox tool `tool_id` on the canvas at a point: what the specification's tool creates there, its point
    /// stored in the registration in the same step; or, where the tool type's module runs the action the tool's drop
    /// names itself, that action on the canvas at the point (the agent behavior model's drop under the nearest node
    /// above, `AbmContextActionProvider.PlaceDrop`), whose question for the new node's name, if it asks one, is left
    /// unanswered.
    pub fn drop_tool(
        &self,
        workspace: &mut DiagramWorkspace,
        tool_id: &str,
        x: f64,
        y: f64,
        ids: &dyn IdSource,
    ) -> Result<(), String> {
        if self.view.is_some() {
            return Err(NOT_OFFERED.to_owned());
        }
        let document = self.document(workspace.documents())?;
        let dropped = document
            .tool()
            .definition
            .toolbox()
            .iter()
            .find(|item| item.id == tool_id && !item.drop_action_id.is_empty())
            .map(|item| item.drop_action_id.clone());
        if let (Some(action), Some(module)) = (dropped, self.module(workspace.documents()))
            && let Some(outcome) = module.execute(self, workspace, &ActionTarget::Canvas { x, y }, &action, ids)
        {
            return match outcome {
                ActionOutcome::Refused(refusal) => Err(refusal),
                ActionOutcome::Done | ActionOutcome::Prompt(_) => Ok(()),
            };
        }
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
        let rows_of = |id: &str| -> Option<Vec<PropertyRow>> {
            if let Some(view) = &self.view {
                return Some(view.properties(documents, id));
            }
            let document = self.document(documents).ok()?;
            let element = document.element(id)?;
            Some(document.tool().definition.rows(&element, &self.env))
        };
        let mut shared: Option<Vec<PropertyRow>> = None;
        for id in element_ids {
            let Some(rows) = rows_of(id) else {
                return Vec::new();
            };
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
        if let Some(view) = &self.view {
            return view.set_property(workspace, element_ids, property_id, value);
        }
        let document = self.document(workspace.documents())?;
        let definition = &document.tool().definition;
        let row = definition.row_key(property_id);
        let specification = definition.specification();
        // The form commit runs on a copy of the reading: it assigns as it goes, and the reading is shared by every
        // document that holds the same bytes.
        let working = builder::from(document.fbl(), specification).diagram;
        let mut changes = Vec::new();
        for id in element_ids {
            if let Some(written) = document
                .tool()
                .module
                .as_ref()
                .and_then(|m| m.set_property(document, id, property_id, value))
            {
                changes.extend(written?);
                continue;
            }
            let element = working.element_by_id(id).ok_or_else(|| GONE.to_owned())?;
            if let Some(shown) = definition
                .rows(&element, &self.env)
                .iter()
                .find(|r| r.id == property_id)
            {
                // A value set to what the row shows is no change, so nothing is written: a label `yes` stays unquoted
                // and a row written `"3"` keeps its quotes, as the standalone host's handlers left them.
                if shown.value == value {
                    continue;
                }
                // A row with a `parse` also takes a value as the document writes it, which its `accepts` judges
                // below rather than its options (DISL 0.3, 6.12), as the standalone host's form commit did.
                if !form_commit::parses(specification, &element, &row, "inspector") {
                    shown.validate(value)?;
                }
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

    /// The property rows of the diagram itself (etalii.adp spec 074, FR-025), which an empty selection shows; none for a
    /// tool type whose specification gives the diagram no form, or a module's own view.
    pub fn diagram_properties(&self, documents: &DiagramDocuments) -> Vec<PropertyRow> {
        if self.view.is_some() {
            return Vec::new();
        }
        let Ok(document) = self.document(documents) else {
            return Vec::new();
        };
        document
            .tool()
            .definition
            .diagram_rows(&document.model().diagram, &self.env)
    }

    /// Sets property `property_id` of the diagram itself to `value`, as one command and one undo; a value the row
    /// already shows writes nothing.
    pub fn set_diagram_property(
        &self,
        workspace: &mut DiagramWorkspace,
        property_id: &str,
        value: &str,
    ) -> Result<(), String> {
        let document = self.document(workspace.documents())?;
        let definition = &document.tool().definition;
        let specification = definition.specification();
        let row = definition.row_key(property_id);
        let working = builder::from(document.fbl(), specification).diagram;
        if definition
            .diagram_rows(&working, &self.env)
            .iter()
            .any(|r| r.id == property_id && r.value == value)
        {
            return Ok(());
        }
        let assigned = form_commit::diagram_row(specification, &working, &row, value, &self.env, "inspector")?;
        let mut changes = Vec::new();
        for (attribute, written) in &assigned {
            changes.extend(
                write::diagram_set(specification, document.fbl(), attribute, written)?
                    .into_iter()
                    .map(DiagramChange::Body),
            );
        }
        let edit = document.plan("Change", &changes)?;
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
        if let Some(view) = &self.view {
            return view.actions(documents, target);
        }
        let Ok(document) = self.document(documents) else {
            return Vec::new();
        };
        // What `self.view` reads is the viewer's: a fold's Collapse or Expand follows it.
        document.bind_views();
        match Self::menu_target(document, target) {
            Some(menu) => document.tool().definition.menus(&menu, &self.env),
            None => Vec::new(),
        }
    }

    /// The action a double-click on the element `element_id` runs, as [`DiagramSession::execute`] takes it: the
    /// standalone host's `activate` gesture (`compileNotation.ts`). The element's node names the entry in its
    /// `doubleClick` (DISL section 6.9; `editLabel` when it names none): the entry of that kind in its context menu,
    /// else, for an operation of the specification's behaviour (section 9.3), the entry that runs it. None when its
    /// menu has no such entry, `none` among them, and for a view whose module declares no double-click.
    pub fn double_click(&self, documents: &DiagramDocuments, element_id: &str) -> Option<String> {
        if let Some(view) = &self.view {
            return view.double_click(documents, element_id);
        }
        let document = self.document(documents).ok()?;
        let element = document.element(element_id)?;
        let specification = document.tool().definition.specification();
        let root = specification.root();
        let named = root
            .get("notation")
            .and_then(|n| n.get("nodes")?.get(element.type_name())?.get("doubleClick")?.as_str())
            .unwrap_or("editLabel")
            .to_owned();
        let operation = root
            .get("behavior")
            .and_then(|b| b.get("operations")?.get(named.as_str()))
            .is_some();
        document.bind_views();
        let menu = Self::menu_target(document, &ActionTarget::Element(element_id.to_owned()))?;
        let entries: Vec<DerivedMenuEntry> = document
            .tool()
            .definition
            .menu_entries(&menu, &self.env)
            .into_iter()
            .flatten()
            .collect();
        entries
            .iter()
            .find(|e| e.kind == named)
            .or_else(|| {
                entries
                    .iter()
                    .find(|e| operation && e.operation.as_deref() == Some(named.as_str()))
            })
            .map(|e| e.id.clone())
    }

    fn entry(&self, document: &DiagramDocument, target: &ActionTarget, action_id: &str) -> Option<DerivedMenuEntry> {
        document.bind_views();
        let menu = Self::menu_target(document, target)?;
        let definition = &document.tool().definition;
        definition
            .menu_entries(&menu, &self.env)
            .into_iter()
            .flatten()
            .find(|e| e.id == action_id)
            .or_else(|| definition.operation_entry(&menu, &self.env, action_id))
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
        if let Some(view) = &self.view {
            return view.execute_with_ids(workspace, target, action_id, ids);
        }
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
                if let Some(refusal) = gestures::delete(specification, &element, Some(&self.env))
                    .into_iter()
                    .next()
                {
                    return ActionOutcome::Refused(refusal.message);
                }
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
            // A connect entry that picks its other end (`target: "pick"`, DISL section 7.3) asks which.
            ("connect", ActionTarget::Element(_)) if entry.via.is_some() => {
                let title = entry.label.trim_end_matches(['\u{2026}', '.']).trim_end().to_owned();
                ActionOutcome::Prompt(Prompt::Input {
                    title: title.clone(),
                    icon: entry.icon.clone(),
                    label: title.clone(),
                    initial_value: String::new(),
                    confirm_label: title,
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
                // An entry hands the operation what its `args` state (DISL section 7.3), a pending connection its ends
                // among them (`"args": {"from": "source", "to": "target"}`); only what they leave is asked.
                let given = given_arguments(&parameters, &entry);
                let asked: Vec<_> = parameters.iter().filter(|p| !given.contains_key(&p.name)).collect();
                if !given.is_empty() && asked.is_empty() {
                    return match self.run_operation(workspace, target, &entry, &operation, Some(given), ids) {
                        Ok(()) => ActionOutcome::Done,
                        Err(refusal) => ActionOutcome::Refused(refusal),
                    };
                }
                match asked.as_slice() {
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

    /// Answers the prompt as [`DiagramSession::answer`] does, where what a module's own view creates in answer takes
    /// the ids `ids` hands out (a supply chain's group put around a node); the specification's answers take their own.
    pub fn answer_with_ids(
        &self,
        workspace: &mut DiagramWorkspace,
        target: &ActionTarget,
        action_id: &str,
        value: &str,
        ids: &dyn IdSource,
    ) -> ActionOutcome {
        match &self.view {
            Some(view) => view.answer_with_ids(workspace, target, action_id, value, ids),
            None => self.answer_from(workspace, target, action_id, value, Some(ids)),
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
        self.answer_from(workspace, target, action_id, value, None)
    }

    /// [`DiagramSession::answer`], what an operation that asks first creates taking the ids `given`, when there are any.
    fn answer_from(
        &self,
        workspace: &mut DiagramWorkspace,
        target: &ActionTarget,
        action_id: &str,
        value: &str,
        given: Option<&dyn IdSource>,
    ) -> ActionOutcome {
        if let Some(view) = &self.view {
            return view.answer(workspace, target, action_id, value);
        }
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
            (Some(entry), ActionTarget::Element(id)) if entry.kind == "connect" && entry.via.is_some() => {
                let via = entry.via.clone().unwrap_or_default();
                let specification = document.tool().definition.specification();
                let minted = adp_disl::behavior::ids::of(specification);
                let stand_ins = StandInIds::default();
                let ids: &dyn adp_disl::IdSource = match (&minted, given) {
                    (_, Some(given)) => given,
                    (Ok(minted), None) => minted,
                    (Err(_), None) => &stand_ins,
                };
                match self.connect(workspace, &via, id, value, ids) {
                    Ok(()) => ActionOutcome::Done,
                    Err(refusal) => ActionOutcome::Refused(refusal),
                }
            }
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
                let handed = given_arguments(&parameters, &entry);
                let asked: Vec<_> = parameters.iter().filter(|p| !handed.contains_key(&p.name)).collect();
                let [parameter] = asked.as_slice() else {
                    return ActionOutcome::Refused(format!("'{action_id}' asks for nothing."));
                };
                let Some(answered) = parameter.value_of(value) else {
                    return ActionOutcome::Refused(format!("'{value}' is not a value of {}.", parameter.label));
                };
                let parameters: Map = handed
                    .iter()
                    .map(|(name, value)| (name.clone(), value.clone()))
                    .chain([(parameter.name.clone(), answered)])
                    .collect();
                // An answer carries no ids unless a replay gives them: what an operation that asks first creates
                // takes the specification's own strategy, else, for a `natural` one whose ids the binding reads back
                // from the keys written, a stand-in (the causal loop's link and the loops its hook claims; the
                // standalone host's `ClaimedLoopIds`).
                let minted = adp_disl::behavior::ids::of(specification);
                let stand_ins = StandInIds::default();
                let ids: &dyn adp_disl::IdSource = match (&minted, given) {
                    (Ok(_), Some(given)) => given,
                    (Ok(minted), None) => minted,
                    (Err(_), _) => &stand_ins,
                };
                match self.run_operation(workspace, target, &entry, &operation, Some(parameters), ids) {
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
        if self.view.is_some() {
            return None;
        }
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
        if self.view.is_some() {
            return Err(NOT_OFFERED.to_owned());
        }
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
        if self.view.is_some() {
            return Err(NOT_OFFERED.to_owned());
        }
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
        if self.view.is_some() {
            return Err(NOT_OFFERED.to_owned());
        }
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
            &position_at(document, x, y),
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
        // `self.view` reads this viewer's view state (DISL section 12.2), the module's to give.
        if let Some(module) = &document.tool().module {
            for node in working.nodes() {
                if let Some(view) = module.view_of(document, &node) {
                    node.set_view(Some(view));
                }
            }
        }
        let element = match target {
            ActionTarget::Element(id) => working.element_by_id(id),
            _ => None,
        };
        // An element a parameter holds is the working diagram's, which the operation's changes are made on.
        let parameters = parameters.map(|given| {
            given
                .iter()
                .map(|(name, value)| {
                    let value = match DislElement::from_value(value).and_then(|e| working.element_by_id(&e.id())) {
                        Some(element) => element.to_value(),
                        None => value.clone(),
                    };
                    (name.clone(), value)
                })
                .collect::<Map>()
        });
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
        // The `on: connect` hooks of every relation the operation drew run in the same edit (DISL section 9.2), each
        // on the working diagram the operation left (the standalone host's `HookRunner.AfterConnect`, run by its
        // causal loop module for the loops a new link closes).
        if transaction.refusal.is_none() {
            let drawn: Vec<String> = transaction
                .changes
                .iter()
                .filter_map(|change| match change {
                    DislChange::Connect { id, .. } => Some(id.clone()),
                    _ => None,
                })
                .collect();
            for id in drawn {
                let Some(relation) = working.element_by_id(&id) else {
                    continue;
                };
                let hooked = hooks::after_connect(specification, &relation, ids, Some(&self.env), "user")?;
                if let Some(refusal) = hooked.refusal {
                    return Err(refusal);
                }
                transaction.changes.extend(hooked.changes);
            }
        }
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
        // A `view` action sets this viewer's view state, which is the module's to keep; nothing is written.
        if transaction.refusal.is_none()
            && let Some(module) = &document.tool().module
        {
            for action in &transaction.host_actions {
                if let HostAction::View { element_id, values } = action
                    && let Some(element) = document.element(element_id)
                {
                    module.set_view(document, &element, values);
                }
            }
            // What the drawing reads of the view is the document's: bound again from the module.
            document.bind_views();
        }
        *self.host_actions.borrow_mut() = transaction.host_actions.clone();
        self.write(workspace, &entry.label, transaction)
    }

    /// What the last operation that ran handed back for the host to carry out, in order: the selection to make, the
    /// label to edit inline, the layout to run.
    pub fn take_host_actions(&self) -> Vec<HostAction> {
        std::mem::take(&mut *self.host_actions.borrow_mut())
    }
}

/// Ids that stand in for those of a `natural` strategy while an answered operation runs: unique, and never written,
/// since such an element's id is read back from the keys its binding writes.
#[derive(Default)]
struct StandInIds {
    issued: std::cell::Cell<usize>,
}

impl adp_disl::IdSource for StandInIds {
    fn next(&self, type_name: &str) -> Result<String, String> {
        self.issued.set(self.issued.get() + 1);
        Ok(format!("{type_name}:answered-{}", self.issued.get()))
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
        .and_then(|m| m.position_in(&document.model().diagram, x, y))
        .unwrap_or_else(|| bounds(x, y))
}

/// The plugin a layout algorithm runs (`layout.algorithms.<name>.algorithm` of `plugin:<plugin>`), else the algorithm
/// it names (`rows`, the hype cycle graph's arrangement), which the operation leaves to the tool type's module.
fn plugin_layout(specification: &adp_disl::DislSpecification, algorithm: &str) -> Option<String> {
    let declared = specification
        .root()
        .get("layout")?
        .get("algorithms")?
        .get(algorithm)?
        .get("algorithm")?
        .as_str()?;
    Some(declared.strip_prefix("plugin:").unwrap_or(declared).to_owned())
}

/// What a menu entry's `args` give an operation's `parameters`, by name (DISL section 7.3).
fn given_arguments(parameters: &[operations::DislOperationParameter], entry: &DerivedMenuEntry) -> Map {
    parameters
        .iter()
        .filter_map(|p| Some((p.name.clone(), entry.arguments.get(&p.name)?.clone())))
        .collect()
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
