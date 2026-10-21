//! Reading an activity file (agent-activity-diagram Requirements 2.1 to 2.5, 2.8, 2.9): every field the file holds
//! reaches the model, and a file nobody edited is written back as it was. Ported from the standalone host's
//! `AadDocument.Tests.cs`, `AadDocumentStore.Tests.cs`, `AadExamples.Tests.cs`, `BundledDefinition.Tests.cs` and
//! `ServiceCollectionAddAgentActivityDiagram.Tests.cs` at `8b6ccfc`.

mod common;

use std::collections::BTreeSet;
use std::fs;

use adp_diagram::DiagramProblemSeverity;
use adp_diagram_agent_activity::body::{AadBody, empty_document};
use adp_diagram_agent_activity::commands::AadEdit;
use adp_diagram_agent_activity::definition::{self, PULL_REQUESTS_GROUP};
use adp_diagram_agent_activity::model::{AadKind, AadModel, AadPlacement, AadRow};
use adp_diagram_agent_activity::{TEMPLATE, validator};
use common::{fixture, fixtures, one_day, open, open_one_day, read};

fn one_day_model() -> AadModel {
    read(&one_day()).model.as_ref().clone()
}

#[test]
fn a_file_nobody_edited_is_written_back_byte_for_byte() {
    let text = one_day();
    let document = read(&text);
    assert!(document.is_usable(), "{:?}", document.unreadable);
    assert_eq!(text, document.body.text());
}

#[test]
fn every_element_is_read_with_its_fields() {
    let model = one_day_model();
    assert_eq!(
        vec![
            "Project p-standalone",
            "Project p-adp",
            "Specification s-knowledge",
            "Specification s-rider",
            "Agent a-dev2",
            "Location l-kd",
            "Environment e-fractal"
        ],
        model
            .elements
            .iter()
            .map(|e| format!("{:?} {}", e.kind, e.id))
            .collect::<Vec<_>>()
    );
    let specification = model.element("s-knowledge").unwrap();
    assert_eq!("Knowledge designer", specification.name);
    assert_eq!("progressing", specification.status);
    assert_eq!("Progressing", specification.status_label);
    assert_eq!(
        ".spec-workflow/specs/knowledge-designer/requirements.md",
        specification.link
    );

    let location = model.element("l-kd").unwrap();
    assert_eq!("features/knowledge-designer", location.name);
    assert_eq!(".claude/worktrees/kd", location.folder);
    assert_eq!(
        "C:/git/etalii.adp.ide.standalone/.claude/worktrees/kd",
        location.folder_link
    );
    assert!(location.branch_link.starts_with("https://github.com/"));

    assert_eq!("Local machine", model.element("e-fractal").unwrap().kind_label);
    assert_eq!("archived", model.element("s-rider").unwrap().status);
}

#[test]
fn tasks_and_pull_requests_are_rows_of_their_element_not_elements() {
    let model = one_day_model();
    let tasks = &model.element("s-knowledge").unwrap().rows;
    assert_eq!(
        vec!["t-k2", "t-k6"],
        tasks.iter().map(|r| r.id.as_str()).collect::<Vec<_>>()
    );
    assert_eq!(
        AadRow {
            id: "t-k6".into(),
            title: "Table component".into(),
            status: "inputRequired".into(),
            updated: "2026-10-08T21:10:00+02:00".into(),
            link: "https://github.com/etalii-adp/etalii.adp.ide.standalone/pull/150".into(),
        },
        tasks[1]
    );
    let pull_requests = &model.element("l-kd").unwrap().rows;
    assert_eq!(1, pull_requests.len());
    assert_eq!("150: Knowledge designer, tasks 2 and 6", pull_requests[0].title);
    assert_eq!("", pull_requests[0].status);
    assert!(model.element("t-k2").is_none() && model.element("pr-150").is_none());
}

#[test]
fn a_relation_is_a_key_on_the_element_at_its_one_end() {
    let model = one_day_model();
    assert_eq!(
        vec![
            "ProjectSpecification p-standalone > s-knowledge",
            "ProjectSpecification p-standalone > s-rider",
            "SpecificationAgent s-knowledge > a-dev2",
            "AgentLocation a-dev2 > l-kd",
            "LocationEnvironment l-kd > e-fractal",
        ],
        model
            .relations
            .iter()
            .map(|r| format!("{} {} > {}", r.type_name, r.from, r.to))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        "specification:a-dev2",
        model
            .relations
            .iter()
            .find(|r| r.type_name == "SpecificationAgent")
            .unwrap()
            .id
    );
}

