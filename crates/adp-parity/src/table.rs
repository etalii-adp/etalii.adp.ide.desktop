//! The replay of a designer's script (etalii.adp spec 030, Story 4, T063): the document opened through the Knowledge
//! designer's [`KnowledgeSession`], as `DesignerService.OpenTable` opened it in the standalone host, and each step
//! mapped onto the session as the recorder mapped it onto the standalone host's RPCs (`parity/recorder/README.md`,
//! "Tables"):
//!
//! - `{ "table": { "kind", "row", "column", "view", "values", "index", "target", "settings" } }` is
//!   `DesignerService.Edit`: the gesture, said in the view the connection shows unless it names one, then written as
//!   the standalone host writes it behind; the step is refused when the session refuses the gesture or its write.
//! - `{ "view": "<id>" }` is `DesignerService.SetTableView`, which writes the view the file is left in.
//! - `undo` and `redo` are the project's history, which in the desktop host is the session's: every edit of these
//!   scripts goes through one session.
//! - `$N` names the N-th id the standalone host generated, and an edit that makes something - a row, an option, a
//!   view - is handed the id the standalone host generated at the same step, from `recorded.json`.

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use adp_designer_knowledge::{KnowledgeSession, project};
use adp_desl::table::model::{TableChange, TableGesture};
use adp_desl::table::session::DesignerSession;
use adp_disl::IdSource;
use serde_json::Value as Json;

use crate::{Case, RecordedIds, StepDifference, Verdict, compact, compare_files};

/// The tool types whose scripts are replayed as tables.
pub fn is_table(tool: &str) -> bool {
    tool == adp_designer_knowledge::ORIGIN
}

/// Replays `case` on the copy of its fixture's folder at `project`, opened at `document` (relative to it).
pub fn replay(case: &Case, project: &Path, document: &Path) -> Result<Verdict, String> {
    let opened = project.join(document);
    let Some(body) = project::body_of(&opened) else {
        return Ok(Verdict::CannotOpen(format!(
            "{} is no knowledge table",
            document.display()
        )));
    };
    let mut session = KnowledgeSession::open_for_editing(&body, project);
    if !session.refusal().is_empty() {
        return Ok(Verdict::CannotOpen(session.refusal().to_owned()));
    }
    // The recorder asked for the rows in sight, as a tab does once it is laid out.
    let baseline = session.baseline();
    if !baseline.read_only_reason.is_empty() {
        return Ok(Verdict::CannotOpen(baseline.read_only_reason));
    }
    session.set_window(0, 60);
    session.take_changes();

    let ids = Rc::new(RecordedIds::new(case.recorded.generated.clone()));
    let unrecorded = Rc::new(RefCell::new(Vec::<String>::new()));
    {
        let (ids, unrecorded) = (ids.clone(), unrecorded.clone());
        session.set_id_source(Box::new(move || {
            ids.next("id").unwrap_or_else(|reason| {
                unrecorded.borrow_mut().push(reason);
                adp_designer_knowledge::definition::new_id()
            })
        }));
    }

    let mut table = Table {
        session,
        ids: ids.clone(),
        next_edit: 0,
    };
    let mut steps = Vec::new();
    for (index, step) in case.script.steps.iter().enumerate() {
        ids.step.set(index);
        let refusal_expected = step.get("expect").and_then(Json::as_str) == Some("refused");
        let mut result = table.step(step);
        if let Some(reason) = unrecorded.borrow_mut().drain(..).next() {
            result = Err(reason);
        }
        let what = match (result, refusal_expected) {
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
    table.session.close();
    let files = compare_files(case, project)?;
    Ok(Verdict::Replayed { steps, files })
}

/// What a table's replay keeps between steps: the session, as the recorder's connection kept its stream.
struct Table {
    session: KnowledgeSession,
    ids: Rc<RecordedIds>,
    next_edit: u64,
}

impl Table {
    fn step(&mut self, step: &Json) -> Result<(), String> {
        if let Some(gesture) = step.get("table") {
            return self.edit(gesture);
        }
        if let Some(view) = step.get("view").and_then(Json::as_str) {
            let view = self.ids.resolve(view)?;
            self.session.set_active_view(&view);
            return self.written();
        }
        if let Some(n) = step.get("undo").and_then(Json::as_u64) {
            return self.history(n, true);
        }
        if let Some(n) = step.get("redo").and_then(Json::as_u64) {
            return self.history(n, false);
        }
        if step.get("save").and_then(Json::as_bool) == Some(true) {
            return Ok(());
        }
        Err(format!("the runner knows no table step {}", compact(step)))
    }

    /// `DesignerService.Edit`: the gesture, accepted or refused at once, and its write.
    fn edit(&mut self, step: &Json) -> Result<(), String> {
        let text = |name: &str| -> Result<String, String> {
            match step.get(name).and_then(Json::as_str) {
                Some(value) => self.ids.resolve(value),
                None => Ok(String::new()),
            }
        };
        let kind = step
            .get("kind")
            .and_then(Json::as_str)
            .ok_or("a table step names its gesture's kind")?;
        let values = step
            .get("values")
            .and_then(Json::as_array)
            .map(|values| {
                values
                    .iter()
                    .map(|v| self.ids.resolve(v.as_str().unwrap_or_default()))
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?
            .unwrap_or_default();
        let view = text("view")?;
        let mut gesture = TableGesture::new(kind)
            .row(text("row")?)
            .column(text("column")?)
            // Every gesture is said in the view it was made in, as the Knowledge designer's panel sends it.
            .view(if view.is_empty() {
                self.session.active_view_id().to_owned()
            } else {
                view
            })
            .values(values)
            .index(step.get("index").and_then(Json::as_i64).unwrap_or(0))
            .target(text("target")?);
        if let Some(settings) = step.get("settings").and_then(Json::as_object) {
            for (name, value) in settings {
                gesture = gesture.setting(name, self.ids.resolve(value.as_str().unwrap_or_default())?);
            }
        }
        self.next_edit += 1;
        let answer = self.session.edit(&format!("edit-{}", self.next_edit), &gesture);
        if !answer.is_empty() {
            return Err(answer);
        }
        self.written()
    }

    /// Writes what waits, as the standalone host writes behind, and says whether every write landed.
    fn written(&mut self) -> Result<(), String> {
        self.session.write_pending();
        for change in self.session.take_changes() {
            if let TableChange::EditSettled {
                written: false, error, ..
            } = change
            {
                return Err(format!("could not be written: {error}"));
            }
        }
        Ok(())
    }

    /// `history.undo` or `history.redo` against the project, `n` times.
    fn history(&mut self, n: u64, undo: bool) -> Result<(), String> {
        self.session.write_pending();
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
