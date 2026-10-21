//! The three forms of a link and the conversions between them (mindmap Requirement 12.3), and the rewrite when a map
//! moves (12.14). Ported from the standalone host's `MindmapLinks.Tests.cs` at `8b6ccfc`.

use std::fs;
use std::path::{Path, PathBuf};

use adp_diagram_mindmap::document::MindmapDocument;
use adp_diagram_mindmap::links;

struct Project {
    folder: tempfile::TempDir,
}

impl Project {
    fn new() -> Self {
        let folder = tempfile::tempdir().unwrap();
        fs::create_dir_all(folder.path().join("docs")).unwrap();
        fs::create_dir_all(folder.path().join("src/backend")).unwrap();
        fs::write(folder.path().join("src/backend/Service.cs"), "").unwrap();
        Project { folder }
    }

    fn root(&self) -> &Path {
        self.folder.path()
    }

    fn map_at(&self, folder: &str) -> PathBuf {
        self.root().join(folder).join("map.mm")
    }
}

#[test]
fn to_map_relative_produces_freeplanes_form_forward_slashes_from_the_maps_folder() {
    let project = Project::new();
    let stored = links::to_map_relative(
        &project.map_at("docs"),
        &["src", "backend", "Service.cs"],
        project.root(),
    );
    assert_eq!(stored, "../src/backend/Service.cs");
}

#[test]
fn resolve_within_project_turns_a_stored_link_back_into_the_file() {
    let project = Project::new();
    let resolved = links::resolve_within_project(&project.map_at("docs"), "../src/backend/Service.cs", project.root());
    assert_eq!(
        resolved,
        Some(std::path::absolute(project.root().join("src").join("backend").join("Service.cs")).unwrap())
    );
}

#[test]
fn resolve_within_project_refuses_a_link_that_leaves_the_project() {
    let project = Project::new();
    assert_eq!(
        links::resolve_within_project(&project.map_at("docs"), "../../../etc/passwd", project.root()),
        None
    );
}

#[test]
fn resolve_within_project_leaves_a_url_alone() {
    let project = Project::new();
    assert!(links::is_external("https://www.freeplane.org/"));
    assert_eq!(
        links::resolve_within_project(&project.map_at("docs"), "https://www.freeplane.org/", project.root()),
        None
    );
}

#[test]
fn project_relative_splits_into_segments() {
    let project = Project::new();
    assert_eq!(
        links::project_relative(&project.root().join("src/backend/Service.cs"), project.root()),
        ["src", "backend", "Service.cs"]
    );
}

#[test]
fn rebase_rewrites_every_file_link_when_the_map_changes_folder() {
    let project = Project::new();
    let mut document = MindmapDocument::parse(
        "<map version=\"freeplane 1.11.5\"><node TEXT=\"r\" ID=\"r\" LINK=\"../src/backend/Service.cs\"><node TEXT=\"a\" ID=\"a\" LINK=\"https://example.org/\"/><node TEXT=\"b\" ID=\"b\"/></node></map>",
        true,
    )
    .unwrap();
    let rewritten = links::rebase(&mut document, &project.map_at("docs"), &project.map_at("src/backend"));
    assert_eq!(rewritten, 1);
    assert_eq!(document.link(document.root()), Some("Service.cs"));
    assert_eq!(document.link(document.find("a").unwrap()), Some("https://example.org/"));
    assert_eq!(document.link(document.find("b").unwrap()), None);
}

#[test]
fn rebase_is_a_no_op_for_a_rename_within_the_folder() {
    let project = Project::new();
    let mut document = MindmapDocument::parse(
        "<map version=\"freeplane 1.11.5\"><node TEXT=\"r\" ID=\"r\" LINK=\"../x.cs\"/></map>",
        true,
    )
    .unwrap();
    let rewritten = links::rebase(
        &mut document,
        &project.map_at("docs"),
        &project.root().join("docs/renamed.mm"),
    );
    assert_eq!(rewritten, 0);
    assert_eq!(document.link(document.root()), Some("../x.cs"));
}
