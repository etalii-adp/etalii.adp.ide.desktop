//! The three readers (etalii.adp spec 030, task T082): the standalone host's `SolutionReader.Tests.cs`,
//! `ProjectReader.Tests.cs` and `PackageDescriptionReader.Tests.cs` at `8b6ccfc`, with the same expectations.

mod common;

use std::fs;
use std::path::Path;

use adp_diagram_dotnet_dependency_graph::descriptions::PackageDescriptionReader;
use adp_diagram_dotnet_dependency_graph::{project, solution};
use common::{cache, fixture};

// ---- the solution ------------------------------------------------------------------------------------------------------

#[test]
fn the_projects_that_exist_are_found_in_either_serialization() {
    for name in ["solution/Both.sln", "solution/Both.slnx", "solution/Flat.sln"] {
        let reading = solution::read(&fixture(name));
        let mut names: Vec<&str> = reading.projects.iter().map(|p| p.name.as_str()).collect();
        names.sort();
        assert_eq!(names, ["Alpha", "Beta"], "{name}");
        let mut paths: Vec<&str> = reading.projects.iter().map(|p| p.relative_path.as_str()).collect();
        paths.sort();
        assert_eq!(paths, ["Alpha.csproj", "nested/Beta.csproj"], "{name}");
    }
}

#[test]
fn a_named_project_that_is_not_there_is_reported_and_the_rest_kept() {
    for name in ["solution/Both.sln", "solution/Both.slnx"] {
        let reading = solution::read(&fixture(name));
        assert_eq!(reading.failures.len(), 1, "{name}");
        assert_eq!(reading.failures[0].path, "Missing.csproj");
        assert_eq!(
            reading.failures[0].reason,
            "The solution names this project, but the file is not there."
        );
        assert_eq!(reading.projects.len(), 2);
    }
}

#[test]
fn a_classic_solution_folder_is_skipped_rather_than_reported_missing() {
    let reading = solution::read(&fixture("solution/Both.sln"));
    assert!(!reading.projects.iter().any(|p| p.name == "nested"));
    assert!(!reading.failures.iter().any(|f| f.path == "nested"));
}

#[test]
fn the_two_classic_shapes_the_sdk_produces_agree() {
    let flat = solution::read(&fixture("solution/Flat.sln"));
    let folders = solution::read(&fixture("solution/Both.sln"));
    let paths = |r: &solution::SolutionReading| r.projects.iter().map(|p| p.relative_path.clone()).collect::<Vec<_>>();
    assert_eq!(paths(&flat), paths(&folders));
    assert_eq!(flat.failures.len(), folders.failures.len());
}

#[test]
fn an_absent_solution_is_a_failure_rather_than_a_panic() {
    let reading = solution::read(&fixture("solution/Absent.slnx"));
    assert!(reading.projects.is_empty());
    assert_eq!(reading.failures.len(), 1);
    assert_eq!(reading.failures[0].reason, "The solution file does not exist.");
}

#[test]
fn an_unparseable_solution_reports_the_reason() {
    let path = fixture("solution/Broken.slnx");
    let reading = solution::read(&path);
    assert!(reading.projects.is_empty());
    assert_eq!(reading.failures.len(), 1);
    assert_eq!(reading.failures[0].path, path.to_string_lossy());
    assert_eq!(reading.failures[0].reason, "The solution file could not be parsed.");
}

#[test]
fn a_solution_that_names_no_projects_is_not_a_failure() {
    let reading = solution::read(&fixture("solution/Empty.slnx"));
    assert!(reading.projects.is_empty());
    assert!(reading.failures.is_empty());
}

#[test]
fn an_slnx_takes_projects_and_not_the_files_filed_beside_them() {
    let reading = solution::read(&fixture("solution/Both.slnx"));
    assert!(!reading.projects.iter().any(|p| p.name == "Directory.Packages"));
    assert!(!reading.failures.iter().any(|f| f.path.contains("Directory.Packages")));
}

