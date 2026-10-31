//! What a tool type's module adds to the host where its specification and binding cannot say it (etalii.adp spec
//! 030, FR-018, FR-030 and FR-031; plan.md, "Project Structure": `modules/diagrams/*`).
//!
//! The standalone host at `8b6ccfc` built every diagram type as a module of its own (`src/diagrams/<type>/backend`),
//! registered through dependency injection: its document store, its `IContextActionProvider`, its layout. Here one
//! value per origin carries what is left of it, given to [`crate::DiagramTools::with_module`]: the persistence
//! plugin its binding names as its reader (FBL section 11), the actions it runs itself, the positions its layout
//! gives (the mind map's), the implementations of the specification's plugin CEL functions (DISL section 13.1.1),
//! the canvas coordinates of a coordinate system whose placement attributes are not plain numbers (section 5; the
//! timeline's `TimelineScale` and `TimelineRows`), the plugin layouts its `layout` actions name (section 10), the
//! attribute an inline rename writes where a computed label names none, the size a node is drawn at and what a resize
//! of it writes where its coordinates are not plain numbers, the changes as its body writes them, and what a property
//! row and a drag write where the module's own commands wrote them (the agent behavior model's Kind row and its drag of
//! a whole row).
//! Every hook has the generic answer as its default, so a module implements only what it adds, and a tool type
//! without a module is drawn and edited from its specification alone.
//!
//! A module may also read, draw and edit its tool type's documents itself, without a specification the host loads
//! ([`DiagramModule::opens_documents`]): the standalone host's C4 module at `8b6ccfc` read Structurizr DSL with its own
//! parser and wrote it with its own commands, and no DISL specification of it loads there (the vendored
//! `c4-*.dis` describe a later stage). Such a module opens a [`ModuleView`] per view, which the session hands every
//! gesture, property edit and action to; its documents are kept in the project's [`DiagramDocuments`]
//! ([`DiagramDocuments::module_state`]), so its edits are commands on the project's history like every other.

use std::collections::HashMap;
use std::fmt;
use std::path::Path;
use std::rc::Rc;

use adp_cel::Map;
use adp_context::{ActionGroup, PropertyRow};
use adp_disl::derivation::finding::DislReaderFinding;
use adp_disl::{
    DislChange, DislConstraintOptions, DislDiagram, DislElement, DislPluginFunctions, DislSpecification, IdSource,
};
use adp_fbl::model::RuleObserver;
use adp_fbl::plugins::PersistencePlugin;

use crate::definition::ToolboxItem;
use crate::document::{DiagramChange, DiagramDocument, DiagramDocuments, DocumentEdit};
use crate::element::{DiagramElement, DiagramViewport};
use crate::problems::DiagramProblem;
use crate::session::{ActionOutcome, ActionTarget, DiagramSession};
use crate::sizing::{Extent, Resize};
use crate::workspace::{DiagramWorkspace, OpenedDiagram};

/// Where a gesture puts an element, as a module's coordinates say: the attributes to set, and the bounds before and
/// after that the specification's placement constraints judge (DISL section 8.4, `oldBounds` and `newBounds`).
#[derive(Debug, Clone, Default)]
pub struct ModulePlacement {
    /// The attributes the move writes; one already holding its value is left out, so it is not written again.
    pub attributes: Map,
    pub old_bounds: Map,
    pub new_bounds: Map,
}

/// A file a tool type's module can write from an open document (etalii.adp spec 074, FR-002): the frame shows one ribbon
/// button per export and writes what it gives beside the document, without knowing what the tool type or the file is.
#[derive(Clone)]
pub struct Export {
    /// What the ribbon asks it by.
    pub id: String,
    pub label: String,
    /// What the button says while it can run.
    pub hint: String,
    /// Why it cannot run while the active document is of another tool type, or none is open.
    pub elsewhere: String,
    /// The file `document` gives.
    pub write: Rc<dyn Fn(&DiagramDocument) -> ExportOutput>,
}

/// What an export gives: the file's name, beside the document, and its bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportOutput {
    pub file_name: String,
    pub bytes: Vec<u8>,
}

/// The residue of one tool type: what its module implements beside its specification. Each method's default is the
/// session's generic behaviour.
pub trait DiagramModule {
    /// The size of `element` where the module measures it; none leaves it to the notation's own default.
    fn size_of(&self, _document: &DiagramDocument, _element: &DislElement) -> Option<(f64, f64)> {
        None
    }

