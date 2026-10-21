//! The one edit of a .NET dependency graph, a position stored in its registration, as a command on the project's
//! history (etalii.adp spec 030, task T082).
//!
//! Ported from the standalone host's core at `8b6ccfc`: `src/backend/EtAlii.Adp.Hierarchy/Commands/
//! SetRegistrationLayoutCommandHandler.cs`, which `DotNetDependencyGraphSession.MoveElementToAsync` dispatches. The
//! inverse restores what was there before: the prior position, or its absence, so undoing an element's first move
//! leaves the registration without its entry (and without the block, when it was the last). No solution or project file
//! is ever written.

use std::fs;
use std::path::{Path, PathBuf};

use adp_diagram::DiagramDocuments;
use adp_history::{Applied, Command, Refusal};

use crate::registration_layout;
use crate::store;

/// The refusal of a registration that is gone.
pub const GONE: &str = "The registration file is no longer there.";

/// Stores one element's position in its registration's `layout:` block (`SetRegistrationLayoutCommand`).
#[derive(Debug, Clone, PartialEq)]
pub struct SetRegistrationLayout {
    pub registration: PathBuf,
    pub element_id: String,
    pub x: f64,
    pub y: f64,
}

/// Removes one element's entry from the `layout:` block (`RemoveRegistrationLayoutCommand`), the inverse of a first
/// position.
#[derive(Debug, Clone, PartialEq)]
pub struct RemoveRegistrationLayout {
    pub registration: PathBuf,
    pub element_id: String,
}

fn read(registration: &Path) -> Result<String, Refusal> {
    if !registration.is_file() {
        return Err(Refusal::new(GONE));
    }
    fs::read(registration)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .map_err(|e| Refusal::new(format!("Could not store the position: {e}")))
}

impl Command<DiagramDocuments> for SetRegistrationLayout {
    fn label(&self) -> String {
        "Move".to_owned()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        let text = read(&self.registration)?;
        let (written, prior) = registration_layout::set_position(&text, &self.element_id, (self.x, self.y));
        store::save_registration(documents, &self.registration, &written).map_err(Refusal::new)?;
        Ok(match prior {
            Some((x, y)) => Applied::undoable(SetRegistrationLayout { x, y, ..self.clone() }),
            None => Applied::undoable(RemoveRegistrationLayout {
                registration: self.registration.clone(),
                element_id: self.element_id.clone(),
            }),
        })
    }
}

impl Command<DiagramDocuments> for RemoveRegistrationLayout {
    fn label(&self) -> String {
        "Move".to_owned()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        let text = read(&self.registration)?;
        let Some(&(x, y)) = registration_layout::read(&text).get(&self.element_id) else {
            return Err(Refusal::new("That element has no stored position any more."));
        };
        let written = registration_layout::remove_position(&text, &self.element_id).unwrap_or(text);
        store::save_registration(documents, &self.registration, &written)
            .map_err(|e| Refusal::new(e.replace("store", "remove")))?;
        Ok(Applied::undoable(SetRegistrationLayout {
            registration: self.registration.clone(),
            element_id: self.element_id.clone(),
            x,
            y,
        }))
    }
}
