//! The derived graph (etalii.adp spec 030, task T082), ported from the standalone host's `DependencyGraph.cs`,
//! `DependencyGraphStore.Derive` and the records of `_Model/` at `8b6ccfc`: one node per project, one per package id,
//! one edge per declared reference, and the failures that travel with them.
//!
//! The element ids are the load-bearing decision, because a stored position is bound to one: a project is
//! `project:<path relative to the solution>`, a package `package:<id>` without its version, so a version bump does not
//! cost a position, and a reference `depends:<from>-><to>`. A package referenced by at least `max(5, ceil(0.15 x the
//! projects))` projects is marked ambient: a marking the canvas reads, never a removal.

use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

use crate::descriptions::PackageDescriptionReader;
use crate::paths;
use crate::project::{self, ProjectReading};
use crate::solution::{self, SolutionReading};

/// The share of a solution's projects at which a package stops discriminating anything and becomes ambient.
pub const AMBIENT_SHARE: f64 = 0.15;

/// The fewest dependents a package can have and still be ambient, whatever the share says.
const AMBIENT_FLOOR: i64 = 5;

/// `net10.0`, `net8.0`: the target frameworks that name a .NET version.
static DOTNET_TARGET_FRAMEWORK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^net(?P<version>\d+\.\d+)$").expect("the framework pattern compiles"));

/// Something the solution named that could not be read: what it named, and why, in a sentence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolutionFailure {
    pub path: String,
    pub reason: String,
}

/// One project of the solution, as the graph draws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectNode {
    pub id: String,
    pub name: String,
    pub relative_path: String,
    pub target_frameworks: Vec<String>,
    /// The .NET version the frameworks name, where they name one.
    pub dotnet_version: Option<String>,
}

/// One NuGet package the solution consumes, however many projects consume it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageNode {
    pub id: String,
    pub package_id: String,
    /// Every version the projects ask for, ordered ignoring case.
    pub versions: Vec<String>,
    pub has_version_conflict: bool,
    /// From the local NuGet cache; none for not obtainable.
    pub description: Option<String>,
    pub dependent_project_count: i64,
    pub is_ambient: bool,
}

/// What kind of thing a reference points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependsOnKind {
    /// A `ProjectReference`: project to project.
    Project,
    /// A `PackageReference`: project to package.
    Package,
}

/// One declared dependency, from the project that declares it to what it depends on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependsOnEdge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub kind: DependsOnKind,
}

/// The derived graph, and the reasons for what it could not resolve.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DependencyGraphModel {
    pub projects: Vec<ProjectNode>,
    pub packages: Vec<PackageNode>,
    pub edges: Vec<DependsOnEdge>,
    pub failures: Vec<SolutionFailure>,
}

fn id_of_project(relative_path: &str) -> String {
    format!("project:{relative_path}")
}

fn id_of_package(package_id: &str) -> String {
    format!("package:{package_id}")
}

fn id_of_edge(from: &str, to: &str) -> String {
    format!("depends:{from}->{to}")
}

/// The .NET version the frameworks name: each distinct `net<major>.<minor>`, in ordinal order, joined by `, `.
fn dotnet_version_of(frameworks: &[String]) -> Option<String> {
    let mut versions: Vec<String> = Vec::new();
    for framework in frameworks {
        if let Some(captures) = DOTNET_TARGET_FRAMEWORK.captures(framework) {
            let version = captures["version"].to_owned();
            if !versions.contains(&version) {
                versions.push(version);
            }
        }
    }
    versions.sort();
    (!versions.is_empty()).then(|| versions.join(", "))
}

/// The versions one package id is asked for, under the spelling first read.
struct Asked {
    package_id: String,
    versions: Vec<String>,
}

