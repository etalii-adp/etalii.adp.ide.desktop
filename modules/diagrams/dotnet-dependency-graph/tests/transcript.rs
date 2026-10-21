//! The standalone host's parity transcript of the module (`Parity/dotnet.transcript.json` at `8b6ccfc`, written by
//! `DotNetTranscript.cs`), replayed here (etalii.adp spec 030, task T082): for every solution of its corpus, the
//! derived graph (projects, packages, references, failures), every element's computed position and one with a stored
//! position, the property rows of every element id, the empty id and an unknown one, and the answer to a write; for
//! each graph it built by hand, the positions and the rows. The payloads are compared by id, type and position: the
//! desktop host carries a payload as an element's attributes, which the session tests check.

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use adp_diagram_dotnet_dependency_graph::definition::DotNetDefinition;
use adp_diagram_dotnet_dependency_graph::descriptions::PackageDescriptionReader;
use adp_diagram_dotnet_dependency_graph::graph::{
    self, DependencyGraphModel, DependsOnEdge, DependsOnKind, PackageNode, ProjectNode, SolutionFailure,
};
use adp_diagram_dotnet_dependency_graph::mapper;
use common::{cache, fixture};
use serde_json::Value;

const UNKNOWN: &str = "parity:unknown";

fn transcript() -> Value {
    serde_json::from_slice(&fs::read(fixture("dotnet.transcript.json")).unwrap()).unwrap()
}

fn lines(value: &Value) -> Vec<String> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l.as_str().unwrap().to_owned())
        .collect()
}

fn quoted(value: Option<&str>) -> String {
    value.map_or_else(|| "null".to_owned(), |v| serde_json::to_string(v).unwrap())
}

fn capital(value: bool) -> &'static str {
    if value { "True" } else { "False" }
}

/// Where a solution of the transcript's corpus is here, and what its absolute paths are masked with.
fn located(relative: &str) -> Option<(PathBuf, String, PathBuf)> {
    const FIXTURES: &str =
        "diagrams/dotnet-dependency-graph/backend/EtAlii.Adp.Diagram.DotNetDependencyGraph.Tests/Fixtures/";
    const EXAMPLES: &str = "diagrams/dotnet-dependency-graph/examples/";
    if let Some(rest) = relative.strip_prefix(FIXTURES) {
        return Some((
            fixture(rest),
            format!("{{src}}/{}", FIXTURES.trim_end_matches('/')),
            fixture(""),
        ));
    }
    relative.strip_prefix(EXAMPLES).map(|rest| {
        (
            fixture(&format!("examples/{rest}")),
            format!("{{src}}/{}", EXAMPLES.trim_end_matches('/')),
            fixture("examples"),
        )
    })
}

fn project_line(p: &ProjectNode) -> String {
    format!(
        "{} | {} | {} | [{}] | {}",
        p.id,
        p.name,
        p.relative_path,
        p.target_frameworks.join(", "),
        quoted(p.dotnet_version.as_deref())
    )
}

fn package_line(p: &PackageNode) -> String {
    format!(
        "{} | {} | [{}] | conflict {} | dependents {} | ambient {} | {}",
        p.id,
        p.package_id,
        p.versions.join(", "),
        capital(p.has_version_conflict),
        p.dependent_project_count,
        capital(p.is_ambient),
        quoted(p.description.as_deref())
    )
}

fn reference_line(e: &DependsOnEdge) -> String {
    let kind = match e.kind {
        DependsOnKind::Project => "Project",
        DependsOnKind::Package => "Package",
    };
    format!("{} | {kind} | {} -> {}", e.id, e.from, e.to)
}

/// The first three fields of a payload line: id, type and position.
fn placed(line: &str) -> String {
    line.splitn(4, " | ").take(3).collect::<Vec<_>>().join(" | ")
}

