//! Every edit of a Wardley map, as a command on the project's history (etalii.adp spec 030, task T070).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/wardley-map/backend/EtAlii.Adp.Diagram.WardleyMap/
//! Commands/` (`WardleyElementCommands.cs`, `WardleyElementCommandHandlers.cs`, `MoveWardleyElementCommand.cs`,
//! `RestoreWardleyLineCommand.cs`) over the core's `DocumentEditRun` and `RestoreDocumentCommandHandler`. A command
//! names what it changes by the body's path and the element's id, so it is plain data a history can hold; it locates
//! its element against the map as it is now, refuses with a sentence when that fails, and edits a copy of the lines
//! through [`crate::writer`]. Two kinds of undo: an edit of one line names its own inverse, a [`RestoreWardleyLine`]
//! that puts the line back exactly as it was; an edit of several lines (an add, a remove, a rename) is undone by
//! restoring the whole previous text ([`RestoreWardleyDocument`]), whose own undo runs the edit again.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use adp_diagram::DiagramDocuments;
use adp_history::{Applied, Command, Refusal};

use crate::axis;
use crate::identities::{WardleyIdentityEntry, keys, kind};
use crate::lines::{LineDocument, format_number};
use crate::model::{
    WardleyComponent, WardleyCoordinate, WardleyDecorator, WardleyLinkKind, WardleyMap, WardleyPipelineChild,
    WardleyPipelineForm,
};
use crate::parser;
use crate::store;
use crate::writer;

/// The refusal of an edit whose element the map no longer has.
pub const GONE: &str = "That element is no longer on this map.";

type DocumentCommand = Box<dyn Command<DiagramDocuments>>;

/// What an edit of a copy of the lines came to, when it was made.
#[derive(Default)]
struct Edited {
    /// The command that puts the edited line back; none to restore the whole previous text.
    inverse: Option<DocumentCommand>,
    /// A component renamed from the first name to the second, whose identity goes with it.
    rekey: Option<(String, String)>,
}

impl Edited {
    fn undone_by(inverse: impl Command<DiagramDocuments> + 'static) -> Result<Edited, String> {
        Ok(Edited {
            inverse: Some(Box::new(inverse)),
            rekey: None,
        })
    }
}

/// The edit run (`DocumentEditRun.Run`): the map loaded, `edit` made on a copy of its lines with the map and its
/// identities as they are now, the copy saved, and the inverse recorded: the edit's own, else the restore of the text
/// from before it, whose undo is `redo`.
fn run(
    documents: &mut DiagramDocuments,
    body_path: &Path,
    redo: DocumentCommand,
    edit: impl FnOnce(&mut LineDocument, &WardleyMap, &[WardleyIdentityEntry]) -> Result<Edited, String>,
) -> Result<Applied<DiagramDocuments>, Refusal> {
    let document = store::get_or_load(documents, body_path);
    if let Some(reason) = &document.unreadable {
        return Err(Refusal::new(format!(
            "{} could not be read, so nothing can be edited until it can: {reason}",
            file_name(body_path)
        )));
    }
    let before = document.text.clone();
    let mut copy = document.lines();
    let map = parser::parse(&copy);
    let identities = store::identities_of(documents, body_path).to_vec();
    let edited = edit(&mut copy, &map, &identities).map_err(Refusal::new)?;
    if let Some((old, new)) = &edited.rekey {
        store::rekey(documents, body_path, kind::COMPONENT, old, new);
    }
    let warning = store::save(documents, body_path, &copy).map_err(Refusal::new)?;
    let inverse = edited.inverse.unwrap_or_else(|| {
        Box::new(RestoreWardleyDocument {
            body_path: body_path.to_path_buf(),
            text: before,
            redo: Arc::new(Mutex::new(redo)),
        })
    });
    Ok(Applied {
        inverse: Some(inverse),
        warning,
    })
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The component an element id names, or none.
pub fn component_of<'a>(
    map: &'a WardleyMap,
    identities: &[WardleyIdentityEntry],
    element_id: &str,
) -> Option<&'a WardleyComponent> {
    let entry = identities
        .iter()
        .find(|e| e.id == element_id && e.kind == kind::COMPONENT)?;
    map.components.iter().find(|c| keys::component(c) == entry.key)
}

