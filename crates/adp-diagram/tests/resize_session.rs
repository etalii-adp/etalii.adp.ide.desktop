//! A resize through the session (etalii.adp spec 030, FR-018, task T056): the standalone host's hype cycle fixture,
//! whose notes keep their size in `width` and `height` (`size.width`, `size.height`) and whose trends keep their far
//! edge in `stop` (`placement.x2`), resized by an edge as one command, written in the body's own lines and put back by
//! one undo.
//!
//! The fixture is `src/diagrams/gartner-hype-cycle-graph/backend/EtAlii.Adp.Diagram.GartnerHypeCycleGraph.Tests/
//! Fixtures/triggers-and-notes.ghg` of etalii.adp.ide.standalone at `8b6ccfc`, copied unchanged.

use std::fs;
use std::path::{Path, PathBuf};

use adp_core::Catalogue;
use adp_diagram::session::{NOT_RESIZABLE, NOT_SIZED};
use adp_diagram::tool::DiagramTools;
use adp_diagram::workspace::DiagramWorkspace;
use adp_diagram::{DiagramSession, Extent, Length, Resize, ResizedEdge};
use adp_disl::DislPluginFunctions;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/gartner-hype-cycle-graph")
}

fn tools() -> DiagramTools {
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor");
    DiagramTools::new(Catalogue::load(&vendor), |_| DislPluginFunctions::none())
}

struct Opened {
    folder: tempfile::TempDir,
    workspace: DiagramWorkspace,
    session: DiagramSession,
    original: String,
}

impl Opened {
    fn new() -> Opened {
        let folder = tempfile::tempdir().unwrap();
        fs::copy(
            fixtures().join("triggers-and-notes.ghg"),
            folder.path().join("notes.ghg"),
        )
        .unwrap();
        let mut tools = tools();
        let mut workspace = DiagramWorkspace::new(folder.path());
        let (_, session) = workspace.open(&mut tools, Path::new("notes.ghg")).unwrap();
        let original = fs::read_to_string(folder.path().join("notes.ghg")).unwrap();
        Opened {
            folder,
            workspace,
            session,
            original,
        }
    }

    fn written(&self) -> String {
        fs::read_to_string(self.folder.path().join("notes.ghg")).unwrap()
    }

    fn extents(&self) -> std::collections::HashMap<String, Extent> {
        self.workspace
            .documents()
            .get(&self.folder.path().join("notes.ghg"))
            .unwrap()
            .extents()
    }
}

#[test]
fn the_sizes_a_notation_binds_are_read_in_canvas_units_and_a_far_edge_in_the_axis_units() {
    let opened = Opened::new();
    let extents = opened.extents();
    assert_eq!(
        extents["note-1"],
        Extent {
            width: Some(Length::Canvas(160.0)),
            height: Some(Length::Canvas(64.0)),
        }
    );
    assert_eq!(
        extents["transistors"].width,
        Some(Length::Domain(480.0)),
        "1950-01 to 1990-01 is 480 months"
    );
    assert!(
        !extents.contains_key("transistor-invented"),
        "a trigger's size is fixed"
    );
}

#[test]
fn a_right_edge_resize_writes_the_width_and_one_undo_puts_the_bytes_back() {
    let mut opened = Opened::new();
    let resize = Resize {
        edge: ResizedEdge::Right,
        x: 23400.0,
        y: 3.0,
        right: 23620.0,
        bottom: 3.0,
        width: 220.0,
        height: 64.0,
    };
    opened
        .session
        .resize_element(&mut opened.workspace, "note-1", &resize)
        .unwrap();
    let expected = opened.original.replacen("    width: 160\n", "    width: 220\n", 1);
    assert_ne!(expected, opened.original);
    assert_eq!(
        opened.written(),
        expected,
        "the width alone is written, in its own line"
    );

    opened.workspace.undo();
    assert_eq!(opened.written(), opened.original, "one undo puts every byte back");
}

#[test]
fn a_left_edge_resize_moves_the_node_and_sizes_it_in_one_step() {
    let mut opened = Opened::new();
    let resize = Resize {
        edge: ResizedEdge::Left,
        x: 23388.0,
        y: 3.0,
        right: 23560.0,
        bottom: 3.0,
        width: 172.0,
        height: 64.0,
    };
    opened
        .session
        .resize_element(&mut opened.workspace, "note-1", &resize)
        .unwrap();
    let written = opened.written();
    assert!(written.contains("    at: 1949-01\n"), "{written}");
    assert!(written.contains("    width: 172\n"), "{written}");
    opened.workspace.undo();
    assert_eq!(opened.written(), opened.original, "the move and the size are one undo");
}

#[test]
fn a_trend_is_resized_by_its_far_edge_and_what_keeps_no_size_is_refused() {
    let mut opened = Opened::new();
    let resize = Resize {
        edge: ResizedEdge::Right,
        x: 23400.0,
        y: 1.0,
        right: 23892.0,
        bottom: 1.0,
        width: 492.0,
        height: 32.0,
    };
    opened
        .session
        .resize_element(&mut opened.workspace, "transistors", &resize)
        .unwrap();
    assert_eq!(
        opened.written(),
        opened
            .original
            .replacen("    stop: 1990-01\n", "    stop: 1991-01\n", 1)
    );

    let height = Resize {
        edge: ResizedEdge::Bottom,
        ..resize
    };
    assert_eq!(
        opened
            .session
            .resize_element(&mut opened.workspace, "transistors", &height),
        Err(NOT_SIZED.to_owned()),
        "a trend keeps no height"
    );
    assert_eq!(
        opened.session.resize_element(&mut opened.workspace, "i-12", &resize),
        Err(NOT_RESIZABLE.to_owned())
    );
}
