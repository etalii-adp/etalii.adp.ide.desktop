//! A .NET dependency graph on the canvas with its module (etalii.adp spec 030, task T082): the standalone host's
//! `pipeline-toolkit` example, a solution registered by its `.adp` with a `body:` header, opened from the parity
//! fixtures, drawn on the module's depth-layered layout with its band of packages, a box dragged by its top-left into
//! the registration's `layout:` block, a reference refused a move, and the drag undone through the project's history.

use std::fs;
use std::path::{Path, PathBuf};

use adp_diagram_dotnet_dependency_graph::registration_layout::number;
use adp_scene::scene::{Owner, Primitive};

use super::{Harness, texts_of};

const CORE: &str = "project:src/Pipeline.Core/Pipeline.Core.csproj";
const CLI: &str = "project:src/Pipeline.Cli/Pipeline.Cli.csproj";
const SERILOG: &str = "package:Serilog";

fn files_under(root: &Path, folder: &Path, out: &mut Vec<String>) {
    for entry in fs::read_dir(folder).unwrap().flatten() {
        if entry.path().is_dir() {
            files_under(root, &entry.path(), out);
        } else {
            let relative = entry
                .path()
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace(std::path::MAIN_SEPARATOR, "/");
            out.push(relative);
        }
    }
}

pub(super) fn pipeline_toolkit() -> Harness {
    let folder: PathBuf =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/fixtures/dotnet/dependency-graph/pipeline-toolkit");
    let mut files = Vec::new();
    files_under(&folder, &folder, &mut files);
    let names: Vec<&str> = files.iter().map(String::as_str).collect();
    Harness::open_from(&folder, &names, "PipelineToolkit.adp", "PipelineToolkit.adp")
}

#[test]
fn a_solution_opens_on_its_layered_layout_and_a_box_is_dragged_into_its_registration() {
    let mut harness = pipeline_toolkit();
    assert!(harness.tab.session.module_view().is_some());
    harness.settle();

    // The four projects with their frameworks, and the three packages with their versions.
    let scene = harness.tab.scene().expect("a scene");
    let texts = texts_of(scene);
    for label in [
        "Pipeline.Core",
        "Pipeline.Storage",
        "Pipeline.Cli",
        "Pipeline.Core.Tests",
        "net10.0, netstandard2.0",
        "Serilog",
        "3.1.1, 4.4.0",
        "System.Text.Json",
        "xunit.v3",
    ] {
        assert!(texts.contains(&label), "{label} is drawn: {texts:?}");
    }
    let references = scene
        .primitives
        .iter()
        .filter(|p| matches!(p, Primitive::Edge(e) if matches!(e.owner, Owner::Relation(_))))
        .count();
    assert_eq!(references, 9);
    // Layered by depth, left to right, the packages banded beyond the last project column.
    let core = harness.tab.bounds_of(CORE).expect("a project is drawn");
    let cli = harness.tab.bounds_of(CLI).unwrap();
    let serilog = harness.tab.bounds_of(SERILOG).unwrap();
    assert_eq!((core.x, core.y, core.width, core.height), (0.0, 0.0, 220.0, 56.0));
    assert_eq!((cli.x, cli.y), (640.0, 0.0));
    assert_eq!((serilog.x, serilog.y), (1160.0, 0.0));

    // A drag writes where the box's top-left went, into the registration and nowhere else.
    let original = harness.written();
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.move_by(host.workspace(root), &[SERILOG.to_owned()], (40.5, 20.0));
    let moved = harness.written();
    assert!(
        moved.ends_with(&format!(
            "layout:\r\n  {SERILOG}: {} {}\r\n",
            number(serilog.x + 40.5),
            number(serilog.y + 20.0)
        )),
        "{moved}"
    );
    harness.tab.changed();
    harness.settle();
    let after = harness.tab.bounds_of(SERILOG).unwrap();
    assert!(
        (after.x - (serilog.x + 40.5)).abs() < 1e-3 && (after.y - (serilog.y + 20.0)).abs() < 1e-3,
        "{after:?}"
    );

    // A reference follows its ends, so it is never moved.
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.move_by(
        host.workspace(root),
        &[format!("depends:{CORE}->{SERILOG}")],
        (40.0, 20.0),
    );
    assert_eq!(harness.written(), moved);

    // One undo takes the drag back, byte for byte.
    assert!(harness.workspace().undo().is_done());
    assert_eq!(harness.written(), original);
    assert_eq!(fs::read_to_string(&harness.body).unwrap(), harness.original);

    harness.screenshot(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/screenshots/dotnet-dependency-graph.png"),
    );
}
