//! A query opened in a project, and what is asked of it (etalii.adp spec 030, task T084): the standalone host's
//! `SparqlSession.Tests.cs`, `SparqlProviders.Tests.cs` (the property rows, the action provider, the validator and the
//! source resolver) and `ExampleCorpus.Tests.cs` at `8b6ccfc`, with the same expectations. The standalone host's
//! sessions heard a query change through its reloader; here the project's reconciliation hands the changed path to
//! the module.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use adp_core::catalogue::Catalogue;
use adp_diagram::session::ActionTarget;
use adp_diagram::{
    DiagramDelta, DiagramElement, DiagramProblemLocation, DiagramProblemSeverity, DiagramSession, DiagramTools,
    DiagramViewport, DiagramWorkspace, DiskChange,
};
use adp_diagram_sparql::mapper::{
    ANNOTATION_TYPE, EDGE_TYPE, HEADER_ID, REGION_TYPE, TRUNCATION_ID, VARIABLE_TYPE,
};
use adp_diagram_sparql::rows::{self, READ_ONLY_REASON};
use adp_diagram_sparql::view::{NO_REGISTRATION, PARENT_REFUSAL};
use adp_diagram_sparql::{findings, parser, projection, registration_layout};
use adp_disl::DislPluginFunctions;
use common::{examples, fixture};

fn tools() -> DiagramTools {
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
    adp_diagram_sparql::register(DiagramTools::new(Catalogue::load(&vendor), |_| {
        DislPluginFunctions::none()
    }))
}

struct Opened {
    _folder: tempfile::TempDir,
    body: PathBuf,
    registration: PathBuf,
    workspace: DiagramWorkspace,
    session: DiagramSession,
}

/// The fixture `name` copied into a project, with a registration `query.adp` naming it, as the standalone host's
/// tests wrote one; opened through its registration, or bare when `registered` is false.
fn open(name: &str, registered: bool) -> Opened {
    let folder = tempfile::tempdir().unwrap();
    let body = folder.path().join(name);
    fs::copy(fixture(name), &body).unwrap();
    let registration = folder.path().join("query.adp");
    fs::write(&registration, format!("w3c/sparql\r\nbody: {name}\r\n")).unwrap();
    let mut workspace = DiagramWorkspace::new(folder.path());
    let opened_path = if registered { "query.adp" } else { name };
    let (opened, session) = workspace.open(&mut tools(), Path::new(opened_path)).unwrap();
    assert_eq!(opened.origin, "w3c/sparql");
    assert_eq!(opened.body_path, body);
    assert_eq!(opened.registration_path.is_some(), registered);
    Opened {
        _folder: folder,
        body,
        registration,
        workspace,
        session,
    }
}

impl Opened {
    fn delivered(&mut self) -> Vec<DiagramElement> {
        match self
            .session
            .baseline(self.workspace.documents())
            .unwrap()
            .into_iter()
            .next()
        {
            Some(DiagramDelta::Add(elements)) => elements,
            other => panic!("{other:?}"),
        }
    }

    fn move_to(&mut self, id: &str, x: f64, y: f64) -> Result<(), String> {
        self.session.move_element_to(&mut self.workspace, id, x, y)
    }
}

/// A viewport tight around one element.
fn around(element: &DiagramElement) -> DiagramViewport {
    DiagramViewport::new(element.x - 1.0, element.y - 1.0, element.x + 1.0, element.y + 1.0)
}

#[test]
fn a_registered_query_opens_with_nodes_edges_regions_and_the_header() {
    let mut opened = open("groups.rq", true);
    assert!(opened.session.module_view().is_some());
    let elements = opened.delivered();
    let has = |id: &str, kind: &str| elements.iter().any(|e| e.id == id && e.element_type == kind);
    assert!(has("var:x", VARIABLE_TYPE));
    assert!(elements.iter().any(|e| e.element_type == EDGE_TYPE));
    assert!(has("region:where/optional.0", REGION_TYPE));
    assert!(elements.iter().any(|e| e.element_type == ANNOTATION_TYPE));
    assert!(elements.iter().any(|e| e.id == HEADER_ID));
    assert!(!elements.iter().any(|e| e.id == TRUNCATION_ID));
}

