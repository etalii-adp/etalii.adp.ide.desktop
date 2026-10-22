//! The shapes reading over the family's one store: what its view draws, the positions it stores in the registration
//! and refuses to, one file served to two readings on one history, the starter document, and the viewport. Ported from
//! the standalone host's `ShaclSession.Tests.cs` at `8b6ccfc`, through `adp-diagram`'s workspace and the family's
//! module. The definition's routing (`SharedExtension`, `DeclaresExtension`) is the workspace's and Add's, not this
//! module's; its marker test is [`shacl::suggests_body`].

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use adp_core::catalogue::Catalogue;
use adp_diagram::{DiagramDelta, DiagramElement, DiagramSession, DiagramTools, DiagramViewport, DiagramWorkspace};
use adp_diagram_rdf::commands::{RdfCommand, RdfEdit};
use adp_diagram_rdf::lines::LineDocument;
use adp_diagram_rdf::shacl::projection::{self, DEFAULT_BUDGET};
use adp_diagram_rdf::shacl::{self, SHAPE_TYPE, refusals};
use adp_diagram_rdf::{empty_shacl_document, graph, parser, store, view_of};
use adp_disl::DislPluginFunctions;
use adp_history::Outcome;

fn tools() -> DiagramTools {
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
    adp_diagram_rdf::register(DiagramTools::new(Catalogue::load(&vendor), |_| {
        DislPluginFunctions::none()
    }))
}

/// A C# raw string literal from a file checked out with CRLF.
fn raw(lines: &[&str]) -> String {
    lines.join("\r\n")
}

fn shapes() -> String {
    raw(&[
        "@prefix sh: <http://www.w3.org/ns/shacl#> .",
        "@prefix ex: <http://example.org/> .",
        "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .",
        "",
        "ex:PersonShape a sh:NodeShape ;",
        "    sh:targetClass ex:Person ;",
        "    sh:property [ sh:path ex:name ; sh:datatype xsd:string ; sh:minCount 1 ] .",
    ])
}

fn two_shapes() -> String {
    raw(&[
        "@prefix sh: <http://www.w3.org/ns/shacl#> .",
        "@prefix ex: <http://example.org/> .",
        "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .",
        "",
        "ex:PersonShape a sh:NodeShape ;",
        "    sh:targetClass ex:Person ;",
        "    sh:property [ sh:path ex:name ; sh:datatype xsd:string ; sh:minCount 1 ] .",
        "",
        "ex:OrganisationShape a sh:NodeShape ;",
        "    sh:targetClass ex:Organisation ;",
        "    sh:property [ sh:path ex:label ; sh:datatype xsd:string ; sh:minCount 1 ] .",
        "",
        "ex:PlaceShape a sh:NodeShape ;",
        "    sh:targetClass ex:Place ;",
        "    sh:property [ sh:path ex:located ; sh:datatype xsd:string ] .",
    ])
}

/// A project holding one body, opened under a registration of `origin`.
struct Project {
    folder: tempfile::TempDir,
    workspace: DiagramWorkspace,
}

impl Project {
    fn new() -> Self {
        let folder = tempfile::tempdir().unwrap();
        let workspace = DiagramWorkspace::new(folder.path());
        Project { folder, workspace }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.folder.path().join(name)
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.path(name);
        fs::write(&path, text).unwrap();
        path
    }

    fn open(&mut self, registration: &str) -> DiagramSession {
        self.workspace.open(&mut tools(), Path::new(registration)).unwrap().1
    }
}

