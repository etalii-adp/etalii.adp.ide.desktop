//! A registered chart folder opened in a project, and what is asked of it (etalii.adp spec 030, task T080): the
//! standalone host's `HelmSession.Tests.cs`, `HelmContextSourceResolver.Tests.cs`, `HelmContextPropertyProvider.Tests.cs`,
//! `HelmValidator.Tests.cs` and `ZeroWrites.Tests.cs` at `8b6ccfc`, with the same expectations. The standalone host's
//! sessions heard the folder through a watcher; here the project's reconciliation hands a changed path to the module.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use adp_diagram::{DiagramProblemLocation, DiagramViewport, DiskChange};
use adp_diagram_helm_chart::view::{EDGE_REFUSAL, REPARENT_REFUSAL};
use adp_diagram_helm_chart::{mapper, registration_layout};
use common::{Opened, copy_tree, fixture, open, open_registered, registered};

fn snapshot(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(folder) = pending.pop() {
        for entry in fs::read_dir(folder).unwrap().flatten() {
            if entry.path().is_dir() {
                pending.push(entry.path());
            } else {
                files.push((entry.path(), fs::read(entry.path()).unwrap()));
            }
        }
    }
    files.sort();
    files
}

fn stored(opened: &Opened) -> std::collections::BTreeMap<String, (f64, f64)> {
    registration_layout::read(&fs::read_to_string(&opened.registration).unwrap())
}

// ---- the subject is the folder -------------------------------------------------------------------------------------

#[test]
fn a_charts_own_registration_named_dot_adp_opens_the_folder_around_it() {
    let mut opened = open("examples/nginx");
    assert!(opened.registration.ends_with(".adp"));
    let elements = opened.delivered();
    assert_eq!(elements.len(), 125);
    assert!(opened.session.module_view().is_some());
    // The registration's stored positions win over the computed ones; the rest are the bands'.
    let chart = elements.iter().find(|e| e.id == "chart").unwrap();
    assert_eq!((chart.x, chart.y, chart.placed), (1327.665, 566.461, true));
    let values = elements.iter().find(|e| e.id == "values:values.yaml").unwrap();
    assert_eq!((values.x, values.y, values.placed), (320.0, 40.0, false));
    assert_eq!(values.element_type, mapper::VALUES_TYPE);
}

#[test]
fn every_node_kind_travels_with_its_own_wire_type() {
    let mut opened = open("well-formed");
    let elements = opened.delivered();
    let type_of = |id: &str| {
        elements
            .iter()
            .find(|e| e.id == id)
            .unwrap_or_else(|| panic!("no {id}"))
            .element_type
            .clone()
    };
    assert_eq!(type_of("chart"), mapper::CHART_TYPE);
    assert_eq!(type_of("schema:values.schema.json"), mapper::SCHEMA_TYPE);
    assert_eq!(type_of("tpl:templates/_helpers.tpl"), mapper::PARTIAL_TYPE);
    assert_eq!(type_of("tpl:templates/deployment.yaml"), mapper::TEMPLATE_TYPE);
    assert_eq!(type_of("crds"), mapper::CRDS_TYPE);
    assert_eq!(type_of("sub:charts/redis"), mapper::SUBCHART_TYPE);
    assert_eq!(type_of("lock:Chart.lock"), mapper::LOCK_TYPE);
    assert!(elements.iter().any(|e| e.element_type == mapper::DEPENDENCY_TYPE));
    assert!(elements.iter().any(|e| e.element_type == mapper::EDGE_TYPE));
}

#[test]
fn a_narrow_viewport_delivers_what_it_touches_and_one_hop() {
    let opened = open("well-formed");
    let documents = opened.workspace.documents();
    let all = opened.session.render(documents).unwrap();
    // Only the chart's box (40, 40, 220 x 110) is in view: it comes with what it declares, nothing further.
    let view = opened.session.module_view().unwrap();
    let narrowed = view
        .render(documents, DiagramViewport::new(50.0, 50.0, 60.0, 60.0))
        .unwrap();
    assert!(narrowed.len() < all.len());
    assert!(narrowed.iter().any(|e| e.id == "chart"));
    assert!(narrowed.iter().any(|e| e.element_type == mapper::DEPENDENCY_TYPE));
    assert!(!narrowed.iter().any(|e| e.id == "tpl:templates/deployment.yaml"));
}

