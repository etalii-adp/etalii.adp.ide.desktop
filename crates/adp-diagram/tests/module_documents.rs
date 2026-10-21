//! A tool type whose module reads, draws and edits its documents itself (etalii.adp spec 030, FR-018 and FR-031;
//! task T061): the hook the C4 module opens its Structurizr DSL workspaces through, as the standalone host's C4 module
//! read them with its own code rather than through a specification. The workspace opens the registration through the
//! module, keeps what the module reads in the project's documents, and the session hands the module's view every
//! question and gesture; the module's edits are commands on the project's history, so undo puts the bytes back.
//!
//! The probe stands in for any such module: its body is one line of text, its one element `text` has one property,
//! `probe.value`, which rewrites the line.

use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_context::{ActionGroup, PropertyEditor, PropertyRow};
use adp_core::Catalogue;
use adp_diagram::session::NOT_OFFERED;
use adp_diagram::workspace::OpenedDiagram;
use adp_diagram::{
    ActionOutcome, ActionTarget, DiagramDocuments, DiagramElement, DiagramModule, DiagramProblem, DiagramTools,
    DiagramViewport, DiagramWorkspace, ModuleView, ToolboxItem,
};
use adp_disl::DislPluginFunctions;
use adp_disl::behavior::ids;
use adp_history::{Applied, Command, Outcome, Refusal};

const ORIGIN: &str = "probe/text";
const KEY: &str = "probe";

/// What the probe keeps of its documents: each body's text, by path.
#[derive(Default)]
struct Texts(Vec<(PathBuf, String)>);

fn text_of(documents: &DiagramDocuments, body: &Path) -> Option<String> {
    let texts = documents.module_state_ref::<Texts>(KEY)?;
    texts.0.iter().find(|(p, _)| p == body).map(|(_, t)| t.clone())
}

/// Writes `text` as the body and keeps it; its inverse writes the text it replaced.
struct SetText {
    body: PathBuf,
    text: String,
}

impl Command<DiagramDocuments> for SetText {
    fn label(&self) -> String {
        "Change".to_owned()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        let was = text_of(documents, &self.body).ok_or_else(|| Refusal::new("not open"))?;
        fs::write(&self.body, &self.text).map_err(|e| Refusal::new(e.to_string()))?;
        let texts = documents.module_state::<Texts>(KEY);
        for entry in &mut texts.0 {
            if entry.0 == self.body {
                entry.1 = self.text.clone();
            }
        }
        documents.mark_changed(&self.body);
        Ok(Applied::undoable(SetText {
            body: self.body.clone(),
            text: was,
        }))
    }
}

struct ProbeView {
    body: PathBuf,
}

impl ModuleView for ProbeView {
    fn render(&self, documents: &DiagramDocuments, _viewport: DiagramViewport) -> Result<Vec<DiagramElement>, String> {
        let text = text_of(documents, &self.body).ok_or("not open")?;
        let mut element = DiagramElement::node("text", "Text");
        element.attributes.push(("value".to_owned(), text));
        Ok(vec![element])
    }

    fn describe(&self, _documents: &DiagramDocuments, element_id: &str) -> Option<(Vec<String>, String)> {
        (element_id == "text").then(|| (vec!["Text".to_owned()], "The text".to_owned()))
    }

    fn toolbox(&self) -> Vec<ToolboxItem> {
        Vec::new()
    }

    fn problems(&self, _documents: &DiagramDocuments) -> Vec<DiagramProblem> {
        Vec::new()
    }

    fn actions(&self, _documents: &DiagramDocuments, _target: &ActionTarget) -> Vec<ActionGroup> {
        Vec::new()
    }

    fn execute(&self, _workspace: &mut DiagramWorkspace, _target: &ActionTarget, action_id: &str) -> ActionOutcome {
        ActionOutcome::Refused(format!("'{action_id}' does not apply to this selection."))
    }

    fn answer(
        &self,
        _workspace: &mut DiagramWorkspace,
        _target: &ActionTarget,
        action_id: &str,
        _value: &str,
    ) -> ActionOutcome {
        ActionOutcome::Refused(format!("'{action_id}' does not apply to this selection."))
    }

    fn properties(&self, documents: &DiagramDocuments, element_id: &str) -> Vec<PropertyRow> {
        match (element_id, text_of(documents, &self.body)) {
            ("text", Some(text)) => vec![PropertyRow::new("probe.value", "Value", PropertyEditor::Line, text)],
            _ => Vec::new(),
        }
    }

    fn set_property(
        &self,
        workspace: &mut DiagramWorkspace,
        _element_ids: &[&str],
        _property_id: &str,
        value: &str,
    ) -> Result<(), String> {
        let command = SetText {
            body: self.body.clone(),
            text: value.to_owned(),
        };
        match workspace.execute_command(&command) {
            Outcome::Done { .. } => Ok(()),
            Outcome::Refused(refusal) => Err(refusal.0),
        }
    }

