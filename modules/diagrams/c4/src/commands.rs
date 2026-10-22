//! Every edit of a C4 document, as a command on the project's history (etalii.adp spec 030, task T061).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4/Commands/`
//! (`C4Commands.cs`, `AddC4ElementCommand.cs`, `AddC4ViewCommand.cs`, `C4ViewMembershipCommands.cs`) and the core's
//! `EtAlii.Adp.History/DocumentEditRun.cs`. A command names what it changes by the body's path and the element's id,
//! never by a live object, so it is plain data a history can hold; it is run against the project's documents, on a
//! copy of the body's lines. An edit rewrites the one argument of the one line it changes, and every other byte of the
//! document stays as it arrived; its inverse puts that line back exactly as it was ([`RestoreC4Line`]), so an undone
//! description added to an element that had none leaves no `""` behind, and an undone rename finds its line by number
//! where the element's id changed with its name. A move is not an edit of the body at all: a position is view state, written to the layout sidecar.

use std::path::{Path, PathBuf};

use adp_diagram::DiagramDocuments;
use adp_history::{Applied, Command, Refusal};

use crate::lines::LineDocument;
use crate::model::{C4ElementKind, C4ViewKind, C4Workspace};
use crate::parser;
use crate::placement;
use crate::sidecar;
use crate::store;
use crate::tokens;

