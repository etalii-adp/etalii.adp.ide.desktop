//! The shapes reading's context surface and property grid, offered through the family's providers: scoped to targets
//! opened as `w3c/shacl`, a target chip removable by its own address, the gate refusing a blank-rooted selection
//! before anything runs, and blank-rooted content readable everywhere and writable nowhere. Ported from the standalone
//! host's `ShaclActions.Tests.cs` and `ShaclProperties.Tests.cs` at `8b6ccfc`, through `shacl::actions` and
//! `shacl::properties` with the edits they make run on the project's history.

use std::fs;
use std::path::PathBuf;

use adp_context::{Prompt, PropertyRow};
use adp_diagram::DiagramDocuments;
use adp_diagram_rdf::commands::{RdfCommand, RdfEdit};
use adp_diagram_rdf::family::{Run, Target};
use adp_diagram_rdf::shacl::projection::{self, DEFAULT_BUDGET};
use adp_diagram_rdf::shacl::refusals::BLANK_ROOTED;
use adp_diagram_rdf::shacl::{self, actions, properties};
use adp_diagram_rdf::store::{self, RdfDocument};
use adp_diagram_rdf::vocabulary::shacl as sh;
use adp_diagram_rdf::{graph, skos};
use adp_history::{History, Outcome};

/// A C# raw string literal from a file checked out with CRLF: its lines joined by CRLF, no line break after the last.
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
        "    sh:property [ sh:path ex:name ; sh:datatype xsd:string ] .",
    ])
}

const CARD_ID: &str = "res:http://example.org/PersonShape";

struct Project {
    _folder: tempfile::TempDir,
    body: PathBuf,
    documents: DiagramDocuments,
    history: History<DiagramDocuments>,
}

impl Project {
    fn new(text: &str) -> Self {
        let folder = tempfile::tempdir().unwrap();
        let body = folder.path().join("shapes.ttl");
        fs::write(&body, text).unwrap();
        Project {
            _folder: folder,
            body,
            documents: DiagramDocuments::default(),
            history: History::default(),
        }
    }

    fn document(&mut self) -> RdfDocument {
        store::get_or_load(&mut self.documents, &self.body).clone()
    }

    fn target<'a>(&'a self, element_id: &'a str, origin: &'a str) -> Target<'a> {
        Target {
            element_id,
            body_path: &self.body,
            origin,
        }
    }

    fn execute(&mut self, edit: RdfEdit) -> Outcome {
        let command = RdfCommand::new(&self.body, edit);
        self.history.execute(&mut self.documents, &command)
    }

    fn text(&self) -> String {
        fs::read_to_string(&self.body).unwrap()
    }
}

fn ids(groups: &[adp_context::ActionGroup]) -> Vec<String> {
    groups.iter().flat_map(|g| &g.actions).map(|a| a.id.clone()).collect()
}

#[test]
fn the_cases_answer_for_their_own_origin() {
    let mut project = Project::new(&shapes());
    let document = project.document();
    let groups = actions::discover(&document, &project.target(CARD_ID, shacl::ORIGIN));
    assert_eq!(groups.len(), 1);
    let ids = ids(&groups);
    for id in [
        actions::ADD_TARGET_CLASS,
        actions::ADD_PROPERTY_ROW,
        actions::DEACTIVATE,
        actions::REMOVE_SHAPE,
    ] {
        assert!(ids.iter().any(|i| i == id), "{id}");
    }
}

#[test]
fn the_cases_stay_silent_for_another_readings_origin() {
    let mut project = Project::new(&shapes());
    let document = project.document();
    assert!(actions::discover(&document, &project.target(CARD_ID, graph::ORIGIN)).is_empty());
    assert!(actions::discover(&document, &project.target(CARD_ID, skos::ORIGIN)).is_empty());
}

#[test]
fn a_target_chip_is_removable_by_its_own_address() {
    let mut project = Project::new(&shapes());
    let document = project.document();
    let groups = actions::discover(&document, &project.target(CARD_ID, shacl::ORIGIN));
    let removes: Vec<_> = groups[0]
        .actions
        .iter()
        .filter(|a| a.id.starts_with(actions::REMOVE_TARGET_PREFIX))
        .collect();
    assert_eq!(removes.len(), 1);
    assert_eq!(
        removes[0].id,
        format!(
            "{}{}|http://example.org/Person",
            actions::REMOVE_TARGET_PREFIX,
            sh::TARGET_CLASS
        )
    );
    assert!(removes[0].label.contains("class ex:Person"));
}

#[test]
fn the_remove_entry_states_the_count_before_anything_runs() {
    let mut project = Project::new(&shapes());
    let document = project.document();
    let groups = actions::discover(&document, &project.target(CARD_ID, shacl::ORIGIN));
    let remove = groups[0]
        .actions
        .iter()
        .find(|a| a.id == actions::REMOVE_SHAPE)
        .unwrap();
    assert!(remove.label.contains("with 5 statements"), "{}", remove.label);
}