    /// Whether the module reads, draws and edits the documents of its tool type itself, so that the host loads no
    /// specification for it and opens it through [`DiagramModule::open`]; false for a tool type drawn from its
    /// specification.
    fn opens_documents(&self) -> bool {
        false
    }

    /// The extensions, lower case with their dot, of the bodies the module opens itself when one is opened without a
    /// registration; none by default, the catalogue's alone.
    fn extensions(&self) -> Vec<String> {
        Vec::new()
    }

    /// Opens the document at `path` (a registration of tool type `origin`, or a body the module claims), in the
    /// project at `root`, as a view the module draws and edits itself, keeping what it reads in `documents`. Asked
    /// only of a module that [opens its documents](DiagramModule::opens_documents).
    fn open(
        &self,
        root: &Path,
        path: &Path,
        origin: &str,
        documents: &mut DiagramDocuments,
    ) -> Result<(OpenedDiagram, Rc<dyn ModuleView>), String> {
        let _ = (root, path, origin, documents);
        Err(crate::workspace::CANNOT_OPEN.to_owned())
    }

    /// The files the module can write from an open document of its tool type; none by default.
    fn exports(&self) -> Vec<Export> {
        Vec::new()
    }

    /// The persistence plugin with id `plugin_id` (FBL section 11), when the module provides it.
    fn persistence(&self, plugin_id: &str) -> Option<Rc<dyn PersistencePlugin>> {
        let _ = plugin_id;
        None
    }

    /// Runs action `action_id` of `target`'s menu itself, or none to leave it to the specification's behaviour.
    fn execute(
        &self,
        session: &DiagramSession,
        workspace: &mut DiagramWorkspace,
        target: &ActionTarget,
        action_id: &str,
        ids: &dyn IdSource,
    ) -> Option<ActionOutcome> {
        let _ = (session, workspace, target, action_id, ids);
        None
    }

    /// Answers the prompt action `action_id` of `target` raised, or none to leave it to the session.
    fn answer(
        &self,
        session: &DiagramSession,
        workspace: &mut DiagramWorkspace,
        target: &ActionTarget,
        action_id: &str,
        value: &str,
    ) -> Option<ActionOutcome> {
        let _ = (session, workspace, target, action_id, value);
        None
    }

    /// Where the module's layout puts each element, by id, in the diagram's units; none to keep the registration's.
    fn positions(&self, document: &DiagramDocument) -> Option<HashMap<String, (f64, f64)>> {
        let _ = document;
        None
    }

    /// Sets this viewer's view of `element` to `values`, as a `view` action asks (DISL section 9.4); false when the
    /// module keeps no view state, and the action then changes nothing.
    fn set_view(&self, document: &DiagramDocument, element: &DislElement, values: &Map) -> bool {
        let _ = (document, element, values);
        false
    }

    /// The viewer's state of `element` in `document`, which `self.view` reads in a menu, a label or a condition
    /// (DISL section 12.2; the mind map's folds, `persistence.view.viewer`): none for a tool type that keeps none.
    fn view_of(&self, document: &DiagramDocument, element: &DislElement) -> Option<Map> {
        let _ = (document, element);
        None
    }

    /// The host's implementations of the CEL functions the specification's plugins declare.
    fn plugin_functions(&self) -> DislPluginFunctions {
        DislPluginFunctions::none()
    }

    /// Where `element` is drawn, in the canvas units of the specification's coordinate system; none to read its
    /// placement attributes as plain numbers.
    fn position(&self, _element: &DislElement) -> Option<(f64, f64)> {
        None
    }

    /// What a gesture that puts `element` at (`x`, `y`) in canvas units writes, or why it may not; none to write the
    /// point into the placement attributes as numbers.
    fn placement(&self, _element: &DislElement, _x: f64, _y: f64) -> Option<Result<ModulePlacement, String>> {
        None
    }

    /// `element`'s stored size, in the units of [`DiagramModule::position`] or in canvas units; none to read it from
    /// the attributes its notation binds (DISL sections 5.8 and 6.9).
    fn extent(&self, _element: &DislElement) -> Option<Extent> {
        None
    }

    /// What a resize of `element` to `resize` writes, or why it may not; none to write it into the attributes its
    /// notation binds.
    fn resize(&self, _element: &DislElement, _resize: &Resize) -> Option<Result<ModulePlacement, String>> {
        None
    }

