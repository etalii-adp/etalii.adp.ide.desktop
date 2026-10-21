//! The persistence plugin `net.etalii.adp.ansible.folder` and the listing it learns the folder from (etalii.adp spec
//! 030, task T079): the standalone host's `AnsibleFolderPlugin.Tests.cs` and `AnsibleFolderListing.Tests.cs` at
//! `8b6ccfc`, with the same expectations. A test that needs a symbolic link is skipped where the platform refuses to
//! make one (Windows without developer mode), as it cannot say anything there.

mod common;

use std::fs;
use std::path::Path;
use std::rc::Rc;

use adp_cel::Value;
use adp_diagram_ansible_structure::definition::AnsibleDefinition;
use adp_diagram_ansible_structure::listing::AnsibleFolderListing;
use adp_diagram_ansible_structure::plugin::{AnsibleFolderPlugin, PLUGIN_ID};
use adp_fbl::model::{FblElement, codes};
use adp_fbl::plugins::{FolderBody, PersistencePlugin, PluginFile, PluginReadRequest, PluginReadResult};
use adp_fbl::{FblOptions, ModelChange, PlanResult};
use common::fixture;

fn plugin(folder: &Path) -> AnsibleFolderPlugin {
    AnsibleFolderPlugin::new(AnsibleFolderListing::new(folder), AnsibleDefinition::get().binding)
}

fn selected(root: &Path) -> Vec<PluginFile> {
    adp_fbl::routing::folder_files(&AnsibleDefinition::get().binding, root, None)
        .into_iter()
        .map(|f| PluginFile::new(f.relative_path, fs::read(f.full_path).unwrap()))
        .collect()
}

fn line(element: &FblElement) -> String {
    let mut line = format!(
        "{} {} {}:{}",
        element.type_name,
        element.id,
        element.file.as_deref().unwrap_or_default(),
        element.line
    );
    if element.is_relation {
        line.push_str(&format!(
            " {} -> {}",
            element.source.as_deref().unwrap_or_default(),
            element.target.as_deref().unwrap_or("(none)")
        ));
    }
    line
}

