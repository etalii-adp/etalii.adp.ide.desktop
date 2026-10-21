//! The open diagram documents of a project, every edit of them as a command on the project's history, and a body
//! changed on disk reconciled with them (etalii.adp spec 030, FR-016, FR-023, FR-031 to FR-033; data-model.md,
//! Document, Command and History, Splice, Registration).
//!
//! Ported from what the standalone host's modules shared at `8b6ccfc`: the document store each module registered
//! (`LifecycleDocumentStore`, one document per path, shared by every view of it), the body read and written through
//! an FBL binding (`DependencyGraphBody`, `TimelineBody`), `EtAlii.Adp.Diagram.Modules/FblBodyEdits.cs`, the
//! registration's layout command (`SetRegistrationLayoutCommand`), and the history's restore command that undid an
//! edit by putting the text back.
//!
//! Every write is a model change the FBL library plans as splices, in the body's own conventions, so a body keeps
//! its comments, unknown keys and blank lines because no splice touches them; an authored position is an edit of the
//! registration, never of a body another tool owns (FR-033). An edit is planned once, where the gesture happens,
//! against a copy of the document, change after change as one transaction; the [`DocumentEdit`] that results
//! carries the splices of each step, so applying it again on redo writes exactly the same bytes, and its inverse is
//! the splices that put the replaced bytes back. A step is applied only to the bytes it was planned against: on any
//! other it is refused with FBL section 7.2's sentence and nothing is written. Several elements changed by one
//! gesture are one edit, so one undo.

use std::any::Any;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_cel::Value;
use adp_disl::behavior::write;
use adp_disl::model::{builder, natural_ids};
use adp_disl::{DislChange, DislElement, DislModel};
use adp_documents::SelfWriteGuard;
use adp_fbl::history::{DRIFT_UNDO, apply_in_order, inverse};
use adp_fbl::model::{SourceLocation, codes};
use adp_fbl::plugins::PluginBody;
use adp_fbl::{
    FblModel, FblOptions, Finding, FindingSeverity, ModelChange, OpenBody, OpenRegistration, PlanResult,
    RegistrationDocument, Splice,
};
use adp_history::{Applied, Command, Refusal};

use crate::change_handler::same_path;
use crate::element::{DiagramElement, ElementKind};
use crate::memo::RecentBodies;
use crate::sizing::{Extent, Length, SizeSource, SizeSources};
use crate::tool::DiagramTool;

/// The refusal of a position for a diagram opened without a registration, as the standalone host words it.
pub const NO_REGISTRATION: &str = "This diagram was opened without a registration, so there is nowhere to store a position. Register the file to arrange it.";

/// The refusal of an edit to a diagram that is not open.
pub const NOT_OPEN: &str = "The diagram is not open.";

/// Which file of a document a step of an edit writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditedFile {
    Body,
    Registration,
}

/// The length and hash of the bytes a step was planned against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Fingerprint {
    length: usize,
    hash: u64,
}

impl Fingerprint {
    fn of(bytes: &[u8]) -> Self {
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        Fingerprint {
            length: bytes.len(),
            hash: hasher.finish(),
        }
    }
}

/// One step of an edit: the splices of one model change, in body order, offsets into the file before the step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditStep {
    pub file: EditedFile,
    pub splices: Vec<Splice>,
    before: Fingerprint,
}

/// One edit of one document, as one entry of the project's history: its steps in the order they apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentEdit {
    pub label: String,
    pub body_path: PathBuf,
    pub steps: Vec<EditStep>,
}

impl DocumentEdit {
    /// Whether the edit writes nothing: a change to what a document already holds.
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Whether any step writes `file`.
    pub fn writes(&self, file: EditedFile) -> bool {
        self.steps.iter().any(|s| s.file == file)
    }
}

impl Command<DiagramDocuments> for DocumentEdit {
    fn label(&self) -> String {
        self.label.clone()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        documents.apply(self).map(Applied::undoable)
    }
}

