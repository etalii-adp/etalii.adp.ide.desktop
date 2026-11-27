//! `reader.msbuildSolution` and `analysis.msbuildImports` (etalii.adp spec 035, T132): a solution read from the files
//! handed beside it, and the version each package reference resolves to.

use adp_diagram_dotnet_dependency_graph::plugin::{SolutionPlugin, imports};
use adp_fbl::plugins::{PersistencePlugin, PluginFile, PluginReadRequest};

const SOLUTION: &str = "<Solution>\n  <Project Path=\"src/App/App.csproj\" />\n</Solution>\n";
const PROJECT: &str = "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup><TargetFramework>net10.0</TargetFramework></PropertyGroup>\n  <ItemGroup><PackageReference Include=\"Serilog\" /></ItemGroup>\n</Project>\n";
const VERSIONS: &str =
    "<Project>\n  <ItemGroup><PackageVersion Include=\"Serilog\" Version=\"4.4.0\" /></ItemGroup>\n</Project>\n";

#[test]
fn a_solution_is_read_with_the_files_handed_beside_it() {
    let read = SolutionPlugin.read(&PluginReadRequest {
        files: vec![
            PluginFile::new("", SOLUTION.as_bytes().to_vec()),
            PluginFile::new("src/App/App.csproj", PROJECT.as_bytes().to_vec()),
            PluginFile::new("Directory.Packages.props", VERSIONS.as_bytes().to_vec()),
        ],
        supplementary_entries: Vec::new(),
    });
    let ids: Vec<(&str, &str)> = read
        .elements
        .iter()
        .map(|e| (e.id.as_str(), e.type_name.as_str()))
        .collect();
    assert_eq!(
        ids,
        [
            ("src/App/App.csproj", "Project"),
            ("Serilog", "Package"),
            (
                "depends:project:src/App/App.csproj->package:Serilog",
                "PackageReference"
            ),
        ]
    );
    assert!(read.findings.is_empty() && !read.unreadable);
}

#[test]
fn a_central_version_is_the_one_a_reference_resolves_to() {
    let output = imports(&serde_json::json!({ "elements": [
        { "path": "src/App/App.csproj", "text": PROJECT },
        { "path": "Directory.Packages.props", "text": VERSIONS }
    ] }))
    .unwrap();
    assert_eq!(output["elements"][0]["attributes"]["version"], "4.4.0");
    assert_eq!(output["elements"][0]["id"], "src/App/App.csproj->Serilog");
}