#[test]
fn a_changed_viewport_removes_what_left_then_adds_what_appeared() {
    let mut opened = open("groups.rq", true);
    let regions: Vec<DiagramElement> = opened
        .delivered()
        .into_iter()
        .filter(|e| e.element_type == REGION_TYPE)
        .collect();
    let leftmost = regions.iter().min_by(|a, b| a.x.total_cmp(&b.x)).unwrap().clone();
    let rightmost = regions.iter().max_by(|a, b| a.x.total_cmp(&b.x)).unwrap().clone();
    assert_ne!(leftmost.id, rightmost.id);

    let narrowed = opened
        .session
        .update_view(opened.workspace.documents(), around(&leftmost))
        .unwrap();
    assert!(narrowed
        .iter()
        .any(|d| matches!(d, DiagramDelta::Remove(ids) if ids.contains(&rightmost.id))));

    let moved = opened
        .session
        .update_view(opened.workspace.documents(), around(&rightmost))
        .unwrap();
    assert_eq!(moved.len(), 2, "{moved:?}");
    assert!(matches!(&moved[0], DiagramDelta::Remove(ids) if ids.contains(&leftmost.id)));
    assert!(matches!(&moved[1], DiagramDelta::Add(elements) if elements.iter().any(|e| e.id == rightmost.id)));
}

#[test]
fn a_stored_position_overlays_the_computed_one() {
    let folder = tempfile::tempdir().unwrap();
    fs::copy(fixture("groups.rq"), folder.path().join("groups.rq")).unwrap();
    fs::write(
        folder.path().join("query.adp"),
        "w3c/sparql\r\nbody: groups.rq\r\nlayout:\r\n  var:x: 640 480\r\n",
    )
    .unwrap();
    let mut workspace = DiagramWorkspace::new(folder.path());
    let (_, mut session) = workspace.open(&mut tools(), Path::new("query.adp")).unwrap();
    let Some(DiagramDelta::Add(elements)) = session.baseline(workspace.documents()).unwrap().into_iter().next() else {
        panic!("a baseline");
    };
    let x = elements.iter().find(|e| e.id == "var:x").unwrap();
    assert_eq!((x.x, x.y), (640.0, 480.0));
}

#[test]
fn repositioning_writes_the_registration_and_leaves_the_query_byte_identical() {
    let mut opened = open("groups.rq", true);
    let original = fs::read(&opened.body).unwrap();
    opened.move_to("var:x", 320.0, 200.0).unwrap();
    assert_eq!(fs::read(&opened.body).unwrap(), original);
    assert!(fs::read_to_string(&opened.registration).unwrap().contains("var:x: 320 200"));
    // And the next rendering draws it there.
    let rendered = opened.session.render(opened.workspace.documents()).unwrap();
    let x = rendered.iter().find(|e| e.id == "var:x").unwrap();
    assert_eq!((x.x, x.y), (320.0, 200.0));
}

#[test]
fn repositioning_is_one_undo_away_and_undo_also_leaves_the_query_alone() {
    let mut opened = open("groups.rq", true);
    let original = fs::read(&opened.body).unwrap();
    let before = fs::read(&opened.registration).unwrap();
    opened.move_to("var:x", 320.0, 200.0).unwrap();
    assert!(opened.workspace.undo().is_done());
    assert_eq!(fs::read(&opened.registration).unwrap(), before);
    assert_eq!(fs::read(&opened.body).unwrap(), original);
}

#[test]
fn moving_a_region_moves_its_frame_with_its_contents_following() {
    let mut opened = open("groups.rq", true);
    let elements = opened.delivered();
    let frame = elements.iter().find(|e| e.id == "region:where/optional.1").unwrap().clone();
    let y = elements.iter().find(|e| e.id == "var:y").unwrap().clone();
    opened.move_to(&frame.id, frame.x + 100.0, frame.y + 50.0).unwrap();
    let stored = registration_layout::read(&fs::read_to_string(&opened.registration).unwrap());
    assert_eq!(stored[&frame.id], (frame.x + 100.0, frame.y + 50.0));
    let rendered = opened.session.render(opened.workspace.documents()).unwrap();
    let moved = rendered.iter().find(|e| e.id == "var:y").unwrap();
    assert_eq!((moved.x, moved.y), (y.x + 100.0, y.y + 50.0));
}

