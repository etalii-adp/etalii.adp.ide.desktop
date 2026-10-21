//! A session over the field-service example, through the project's documents on a real file: what it delivers, what
//! a selection reads as, what the panel lists, and what a change on disk sends (ported from the standalone host's
//! `FdgSession.Tests.cs`, `FdgContextSourceResolver.Tests.cs`, `FdgDocumentStore.Tests.cs`,
//! `FdgDocumentReloader.Tests.cs` and `ServiceCollectionAddFunctionalDecompositionGraph.Tests.cs` at `8b6ccfc`).

mod common;

use std::fs;
use std::path::Path;

use adp_diagram::{DiagramDelta, DiagramElement, DiagramWorkspace, DocumentChange, ElementKind};
use adp_diagram_functional_decomposition_graph::lines::LineDocument;
use adp_diagram_functional_decomposition_graph::mapper::wire_type;
use adp_diagram_functional_decomposition_graph::model::{FdgModel, geometry};
use adp_diagram_functional_decomposition_graph::{EMPTY_DOCUMENT, ORIGIN, parser, writer};
use common::{Graph, example_project, tools};

fn added(deltas: &[DiagramDelta]) -> Vec<DiagramElement> {
    deltas
        .iter()
        .flat_map(|d| match d {
            DiagramDelta::Add(elements) => elements.clone(),
            _ => Vec::new(),
        })
        .collect()
}

fn number(element: &DiagramElement, name: &str) -> f64 {
    element.attribute(name).parse().unwrap()
}

#[test]
fn a_fresh_session_delivers_every_element_and_connection_of_the_example() {
    let (folder, body) = example_project();
    let model = common::parse_file(&body);
    let mut workspace = DiagramWorkspace::new(folder.path());
    let (opened, mut session) = workspace.open(&mut tools(), Path::new("field-service.adp")).unwrap();
    assert_eq!(opened.origin, ORIGIN);
    assert_eq!(opened.body_path, body);

    let mut delivered: Vec<String> = added(&session.baseline(workspace.documents()).unwrap())
        .into_iter()
        .map(|e| e.id)
        .collect();
    let mut expected: Vec<String> = model
        .elements
        .iter()
        .map(|e| e.id.clone())
        .chain(model.connections.iter().map(|c| c.id.clone()))
        .collect();
    delivered.sort();
    expected.sort();
    assert_eq!(delivered, expected);
}

#[test]
fn a_body_opens_on_its_own_and_with_the_registration_beside_it() {
    let (folder, body) = example_project();
    let mut workspace = DiagramWorkspace::new(folder.path());
    let (opened, _) = workspace.open(&mut tools(), Path::new("field-service.fdg")).unwrap();
    assert_eq!(opened.origin, ORIGIN);
    assert_eq!(opened.body_path, body);
    assert_eq!(opened.registration_path, Some(folder.path().join("field-service.adp")));
}

#[test]
fn every_element_arrives_at_the_shared_height_and_a_comment_at_its_own() {
    let graph = Graph::example();
    let elements = graph.session.render(graph.workspace.documents()).unwrap();
    let nodes: Vec<&DiagramElement> = elements.iter().filter(|e| e.kind == ElementKind::Node).collect();
    assert!(!nodes.is_empty());
    for node in nodes {
        let expected = if node.element_type == wire_type("comment") {
            96.0
        } else {
            geometry::SHARED_HEIGHT
        };
        assert_eq!(number(node, "height"), expected, "{}", node.id);
    }
}

#[test]
fn an_element_arrives_at_its_centre_with_its_size() {
    // The example's Planning element sits at top-left (600, 40), 140 wide.
    let graph = Graph::example();
    let elements = graph.session.render(graph.workspace.documents()).unwrap();
    let planning = elements.iter().find(|e| e.id == "planning").unwrap();
    assert_eq!(
        planning.element_type,
        "etalii/functional-decomposition-graph+ui-element"
    );
    assert_eq!((planning.x, planning.y), (600.0 + 70.0, 40.0 + 24.0));
    assert_eq!(number(planning, "width"), 140.0);
    assert_eq!(planning.attribute("name"), "Planning");

    let shows = elements.iter().find(|e| e.id == "c-open-shows-detail").unwrap();
    assert_eq!(shows.kind, ElementKind::Relation);
    assert_eq!(
        (shows.source.as_deref(), shows.target.as_deref()),
        (Some("open-task"), Some("task-detail"))
    );
    assert_eq!(shows.attribute("name"), "opens");
}

fn edit_on_disk(graph: &mut Graph, id: &str, edit: impl Fn(&mut LineDocument, &FdgModel, &str) -> Result<(), String>) {
    let mut document = LineDocument::parse(&fs::read_to_string(&graph.body).unwrap());
    let model = parser::parse(&document);
    edit(&mut document, &model, id).unwrap();
    fs::write(&graph.body, document.text()).unwrap();
    graph.workspace.reconcile(&graph.body);
}

#[test]
fn a_resize_on_disk_arrives_as_a_delta() {
    let mut graph = Graph::example();
    edit_on_disk(&mut graph, "task-list", |document, model, id| {
        writer::resize(document, model.element_of(id).unwrap(), 220.0, None)
    });
    let body = graph.body.clone();
    let DocumentChange::Changed(deltas) = graph.session.on_document_changed(graph.workspace.documents(), &body) else {
        panic!("the resize is sent");
    };
    let changed = added(&deltas);
    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0].id, "task-list");
    assert_eq!(number(&changed[0], "width"), 220.0);
}