/// A point written as the document writes one: `[visibility, maturity]`.
fn pair(position: WardleyCoordinate) -> String {
    format!(
        "[{}, {}]",
        format_number(position.visibility),
        format_number(position.maturity)
    )
}

/// Adds a `component`, `anchor` or `submap`, with the decorator it carries from the moment it is written: the
/// toolbox's Market and Ecosystem drop a component carrying one, in one command, so one drop is one undo.
#[derive(Debug, Clone, PartialEq)]
pub struct AddWardleyElement {
    pub body_path: PathBuf,
    pub kind: String,
    pub name: String,
    pub visibility: f64,
    pub maturity: f64,
    pub decorator: Option<WardleyDecorator>,
}

impl Command<DiagramDocuments> for AddWardleyElement {
    fn label(&self) -> String {
        format!("Add {}", self.kind.to_lowercase())
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        run(documents, &self.body_path, Box::new(self.clone()), |body, map, _| {
            if !["component", "anchor", "submap"]
                .iter()
                .any(|k| k.eq_ignore_ascii_case(&self.kind))
            {
                return Err(format!("A Wardley map has no '{}' statement.", self.kind));
            }
            if self.name.trim().is_empty() {
                return Err("An element needs a name.".to_owned());
            }
            // Two components sharing a name make every reference to it ambiguous.
            if map.components.iter().any(|c| c.name == self.name) {
                return Err(format!("This map already has an element called '{}'.", self.name));
            }
            let position = axis::clamp(WardleyCoordinate::new(self.visibility, self.maturity));
            let decorator = self.decorator.map(|d| format!(" ({})", d.spell())).unwrap_or_default();
            writer::append(
                body,
                &format!(
                    "{} {} {}{decorator}",
                    self.kind.to_lowercase(),
                    self.name,
                    pair(position)
                ),
            );
            Ok(Edited::default())
        })
    }
}

/// Adds a `note`: free text pinned at a position.
#[derive(Debug, Clone, PartialEq)]
pub struct AddWardleyNote {
    pub body_path: PathBuf,
    pub text: String,
    pub visibility: f64,
    pub maturity: f64,
}

impl Command<DiagramDocuments> for AddWardleyNote {
    fn label(&self) -> String {
        "Add note".to_owned()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        run(documents, &self.body_path, Box::new(self.clone()), |body, _, _| {
            if self.text.trim().is_empty() {
                // A note's text is also its identity's only handle.
                return Err("A note needs something to say.".to_owned());
            }
            let position = axis::clamp(WardleyCoordinate::new(self.visibility, self.maturity));
            writer::append(body, &format!("note {} {}", self.text.trim(), pair(position)));
            Ok(Edited::default())
        })
    }
}

/// Adds a numbered `annotation`, numbered one past the highest in use rather than by the caller.
#[derive(Debug, Clone, PartialEq)]
pub struct AddWardleyAnnotation {
    pub body_path: PathBuf,
    pub text: String,
    pub visibility: f64,
    pub maturity: f64,
}

impl Command<DiagramDocuments> for AddWardleyAnnotation {
    fn label(&self) -> String {
        "Add annotation".to_owned()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        run(documents, &self.body_path, Box::new(self.clone()), |body, map, _| {
            if self.text.trim().is_empty() {
                return Err("An annotation needs something to say.".to_owned());
            }
            let position = axis::clamp(WardleyCoordinate::new(self.visibility, self.maturity));
            let number = map.annotations.iter().map(|a| a.number).max().map_or(1, |n| n + 1);
            writer::append(
                body,
                &format!("annotation {number} {} {}", pair(position), self.text.trim()),
            );
            Ok(Edited::default())
        })
    }
}

/// Removes an element and every link and `evolve` that names it.
#[derive(Debug, Clone, PartialEq)]
pub struct RemoveWardleyElement {
    pub body_path: PathBuf,
    pub element_id: String,
}