/// One change of a transaction, before it is planned.
#[derive(Debug, Clone)]
pub enum DiagramChange {
    /// A change in the specification's terms, written through `persistence.typeMap`; a create made at a point also
    /// stores that point in the registration, in the same step of the history.
    Disl(DislChange),
    /// A change in the binding's terms.
    Body(ModelChange),
    /// An authored position, stored in the registration's `layout:`.
    Place { id: String, x: f64, y: f64 },
}

/// What reading a body gives, shared by every view of it.
#[derive(Debug)]
pub struct Reading {
    pub fbl: FblModel,
    pub model: DislModel,
    pub registration: Option<RegistrationDocument>,
}

/// One open diagram: its body and registration as they are on disk after every edit, and their reading.
#[derive(Debug)]
pub struct DiagramDocument {
    tool: Rc<DiagramTool>,
    body_path: PathBuf,
    registration_path: Option<PathBuf>,
    body: Rc<[u8]>,
    registration: Option<Rc<[u8]>>,
    reading: Rc<Reading>,
    deleted: bool,
}

impl DiagramDocument {
    pub fn tool(&self) -> &Rc<DiagramTool> {
        &self.tool
    }

    pub fn body_path(&self) -> &Path {
        &self.body_path
    }

    pub fn registration_path(&self) -> Option<&Path> {
        self.registration_path.as_deref()
    }

    pub fn body_bytes(&self) -> &[u8] {
        &self.body
    }

    pub fn registration_bytes(&self) -> Option<&[u8]> {
        self.registration.as_deref()
    }

    /// The DISL model of the body as it is now.
    pub fn model(&self) -> &DislModel {
        &self.reading.model
    }

    /// The binding's reading of the body as it is now.
    pub fn fbl(&self) -> &FblModel {
        &self.reading.fbl
    }

    pub fn registration(&self) -> Option<&RegistrationDocument> {
        self.reading.registration.as_ref()
    }

    /// The element `id` names, or none.
    pub fn element(&self, id: &str) -> Option<DislElement> {
        self.reading.model.diagram.element_by_id(id)
    }

    /// Why the document cannot be edited, or none: a body that could not be read, a body that is gone, a binding
    /// that is read-only.
    pub fn refusal(&self) -> Option<String> {
        let name = file_name(&self.body_path);
        if self.deleted {
            return Some(format!("{name} is no longer there, so it cannot be edited."));
        }
        if self.reading.model.is_unreadable {
            return Some(format!("{name} could not be read, so it cannot be edited."));
        }
        self.tool.binding.read_only.as_ref().map(|reason| {
            if reason.is_empty() {
                "This diagram is read-only.".to_owned()
            } else {
                format!("This diagram is read-only: {reason}")
            }
        })
    }

    /// The authored positions, by element id, in the diagram's domain units: a node the tool type's module places is
    /// where its coordinates say, a node whose notation places it by two of its attributes where they say, any other
    /// where the registration's `layout:` puts it; for a tool type whose module lays it out, where that layout puts
    /// it.
    pub fn positions(&self) -> HashMap<String, (f64, f64)> {
        if let Some(positions) = self.laid_out() {
            return positions;
        }
        let mut positions = HashMap::new();
        if let Some(layout) = self.registration().and_then(|r| r.layout.as_ref()) {
            for entry in &layout.entries {
                let mut parts = entry.value.split_whitespace().map(str::parse::<f64>);
                if let (Some(Ok(x)), Some(Ok(y)), None) = (parts.next(), parts.next(), parts.next()) {
                    positions.insert(entry.key.clone(), (x, y));
                }
            }
        }
        let module = self.tool.module.as_deref();
        for element in self.reading.model.diagram.nodes() {
            if let Some(position) = module.and_then(|m| m.position(&element)) {
                positions.insert(element.id(), position);
            } else if let Some((x, y)) = self.placement_attributes(&element) {
                let number = |name: &str| element.value_of(name).as_number().unwrap_or_default();
                positions.insert(element.id(), (number(&x), number(&y)));
            }
        }
        positions
    }