#[test]
fn a_blank_rooted_selection_is_marked_unavailable_and_refused_on_execute() {
    let mut project = Project::new(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .\r\n@prefix ex: <http://example.org/> .\r\n\r\nex:S a sh:NodeShape ; sh:node [ sh:closed true ] .\r\n",
    );
    let document = project.document();
    let blank_id = projection::project(&document.model, DEFAULT_BUDGET)
        .cards
        .into_iter()
        .find(|c| c.blank)
        .unwrap()
        .id;
    let target = project.target(&blank_id, shacl::ORIGIN);
    let groups = actions::discover(&document, &target);
    assert_eq!(groups.len(), 1);
    for action in &groups[0].actions {
        assert!(!action.is_available());
        assert_eq!(action.unavailable_reason, BLANK_ROOTED);
    }
    assert_eq!(
        actions::execute(&document, &target, actions::DEACTIVATE),
        Some(Run::Refused(BLANK_ROOTED.to_owned()))
    );
}

#[test]
fn adding_a_target_asks_for_a_term_then_commits_it_as_one_undoable_edit() {
    let mut project = Project::new(&shapes());
    let document = project.document();
    let target = project.target(CARD_ID, shacl::ORIGIN);
    assert!(matches!(
        actions::execute(&document, &target, actions::ADD_TARGET_CLASS),
        Some(Run::Ask(Prompt::Input { .. }))
    ));
    assert!(matches!(
        actions::validate(&document, actions::ADD_TARGET_CLASS, "nope:Thing"),
        Some(Err(_))
    ));
    let edit = actions::command_for(&document, &target, actions::ADD_TARGET_CLASS, "ex:Employee").unwrap();
    assert!(matches!(edit, RdfEdit::AddShaclTarget { .. }));
    assert!(matches!(project.execute(edit), Outcome::Done { .. }));
    assert!(project.text().contains("sh:targetClass ex:Employee"));
}

#[test]
fn removing_a_target_by_its_chip_address_takes_exactly_that_declaration() {
    let mut project = Project::new(&shapes());
    let document = project.document();
    let action = format!(
        "{}{}|http://example.org/Person",
        actions::REMOVE_TARGET_PREFIX,
        sh::TARGET_CLASS
    );
    let Some(Run::Edit(edit)) = actions::execute(&document, &project.target(CARD_ID, shacl::ORIGIN), &action) else {
        panic!("not an edit");
    };
    assert!(matches!(project.execute(edit), Outcome::Done { .. }));
    let text = project.text();
    assert!(!text.contains("sh:targetClass"));
    assert!(text.contains("sh:property [ sh:path ex:name"));
}

#[test]
fn the_palette_offers_the_two_entries_the_menu_also_runs() {
    let drops: Vec<String> = shacl::toolbox().into_iter().map(|i| i.drop_action_id).collect();
    assert_eq!(drops, [actions::ADD_NODE_SHAPE, actions::ADD_PROPERTY_ROW]);
}

// ShaclProperties.Tests.cs

fn property_shapes() -> String {
    raw(&[
        "@prefix sh: <http://www.w3.org/ns/shacl#> .",
        "@prefix ex: <http://example.org/> .",
        "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .",
        "",
        "ex:PersonShape a sh:NodeShape ;",
        "    sh:name \"Person\" ;",
        "    sh:description \"Every person we hold.\" ;",
        "    sh:message \"A person must have a name.\"@en ;",
        "    sh:closed true ;",
        "    sh:severity sh:Warning ;",
        "    sh:targetClass ex:Person ;",
        "    sh:property [ sh:path ex:name ; sh:datatype xsd:string ; sh:minCount 1 ; sh:maxCount 1 ] ;",
        "    sh:sparql [ sh:select \"SELECT $this WHERE { $this ex:name ?n }\" ] .",
        "",
        "# A shape with no IRI of its own, stated free-standing rather than as some other shape's",
        "# property - so it is drawn as a card, and the identity boundary applies to a whole card.",
        "[] a sh:NodeShape ;",
        "    sh:targetClass ex:Thing ;",
        "    sh:name \"Anonymous\" .",
    ])
}

fn grid(project: &mut Project) -> Vec<PropertyRow> {
    let document = project.document();
    properties::describe(&document, &project.target(CARD_ID, shacl::ORIGIN)).unwrap()
}

fn row<'a>(rows: &'a [PropertyRow], id: &str) -> &'a PropertyRow {
    rows.iter()
        .find(|r| r.id == id)
        .unwrap_or_else(|| panic!("no row {id}"))
}