/// What an edit of a copy of the body came to.
enum Edit {
    /// It may not be made, and why.
    Refused(String),
    /// It was made, and this undoes it.
    UndoneBy(Box<dyn Command<DiagramDocuments>>),
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The edit run: the body loaded, `edit` made on a copy of its lines and their reading, the copy written and read
/// again, and the inverse recorded.
fn run(
    documents: &mut DiagramDocuments,
    body_path: &Path,
    edit: impl FnOnce(&mut LineDocument, &C4Workspace) -> Edit,
) -> Result<Applied<DiagramDocuments>, Refusal> {
    let document = store::get_or_load(documents, body_path);
    if let Some(reason) = &document.unreadable {
        return Err(Refusal::new(format!(
            "{} could not be read, so nothing can be edited until it can: {reason}",
            file_name(body_path)
        )));
    }
    let mut copy = document.lines();
    let workspace = parser::parse(&copy);
    match edit(&mut copy, &workspace) {
        Edit::Refused(reason) => Err(Refusal::new(reason)),
        Edit::UndoneBy(inverse) => {
            store::save(documents, body_path, &copy).map_err(Refusal::new)?;
            Ok(Applied {
                inverse: Some(inverse),
                warning: String::new(),
            })
        }
    }
}

/// Where an element's arguments start on its line: after `identifier = keyword`, or after the keyword when the
/// declaration named no identifier.
fn keyword_index_of(line: &str) -> Option<usize> {
    let tokens = tokens::split_with_spans(line);
    if tokens.len() >= 3 && tokens[1].value == "=" {
        return Some(2);
    }
    (!tokens.is_empty()).then_some(0)
}

/// Rewrites argument `argument` of the line element `element_id` was declared on.
fn edit_element(
    document: &mut LineDocument,
    workspace: &C4Workspace,
    element_id: &str,
    argument: usize,
    value: &str,
    body_path: &Path,
) -> Edit {
    let Some(element) = workspace.find(element_id) else {
        return Edit::Refused(format!("'{element_id}' is not in this model any more."));
    };
    let Some(line) = document.lines().get(element.line as usize - 1).map(|l| l.text.clone()) else {
        return Edit::Refused(format!("'{element_id}' is not in this model any more."));
    };
    let Some(keyword) = keyword_index_of(&line) else {
        return Edit::Refused(format!(
            "Line {} does not look like an element declaration any more.",
            element.line
        ));
    };
    let undo = RestoreC4Line::of(body_path, element.line, &line);
    match document.replace_line(element.line, &tokens::replace_argument(&line, keyword, argument, value)) {
        Ok(()) => Edit::UndoneBy(undo),
        Err(reason) => Edit::Refused(reason),
    }
}

/// Rewrites argument `argument` of a relationship's line, whose arguments follow its destination.
fn edit_relationship(
    document: &mut LineDocument,
    workspace: &C4Workspace,
    relationship_id: &str,
    argument: usize,
    value: &str,
    body_path: &Path,
) -> Edit {
    let Some(relationship) = workspace.relationship(relationship_id) else {
        return Edit::Refused("That relationship is not in this model any more.".to_owned());
    };
    let Some(line) = document
        .lines()
        .get(relationship.line as usize - 1)
        .map(|l| l.text.clone())
    else {
        return Edit::Refused("That relationship is not in this model any more.".to_owned());
    };
    let tokens = tokens::split_with_spans(&line);
    // `source -> destination "description" "technology"`, or `-> destination ...` inside an element block.
    let arrow = tokens.iter().position(|t| !t.quoted && t.value == "->");
    let Some(arrow) = arrow.filter(|a| a + 1 < tokens.len()) else {
        return Edit::Refused(format!(
            "Line {} does not look like a relationship any more.",
            relationship.line
        ));
    };
    let undo = RestoreC4Line::of(body_path, relationship.line, &line);
    match document.replace_line(
        relationship.line,
        &tokens::replace_argument(&line, arrow + 1, argument, value),
    ) {
        Ok(()) => Edit::UndoneBy(undo),
        Err(reason) => Edit::Refused(reason),
    }
}

/// Renames an element.
#[derive(Debug, Clone, PartialEq)]
pub struct SetElementName {
    pub body_path: PathBuf,
    pub element_id: String,
    pub name: String,
}

impl Command<DiagramDocuments> for SetElementName {
    fn label(&self) -> String {
        "Rename".to_owned()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        run(documents, &self.body_path, |document, workspace| {
            edit_element(document, workspace, &self.element_id, 0, &self.name, &self.body_path)
        })
    }
}

/// Sets an element's description, which C4 asks for on every element.
#[derive(Debug, Clone, PartialEq)]
pub struct SetElementDescription {
    pub body_path: PathBuf,
    pub element_id: String,
    pub description: String,
}

impl Command<DiagramDocuments> for SetElementDescription {
    fn label(&self) -> String {
        "Describe".to_owned()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        run(documents, &self.body_path, |document, workspace| {
            edit_element(
                document,
                workspace,
                &self.element_id,
                1,
                &self.description,
                &self.body_path,
            )
        })
    }
}

/// Sets the technology of a container, a component or a node; a person and a software system have none in C4.
#[derive(Debug, Clone, PartialEq)]
pub struct SetElementTechnology {
    pub body_path: PathBuf,
    pub element_id: String,
    pub technology: String,
}

impl Command<DiagramDocuments> for SetElementTechnology {
    fn label(&self) -> String {
        "Set technology".to_owned()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        run(documents, &self.body_path, |document, workspace| {
            if let Some(element) = workspace.find(&self.element_id)
                && !element.kind.has_technology()
            {
                // A person and a software system take (name, description, tags): writing position 2 would turn their
                // tags into a technology.
                return Edit::Refused(format!(
                    "A {} has no technology in C4; only containers, components and infrastructure do.",
                    element.kind.lower()
                ));
            }
            edit_element(
                document,
                workspace,
                &self.element_id,
                2,
                &self.technology,
                &self.body_path,
            )
        })
    }
}

/// Relabels a relationship: C4 asks the label to say what it is for.
#[derive(Debug, Clone, PartialEq)]
pub struct SetRelationshipDescription {
    pub body_path: PathBuf,
    pub relationship_id: String,
    pub description: String,
}

impl Command<DiagramDocuments> for SetRelationshipDescription {
    fn label(&self) -> String {
        "Relabel".to_owned()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        run(documents, &self.body_path, |document, workspace| {
            edit_relationship(
                document,
                workspace,
                &self.relationship_id,
                0,
                &self.description,
                &self.body_path,
            )
        })
    }
}

/// Sets the protocol two elements communicate over.
#[derive(Debug, Clone, PartialEq)]
pub struct SetRelationshipTechnology {
    pub body_path: PathBuf,
    pub relationship_id: String,
    pub technology: String,
}

impl Command<DiagramDocuments> for SetRelationshipTechnology {
    fn label(&self) -> String {
        "Set technology".to_owned()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        run(documents, &self.body_path, |document, workspace| {
            edit_relationship(
                document,
                workspace,
                &self.relationship_id,
                1,
                &self.technology,
                &self.body_path,
            )
        })
    }
}

/// Puts one line of the body back exactly as it was: the inverse of an edit of one element or relationship. Its own
/// inverse puts back what it replaced.
#[derive(Debug, Clone, PartialEq)]
pub struct RestoreC4Line {
    pub body_path: PathBuf,
    pub line: u32,
    pub text: String,
}

impl RestoreC4Line {
    fn of(body_path: &Path, line: u32, text: &str) -> Box<dyn Command<DiagramDocuments>> {
        Box::new(RestoreC4Line {
            body_path: body_path.to_path_buf(),
            line,
            text: text.to_owned(),
        })
    }
}

impl Command<DiagramDocuments> for RestoreC4Line {
    fn label(&self) -> String {
        "Restore line".to_owned()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        run(documents, &self.body_path, |document, _| {
            let Some(replaced) = document.lines().get(self.line as usize - 1).map(|l| l.text.clone()) else {
                return Edit::Refused("That line is no longer in this model.".to_owned());
            };
            match document.replace_line(self.line, &self.text) {
                Ok(()) => Edit::UndoneBy(RestoreC4Line::of(&self.body_path, self.line, &replaced)),
                Err(reason) => Edit::Refused(reason),
            }
        })
    }
}

/// Records where an element was dragged on one view, in the layout sidecar beside the body; the body is not edited.
#[derive(Debug, Clone, PartialEq)]
pub struct MoveC4Element {
    pub body_path: PathBuf,
    pub view_key: String,
    pub element_id: String,
    pub x: f64,
    pub y: f64,
}

impl Command<DiagramDocuments> for MoveC4Element {
    fn label(&self) -> String {
        "Move".to_owned()
    }

