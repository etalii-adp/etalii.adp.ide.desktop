//! An Ansible structure diagram on the canvas with its module (etalii.adp spec 030, task T079): the standalone host's
//! `infrastructure` example, a folder registered by its `.adp`, opened from the parity fixtures, drawn on the module's
//! ranked layout with its band of inventories, a box dragged by its top-left into the registration's `layout:` block,
//! a relation refused a move, and the drag undone through the project's history.

use std::fs;
use std::path::{Path, PathBuf};

use adp_diagram_ansible_structure::registration_layout::number;
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

pub(super) fn infrastructure() -> Harness {
    let folder: PathBuf =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/fixtures/ansible/structure/infrastructure");
    let mut files = Vec::new();
    files_under(&folder, &folder, &mut files);
    let names: Vec<&str> = files.iter().map(String::as_str).collect();
    Harness::open_from(&folder, &names, ".adp", ".adp")
}

#[test]
fn an_ansible_folder_opens_on_its_ranked_layout_and_a_box_is_dragged_into_its_registration() {
    let mut harness = infrastructure();
    assert!(harness.tab.session.module_view().is_some());
    harness.settle();

    // The playbooks, roles, the included task file and the band of inventories, as the module laid them out.
    let scene = harness.tab.scene().expect("a scene");
    let texts = texts_of(scene);
    for label in [
        "site.yml",
        "webservers.yml",
        "nginx",
        "tls.yml",
        "production",
        "group_vars",
        "include_tasks when nginx_tls_enabled | default(false)",
    ] {
        assert!(texts.contains(&label), "{label} is drawn: {texts:?}");
    }
    let relations = scene
        .primitives
        .iter()
        .filter(|p| matches!(p, Primitive::Edge(e) if matches!(e.owner, Owner::Relation(_))))
        .count();
    assert_eq!(relations, 13);
    // The story reads left to right, the inventories in a band beneath it.
    let site = harness
        .tab
        .bounds_of("playbook:site.yml")
        .expect("the entry playbook is drawn");
    let nginx = harness.tab.bounds_of("role:nginx").expect("the role is drawn");
    let production = harness.tab.bounds_of("inventory:inventories/production").unwrap();
    assert!(site.right() < nginx.x, "{site:?} {nginx:?}");
    assert!(production.y > nginx.bottom(), "{production:?} {nginx:?}");
    assert_eq!((0.0, 0.0, 32.0), (site.x, site.y, site.height));

    // A drag writes where the box's top-left went, into the registration and nowhere else.
    let original = harness.written();
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.move_by(host.workspace(root), &["role:nginx".to_owned()], (40.0, 20.0));
    let moved = harness.written();
    assert!(
        moved.ends_with(&format!(
            "layout:\r\n  role:nginx: {} {}\r\n",
            number(nginx.x + 40.0),
            number(nginx.y + 20.0)
        )),
        "{moved}"
    );
    harness.tab.changed();
    harness.settle();
    let after = harness.tab.bounds_of("role:nginx").unwrap();
    // Drawn where the registration says, which keeps three decimals.
    assert!(
        (after.x - (nginx.x + 40.0)).abs() < 1e-3 && (after.y - (nginx.y + 20.0)).abs() < 1e-3,
        "{after:?}"
    );

    // A relation follows its ends, so it is never moved.
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.move_by(
        host.workspace(root),
        &["edge:role:nginx|DependsOn|common".to_owned()],
        (40.0, 20.0),
    );
    assert_eq!(harness.written(), moved);

    // One undo takes the drag back, byte for byte.
    assert!(harness.workspace().undo().is_done());
    assert_eq!(harness.written(), original);
    assert_eq!(fs::read_to_string(&harness.body).unwrap(), harness.original);

    harness.screenshot(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/screenshots/ansible-structure.png"));
}