    /// Where the tool type's module lays every node out, by id, when its layout places the diagram; a node it leaves
    /// out, under a collapsed branch, is not drawn. None when the positions are authored.
    pub fn laid_out(&self) -> Option<HashMap<String, (f64, f64)>> {
        self.tool.module.as_ref().and_then(|module| module.positions(self))
    }

    /// Binds every node's `self.view` to the viewer's state the tool type's module keeps of it (DISL section 12.2),
    /// so a menu and a label read the viewer's folds; nothing for a tool type that keeps none.
    pub fn bind_views(&self) {
        let Some(module) = &self.tool.module else { return };
        for node in self.reading.model.diagram.nodes() {
            if let Some(view) = module.view_of(self, &node) {
                node.set_view(Some(view));
            }
        }
    }

    /// Whether a drag of `element` proposes a new parent rather than a place: its notation's placement leaves both
    /// axes to the layout (`placement.x.layout` and `placement.y.layout`), and its type may hold children (the mind
    /// map: "dropping it on another node makes it that node's last child … The drag never writes a position").
    pub fn drag_reparents(&self, element: &DislElement) -> bool {
        let Some(placement) = self.notation_of(element).and_then(|n| n.get("placement")) else {
            return false;
        };
        let laid_out = |axis: &str| {
            placement
                .get(axis)
                .and_then(|a| a.get("layout"))
                .and_then(serde_json::Value::as_bool)
                == Some(true)
        };
        laid_out("x") && laid_out("y") && !element.element_type().child_types.is_empty()
    }

    /// The two attributes the notation places a node by (`placement.x.attribute` and `placement.y.attribute` of the
    /// node type, or of the nearest type it is one of), or none when its place is the registration's.
    pub fn placement_attributes(&self, element: &DislElement) -> Option<(String, String)> {
        let placement = self.notation_of(element)?.get("placement")?;
        let attribute = |axis: &str| Some(placement.get(axis)?.get("attribute")?.as_str()?.to_owned());
        Some((attribute("x")?, attribute("y")?))
    }

    /// The node notation `element` is drawn by: its type's, or that of the nearest type it is one of.
    fn notation_of(&self, element: &DislElement) -> Option<&serde_json::Value> {
        let nodes = self
            .tool
            .definition
            .specification()
            .root()
            .get("notation")?
            .get("nodes")?
            .as_object()?;
        nodes
            .get(&element.type_name())
            .or_else(|| nodes.iter().find(|(name, _)| element.is_a(name)).map(|(_, n)| n))
    }

    /// The attributes the notation keeps `element`'s width and height in (DISL sections 5.8 and 6.9).
    pub fn size_sources(&self, element: &DislElement) -> SizeSources {
        self.notation_of(element).map(SizeSources::of).unwrap_or_default()
    }

    /// The stored sizes, by element id, of the nodes whose size is kept: where the tool type's module says, else
    /// in the attributes the notation binds. A node whose size is the notation's alone is not listed.
    pub fn extents(&self) -> HashMap<String, Extent> {
        let module = self.tool.module.as_deref();
        let mut extents = HashMap::new();
        for element in self.reading.model.diagram.nodes() {
            let extent = match module.and_then(|m| m.extent(&element)) {
                Some(extent) => extent,
                // A node the module places by coordinates of its own has no size the attributes give as numbers.
                None if module.is_some_and(|m| m.position(&element).is_some()) => Extent::default(),
                None => {
                    let sources = self.size_sources(&element);
                    let placement = self.placement_attributes(&element);
                    let number = |name: &str| element.value_of(name).as_number();
                    let length = |source: &Option<SizeSource>, near: Option<&String>| -> Option<Length> {
                        match source.as_ref()? {
                            SizeSource::Size(a) => number(a).map(Length::Canvas),
                            SizeSource::Span(a) => number(a).map(Length::Domain),
                            SizeSource::Far(a) => Some(Length::Domain(number(a)? - number(near?).unwrap_or_default())),
                        }
                    };
                    Extent {
                        width: length(&sources.width, placement.as_ref().map(|(x, _)| x)),
                        height: length(&sources.height, placement.as_ref().map(|(_, y)| y)),
                    }
                }
            };
            if extent != Extent::default() {
                extents.insert(element.id(), extent);
            }
        }
        extents
    }