    /// Succeeds even when the position could not be recorded, the element having moved, and says so; such a move is
    /// not recorded for undo. Its inverse restores the previous authored position, or clears it when there was none,
    /// so undoing an element's first drag hands it back to the layout.
    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        if store::get_or_load(documents, &self.body_path)
            .workspace
            .find(&self.element_id)
            .is_none()
        {
            return Err(Refusal::new(format!(
                "'{}' is not in this model any more.",
                self.element_id
            )));
        }
        let was = sidecar::read(&self.body_path, &self.view_key).get(&self.element_id);
        let previous = RestoreC4ElementPosition {
            body_path: self.body_path.clone(),
            view_key: self.view_key.clone(),
            element_id: self.element_id.clone(),
            position: was,
        };
        let warning = sidecar::write(&self.body_path, &self.view_key, &self.element_id, self.x, self.y);
        store::touch(documents, &self.body_path);
        Ok(recorded(previous, warning))
    }
}

/// Puts an element back where it was, or back to wherever the layout computes when `position` is none.
#[derive(Debug, Clone, PartialEq)]
pub struct RestoreC4ElementPosition {
    pub body_path: PathBuf,
    pub view_key: String,
    pub element_id: String,
    pub position: Option<(f64, f64)>,
}

impl Command<DiagramDocuments> for RestoreC4ElementPosition {
    fn label(&self) -> String {
        "Move".to_owned()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        let current = sidecar::read(&self.body_path, &self.view_key).get(&self.element_id);
        let warning = match self.position {
            Some((x, y)) => sidecar::write(&self.body_path, &self.view_key, &self.element_id, x, y),
            None => sidecar::remove(&self.body_path, &self.element_id),
        };
        store::touch(documents, &self.body_path);
        // Redoing a restore to the computed layout moves back to where the drag put it; when there was nothing to move
        // back to, the inverse restores nothing.
        let inverse: Box<dyn Command<DiagramDocuments>> = match current {
            Some((x, y)) => Box::new(MoveC4Element {
                body_path: self.body_path.clone(),
                view_key: self.view_key.clone(),
                element_id: self.element_id.clone(),
                x,
                y,
            }),
            None => Box::new(RestoreC4ElementPosition {
                position: None,
                ..self.clone()
            }),
        };
        Ok(Applied {
            inverse: Some(inverse),
            warning,
        })
    }
}

