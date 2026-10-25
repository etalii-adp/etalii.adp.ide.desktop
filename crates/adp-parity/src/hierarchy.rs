//! The replay of a script of a designer of the `hierarchy` kind (etalii.adp spec 036, T021,
//! `contracts/parity-scripts.md`): the document opened through the Hierarchical Requirement Designer's
//! [`RequirementSession`], as `DesignerService.OpenHierarchySurface` opened it in the standalone host, and each step
//! mapped onto the session as the recorder mapped it onto the standalone host's RPCs (`parity/recorder/Replay.Hierarchy.cs`):
//!
//! - `{ "hierarchy": { "operation", "target", "params", "expect" } }` is `DesignerService.HierarchySurfaceGesture`:
//!   the gesture, refused at once or accepted and written; the step is refused when the session refuses the gesture
//!   or its write.
//! - `{ "outside": { "file", "replace": { "from", "to" } } }` writes the file outside the host, as an agent would,
//!   and tells the session the file changed, as the watcher does.
//! - `undo` and `redo` are the project's history, which in the desktop host is the session's.
//! - `{ "action": "hierarchy.add", "args": { "entry", "value", "text" } }` is the explorer's add, as for a diagram: a
//!   new document from the designer's template.

use std::fs;
use std::path::{Path, PathBuf};

use adp_designer_hierarchical_requirement::session::RequirementSession;
use adp_desl::hierarchy::model::{Gesture, SurfaceChange};
use adp_desl::hierarchy::session::HierarchySurfaceSession;
use adp_fbl::RegistrationDocument;
use serde_json::Value as Json;

use crate::{Case, StepDifference, Verdict, compact, compare_files};

/// The tool types whose scripts are replayed as hierarchy surfaces.
pub fn is_hierarchy(tool: &str) -> bool {
    tool == adp_designer_hierarchical_requirement::ORIGIN
}

/// Replays `case` on the copy of its fixture's folder at `project`, opened at `document` (relative to it): the body,
/// or its registration.
pub fn replay(case: &Case, project: &Path, document: &Path) -> Result<Verdict, String> {
    let opened = project.join(document);
    let (body, registration) = files_of(&opened);
    let mut page = Page {
        session: RequirementSession::open(&body, registration),
        project: project.to_owned(),
        next_edit: 0,
    };
    let model = page.session.model();
    if !model.read_only_reason.is_empty() {
        return Ok(Verdict::CannotOpen(model.read_only_reason));
    }

    let mut steps = Vec::new();
    for (index, step) in case.script.steps.iter().enumerate() {
        let refusal_expected = expects_refusal(step);
        let what = match (page.step(step), refusal_expected) {
            (Ok(()), false) | (Err(_), true) => None,
            (Ok(()), true) => Some("the standalone host refused it; the desktop host accepted it".to_owned()),
            (Err(reason), false) => Some(format!("the standalone host accepted it; the desktop host: {reason}")),
        };
        if let Some(what) = what {
            steps.push(StepDifference {
                step: index,
                step_text: compact(step),
                what,
            });
        }
    }
    let files = compare_files(case, project)?;
    Ok(Verdict::Replayed { steps, files })
}

/// The body and the registration a document names: a registration names its body, or has the body of its base name
/// beside it (FBL 0.7 section 8.1); a body is opened with the registration of its base name, when there is one.
fn files_of(opened: &Path) -> (PathBuf, Option<PathBuf>) {
    let is_registration = opened.extension().is_some_and(|e| e.eq_ignore_ascii_case("adp"));
    if !is_registration {
        return (opened.to_owned(), None);
    }
    let named = fs::read(opened)
        .ok()
        .and_then(|bytes| RegistrationDocument::read(bytes).body().map(str::to_owned));
    let body = match named {
        Some(name) => opened.with_file_name(name),
        None => opened.with_extension("md"),
    };
    (body, Some(opened.to_owned()))
}

/// Whether the step says the standalone host refuses it: on the step, or inside its gesture as
/// `contracts/parity-scripts.md` writes it.
fn expects_refusal(step: &Json) -> bool {
    let refused = |value: Option<&Json>| value.and_then(Json::as_str) == Some("refused");
    refused(step.get("expect")) || refused(step.get("hierarchy").and_then(|h| h.get("expect")))
}

/// How many times an undo or a redo step goes: its number, or once for an object, as the contract writes
/// `{"undo": {}}`.
fn count(value: &Json) -> u64 {
    value.as_u64().unwrap_or(1)
}

/// What a page's replay keeps between steps: the session, as the recorder's connection kept its stream.
struct Page {
    session: RequirementSession,
    project: PathBuf,
    next_edit: u64,
}

impl Page {
    fn step(&mut self, step: &Json) -> Result<(), String> {
        if let Some(gesture) = step.get("hierarchy") {
            return self.gesture(gesture);
        }
        if let Some(outside) = step.get("outside") {
            return self.outside(outside);
        }
        if step.get("action").and_then(Json::as_str) == Some("hierarchy.add") {
            return crate::add_entry(&self.project, step.get("args").unwrap_or(&Json::Null));
        }
        if let Some(n) = step.get("undo") {
            return self.history(count(n), true);
        }
        if let Some(n) = step.get("redo") {
            return self.history(count(n), false);
        }
        Err(format!("the runner knows no hierarchy step {}", compact(step)))
    }

    /// `DesignerService.HierarchySurfaceGesture`: the gesture, refused at once or accepted, and its write.
    fn gesture(&mut self, step: &Json) -> Result<(), String> {
        let text = |name: &str| step.get(name).and_then(Json::as_str).unwrap_or_default().to_owned();
        let operation = text("operation");
        if operation.is_empty() {
            return Err("a hierarchy step names its operation".into());
        }
        self.next_edit += 1;
        let gesture = Gesture {
            edit_id: format!("edit-{}", self.next_edit),
            operation,
            target_id: text("target"),
            params: step
                .get("params")
                .and_then(Json::as_object)
                .into_iter()
                .flatten()
                .map(|(name, value)| (name.clone(), value.as_str().unwrap_or_default().to_owned()))
                .collect(),
        };
        self.session.gesture(gesture)?;
        for change in self.session.take_changes() {
            if let SurfaceChange::Outcome(outcome) = change
                && !outcome.written
            {
                return Err(format!("could not be written: {}", outcome.error));
            }
        }
        Ok(())
    }

    /// The file written as another program would, and the session told of it.
    fn outside(&mut self, step: &Json) -> Result<(), String> {
        let file = step
            .get("file")
            .and_then(Json::as_str)
            .ok_or("an outside step names its file")?;
        let replace = step.get("replace").ok_or("an outside step says what it replaces")?;
        let from = replace.get("from").and_then(Json::as_str).unwrap_or_default();
        let to = replace.get("to").and_then(Json::as_str).unwrap_or_default();
        let path = self.project.join(file);
        let text = fs::read_to_string(&path).map_err(|e| format!("{file}: {e}"))?;
        if !text.contains(from) {
            return Err(format!("'{from}' is not in {file}"));
        }
        fs::write(&path, text.replacen(from, to, 1)).map_err(|e| format!("{file}: {e}"))?;
        self.session.file_changed(&path);
        self.session.take_changes();
        Ok(())
    }

    /// `history.undo` or `history.redo` against the project, `n` times.
    fn history(&mut self, n: u64, undo: bool) -> Result<(), String> {
        for _ in 0..n {
            if undo {
                self.session.undo()?;
            } else {
                self.session.redo()?;
            }
        }
        self.session.take_changes();
        Ok(())
    }
}
