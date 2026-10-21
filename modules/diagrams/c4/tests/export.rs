//! ADP's views against the diagrams Structurizr exported for the corpus (etalii.adp spec 030, task T061), ported from
//! the standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/C4Export.Tests.cs` and its helper
//! `C4Export.cs` at `8b6ccfc`: every fixture's committed Mermaid export draws something, every element ADP puts on a
//! view is drawn there, and the helper tells an element's own label from a mention in a relationship.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use adp_diagram_c4::model::C4ElementKind;
use adp_diagram_c4::{parser, rules};

const REGENERATION_HINT: &str = "Regenerate the exports with the Structurizr CLI (`structurizr export -workspace <fixture> -format mermaid -output Fixtures/exports/<fixture>`) and see C4InteropTests.EveryExport_IsStillWhatStructurizrDraws.";

/// The corpus every fixture test runs over (`C4Lines.Tests.cs`, `Corpus`).
const CORPUS: [&str; 12] = [
    "minimal.dsl",
    "comments-everywhere.dsl",
    "crlf-line-endings.dsl",
    "lf-line-endings.dsl",
    "no-trailing-newline.dsl",
    "unusual-indentation.dsl",
    "unmodelled-constructs.dsl",
    "deployment-nested.dsl",
    "dynamic-interactions.dsl",
    "big-bank-plc.dsl",
    "aws-deployment.dsl",
    "crowded-component-view.dsl",
];

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// The committed `.mmd` exports of `fixture`, by file name, or why there are none.
fn read(fixture: &str) -> Result<BTreeMap<String, String>, String> {
    let stem = Path::new(fixture).file_stem().unwrap().to_string_lossy().into_owned();
    let directory = fixtures().join("exports").join(&stem);
    if !directory.is_dir() {
        return Err(format!(
            "No committed exports for '{fixture}'. Every fixture in the corpus needs them. {REGENERATION_HINT}"
        ));
    }
    let mut diagrams = BTreeMap::new();
    for entry in fs::read_dir(&directory).unwrap().flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "mmd") {
            diagrams.insert(
                entry.file_name().to_string_lossy().into_owned(),
                fs::read_to_string(&path).unwrap(),
            );
        }
    }
    if diagrams.is_empty() {
        return Err(format!(
            "'{}' holds no diagrams, so nothing was drawn for '{fixture}'. {REGENERATION_HINT}",
            directory.display()
        ));
    }
    Ok(diagrams)
}

/// Whether `diagram` draws `element`: its own bold label, or a boundary titled with it.
fn draws(diagram: &str, element: &str) -> bool {
    if diagram.contains(&format!("font-weight: bold'>{element}</div>")) {
        return true;
    }
    let title = format!("[\"{element}\"]");
    diagram
        .split('\n')
        .map(str::trim)
        .any(|line| line.starts_with("subgraph ") && line.ends_with(&title))
}

#[test]
fn every_fixture_draws_something() {
    for fixture in CORPUS {
        let diagrams = read(fixture).unwrap();
        assert!(!diagrams.is_empty(), "'{fixture}' yielded no diagrams to check");
        for (name, diagram) in &diagrams {
            assert!(
                diagram.contains("font-weight: bold"),
                "'{fixture}' exported '{name}' with no elements drawn in it at all. {REGENERATION_HINT}"
            );
        }
    }
}

#[test]
fn every_element_adp_puts_on_a_view_was_drawn() {
    for fixture in CORPUS {
        let workspace = parser::parse_text(&fs::read_to_string(fixtures().join(fixture)).unwrap());
        let everything: String = read(fixture).unwrap().into_values().collect();
        let mut undrawn: Vec<String> = workspace
            .views
            .iter()
            .flat_map(|view| rules::members_of(&workspace, view))
            .filter(|e| {
                !matches!(
                    e.kind,
                    C4ElementKind::ContainerInstance | C4ElementKind::SoftwareSystemInstance
                )
            })
            .map(|e| e.name.clone())
            .filter(|name| !name.is_empty() && !draws(&everything, name))
            .collect();
        undrawn.sort();
        undrawn.dedup();
        assert!(
            undrawn.is_empty(),
            "ADP puts {undrawn:?} on a view of '{fixture}', and Structurizr drew none of them. {REGENERATION_HINT}"
        );
    }
}

#[test]
fn an_empty_diagram_is_recognised_as_empty() {
    let empty = "graph LR\n  linkStyle default fill:#ffffff\n\n  subgraph diagram [\"Deployment View: Live\"]\n    style diagram fill:#ffffff,stroke:#ffffff\n\n  end";
    assert!(!empty.contains("font-weight: bold"));
    assert!(!draws(empty, "Web Application"));
}

#[test]
fn an_element_named_only_in_a_relationship_does_not_count_as_drawn() {
    let diagram = "graph LR\n  1[\"<div style='font-weight: bold'>Web Application</div><div style='font-size: 70%; margin-top: 0px'>[Container]</div>\"]\n  1-. \"<div>Reads from the Database</div>\" .->1";
    assert!(draws(diagram, "Web Application"));
    assert!(!draws(diagram, "Database"));
}

#[test]
fn a_fixture_with_no_exports_fails_rather_than_passing_quietly() {
    let problem = read("no-such-fixture.dsl").unwrap_err();
    assert!(problem.contains("no-such-fixture"));
    assert!(problem.contains("structurizr export"));
}