fn check_graph(name: &str, recorded: &Value, model: &DependencyGraphModel, mask: &dyn Fn(&str) -> String) {
    assert_eq!(
        model.projects.iter().map(project_line).collect::<Vec<_>>(),
        lines(&recorded["projects"]),
        "{name}: projects"
    );
    assert_eq!(
        model.packages.iter().map(package_line).collect::<Vec<_>>(),
        lines(&recorded["packages"]),
        "{name}: packages"
    );
    assert_eq!(
        model.edges.iter().map(reference_line).collect::<Vec<_>>(),
        lines(&recorded["references"]),
        "{name}: references"
    );
    assert_eq!(
        model
            .failures
            .iter()
            .map(|f| mask(&format!("{} | {}", f.path, f.reason)))
            .collect::<Vec<_>>(),
        lines(&recorded["failures"]),
        "{name}: failures"
    );

    let elements = mapper::elements(model, &BTreeMap::new());
    assert_eq!(
        elements
            .iter()
            .map(|e| format!("{} | {} | {}, {}", e.id, e.element_type, e.x, e.y))
            .collect::<Vec<_>>(),
        lines(&recorded["payloads"])
            .iter()
            .map(|l| placed(l))
            .collect::<Vec<_>>(),
        "{name}: payloads"
    );
    if let Some(first) = model.projects.first() {
        let stored: BTreeMap<String, (f64, f64)> = [
            (first.id.clone(), (10.5, -20.0)),
            ("project:gone.csproj".to_owned(), (1.0, 1.0)),
        ]
        .into_iter()
        .collect();
        let element = mapper::elements(model, &stored)
            .into_iter()
            .find(|e| e.id == first.id)
            .unwrap();
        assert!(element.placed);
        assert_eq!(
            vec![format!(
                "{} | {} | {}, {}",
                element.id, element.element_type, element.x, element.y
            )],
            lines(&recorded["payloadsWithAStoredPosition"])
                .iter()
                .map(|l| placed(l))
                .collect::<Vec<_>>(),
            "{name}: a stored position"
        );
    }

    let definition = DotNetDefinition::get();
    for entry in recorded["rows"].as_array().unwrap() {
        let target = entry["target"].as_str().unwrap();
        let rows: Vec<String> = definition
            .rows(model, target)
            .iter()
            .map(|r| {
                format!(
                    "{} | {} | {} | Line | read-only ({}) | group {}",
                    r.id,
                    r.label,
                    quoted(Some(&r.value)),
                    r.read_only_reason,
                    r.group
                )
            })
            .collect();
        assert_eq!(rows, lines(&entry["rows"]), "{name}: rows of '{target}'");
    }
    assert_eq!(
        format!("refused: {}", definition.refusal()),
        recorded["write"].as_str().unwrap(),
        "{name}: write"
    );
}

#[test]
fn every_solution_of_the_corpus_reads_as_the_standalone_host_read_it() {
    let transcript = transcript();
    let descriptions = PackageDescriptionReader::new(cache());
    let mut checked = 0;
    for recorded in transcript["solutions"].as_array().unwrap() {
        if recorded.get("sameAs").is_some() {
            continue;
        }
        let relative = recorded["solution"].as_str().unwrap();
        let (path, masked_as, root) = located(relative).unwrap_or_else(|| panic!("{relative} is not here"));
        let model = graph::load(&path, &descriptions);
        let root = root.to_string_lossy().trim_end_matches('/').to_owned();
        let mask = |text: &str| text.replace(&root, &masked_as).replace('\\', "/");
        check_graph(relative, recorded, &model, &mask);
        checked += 1;
    }
    assert_eq!(checked, 6);
}

fn project(path: &str, frameworks: &[&str], version: Option<&str>) -> ProjectNode {
    let name = path.rsplit('/').next().unwrap().trim_end_matches(".csproj").to_owned();
    ProjectNode {
        id: format!("project:{path}"),
        name,
        relative_path: path.to_owned(),
        target_frameworks: frameworks.iter().map(|f| (*f).to_owned()).collect(),
        dotnet_version: version.map(str::to_owned),
    }
}

fn package(id: &str, versions: &[&str], conflict: bool, description: Option<&str>, dependents: i64) -> PackageNode {
    PackageNode {
        id: format!("package:{id}"),
        package_id: id.to_owned(),
        versions: versions.iter().map(|v| (*v).to_owned()).collect(),
        has_version_conflict: conflict,
        description: description.map(str::to_owned),
        dependent_project_count: dependents,
        is_ambient: false,
    }
}