#[test]
fn each_unmovable_surface_refuses_with_its_own_reason() {
    let mut opened = open("groups.rq", true);
    let before = fs::read(&opened.registration).unwrap();
    for (id, expected) in [
        ("anon:0", "anonymous variable"),
        ("edge:var:s|ex:p|var:x|0", "endpoints"),
        ("note:where/filter.0", "annotation stays"),
        (HEADER_ID, "header band"),
        (TRUNCATION_ID, "banner"),
    ] {
        let refusal = opened.move_to(id, 10.0, 10.0).unwrap_err();
        assert!(refusal.to_lowercase().contains(expected), "{id}: {refusal}");
    }
    assert_eq!(fs::read(&opened.registration).unwrap(), before);
}

#[test]
fn a_bare_unregistered_query_refuses_naming_the_missing_registration() {
    let mut opened = open("groups.rq", false);
    let refusal = opened.move_to("var:x", 10.0, 10.0).unwrap_err();
    assert_eq!(refusal, NO_REGISTRATION);
    assert!(refusal.contains("without a registration"));
    assert!(refusal.contains("Register the file"));
}

#[test]
fn reparenting_is_refused_because_structure_comes_from_the_text() {
    let mut opened = open("groups.rq", true);
    let refusal = opened
        .session
        .move_element(&mut opened.workspace, "var:x", Some("region:where/optional.0"), 0)
        .unwrap_err();
    assert_eq!(refusal, PARENT_REFUSAL);
    assert!(refusal.contains("comes from its text"));
}

#[test]
fn an_external_edit_reaches_the_open_session() {
    let mut opened = open("lf-line-endings.rq", true);
    opened.delivered();
    fs::write(
        &opened.body,
        "PREFIX ex: <http://example.org/>\nSELECT ?s ?extra\nWHERE { ?s ex:p ?o . ?s ex:q ?extra }\n",
    )
    .unwrap();
    assert_eq!(
        opened.workspace.reconcile(&opened.body),
        DiskChange::Reloaded(opened.body.clone())
    );
    let rendered = opened.session.render(opened.workspace.documents()).unwrap();
    assert!(rendered.iter().any(|e| e.id == "var:extra"));
    // Read again with nothing new, it is unchanged.
    assert_eq!(opened.workspace.reconcile(&opened.body), DiskChange::Unchanged);
}

#[test]
fn a_query_that_does_not_parse_opens_with_only_its_header() {
    let mut opened = open("broken.rq", true);
    let elements = opened.delivered();
    assert_eq!(elements.len(), 1);
    assert_eq!(elements[0].id, HEADER_ID);
    // Its one finding names the line it stopped at.
    let problems = opened.session.problems(opened.workspace.documents());
    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0].rule_id, findings::UNPARSEABLE_RULE_ID);
    assert_eq!(problems[0].location, Some(DiagramProblemLocation::Line(4)));
    // Its header reads as a plain "Query", not the SELECT its empty model would be called.
    assert_eq!(
        opened.session.select(opened.workspace.documents(), HEADER_ID).unwrap().text,
        "Query"
    );
}

// -- The rows and the selection ----------------------------------------------------------------------------------------

#[test]
fn every_row_is_read_only_and_says_why() {
    let opened = open("constructs.rq", true);
    let mut every_row = Vec::new();
    for id in ["var:person", "iri:http://example.org/carol", HEADER_ID] {
        every_row.extend(opened.session.properties(opened.workspace.documents(), &[id]));
    }
    assert!(!every_row.is_empty());
    for row in &every_row {
        assert_eq!(row.read_only_reason, READ_ONLY_REASON);
        assert!(row.read_only_reason.contains("text editor"));
    }
}