    /// `value` as `element`'s attribute `name` holds it: a whole number for an integer or a `yearMonth` attribute.
    pub fn typed(&self, element: &DislElement, name: &str, value: f64) -> Value {
        match element
            .element_type()
            .attributes
            .get(name)
            .map(|a| a.type_name.as_str())
        {
            Some("int" | "integer" | "long" | "yearMonth") => Value::Int(value.round() as i64),
            _ => Value::Double(value),
        }
    }

    /// The change that puts `element` at (`x`, `y`) in the diagram's domain units: its placement attributes set,
    /// whole numbers for an integer attribute, or its position stored in the registration.
    pub fn placement_change(&self, element: &DislElement, x: f64, y: f64) -> DiagramChange {
        let Some((x_attribute, y_attribute)) = self.placement_attributes(element) else {
            return DiagramChange::Place { id: element.id(), x, y };
        };
        let typed = |name: &str, value: f64| self.typed(element, name, value);
        DiagramChange::Disl(DislChange::Set {
            element_id: element.id(),
            type_name: element.type_name(),
            attributes: [
                (x_attribute.clone(), typed(&x_attribute, x)),
                (y_attribute.clone(), typed(&y_attribute, y)),
            ]
            .into_iter()
            .collect(),
        })
    }

    /// Every element of the diagram, in the model's order, placed where the registration puts it.
    pub fn elements(&self) -> Vec<DiagramElement> {
        let positions = self.positions();
        self.reading
            .model
            .diagram
            .elements()
            .iter()
            .map(|element| {
                let id = element.id();
                let position = positions.get(&id).copied();
                let (x, y) = position.unwrap_or_default();
                DiagramElement {
                    x,
                    y,
                    placed: position.is_some(),
                    element_type: element.type_name(),
                    kind: if element.is_relation() {
                        ElementKind::Relation
                    } else {
                        ElementKind::Node
                    },
                    parent: element.parent().map(|p| p.id()),
                    // The node an end resolves to, else the id it was read with, so a dangling end still says
                    // what it names.
                    source: element.source().map(|s| s.id()).or_else(|| element.source_id()),
                    target: element.target().map(|t| t.id()).or_else(|| element.target_id()),
                    attributes: element
                        .attributes()
                        .iter()
                        .map(|(k, v)| (k.clone(), text_of(v)))
                        .collect(),
                    id,
                }
            })
            .collect()
    }

    /// Plans `changes` as one edit labelled `label`, each against the document as the ones before it left it; the
    /// first refusal, the library's or a missing registration's, plans nothing.
    pub fn plan(&self, label: &str, changes: &[DiagramChange]) -> Result<DocumentEdit, String> {
        if let Some(refusal) = self.refusal() {
            return Err(refusal);
        }
        let specification = self.tool.definition.specification();
        let options = self.tool.options(&file_name(&self.body_path), self.registration());
        let mut body = Body::open(&self.tool, self.body.to_vec(), options, &file_name(&self.body_path));
        let mut registration = self.registration.as_ref().map(|r| {
            let mut open = OpenRegistration::open(r.to_vec());
            open.created_block_ending = self.tool.module.as_ref().and_then(|m| m.created_block_ending());
            open
        });
        let mut steps = Vec::new();
        for change in changes {
            let (model_change, place, explicit) = match change {
                DiagramChange::Disl(disl) => (
                    Some(write::to_fbl(specification, disl)),
                    placement_of(specification, disl),
                    false,
                ),
                DiagramChange::Body(model_change) => (Some(model_change.clone()), None, false),
                DiagramChange::Place { id, x, y } => (None, Some((id.clone(), *x, *y)), true),
            };
            if let Some(model_change) = model_change {
                let before = Fingerprint::of(body.bytes());
                match body.change(&model_change) {
                    PlanResult::Planned(edit) if !edit.splices.is_empty() => steps.push(EditStep {
                        file: EditedFile::Body,
                        splices: edit.splices,
                        before,
                    }),
                    PlanResult::Planned(_) => {}
                    PlanResult::Refused(reason) | PlanResult::Invalid(reason) => return Err(reason),
                }
            }
            if let Some((id, x, y)) = place {
                let Some(registration) = registration.as_mut() else {
                    if explicit {
                        return Err(NO_REGISTRATION.to_owned());
                    }
                    // A create at a point in a diagram without a registration: there is nowhere to keep the point.
                    continue;
                };
                let before = Fingerprint::of(registration.bytes());
                match registration.change(&ModelChange::Place { id, x, y }) {
                    PlanResult::Planned(edit) if !edit.splices.is_empty() => steps.push(EditStep {
                        file: EditedFile::Registration,
                        splices: edit.splices,
                        before,
                    }),
                    PlanResult::Planned(_) => {}
                    PlanResult::Refused(reason) | PlanResult::Invalid(reason) => return Err(reason),
                }
            }
        }
        Ok(DocumentEdit {
            label: label.to_owned(),
            body_path: self.body_path.clone(),
            steps,
        })
    }
}

