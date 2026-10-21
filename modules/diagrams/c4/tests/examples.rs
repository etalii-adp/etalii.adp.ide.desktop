//! The standalone host's two C4 example projects, checked against the implementation they are examples of (etalii.adp
//! spec 030, task T061): every model parses and comes back byte for byte, every registration opens a view that exists,
//! nothing in either breaks a C4 rule, and every view lays out without overlaps.
//!
//! Ported from the standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/Examples.Tests.cs` (with
//! `C4ExamplesStubCatalog.cs`) at `8b6ccfc`, over the examples of `src/diagrams/c4/examples/`, copied unchanged to
//! `tests/fixtures/examples/`. A registration resolves, and a project validates, through the session the desktop host
//! opens it in. Not ported: `ExactlyOneRegistrationPerProject_OwnsTheModelDocument`, because which registration owns
//! a body (what a delete or a rename in the explorer carries off with it) is the explorer's, not this module's.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use adp_core::Catalogue;
use adp_diagram::{DiagramTools, DiagramWorkspace};
use adp_diagram_c4::layout;
use adp_diagram_c4::lines::LineDocument;
use adp_diagram_c4::metrics::C4Metrics;
use adp_diagram_c4::model::{C4ViewKind, C4Workspace};
use adp_diagram_c4::{parser, rules, toolbox};
use adp_disl::DislPluginFunctions;

const EVERY_PROJECT: [&str; 2] = ["reference", "industrial-plant"];

fn examples_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/examples")
}

fn files_of(folder: &Path, extension: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut folders = vec![folder.to_path_buf()];
    while let Some(folder) = folders.pop() {
        for entry in fs::read_dir(&folder).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                folders.push(path);
            } else if path.extension().is_some_and(|e| e == extension) {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

fn models_of(project: &str) -> Vec<PathBuf> {
    files_of(&examples_root().join(project), "dsl")
}

fn registrations_of(project: &str) -> Vec<PathBuf> {
    files_of(&examples_root().join(project), "adp")
}

fn read(path: &Path) -> C4Workspace {
    parser::parse_text(&fs::read_to_string(path).unwrap())
}

fn first_line(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .next()
        .unwrap_or_default()
        .to_owned()
}

fn tools() -> DiagramTools {
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
    adp_diagram_c4::register(DiagramTools::new(Catalogue::load(&vendor), |_| {
        DislPluginFunctions::none()
    }))
}

#[test]
fn every_project_has_exactly_one_model_document() {
    for project in EVERY_PROJECT {
        assert_eq!(models_of(project).len(), 1, "{project}");
    }
}

#[test]
fn every_model_parses_and_round_trips_byte_for_byte() {
    for project in EVERY_PROJECT {
        let text = fs::read_to_string(&models_of(project)[0]).unwrap();
        let document = LineDocument::parse(&text);
        assert_eq!(document.text(), text, "{project}");
        assert!(!parser::parse(&document).elements.is_empty(), "{project}");
    }
}

#[test]
fn every_registration_names_a_view_that_exists() {
    for project in EVERY_PROJECT {
        let workspace = read(&models_of(project)[0]);
        let registrations = registrations_of(project);
        assert!(!registrations.is_empty(), "{project}");
        for registration in registrations {
            let text = fs::read_to_string(&registration).unwrap();
            let lines: Vec<&str> = text.lines().collect();
            assert!(lines[0].starts_with("c4/"), "{}", registration.display());
            let Some(view) = lines.iter().find(|line| line.starts_with("view:")) else {
                // Only the type that keeps no document has no view: its registration is the whole diagram.
                assert_eq!(lines[0], "c4/code");
                continue;
            };
            let key = view["view:".len()..].trim();
            assert!(
                workspace.find_view(key).is_some(),
                "{} names view '{key}', which {project} does not have.",
                registration.display()
            );
        }
    }
}

#[test]
fn every_registration_resolves_through_the_same_code_the_backend_uses() {
    let mut tools = tools();
    for project in EVERY_PROJECT {
        let root = examples_root().join(project);
        let mut workspace = DiagramWorkspace::new(&root);
        for registration in registrations_of(project) {
            let opened = workspace.open(&mut tools, &registration);
            if first_line(&registration) == "c4/code" {
                // The one type that keeps no document opens nothing.
                assert_eq!(opened.err().as_deref(), Some(toolbox::CODE_UNAVAILABLE));
                continue;
            }
            let (opened, _) = opened.unwrap_or_else(|e| panic!("{}: {e}", registration.display()));
            assert!(opened.body_path.is_file(), "{}", registration.display());
        }
    }
}

#[test]
fn every_model_is_clean_by_c4s_rules() {
    for project in EVERY_PROJECT {
        let problems: Vec<String> = rules::validate(&read(&models_of(project)[0]))
            .into_iter()
            .map(|p| format!("{}: {}", p.rule_id, p.message))
            .collect();
        assert!(problems.is_empty(), "{project}: {problems:#?}");
    }
}

#[test]
fn every_view_in_every_model_lays_out_without_overlaps() {
    for project in EVERY_PROJECT {
        let workspace = read(&models_of(project)[0]);
        for view in &workspace.views {
            let boxes: Vec<_> = layout::compute(&workspace, view, &C4Metrics::default(), None)
                .boxes
                .iter()
                .map(|(id, rect)| (id.to_owned(), rect))
                .collect();
            for i in 0..boxes.len() {
                for j in i + 1..boxes.len() {
                    assert!(
                        !boxes[i].1.overlaps(&boxes[j].1),
                        "{project}/{}: '{}' overlaps '{}'",
                        view.key,
                        boxes[i].0,
                        boxes[j].0
                    );
                }
            }
        }
    }
}

#[test]
fn the_reference_project_shows_every_supported_view_kind() {
    let kinds: HashSet<C4ViewKind> = read(&models_of("reference")[0]).views.iter().map(|v| v.kind).collect();
    let every: HashSet<C4ViewKind> = toolbox::TYPES.iter().filter_map(|(_, kind)| *kind).collect();
    assert_eq!(kinds, every);
}

#[test]
fn the_reference_project_registers_every_supported_diagram_type() {
    let mut declared: Vec<String> = registrations_of("reference").iter().map(|p| first_line(p)).collect();
    declared.sort();
    let mut types: Vec<String> = toolbox::TYPES.iter().map(|(origin, _)| (*origin).to_owned()).collect();
    types.sort();
    assert_eq!(declared, types);
}

#[test]
fn validating_a_project_succeeds_and_reports_nothing() {
    let mut tools = tools();
    for project in EVERY_PROJECT {
        let root = examples_root().join(project);
        let mut workspace = DiagramWorkspace::new(&root);
        let mut bodies = HashSet::new();
        for registration in registrations_of(project) {
            if first_line(&registration) == "c4/code" {
                continue;
            }
            let (opened, session) = workspace.open(&mut tools, &registration).unwrap();
            let problems: Vec<String> = session
                .problems(workspace.documents())
                .into_iter()
                .map(|p| p.message)
                .collect();
            assert!(problems.is_empty(), "{}: {problems:#?}", registration.display());
            bodies.insert(opened.body_path);
        }
        // One judgement per document, however many registrations point at it.
        assert_eq!(bodies.len(), 1, "{project}");
    }
}