#[test]
fn a_variables_rows_state_its_name_projection_join_count_and_definition() {
    let opened = open("groups.rq", true);
    let value = |id: &str, row: &str| {
        let rows = opened.session.properties(opened.workspace.documents(), &[id]);
        let found: Vec<_> = rows.iter().filter(|r| r.id == row).collect();
        assert_eq!(found.len(), 1, "{id} {row}");
        found[0].value.clone()
    };
    assert_eq!(value("var:x", rows::VARIABLE_NAME_PROPERTY), "?x");
    assert_eq!(value("var:x", rows::VARIABLE_PROJECTED_PROPERTY), "Yes");
    assert_eq!(value("var:x", rows::VARIABLE_JOIN_COUNT_PROPERTY), "5");
    assert_eq!(
        value("var:doubled", rows::VARIABLE_DEFINITION_PROPERTY),
        "BIND(?x * 2 AS ?doubled)"
    );
    assert_eq!(value("region:where/service.0", rows::REGION_KIND_PROPERTY), "SERVICE");
    assert!(value("region:where/service.0", rows::REGION_LABEL_PROPERTY).contains("http://example.org/sparql"));
}

#[test]
fn the_header_selection_states_the_form_and_modifiers() {
    let opened = open("constructs.rq", true);
    let rows = opened.session.properties(opened.workspace.documents(), &[HEADER_ID]);
    let form = rows.iter().find(|r| r.id == rows::QUERY_FORM_PROPERTY).unwrap();
    assert_eq!(form.value, "SELECT DISTINCT");
    let modifiers = rows.iter().find(|r| r.id == rows::QUERY_MODIFIERS_PROPERTY).unwrap();
    assert!(modifiers.value.contains("LIMIT 10"));
    assert_eq!(
        opened.session.select(opened.workspace.documents(), HEADER_ID).unwrap().text,
        "SELECT DISTINCT query"
    );
}

#[test]
fn a_set_arriving_anyway_is_refused_with_the_reason() {
    let mut opened = open("groups.rq", true);
    let original = fs::read(&opened.body).unwrap();
    let refusal = opened
        .session
        .set_property(&mut opened.workspace, &["var:x"], rows::VARIABLE_NAME_PROPERTY, "y")
        .unwrap_err();
    assert_eq!(refusal, READ_ONLY_REASON);
    assert_eq!(fs::read(&opened.body).unwrap(), original);
}

#[test]
fn nothing_is_offered_and_every_action_is_refused() {
    let mut opened = open("groups.rq", true);
    let target = ActionTarget::Elements(vec!["var:x".to_owned()]);
    assert!(opened.session.actions(opened.workspace.documents(), &target).is_empty());
    assert!(opened.session.toolbox(opened.workspace.documents()).is_empty());
    let outcome = opened.session.execute(&mut opened.workspace, &target, "anything");
    assert!(format!("{outcome:?}").contains(rows::NO_ACTIONS_REASON), "{outcome:?}");
}

#[test]
fn a_selection_reads_as_what_it_names() {
    let opened = open("groups.rq", true);
    let text = |id: &str| {
        opened
            .session
            .select(opened.workspace.documents(), id)
            .map(|s| s.text)
    };
    assert_eq!(text("var:x").as_deref(), Some("?x"));
    assert_eq!(text("sub:where.0").as_deref(), Some("Subquery"));
    assert_eq!(text("region:where/optional.0").as_deref(), Some("OPTIONAL"));
    assert_eq!(text("region:where/union.0/branch.0").as_deref(), Some("branch group"));
    assert_eq!(text("note:where/bind.0").as_deref(), Some("BIND(?x * 2 AS ?doubled)"));
    assert_eq!(text("var:nobody"), None);
}

// -- The validator ----------------------------------------------------------------------------------------------------

#[test]
fn a_query_that_does_not_parse_is_one_error_naming_its_line() {
    let problems =
        findings::validate("PREFIX ex: <http://example.org/>\nSELECT ?s\nWHERE {\n  ?s undeclared:p ?o\n}\n");
    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0].severity, DiagramProblemSeverity::Error);
    assert_eq!(problems[0].rule_id, findings::UNPARSEABLE_RULE_ID);
    assert_eq!(problems[0].location, Some(DiagramProblemLocation::Line(4)));
}