    fn move_elements(&self, _workspace: &mut DiagramWorkspace, _moves: &[(String, f64, f64)]) -> Result<(), String> {
        Err("The probe has no positions.".to_owned())
    }

    fn move_element(
        &self,
        _workspace: &mut DiagramWorkspace,
        _element_id: &str,
        _new_parent_id: Option<&str>,
        _index: i64,
    ) -> Result<(), String> {
        Err("The probe has no parents.".to_owned())
    }
}

struct ProbeModule;

impl DiagramModule for ProbeModule {
    fn opens_documents(&self) -> bool {
        true
    }

    fn extensions(&self) -> Vec<String> {
        vec![".probe".to_owned()]
    }

    fn open(
        &self,
        _root: &Path,
        path: &Path,
        origin: &str,
        documents: &mut DiagramDocuments,
    ) -> Result<(OpenedDiagram, Rc<dyn ModuleView>), String> {
        let (body, registration) = if path.extension().is_some_and(|e| e == "adp") {
            (path.with_extension("probe"), Some(path.to_path_buf()))
        } else {
            (path.to_path_buf(), None)
        };
        let text = fs::read_to_string(&body).map_err(|e| e.to_string())?;
        documents.module_state::<Texts>(KEY).0.push((body.clone(), text));
        Ok((
            OpenedDiagram {
                origin: origin.to_owned(),
                body_path: body.clone(),
                registration_path: registration,
            },
            Rc::new(ProbeView { body }),
        ))
    }
}

fn tools() -> DiagramTools {
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor");
    DiagramTools::new(Catalogue::load(&vendor), |_| DislPluginFunctions::none())
        .with_module(ORIGIN, Rc::new(ProbeModule))
}

#[test]
fn a_module_that_opens_its_documents_is_asked_everything_and_its_edits_are_undone_through_the_history() {
    let folder = tempfile::tempdir().unwrap();
    let root = folder.path().to_path_buf();
    fs::write(root.join("note.probe"), "first").unwrap();
    fs::write(root.join("note.adp"), format!("{ORIGIN}\n")).unwrap();
    let mut tools = tools();
    assert!(tools.opened_by_module(ORIGIN));
    assert!(tools.for_file("other.probe").contains(&ORIGIN.to_owned()));

    let mut workspace = DiagramWorkspace::new(&root);
    let (opened, mut session) = workspace.open(&mut tools, Path::new("note.adp")).unwrap();
    assert_eq!(opened.origin, ORIGIN);
    assert_eq!(opened.body_path, root.join("note.probe"));
    assert!(session.module_view().is_some());

    let baseline = session.baseline(workspace.documents()).unwrap();
    assert_eq!(baseline.len(), 1);
    let selected = session.select(workspace.documents(), "text").unwrap();
    assert_eq!(selected.text, "The text");
    assert!(session.select(workspace.documents(), "nothing").is_none());

    let rows = session.properties(workspace.documents(), &["text"]);
    assert_eq!(rows[0].value, "first");
    session
        .set_property(&mut workspace, &["text"], "probe.value", "second")
        .unwrap();
    assert_eq!(fs::read_to_string(root.join("note.probe")).unwrap(), "second");
    assert_eq!(workspace.take_changed(), vec![root.join("note.probe")]);
    assert_eq!(
        session.render(workspace.documents()).unwrap()[0].attribute("value"),
        "second"
    );

    assert!(workspace.undo().is_done());
    assert_eq!(fs::read_to_string(root.join("note.probe")).unwrap(), "first");
    assert!(workspace.redo().is_done());
    assert_eq!(fs::read_to_string(root.join("note.probe")).unwrap(), "second");

    assert_eq!(
        session.move_element(&mut workspace, "text", None, -1),
        Err("The probe has no parents.".to_owned())
    );
    assert_eq!(
        session.drop_tool(&mut workspace, "anything", 0.0, 0.0, &ids::fixed(&[])),
        Err(NOT_OFFERED.to_owned())
    );
}

#[test]
fn a_body_the_module_claims_opens_through_it_without_a_registration() {
    let folder = tempfile::tempdir().unwrap();
    let root = folder.path().to_path_buf();
    fs::write(root.join("bare.probe"), "bare").unwrap();
    let mut tools = tools();
    let mut workspace = DiagramWorkspace::new(&root);
    let (opened, session) = workspace.open(&mut tools, Path::new("bare.probe")).unwrap();
    assert_eq!(opened.registration_path, None);
    assert_eq!(session.properties(workspace.documents(), &["text"])[0].value, "bare");
}
