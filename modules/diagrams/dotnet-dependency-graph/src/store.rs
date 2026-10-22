//! The open solutions of a project: one derived graph per solution, shared by every view of it, and the registrations
//! that store their positions (etalii.adp spec 030, task T082).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/dotnet-dependency-graph/backend/
//! EtAlii.Adp.Diagram.DotNetDependencyGraph/DependencyGraphStore.cs`. What is watched is what was read: the solution,
//! every project file it resolved, and every `Directory.Packages.props` above them (`WatchedFiles`), since a version
//! bump lands in a props file and in no project file at all. A change on disk reaches the store through the project's
//! reconciliation ([`ModuleDocuments`]): a change to a registration is read again without reading the solution, a
//! change the graph was derived from derives it again, and the store's own write of a registration is ignored. Nothing
//! here writes but a registration.

use std::fs;
use std::path::{Path, PathBuf};

use adp_diagram::{DiagramDocuments, DiskChange, ModuleDocuments};

use crate::descriptions::PackageDescriptionReader;
use crate::graph::{self, DependencyGraphModel};
use crate::{paths, project, solution};

/// The key the open solutions are kept under in the project's documents.
pub const STATE_KEY: &str = "dotnet-dependency-graph";

/// The name of the file central package versions are kept in.
const CENTRAL_PACKAGE_VERSIONS_FILE: &str = "directory.packages.props";

/// One open solution: its graph, and the files the graph was derived from.
#[derive(Debug)]
pub struct OpenSolution {
    /// The solution's full path.
    pub path: PathBuf,
    /// The graph as last derived.
    pub model: DependencyGraphModel,
    watched: Vec<PathBuf>,
    /// The folders of the projects it resolved, under which a new props file changes a version.
    folders: Vec<PathBuf>,
}

impl OpenSolution {
    /// Derives the graph of the solution at `path`.
    pub fn open(path: &Path) -> OpenSolution {
        let mut open = OpenSolution {
            path: path.to_path_buf(),
            model: DependencyGraphModel::default(),
            watched: Vec::new(),
            folders: Vec::new(),
        };
        open.reread();
        open
    }

    /// Derives the graph again, and what it was derived from.
    pub fn reread(&mut self) {
        self.model = graph::load(&self.path, &PackageDescriptionReader::default());
        let reading = solution::read(&self.path);
        let mut watched = vec![self.path.clone()];
        let mut folders = Vec::new();
        for project in &reading.projects {
            watched.push(project.absolute_path.clone());
            watched.extend(project::central_package_files(&project.absolute_path));
            if let Some(folder) = project.absolute_path.parent() {
                folders.push(folder.to_path_buf());
            }
        }
        self.watched = watched;
        self.folders = folders;
    }

    /// Whether a change of the file at `path` can change the graph: a file it was derived from, or a props file
    /// appearing above one of its projects.
    fn depends_on(&self, path: &Path) -> bool {
        if self.watched.iter().any(|w| paths::same(w, path)) {
            return true;
        }
        let is_props = path
            .file_name()
            .is_some_and(|n| n.to_string_lossy().to_lowercase() == CENTRAL_PACKAGE_VERSIONS_FILE);
        is_props
            && path.parent().is_some_and(|parent| {
                let parent = paths::key(parent);
                self.folders.iter().any(|f| {
                    let folder = paths::key(f);
                    folder == parent || folder.starts_with(&format!("{parent}{}", std::path::MAIN_SEPARATOR))
                })
            })
    }
}

/// One registration of a solution: its path, the solution it names, and its text as last read or written.
#[derive(Debug)]
struct Registration {
    path: PathBuf,
    solution: PathBuf,
    text: String,
}

fn read_text(path: &Path) -> String {
    fs::read(path)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default()
}

/// The open solutions of one project, and their registrations.
#[derive(Debug, Default)]
pub struct OpenSolutions {
    solutions: Vec<OpenSolution>,
    registrations: Vec<Registration>,
}