#[test]
fn an_update_document_is_reported_through_the_same_door_by_name() {
    let problems = findings::validate("PREFIX ex: <http://example.org/>\nINSERT DATA { ex:s ex:p ex:o }\n");
    assert_eq!(problems.len(), 1);
    assert!(problems[0].message.contains("SPARQL Update"));
}

#[test]
fn a_projected_variable_the_where_clause_never_binds_is_warned() {
    let problems = findings::validate("PREFIX ex: <http://example.org/>\nSELECT ?s ?missing WHERE { ?s ex:p ?o }\n");
    let warned: Vec<_> = problems
        .iter()
        .filter(|p| p.rule_id == findings::UNBOUND_PROJECTION_RULE_ID)
        .collect();
    assert_eq!(warned.len(), 1);
    assert_eq!(warned[0].severity, DiagramProblemSeverity::Warning);
    assert!(warned[0].message.contains("?missing"));
}

#[test]
fn an_unused_prefix_declaration_is_info_at_its_line() {
    let problems = findings::validate(
        "PREFIX ex: <http://example.org/>\nPREFIX unused: <http://example.org/unused/>\nSELECT * WHERE { ?s ex:p ?o }\n",
    );
    let info: Vec<_> = problems
        .iter()
        .filter(|p| p.rule_id == findings::UNUSED_PREFIX_RULE_ID)
        .collect();
    assert_eq!(info.len(), 1);
    assert_eq!(info[0].severity, DiagramProblemSeverity::Info);
    assert!(info[0].message.contains("unused:"));
    assert_eq!(info[0].location, Some(DiagramProblemLocation::Line(2)));
}

#[test]
fn a_clean_query_and_a_service_clause_report_nothing() {
    assert!(findings::validate("PREFIX ex: <http://example.org/>\nSELECT ?s WHERE { ?s ex:p ?o }\nLIMIT 10\n").is_empty());
    assert!(findings::validate(
        "PREFIX ex: <http://example.org/>\nSELECT ?s WHERE { SERVICE <http://example.invalid/sparql> { ?s ex:p ?o } }\n"
    )
    .is_empty());
}

// -- The example corpus -----------------------------------------------------------------------------------------------

#[test]
fn every_example_parses_draws_and_reports_nothing_serious() {
    let all = examples();
    assert_eq!(all.len(), 10);
    for path in all {
        let text = fs::read_to_string(&path).unwrap();
        let model = parser::parse(&text).unwrap_or_else(|e| panic!("{}: {e:?}", path.display()));
        let drawn = projection::project(&model);
        assert!(!drawn.nodes.is_empty(), "{}", path.display());
        assert!(!drawn.truncated());
        assert!(
            findings::judge(&model, &text)
                .iter()
                .all(|p| p.severity == DiagramProblemSeverity::Info),
            "{}",
            path.display()
        );
        // Each has its registration beside it.
        let registration = fs::read_to_string(path.with_extension("adp")).unwrap();
        let lines: Vec<&str> = registration.lines().collect();
        assert_eq!(lines[0], "w3c/sparql");
        assert_eq!(
            lines[1],
            format!("body: {}", path.file_name().unwrap().to_string_lossy())
        );
    }
}

#[test]
fn every_example_opens_through_its_registration() {
    let corpus = fixture("examples/w3c-sparql");
    let folder = tempfile::tempdir().unwrap();
    for entry in fs::read_dir(&corpus).unwrap().flatten() {
        fs::copy(entry.path(), folder.path().join(entry.file_name())).unwrap();
    }
    let mut workspace = DiagramWorkspace::new(folder.path());
    for name in ["union.adp", "optional.adp", "construct.adp"] {
        let (opened, mut session) = workspace.open(&mut tools(), Path::new(name)).unwrap();
        assert_eq!(opened.body_path, folder.path().join(name.replace(".adp", ".rq")));
        assert!(!session.baseline(workspace.documents()).unwrap().is_empty());
    }
}
