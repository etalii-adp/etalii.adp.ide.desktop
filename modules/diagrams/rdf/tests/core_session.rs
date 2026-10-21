//! The data-graph reading opened in a project: a registered file opens with cards and edges, a stored position wins,
//! a reposition lands in the registration one undo away without the body changing by a byte, the refusals answer with
//! their sentences, and the viewport adds and removes without moving anything or cutting an edge loose. Ported from
//! the standalone host's `RdfSession.Tests.cs` and, for the routing of a bare `.ttl` and `.nt`, `Diagram.Tests.cs` at
//! `8b6ccfc`. The definition's own flags (`DeclaresExtension`, `RoutesBareBody`) are the catalogue's.

mod common;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use adp_core::catalogue::Catalogue;
use adp_diagram::{DiagramDelta, DiagramElement, DiagramSession, DiagramTools, DiagramViewport, DiagramWorkspace};
use adp_diagram_rdf::graph::{EDGE_TYPE, RESOURCE_TYPE, TRUNCATION_TYPE};
use adp_diagram_rdf::projection::DEFAULT_BUDGET;
use adp_diagram_rdf::registration_layout;
use adp_diagram_rdf::selection::TRUNCATION_ID;
use adp_disl::DislPluginFunctions;
use adp_history::Outcome;

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
        Project {
            workspace: DiagramWorkspace::new(&root),
            root,
            _folder: folder,
        }
    }

    fn copy_fixture(&self, name: &str) -> PathBuf {
        let destination = self.root.join(name);
        fs::copy(common::fixture_path(name), &destination).unwrap();
        destination
    }

    fn register(&self, body: &str, extra: &str) -> PathBuf {
        let path = self.root.join("graph.adp");
        fs::write(&path, format!("w3c/rdf\r\nbody: {body}\r\n{extra}")).unwrap();
        path
    }

    fn open(&mut self, relative: &str) -> DiagramSession {
        let (opened, session) = self.workspace.open(&mut tools(), Path::new(relative)).unwrap();
        assert_eq!(opened.origin, "w3c/rdf");
        assert!(session.module_view().is_some());
        session
    }

    fn baseline(&self, session: &mut DiagramSession) -> Vec<DiagramElement> {
        match session.baseline(self.workspace.documents()).unwrap().into_iter().next() {
            Some(DiagramDelta::Add(elements)) => elements,
            other => panic!("{other:?}"),
        }
    }

    fn update(&self, session: &mut DiagramSession, viewport: DiagramViewport) -> Vec<DiagramDelta> {
        session.update_view(self.workspace.documents(), viewport).unwrap()
    }
}

fn viewport(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> DiagramViewport {
    DiagramViewport {
        min_x,
        min_y,
        max_x,
        max_y,
    }
}

fn removed(deltas: &[DiagramDelta]) -> HashSet<String> {
    deltas
        .iter()
        .flat_map(|d| match d {
            DiagramDelta::Remove(ids) => ids.clone(),
            _ => Vec::new(),
        })
        .collect()
}

fn added(deltas: &[DiagramDelta]) -> Vec<DiagramElement> {
    deltas
        .iter()
        .flat_map(|d| match d {
            DiagramDelta::Add(elements) => elements.clone(),
            _ => Vec::new(),
        })
        .collect()
}

/// The resource cards of `elements`, the first by its place (top, then left) first.
fn resources(elements: &[DiagramElement]) -> Vec<DiagramElement> {
    let mut cards: Vec<DiagramElement> = elements
        .iter()
        .filter(|e| e.element_type == RESOURCE_TYPE)
        .cloned()
        .collect();
    cards.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));
    cards
}

fn large_body(project: &Project) {
    let mut text = String::from("@prefix ex: <http://example.org/> .\n");
    for i in 0..DEFAULT_BUDGET + 200 {
        text.push_str(&format!("ex:r{i:05} ex:index \"{i}\" .\n"));
    }
    fs::write(project.root.join("large.ttl"), text).unwrap();
}

#[test]
fn a_registered_turtle_file_opens_with_cards_and_edges() {
    let mut project = Project::new();
    project.copy_fixture("constructs.ttl");
    project.register("constructs.ttl", "");
    let mut session = project.open("graph.adp");
    let elements = project.baseline(&mut session);

    assert!(
        elements
            .iter()
            .any(|e| e.id == "res:http://example.org/alice" && e.element_type == RESOURCE_TYPE)
    );
    assert!(elements.iter().any(|e| e.element_type == EDGE_TYPE));
    assert!(!elements.iter().any(|e| e.element_type == TRUNCATION_TYPE));
}

#[test]
fn a_stored_position_wins_over_the_computed_one() {
    let mut project = Project::new();
    project.copy_fixture("crlf-line-endings.ttl");
    project.register(
        "crlf-line-endings.ttl",
        "layout:\r\n  res:http://example.org/a: 555 666\r\n",
    );
    let mut session = project.open("graph.adp");
    let elements = project.baseline(&mut session);

    let moved = elements.iter().find(|e| e.id == "res:http://example.org/a").unwrap();
    assert_eq!((moved.x, moved.y), (555.0, 666.0));
}