fn value(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::List(items) => format!("[{}]", items.iter().map(self::value).collect::<Vec<_>>().join(", ")),
        Value::Map(map) => format!(
            "{{{}}}",
            map.iter()
                .map(|(k, v)| format!("{k}={}", self::value(v)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        other => format!("{other:?}"),
    }
}

fn lines(result: &PluginReadResult) -> Vec<String> {
    result
        .elements
        .iter()
        .map(|e| {
            let attributes: Vec<String> = e.attributes.iter().map(|(k, v)| format!("{k}={}", value(v))).collect();
            format!("{} {}", line(e), attributes.join(", "))
        })
        .chain(result.findings.iter().map(|f| format!("{f:?}")))
        .collect()
}

fn body(root: &Path) -> FolderBody {
    let definition = AnsibleDefinition::get();
    FolderBody::open(
        root,
        definition.binding.clone(),
        Some(Rc::new(plugin(root))),
        Rc::new(FblOptions::default()),
        None,
    )
    .unwrap()
}

// ---- the plugin --------------------------------------------------------------------------------------------------------

#[test]
fn the_same_bytes_give_the_same_elements() {
    let root = fixture("infrastructure");
    let files = selected(&root);
    let first = plugin(&root).read(&PluginReadRequest {
        files: files.clone(),
        supplementary_entries: Vec::new(),
    });
    let second = plugin(&root).read(&PluginReadRequest {
        files,
        supplementary_entries: Vec::new(),
    });
    assert!(!first.elements.is_empty());
    assert_eq!(lines(&first), lines(&second));
}

#[test]
fn a_handed_file_set_with_no_disk_behind_it_reads() {
    let folder = tempfile::tempdir().unwrap();
    let nowhere = folder.path().join("not-there");
    let files = vec![
        PluginFile::new(
            "site.yml",
            b"- name: Web\r\n  hosts: web\r\n  roles:\r\n    - nginx\r\n".to_vec(),
        ),
        PluginFile::new("hosts", b"[web]\r\nweb-01\r\n".to_vec()),
    ];
    let result = plugin(&nowhere).read(&PluginReadRequest {
        files,
        supplementary_entries: Vec::new(),
    });
    assert!(!nowhere.exists());
    let read: Vec<String> = result.elements.iter().map(line).collect();
    assert_eq!(
        read,
        [
            "Playbook playbook:site.yml site.yml:0",
            "Inventory inventory:hosts hosts:0",
            "UsesRole edge:playbook:site.yml|UsesRole|nginx site.yml:4 playbook:site.yml -> (none)",
            "Targets edge:playbook:site.yml|Targets|web|inventory:hosts :1 playbook:site.yml -> inventory:hosts",
        ]
    );
    assert!(result.findings.is_empty());
}

/// A file that does not parse is one finding at its file and the parser's line.
#[test]
fn a_file_that_does_not_parse_is_one_finding_with_the_parsers_words_and_line() {
    let folder = tempfile::tempdir().unwrap();
    let bytes = b"- name: Web\r\n  hosts: all\r\n  roles: [web\r\n".to_vec();
    let result = plugin(&folder.path().join("not-there")).read(&PluginReadRequest {
        files: vec![PluginFile::new("site.yml", bytes)],
        supplementary_entries: Vec::new(),
    });
    let [finding] = result.findings.as_slice() else {
        panic!("one finding: {:?}", result.findings)
    };
    assert_eq!(finding.code, codes::UNPARSEABLE);
    let location = finding.location.as_ref().unwrap();
    assert_eq!(location.file, "site.yml");
    assert!(location.line > 0, "line {}", location.line);
    assert!(!finding.message.is_empty());
    assert!(result.elements.is_empty());
}

#[test]
fn a_change_is_refused_with_the_bindings_reason_and_the_plugin_is_never_asked_to_plan() {
    let body = body(&fixture("infrastructure"));
    let mut attributes = adp_cel::Map::default();
    attributes.insert("name", Value::String("renamed".into()));
    let result = body.plan(&ModelChange::Set {
        id: "role:nginx".into(),
        attributes,
    });
    assert!(body.is_read_only());
    match result {
        PlanResult::Refused(reason) => assert_eq!(reason, AnsibleDefinition::get().write_refusal()),
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_folders_it_counts_in_are_what_the_folder_body_watches() {
    let root = fixture("infrastructure");
    let dependencies = body(&root).dependencies();
    let full = std::path::absolute(&root).unwrap();
    assert!(dependencies.contains(&full.join("roles")), "{dependencies:?}");
    assert!(dependencies.contains(&full.join("ansible.cfg")), "{dependencies:?}");
}

#[test]
fn the_plugin_is_the_one_the_binding_names() {
    assert_eq!(
        AnsibleDefinition::get()
            .binding
            .plugin
            .as_ref()
            .map(|p| p.plugin.as_str()),
        Some(PLUGIN_ID)
    );
}

// ---- the listing -------------------------------------------------------------------------------------------------------

fn write(root: &Path, relative: &str, text: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

#[test]
fn an_empty_folder_is_there_and_holds_no_file() {
    let folder = tempfile::tempdir().unwrap();
    fs::create_dir_all(folder.path().join("roles/empty")).unwrap();
    let listing = AnsibleFolderListing::new(folder.path());
    assert!(listing.folder_exists("roles/empty"));
    assert!(!listing.folder_exists("roles/absent"));
    assert_eq!(listing.folders("roles"), ["empty"]);
    assert_eq!(listing.file_count("roles/empty"), 0);
}

#[test]
fn a_folder_that_is_not_there_lists_nothing() {
    let folder = tempfile::tempdir().unwrap();
    let listing = AnsibleFolderListing::new(&folder.path().join("not-there"));
    assert!(!listing.folder_exists(""));
    assert!(listing.folders("roles").is_empty());
    assert_eq!(listing.file_count(""), 0);
    assert!(listing.linked_files(&AnsibleDefinition::get().binding).is_empty());
}

#[cfg(windows)]
fn link_file(target: &Path, link: &Path) -> bool {
    std::os::windows::fs::symlink_file(target, link).is_ok()
}

#[cfg(not(windows))]
fn link_file(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

#[cfg(windows)]
fn link_folder(target: &Path, link: &Path) -> bool {
    std::os::windows::fs::symlink_dir(target, link).is_ok()
}

#[cfg(not(windows))]
fn link_folder(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

#[test]
fn a_linked_folder_found_by_listing_is_skipped_and_one_named_by_the_layout_is_entered() {
    let folder = tempfile::tempdir().unwrap();
    let root = folder.path();
    write(root, "elsewhere/role/tasks/main.yml", "- debug: msg=hi\r\n");
    write(root, "elsewhere/plays/deploy.yml", "- hosts: all\r\n");
    write(root, "roles/real/tasks/main.yml", "- debug: msg=hi\r\n");
    if !link_folder(&root.join("elsewhere/role"), &root.join("roles/linked"))
        || !link_folder(&root.join("elsewhere/plays"), &root.join("playbooks"))
    {
        eprintln!("skipped: this platform makes no symbolic link here");
        return;
    }
    let listing = AnsibleFolderListing::new(root);
    let linked = listing.linked_files(&AnsibleDefinition::get().binding);
    assert_eq!(listing.folders("roles"), ["real"]);
    let paths: Vec<&str> = linked.iter().map(|f| f.relative_path.as_str()).collect();
    assert_eq!(paths, ["playbooks/deploy.yml"]);
    assert_eq!(linked[0].bytes, b"- hosts: all\r\n");
}

#[test]
fn a_linked_file_is_read_with_its_bytes_and_a_file_that_is_no_link_is_left_to_the_folder_subject() {
    let folder = tempfile::tempdir().unwrap();
    let root = folder.path();
    write(root, "elsewhere/play.yml", "- hosts: web\r\n");
    write(root, "site.yml", "- hosts: all\r\n");
    if !link_file(&root.join("elsewhere/play.yml"), &root.join("linked.yml")) {
        eprintln!("skipped: this platform makes no symbolic link here");
        return;
    }
    let linked = AnsibleFolderListing::new(root).linked_files(&AnsibleDefinition::get().binding);
    let [file] = linked.as_slice() else {
        panic!("one: {linked:?}")
    };
    assert_eq!(file.relative_path, "linked.yml");
    assert_eq!(file.bytes, b"- hosts: web\r\n");
}

#[test]
fn a_file_that_cannot_be_read_is_counted_so_counting_reads_nothing() {
    let folder = tempfile::tempdir().unwrap();
    let root = folder.path();
    write(root, "roles/web/templates/site.conf.j2", "server {}");
    if !link_file(&root.join("nothing-here.j2"), &root.join("roles/web/templates/gone.j2")) {
        eprintln!("skipped: this platform makes no symbolic link here");
        return;
    }
    assert_eq!(AnsibleFolderListing::new(root).file_count("roles/web/templates"), 2);
}