impl Command<DiagramDocuments> for RemoveWardleyElement {
    fn label(&self) -> String {
        "Remove".to_owned()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        run(
            documents,
            &self.body_path,
            Box::new(self.clone()),
            |body, map, identities| {
                let component = component_of(map, identities, &self.element_id).ok_or(GONE)?;
                let mut lines = vec![component.line];
                lines.extend(
                    map.links
                        .iter()
                        .filter(|l| l.source == component.name || l.target == component.name)
                        .map(|l| l.line),
                );
                lines.extend(map.evolves.iter().filter(|e| e.name == component.name).map(|e| e.line));
                lines.sort_unstable();
                lines.dedup();
                // Descending, so removing one line cannot shift the next.
                for line in lines.into_iter().rev() {
                    writer::remove_line(body, line);
                }
                Ok(Edited::default())
            },
        )
    }
}

/// Renames an element, rewriting every statement that names it, and carries its identity to the new name.
#[derive(Debug, Clone, PartialEq)]
pub struct RenameWardleyElement {
    pub body_path: PathBuf,
    pub element_id: String,
    pub new_name: String,
}

impl Command<DiagramDocuments> for RenameWardleyElement {
    fn label(&self) -> String {
        "Rename".to_owned()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        run(
            documents,
            &self.body_path,
            Box::new(self.clone()),
            |body, map, identities| {
                if self.new_name.trim().is_empty() {
                    return Err("An element needs a name.".to_owned());
                }
                let component = component_of(map, identities, &self.element_id).ok_or(GONE)?;
                if map.components.iter().any(|c| c.name == self.new_name) {
                    return Err(format!("This map already has an element called '{}'.", self.new_name));
                }
                if writer::rename(body, map, &component.name, &self.new_name) == 0 {
                    return Err("That element could not be renamed.".to_owned());
                }
                Ok(Edited {
                    inverse: None,
                    rekey: Some((component.name.clone(), self.new_name.clone())),
                })
            },
        )
    }
}

/// Moves one element to a position in the document's own axis order, clamped to the map: a component's pair, or a
/// pipeline child's maturity alone.
#[derive(Debug, Clone, PartialEq)]
pub struct MoveWardleyElement {
    pub body_path: PathBuf,
    pub element_id: String,
    pub visibility: f64,
    pub maturity: f64,
}

/// The component or pipeline child an element id names.
enum Located<'a> {
    Component(&'a WardleyComponent),
    Child(&'a WardleyPipelineChild),
}

fn locate<'a>(map: &'a WardleyMap, identities: &[WardleyIdentityEntry], element_id: &str) -> Option<Located<'a>> {
    let entry = identities.iter().find(|e| e.id == element_id)?;
    match entry.kind.as_str() {
        kind::COMPONENT => map
            .components
            .iter()
            .find(|c| keys::component(c) == entry.key)
            .map(Located::Component),
        kind::PIPELINE_CHILD => map.pipelines.iter().find_map(|pipeline| {
            pipeline
                .children
                .iter()
                .find(|child| keys::pipeline_child(pipeline, child) == entry.key)
                .map(Located::Child)
        }),
        _ => None,
    }
}

impl Command<DiagramDocuments> for MoveWardleyElement {
    fn label(&self) -> String {
        "Move".to_owned()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        let body_path = self.body_path.clone();
        run(
            documents,
            &self.body_path,
            Box::new(self.clone()),
            |body, map, identities| {
                let target = locate(map, identities, &self.element_id).ok_or(GONE)?;
                let clamped = axis::clamp(WardleyCoordinate::new(self.visibility, self.maturity));
                let line = match &target {
                    Located::Component(c) => c.line,
                    Located::Child(c) => c.line,
                };
                // Captured before the edit, so the inverse restores the line rather than the numbers.
                let before = body.line_at(line).unwrap_or_default().to_owned();
                let moved = match target {
                    Located::Child(child) => writer::set_pipeline_child_maturity(body, child, clamped.maturity),
                    Located::Component(component) => writer::set_position(body, component, clamped),
                };
                if !moved {
                    return Err("That element could not be moved.".to_owned());
                }
                Edited::undone_by(RestoreWardleyLine {
                    body_path,
                    line,
                    text: before,
                })
            },
        )
    }
}