#[test]
fn the_grid_answers_for_its_own_origin_only() {
    let mut project = Project::new(&property_shapes());
    let document = project.document();
    assert!(properties::describe(&document, &project.target(CARD_ID, shacl::ORIGIN)).is_some());
    assert!(properties::describe(&document, &project.target(CARD_ID, graph::ORIGIN)).is_none());
    assert!(properties::describe(&document, &project.target(CARD_ID, skos::ORIGIN)).is_none());
}

#[test]
fn a_card_shows_its_identity_annotations_and_flags() {
    let mut project = Project::new(&property_shapes());
    let rows = grid(&mut project);
    assert_eq!(row(&rows, "shacl.iri").value, "http://example.org/PersonShape");
    assert_eq!(row(&rows, "shacl.name").value, "Person");
    assert_eq!(row(&rows, "shacl.description").value, "Every person we hold.");
    assert_eq!(row(&rows, "shacl.message").value, "A person must have a name.");
    assert_eq!(row(&rows, "shacl.closed").value, "true");
    assert_eq!(row(&rows, "shacl.severity").value, "sh:Warning");
    assert_eq!(row(&rows, "shacl.deactivated").value, "false");
}

#[test]
fn only_the_two_annotations_requirement_5_writes_are_editable() {
    let mut project = Project::new(&property_shapes());
    let rows = grid(&mut project);
    assert!(!row(&rows, "shacl.name").is_read_only());
    assert!(!row(&rows, "shacl.description").is_read_only());
    let read_only: Vec<&PropertyRow> = rows.iter().filter(|r| r.is_read_only()).collect();
    assert_eq!(read_only.len(), rows.len() - 2);
}

#[test]
fn a_blank_rooted_row_is_fully_readable_and_refused_with_the_one_sentence() {
    let mut project = Project::new(&property_shapes());
    let rows = grid(&mut project);
    let constraint = row(&rows, "shacl.row:0");
    assert_eq!(constraint.label, "ex:name");
    assert!(constraint.value.contains("xsd:string"));
    assert!(constraint.value.contains("[1..1]"));
    assert_eq!(constraint.read_only_reason, BLANK_ROOTED);
}

#[test]
fn an_absent_target_reads_as_a_fact_rather_than_a_finding() {
    let mut project = Project::new(&property_shapes());
    let rows = grid(&mut project);
    let target = row(&rows, "shacl.target:0");
    assert_eq!(target.label, "Target class");
    assert_eq!(target.value, "ex:Person (not described in this file)");
    assert!(!target.value.to_lowercase().contains("error"));
    assert!(!target.value.to_lowercase().contains("missing"));
}

#[test]
fn a_sparql_constraint_shows_its_query_text_and_says_it_is_not_run() {
    let mut project = Project::new(&property_shapes());
    let rows = grid(&mut project);
    let query = row(&rows, "shacl.sparql:0");
    assert_eq!(query.value, "SELECT $this WHERE { $this ex:name ?n }");
    assert!(query.is_read_only());
    assert!(query.read_only_reason.contains("never executed"));
}

#[test]
fn editing_the_name_lands_as_a_minimal_diff_and_undoes_to_the_byte() {
    let mut project = Project::new(&property_shapes());
    let before = project.text();
    let document = project.document();
    let edit = properties::command_for(
        &document,
        &project.target(CARD_ID, shacl::ORIGIN),
        "shacl.name",
        "Human",
    )
    .unwrap();
    assert!(matches!(project.execute(edit), Outcome::Done { .. }));
    assert_eq!(project.text(), before.replace("\"Person\"", "\"Human\""));
    assert!(matches!(
        project.history.undo(&mut project.documents),
        Outcome::Done { .. }
    ));
    assert_eq!(project.text(), before);
}

#[test]
fn a_blank_card_is_readable_throughout_and_writable_nowhere() {
    let mut project = Project::new(&property_shapes());
    let document = project.document();
    let blank_id = projection::project(&document.model, DEFAULT_BUDGET)
        .cards
        .into_iter()
        .find(|c| c.blank)
        .unwrap()
        .id;
    let target = project.target(&blank_id, shacl::ORIGIN);
    let rows = properties::describe(&document, &target).unwrap();
    assert!(!rows.is_empty());
    assert!(rows.iter().all(PropertyRow::is_read_only));
    assert_eq!(row(&rows, "shacl.iri").read_only_reason, BLANK_ROOTED);
    assert!(properties::command_for(&document, &target, "shacl.name", "anything").is_none());
}

#[test]
fn a_row_that_is_not_this_readings_card_gets_no_grid_at_all() {
    let mut project = Project::new(&property_shapes());
    let document = project.document();
    assert!(
        properties::describe(
            &document,
            &project.target("res:http://example.org/Person", shacl::ORIGIN)
        )
        .is_none()
    );
}
