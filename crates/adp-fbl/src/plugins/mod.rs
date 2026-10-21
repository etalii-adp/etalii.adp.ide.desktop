//! The persistence plugin contract (FBL sections 11.2 to 11.4; the standalone host's `Plugins/IPersistencePlugin`
//! and `Plugins/_Model`), as data exchanged through a trait so that a host binds it to its own plugin mechanism: a
//! plugin reads and plans, and never writes files, keeps no undo history and stores no view data (FBL section
//! 11.4). The host side is here too: a file body read by a plugin ([`PluginBody`], FBL section 11.3), a folder
//! subject read by one ([`FolderBody`], FBL section 10), and a new body made by a plugin's `template` when the
//! binding has no template text ([`produce`], FBL section 13). No plugin is implemented in this crate.

mod folder_body;
mod plugin_body;

use std::collections::BTreeMap;
use std::path::Path;

pub use folder_body::{FolderBody, FolderBodyChange, SUBJECT_DETAIL};
pub use plugin_body::PluginBody;

use crate::documents::types::FblBinding;
use crate::model::{FblElement, Finding};
use crate::planning::ModelChange;
use crate::splice::Splice;

/// One file a plugin reads: the body itself (`relative_path` empty) or a file of a folder subject.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginFile {
    pub relative_path: String,
    pub bytes: Vec<u8>,
    /// The file is larger than the host reads ([`crate::FblOptions::max_body_bytes`], FBL section 16), so it is
    /// handed by its path with no bytes, and the plugin decides whether that is a finding.
    pub is_oversized: bool,
}

impl PluginFile {
    pub fn new(relative_path: impl Into<String>, bytes: Vec<u8>) -> Self {
        PluginFile {
            relative_path: relative_path.into(),
            bytes,
            is_oversized: false,
        }
    }
}

/// What `read` receives (FBL section 11.2): the body's bytes, or every file a folder subject's file rules select.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PluginReadRequest {
    pub files: Vec<PluginFile>,
    /// The entries a host adds beside the selected files ([`crate::FblOptions::supplementary_entries`]), each a
    /// path relative to the folder, with no bytes, in ordinal order. Outside FBL 0.2; empty for a file body.
    pub supplementary_entries: Vec<String>,
}

/// What `read` delivers: the elements and relations with their source spans, the findings, and whether the body is
/// unreadable.
#[derive(Debug, Clone, Default)]
pub struct PluginReadResult {
    pub elements: Vec<FblElement>,
    pub findings: Vec<Finding>,
    pub unreadable: bool,
}

impl PluginReadResult {
    pub fn new(elements: Vec<FblElement>, findings: Vec<Finding>, unreadable: bool) -> Self {
        PluginReadResult {
            elements,
            findings,
            unreadable,
        }
    }

    pub fn find(&self, id: &str) -> Option<&FblElement> {
        self.elements.iter().find(|e| e.id == id)
    }
}

/// What `plan` receives: the current files, the last `read` result, and one model change.
#[derive(Debug, Clone, Copy)]
pub struct PluginPlanRequest<'a> {
    pub files: &'a [PluginFile],
    pub last: &'a PluginReadResult,
    pub change: &'a ModelChange,
}

/// One splice of a plugin's plan, in the file it names (empty for a file body).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginSplice {
    pub file: String,
    pub splice: Splice,
}

/// What `plan` delivers: splices, or the sentence the host shows when the change is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginPlanResult {
    Planned(Vec<PluginSplice>),
    Refused(String),
}

/// What `template` receives: the new body's name and the template placeholders `name`, `base` and `key` (FBL
/// section 13).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginTemplateRequest {
    pub name: String,
    pub placeholders: BTreeMap<String, String>,
}

/// The persistence plugin contract of FBL section 11.2. A host holds a plugin as `Rc<dyn PersistencePlugin>` and
/// shares it between the bodies it reads; a plugin that keeps state between calls does so behind `&self`.
pub trait PersistencePlugin {
    /// The plugin's id, as a binding's `reader.plugin` names it.
    fn id(&self) -> &str;

    /// `read`: the model of a body, never failing on content.
    fn read(&self, request: &PluginReadRequest) -> PluginReadResult;

    /// `plan`: the splices that realise one model change, or the refusal the host shows.
    fn plan(&self, request: &PluginPlanRequest<'_>) -> PluginPlanResult;

    /// `template`: the bytes of a new body, asked only when the binding has no `template.text`.
    fn template(&self, request: &PluginTemplateRequest) -> Vec<u8>;

    /// `watch`: the paths a folder subject's reading depends on beyond its file rules, given the plugin's last
    /// `read` result. Each path is relative to the folder or full; [`FolderBody::dependencies`] names them for the
    /// host to watch.
    fn watch(&self, last: &PluginReadResult) -> Vec<String> {
        let _ = last;
        Vec::new()
    }
}

/// The text of a new body named `file_name` (FBL section 13; the standalone host's `TemplateWriter.Produce` with a
/// plugin): the binding's template as [`crate::routing::produce`] writes it, else, for a binding read by a plugin,
/// the plugin's `template` operation; none when there is neither.
pub fn produce(
    binding: &FblBinding,
    origin: Option<&str>,
    file_name: &str,
    new_id: &dyn Fn(&str) -> String,
    plugin: Option<&dyn PersistencePlugin>,
) -> Option<Vec<u8>> {
    if let Some(bytes) = crate::routing::produce(binding, origin, file_name, new_id) {
        return Some(bytes);
    }
    let plugin = plugin.filter(|_| binding.plugin.is_some())?;
    Some(plugin.template(&PluginTemplateRequest {
        name: file_name_of(file_name),
        placeholders: placeholders(file_name),
    }))
}

fn file_name_of(file_name: &str) -> String {
    Path::new(file_name)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The values of `{name}`, `{base}` and `{key}` for a new file.
fn placeholders(file_name: &str) -> BTreeMap<String, String> {
    let name = file_name_of(file_name);
    let base = Path::new(&name)
        .file_stem()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let key = crate::routing::key(&base);
    BTreeMap::from([("name".into(), name), ("base".into(), base), ("key".into(), key)])
}

#[cfg(test)]
pub(crate) mod tests;