#[test]
fn the_readers_own_part_is_read_beside_the_model() {
    let model = one_day_model();
    assert!(!model.show_archived);
    assert_eq!(1, model.placements.len());
    assert_eq!(
        Some(&AadPlacement { x: 240.0, y: -80.0 }),
        model.placements.get("s-knowledge")
    );
    // The file unfolds Pending for this specification; Finished keeps its default, folded.
    assert_eq!(BTreeSet::from(["finished".to_owned()]), model.collapsed["s-knowledge"]);
    // A specification nobody touched has the defaults.
    assert_eq!(
        BTreeSet::from(["finished".to_owned(), "pending".to_owned()]),
        model.collapsed["s-rider"]
    );
    assert_eq!(
        BTreeSet::from([PULL_REQUESTS_GROUP.to_owned()]),
        model.collapsed["l-kd"]
    );
}

#[test]
fn a_folder_left_out_is_the_main_checkout() {
    let document = read("agent-activity-diagram: 1\r\nlocations:\r\n  - id: l1\r\n    branch: develop\r\n");
    assert_eq!("Default", document.model.elements[0].folder);
}

#[test]
fn a_new_file_is_the_header_alone_and_reads_empty_without_a_finding() {
    let text = empty_document("\r\n");
    let document = read(&text);
    assert_eq!("agent-activity-diagram: 1\r\n", text);
    assert_eq!(TEMPLATE, text);
    assert!(document.is_usable(), "{:?}", document.unreadable);
    assert!(document.model.elements.is_empty());
    assert!(
        document.body.model().findings.is_empty(),
        "{:?}",
        document.body.model().findings
    );
}

#[test]
fn a_file_that_is_not_an_activity_file_is_held_unreadable_with_the_readers_reason() {
    let document = read("projects: [unterminated\r\n");
    assert!(!document.is_usable());
    assert!(!document.unreadable.clone().unwrap().is_empty());
    assert!(document.model.elements.is_empty());
}

// ---- the store (`AadDocumentStore.Tests.cs`) ----

#[test]
fn a_file_that_is_not_an_activity_file_is_held_unreadable_and_an_edit_is_refused_unwritten() {
    let mut diagram = open("projects: [unterminated\r\n");
    assert!(!diagram.document().is_usable());
    let edit = diagram.run(AadEdit::SetShowArchived { show: true });
    assert!(edit.is_err());
    assert_eq!("projects: [unterminated\r\n", diagram.text());
}

#[test]
fn an_edit_is_applied_to_what_the_file_holds_now_when_it_was_written_since_it_was_read() {
    let mut diagram = open_one_day();
    let written = format!("{}# an agent's line\r\n", one_day());
    fs::write(&diagram.body, &written).unwrap();
    diagram
        .run(AadEdit::SetAttribute {
            element_id: "p-standalone".into(),
            attribute: "name".into(),
            value: "Store contract marker".into(),
        })
        .unwrap();
    let text = diagram.text();
    assert!(text.contains("# an agent's line"), "{text}");
    assert!(text.contains("Store contract marker"), "{text}");
}

#[test]
fn a_refused_edit_writes_nothing() {
    let mut diagram = open_one_day();
    let refused = diagram.run(AadEdit::Unpin {
        element_id: Some("l-kd".into()),
    });
    assert_eq!(Err("Its position is not locked.".to_owned()), refused);
    assert_eq!(one_day(), diagram.text());
    assert!(!diagram.workspace.history().availability().can_undo);
}

#[test]
fn a_change_on_disk_reaches_the_store_and_a_file_that_can_no_longer_be_read_keeps_showing_what_was_last_read() {
    let mut diagram = open_one_day();
    let renamed = one_day().replace("    name: Developer 2\r\n", "    name: Developer two\r\n");
    diagram.agent_writes(&renamed);
    assert_eq!(
        "Developer two",
        diagram.document().model.element("a-dev2").unwrap().name
    );

    let before = diagram.document().model.elements.len();
    diagram.agent_writes("agent-activity-diagram: 1\r\nprojects: [\r\n");
    assert!(!diagram.document().is_usable());
    assert_eq!(before, diagram.document().model.elements.len());
    let elements = diagram.session.render(diagram.workspace.documents()).unwrap();
    assert!(elements.iter().any(|e| e.id == "a-dev2"));
}

// ---- the examples (`AadExamples.Tests.cs`) ----

const EXAMPLES: [&str; 3] = ["adp-one-day", "every-state", "two-projects"];

fn example(name: &str) -> String {
    fixture(&format!("examples/{name}/{name}.aad"))
}

#[test]
fn the_module_ships_its_three_examples_each_with_a_registration() {
    for name in EXAMPLES {
        assert!(
            fixtures().join(format!("examples/{name}/{name}.adp")).is_file(),
            "{name}"
        );
        assert!(
            fs::read_to_string(fixtures().join(format!("examples/{name}/{name}.adp")))
                .unwrap()
                .starts_with("etalii/agent-activity-diagram")
        );
    }
}

#[test]
fn an_example_opens_without_an_error_and_is_written_back_as_it_was() {
    for name in EXAMPLES {
        let text = example(name);
        let document = read(&text);
        let problems = validator::validate(&document.body);
        assert!(document.is_usable(), "{name}: {:?}", document.unreadable);
        assert!(
            problems.iter().all(|p| p.severity == DiagramProblemSeverity::Info),
            "{name}: {problems:?}"
        );
        assert_eq!(text, document.body.text(), "{name}");
        assert!(!document.model.elements.is_empty(), "{name}");
    }
}