#[test]
fn relative_paths_are_forward_slashed_whatever_the_solution_wrote() {
    let reading = solution::read(&fixture("solution/Both.sln"));
    let beta = reading.projects.iter().find(|p| p.name == "Beta").unwrap();
    assert_eq!(beta.relative_path, "nested/Beta.csproj");
}

#[test]
fn a_project_outside_the_solutions_folder_is_named_by_its_relative_path_up() {
    let folder = tempfile::tempdir().unwrap();
    fs::create_dir_all(folder.path().join("sln")).unwrap();
    fs::create_dir_all(folder.path().join("lib")).unwrap();
    fs::write(folder.path().join("lib/Lib.csproj"), "<Project />").unwrap();
    let path = folder.path().join("sln/S.slnx");
    fs::write(&path, "<Solution><Project Path=\"..\\lib\\Lib.csproj\" /></Solution>").unwrap();
    let reading = solution::read(&path);
    assert_eq!(reading.projects.len(), 1);
    assert_eq!(reading.projects[0].relative_path, "../lib/Lib.csproj");
    assert_eq!(reading.projects[0].name, "Lib");
}

// ---- a project ---------------------------------------------------------------------------------------------------------

#[test]
fn multi_targeted_frameworks_are_read_not_only_the_singular_form() {
    let reading = project::read(&fixture("cpm/nested/Nested.csproj"));
    assert_eq!(reading.target_frameworks, ["net10.0", "net8.0"]);
}

#[test]
fn a_versionless_reference_is_resolved_from_central_package_management() {
    let reading = project::read(&fixture("cpm/nested/Nested.csproj"));
    let serilog = reading
        .package_references
        .iter()
        .find(|p| p.package_id == "Serilog")
        .unwrap();
    assert_eq!(serilog.version.as_deref(), Some("4.4.0"));
}

#[test]
fn the_nearest_central_version_wins_as_msbuild_has_it() {
    let reading = project::read(&fixture("cpm/nested/Nested.csproj"));
    let shadowed = reading
        .package_references
        .iter()
        .find(|p| p.package_id == "Shadowed")
        .unwrap();
    assert_eq!(shadowed.version.as_deref(), Some("2.0.0"));
}

#[test]
fn a_version_written_on_the_reference_wins_over_the_central_one() {
    let reading = project::read(&fixture("cpm/nested/Nested.csproj"));
    let pinned = reading
        .package_references
        .iter()
        .find(|p| p.package_id == "Pinned")
        .unwrap();
    assert_eq!(pinned.version.as_deref(), Some("9.9.9"));
}

#[test]
fn a_version_written_as_a_child_element_is_read() {
    let reading = project::read(&fixture("cpm/nested/Nested.csproj"));
    let child = reading
        .package_references
        .iter()
        .find(|p| p.package_id == "ChildVersion")
        .unwrap();
    assert_eq!(child.version.as_deref(), Some("3.3.3"));
}

#[test]
fn a_reference_whose_version_is_not_discoverable_is_kept_without_one() {
    let reading = project::read(&fixture("cpm/nested/Nested.csproj"));
    let unknowable = reading
        .package_references
        .iter()
        .find(|p| p.package_id == "Unknowable")
        .unwrap();
    assert_eq!(unknowable.version, None);
}

#[test]
fn a_project_reference_is_resolved_to_a_full_path() {
    let reading = project::read(&fixture("cpm/nested/Nested.csproj"));
    assert_eq!(reading.project_references.len(), 1);
    assert_eq!(reading.project_references[0].absolute_path, fixture("cpm/Root.csproj"));
    assert_eq!(reading.project_references[0].include, "..\\Root.csproj");
}

