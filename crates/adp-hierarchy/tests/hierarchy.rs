//! The workspace tree (etalii.adp spec 030, Story 1, FR-013, FR-023, task T023), with the standalone host's nesting
//! rules (adp-file-nesting) and name rules (`EntryNameRules`) at `8b6ccfc`.

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use adp_hierarchy::{
    Change, Hierarchy, Kind, Node, RegistrationName, Watcher, compare_registrations, nest, validate_name,
};

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn names(nodes: &[Node]) -> Vec<String> {
    nodes.iter().map(|n| n.name.clone()).collect()
}

#[test]
fn registration_names_parse_as_the_standalone_host_reads_them() {
    let folder = RegistrationName::parse(".adp").unwrap();
    assert!(folder.folder_scoped);
    let single = RegistrationName::parse("ideas.mm.adp").unwrap();
    assert_eq!(
        (
            single.full_base.as_str(),
            single.subject_base.as_str(),
            single.qualifier.as_str()
        ),
        ("ideas.mm", "ideas", "mm")
    );
    let plain = RegistrationName::parse("ideas.adp").unwrap();
    assert!(!plain.is_qualified());
    assert_eq!(RegistrationName::parse(".hidden.adp").unwrap().subject_base, ".hidden");
    assert!(RegistrationName::parse("ideas.mm").is_none());
}

#[test]
fn a_registration_nests_under_its_subject_file_never_a_folder() {
    let siblings = vec![
        ("templates".to_string(), true),
        ("templates.yml".to_string(), false),
        ("templates.adp".to_string(), false),
        (".adp".to_string(), false),
        ("gone.adp".to_string(), false),
        ("other.adp".to_string(), false),
    ];
    let placed = nest(&siblings, |name| match name {
        "templates.adp" => Some("templates.yml".into()),
        "gone.adp" => Some("gone.mm".into()),
        _ => None,
    });
    let find = |name: &str| placed.iter().find(|n| n.name == name).unwrap();
    assert_eq!(find("templates.adp").subject.as_deref(), Some("templates.yml"));
    assert_eq!(find(".adp").subject, None);
    assert!(find("gone.adp").orphan);
    assert!(!find("other.adp").orphan && find("other.adp").subject.is_none());
}

#[test]
fn several_registrations_of_one_subject_order_the_unqualified_first() {
    let mut names = vec!["map.wardley.adp", "map.adp", "map.c4.adp"];
    names.sort_by(|a, b| compare_registrations(a, b));
    assert_eq!(names, ["map.adp", "map.c4.adp", "map.wardley.adp"]);
}

#[test]
fn the_tree_lists_folders_first_and_nests_registrations_under_their_files() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "b.md", "b");
    write(root.path(), "a/x.txt", "x");
    write(root.path(), "ideas.mm", "<map/>");
    write(root.path(), "ideas.adp", "freeplane/mindmap\nbody: ideas.mm\n");
    write(root.path(), "ideas.second.adp", "freeplane/mindmap\nbody: ideas.mm\n");
    write(root.path(), "a/.adp", "etalii/knowledge\n");
    write(root.path(), ".git/HEAD", "ref");
    let tree = Hierarchy::scan(root.path()).unwrap();
    assert_eq!(names(&tree.children), ["a", "b.md", "ideas.mm"]);
    let ideas = &tree.children[2];
    assert_eq!(names(&ideas.children), ["ideas.adp", "ideas.second.adp"]);
    assert!(ideas.children.iter().all(|r| r.kind == Kind::Registration));
    let a = &tree.children[0];
    assert_eq!(a.kind, Kind::Folder);
    assert_eq!(names(&a.children), [".adp", "x.txt"]);
    assert_eq!(a.children[0].kind, Kind::Registration);
}

#[test]
fn a_registration_without_a_body_header_finds_its_subject_by_name() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "roadmap.ghg", "phases: []");
    write(root.path(), "roadmap.ghg.adp", "gartner/hypecycle-graph\n");
    write(root.path(), "plan.tml", "x");
    write(root.path(), "plan.adp", "generic/timeline\n");
    let tree = Hierarchy::scan(root.path()).unwrap();
    let ghg = tree.children.iter().find(|n| n.name == "roadmap.ghg").unwrap();
    assert_eq!(names(&ghg.children), ["roadmap.ghg.adp"]);
    // `plan.adp` names no body and no file is called `plan`, so it stays where it is, unnested and not an orphan.
    let plan = tree.children.iter().find(|n| n.name == "plan.adp").unwrap();
    assert!(!plan.orphan);
}

#[test]
fn names_are_checked_as_the_standalone_host_checks_them() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "taken.md", "x");
    assert_eq!(validate_name(" ", root.path(), None), Err("Enter a name.".into()));
    assert_eq!(
        validate_name("a/b", root.path(), None),
        Err("A name cannot contain a path or any of \\ / : * ? \" < > |".into())
    );
    assert_eq!(
        validate_name("..", root.path(), None),
        Err("A name cannot contain a path or any of \\ / : * ? \" < > |".into())
    );
    assert_eq!(
        validate_name("taken.md", root.path(), None),
        Err("An item named 'taken.md' already exists in this folder.".into())
    );
    assert_eq!(
        validate_name("taken.md", root.path(), Some("taken.md")),
        Err("Enter a name that differs from the current one.".into())
    );
    assert_eq!(validate_name("Taken.md", root.path(), Some("taken.md")), Ok(()));
    assert_eq!(validate_name("new.md", root.path(), None), Ok(()));
}

#[test]
fn changes_on_disk_arrive_as_changes() {
    let root = tempfile::tempdir().unwrap();
    let watcher = Watcher::start(root.path()).unwrap();
    write(root.path(), "new.md", "x");
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut seen = Vec::new();
    while Instant::now() < deadline {
        if let Ok(change) = watcher.changes().recv_timeout(Duration::from_millis(200)) {
            seen.push(change);
            if seen
                .iter()
                .any(|c| matches!(c, Change::Added(p) | Change::Modified(p) if p.ends_with("new.md")))
            {
                return;
            }
        }
    }
    panic!("no change for new.md arrived: {seen:?}");
}
