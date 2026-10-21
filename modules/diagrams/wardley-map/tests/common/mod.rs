//! What the Wardley map module's tests share: the fixtures of the standalone host's
//! `src/diagrams/wardley-map/backend/EtAlii.Adp.Diagram.WardleyMap.Tests/Fixtures` at `8b6ccfc`, copied under
//! `tests/fixtures/` byte for byte, and a project in a temporary folder holding a map.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use adp_diagram_wardley_map::lines::LineDocument;
use adp_diagram_wardley_map::model::WardleyMap;
use adp_diagram_wardley_map::parser;

/// Where the fixtures are.
pub fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// The text of fixture `name`.
pub fn fixture(name: &str) -> String {
    fs::read_to_string(fixtures().join(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// Every `.owm` fixture's name, in ordinal order.
pub fn corpus() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(fixtures())
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".owm"))
        .collect();
    names.sort();
    names
}

pub fn parse_text(text: &str) -> WardleyMap {
    parser::parse(&LineDocument::parse(text))
}

pub fn parse_fixture(name: &str) -> WardleyMap {
    parse_text(&fixture(name))
}

use adp_core::Catalogue;
use adp_diagram::{DiagramSession, DiagramTools, DiagramWorkspace};
use adp_diagram_wardley_map::identities::{WardleyIdentityEntry, kind};
use adp_diagram_wardley_map::store;
use adp_disl::DislPluginFunctions;
use adp_history::{Command, Outcome};

/// A project in a temporary folder holding the map `map.owm`, opened through the module as the shell opens it.
pub struct Project {
    _folder: tempfile::TempDir,
    pub root: PathBuf,
    pub body: PathBuf,
    pub workspace: DiagramWorkspace,
    pub session: DiagramSession,
}

impl Project {
    /// A project whose map holds `text`.
    pub fn new(text: &str) -> Project {
        Project::with_files(text, &[])
    }

    /// A project whose map holds `text`, with `files` beside it.
    pub fn with_files(text: &str, files: &[(&str, &str)]) -> Project {
        let folder = tempfile::tempdir().unwrap();
        let root = folder.path().to_path_buf();
        fs::write(root.join("map.owm"), text).unwrap();
        for (name, content) in files {
            fs::write(root.join(name), content).unwrap();
        }
        let mut tools = adp_diagram_wardley_map::register(DiagramTools::new(Catalogue::load(&vendor()), |_| {
            DislPluginFunctions::none()
        }));
        let mut workspace = DiagramWorkspace::new(&root);
        let (opened, session) = workspace.open(&mut tools, Path::new("map.owm")).unwrap();
        Project {
            _folder: folder,
            root,
            body: opened.body_path,
            workspace,
            session,
        }
    }

    /// A project whose map's body is a folder: it exists, and it cannot be read.
    pub fn with_body_folder() -> Project {
        let folder = tempfile::tempdir().unwrap();
        let root = folder.path().to_path_buf();
        fs::create_dir(root.join("map.owm")).unwrap();
        fs::write(root.join("map.adp"), "wardley/map\r\n").unwrap();
        let mut tools = adp_diagram_wardley_map::register(DiagramTools::new(Catalogue::load(&vendor()), |_| {
            DislPluginFunctions::none()
        }));
        let mut workspace = DiagramWorkspace::new(&root);
        let (opened, session) = workspace.open(&mut tools, Path::new("map.adp")).unwrap();
        Project {
            _folder: folder,
            root,
            body: opened.body_path,
            workspace,
            session,
        }
    }

    /// What the map's file holds.
    pub fn read(&self) -> String {
        fs::read_to_string(&self.body).unwrap()
    }

    /// The identities held for the map.
    pub fn identities(&self) -> Vec<WardleyIdentityEntry> {
        store::identities_of(self.workspace.documents(), &self.body).to_vec()
    }

    /// The id of the component called `name`.
    pub fn id_of(&self, name: &str) -> String {
        self.identities()
            .into_iter()
            .find(|e| e.kind == kind::COMPONENT && e.key == name)
            .unwrap_or_else(|| panic!("no component '{name}'"))
            .id
    }

    /// Runs `command` on the project's history: empty when it was done, else why it was refused.
    pub fn execute(&mut self, command: &dyn Command<adp_diagram::DiagramDocuments>) -> Result<String, String> {
        match self.workspace.execute_command(command) {
            Outcome::Done { warning, .. } => Ok(warning),
            Outcome::Refused(refusal) => Err(refusal.0),
        }
    }

    pub fn undo(&mut self) {
        assert!(self.workspace.undo().is_done(), "the undo was refused");
    }

    pub fn redo(&mut self) {
        assert!(self.workspace.redo().is_done(), "the redo was refused");
    }
}

/// Where the vendored specifications are.
pub fn vendor() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor")
}

use adp_context::{Action, Prompt, PropertyRow};
use adp_diagram::{ActionOutcome, ActionTarget};

impl Project {
    /// The actions element `id` offers, `""` being the map itself.
    pub fn actions_for(&self, id: &str) -> Vec<Action> {
        self.session
            .actions(self.workspace.documents(), &ActionTarget::Element(id.to_owned()))
            .into_iter()
            .flat_map(|group| group.actions)
            .collect()
    }

    /// The ids of [`Project::actions_for`].
    pub fn action_ids_for(&self, id: &str) -> Vec<String> {
        self.actions_for(id).into_iter().map(|a| a.id).collect()
    }

    /// Runs action `action_id` on element `id`.
    pub fn run(&mut self, id: &str, action_id: &str) -> ActionOutcome {
        let ids = adp_disl::behavior::ids::fixed(&[]);
        self.session.execute(
            &mut self.workspace,
            &ActionTarget::Element(id.to_owned()),
            action_id,
            &ids,
        )
    }

    /// The prompt action `action_id` on element `id` asks.
    pub fn ask(&mut self, id: &str, action_id: &str) -> Prompt {
        match self.run(id, action_id) {
            ActionOutcome::Prompt(prompt) => prompt,
            other => panic!("{action_id} asked nothing: {other:?}"),
        }
    }

    /// Answers the prompt of action `action_id` on element `id` with `value`: empty when it was done, else why not.
    pub fn answer(&mut self, id: &str, action_id: &str, value: &str) -> Result<(), String> {
        match self.session.answer(
            &mut self.workspace,
            &ActionTarget::Element(id.to_owned()),
            action_id,
            value,
        ) {
            ActionOutcome::Done => Ok(()),
            ActionOutcome::Refused(refusal) => Err(refusal),
            ActionOutcome::Prompt(prompt) => Err(format!("asked again: {prompt:?}")),
        }
    }

    /// The property rows of element `id`.
    pub fn rows(&self, id: &str) -> Vec<PropertyRow> {
        self.session.properties(self.workspace.documents(), &[id])
    }

    /// Sets row `property_id` of element `id` to `value`.
    pub fn set(&mut self, id: &str, property_id: &str, value: &str) -> Result<(), String> {
        self.session
            .set_property(&mut self.workspace, &[id], property_id, value)
    }

    /// The id of the first element of `kind`.
    pub fn first_of(&self, kind: &str) -> String {
        self.identities()
            .into_iter()
            .find(|e| e.kind == kind)
            .unwrap_or_else(|| panic!("no {kind}"))
            .id
    }
}
