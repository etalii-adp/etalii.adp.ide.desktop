//! The ontology reading opened in a project (etalii.adp spec 030, task T083), after the standalone host's
//! `OwlSession.Tests.cs` at `8b6ccfc`: a registered ontology opens with the VOWL elements and the overlaid layout,
//! expression nodes refuse repositioning, IRI-derived ids reposition undoably without the body changing by a byte,
//! two readings of one file share one document, and a viewport change adds and removes without moving anything.
//! The definition tests (`OwlDefinitionTests`) are the catalogue's, not this module's.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use adp_core::catalogue::Catalogue;
use adp_diagram::{DiagramDelta, DiagramElement, DiagramSession, DiagramTools, DiagramViewport, DiagramWorkspace};
use adp_diagram_rdf::owl::mapper::{EDGE_TYPE, EXPRESSION_TYPE, NODE_TYPE, TRUNCATION_TYPE};
use adp_diagram_rdf::registration_layout;
use adp_disl::DislPluginFunctions;

const NS: &str = "http://example.org/pizza#";

fn tools() -> DiagramTools {
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
    adp_diagram_rdf::register(DiagramTools::new(Catalogue::load(&vendor), |_| {
        DislPluginFunctions::none()
    }))
}

struct Project {
    _folder: tempfile::TempDir,
    root: PathBuf,
    workspace: DiagramWorkspace,
}

impl Project {
    fn new() -> Self {
        let folder = tempfile::tempdir().unwrap();
        let root = folder.path().to_path_buf();
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/owl-ontology.ttl"),
            root.join("owl-ontology.ttl"),
        )
        .unwrap();
        Project {
            workspace: DiagramWorkspace::new(&root),
            root,
            _folder: folder,
        }
    }

    fn body(&self) -> PathBuf {
        self.root.join("owl-ontology.ttl")
    }

    fn register(&self, origin: &str, extra: &str) -> PathBuf {
        let path = self.root.join("ontology.adp");
        fs::write(&path, format!("{origin}\r\nbody: owl-ontology.ttl\r\n{extra}")).unwrap();
        path
    }

    fn open(&mut self, relative: &str) -> DiagramSession {
        let (opened, session) = self.workspace.open(&mut tools(), Path::new(relative)).unwrap();
        assert!(session.module_view().is_some(), "{}", opened.origin);
        session
    }
}