/// The text of line `number` of `body`, before an edit of it.
fn line_before(body: &LineDocument, number: u32) -> String {
    body.line_at(number).unwrap_or_default().to_owned()
}

/// Sets or clears `inertia` on a component.
#[derive(Debug, Clone, PartialEq)]
pub struct SetWardleyInertia {
    pub body_path: PathBuf,
    pub element_id: String,
    pub inertia: bool,
}

impl Command<DiagramDocuments> for SetWardleyInertia {
    fn label(&self) -> String {
        "Set inertia".to_owned()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        run(
            documents,
            &self.body_path,
            Box::new(self.clone()),
            |body, map, identities| {
                let component = component_of(map, identities, &self.element_id).ok_or(GONE)?;
                let before = line_before(body, component.line);
                if !writer::set_inertia(body, component, self.inertia) {
                    return Err("That element's inertia could not be changed.".to_owned());
                }
                Edited::undone_by(RestoreWardleyLine {
                    body_path: self.body_path.clone(),
                    line: component.line,
                    text: before,
                })
            },
        )
    }
}

/// Sets a component's decorators to exactly this set, as one command: the property grid shows them as one row.
#[derive(Debug, Clone, PartialEq)]
pub struct SetWardleyDecorators {
    pub body_path: PathBuf,
    pub element_id: String,
    pub decorators: Vec<WardleyDecorator>,
}

impl Command<DiagramDocuments> for SetWardleyDecorators {
    fn label(&self) -> String {
        "Set decorators".to_owned()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        run(
            documents,
            &self.body_path,
            Box::new(self.clone()),
            |body, map, identities| {
                let component = component_of(map, identities, &self.element_id).ok_or(GONE)?;
                // One line, captured once: several writes follow, and the inverse puts the line back as it was.
                let before = line_before(body, component.line);
                for decorator in WardleyDecorator::ALL {
                    let present = self.decorators.contains(&decorator);
                    if present == component.decorators.contains(&decorator) {
                        continue;
                    }
                    // Read again after each change: the statement was rewritten.
                    let current_map = parser::parse(body);
                    let current = component_of(&current_map, identities, &self.element_id);
                    let changed = current.is_some_and(|c| writer::set_decorator(body, c, decorator.spell(), present));
                    if !changed {
                        return Err("That element's decorators could not be changed.".to_owned());
                    }
                }
                Edited::undone_by(RestoreWardleyLine {
                    body_path: self.body_path.clone(),
                    line: component.line,
                    text: before,
                })
            },
        )
    }
}

/// Sets or clears one of the five decorators on a component.
#[derive(Debug, Clone, PartialEq)]
pub struct SetWardleyDecorator {
    pub body_path: PathBuf,
    pub element_id: String,
    pub decorator: WardleyDecorator,
    pub present: bool,
}

impl Command<DiagramDocuments> for SetWardleyDecorator {
    fn label(&self) -> String {
        format!("Set {}", self.decorator.spell())
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        run(
            documents,
            &self.body_path,
            Box::new(self.clone()),
            |body, map, identities| {
                let component = component_of(map, identities, &self.element_id).ok_or(GONE)?;
                let before = line_before(body, component.line);
                let word = self.decorator.spell();
                if !writer::set_decorator(body, component, word, self.present) {
                    return Err(format!("That element's '{word}' could not be changed."));
                }
                Edited::undone_by(RestoreWardleyLine {
                    body_path: self.body_path.clone(),
                    line: component.line,
                    text: before,
                })
            },
        )
    }
}

/// Sets or clears a link between two elements; the kind is part of what identifies it.
#[derive(Debug, Clone, PartialEq)]
pub struct SetWardleyLink {
    pub body_path: PathBuf,
    pub source_element_id: String,
    pub target_element_id: String,
    pub kind: WardleyLinkKind,
    pub present: bool,
    /// The text after the `;`; empty writes the link with no context at all.
    pub context: String,
}