/// A body opened to be read or planned against: through its binding's declared rules, through the persistence plugin
/// the binding names as its reader (FBL section 11.3), or, when the host lacks that plugin, as unreadable with the
/// finding that says so, so it opens with a problem and refuses every edit rather than opening empty.
enum Body {
    Declared(Box<OpenBody>),
    Plugin(PluginBody),
    Missing { bytes: Vec<u8>, finding: Finding },
}

impl Body {
    fn open(tool: &DiagramTool, bytes: Vec<u8>, options: FblOptions, file_name: &str) -> Body {
        let Some(reader) = &tool.binding.plugin else {
            return Body::Declared(Box::new(OpenBody::open(bytes, tool.binding.clone(), Rc::new(options))));
        };
        let Some(plugin) = tool.persistence() else {
            let finding = Finding {
                code: codes::PLUGIN_MISSING.into(),
                severity: FindingSeverity::Error,
                message: format!(
                    "The plugin '{}' that reads {file_name} is not part of this host, so it cannot be read.",
                    reader.plugin
                ),
                location: Some(SourceLocation {
                    file: file_name.to_owned(),
                    line: 1,
                    column: 1,
                    length: 0,
                }),
                detail: None,
            };
            return Body::Missing { bytes, finding };
        };
        match PluginBody::open(bytes.clone(), tool.binding.clone(), Some(plugin), file_name) {
            Ok(body) => Body::Plugin(body),
            Err(reason) => Body::Missing {
                bytes,
                finding: Finding {
                    code: codes::PLUGIN_MISSING.into(),
                    severity: FindingSeverity::Error,
                    message: reason,
                    location: None,
                    detail: None,
                },
            },
        }
    }

    fn bytes(&self) -> &[u8] {
        match self {
            Body::Declared(body) => body.bytes(),
            Body::Plugin(body) => body.bytes(),
            Body::Missing { bytes, .. } => bytes,
        }
    }

    fn model(&self) -> FblModel {
        match self {
            Body::Declared(body) => body.model(),
            Body::Plugin(body) => body.model(),
            Body::Missing { finding, .. } => FblModel {
                findings: vec![finding.clone()],
                unreadable: true,
                ..FblModel::default()
            },
        }
    }

    fn change(&mut self, change: &ModelChange) -> PlanResult {
        match self {
            Body::Declared(body) => body.change(change),
            Body::Plugin(body) => body.change(change),
            Body::Missing { finding, .. } => PlanResult::Refused(finding.message.clone()),
        }
    }
}

