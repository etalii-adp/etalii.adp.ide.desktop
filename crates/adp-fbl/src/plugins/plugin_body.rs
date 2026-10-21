//! A file body read by a persistence plugin (FBL section 11.3; the standalone host's `Plugins/PluginBody`): the
//! plugin reads and plans, and this host side applies the splices, keeps the history, checks drift and saves,
//! exactly as for a declared body. Without the plugin the body opens read-only with DISL's `std.pluginMissing`
//! (FBL section 15.1).

use std::rc::Rc;

use crate::documents::types::FblBinding;
use crate::history::{SplicedFile, UndoResult};
use crate::model::{FblModel, Finding, FindingSeverity, SourceLocation, codes};
use crate::planning::{ModelChange, PlanResult, PlannedEdit};
use crate::splice::Splice;

use super::{PersistencePlugin, PluginFile, PluginPlanRequest, PluginPlanResult, PluginReadRequest, PluginReadResult};

/// One open file body whose binding names a plugin: its bytes, the plugin's last reading, and its one history.
pub struct PluginBody {
    file: SplicedFile,
    binding: Rc<FblBinding>,
    plugin: Option<Rc<dyn PersistencePlugin>>,
    file_name: String,
    last: PluginReadResult,
}

impl std::fmt::Debug for PluginBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginBody")
            .field("binding", &self.binding.name)
            .field("file_name", &self.file_name)
            .finish_non_exhaustive()
    }
}

impl PluginBody {
    /// Opens a body whose binding names a plugin. `plugin` is the one the caller has installed, or none; a plugin
    /// with another id counts as missing. A binding read by its declared rules is the caller's error.
    pub fn open(
        bytes: Vec<u8>,
        binding: Rc<FblBinding>,
        plugin: Option<Rc<dyn PersistencePlugin>>,
        file_name: &str,
    ) -> Result<PluginBody, String> {
        let Some(reader) = &binding.plugin else {
            return Err(format!(
                "The binding '{}' is read by its declared rules, not by a plugin.",
                binding.name
            ));
        };
        let plugin = plugin.filter(|p| p.id() == reader.plugin);
        let mut body = PluginBody {
            file: SplicedFile::new(bytes),
            binding,
            plugin,
            file_name: file_name.to_owned(),
            last: PluginReadResult::default(),
        };
        body.reread();
        Ok(body)
    }

    pub fn bytes(&self) -> &[u8] {
        self.file.bytes()
    }

    pub fn model(&self) -> FblModel {
        FblModel {
            elements: self.last.elements.clone(),
            findings: self.last.findings.clone(),
            unreadable: self.last.unreadable,
            ..FblModel::default()
        }
    }

    /// Read-only without the plugin, when the plugin reports the body unreadable, or when the binding is read-only.
    pub fn is_read_only(&self) -> bool {
        self.plugin.is_none() || self.last.unreadable || self.binding.read_only.is_some()
    }

    pub fn can_undo(&self) -> bool {
        self.file.can_undo()
    }

    /// The plan for one model change, without applying it. A read-only binding's plugin is never asked to plan
    /// (FBL section 11.4).
    pub fn plan(&self, change: &ModelChange) -> PlanResult {
        let Some(plugin) = &self.plugin else {
            return PlanResult::Refused(self.missing_reason());
        };
        if self.last.unreadable {
            return PlanResult::Refused("The file could not be read, so it is never written.".into());
        }
        if let Some(reason) = &self.binding.read_only {
            return PlanResult::Refused(if reason.is_empty() {
                "This file is read-only.".into()
            } else {
                reason.clone()
            });
        }
        if matches!(change, ModelChange::Save) {
            return PlanResult::Planned(PlannedEdit::default());
        }
        let files = [PluginFile::new("", self.file.bytes().to_vec())];
        let result = plugin.plan(&PluginPlanRequest {
            files: &files,
            last: &self.last,
            change,
        });
        match result {
            PluginPlanResult::Refused(reason) => PlanResult::Refused(reason),
            PluginPlanResult::Planned(splices) if splices.iter().any(|s| !s.file.is_empty()) => {
                PlanResult::Refused("The plugin planned a change to another file than the body.".into())
            }
            PluginPlanResult::Planned(splices) => match order(splices.into_iter().map(|s| s.splice).collect()) {
                Ok(splices) => PlanResult::Planned(PlannedEdit {
                    splices,
                    snapshot: false,
                }),
                Err(problem) => PlanResult::Invalid(problem),
            },
        }
    }

    /// Plans and applies `change`: the planned edit, or the refusal with nothing written.
    pub fn change(&mut self, change: &ModelChange) -> PlanResult {
        let result = self.plan(change);
        if let PlanResult::Planned(edit) = &result {
            match self.file.apply(edit) {
                Ok(true) => self.reread(),
                Ok(false) => {}
                Err(problem) => return PlanResult::Invalid(problem),
            }
        }
        result
    }

    /// Undoes the most recent edit, refused when `current`, the file as it is now, is not what the edit left.
    pub fn undo(&mut self, current: Option<&[u8]>) -> UndoResult {
        let result = self.file.undo(current);
        if let UndoResult::Done(_) = result {
            self.reread();
        }
        result
    }

    /// Redoes the most recently undone edit, refused on drift as [`PluginBody::undo`] is.
    pub fn redo(&mut self, current: Option<&[u8]>) -> UndoResult {
        let result = self.file.redo(current);
        if let UndoResult::Done(_) = result {
            self.reread();
        }
        result
    }

    /// Hands the body's bytes to the host's atomic writer; never a read-only body.
    pub fn save(&self, write: impl FnOnce(&[u8])) -> Result<(), String> {
        if self.is_read_only() {
            return Err("A read-only body is never written.".into());
        }
        write(self.file.bytes());
        Ok(())
    }

    fn missing_reason(&self) -> String {
        format!(
            "The plugin '{}' that reads this file is not installed, so it is opened read-only.",
            self.binding
                .plugin
                .as_ref()
                .map(|p| p.plugin.as_str())
                .unwrap_or_default()
        )
    }

    fn reread(&mut self) {
        self.last = match &self.plugin {
            None => PluginReadResult::new(
                Vec::new(),
                vec![Finding {
                    code: codes::PLUGIN_MISSING.into(),
                    severity: FindingSeverity::Warning,
                    message: self.missing_reason(),
                    location: Some(SourceLocation {
                        file: self.file_name.clone(),
                        line: 1,
                        column: 1,
                        length: 0,
                    }),
                    detail: None,
                }],
                false,
            ),
            Some(plugin) => plugin.read(&PluginReadRequest {
                files: vec![PluginFile::new("", self.file.bytes().to_vec())],
                supplementary_entries: Vec::new(),
            }),
        };
    }
}

/// A plugin's splices in body order, as FBL section 6.5 applies them; overlapping splices are the plugin's error.
fn order(mut splices: Vec<Splice>) -> Result<Vec<Splice>, String> {
    splices.sort_by_key(|s| s.start);
    for pair in splices.windows(2) {
        if pair[1].start < pair[0].end {
            return Err(format!(
                "The plugin planned overlapping splices: {} and {}.",
                record(&pair[0]),
                record(&pair[1])
            ));
        }
    }
    Ok(splices)
}

/// A splice as the standalone host's record prints one.
fn record(splice: &Splice) -> String {
    format!(
        "Splice {{ Operation = {:?}, Start = {}, End = {}, Text = {} }}",
        splice.operation, splice.start, splice.end, splice.text
    )
}