#[test]
fn a_reposition_lands_in_the_registration_is_one_undo_away_and_the_body_never_changes() {
    let mut project = Project::new();
    let body = project.copy_fixture("crlf-line-endings.ttl");
    let body_bytes = fs::read(&body).unwrap();
    let adp = project.register("crlf-line-endings.ttl", "");
    let adp_before = fs::read_to_string(&adp).unwrap();
    let session = project.open("graph.adp");

    session
        .move_element_to(&mut project.workspace, "res:http://example.org/a", 120.0, 240.0)
        .unwrap();

    let stored = registration_layout::read(&fs::read_to_string(&adp).unwrap());
    assert_eq!(stored["res:http://example.org/a"], (120.0, 240.0));
    assert_eq!(fs::read(&body).unwrap(), body_bytes);
    assert!(matches!(project.workspace.undo(), Outcome::Done { .. }));
    assert_eq!(fs::read_to_string(&adp).unwrap(), adp_before);
}

#[test]
fn the_refusals_answer_with_their_sentences() {
    let mut project = Project::new();
    project.copy_fixture("constructs.ttl");

    let bare = project.open("constructs.ttl");
    let refusal = bare
        .move_element_to(&mut project.workspace, "res:http://example.org/alice", 1.0, 2.0)
        .unwrap_err();
    assert!(refusal.contains("Register the file"), "{refusal}");

    project.register("constructs.ttl", "");
    let session = project.open("graph.adp");
    let blank = session
        .move_element_to(&mut project.workspace, "blank:0", 1.0, 2.0)
        .unwrap_err();
    assert!(blank.contains("blank node"), "{blank}");
    let edge = session
        .move_element_to(&mut project.workspace, "edge:whatever", 1.0, 2.0)
        .unwrap_err();
    assert!(edge.contains("not something this diagram can move"), "{edge}");
}

#[test]
fn a_document_edit_while_narrowed_never_resends_what_the_viewport_culled() {
    let mut project = Project::new();
    let body = project.copy_fixture("constructs.ttl");
    project.register("constructs.ttl", "");
    let mut session = project.open("graph.adp");
    let all = resources(&project.baseline(&mut session));
    let first = &all[0];

    let narrowed = project.update(&mut session, viewport(first.x, first.y, first.x + 1.0, first.y + 1.0));
    let culled = removed(&narrowed);
    assert!(!culled.is_empty());

    let mut text = fs::read_to_string(&body).unwrap();
    text.push_str("\n# an edit while the reader is zoomed in\n");
    fs::write(&body, text).unwrap();
    project.workspace.reconcile(&body);
    let change = session.on_document_changed(project.workspace.documents(), &body);

    match change {
        adp_diagram::DocumentChange::Changed(deltas) => {
            assert!(added(&deltas).iter().all(|e| !culled.contains(&e.id)));
        }
        adp_diagram::DocumentChange::NothingVisible => {}
        other => panic!("{other:?}"),
    }
}

#[test]
fn no_edge_is_ever_delivered_with_one_end_missing() {
    let mut project = Project::new();
    project.copy_fixture("constructs.ttl");
    project.register("constructs.ttl", "");
    let mut session = project.open("graph.adp");
    let drawn = project.baseline(&mut session);
    let anchor = resources(&drawn)[0].clone();
    let mut held: HashSet<String> = drawn.iter().map(|e| e.id.clone()).collect();

    for view in [
        viewport(anchor.x, anchor.y, anchor.x + 1.0, anchor.y + 1.0),
        viewport(-10_000.0, -10_000.0, -9_000.0, -9_000.0),
        DiagramViewport::UNBOUNDED,
    ] {
        for delta in project.update(&mut session, view) {
            match delta {
                DiagramDelta::Add(elements) => held.extend(elements.into_iter().map(|e| e.id)),
                DiagramDelta::Remove(ids) => {
                    for id in ids {
                        held.remove(&id);
                    }
                }
                _ => {}
            }
        }
        for edge in held.iter().filter(|id| id.starts_with("edge:")) {
            let parts: Vec<&str> = edge["edge:".len()..].split('|').collect();
            assert!(held.contains(parts[0]), "{edge}");
            assert!(held.contains(parts[2]), "{edge}");
        }
    }
}