/// Derives the graph from one solution reading and each resolved project's reading, keyed by
/// [`paths::key`] of its absolute path (`DependencyGraph.Derive`).
pub fn derive(solution: &SolutionReading, readings: &HashMap<String, ProjectReading>) -> DependencyGraphModel {
    let mut failures = solution.failures.clone();
    let mut projects = Vec::new();
    let mut edges: Vec<DependsOnEdge> = Vec::new();
    let mut asked: Vec<Asked> = Vec::new();
    let mut versionless: Vec<String> = Vec::new();

    let mut id_by_path: HashMap<String, String> = HashMap::new();
    for project in &solution.projects {
        id_by_path
            .entry(paths::key(&project.absolute_path))
            .or_insert_with(|| id_of_project(&project.relative_path));
    }

    for project in &solution.projects {
        let key = paths::key(&project.absolute_path);
        let Some(reading) = readings.get(&key) else {
            failures.push(SolutionFailure {
                path: project.relative_path.clone(),
                reason: "This project was not read, so its references are unknown.".to_owned(),
            });
            continue;
        };
        failures.extend(reading.failures.iter().cloned());
        let id = id_by_path[&key].clone();
        projects.push(ProjectNode {
            id: id.clone(),
            name: project.name.clone(),
            relative_path: project.relative_path.clone(),
            target_frameworks: reading.target_frameworks.clone(),
            dotnet_version: dotnet_version_of(&reading.target_frameworks),
        });

        for reference in &reading.project_references {
            let Some(target) = id_by_path.get(&paths::key(&reference.absolute_path)) else {
                failures.push(SolutionFailure {
                    path: reference.include.clone(),
                    reason: format!(
                        "{} references this project, but the solution does not contain it.",
                        project.name
                    ),
                });
                continue;
            };
            edges.push(DependsOnEdge {
                id: id_of_edge(&id, target),
                from: id.clone(),
                to: target.clone(),
                kind: DependsOnKind::Project,
            });
        }

        for reference in &reading.package_references {
            let package = id_of_package(&reference.package_id);
            let at = match asked
                .iter()
                .position(|a| paths::equal_ignoring_case(&a.package_id, &reference.package_id))
            {
                Some(at) => at,
                None => {
                    asked.push(Asked {
                        package_id: reference.package_id.clone(),
                        versions: Vec::new(),
                    });
                    asked.len() - 1
                }
            };
            match &reference.version {
                None => {
                    if !versionless
                        .iter()
                        .any(|v| paths::equal_ignoring_case(v, &reference.package_id))
                    {
                        versionless.push(reference.package_id.clone());
                    }
                }
                Some(version) => {
                    let versions = &mut asked[at].versions;
                    if !versions.iter().any(|v| paths::equal_ignoring_case(v, version)) {
                        versions.push(version.clone());
                        versions.sort_by(|a, b| paths::compare_ignoring_case(a, b));
                    }
                }
            }
            let edge = DependsOnEdge {
                id: id_of_edge(&id, &package),
                from: id.clone(),
                to: package,
                kind: DependsOnKind::Package,
            };
            if edges.iter().all(|e| e.id != edge.id) {
                edges.push(edge);
            }
        }
    }

    let mut dependents: HashMap<&str, i64> = HashMap::new();
    for edge in edges.iter().filter(|e| e.kind == DependsOnKind::Package) {
        *dependents.entry(edge.to.as_str()).or_default() += 1;
    }
    let share = (projects.len() as f64 * AMBIENT_SHARE).ceil() as i64;
    let ambient_at = AMBIENT_FLOOR.max(share);

    let mut packages: Vec<PackageNode> = asked
        .iter()
        .map(|a| {
            let id = id_of_package(&a.package_id);
            let count = dependents.get(id.as_str()).copied().unwrap_or(0);
            let unknown = versionless.iter().any(|v| paths::equal_ignoring_case(v, &a.package_id));
            PackageNode {
                has_version_conflict: a.versions.len() > 1 || (!a.versions.is_empty() && unknown),
                id,
                package_id: a.package_id.clone(),
                versions: a.versions.clone(),
                description: None,
                dependent_project_count: count,
                is_ambient: count >= ambient_at,
            }
        })
        .collect();
    packages.sort_by(|a, b| paths::compare_ignoring_case(&a.package_id, &b.package_id));

    DependencyGraphModel {
        projects,
        packages,
        edges,
        failures,
    }
}

/// The graph of the solution at `path`: the solution read, every project it resolves read, the graph derived, and each
/// package's description from `descriptions` (`DependencyGraphStore.Derive`).
pub fn load(path: &Path, descriptions: &PackageDescriptionReader) -> DependencyGraphModel {
    let solution = solution::read(path);
    let readings: HashMap<String, ProjectReading> = solution
        .projects
        .iter()
        .map(|p| (paths::key(&p.absolute_path), project::read(&p.absolute_path)))
        .collect();
    let mut graph = derive(&solution, &readings);
    for package in &mut graph.packages {
        package.description = descriptions.read(&package.package_id, &package.versions);
    }
    graph
}