#[test]
fn between_them_the_examples_show_everything_the_notation_has() {
    let definition = definition::get();
    let models: Vec<AadModel> = EXAMPLES
        .iter()
        .map(|name| read(&example(name)).model.as_ref().clone())
        .collect();
    let elements: Vec<_> = models.iter().flat_map(|m| &m.elements).collect();
    let mut missing = Vec::new();
    let mut shows = |shown: bool, what: String| {
        if !shown {
            missing.push(what);
        }
    };
    for status in &definition.specification_status().members {
        shows(
            elements
                .iter()
                .any(|e| e.kind == AadKind::Specification && e.status == status.name),
            format!("a specification that is {}", status.label),
        );
    }
    for status in &definition.task_status().members {
        shows(
            elements
                .iter()
                .filter(|e| e.kind == AadKind::Specification)
                .flat_map(|e| &e.rows)
                .any(|r| r.status == status.name),
            format!("a task that is {}", status.label),
        );
    }
    for kind in &definition.environment_kind().members {
        shows(
            elements
                .iter()
                .any(|e| e.kind == AadKind::Environment && e.kind_label == kind.label),
            format!("an environment of kind {}", kind.label),
        );
    }
    let several = |type_name: &str, by_from: bool| {
        models.iter().any(|m| {
            let ends: Vec<&str> = m
                .relations
                .iter()
                .filter(|r| r.type_name == type_name)
                .map(|r| if by_from { r.from.as_str() } else { r.to.as_str() })
                .collect();
            ends.iter().any(|end| ends.iter().filter(|e| *e == end).count() > 1)
        })
    };
    shows(
        several("SpecificationAgent", true),
        "a specification with several agents".into(),
    );
    shows(several("AgentLocation", true), "an agent with several locations".into());
    shows(
        several("LocationEnvironment", false),
        "an environment shared by several locations".into(),
    );
    shows(
        elements
            .iter()
            .any(|e| e.kind == AadKind::Location && e.folder == "Default"),
        "a location in the folder Default".into(),
    );
    shows(
        elements
            .iter()
            .any(|e| e.kind == AadKind::Location && e.folder != "Default"),
        "a location in a worktree".into(),
    );
    shows(
        elements
            .iter()
            .any(|e| e.kind == AadKind::Location && !e.rows.is_empty()),
        "a pull request".into(),
    );
    shows(
        models
            .iter()
            .any(|m| !m.placements.is_empty() && m.placements.len() < m.elements.len()),
        "locked beside unlocked elements".into(),
    );
    shows(
        models.iter().any(|m| {
            m.collapsed
                .values()
                .any(|groups| groups.iter().any(|g| !definition.collapsed_by_default().contains(g)))
        }),
        "a group folded that comes unfolded".into(),
    );
    shows(
        models
            .iter()
            .any(|m| m.elements.iter().filter(|e| e.kind == AadKind::Project).count() > 1),
        "more than one project".into(),
    );
    assert!(missing.is_empty(), "No example shows: {}", missing.join("; "));
}

// ---- the bundled definition (`BundledDefinition.Tests.cs`) ----

#[test]
fn the_bundled_definition_is_this_modules_language() {
    let definition = definition::get();
    let language = &definition.specification().root()["language"];
    assert_eq!("net.etalii.adp.etalii.agent-activity-diagram", language["id"]);
    assert_eq!(adp_diagram_agent_activity::ORIGIN, language["origin"]);
    assert_eq!(
        adp_diagram_agent_activity::EXTENSION,
        format!(".{}", language["fileExtension"].as_str().unwrap())
    );
}

#[test]
fn the_bundled_definition_loads_in_this_host_and_says_what_the_module_reads_off_it() {
    let definition = definition::get();
    assert_eq!(
        vec!["progressing", "pending", "inputRequired", "finished"],
        definition
            .task_status()
            .members
            .iter()
            .map(|m| m.name.as_str())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        "input-required",
        definition.task_status().find("inputRequired").unwrap().stored
    );
    assert_eq!(
        "Input Required",
        definition.specification_status().find("input-required").unwrap().label
    );
    assert_eq!(
        "Local machine",
        definition.environment_kind().find("local-machine").unwrap().label
    );
    // Progressing and Input Required start unfolded; Pending, Finished and a location's pull requests start folded.
    assert_eq!(
        vec!["finished", "pending", PULL_REQUESTS_GROUP],
        definition
            .collapsed_by_default()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
    );
}

#[test]
fn a_body_reads_the_version_its_header_declares() {
    assert_eq!(1, AadBody::parse(&one_day()).version());
    assert_eq!(2, AadBody::parse(&fixture("findings/newer-version.aad")).version());
}