/// The point a create was made at, when it says one: a map with `x` and `y`, under the id the element is read back
/// with, which under the `natural` strategy is its keys' rather than the one it was created with.
fn placement_of(specification: &adp_disl::DislSpecification, change: &DislChange) -> Option<(String, f64, f64)> {
    let DislChange::Create {
        id,
        type_name,
        attributes,
        at: Value::Map(at),
        ..
    } = change
    else {
        return None;
    };
    let x = at.get("x")?.as_number()?;
    let y = at.get("y")?.as_number()?;
    let id = natural_ids::id_of(specification, type_name, attributes).unwrap_or_else(|| id.clone());
    Some((id, x, y))
}

/// A value as text: a number in .NET's invariant form, as the standalone host wrote it, a list as its items joined.
pub fn text_of(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Int(i) => i.to_string(),
        Value::Double(d) => adp_fbl::planning::dotnet_double(*d),
        Value::String(s) => s.clone(),
        Value::List(items) => items.iter().map(text_of).collect::<Vec<_>>().join(", "),
        other => other.to_string(),
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// What a change on disk meant for the open documents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiskChange {
    /// No open document has a body or registration at the path.
    NotTracked,
    /// The change is the store's own write.
    OwnWrite,
    /// The file holds what the document already has.
    Unchanged,
    /// The document of the body at the path was read again.
    Reloaded(PathBuf),
    /// The body at the path is gone; its document is empty.
    Deleted(PathBuf),
}

/// The open diagram documents of one project, one per body, shared by every view of it: the model of the project's
/// history, which every edit command changes.
#[derive(Default)]
pub struct DiagramDocuments {
    documents: Vec<DiagramDocument>,
    guard: SelfWriteGuard,
    recent: RecentBodies<Reading>,
    changed: Vec<PathBuf>,
    /// What the modules that read their documents themselves keep of them, by the key each names
    /// ([`crate::DiagramModule::opens_documents`]).
    modules: HashMap<String, Box<dyn ModuleDocuments>>,
}

/// What a module that reads its documents itself keeps of them in the project's documents
/// ([`DiagramDocuments::module_state`]): a change on disk and a close reach it as they reach every other document.
pub trait ModuleDocuments: Any {
    /// Reconciles the file at `path`, reported changed on disk, with the module's document of it: the module's own
    /// write ignored, a body that holds what the document has needing nothing, any other change read again and a
    /// deleted body ended; [`DiskChange::NotTracked`] when no document of the module has that file.
    fn reconcile(&mut self, path: &Path) -> DiskChange {
        let _ = path;
        DiskChange::NotTracked
    }

    /// Forgets the document of the body at `body_path`, once no view shows it.
    fn close(&mut self, body_path: &Path) {
        let _ = body_path;
    }
}

impl DiagramDocuments {
    /// Opens the document whose body is at `body_path`, read through `tool`, with the registration at
    /// `registration_path` when there is one; an open document of the same body is shared.
    pub fn open(
        &mut self,
        tool: Rc<DiagramTool>,
        body_path: &Path,
        registration_path: Option<&Path>,
    ) -> Result<&DiagramDocument, String> {
        if let Some(index) = self.index_of(body_path) {
            return Ok(&self.documents[index]);
        }
        let read = |path: &Path| fs::read(path).map_err(|e| format!("'{}' could not be read: {e}", file_name(path)));
        let body = read(body_path)?;
        let registration = registration_path.map(read).transpose()?;
        let document = self.read(
            tool,
            body_path.to_path_buf(),
            registration_path.map(Path::to_path_buf),
            Rc::from(body),
            registration.map(Rc::from),
        );
        self.documents.push(document);
        Ok(self.documents.last().expect("just opened"))
    }

    /// Closes the document of the body at `body_path`.
    pub fn close(&mut self, body_path: &Path) {
        for state in self.modules.values_mut() {
            state.close(body_path);
        }
        if let Some(index) = self.index_of(body_path) {
            let document = self.documents.remove(index);
            self.guard.forget(&document.body_path);
            if let Some(registration) = &document.registration_path {
                self.guard.forget(registration);
            }
        }
    }

    /// The open document of the body at `body_path`.
    pub fn get(&self, body_path: &Path) -> Option<&DiagramDocument> {
        self.index_of(body_path).map(|i| &self.documents[i])
    }