impl Command<DiagramDocuments> for SetWardleyLink {
    fn label(&self) -> String {
        if self.present { "Link" } else { "Remove link" }.to_owned()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        run(
            documents,
            &self.body_path,
            Box::new(self.clone()),
            |body, map, identities| {
                let source = component_of(map, identities, &self.source_element_id);
                let target = component_of(map, identities, &self.target_element_id);
                let (Some(source), Some(target)) = (source, target) else {
                    return Err(GONE.to_owned());
                };
                if source.name == target.name {
                    return Err("A link needs two different elements.".to_owned());
                }
                let existing = map
                    .links
                    .iter()
                    .find(|l| l.source == source.name && l.target == target.name && l.kind == self.kind);
                if !self.present {
                    let existing =
                        existing.ok_or_else(|| "There is no such link between these two elements.".to_owned())?;
                    writer::remove_line(body, existing.line);
                    return Ok(Edited::default());
                }
                let context = self.context.trim();
                let statement = if context.is_empty() {
                    format!("{}{}{}", source.name, self.kind.arrow(), target.name)
                } else {
                    format!("{}{}{}; {context}", source.name, self.kind.arrow(), target.name)
                };
                // An existing link is rewritten in place rather than a second one appended.
                match existing {
                    None => writer::append(body, &statement),
                    Some(existing) => {
                        writer::replace_line(body, existing.line, &statement);
                    }
                }
                Ok(Edited::default())
            },
        )
    }
}

/// Sets or clears a component's `evolve` statement.
#[derive(Debug, Clone, PartialEq)]
pub struct SetWardleyEvolve {
    pub body_path: PathBuf,
    pub element_id: String,
    pub present: bool,
    pub maturity: f64,
    /// What it is called once it arrives; empty keeps its name.
    pub override_name: String,
}

impl Command<DiagramDocuments> for SetWardleyEvolve {
    fn label(&self) -> String {
        if self.present { "Evolve" } else { "Stop evolving" }.to_owned()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        run(
            documents,
            &self.body_path,
            Box::new(self.clone()),
            |body, map, identities| {
                let component = component_of(map, identities, &self.element_id).ok_or(GONE)?;
                let existing = map.evolves.iter().find(|e| e.name == component.name);
                if !self.present {
                    let existing = existing.ok_or_else(|| format!("'{}' is not evolving.", component.name))?;
                    writer::remove_line(body, existing.line);
                    return Ok(Edited::default());
                }
                // Clamped rather than refused, as a drag is.
                let maturity = self.maturity.clamp(0.0, 1.0);
                let name = if self.override_name.trim().is_empty() {
                    component.name.clone()
                } else {
                    format!("{}->{}", component.name, self.override_name.trim())
                };
                let statement = format!("evolve {name} {}", format_number(maturity));
                match existing {
                    None => writer::append(body, &statement),
                    Some(existing) => {
                        writer::replace_line(body, existing.line, &statement);
                    }
                }
                Ok(Edited::default())
            },
        )
    }
}

/// Adds a component to a pipeline or takes one out: the one edit that writes a block rather than a line.
#[derive(Debug, Clone, PartialEq)]
pub struct SetWardleyPipelineMembership {
    pub body_path: PathBuf,
    pub parent_element_id: String,
    pub child_name: String,
    pub member: bool,
    pub maturity: f64,
}

