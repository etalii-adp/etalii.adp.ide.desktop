//! Where a hand-made arrangement lives and when it wins: the layout sidecar beside the `.dsl`, never inside it and
//! never something a diagram depends on (etalii.adp spec 030, task T061).
//!
//! Ported from the standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/C4LayoutSidecar.Tests.cs`
//! and `C4LayoutSidecar.ContendedWrite.Tests.cs` at `8b6ccfc`: the sidecar's path, positions written and read back
//! per view, a missing or corrupt sidecar read as none, a view cleared, an authored position winning over the computed
//! one unless the document declares `autoLayout`; and a change that refuses rather than wiping every other view's
//! arrangement while the file cannot be read, and replaces a sidecar that is already nonsense.

use std::fs;
use std::path::PathBuf;

use adp_diagram_c4::layout;
use adp_diagram_c4::metrics::C4Metrics;
use adp_diagram_c4::model::C4Workspace;
use adp_diagram_c4::parser;
use adp_diagram_c4::sidecar::{self, Positions};

const MODEL: &str = "workspace {
    model {
        a = softwareSystem \"A\" \"desc\"
        b = softwareSystem \"B\" \"desc\"
        a -> b \"Calls\" \"HTTPS\"
    }
    views {
        systemLandscape \"all\" {
            include *
        }
    }
}";

fn body() -> (tempfile::TempDir, PathBuf) {
    let folder = tempfile::tempdir().unwrap();
    let body = folder.path().join("model.dsl");
    fs::write(&body, MODEL).unwrap();
    (folder, body)
}

fn load(dsl: &str) -> C4Workspace {
    parser::parse_text(dsl)
}

fn authored(id: &str, x: f64, y: f64) -> Positions {
    let mut positions = Positions::default();
    positions.set(id, x, y);
    positions
}

#[test]
fn the_sidecar_sits_beside_the_document_and_is_not_the_document() {
    let (folder, body) = body();
    assert_eq!(sidecar::path_for(&body), folder.path().join("model.layout.json"));
}

#[test]
fn a_position_written_is_read_back() {
    let (_folder, body) = body();
    assert_eq!(sidecar::write(&body, "all", "a", 120.0, 340.0), "");
    assert_eq!(sidecar::read(&body, "all").get("a"), Some((120.0, 340.0)));
}

#[test]
fn positions_are_kept_per_view_so_arranging_one_does_not_move_another() {
    let (_folder, body) = body();
    sidecar::write(&body, "all", "a", 10.0, 10.0);
    sidecar::write(&body, "other", "a", 99.0, 99.0);
    assert_eq!(sidecar::read(&body, "all").get("a"), Some((10.0, 10.0)));
    assert_eq!(sidecar::read(&body, "other").get("a"), Some((99.0, 99.0)));
}

#[test]
fn no_sidecar_reads_as_no_positions_rather_than_throwing() {
    let (_folder, body) = body();
    assert!(sidecar::read(&body, "all").is_empty());
}

#[test]
fn a_corrupt_sidecar_costs_the_arrangement_not_the_diagram() {
    let (_folder, body) = body();
    fs::write(sidecar::path_for(&body), "{ this is not json").unwrap();
    assert!(sidecar::read(&body, "all").is_empty());
}

#[test]
fn clear_forgets_one_views_positions() {
    let (_folder, body) = body();
    sidecar::write(&body, "all", "a", 10.0, 10.0);
    sidecar::write(&body, "other", "b", 20.0, 20.0);
    assert_eq!(sidecar::clear(&body, "all"), "");
    assert!(sidecar::read(&body, "all").is_empty());
    assert!(!sidecar::read(&body, "other").is_empty());
}

#[test]
fn an_authored_position_wins_over_the_computed_one() {
    let workspace = load(MODEL);
    let view = &workspace.views[0];
    let computed = layout::compute(&workspace, view, &C4Metrics::default(), None);
    let laid_out = layout::compute(
        &workspace,
        view,
        &C4Metrics::default(),
        Some(&authored("a", 500.0, 600.0)),
    );
    let a = laid_out.boxes.get("a").unwrap();
    assert_eq!((a.x, a.y), (500.0, 600.0));
    assert_eq!(a.width, computed.boxes.get("a").unwrap().width);
}

#[test]
fn an_element_with_no_authored_position_keeps_the_computed_one() {
    let workspace = load(MODEL);
    let view = &workspace.views[0];
    let computed = layout::compute(&workspace, view, &C4Metrics::default(), None);
    let laid_out = layout::compute(
        &workspace,
        view,
        &C4Metrics::default(),
        Some(&authored("a", 500.0, 600.0)),
    );
    assert_eq!(computed.boxes.get("b"), laid_out.boxes.get("b"));
}

#[test]
fn a_declared_auto_layout_beats_the_arrangement_and_says_so() {
    let workspace = load(&MODEL.replace("include *", "include *\n            autoLayout lr"));
    let laid_out = layout::compute(
        &workspace,
        &workspace.views[0],
        &C4Metrics::default(),
        Some(&authored("a", 500.0, 600.0)),
    );
    assert_ne!(laid_out.boxes.get("a").unwrap().x, 500.0);
    assert!(laid_out.authored_positions_ignored);
}

#[test]
fn with_no_arrangement_at_all_nothing_is_reported_as_ignored() {
    let workspace = load(&MODEL.replace("include *", "include *\n            autoLayout lr"));
    let laid_out = layout::compute(&workspace, &workspace.views[0], &C4Metrics::default(), None);
    assert!(!laid_out.authored_positions_ignored);
}

#[test]
fn an_authored_position_for_something_not_on_the_view_is_ignored_quietly() {
    let workspace = load(MODEL);
    let laid_out = layout::compute(
        &workspace,
        &workspace.views[0],
        &C4Metrics::default(),
        Some(&authored("ghost", 500.0, 600.0)),
    );
    assert!(!laid_out.boxes.contains("ghost"));
    assert!(!laid_out.authored_positions_ignored);
}

// ---- C4LayoutSidecar.ContendedWrite.Tests.cs ----------------------------------------------------------------------

#[test]
#[cfg(windows)]
fn write_while_the_sidecar_cannot_be_read_refuses_rather_than_wiping_other_views() {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_SHARE_WRITE: u32 = 2;
    let (_folder, body) = body();
    sidecar::write(&body, "context", "a", 100.0, 200.0);
    {
        // A holder that blocks reads but not writes.
        let _holder = fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_WRITE)
            .open(sidecar::path_for(&body))
            .unwrap();
        let warning = sidecar::write(&body, "containers", "b", 300.0, 400.0);
        assert_ne!(warning, "");
    }
    assert_eq!(sidecar::read(&body, "context").get("a"), Some((100.0, 200.0)));
}

#[test]
fn write_over_a_nonsense_sidecar_still_proceeds() {
    let (_folder, body) = body();
    fs::write(sidecar::path_for(&body), "{ not json").unwrap();
    sidecar::write(&body, "context", "a", 1.0, 2.0);
    assert_eq!(sidecar::read(&body, "context").get("a"), Some((1.0, 2.0)));
}
