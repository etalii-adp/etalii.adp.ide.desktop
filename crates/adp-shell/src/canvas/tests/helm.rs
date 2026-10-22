//! A Helm chart diagram on the canvas with its module (etalii.adp spec 030, task T080): the standalone host's `nginx`
//! example, a chart folder registered by its `.adp`, opened from the parity fixtures, drawn on the module's bands with
//! the positions its registration stores, a box dragged by its top-left into the registration's `layout:` block, a
//! relation refused a move, and the drag undone through the project's history.

use std::fs;
use std::path::{Path, PathBuf};

use adp_diagram_helm_chart::registration_layout::read;
use adp_scene::scene::{Owner, Primitive};

use super::{Harness, texts_of};

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

fn nginx() -> Harness {
    let folder: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/fixtures/helm/chart/nginx");
    let mut files = Vec::new();
    files_under(&folder, &folder, &mut files);
    let names: Vec<&str> = files.iter().map(String::as_str).collect();
    Harness::open_from(&folder, &names, ".adp", ".adp")
}

#[test]
fn a_helm_chart_opens_on_its_bands_and_a_box_is_dragged_into_its_registration() {
    let mut harness = nginx();
    assert!(harness.tab.session.module_view().is_some());
    harness.settle();

    // The chart, its values stack, its templates, its dependency and what charts/ holds, as the module laid them out.
    let scene = harness.tab.scene().expect("a scene");
    let texts = texts_of(scene);
    for label in [
        "nginx",
        "values.yaml",
        "values-dev.yaml",
        "deployment.yaml",
        "_helpers.tpl",
        "common",
        "Chart.lock",
        "2.x.x",
    ] {
        assert!(texts.contains(&label), "{label} is drawn: {texts:?}");
    }
    let relations = scene
        .primitives
        .iter()
        .filter(|p| matches!(p, Primitive::Edge(e) if matches!(e.owner, Owner::Relation(_))))
        .count();
    assert_eq!(relations, 99);
    // The default values layer sits where the band put it, the chart where the registration stores it.
    let values = harness.tab.bounds_of("values:values.yaml").unwrap();
    assert_eq!(
        (values.x, values.y, values.width, values.height),
        (320.0, 40.0, 200.0, 72.0)
    );
    let chart = harness.tab.bounds_of("chart").unwrap();
    assert_eq!((chart.x, chart.y), (1327.665, 566.461));

    // A drag writes where the box's top-left went, into the registration's block and nowhere else.
    let original = harness.written();
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.move_by(host.workspace(root), &["values:values.yaml".to_owned()], (40.0, 20.0));
    let moved = harness.written();
    assert!(
        moved.ends_with("  values:values-prod.yaml: 37.267 128.195\r\n  values:values.yaml: 360 60\r\n"),
        "{moved}"
    );
    harness.tab.changed();
    harness.settle();
    let after = harness.tab.bounds_of("values:values.yaml").unwrap();
    assert_eq!((after.x, after.y), (360.0, 60.0));

    // A relation follows its ends, so it is never moved.
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.move_by(
        host.workspace(root),
        &["edge:values:values-dev.yaml|Overrides|values:values.yaml|".to_owned()],
        (40.0, 20.0),
    );
    assert_eq!(harness.written(), moved);

    // One undo takes the drag back: the registration's first line as it was, the block rewritten whole in CRLF, as
    // the standalone host's `RegistrationLayout` writes it.
    assert!(harness.workspace().undo().is_done());
    assert_eq!(
        harness.written(),
        original.replace("\n", "\r\n").replacen("\r\n", "\n", 1)
    );
    assert_eq!(read(&harness.written()), read(&harness.original));

    harness.screenshot(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/screenshots/helm-chart.png"));
}