    /// The `position` an operation or a tool run at the canvas point (`x`, `y`) reads (DISL section 12.3): none for
    /// the point itself, as a map of `x` and `y`.
    fn position_at(&self, _x: f64, _y: f64) -> Option<Map> {
        None
    }

    /// The `position` an operation or a tool run at the canvas point (`x`, `y`) of `diagram` reads, where it depends
    /// on the diagram (the hype cycle graph's time unit): [`DiagramModule::position_at`] by default.
    fn position_in(&self, _diagram: &DislDiagram, x: f64, y: f64) -> Option<Map> {
        self.position_at(x, y)
    }

    /// Runs the plugin layout `plugin` (a layout algorithm `plugin:<plugin>`, DISL section 10), or the algorithm an
    /// operation's `layout` action names that the runtime leaves to the module (the hype cycle graph's `rows`), over
    /// `diagram`: the changes it makes, as one step, or why it cannot; none when the module does not implement it.
    fn layout(
        &self,
        _specification: &DislSpecification,
        _plugin: &str,
        _diagram: &DislDiagram,
    ) -> Option<Result<Vec<DislChange>, String>> {
        None
    }

    /// The attribute an inline edit of `element`'s label `label` writes when the label is computed and binds none
    /// (DISL section 6.12): none for the specification's answer alone.
    fn label_attribute(&self, _element: &DislElement, _label: &str) -> Option<String> {
        None
    }

    /// The changes of a transaction on `diagram` as the body writes them, before they are planned, or why the body
    /// may not hold them: where the binding's rules tell an element's type from what it holds, a retype is the
    /// attribute change beside it, and the module leaves it out; a module refuses what no edit may make, whichever
    /// gesture, form or operation made it. The changes as they are by default.
    fn written(&self, _diagram: &DislDiagram, changes: Vec<DislChange>) -> Result<Vec<DislChange>, String> {
        Ok(changes)
    }

    /// The changes that set property `property_id` of `element_id` in `document` to `value`, or why they may not; none
    /// to commit it through the specification's form. The agent behavior model's module changed a node's kind, label,
    /// attempts and notes with its own commands (`AbmContextPropertyProvider.Change`), its Kind by the menu name of a
    /// kind, which a form's `type` item does not commit.
    fn set_property(
        &self,
        document: &DiagramDocument,
        element_id: &str,
        property_id: &str,
        value: &str,
    ) -> Option<Result<Vec<DiagramChange>, String>> {
        let _ = (document, element_id, property_id, value);
        None
    }

    /// What a drag of the elements of `moves`, each to its point in the diagram's units, writes in `document`, as one
    /// edit, or why it may not; none to place each where its placement attributes or the registration's `layout:` keep
    /// it. The agent behavior model's drag moves a whole row and may reorder it, writing the Markdown and the
    /// registration together (`ArrangeAbmNodeCommand.cs`).
    fn move_elements(
        &self,
        document: &DiagramDocument,
        moves: &[(String, f64, f64)],
    ) -> Option<Result<DocumentEdit, String>> {
        let _ = (document, moves);
        None
    }

    /// The line ending a block the registration does not have yet is created with (its `layout:` on a first
    /// placement), in place of the registration's own (FBL section 8.3): none for the registration's own. The
    /// standalone host wrote a new `layout:` block with CRLF on every platform (`AdpFileWriter.NewLine`).
    fn created_block_ending(&self) -> Option<String> {
        None
    }

    /// Told of each rule of the binding as it reads an entry, as a debugger is: none by default.
    fn rule_observer(&self) -> Option<RuleObserver> {
        None
    }

    /// What the reader could not read in `document`, beside what the session reports itself, each as the built-in
    /// it is reported as (DISL section 8.6): none by default. The causal loop module words each statement its
    /// binding leaves unread in the tool's own sentence (`causal-loop-diagram.md`, gap 1).
    fn reader_findings(&self, document: &DiagramDocument) -> Vec<DislReaderFinding> {
        let _ = document;
        Vec::new()
    }

    /// The findings of `document`'s constraints (DISL section 8) as the module reports them, evaluated from the
    /// session's `options`: in the module's order, each where the module locates it, with the ids the file writes
    /// rather than the model's. None for the specification's findings as the session reports them.
    fn problems(&self, _document: &DiagramDocument, _options: DislConstraintOptions) -> Option<Vec<DiagramProblem>> {
        None
    }
}

impl fmt::Debug for dyn DiagramModule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DiagramModule")
    }
}