#[test]
fn a_folder_that_is_no_chart_draws_nothing_and_says_so_once() {
    let opened = open("not-a-chart");
    assert!(opened.session.render(opened.workspace.documents()).unwrap().is_empty());
    let problems = opened.session.problems(opened.workspace.documents());
    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0].rule_id, "helm.not-a-chart");
    assert_eq!(
        problems[0].message,
        "This folder is not a Helm chart: it has no Chart.yaml."
    );
}

#[test]
fn a_chart_below_the_project_root_reports_its_findings_from_the_root() {
    let project = tempfile::tempdir().unwrap();
    let chart = project.path().join("deploy").join("chart");
    copy_tree(&fixture("broken"), &chart);
    let registration = chart.join("chart.adp");
    fs::write(&registration, "helm/chart\r\n").unwrap();
    let opened = open_registered(project, registration);
    let problems = opened.session.problems(opened.workspace.documents());
    let at: Vec<String> = problems
        .iter()
        .map(|p| match &p.location {
            Some(DiagramProblemLocation::File { relative_path, line }) => format!("{relative_path}:{line}"),
            other => format!("{other:?}"),
        })
        .collect();
    assert_eq!(at, ["deploy/chart/values.yaml:4", "deploy/chart/Chart.yaml:1"]);

    let other = tempfile::tempdir().unwrap();
    let folder = other.path().join("nested");
    copy_tree(&fixture("not-a-chart"), &folder);
    let registration = folder.join("nested.adp");
    fs::write(&registration, "helm/chart\r\n").unwrap();
    let opened = open_registered(other, registration);
    match &opened.session.problems(opened.workspace.documents())[0].location {
        Some(DiagramProblemLocation::File { relative_path, .. }) => assert_eq!(relative_path, "nested"),
        other => panic!("{other:?}"),
    }
}

// ---- following the folder ------------------------------------------------------------------------------------------

#[test]
fn a_change_on_disk_reaches_the_view() {
    let mut opened = open("well-formed");
    opened.delivered();
    let layer = opened.root.join("values-production.yaml");
    fs::write(&layer, "replicaCount: 5\n").unwrap();
    assert_eq!(
        opened.workspace.reconcile(&layer),
        DiskChange::Reloaded(opened.registration.clone())
    );
    let rendered = opened.session.render(opened.workspace.documents()).unwrap();
    assert!(rendered.iter().any(|e| e.id == "values:values-production.yaml"));
    // A file nothing reads changes nothing.
    let note = opened.root.join("notes-for-humans.md");
    fs::write(&note, "changed").unwrap();
    assert_eq!(opened.workspace.reconcile(&note), DiskChange::Unchanged);
}

// ---- what it refuses -----------------------------------------------------------------------------------------------

#[test]
fn a_reparenting_move_is_refused_and_changes_nothing_on_disk() {
    let mut opened = open("well-formed");
    let before = snapshot(&opened.root);
    let reason = opened
        .session
        .move_element(&mut opened.workspace, "values:values.yaml", Some("chart"), 0)
        .unwrap_err();
    assert_eq!(reason, REPARENT_REFUSAL);
    assert_eq!(before, snapshot(&opened.root));
}

#[test]
fn a_property_write_is_refused_with_the_definitions_sentence() {
    let mut opened = open("well-formed");
    let reason = opened
        .session
        .set_property(&mut opened.workspace, &["chart"], "helm.name", "renamed")
        .unwrap_err();
    assert_eq!(
        reason,
        "A helm chart diagram shows the folder as it is and changes nothing in it. Edit the file this value comes from in a text editor."
    );
}

#[test]
fn the_palette_is_empty_and_no_action_is_offered() {
    let opened = open("well-formed");
    assert!(opened.session.toolbox(opened.workspace.documents()).is_empty());
}

// ---- the one edit: a reposition ------------------------------------------------------------------------------------

#[test]
fn a_move_writes_the_authored_position_into_the_registration_and_is_undone_byte_for_byte() {
    let mut opened = open("well-formed");
    let before = fs::read(&opened.registration).unwrap();
    opened
        .session
        .move_element_to(&mut opened.workspace, "tpl:templates/service.yaml", 321.5, -12.0)
        .unwrap();
    assert_eq!(
        fs::read_to_string(&opened.registration).unwrap(),
        "helm/chart\r\nlayout:\r\n  tpl:templates/service.yaml: 321.5 -12\r\n"
    );
    let moved = opened
        .session
        .render(opened.workspace.documents())
        .unwrap()
        .into_iter()
        .find(|e| e.id == "tpl:templates/service.yaml")
        .unwrap();
    assert_eq!((moved.x, moved.y, moved.placed), (321.5, -12.0, true));
    assert!(opened.workspace.undo().is_done());
    assert_eq!(before, fs::read(&opened.registration).unwrap());
    assert!(opened.workspace.redo().is_done());
    assert_eq!(stored(&opened)["tpl:templates/service.yaml"], (321.5, -12.0));
}