/// A success recorded with `inverse`, or, with a warning, told and not recorded, as the standalone host's history did.
fn recorded(inverse: impl Command<DiagramDocuments> + 'static, warning: String) -> Applied<DiagramDocuments> {
    if warning.is_empty() {
        Applied::undoable(inverse)
    } else {
        Applied::final_change().with_warning(warning)
    }
}

/// Adds one element to a model: inside `parent_id`, or at the top of the model when it is empty, under `identifier`,
/// or one made from its name when that is empty.
#[derive(Debug, Clone, PartialEq)]
pub struct AddC4Element {
    pub body_path: PathBuf,
    pub kind: C4ElementKind,
    pub name: String,
    pub parent_id: String,
    pub identifier: String,
}

impl Command<DiagramDocuments> for AddC4Element {
    fn label(&self) -> String {
        "Add".to_owned()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        if self.name.trim().is_empty() {
            return Err(Refusal::new("An element needs a name."));
        }
        run(documents, &self.body_path, |document, workspace| {
            let name = self.name.trim();
            if let Some(refusal) = placement::refuse(workspace, self.kind, &self.parent_id) {
                return Edit::Refused(refusal);
            }
            let identifier = if self.identifier.is_empty() {
                placement::identifier_for(workspace, name)
            } else {
                self.identifier.clone()
            };
            if workspace.find(&identifier).is_some() {
                return Edit::Refused(format!("This model already has something called '{identifier}'."));
            }
            match placement::try_insert(document, workspace, self.kind, name, &identifier, &self.parent_id) {
                Ok(opened_block) => Edit::UndoneBy(Box::new(RemoveC4Element {
                    body_path: self.body_path.clone(),
                    identifier,
                    collapse_parent: if opened_block {
                        self.parent_id.clone()
                    } else {
                        String::new()
                    },
                })),
                Err(error) => Edit::Refused(error),
            }
        })
    }
}

/// Undoes an [`AddC4Element`]: removes the element it declared, and the block it had to open on its parent.
#[derive(Debug, Clone, PartialEq)]
pub struct RemoveC4Element {
    pub body_path: PathBuf,
    pub identifier: String,
    /// The element whose block the add opened, or empty when it opened none.
    pub collapse_parent: String,
}

impl Command<DiagramDocuments> for RemoveC4Element {
    fn label(&self) -> String {
        "Remove".to_owned()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        run(documents, &self.body_path, |document, workspace| {
            let Some(element) = workspace.find(&self.identifier) else {
                return Edit::Refused("That element is no longer in this model.".to_owned());
            };
            let inverse = AddC4Element {
                body_path: self.body_path.clone(),
                kind: element.kind,
                name: element.name.clone(),
                parent_id: element.parent_id.clone().unwrap_or_default(),
                identifier: element.id.clone(),
            };
            // Resolved before the removal shifts anything.
            let parent = if self.collapse_parent.is_empty() {
                None
            } else {
                workspace.find(&self.collapse_parent)
            };
            if let Err(error) = placement::try_remove(document, element) {
                return Edit::Refused(error);
            }
            if let Some(parent) = parent
                && let Err(error) = placement::collapse_empty_block(document, parent)
            {
                return Edit::Refused(error);
            }
            Edit::UndoneBy(Box::new(inverse))
        })
    }
}

/// The tool type whose registration opens a view of `kind`.
pub fn origin_for(kind: C4ViewKind) -> &'static str {
    match kind {
        C4ViewKind::SystemLandscape => "c4/system-landscape",
        C4ViewKind::SystemContext => "c4/context",
        C4ViewKind::Container => "c4/container",
        C4ViewKind::Component => "c4/component",
        C4ViewKind::Dynamic => "c4/dynamic",
        C4ViewKind::Deployment => "c4/deployment",
    }
}

