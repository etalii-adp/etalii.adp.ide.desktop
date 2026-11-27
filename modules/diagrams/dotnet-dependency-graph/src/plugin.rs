//! The capabilities of the .NET dependency graph's description (etalii.adp spec 035, T132): `reader.msbuildSolution`,
//! the module's solution and project readers as an FBL section 11 persistence plugin, and `analysis.msbuildImports`,
//! the version each package reference resolves to. Both read what they are handed, the solution as the body and the
//! project and `Directory.Packages.props` files beside it by their paths relative to it, laid out as a folder for the
//! module's readers, which read from disk; the local NuGet cache is not read, so no package has a description. The
//! reader plans nothing: the binding is read-only.

use std::path::Path;

use adp_cel::{Map, Value};
use adp_fbl::model::{FblElement, Finding, FindingSeverity, SourceLocation, codes};
use adp_fbl::plugins::{
    PersistencePlugin, PluginFile, PluginPlanRequest, PluginPlanResult, PluginReadRequest, PluginReadResult,
    PluginTemplateRequest,
};
use adp_fbl::text::Span;

use crate::descriptions::PackageDescriptionReader;
use crate::graph::{self, DependencyGraphModel, DependsOnKind};

/// The reader capability's name, which the binding's reader names.
pub const READER: &str = "reader.msbuildSolution";

/// The reader, which holds nothing of its own.
#[derive(Debug, Clone, Copy, Default)]
pub struct SolutionPlugin;

/// The solution's file name in the folder laid out: a `.slnx` when the body is XML, else a classic `.sln`.
fn solution_name(body: &[u8]) -> &'static str {
    let text = String::from_utf8_lossy(body);
    if text.trim_start_matches('\u{feff}').trim_start().starts_with('<') {
        "Solution.slnx"
    } else {
        "Solution.sln"
    }
}

/// The files laid out under a temporary folder, the body as the solution; none when the folder cannot be made.
fn laid_out(files: &[PluginFile]) -> Option<(tempfile::TempDir, std::path::PathBuf)> {
    let folder = tempfile::tempdir().ok()?;
    let mut solution = None;
    for file in files {
        let relative = if file.relative_path.is_empty() {
            solution_name(&file.bytes).to_owned()
        } else {
            file.relative_path.clone()
        };
        // A path that leaves the folder is not laid out.
        if relative.split('/').any(|part| part == "..") || Path::new(&relative).is_absolute() {
            continue;
        }
        let path = folder.path().join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok()?;
        }
        std::fs::write(&path, &file.bytes).ok()?;
        if file.relative_path.is_empty() {
            solution = Some(path);
        }
    }
    Some((folder, solution?))
}

/// The graph of the solution among `files`, read without a NuGet cache.
pub fn graph_of(files: &[PluginFile]) -> Option<DependencyGraphModel> {
    let (folder, solution) = laid_out(files)?;
    let nowhere = PackageDescriptionReader::new(folder.path().join(".no-nuget-cache"));
    Some(graph::load(&solution, &nowhere))
}

fn element(id: &str, type_name: &str, attributes: Map) -> FblElement {
    FblElement {
        id: id.to_owned(),
        id_is_stored: true,
        type_name: type_name.to_owned(),
        rule: type_name.to_owned(),
        is_relation: false,
        attributes,
        parent_id: None,
        parent_slot: None,
        source: None,
        target: None,
        referenced: Vec::new(),
        own_span: Span::default(),
        line: 0,
        file: None,
    }
}

fn texts(values: &[String]) -> Value {
    Value::list(values.iter().map(|v| Value::from(v.as_str())).collect())
}