fn edge(from: &str, to: &str, kind: DependsOnKind) -> DependsOnEdge {
    DependsOnEdge {
        id: format!("depends:{from}->{to}"),
        from: from.to_owned(),
        to: to.to_owned(),
        kind,
    }
}

/// The graphs `DotNetTranscript.HandBuiltGraphs` built, in its order.
fn hand_built() -> Vec<DependencyGraphModel> {
    let described = DependencyGraphModel {
        projects: vec![
            project("src/App/App.csproj", &["net10.0", "net8.0"], Some("10.0, 8.0")),
            project("src/Lib/Lib.csproj", &["netstandard2.0"], None),
            project("src/None/None.csproj", &[], None),
            project("tests/App.Tests/App.Tests.csproj", &["net472", "net9.0"], Some("9.0")),
        ],
        packages: vec![
            package(
                "Described",
                &["1.0.0"],
                false,
                Some("A package with a description.\nOver two lines."),
                1,
            ),
            package("EmptyDescription", &["2.0.0"], false, Some(""), 1),
            package("Undescribed", &["3.0.0"], false, None, 1),
            package("TwoVersions", &["1.0.0", "1.1.0"], true, None, 2),
            package("KnownAndUnknown", &["4.0.0"], true, None, 2),
            package("Unknowable", &[], false, None, 1),
        ],
        edges: vec![
            edge(
                "project:src/App/App.csproj",
                "project:src/Lib/Lib.csproj",
                DependsOnKind::Project,
            ),
            edge(
                "project:tests/App.Tests/App.Tests.csproj",
                "project:src/App/App.csproj",
                DependsOnKind::Project,
            ),
            edge(
                "project:src/App/App.csproj",
                "package:Described",
                DependsOnKind::Package,
            ),
            edge(
                "project:src/Lib/Lib.csproj",
                "package:TwoVersions",
                DependsOnKind::Package,
            ),
            edge(
                "project:src/App/App.csproj",
                "package:TwoVersions",
                DependsOnKind::Package,
            ),
        ],
        failures: vec![SolutionFailure {
            path: "Missing.csproj".to_owned(),
            reason: "The solution names this project, but the file is not there.".to_owned(),
        }],
    };
    let many: Vec<ProjectNode> = (1..=7)
        .map(|i| project(&format!("p{i}/P{i}.csproj"), &["net10.0"], Some("10.0")))
        .collect();
    let mut xunit = package("xunit.v3", &["3.0.0"], false, Some("The test framework."), 7);
    xunit.is_ambient = true;
    let mut edges: Vec<DependsOnEdge> = many
        .iter()
        .map(|p| edge(&p.id, "package:xunit.v3", DependsOnKind::Package))
        .collect();
    edges.push(edge("project:p1/P1.csproj", "package:Serilog", DependsOnKind::Package));
    edges.push(edge(
        "project:p2/P2.csproj",
        "project:p1/P1.csproj",
        DependsOnKind::Project,
    ));
    let ambient = DependencyGraphModel {
        projects: many,
        packages: vec![xunit, package("Serilog", &["4.4.0"], false, None, 1)],
        edges,
        failures: Vec::new(),
    };
    vec![described, ambient, DependencyGraphModel::default()]
}

#[test]
fn every_graph_built_by_hand_is_placed_and_described_as_the_standalone_host_did() {
    let transcript = transcript();
    let recorded = transcript["graphs"].as_array().unwrap();
    let built = hand_built();
    assert_eq!(recorded.len(), built.len());
    for (recorded, model) in recorded.iter().zip(&built) {
        let name = recorded["graph"].as_str().unwrap();
        check_graph(name, recorded, model, &|text: &str| text.to_owned());
    }
}

#[test]
fn an_unknown_id_and_the_empty_one_have_no_rows() {
    let model = graph::load(
        &common::example().join("PipelineToolkit.slnx"),
        &PackageDescriptionReader::new(cache()),
    );
    let definition = DotNetDefinition::get();
    assert!(definition.rows(&model, UNKNOWN).is_empty());
    assert!(definition.rows(&model, "").is_empty());
}