/// One open view of a document a module reads, draws and edits itself ([`DiagramModule::opens_documents`]): what the
/// session asks of it in place of the specification. Every edit it makes is a command on the project's history,
/// through the workspace it is handed.
pub trait ModuleView {
    /// Every element of the view that `viewport` shows, in the order it is drawn, at the positions its layout gives
    /// in the diagram's units.
    fn render(&self, documents: &DiagramDocuments, viewport: DiagramViewport) -> Result<Vec<DiagramElement>, String>;

    /// What a selection of `element_id` reads as: the path that names it and its text; none when nothing has that id.
    fn describe(&self, documents: &DiagramDocuments, element_id: &str) -> Option<(Vec<String>, String)>;

    /// The palette.
    fn toolbox(&self) -> Vec<ToolboxItem>;

    /// The findings of the document, each with its severity and where it is.
    fn problems(&self, documents: &DiagramDocuments) -> Vec<DiagramProblem>;

    /// The context menu of `target`.
    fn actions(&self, documents: &DiagramDocuments, target: &ActionTarget) -> Vec<ActionGroup>;

    /// Runs action `action_id` on `target`, or asks first.
    fn execute(&self, workspace: &mut DiagramWorkspace, target: &ActionTarget, action_id: &str) -> ActionOutcome;

    /// Runs action `action_id` on `target`, or asks first, with `ids` giving the id of what it makes: the ids the
    /// standalone host generated, when a parity replay hands them over. By default the view's own
    /// [`ModuleView::execute`], for a view whose actions make nothing that needs an id.
    fn execute_with_ids(
        &self,
        workspace: &mut DiagramWorkspace,
        target: &ActionTarget,
        action_id: &str,
        ids: &dyn IdSource,
    ) -> ActionOutcome {
        let _ = ids;
        self.execute(workspace, target, action_id)
    }

    /// The action a connect gesture from `source` to `target` runs, as an action on the
    /// [connection target](ActionTarget::Connection) the gesture names, or why the view connects no such pair: none
    /// for a view that takes no connection from the canvas (the C4 views), which is the default.
    fn connect_action(
        &self,
        documents: &DiagramDocuments,
        source: &str,
        target: &str,
    ) -> Option<Result<String, String>> {
        let _ = (documents, source, target);
        None
    }

    /// The action a double-click on the element `element_id` runs (the standalone host's `activate` gesture), as the
    /// view's own [`ModuleView::execute`] takes it: none for a view that declares no double-click, which is the
    /// default.
    fn double_click(&self, documents: &DiagramDocuments, element_id: &str) -> Option<String> {
        let _ = (documents, element_id);
        None
    }

    /// Whether a right drag from one element to another draws a connection (the standalone host's
    /// `connectOnRightDrag`); without it a right drag on the view does nothing, and its right click opens the menu. No
    /// by default.
    fn connects_on_right_drag(&self) -> bool {
        false
    }

    /// Answers the prompt action `action_id` of `target` raised with `value`.
    fn answer(
        &self,
        workspace: &mut DiagramWorkspace,
        target: &ActionTarget,
        action_id: &str,
        value: &str,
    ) -> ActionOutcome;

    /// Answers a prompt as [`ModuleView::answer`] does, giving what the answer creates the ids `ids` hands out; by
    /// default the view's own.
    fn answer_with_ids(
        &self,
        workspace: &mut DiagramWorkspace,
        target: &ActionTarget,
        action_id: &str,
        value: &str,
        ids: &dyn IdSource,
    ) -> ActionOutcome {
        let _ = ids;
        self.answer(workspace, target, action_id, value)
    }

    /// The property rows of the element `element_id`.
    fn properties(&self, documents: &DiagramDocuments, element_id: &str) -> Vec<PropertyRow>;

    /// Sets property `property_id` of every element of `element_ids` to `value`, as one command.
    fn set_property(
        &self,
        workspace: &mut DiagramWorkspace,
        element_ids: &[&str],
        property_id: &str,
        value: &str,
    ) -> Result<(), String>;

    /// Places each element of `moves` at its point, in the diagram's units.
    fn move_elements(&self, workspace: &mut DiagramWorkspace, moves: &[(String, f64, f64)]) -> Result<(), String>;

    /// Gives `element_id` another parent, or why the view does not.
    fn move_element(
        &self,
        workspace: &mut DiagramWorkspace,
        element_id: &str,
        new_parent_id: Option<&str>,
        index: i64,
    ) -> Result<(), String>;
}

impl fmt::Debug for dyn ModuleView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ModuleView")
    }
}