impl Command<DiagramDocuments> for SetWardleyPipelineMembership {
    fn label(&self) -> String {
        if self.member {
            "Add to pipeline"
        } else {
            "Remove from pipeline"
        }
        .to_owned()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        run(
            documents,
            &self.body_path,
            Box::new(self.clone()),
            |body, map, identities| {
                if self.child_name.trim().is_empty() {
                    return Err("A pipeline component needs a name.".to_owned());
                }
                let parent = component_of(map, identities, &self.parent_element_id).ok_or(GONE)?;
                let pipeline = map.pipelines.iter().find(|p| p.parent == parent.name);
                let child = pipeline.and_then(|p| p.children.iter().find(|c| c.name == self.child_name));
                if !self.member {
                    let child = child.ok_or_else(|| format!("'{}' is not in this pipeline.", self.child_name))?;
                    writer::remove_line(body, child.line);
                    return Ok(Edited::default());
                }
                if child.is_some() {
                    return Err(format!("'{}' is already in this pipeline.", self.child_name));
                }
                // The legacy form has no block to put a child in, and is written back as it was read.
                if pipeline.is_some_and(|p| p.form == WardleyPipelineForm::Legacy) {
                    return Err(format!(
                        "'{}' has a pipeline written in the older two-coordinate form, which holds no components. Rewrite it as a nested pipeline first.",
                        parent.name
                    ));
                }
                let statement = format!(
                    "  component {} [{}]",
                    self.child_name,
                    format_number(self.maturity.clamp(0.0, 1.0))
                );
                let Some(pipeline) = pipeline else {
                    // No pipeline yet: the parent gains one, in the nested form that can hold what is added.
                    writer::append(body, &format!("pipeline {}", parent.name));
                    writer::append(body, "{");
                    writer::append(body, &statement);
                    writer::append(body, "}");
                    return Ok(Edited::default());
                };
                // The closing brace is found by scanning: a block may hold blank lines, comments and other statements.
                let closing =
                    (pipeline.line..=body.len() as u32).find(|n| body.line_at(*n).is_some_and(|t| t.trim() == "}"));
                let Some(closing) = closing else {
                    return Err(format!(
                        "'{}' has a pipeline that is missing its closing brace.",
                        parent.name
                    ));
                };
                writer::insert_line(body, closing, &statement);
                Ok(Edited::default())
            },
        )
    }
}

/// Puts one line back exactly as it was: what a one-line edit's inverse restores, so an undone drag brings back
/// `[0.90, 0.10]` rather than `[0.9, 0.1]`. Its own inverse puts back what it replaced.
#[derive(Debug, Clone, PartialEq)]
pub struct RestoreWardleyLine {
    pub body_path: PathBuf,
    pub line: u32,
    pub text: String,
}

impl Command<DiagramDocuments> for RestoreWardleyLine {
    fn label(&self) -> String {
        "Restore line".to_owned()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        run(documents, &self.body_path, Box::new(self.clone()), |body, _, _| {
            let replaced = body
                .line_at(self.line)
                .map(str::to_owned)
                .ok_or_else(|| "That line is no longer in this map.".to_owned())?;
            writer::replace_line(body, self.line, &self.text);
            Edited::undone_by(RestoreWardleyLine {
                text: replaced,
                ..self.clone()
            })
        })
    }
}

/// The edit a restore undid, shared by every restore and redo of it.
type SharedCommand = Arc<Mutex<DocumentCommand>>;

/// Restores the whole text of a map from before an edit and reads it again (`RestoreDocumentCommand`): the undo of an
/// edit of several lines, whose own undo runs the edit again.
pub struct RestoreWardleyDocument {
    pub body_path: PathBuf,
    pub text: String,
    redo: SharedCommand,
}

impl Command<DiagramDocuments> for RestoreWardleyDocument {
    fn label(&self) -> String {
        label_of(&self.redo)
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        store::restore(documents, &self.body_path, &self.text).map_err(Refusal::new)?;
        Ok(Applied {
            inverse: Some(Box::new(Redo(self.redo.clone()))),
            warning: String::new(),
        })
    }
}

fn label_of(command: &SharedCommand) -> String {
    command.lock().map(|c| c.label()).unwrap_or_default()
}

/// The edit a restore undid, run again by the redo.
struct Redo(SharedCommand);

impl Command<DiagramDocuments> for Redo {
    fn label(&self) -> String {
        label_of(&self.0)
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        let command = self.0.lock().map_err(|_| Refusal::new("That edit cannot be redone."))?;
        command.apply(documents)
    }
}