impl OpenSolutions {
    pub fn get(&self, solution: &Path) -> Option<&OpenSolution> {
        self.solutions.iter().find(|s| paths::same(&s.path, solution))
    }

    /// The text of `registration` as last read or written; empty when it is not open.
    pub fn registration_text(&self, registration: &Path) -> &str {
        self.registrations
            .iter()
            .find(|r| paths::same(&r.path, registration))
            .map_or("", |r| r.text.as_str())
    }
}

impl ModuleDocuments for OpenSolutions {
    /// A registration read again when it holds something else; a graph derived again when a file it depends on changed.
    fn reconcile(&mut self, path: &Path) -> DiskChange {
        if let Some(registration) = self.registrations.iter_mut().find(|r| paths::same(&r.path, path)) {
            let text = read_text(path);
            if text == registration.text {
                return DiskChange::Unchanged;
            }
            registration.text = text;
            return DiskChange::Reloaded(registration.solution.clone());
        }
        let mut tracked = false;
        let mut changed = None;
        for solution in &mut self.solutions {
            if !solution.depends_on(path) {
                continue;
            }
            tracked = true;
            let before = solution.model.clone();
            solution.reread();
            if solution.model != before && changed.is_none() {
                changed = Some(solution.path.clone());
            }
        }
        match (tracked, changed) {
            (_, Some(solution)) => DiskChange::Reloaded(solution),
            (true, None) => DiskChange::Unchanged,
            (false, None) => DiskChange::NotTracked,
        }
    }

    fn close(&mut self, body_path: &Path) {
        self.solutions.retain(|s| !paths::same(&s.path, body_path));
        self.registrations.retain(|r| !paths::same(&r.solution, body_path));
    }
}

/// The solution at `solution` as the project's documents hold it, or none before it was opened.
pub fn get<'a>(documents: &'a DiagramDocuments, solution: &Path) -> Option<&'a OpenSolution> {
    documents.module_state_ref::<OpenSolutions>(STATE_KEY)?.get(solution)
}

/// The text of `registration` as the project's documents hold it.
pub fn registration_text<'a>(documents: &'a DiagramDocuments, registration: &Path) -> &'a str {
    documents
        .module_state_ref::<OpenSolutions>(STATE_KEY)
        .map_or("", |s| s.registration_text(registration))
}

/// Opens the solution at `solution`, derived the first time it is asked for, with `registration` storing its positions.
pub fn open(documents: &mut DiagramDocuments, solution: &Path, registration: Option<&Path>) {
    let store = documents.module_state::<OpenSolutions>(STATE_KEY);
    if store.get(solution).is_none() {
        store.solutions.push(OpenSolution::open(solution));
    }
    if let Some(registration) = registration {
        match store
            .registrations
            .iter_mut()
            .find(|r| paths::same(&r.path, registration))
        {
            Some(open) => open.text = read_text(registration),
            None => store.registrations.push(Registration {
                path: registration.to_path_buf(),
                solution: solution.to_path_buf(),
                text: read_text(registration),
            }),
        }
    }
}

/// Writes `text` as the registration at `registration`, so every view of its solution draws the stored positions; why
/// when the write failed, with nothing changed.
pub fn save_registration(documents: &mut DiagramDocuments, registration: &Path, text: &str) -> Result<(), String> {
    adp_documents::text::write_atomically(registration, text.as_bytes())
        .map_err(|e| format!("Could not store the position: {e}"))?;
    let store = documents.module_state::<OpenSolutions>(STATE_KEY);
    let solution = match store
        .registrations
        .iter_mut()
        .find(|r| paths::same(&r.path, registration))
    {
        Some(open) => {
            open.text = text.to_owned();
            Some(open.solution.clone())
        }
        None => None,
    };
    if let Some(solution) = solution {
        documents.mark_changed(&solution);
    }
    Ok(())
}