    pub fn documents(&self) -> &[DiagramDocument] {
        &self.documents
    }

    /// The bodies whose documents changed since this was last asked, in order, each once.
    pub fn take_changed(&mut self) -> Vec<PathBuf> {
        std::mem::take(&mut self.changed)
    }

    /// Records that the document of the body at `body_path` changed, so its views refresh: a module that keeps its
    /// documents itself says so after each of its writes, and after a write beside the body that changes what a view
    /// shows (the standalone host's `IC4DocumentStore.Touch`).
    pub fn mark_changed(&mut self, body_path: &Path) {
        if !self.changed.iter().any(|c| same_path(c, body_path)) {
            self.changed.push(body_path.to_path_buf());
        }
    }

    /// What the module keyed `key` keeps of its documents, made empty the first time it is asked for.
    pub fn module_state<T: ModuleDocuments + Default>(&mut self, key: &str) -> &mut T {
        let entry = self
            .modules
            .entry(key.to_owned())
            .or_insert_with(|| Box::new(T::default()));
        if !(entry.as_ref() as &dyn Any).is::<T>() {
            *entry = Box::new(T::default());
        }
        (entry.as_mut() as &mut dyn Any)
            .downcast_mut::<T>()
            .expect("just made of this type")
    }

    /// What the module keyed `key` keeps of its documents, or none before it kept anything.
    pub fn module_state_ref<T: ModuleDocuments>(&self, key: &str) -> Option<&T> {
        (self.modules.get(key)?.as_ref() as &dyn Any).downcast_ref::<T>()
    }

    fn index_of(&self, body_path: &Path) -> Option<usize> {
        self.documents.iter().position(|d| same_path(&d.body_path, body_path))
    }

    fn read(
        &mut self,
        tool: Rc<DiagramTool>,
        body_path: PathBuf,
        registration_path: Option<PathBuf>,
        body: Rc<[u8]>,
        registration: Option<Rc<[u8]>>,
    ) -> DiagramDocument {
        let mut key = body.to_vec();
        key.push(0);
        key.extend_from_slice(registration.as_deref().unwrap_or_default());
        let reading = match self.recent.find(&key) {
            Some(reading) => reading,
            None => {
                let registration_document = registration.as_ref().map(|r| RegistrationDocument::read(r.to_vec()));
                let options = tool.options(&file_name(&body_path), registration_document.as_ref());
                let fbl = Body::open(&tool, body.to_vec(), options, &file_name(&body_path)).model();
                let model = builder::from(&fbl, tool.definition.specification());
                let reading = Rc::new(Reading {
                    fbl,
                    model,
                    registration: registration_document,
                });
                self.recent.remember(Rc::from(key), reading.clone());
                reading
            }
        };
        DiagramDocument {
            tool,
            body_path,
            registration_path,
            body,
            registration,
            reading,
            deleted: false,
        }
    }

    /// Replaces the bytes of the document at `index` and reads it again.
    fn replace(&mut self, index: usize, body: Rc<[u8]>, registration: Option<Rc<[u8]>>, deleted: bool) {
        let old = &self.documents[index];
        let (tool, body_path, registration_path) =
            (old.tool.clone(), old.body_path.clone(), old.registration_path.clone());
        let mut document = self.read(tool, body_path.clone(), registration_path, body, registration);
        document.deleted = deleted;
        self.documents[index] = document;
        self.mark_changed(&body_path);
    }