/// A description is never sent, so a change to one alone sends nothing, while a name change does.
#[test]
fn a_description_changed_on_disk_sends_nothing_while_a_name_change_does() {
    let mut graph = Graph::example();
    let body = graph.body.clone();
    edit_on_disk(&mut graph, "task-list", |document, model, id| {
        writer::set_description(
            document,
            model.element_of(id).unwrap(),
            "A different description altogether.",
        )
    });
    assert!(matches!(
        graph.session.on_document_changed(graph.workspace.documents(), &body),
        DocumentChange::NothingVisible
    ));

    edit_on_disk(&mut graph, "task-list", |document, model, id| {
        writer::set_name(document, model.element_of(id).unwrap(), "Today")
    });
    let DocumentChange::Changed(deltas) = graph.session.on_document_changed(graph.workspace.documents(), &body) else {
        panic!("the rename is sent");
    };
    let changed = added(&deltas);
    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0].attribute("name"), "Today");
}

#[test]
fn nothing_drawn_carries_a_description() {
    let graph = Graph::example();
    for element in graph.session.render(graph.workspace.documents()).unwrap() {
        assert!(
            !element.attributes.iter().any(|(name, _)| name.contains("description")),
            "{}",
            element.id
        );
    }
}

#[test]
fn a_selection_reads_as_the_element_the_canvas_draws() {
    let graph = Graph::example();
    let documents = graph.workspace.documents();
    let selected = |id: &str| graph.session.select(documents, id).map(|s| s.text);
    assert_eq!(selected("planning").as_deref(), Some("Planning"));
    assert_eq!(
        selected("note-offline").as_deref(),
        Some("Everything on these screens must work offline.")
    );
    assert_eq!(selected("c-open-shows-detail").as_deref(), Some("opens"));
    assert_eq!(
        selected("c-planning-task-list").as_deref(),
        Some("planning → task-list")
    );
    assert_eq!(selected("nothing-here"), None);
}

#[test]
fn the_example_lists_no_problem_and_a_broken_document_lists_its_breaches() {
    let graph = Graph::example();
    assert_eq!(graph.session.problems(graph.workspace.documents()), []);

    let broken = Graph::of(&fs::read_to_string(common::fixture_path("rule-second-shows.fdg")).unwrap());
    let problems = broken.session.problems(broken.workspace.documents());
    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0].rule_id, "fdg.second-shows");
}

/// A body changed by another program is read again, and the history stops there.
#[test]
fn a_body_changed_on_disk_is_read_again() {
    let mut graph = Graph::example();
    let text = fs::read_to_string(&graph.body)
        .unwrap()
        .replace("name: Planning", "name: Today's plan");
    fs::write(&graph.body, &text).unwrap();
    graph.workspace.reconcile(&graph.body);
    assert_eq!(graph.model().element_of("planning").unwrap().name, "Today's plan");
    let elements = graph.session.render(graph.workspace.documents()).unwrap();
    assert_eq!(
        elements.iter().find(|e| e.id == "planning").unwrap().attribute("name"),
        "Today's plan"
    );
}

#[test]
fn a_deleted_body_ends_its_document() {
    let mut graph = Graph::example();
    fs::remove_file(&graph.body).unwrap();
    graph.workspace.reconcile(&graph.body);
    assert_eq!(graph.session.render(graph.workspace.documents()).unwrap(), []);
}

#[test]
fn the_empty_document_reads_as_an_empty_diagram_of_the_current_version_with_no_problem() {
    let model = parser::parse(&LineDocument::parse(EMPTY_DOCUMENT));
    assert!(model.problems.is_empty());
    assert!(model.elements.is_empty());
    assert!(model.connections.is_empty());
    assert_eq!(model.version, Some(FdgModel::CURRENT_VERSION));
    assert!(EMPTY_DOCUMENT.ends_with("\r\n"));
    assert_eq!(EMPTY_DOCUMENT, parser::empty_document("\r\n"));
}

/// An empty document takes a first element and a first connection, each section opened from its flow-empty form.
#[test]
fn an_empty_document_takes_its_first_element_and_connection() {
    let mut graph = Graph::of(EMPTY_DOCUMENT);
    use adp_diagram::ActionOutcome;
    assert_eq!(
        graph.execute(common::placement(100.0, 100.0), "fdg.add.ui-element"),
        ActionOutcome::Done
    );
    assert_eq!(
        graph.execute(common::placement(100.0, 300.0), "fdg.add.action"),
        ActionOutcome::Done
    );
    let model = graph.model();
    let (page, action) = (model.elements[0].id.clone(), model.elements[1].id.clone());
    assert_eq!(
        graph.execute(common::gesture(&page, &action), "fdg.connect.owns-action"),
        ActionOutcome::Done
    );
    let model = graph.model();
    assert!(model.problems.is_empty(), "{:?}", model.problems);
    assert_eq!(model.connections.len(), 1);
    let text = String::from_utf8(graph.bytes()).unwrap();
    assert!(
        text.starts_with("functional-decomposition-graph: 1\r\nelements:\r\n  - id: "),
        "{text}"
    );
    assert!(!text.contains("[]"), "{text}");
}