#[test]
fn a_move_of_an_edge_is_refused_because_an_edge_follows_its_endpoints() {
    let mut opened = open("well-formed");
    let edge = opened
        .delivered()
        .into_iter()
        .find(|e| e.element_type == mapper::EDGE_TYPE)
        .unwrap();
    let reason = opened
        .session
        .move_element_to(&mut opened.workspace, &edge.id, 1.0, 2.0)
        .unwrap_err();
    assert_eq!(reason, EDGE_REFUSAL);
    assert!(stored(&opened).is_empty());
}

#[test]
fn nothing_but_a_move_writes_and_a_move_writes_only_the_registration() {
    let (folder, registration) = registered("well-formed");
    let mut opened = open_registered(folder, registration);
    let before = snapshot(&opened.root);
    opened.delivered();
    for id in ["chart", "values:values.yaml", "dep:cache", ""] {
        opened.session.properties(opened.workspace.documents(), &[id]);
    }
    opened.session.problems(opened.workspace.documents());
    assert_eq!(before, snapshot(&opened.root));
    opened
        .session
        .move_elements(
            &mut opened.workspace,
            &[("chart".into(), 1.0, 2.0), ("crds".into(), 3.0, 4.0)],
        )
        .unwrap();
    let after = snapshot(&opened.root);
    let changed: Vec<&PathBuf> = after
        .iter()
        .filter(|(path, bytes)| !before.iter().any(|(p, b)| p == path && b == bytes))
        .map(|(path, _)| path)
        .collect();
    assert_eq!(changed, [&opened.registration]);
    // Two boxes moved at once are one undo.
    assert!(opened.workspace.undo().is_done());
    assert_eq!(before, snapshot(&opened.root));
}

// ---- what a selection reads as -------------------------------------------------------------------------------------

#[test]
fn a_node_resolves_to_its_own_path_and_a_dependency_to_its_declaring_file() {
    let opened = open("well-formed");
    let documents = opened.workspace.documents();
    let selected = opened
        .session
        .select(documents, "tpl:templates/deployment.yaml")
        .unwrap();
    assert_eq!(selected.path, ["templates", "deployment.yaml"]);
    assert_eq!(selected.text, "deployment.yaml");
    let dependency = opened.session.select(documents, "dep:cache").unwrap();
    assert_eq!(dependency.path, ["Chart.yaml"]);
    assert_eq!(dependency.text, "cache");
    assert!(opened.session.select(documents, "tpl:templates/gone.yaml").is_none());
}

#[test]
fn an_edge_resolves_as_its_declaring_side_and_what_it_does() {
    let opened = open("well-formed");
    let documents = opened.workspace.documents();
    let edges: Vec<_> = opened
        .session
        .render(documents)
        .unwrap()
        .into_iter()
        .filter(|e| e.element_type == mapper::EDGE_TYPE)
        .collect();
    let declares = edges.iter().find(|e| e.attribute("edge.kind") == "DECLARES").unwrap();
    let selected = opened.session.select(documents, &declares.id).unwrap();
    assert_eq!(selected.path, ["Chart.yaml"]);
    assert!(selected.text.starts_with("declares "), "{}", selected.text);
    let overrides = edges.iter().find(|e| e.attribute("edge.kind") == "OVERRIDES").unwrap();
    let selected = opened.session.select(documents, &overrides.id).unwrap();
    assert_eq!(selected.path, ["values-staging.yaml"]);
    assert_eq!(selected.text, "overrides");
}

#[test]
fn a_legacy_charts_dependency_resolves_to_requirements_yaml() {
    let opened = open("unconventional");
    let documents = opened.workspace.documents();
    let dependency = opened
        .session
        .render(documents)
        .unwrap()
        .into_iter()
        .find(|e| e.element_type == mapper::DEPENDENCY_TYPE)
        .unwrap();
    assert_eq!(
        opened.session.select(documents, &dependency.id).unwrap().path,
        ["requirements.yaml"]
    );
}