    /// Applies `edit` to its document and writes what changed to disk; the edit that reverses it, or the refusal
    /// with nothing written.
    pub fn apply(&mut self, edit: &DocumentEdit) -> Result<DocumentEdit, Refusal> {
        let index = self.index_of(&edit.body_path).ok_or_else(|| Refusal::new(NOT_OPEN))?;
        let document = &self.documents[index];
        let mut body = document.body.to_vec();
        let mut registration = document.registration.as_ref().map(|r| r.to_vec());
        let mut inverse_steps = Vec::with_capacity(edit.steps.len());
        for step in &edit.steps {
            let bytes = match step.file {
                EditedFile::Body => &mut body,
                EditedFile::Registration => registration
                    .as_mut()
                    .ok_or_else(|| Refusal::new("The registration file is no longer there."))?,
            };
            if Fingerprint::of(bytes) != step.before {
                return Err(Refusal::new(DRIFT_UNDO));
            }
            let after = apply_in_order(bytes, &step.splices).map_err(Refusal::new)?;
            inverse_steps.push(EditStep {
                file: step.file,
                splices: inverse(bytes, &step.splices),
                before: Fingerprint::of(&after),
            });
            *bytes = after;
        }
        inverse_steps.reverse();

        let body_path = document.body_path.clone();
        let registration_path = document.registration_path.clone();
        let old_body = document.body.clone();
        if edit.writes(EditedFile::Body) {
            self.write(&body_path, &body)
                .map_err(|e| Refusal::new(format!("Could not store the change: {e}")))?;
        }
        if edit.writes(EditedFile::Registration)
            && let (Some(path), Some(bytes)) = (&registration_path, &registration)
            && let Err(e) = self.write(path, bytes)
        {
            if edit.writes(EditedFile::Body) {
                // Best effort: put the body back, so the two files still agree.
                let _ = self.write(&body_path, &old_body);
            }
            return Err(Refusal::new(format!("Could not store the position: {e}")));
        }
        self.replace(index, Rc::from(body), registration.map(Rc::from), false);
        Ok(DocumentEdit {
            label: edit.label.clone(),
            body_path,
            steps: inverse_steps,
        })
    }

    fn write(&mut self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        self.guard.begin(path, bytes);
        let written = adp_documents::text::write_atomically(path, bytes);
        self.guard.end(path);
        written
    }

    /// Reconciles the file at `path`, reported changed on disk, with the document whose body or registration it is
    /// (FBL section 7.3): the store's own write is ignored, a body that holds what the document has needs nothing,
    /// and any other change is read again, a deleted body as an empty document.
    pub fn reconcile(&mut self, path: &Path) -> DiskChange {
        let Some((index, file)) = self.documents.iter().enumerate().find_map(|(i, d)| {
            if same_path(&d.body_path, path) {
                Some((i, EditedFile::Body))
            } else if d.registration_path.as_deref().is_some_and(|r| same_path(r, path)) {
                Some((i, EditedFile::Registration))
            } else {
                None
            }
        }) else {
            // Not a document drawn from a specification: perhaps one a module reads itself.
            let mut change = DiskChange::NotTracked;
            for state in self.modules.values_mut() {
                change = state.reconcile(path);
                if change != DiskChange::NotTracked {
                    break;
                }
            }
            if let DiskChange::Reloaded(body) | DiskChange::Deleted(body) = &change {
                let body = body.clone();
                self.mark_changed(&body);
            }
            return change;
        };
        let disk = fs::read(path).ok();
        if self.guard.is_own_write(path, disk.as_deref()) {
            return DiskChange::OwnWrite;
        }
        let document = &self.documents[index];
        let body_path = document.body_path.clone();
        let (body, registration) = (document.body.clone(), document.registration.clone());
        match (file, disk) {
            (EditedFile::Body, None) => {
                self.replace(index, Rc::from(Vec::new()), registration, true);
                DiskChange::Deleted(body_path)
            }
            (EditedFile::Body, Some(bytes)) if *bytes == *body => DiskChange::Unchanged,
            (EditedFile::Body, Some(bytes)) => {
                self.replace(index, Rc::from(bytes), registration, false);
                DiskChange::Reloaded(body_path)
            }
            (EditedFile::Registration, disk) if disk.as_deref() == registration.as_deref() => DiskChange::Unchanged,
            (EditedFile::Registration, disk) => {
                let deleted = document.deleted;
                self.replace(index, body, disk.map(Rc::from), deleted);
                DiskChange::Reloaded(body_path)
            }
        }
    }
}
