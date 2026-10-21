//! Every edit of a `.fdg`, as a command on the project's history (etalii.adp spec 030, task T072).
//!
//! Ported from the standalone host at `8b6ccfc`: the module's `Commands/` (`AddFdgElementCommand.cs`,
//! `ConnectFdgElementsCommand.cs`, `DisconnectFdgConnectionCommand.cs`, `RemoveFdgElementCommand.cs`,
//! `RenameFdgConnectionCommand.cs`, `RenameFdgElementCommand.cs`, `SetFdgDescriptionCommand.cs`,
//! `SetFdgPlacementCommand.cs`, `SetFdgSizeCommand.cs` and their handlers over `FdgEditHandler.cs`), and the core's
//! `DocumentEditRun.cs`, `RestoreDocumentCommand.cs` and `UniqueName.cs`. A command names what it changes by the
//! body's path and ids, never by a live object; it is run on a copy of the body's lines and their reading, so a
//! refusal leaves nothing behind, the copy is written whole, and its undo restores the text the body held before,
//! whose own undo is the command again, under the ids it was given.

use std::path::{Path, PathBuf};

use adp_diagram::DiagramDocuments;
use adp_history::{Applied, Command, Refusal};

use crate::lines::{LineDocument, LineRange};
use crate::model::{FdgConnection, FdgElement, FdgModel, element_types, geometry};
use crate::parser;
use crate::relations;
use crate::store;
use crate::writer;

/// A new element's width, for the four named types.
pub const DEFAULT_WIDTH: f64 = 160.0;
/// A new Comment's width.
pub const COMMENT_WIDTH: f64 = 240.0;
/// A new Comment's height.
pub const COMMENT_HEIGHT: f64 = 96.0;

/// What an edit of an element that is not there answers.
pub const ELEMENT_GONE: &str = "That element is no longer in this graph.";
/// What an edit of a connection that is not there answers.
pub const CONNECTION_GONE: &str = "That connection is no longer in this graph.";
/// What a new element or connection under an id already used answers.
pub const ID_TAKEN: &str = "That id is already used in this graph.";

/// One edit of a `.fdg`.
#[derive(Debug, Clone, PartialEq)]
pub enum FdgEdit {
    /// Adds an element of `element_type` centred on (`x`, `y`), under `element_id`.
    Add {
        element_type: String,
        x: f64,
        y: f64,
        element_id: String,
    },
    /// Removes an element and every connection to or from it, in one edit.
    Remove { element_id: String },
    /// Moves an element so its top-left is (`x`, `y`).
    Place { element_id: String, x: f64, y: f64 },
    /// Resizes an element: a width for any, a height for a Comment only; both clamped to their minimums.
    Size {
        element_id: String,
        width: f64,
        height: Option<f64>,
    },
    /// Links `from` to `to` by `relation`, parent to child, under `connection_id`, once the checks allow it.
    Connect {
        relation: String,
        from: String,
        to: String,
        connection_id: String,
    },
    /// Removes one connection.
    Disconnect { connection_id: String },
    /// Sets an element's name, or a Comment's text.
    Rename { element_id: String, value: String },
    /// Sets the description of an element or a connection; an empty one removes the key.
    Describe { id: String, description: String },
    /// Sets a connection's name.
    RenameConnection { connection_id: String, name: String },
}

/// One command: an edit of the body at `body_path`.
#[derive(Debug, Clone, PartialEq)]
pub struct FdgCommand {
    pub body_path: PathBuf,
    pub edit: FdgEdit,
}

impl FdgCommand {
    pub fn new(body_path: &Path, edit: FdgEdit) -> Self {
        FdgCommand {
            body_path: body_path.to_path_buf(),
            edit,
        }
    }
}

/// The default name of a new element of `element_type`.
fn default_name_of(element_type: &str) -> &'static str {
    match element_type {
        element_types::UI_ELEMENT => "New UI element",
        element_types::ACTION => "New action",
        element_types::DATA_ELEMENT => "New data element",
        element_types::FUNCTION => "New function",
        _ => "New element",
    }
}

/// `name` when no element has it, else the first of `name 2`, `name 3`, … that none has (`UniqueName.Of`).
pub fn unique_name<'a>(taken: impl IntoIterator<Item = &'a str>, name: &str) -> String {
    let names: std::collections::HashSet<&str> = taken.into_iter().collect();
    if !names.contains(name) {
        return name.to_owned();
    }
    (2..)
        .map(|n| format!("{name} {n}"))
        .find(|candidate| !names.contains(candidate.as_str()))
        .expect("some number is free")
}