/// Adds a view to a model, and the registration that opens it: a second diagram of one model.
#[derive(Debug, Clone, PartialEq)]
pub struct AddC4View {
    pub body_path: PathBuf,
    pub registration_path: PathBuf,
    /// The C4 tool type the registration declares.
    pub origin: String,
    pub view_kind: C4ViewKind,
    pub view_key: String,
    /// The body, relative to the registration's folder, as its `body:` header records it.
    pub body_relative_path: String,
}

impl Command<DiagramDocuments> for AddC4View {
    fn label(&self) -> String {
        "Add view".to_owned()
    }

    /// The view goes into the body first; the registration is written only once the body holding the view is, and a
    /// registration that cannot be written takes the view out again.
    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        let added = run(documents, &self.body_path, |document, workspace| {
            if workspace.find_view(&self.view_key).is_some() {
                return Edit::Refused(format!("This model already has a view called '{}'.", self.view_key));
            }
            let Some(insert_at) = placement::end_of_views_block(document) else {
                return Edit::Refused("This document has no views block to add a view to.".to_owned());
            };
            let scope = workspace
                .elements
                .iter()
                .find(|e| e.kind == C4ElementKind::SoftwareSystem)
                .map(|e| e.id.clone());
            if scope.is_none() && self.view_kind != C4ViewKind::SystemLandscape {
                return Edit::Refused("This model has no software system for the view to be about.".to_owned());
            }
            let lines = placement::declare_view(self.view_kind, &self.view_key, scope.as_deref(), workspace);
            for (index, line) in lines.iter().enumerate() {
                if let Err(error) = document.insert_line(insert_at + index as u32, line) {
                    return Edit::Refused(error);
                }
            }
            Edit::UndoneBy(Box::new(RemoveC4View {
                body_path: self.body_path.clone(),
                registration_path: self.registration_path.clone(),
                view_key: self.view_key.clone(),
            }))
        })?;
        let registration = format!(
            "{}\nbody: {}\nview: {}\n",
            self.origin, self.body_relative_path, self.view_key
        );
        if let Err(error) = adp_documents::text::write_atomically(&self.registration_path, registration.as_bytes()) {
            // The view is already in the document; without its registration the job is half done, so undo it.
            let _ = remove_view_saved(documents, &self.body_path, &self.view_key);
            return Err(Refusal::new(format!(
                "Could not create {}: {error}",
                file_name(&self.registration_path)
            )));
        }
        Ok(added)
    }
}

/// Removes a view that was saved into the document at `body_path`, and saves that.
fn remove_view_saved(documents: &mut DiagramDocuments, body_path: &Path, view_key: &str) -> Result<(), String> {
    let mut copy = store::get_or_load(documents, body_path).lines();
    let workspace = parser::parse(&copy);
    let Some(view) = workspace.find_view(view_key) else {
        return Ok(());
    };
    placement::remove_view(&mut copy, view)?;
    store::save(documents, body_path, &copy)
}

/// Undoes an [`AddC4View`]: removes the view block and the registration.
#[derive(Debug, Clone, PartialEq)]
pub struct RemoveC4View {
    pub body_path: PathBuf,
    pub registration_path: PathBuf,
    pub view_key: String,
}

impl Command<DiagramDocuments> for RemoveC4View {
    fn label(&self) -> String {
        "Remove view".to_owned()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        let removed = run(documents, &self.body_path, |document, workspace| {
            let Some(view) = workspace.find_view(&self.view_key) else {
                return Edit::Refused(format!("This model has no view called '{}'.", self.view_key));
            };
            let kind = view.kind;
            if let Err(error) = placement::remove_view(document, view) {
                return Edit::Refused(error);
            }
            // The registration and its body sit side by side, so the file name is the whole relative path.
            Edit::UndoneBy(Box::new(AddC4View {
                body_path: self.body_path.clone(),
                registration_path: self.registration_path.clone(),
                origin: origin_for(kind).to_owned(),
                view_kind: kind,
                view_key: self.view_key.clone(),
                body_relative_path: file_name(&self.body_path),
            }))
        })?;
        if self.registration_path.exists()
            && let Err(error) = std::fs::remove_file(&self.registration_path)
        {
            return Err(Refusal::new(format!(
                "Could not remove {}: {error}",
                file_name(&self.registration_path)
            )));
        }
        Ok(removed)
    }
}