fn baseline(session: &mut DiagramSession, workspace: &DiagramWorkspace) -> Vec<DiagramElement> {
    match session.baseline(workspace.documents()).unwrap().into_iter().next() {
        Some(DiagramDelta::Add(elements)) => elements,
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_registered_ontology_opens_with_the_vowl_elements() {
    let mut project = Project::new();
    project.register("w3c/owl", "");
    let mut session = project.open("ontology.adp");
    let elements = baseline(&mut session, &project.workspace);
    assert!(
        elements
            .iter()
            .any(|e| e.id == format!("res:{NS}Pizza") && e.element_type == NODE_TYPE)
    );
    assert!(elements.iter().any(|e| e.element_type == EDGE_TYPE));
    assert!(elements.iter().any(|e| e.element_type == EXPRESSION_TYPE));
    assert!(!elements.iter().any(|e| e.element_type == TRUNCATION_TYPE));
}

#[test]
fn a_stored_position_wins_for_a_class_and_never_for_an_expression() {
    let mut project = Project::new();
    let expression_id = format!("expr:{NS}Vegetarian|http://www.w3.org/2000/01/rdf-schema#subClassOf|1");
    project.register(
        "w3c/owl",
        &format!("layout:\r\n  res:{NS}Pizza: 555 666\r\n  {expression_id}: 111 222\r\n"),
    );
    let mut session = project.open("ontology.adp");
    let elements = baseline(&mut session, &project.workspace);
    let pizza = elements.iter().find(|e| e.id == format!("res:{NS}Pizza")).unwrap();
    assert_eq!((pizza.x, pizza.y), (555.0, 666.0));
    let expression = elements.iter().find(|e| e.id == expression_id).unwrap();
    assert_ne!((expression.x, expression.y), (111.0, 222.0));
}

#[test]
fn a_reposition_lands_in_the_registration_is_one_undo_away_and_the_body_never_changes() {
    let mut project = Project::new();
    let body_bytes = fs::read(project.body()).unwrap();
    let adp = project.register("w3c/owl", "");
    let adp_before = fs::read(&adp).unwrap();
    let session = project.open("ontology.adp");
    session
        .move_element_to(&mut project.workspace, &format!("res:{NS}Pizza"), 120.0, 240.0)
        .unwrap();
    let text = String::from_utf8(fs::read(&adp).unwrap()).unwrap();
    assert_eq!(
        registration_layout::read(&text)[&format!("res:{NS}Pizza")],
        (120.0, 240.0)
    );
    assert_eq!(fs::read(project.body()).unwrap(), body_bytes);
    project.workspace.undo();
    assert_eq!(fs::read(&adp).unwrap(), adp_before);
}

#[test]
fn the_refusals_answer_with_their_sentences() {
    let mut project = Project::new();

    // A bare file has nowhere to store a position. A bare body opens as the anchor reading, so the ontology view of it
    // is made directly, as the standalone host's factory opened one without a registration, over the body the anchor
    // loaded.
    {
        let _bare = project.open("owl-ontology.ttl");
        let view = adp_diagram_rdf::view_of(adp_diagram_rdf::owl::ORIGIN, &project.root, &project.body(), None);
        let refusal = view
            .move_elements(&mut project.workspace, &[(format!("res:{NS}Pizza"), 1.0, 2.0)])
            .unwrap_err();
        assert!(refusal.contains("Register the file"), "{refusal}");
    }

    project.register("w3c/owl", "");
    let session = project.open("ontology.adp");
    let workspace = &mut project.workspace;

    let expression = session
        .move_element_to(
            workspace,
            &format!("expr:{NS}Vegetarian|http://www.w3.org/2000/01/rdf-schema#subClassOf|1"),
            1.0,
            2.0,
        )
        .unwrap_err();
    assert!(expression.contains("does not survive an edit"), "{expression}");

    let anchor = session
        .move_element_to(workspace, &format!("thing:{NS}toppingOf|domain"), 1.0, 2.0)
        .unwrap_err();
    assert!(anchor.contains("cannot be arranged"), "{anchor}");

    let edge = session
        .move_element_to(workspace, "edge:whatever", 1.0, 2.0)
        .unwrap_err();
    assert!(edge.contains("not something this diagram can move"), "{edge}");
}

#[test]
fn two_readings_of_one_file_share_one_document() {
    let mut project = Project::new();
    project.register("w3c/owl", "");
    let mut owl = project.open("ontology.adp");
    let mut rdf = project.open("owl-ontology.ttl");
    let rdf_before = baseline(&mut rdf, &project.workspace).len();
    let owl_before = baseline(&mut owl, &project.workspace).len();

    let mut text = fs::read_to_string(project.body()).unwrap();
    text.push_str(":Calzone a owl:Class .\r\n");
    fs::write(project.body(), text).unwrap();
    project.workspace.reconcile(&project.body());

    let body = project.body();
    owl.on_document_changed(project.workspace.documents(), &body);
    rdf.on_document_changed(project.workspace.documents(), &body);
    let owl_after = owl.render(project.workspace.documents()).unwrap();
    let rdf_after = rdf.render(project.workspace.documents()).unwrap();
    assert!(owl_after.iter().any(|e| e.id == format!("res:{NS}Calzone")));
    assert!(rdf_after.iter().any(|e| e.id == format!("res:{NS}Calzone")));
    assert!(owl_after.len() > owl_before);
    assert!(rdf_after.len() > rdf_before);
}

#[test]
fn a_viewport_change_adds_what_came_into_view_and_removes_what_left() {
    let mut project = Project::new();
    project.register("w3c/owl", "");
    let mut session = project.open("ontology.adp");
    let mut drawn: Vec<DiagramElement> = baseline(&mut session, &project.workspace)
        .into_iter()
        .filter(|e| e.kind == adp_diagram::ElementKind::Node)
        .collect();
    drawn.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));
    let anchor = drawn[0].clone();

    let narrowed = session
        .update_view(
            project.workspace.documents(),
            DiagramViewport::new(anchor.x, anchor.y, anchor.x + 1.0, anchor.y + 1.0),
        )
        .unwrap();
    let widened = session
        .update_view(project.workspace.documents(), DiagramViewport::UNBOUNDED)
        .unwrap();

    let removed: HashSet<String> = narrowed
        .iter()
        .filter_map(|d| match d {
            DiagramDelta::Remove(ids) => Some(ids.clone()),
            _ => None,
        })
        .flatten()
        .collect();
    assert!(!removed.is_empty());
    assert!(!removed.contains(&anchor.id));
    let restored: HashSet<String> = widened
        .iter()
        .filter_map(|d| match d {
            DiagramDelta::Add(elements) => Some(elements.iter().map(|e| e.id.clone()).collect::<Vec<_>>()),
            _ => None,
        })
        .flatten()
        .collect();
    assert!(removed.is_subset(&restored));
}

#[test]
fn an_unchanged_viewport_says_nothing_twice() {
    let mut project = Project::new();
    project.register("w3c/owl", "");
    let mut session = project.open("ontology.adp");
    let _ = baseline(&mut session, &project.workspace);
    session
        .update_view(project.workspace.documents(), DiagramViewport::UNBOUNDED)
        .unwrap();
    let again = session
        .update_view(project.workspace.documents(), DiagramViewport::UNBOUNDED)
        .unwrap();
    assert!(again.iter().all(|d| match d {
        DiagramDelta::Add(e) => e.is_empty(),
        DiagramDelta::Remove(ids) => ids.is_empty(),
        _ => false,
    }));
}

#[test]
fn panning_does_not_move_the_elements_it_brings_into_view() {
    let mut project = Project::new();
    project.register("w3c/owl", "");
    let mut session = project.open("ontology.adp");
    let at_open: HashMap<String, (f64, f64)> = baseline(&mut session, &project.workspace)
        .into_iter()
        .map(|e| (e.id, (e.x, e.y)))
        .collect();
    session
        .update_view(project.workspace.documents(), DiagramViewport::new(0.0, 0.0, 1.0, 1.0))
        .unwrap();
    let readmitted: Vec<DiagramElement> = session
        .update_view(project.workspace.documents(), DiagramViewport::UNBOUNDED)
        .unwrap()
        .into_iter()
        .filter_map(|d| match d {
            DiagramDelta::Add(elements) => Some(elements),
            _ => None,
        })
        .flatten()
        .collect();
    assert!(!readmitted.is_empty());
    for element in readmitted {
        assert_eq!(at_open[&element.id], (element.x, element.y));
    }
}