#[test]
fn panning_reaches_resources_the_budget_discarded() {
    let mut project = Project::new();
    large_body(&project);
    project.register("large.ttl", "");
    let mut session = project.open("graph.adp");
    let at_open = resources(&project.baseline(&mut session));

    assert_eq!(at_open.len(), DEFAULT_BUDGET);
    let drawn_at_open: HashSet<String> = at_open.iter().map(|e| e.id.clone()).collect();
    let lowest = at_open.iter().map(|e| e.y).fold(f64::NEG_INFINITY, f64::max);

    let deltas = project.update(&mut session, viewport(-1000.0, lowest, 5000.0, lowest + 100_000.0));
    let arrived: Vec<String> = added(&deltas)
        .into_iter()
        .filter(|e| e.element_type == RESOURCE_TYPE)
        .map(|e| e.id)
        .collect();

    assert!(!arrived.is_empty());
    assert!(arrived.iter().any(|id| !drawn_at_open.contains(id)));
}

#[test]
fn a_document_over_the_budget_keeps_its_banner_and_its_refusal() {
    let mut project = Project::new();
    large_body(&project);
    project.register("large.ttl", "");
    let mut session = project.open("graph.adp");

    let at_open = project.baseline(&mut session);
    let narrowed = project.update(&mut session, viewport(0.0, 0.0, 300.0, 200.0));

    assert!(at_open.iter().any(|e| e.element_type == TRUNCATION_TYPE));
    assert!(!removed(&narrowed).contains(TRUNCATION_ID));
}

#[test]
fn a_viewport_change_adds_what_came_into_view_and_removes_what_left() {
    let mut project = Project::new();
    project.copy_fixture("constructs.ttl");
    project.register("constructs.ttl", "");
    let mut session = project.open("graph.adp");
    let all = resources(&project.baseline(&mut session));
    assert!(all.len() >= 2);
    let first = &all[0];

    let narrowed = project.update(&mut session, viewport(first.x, first.y, first.x + 1.0, first.y + 1.0));
    let widened = project.update(&mut session, DiagramViewport::UNBOUNDED);

    let gone = removed(&narrowed);
    assert!(!gone.is_empty());
    assert!(!gone.contains(&first.id));
    let restored: HashSet<String> = added(&widened).into_iter().map(|e| e.id).collect();
    assert!(gone.is_subset(&restored));
}

#[test]
fn a_viewport_change_emits_remove_before_add() {
    let mut project = Project::new();
    project.copy_fixture("constructs.ttl");
    project.register("constructs.ttl", "");
    let mut session = project.open("graph.adp");
    let all = resources(&project.baseline(&mut session));
    let (first, last) = (all[0].clone(), all[all.len() - 1].clone());
    project.update(&mut session, viewport(first.x, first.y, first.x + 1.0, first.y + 1.0));

    let deltas = project.update(&mut session, viewport(last.x, last.y, last.x + 1.0, last.y + 1.0));

    assert_eq!(deltas.len(), 2, "{deltas:?}");
    assert!(matches!(&deltas[0], DiagramDelta::Remove(ids) if ids.contains(&first.id)));
    assert!(matches!(&deltas[1], DiagramDelta::Add(elements) if elements.iter().any(|e| e.id == last.id)));
}

#[test]
fn an_unchanged_viewport_says_nothing_twice() {
    let mut project = Project::new();
    project.copy_fixture("constructs.ttl");
    project.register("constructs.ttl", "");
    let mut session = project.open("graph.adp");
    project.baseline(&mut session);

    project.update(&mut session, DiagramViewport::UNBOUNDED);
    let again = project.update(&mut session, DiagramViewport::UNBOUNDED);

    assert!(again.is_empty(), "{again:?}");
}

#[test]
fn panning_does_not_move_the_nodes_it_brings_into_view() {
    let mut project = Project::new();
    project.copy_fixture("constructs.ttl");
    project.register("constructs.ttl", "");
    let mut session = project.open("graph.adp");
    let at_open: HashMap<String, (f64, f64)> = project
        .baseline(&mut session)
        .into_iter()
        .map(|e| (e.id, (e.x, e.y)))
        .collect();

    project.update(&mut session, viewport(0.0, 0.0, 1.0, 1.0));
    let readmitted = added(&project.update(&mut session, DiagramViewport::UNBOUNDED));

    assert!(!readmitted.is_empty());
    for element in readmitted {
        assert_eq!(at_open[&element.id], (element.x, element.y), "{}", element.id);
    }
}

#[test]
fn a_document_within_the_budget_shows_no_banner_whatever_the_viewport() {
    let mut project = Project::new();
    project.copy_fixture("constructs.ttl");
    project.register("constructs.ttl", "");
    let mut session = project.open("graph.adp");
    project.baseline(&mut session);

    project.update(&mut session, viewport(0.0, 0.0, 1.0, 1.0));
    let elements = added(&project.update(&mut session, DiagramViewport::UNBOUNDED));

    assert!(!elements.iter().any(|e| e.element_type == TRUNCATION_TYPE));
}

#[test]
fn a_bare_file_of_either_serialization_routes_on_sight() {
    for name in ["data.ttl", "data.nt"] {
        let mut project = Project::new();
        fs::write(project.root.join(name), "").unwrap();
        project.open(name);
    }
}