/// The projects, the packages and the references of `graph`, each under the id the binding reads it by: a project by
/// its path and a package by its id, which the description's natural ids prefix.
fn elements(graph: &DependencyGraphModel) -> Vec<FblElement> {
    let read_id = |id: &str| {
        id.strip_prefix("project:")
            .or_else(|| id.strip_prefix("package:"))
            .unwrap_or(id)
            .to_owned()
    };
    let mut elements = Vec::new();
    for project in &graph.projects {
        let mut attributes = Map::default();
        attributes.insert("name", Value::from(project.name.as_str()));
        attributes.insert("path", Value::from(project.relative_path.as_str()));
        attributes.insert("targetFrameworks", texts(&project.target_frameworks));
        if let Some(version) = &project.dotnet_version {
            attributes.insert("dotNetVersion", Value::from(version.as_str()));
        }
        elements.push(element(&read_id(&project.id), "Project", attributes));
    }
    for package in &graph.packages {
        let mut attributes = Map::default();
        attributes.insert("name", Value::from(package.package_id.as_str()));
        attributes.insert("versions", texts(&package.versions));
        attributes.insert("hasVersionConflict", Value::Bool(package.has_version_conflict));
        attributes.insert("dependentProjectCount", Value::Int(package.dependent_project_count));
        attributes.insert("isAmbient", Value::Bool(package.is_ambient));
        if let Some(description) = &package.description {
            attributes.insert("description", Value::from(description.as_str()));
        }
        elements.push(element(&read_id(&package.id), "Package", attributes));
    }
    for edge in &graph.edges {
        let type_name = match edge.kind {
            DependsOnKind::Project => "ProjectReference",
            DependsOnKind::Package => "PackageReference",
        };
        elements.push(FblElement {
            is_relation: true,
            source: Some(read_id(&edge.from)),
            target: Some(read_id(&edge.to)),
            ..element(&edge.id, type_name, Map::default())
        });
    }
    elements
}

impl PersistencePlugin for SolutionPlugin {
    fn id(&self) -> &str {
        READER
    }

    fn read(&self, request: &PluginReadRequest) -> PluginReadResult {
        let Some(graph) = graph_of(&request.files) else {
            return PluginReadResult::new(Vec::new(), Vec::new(), true);
        };
        let findings = graph
            .failures
            .iter()
            .map(|failure| Finding {
                code: codes::UNPARSEABLE.into(),
                severity: FindingSeverity::Error,
                message: failure.reason.clone(),
                location: Some(SourceLocation {
                    file: String::new(),
                    line: 1,
                    column: 1,
                    length: 0,
                }),
                detail: None,
            })
            .collect();
        PluginReadResult::new(elements(&graph), findings, false)
    }

    fn plan(&self, _request: &PluginPlanRequest<'_>) -> PluginPlanResult {
        PluginPlanResult::Refused("A .NET dependency graph is read-only, so its reader plans no change.".into())
    }

    fn template(&self, _request: &PluginTemplateRequest) -> Vec<u8> {
        Vec::new()
    }
}

/// The analysis capability's name.
pub const IMPORTS: &str = "analysis.msbuildImports";

/// `analysis.msbuildImports`: its input's elements are the files of a working copy, each `{ "path", "text" }`, a
/// project file or a file MSBuild imports for one; its result has one element per package reference of each project
/// file, in the order of the input and then of the file, with the project's `path`, the `package` and the `version`
/// MSBuild would use, stated on the reference, centrally or through a property, and none where none is found; and a
/// finding per file that could not be read.
pub fn imports(input: &serde_json::Value) -> Result<serde_json::Value, String> {
    use serde_json::json;
    let files: Vec<PluginFile> = input["elements"]
        .as_array()
        .ok_or("The input has no `elements`.")?
        .iter()
        .filter_map(|e| {
            Some(PluginFile::new(
                e["path"].as_str()?,
                e["text"].as_str()?.as_bytes().to_vec(),
            ))
        })
        .collect();
    let folder = tempfile::tempdir().map_err(|e| e.to_string())?;
    for file in &files {
        if file.relative_path.split('/').any(|part| part == "..") || Path::new(&file.relative_path).is_absolute() {
            continue;
        }
        let path = folder
            .path()
            .join(file.relative_path.replace('/', std::path::MAIN_SEPARATOR_STR));
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, &file.bytes).map_err(|e| e.to_string())?;
    }
    let mut elements = Vec::new();
    let mut findings = Vec::new();
    for file in files.iter().filter(|f| f.relative_path.ends_with("proj")) {
        let path = folder
            .path()
            .join(file.relative_path.replace('/', std::path::MAIN_SEPARATOR_STR));
        let reading = crate::project::read(&path);
        for reference in &reading.package_references {
            let mut attributes = serde_json::Map::new();
            attributes.insert("path".into(), json!(file.relative_path));
            attributes.insert("package".into(), json!(reference.package_id));
            if let Some(version) = &reference.version {
                attributes.insert("version".into(), json!(version));
            }
            elements.push(json!({
                "id": format!("{}->{}", file.relative_path, reference.package_id),
                "attributes": attributes,
            }));
        }
        for failure in &reading.failures {
            findings.push(json!({ "code": "std.unparseable", "severity": "error", "message": failure.reason }));
        }
    }
    Ok(json!({ "elements": elements, "findings": findings }))
}