impl FdgEdit {
    /// Makes the edit on `document`, as `model` reads it.
    fn apply(&self, document: &mut LineDocument, model: &FdgModel) -> writer::EditOutcome {
        let element = |id: &str| model.element_of(id).ok_or_else(|| ELEMENT_GONE.to_owned());
        match self {
            FdgEdit::Add {
                element_type,
                x,
                y,
                element_id,
            } => {
                if model.is_taken(element_id) {
                    return Err(ID_TAKEN.to_owned());
                }
                let is_comment = element_type == element_types::COMMENT;
                let width = if is_comment { COMMENT_WIDTH } else { DEFAULT_WIDTH };
                let height = is_comment.then_some(COMMENT_HEIGHT);
                let drawn_height = height.unwrap_or(geometry::SHARED_HEIGHT);
                // The drop is the centre; the document holds the top-left.
                let element = FdgElement {
                    id: element_id.clone(),
                    element_type: element_type.clone(),
                    name: if is_comment {
                        String::new()
                    } else {
                        unique_name(
                            model.elements.iter().map(|e| e.name.as_str()),
                            default_name_of(element_type),
                        )
                    },
                    description: String::new(),
                    text: if is_comment {
                        "New comment".to_owned()
                    } else {
                        String::new()
                    },
                    x: x - width / 2.0,
                    y: y - drawn_height / 2.0,
                    width,
                    height,
                    range: LineRange::default(),
                };
                writer::add_element(document, model, &element)
            }
            FdgEdit::Remove { element_id } => writer::remove_element(document, model, element(element_id)?),
            FdgEdit::Place { element_id, x, y } => writer::move_to(document, element(element_id)?, *x, *y),
            FdgEdit::Size {
                element_id,
                width,
                height,
            } => writer::resize(document, element(element_id)?, *width, *height),
            FdgEdit::Connect {
                relation,
                from,
                to,
                connection_id,
            } => {
                // Checked against the document as it is now, never against what the canvas believed.
                if let Some(refusal) = relations::refusal_for(model, relation, from, to) {
                    return Err(refusal);
                }
                if model.is_taken(connection_id) {
                    return Err(ID_TAKEN.to_owned());
                }
                let connection = FdgConnection {
                    id: connection_id.clone(),
                    connection_type: relation.clone(),
                    from: from.clone(),
                    to: to.clone(),
                    name: String::new(),
                    description: String::new(),
                    range: LineRange::default(),
                };
                writer::connect(document, model, &connection)
            }
            FdgEdit::Disconnect { connection_id } => match model.connection_of(connection_id) {
                Some(connection) => writer::disconnect(document, connection),
                None => Err(CONNECTION_GONE.to_owned()),
            },
            FdgEdit::Rename { element_id, value } => {
                let element = element(element_id)?;
                if element.is_comment() {
                    writer::set_text(document, element, value)
                } else {
                    writer::set_name(document, element, value)
                }
            }
            FdgEdit::Describe { id, description } => {
                if let Some(element) = model.element_of(id) {
                    return writer::set_description(document, element, description);
                }
                match model.connection_of(id) {
                    Some(connection) => writer::set_connection_description(document, connection, description),
                    None => Err("That is no longer in this graph.".to_owned()),
                }
            }
            FdgEdit::RenameConnection { connection_id, name } => match model.connection_of(connection_id) {
                Some(connection) => writer::set_connection_name(document, connection, name),
                None => Err(CONNECTION_GONE.to_owned()),
            },
        }
    }

    fn label(&self) -> &'static str {
        match self {
            FdgEdit::Add { .. } => "Add",
            FdgEdit::Remove { .. } => "Remove",
            FdgEdit::Place { .. } => "Move",
            FdgEdit::Size { .. } => "Resize",
            FdgEdit::Connect { .. } => "Connect",
            FdgEdit::Disconnect { .. } => "Disconnect",
            FdgEdit::Rename { .. } | FdgEdit::RenameConnection { .. } => "Rename",
            FdgEdit::Describe { .. } => "Describe",
        }
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

impl Command<DiagramDocuments> for FdgCommand {
    fn label(&self) -> String {
        self.edit.label().to_owned()
    }

    /// The edit run: the body loaded, the edit made on a copy of its lines and their reading, the copy written and
    /// read again, and the text it held before recorded as the undo.
    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        if let FdgEdit::Add { element_type, .. } = &self.edit
            && !element_types::is_known(element_type)
        {
            return Err(Refusal::new(format!("This notation has no `{element_type}` element.")));
        }
        let document = store::get_or_load(documents, &self.body_path);
        if let Some(reason) = &document.unreadable {
            return Err(Refusal::new(format!(
                "{} could not be read, so nothing can be edited until it can: {reason}",
                file_name(&self.body_path)
            )));
        }
        let before = document.text.clone();
        let mut copy = LineDocument::parse(&before);
        let model = parser::parse(&copy);
        self.edit.apply(&mut copy, &model).map_err(Refusal::new)?;
        store::save(documents, &self.body_path, &copy.text()).map_err(Refusal::new)?;
        Ok(Applied {
            inverse: Some(Box::new(RestoreFdgDocument {
                body_path: self.body_path.clone(),
                text: before,
                redo: self.clone(),
            })),
            warning: String::new(),
        })
    }
}

/// The undo of an edit: the text the body held before it, whose own undo is the edit again.
#[derive(Debug, Clone, PartialEq)]
pub struct RestoreFdgDocument {
    pub body_path: PathBuf,
    pub text: String,
    pub redo: FdgCommand,
}

impl Command<DiagramDocuments> for RestoreFdgDocument {
    fn label(&self) -> String {
        self.redo.label()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        store::write(documents, &self.body_path, &self.text)
            .map_err(|e| Refusal::new(format!("Could not restore the file: {e}")))?;
        Ok(Applied {
            inverse: Some(Box::new(self.redo.clone())),
            warning: String::new(),
        })
    }
}