fn added(deltas: &[DiagramDelta]) -> Vec<DiagramElement> {
    deltas
        .iter()
        .filter_map(|d| match d {
            DiagramDelta::Add(elements) => Some(elements.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

fn removed(deltas: &[DiagramDelta]) -> HashSet<String> {
    deltas
        .iter()
        .filter_map(|d| match d {
            DiagramDelta::Remove(ids) => Some(ids.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

#[test]
fn the_definition_is_suggested_on_marker_triples_and_not_on_a_plain_graph() {
    assert!(shacl::suggests_body("ex:S a sh:NodeShape ."));
    assert!(shacl::suggests_body("ex:S sh:property [ sh:path ex:p ] ."));
    assert!(shacl::suggests_body("<x> a <http://www.w3.org/ns/shacl#NodeShape> ."));
    assert!(!shacl::suggests_body("ex:alice a ex:Person ; ex:name \"Alice\" ."));
    assert!(!shacl::suggests_body("ex:o a owl:Ontology ."));
}

#[test]
fn the_session_renders_the_shapes_with_chips_and_rows_inside_the_card() {
    let mut project = Project::new();
    project.write("shapes.ttl", &shapes());
    project.write("shapes.adp", "w3c/shacl\r\nbody: shapes.ttl\r\n");
    let mut session = project.open("shapes.adp");
    let elements = added(&session.baseline(project.workspace.documents()).unwrap());
    assert_eq!(elements.len(), 1);
    assert_eq!(elements[0].element_type, SHAPE_TYPE);
    assert_eq!(elements[0].id, "res:http://example.org/PersonShape");
}

#[test]
fn the_session_refuses_to_store_a_position_without_a_registration() {
    let mut project = Project::new();
    let body = project.write("shapes.ttl", &shapes());
    project.write("shapes.adp", "w3c/shacl\r\nbody: shapes.ttl\r\n");
    project.open("shapes.adp");
    // A second view of the same body, opened without one.
    let bare = view_of(shacl::ORIGIN, project.folder.path(), &body, None);
    let refusal = bare
        .move_elements(
            &mut project.workspace,
            &[("res:http://example.org/PersonShape".to_owned(), 10.0, 20.0)],
        )
        .unwrap_err();
    assert!(refusal.contains("nowhere to store a position"), "{refusal}");
}

#[test]
fn the_session_refuses_to_move_an_anonymous_shape_with_the_boundary_sentence() {
    let mut project = Project::new();
    project.write(
        "anon.ttl",
        &raw(&[
            "@prefix sh: <http://www.w3.org/ns/shacl#> .",
            "@prefix ex: <http://example.org/> .",
            "",
            "ex:S a sh:NodeShape ; sh:node [ sh:closed true ] .",
        ]),
    );
    project.write("anon.adp", "w3c/shacl\r\nbody: anon.ttl\r\n");
    let mut session = project.open("anon.adp");
    let elements = added(&session.baseline(project.workspace.documents()).unwrap());
    let blank = elements.iter().find(|e| e.id.starts_with("blank:")).unwrap();
    let refusal = session
        .move_elements(&mut project.workspace, &[(blank.id.clone(), 10.0, 20.0)])
        .unwrap_err();
    assert_eq!(refusal, refusals::BLANK_ROOTED);
}

#[test]
fn the_session_stores_an_iri_cards_position_leaving_the_shapes_file_untouched() {
    let mut project = Project::new();
    let body = project.write("shapes.ttl", &shapes());
    let registration = project.write("shapes.adp", "w3c/shacl\r\nbody: shapes.ttl\r\n");
    let before = fs::read_to_string(&body).unwrap();
    let mut session = project.open("shapes.adp");
    session.baseline(project.workspace.documents()).unwrap();
    session
        .move_elements(
            &mut project.workspace,
            &[("res:http://example.org/PersonShape".to_owned(), 120.0, 240.0)],
        )
        .unwrap();
    assert!(fs::read_to_string(&registration).unwrap().contains("layout:"));
    assert_eq!(fs::read_to_string(&body).unwrap(), before);
}

#[test]
fn storing_a_position_undoes_to_the_byte() {
    let mut project = Project::new();
    project.write("shapes.ttl", &shapes());
    let registration = project.write("shapes.adp", "w3c/shacl\r\nbody: shapes.ttl\r\n");
    let before = fs::read_to_string(&registration).unwrap();
    let mut session = project.open("shapes.adp");
    session.baseline(project.workspace.documents()).unwrap();
    session
        .move_elements(
            &mut project.workspace,
            &[("res:http://example.org/PersonShape".to_owned(), 120.0, 240.0)],
        )
        .unwrap();
    assert_ne!(fs::read_to_string(&registration).unwrap(), before);
    assert!(matches!(project.workspace.undo(), Outcome::Done { .. }));
    assert_eq!(fs::read_to_string(&registration).unwrap(), before);
}

#[test]
fn an_edit_through_one_reading_is_visible_in_the_other_on_one_history() {
    let mut project = Project::new();
    let body = project.write("shapes.ttl", &shapes());
    project.write("shapes.adp", "w3c/shacl\r\nbody: shapes.ttl\r\n");
    project.write("graph.adp", "w3c/rdf\r\nbody: shapes.ttl\r\n");
    let before = fs::read_to_string(&body).unwrap();
    let mut shapes_view = project.open("shapes.adp");
    let mut graph_view = project.open("graph.adp");
    shapes_view.baseline(project.workspace.documents()).unwrap();
    graph_view.baseline(project.workspace.documents()).unwrap();

    let command = RdfCommand::new(
        &body,
        RdfEdit::CreateShaclNodeShape {
            shape: "http://example.org/AddressShape".to_owned(),
        },
    );
    assert!(matches!(
        project.workspace.execute_command(&command),
        Outcome::Done { .. }
    ));
    let model = store::get(project.workspace.documents(), &body).unwrap().model.clone();
    assert_eq!(projection::project(&model, DEFAULT_BUDGET).cards.len(), 2);
    assert!(
        model
            .triples
            .iter()
            .any(|t| t.subject.is_iri("http://example.org/AddressShape"))
    );
    assert_eq!(shapes_view.render(project.workspace.documents()).unwrap().len(), 2);

    assert!(matches!(project.workspace.undo(), Outcome::Done { .. }));
    assert_eq!(fs::read_to_string(&body).unwrap(), before);
    let model = store::get(project.workspace.documents(), &body).unwrap().model.clone();
    assert_eq!(projection::project(&model, DEFAULT_BUDGET).cards.len(), 1);
}

#[test]
fn one_file_under_two_readings_is_served_by_one_store() {
    let mut project = Project::new();
    project.write("shapes.ttl", &shapes());
    project.write("shapes.adp", "w3c/shacl\r\nbody: shapes.ttl\r\n");
    project.write("graph.adp", "w3c/rdf\r\nbody: shapes.ttl\r\n");
    let mut shapes_view = project.open("shapes.adp");
    let mut graph_view = project.open("graph.adp");
    let shape_elements = added(&shapes_view.baseline(project.workspace.documents()).unwrap());
    let graph_elements = added(&graph_view.baseline(project.workspace.documents()).unwrap());
    assert_eq!(shape_elements.len(), 1);
    assert!(graph_elements.len() > shape_elements.len());
    assert!(!graph_elements.iter().any(|e| e.element_type == SHAPE_TYPE));
    assert!(graph_elements.iter().any(|e| e.element_type == graph::RESOURCE_TYPE));
}

#[test]
fn the_document_factory_writes_a_starter_that_opens_as_a_card() {
    let text = empty_shacl_document("Person");
    let model = parser::parse(&LineDocument::parse(&text)).unwrap();
    let projection = projection::project(&model, DEFAULT_BUDGET);
    assert_eq!(projection.cards.len(), 1);
    assert_eq!(projection.cards[0].display, "ex:PersonShape");
    assert_eq!(projection.cards[0].targets.len(), 1);
    assert_eq!(projection.cards[0].rows.len(), 1);
}

fn two_shapes_session(project: &mut Project) -> DiagramSession {
    project.write("shapes.ttl", &two_shapes());
    project.write("shapes.adp", "w3c/shacl\r\nbody: shapes.ttl\r\n");
    project.open("shapes.adp")
}

#[test]
fn a_viewport_change_adds_what_came_into_view_and_removes_what_left() {
    let mut project = Project::new();
    let mut session = two_shapes_session(&mut project);
    let drawn = added(&session.baseline(project.workspace.documents()).unwrap());
    let anchor = drawn
        .iter()
        .min_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)))
        .unwrap()
        .clone();
    let narrowed = session
        .update_view(
            project.workspace.documents(),
            DiagramViewport {
                min_x: anchor.x,
                min_y: anchor.y,
                max_x: anchor.x + 1.0,
                max_y: anchor.y + 1.0,
            },
        )
        .unwrap();
    let widened = session
        .update_view(project.workspace.documents(), DiagramViewport::UNBOUNDED)
        .unwrap();
    let gone = removed(&narrowed);
    assert!(!gone.is_empty());
    assert!(!gone.contains(&anchor.id));
    let restored: HashSet<String> = added(&widened).into_iter().map(|e| e.id).collect();
    assert!(gone.is_subset(&restored));
}

#[test]
fn an_unchanged_viewport_says_nothing_twice() {
    let mut project = Project::new();
    let mut session = two_shapes_session(&mut project);
    session
        .update_view(project.workspace.documents(), DiagramViewport::UNBOUNDED)
        .unwrap();
    let again = session
        .update_view(project.workspace.documents(), DiagramViewport::UNBOUNDED)
        .unwrap();
    assert!(again.is_empty(), "{again:?}");
}

#[test]
fn panning_does_not_move_the_elements_it_brings_into_view() {
    let mut project = Project::new();
    let mut session = two_shapes_session(&mut project);
    let at_open: BTreeMap<String, (f64, f64)> = added(&session.baseline(project.workspace.documents()).unwrap())
        .into_iter()
        .map(|e| (e.id, (e.x, e.y)))
        .collect();
    session
        .update_view(
            project.workspace.documents(),
            DiagramViewport {
                min_x: 0.0,
                min_y: 0.0,
                max_x: 1.0,
                max_y: 1.0,
            },
        )
        .unwrap();
    let readmitted = added(
        &session
            .update_view(project.workspace.documents(), DiagramViewport::UNBOUNDED)
            .unwrap(),
    );
    assert!(!readmitted.is_empty());
    for element in readmitted {
        assert_eq!(at_open[&element.id], (element.x, element.y));
    }
}
