//! A SPARQL query diagram on the canvas with its module (etalii.adp spec 030, task T084): the standalone host's
//! `groups.rq` test query with its registration, opened from the parity fixtures, drawn as the module projected and
//! laid it out, a region dragged by its top-left into the registration's `layout:` block with its contents following,
//! an edge and the header refused a move, and the drag undone through the project's history. The query itself is
//! never written.

use std::fs;
use std::path::{Path, PathBuf};

use adp_diagram_sparql::registration_layout::read;
use adp_scene::scene::{Owner, Primitive};

use super::{Harness, texts_of};

fn groups() -> Harness {
    let folder: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/fixtures/w3c/sparql/groups");
    Harness::open_from(&folder, &["groups.adp", "groups.rq"], "groups.adp", "groups.adp")
}

#[test]
fn a_query_opens_as_its_joins_and_a_region_is_dragged_into_its_registration() {
    let mut harness = groups();
    assert!(harness.tab.session.module_view().is_some());
    harness.settle();
    let query = fs::read(harness.root.join("groups.rq")).unwrap();

    // The variables, the terms, the subquery, the frames with their labels, the badges and the header band.
    let scene = harness.tab.scene().expect("a scene");
    let texts = texts_of(scene);
    for label in [
        "?s",
        "?x",
        "?doubled",
        "true",
        "OPTIONAL",
        "UNION",
        "MINUS",
        "BIND(?x * 2 AS ?doubled)",
        "SELECT",
    ] {
        assert!(texts.contains(&label), "{label} is drawn: {texts:?}");
    }
    let patterns = scene
        .primitives
        .iter()
        .filter(|p| matches!(p, Primitive::Edge(e) if matches!(e.owner, Owner::Relation(_))))
        .count();
    assert_eq!(patterns, 11);
    // The projected variables, ?s and ?x, are marked so the header never has to be consulted.
    let marks = scene
        .primitives
        .iter()
        .filter(|p| matches!(p, Primitive::Edge(e) if !e.selectable))
        .count();
    assert_eq!(marks, 2);
    // Where the layout put them, as the standalone host's stream reported them.
    let union = harness.tab.bounds_of("region:where/union.0").unwrap();
    assert_eq!((union.x, union.y), (342.0, 384.0));
    let s = harness.tab.bounds_of("var:s").unwrap();
    assert_eq!((s.x, s.y, s.width, s.height), (500.0, 90.0, 170.0, 64.0));
    let branch = harness.tab.bounds_of("region:where/union.0/branch.1").unwrap();

    // A drag writes the frame's top-left into the registration's block, and its contents follow it.
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.move_by(host.workspace(root), &["region:where/union.0".to_owned()], (20.0, 40.0));
    let moved = harness.written();
    assert_eq!(
        moved,
        "w3c/sparql\r\nbody: groups.rq\r\nlayout:\r\n  region:where/union.0: 362 424\r\n"
    );
    harness.tab.changed();
    harness.settle();
    let after = harness.tab.bounds_of("region:where/union.0").unwrap();
    assert_eq!((after.x, after.y), (362.0, 424.0));
    let followed = harness.tab.bounds_of("region:where/union.0/branch.1").unwrap();
    assert_eq!((followed.x, followed.y), (branch.x + 20.0, branch.y + 40.0));

    // An edge is drawn between its ends and the header stays on top, so neither moves.
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.move_by(
        host.workspace(root),
        &["edge:var:s|ex:p|var:x|0".to_owned()],
        (40.0, 20.0),
    );
    assert_eq!(harness.written(), moved);

    // One undo takes the drag back, which leaves the registration without its block.
    assert!(harness.workspace().undo().is_done());
    assert_eq!(read(&harness.written()), read(&harness.original));
    assert_eq!(
        fs::read(harness.root.join("groups.rq")).unwrap(),
        query,
        "the query is never written"
    );

    harness.screenshot(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/screenshots/sparql.png"));
}
