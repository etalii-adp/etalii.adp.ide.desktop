//! The open chart folders of a project: one reading per registration, shared by every view of it (etalii.adp spec 030,
//! task T080).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/helm-chart/backend/EtAlii.Adp.Diagram.HelmChart/
//! HelmChartStore.cs`, `HelmBody.cs` and `HelmReading.cs`. A folder is read through the FBL library's folder body
//! (`adp_fbl::plugins::FolderBody`) with the module's binding, its plugin and the directories [`crate::entries`] lists,
//! and the definition's model of each reading is built once. The folder is keyed by its registration, the file that made it a diagram and
//! the one a position is stored in. A change on disk reaches it through the project's reconciliation
//! ([`ModuleDocuments`]): a change to the registration is read again without reading the folder, a change the reading
//! depends on reads the folder again, and the store's own write of the registration is ignored.

use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_diagram::{DiagramDocuments, DiskChange, ModuleDocuments};
use adp_disl::DislModel;
use adp_fbl::FblOptions;
use adp_fbl::plugins::FolderBody;

use adp_fbl::FblModel;

use crate::definition::HelmDefinition;
use crate::entries;
use crate::plugin::HelmChartFolderPlugin;

/// The key the open folders are kept under in the project's documents.
pub const STATE_KEY: &str = "helm-chart";

/// One registered chart folder: its reading, the model of it, and its registration's text.
#[derive(Debug)]
pub struct HelmFolder {
    /// The registration's full path.
    pub registration: PathBuf,
    /// The folder it registers.
    pub folder: PathBuf,
    body: FolderBody,
    /// What the folder body read last.
    pub reading: FblModel,
    /// The definition's model of the last reading.
    pub model: DislModel,
    /// The registration's text as last read or written.
    pub registration_text: String,
}

/// What the folder body is told beside the binding: the directories no file rule can select (gap 3).
pub fn options() -> FblOptions {
    FblOptions {
        supplementary_entries: Some(Rc::new(entries::of)),
        ..FblOptions::default()
    }
}

impl HelmFolder {
    /// Opens the folder `registration` sits in.
    pub fn open(registration: &Path) -> HelmFolder {
        let definition = HelmDefinition::get();
        let folder = registration.parent().map(Path::to_path_buf).unwrap_or_default();
        let body = FolderBody::open(
            &folder,
            definition.binding.clone(),
            Some(Rc::new(HelmChartFolderPlugin)),
            Rc::new(options()),
            Some(registration),
        )
        .expect("the binding is a folder binding read by its plugin");
        let reading = body.model();
        let model = definition.model(&reading);
        HelmFolder {
            registration: registration.to_path_buf(),
            folder,
            body,
            reading,
            model,
            registration_text: read_text(registration),
        }
    }

    /// Whether the folder is a chart: the folder body found it by the binding's `recognise` and could read it.
    pub fn is_chart(&self) -> bool {
        !self.reading.unreadable
    }

    /// Reads the folder again.
    pub fn reread(&mut self) {
        self.body.reread();
        self.reading = self.body.model();
        self.model = HelmDefinition::get().model(&self.reading);
    }

    /// What the last reading delivered, to tell a reading that changed from one that did not.
    fn fingerprint(&self) -> String {
        let model = &self.reading;
        format!("{:?}{:?}{}", model.elements, model.findings, model.unreadable)
    }

    /// The full paths the reading depends on.
    pub fn dependencies(&self) -> Vec<PathBuf> {
        self.body.dependencies()
    }
}

fn read_text(path: &Path) -> String {
    fs::read(path)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default()
}

/// Whether two paths name one entry, ignoring case where the platform does.
fn same_path(a: &Path, b: &Path) -> bool {
    if adp_fbl::routing::platform_ignores_case() {
        a.to_string_lossy().eq_ignore_ascii_case(&b.to_string_lossy())
    } else {
        a == b
    }
}

fn is_under(path: &Path, folder: &Path) -> bool {
    let (path, folder) = (path.to_string_lossy(), folder.to_string_lossy());
    let prefix = format!("{}{}", folder.trim_end_matches(['/', '\\']), std::path::MAIN_SEPARATOR);
    path.len() > prefix.len()
        && if adp_fbl::routing::platform_ignores_case() {
            path[..prefix.len()].eq_ignore_ascii_case(&prefix)
        } else {
            path.starts_with(&prefix)
        }
}

/// The open folders of one project.
#[derive(Debug, Default)]
pub struct HelmFolders {
    folders: Vec<HelmFolder>,
}

impl HelmFolders {
    pub fn get(&self, registration: &Path) -> Option<&HelmFolder> {
        self.folders.iter().find(|f| same_path(&f.registration, registration))
    }

    fn get_mut(&mut self, registration: &Path) -> Option<&mut HelmFolder> {
        self.folders
            .iter_mut()
            .find(|f| same_path(&f.registration, registration))
    }
}

impl ModuleDocuments for HelmFolders {
    /// The registration read again when it holds something else; the folder read again when a file its reading
    /// depends on changed.
    fn reconcile(&mut self, path: &Path) -> DiskChange {
        if let Some(folder) = self.get_mut(path) {
            let text = read_text(path);
            if !path.exists() {
                return DiskChange::Deleted(folder.registration.clone());
            }
            if text == folder.registration_text {
                return DiskChange::Unchanged;
            }
            folder.registration_text = text;
            return DiskChange::Reloaded(folder.registration.clone());
        }
        for folder in &mut self.folders {
            let concerned = folder
                .dependencies()
                .iter()
                .any(|d| same_path(d, path) || is_under(path, d))
                || is_under(path, &folder.folder);
            if !concerned {
                continue;
            }
            let before = folder.fingerprint();
            folder.reread();
            return if folder.fingerprint() == before {
                DiskChange::Unchanged
            } else {
                DiskChange::Reloaded(folder.registration.clone())
            };
        }
        DiskChange::NotTracked
    }

    fn close(&mut self, body_path: &Path) {
        self.folders.retain(|f| !same_path(&f.registration, body_path));
    }
}

/// The folder of `registration` as the project's documents hold it, or none before it was opened.
pub fn get<'a>(documents: &'a DiagramDocuments, registration: &Path) -> Option<&'a HelmFolder> {
    documents.module_state_ref::<HelmFolders>(STATE_KEY)?.get(registration)
}

/// The folder of `registration`, read the first time it is asked for.
pub fn get_or_load<'a>(documents: &'a mut DiagramDocuments, registration: &Path) -> &'a HelmFolder {
    let store = documents.module_state::<HelmFolders>(STATE_KEY);
    let at = match store
        .folders
        .iter()
        .position(|f| same_path(&f.registration, registration))
    {
        Some(at) => at,
        None => {
            store.folders.push(HelmFolder::open(registration));
            store.folders.len() - 1
        }
    };
    &store.folders[at]
}

/// Writes `text` as the registration at `registration`, so every view of it draws the stored positions; why when the
/// write failed, with nothing changed.
pub fn save_registration(documents: &mut DiagramDocuments, registration: &Path, text: &str) -> Result<(), String> {
    adp_documents::text::write_atomically(registration, text.as_bytes())
        .map_err(|e| format!("Could not store the position: {e}"))?;
    if let Some(folder) = documents.module_state::<HelmFolders>(STATE_KEY).get_mut(registration) {
        folder.registration_text = text.to_owned();
    }
    documents.mark_changed(registration);
    Ok(())
}