#[test]
fn a_wildcard_project_reference_is_expanded_as_msbuild_expands_it() {
    let reading = project::read(&fixture("cpm/Host.csproj"));
    let mut paths: Vec<_> = reading
        .project_references
        .iter()
        .map(|r| r.absolute_path.clone())
        .collect();
    paths.sort();
    assert_eq!(
        paths,
        [
            fixture("cpm/Root.csproj"),
            fixture("cpm/globbed/modA/backend/ProjA/ProjA.csproj"),
            fixture("cpm/globbed/modB/backend/ProjB/ProjB.csproj"),
        ]
    );
}

#[test]
fn a_wildcard_matching_nothing_yields_no_phantom_path() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("P.csproj");
    fs::write(
        &path,
        "<Project><ItemGroup><ProjectReference Include=\"nowhere\\*\\*.csproj\" /></ItemGroup></Project>",
    )
    .unwrap();
    let reading = project::read(&path);
    assert!(reading.project_references.is_empty());
}

#[test]
fn an_unreadable_project_is_a_failure_rather_than_a_panic() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("Broken.csproj");
    fs::write(&path, "<Project><ItemGroup>").unwrap();
    let reading = project::read(&path);
    assert_eq!(reading.failures.len(), 1);
    assert!(
        reading.failures[0]
            .reason
            .starts_with("The project file could not be read: ")
    );
    assert!(reading.package_references.is_empty());
    assert!(reading.project_references.is_empty());
}

#[test]
fn an_unexpanded_property_is_not_a_framework_and_duplicates_go() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("P.csproj");
    fs::write(
        &path,
        "<Project><PropertyGroup><TargetFrameworks> net10.0 ; $(Extra);NET10.0;;net8.0</TargetFrameworks></PropertyGroup></Project>",
    )
    .unwrap();
    assert_eq!(project::read(&path).target_frameworks, ["net10.0", "net8.0"]);
}

// ---- a package's description -------------------------------------------------------------------------------------------

#[test]
fn the_description_is_taken_from_a_cached_nuspec() {
    let reader = PackageDescriptionReader::new(cache());
    assert_eq!(
        reader.read("Cached", &["1.0.0".to_owned()]).as_deref(),
        Some("Simple .NET logging with fully-structured events")
    );
}

#[test]
fn the_cache_path_is_lowercased_because_that_is_how_nuget_restores() {
    assert_eq!(
        PackageDescriptionReader::nuspec_path_for(Path::new("root"), "Serilog.Sinks.File", "4.4.0"),
        Path::new("root")
            .join("serilog.sinks.file")
            .join("4.4.0")
            .join("serilog.sinks.file.nuspec")
    );
}

#[test]
fn a_package_is_found_whatever_casing_the_reference_used() {
    let reader = PackageDescriptionReader::new(cache());
    assert_eq!(
        reader.read("CACHED", &["1.0.0".to_owned()]).as_deref(),
        Some("Simple .NET logging with fully-structured events")
    );
}

#[test]
fn a_package_never_restored_is_absence_rather_than_an_error() {
    let reader = PackageDescriptionReader::new(cache());
    assert_eq!(reader.read("NeverRestored", &["1.0.0".to_owned()]), None);
}

#[test]
fn an_empty_description_is_empty_rather_than_absent() {
    let reader = PackageDescriptionReader::new(cache());
    assert_eq!(
        reader.read("EmptyDescription", &["1.0.0".to_owned()]).as_deref(),
        Some("")
    );
}

#[test]
fn every_version_offered_is_tried() {
    let reader = PackageDescriptionReader::new(cache());
    assert_eq!(
        reader
            .read("Cached", &["0.0.1".to_owned(), "1.0.0".to_owned()])
            .as_deref(),
        Some("Simple .NET logging with fully-structured events")
    );
}

#[test]
fn an_absent_cache_folder_is_absence_rather_than_a_panic() {
    let reader = PackageDescriptionReader::new(fixture("no-such-cache"));
    assert_eq!(reader.read("Cached", &["1.0.0".to_owned()]), None);
}
