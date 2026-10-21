//! The folder becoming a project (etalii.adp spec 030, task T079), read against the committed fixture trees: the
//! standalone host's `AnsibleProjectReader.Tests.cs` at `8b6ccfc`, with the same expectations.

mod common;

use std::fs;

use adp_diagram_ansible_structure::definition::AnsibleDefinition;
use adp_diagram_ansible_structure::model::{AnsibleDirective, AnsibleDirectiveKind, AnsibleProject};
use adp_diagram_ansible_structure::reader;
use common::{copy_tree, fixture, read};

fn names<T>(items: &[T], name: impl Fn(&T) -> &str) -> Vec<String> {
    items.iter().map(|i| name(i).to_owned()).collect()
}

// ---- what it recognises ----------------------------------------------------------------------------------------------

#[test]
fn the_best_practice_tree_reads_every_node_kind() {
    let project = read("infrastructure");
    assert!(
        std::path::Path::new(&project.folder_path).is_absolute(),
        "{}",
        project.folder_path
    );
    assert!(project.folder_path.ends_with("infrastructure"));
    let mut playbooks = names(&project.playbooks, |p| &p.name);
    playbooks.sort();
    assert_eq!(playbooks, ["dbservers.yml", "site.yml", "webservers.yml"]);
    assert_eq!(names(&project.roles, |r| &r.name), ["common", "nginx", "postgres"]);
    assert_eq!(names(&project.inventories, |i| &i.name), ["production", "staging"]);
    assert!(project.failures.is_empty());
}

#[test]
fn a_play_carries_its_name_hosts_and_roles() {
    let project = read("infrastructure");
    let playbook = project.playbooks.iter().find(|p| p.name == "webservers.yml").unwrap();
    let [play] = playbook.plays.as_slice() else {
        panic!("one play")
    };
    assert_eq!(play.name, "Configure the web tier");
    assert_eq!(play.hosts, "web");
    let roles: Vec<&str> = play
        .directives
        .iter()
        .filter(|d| d.kind == AnsibleDirectiveKind::Roles)
        .map(|d| d.target.as_str())
        .collect();
    assert_eq!(roles, ["common", "nginx"]);
}

#[test]
fn a_roles_entry_in_mapping_form_is_read_and_keeps_its_condition() {
    let project = read("infrastructure");
    let playbook = project.playbooks.iter().find(|p| p.name == "dbservers.yml").unwrap();
    let [play] = playbook.plays.as_slice() else {
        panic!("one play")
    };
    let postgres: Vec<&AnsibleDirective> = play.directives.iter().filter(|d| d.target == "postgres").collect();
    assert_eq!(postgres.len(), 1);
    assert_eq!(postgres[0].condition, "postgres_enabled | default(true)");
}

#[test]
fn import_playbook_is_a_playbooks_sibling_not_a_plays_child() {
    let project = read("infrastructure");
    let site = project.playbooks.iter().find(|p| p.name == "site.yml").unwrap();
    assert!(site.plays.is_empty());
    assert_eq!(names(&site.imports, |i| &i.target), ["webservers.yml", "dbservers.yml"]);
    assert!(
        site.imports
            .iter()
            .all(|i| i.kind == AnsibleDirectiveKind::ImportPlaybook)
    );
}

#[test]
fn a_roles_task_includes_are_read_with_their_condition_and_line() {
    let project = read("infrastructure");
    let nginx = project.role("nginx").unwrap();
    let [include] = nginx.task_includes.as_slice() else {
        panic!("one include")
    };
    assert_eq!(include.kind, AnsibleDirectiveKind::IncludeTasks);
    assert_eq!(include.target, "tls.yml");
    assert_eq!(include.condition, "nginx_tls_enabled | default(false)");
    assert_eq!(include.declared_in, "roles/nginx/tasks/main.yml");
    assert!(include.line > 0);
}

#[test]
fn a_fully_qualified_directive_name_is_recognised() {
    assert_eq!(read("infrastructure").role("nginx").unwrap().task_includes.len(), 1);
}

#[test]
fn meta_dependencies_are_read_in_both_shapes() {
    let project = read("infrastructure");
    assert_eq!(
        names(&project.role("nginx").unwrap().dependencies, |d| &d.target),
        ["common"]
    );
    assert_eq!(
        names(&project.role("postgres").unwrap().dependencies, |d| &d.target),
        ["common"]
    );
    assert!(project.role("common").unwrap().dependencies.is_empty());
}

#[test]
fn a_roles_contents_are_summarised() {
    let nginx = read("infrastructure").role("nginx").unwrap().contents;
    assert_eq!(nginx.task_files, 2);
    assert_eq!(nginx.handlers, 1);
    assert_eq!(nginx.templates, 1);
    assert!(nginx.has_meta);
    assert!(!nginx.is_hollow());
    assert_eq!(nginx.defaults, 0);
}

#[test]
fn an_inventory_carries_its_groups_and_variable_folders() {
    let project = read("infrastructure");
    let production = project.inventories.iter().find(|i| i.name == "production").unwrap();
    assert_eq!(names(&production.groups, |g| &g.name), ["web", "db"]);
    assert_eq!(
        production.groups.iter().find(|g| g.name == "web").unwrap().host_count,
        2
    );
    assert_eq!(
        names(&production.variable_folders, |v| &v.name),
        ["group_vars", "host_vars"]
    );
    assert_eq!(
        production
            .variable_folders
            .iter()
            .find(|v| v.name == "group_vars")
            .unwrap()
            .file_count,
        2
    );
}

#[test]
fn a_root_ini_inventory_carries_its_groups() {
    let project = read("ini-inventory");
    let inventory = project.inventories.iter().find(|i| i.name == "hosts").unwrap();
    assert_eq!(names(&inventory.groups, |g| &g.name), ["web", "db", "app"]);
    let count = |name: &str| inventory.groups.iter().find(|g| g.name == name).unwrap().host_count;
    assert_eq!(count("web"), 2);
    assert_eq!(count("db"), 1);
    // A :children group holds the sum of the groups it gathers; a :vars section adds no group.
    assert_eq!(count("app"), 3);
}

#[test]
fn the_annotations_are_recorded_rather_than_drawn() {
    let project = read("infrastructure");
    assert!(project.annotations.contains(&"ansible.cfg".to_owned()));
    assert!(project.annotations.contains(&"collections/requirements.yml".to_owned()));
}

// ---- what it ignores, and how it degrades ------------------------------------------------------------------------------

#[test]
fn content_that_is_not_ansible_is_ignored_without_complaint() {
    let project = read("infrastructure");
    assert!(project.failures.is_empty());
    assert!(!project.annotations.iter().any(|a| a.contains("Makefile")));
}

#[test]
fn a_yaml_file_that_is_a_mapping_at_the_top_is_not_a_playbook() {
    assert!(
        !read("infrastructure")
            .playbooks
            .iter()
            .any(|p| p.name == "topology.yml")
    );
}

#[test]
fn an_unparsable_file_degrades_only_itself() {
    let project = read("broken");
    let [failure] = project.failures.as_slice() else {
        panic!("one failure: {:?}", project.failures)
    };
    assert_eq!(failure.relative_path, "unparsable.yml");
    assert!(!project.playbooks.is_empty());
    assert!(!project.roles.is_empty());
}

#[test]
fn a_hollow_role_is_still_a_role_and_visibly_hollow() {
    let project = read("broken");
    let hollow = project.role("hollow").unwrap();
    assert!(hollow.contents.is_hollow());
    assert!(hollow.task_files.is_empty());
}

#[test]
fn an_expression_target_is_marked_as_one_rather_than_taken_literally() {
    let project = read("broken");
    let deploy = project.playbooks.iter().find(|p| p.name == "deploy.yml").unwrap();
    let expressions: Vec<&AnsibleDirective> = deploy
        .plays
        .iter()
        .flat_map(|p| &p.directives)
        .filter(|d| d.is_expression())
        .collect();
    assert_eq!(expressions.len(), 1);
    assert_eq!(expressions[0].target, "{{ role_name }}");
}

#[test]
fn an_unrecognised_layout_reads_as_nothing_but_its_annotation() {
    let project = read("unconventional");
    assert!(project.is_empty());
    assert!(project.failures.is_empty());
    assert_eq!(project.annotations, ["ansible.cfg"]);
}

#[test]
fn a_folder_that_is_not_there_reads_as_empty_rather_than_failing() {
    let project = read("no-such-folder");
    assert!(project.is_empty());
    assert!(project.failures.is_empty());
}

// ---- determinism -------------------------------------------------------------------------------------------------------

#[test]
fn the_same_tree_reads_to_the_same_model_twice() {
    assert_eq!(describe(&read("infrastructure")), describe(&read("infrastructure")));
}

#[test]
fn the_order_of_every_collection_is_ordinal_rather_than_the_filesystems() {
    let project = read("infrastructure");
    let sorted = |mut v: Vec<String>| {
        v.sort();
        v
    };
    let roles = names(&project.roles, |r| &r.name);
    assert_eq!(sorted(roles.clone()), roles);
    let inventories = names(&project.inventories, |i| &i.name);
    assert_eq!(sorted(inventories.clone()), inventories);
    let tasks = names(&project.role("nginx").unwrap().task_files, |t| &t.name);
    assert_eq!(sorted(tasks.clone()), tasks);
}

// ---- what the binding selects and the reader passes over ---------------------------------------------------------------

/// `inventories/*/*` selects every file of an environment, and the reader passes over what is not an inventory.
#[test]
fn a_note_beside_an_inventory_is_selected_and_changes_nothing() {
    let copy = tempfile::tempdir().unwrap();
    let without = copy.path().join("without");
    let with = copy.path().join("with");
    copy_tree(&fixture("infrastructure"), &without);
    copy_tree(&fixture("infrastructure"), &with);
    fs::write(
        with.join("inventories/production/notes.txt"),
        "[notes]\r\nremember-to-rotate-certs\r\n",
    )
    .unwrap();
    let binding = AnsibleDefinition::get().binding;

    let selected: Vec<String> = adp_fbl::routing::folder_files(&binding, &with, None)
        .into_iter()
        .map(|f| f.relative_path)
        .collect();
    let read_with = reader::read_folder(&with, &binding);

    assert!(selected.contains(&"inventories/production/notes.txt".to_owned()));
    assert_eq!(describe(&reader::read_folder(&without, &binding)), describe(&read_with));
}

/// The whole project as one ordered string: every node, every directive, every count, in the order read.
fn describe(project: &AnsibleProject) -> String {
    let mut lines = Vec::new();
    for playbook in &project.playbooks {
        lines.push(format!("playbook {}", playbook.relative_path));
        lines.extend(playbook.imports.iter().map(directive));
        for play in &playbook.plays {
            lines.push(format!(
                "  play {} '{}' hosts={} line={}",
                play.index, play.name, play.hosts, play.line
            ));
            lines.extend(play.directives.iter().map(directive));
        }
    }
    for role in &project.roles {
        lines.push(format!(
            "role {} at {} contents={:?} hollow={}",
            role.name,
            role.relative_path,
            role.contents,
            role.contents.is_hollow()
        ));
        lines.extend(role.task_files.iter().map(|f| format!("  task {}", f.relative_path)));
        lines.extend(role.dependencies.iter().map(directive));
        lines.extend(role.task_includes.iter().map(directive));
    }
    for inventory in &project.inventories {
        lines.push(format!("inventory {} at {}", inventory.name, inventory.relative_path));
        lines.extend(
            inventory
                .groups
                .iter()
                .map(|g| format!("  group {} hosts={}", g.name, g.host_count)),
        );
        lines.extend(
            inventory
                .variable_folders
                .iter()
                .map(|f| format!("  vars {} files={}", f.relative_path, f.file_count)),
        );
    }
    lines.extend(project.annotations.iter().map(|a| format!("annotation {a}")));
    lines.extend(
        project
            .failures
            .iter()
            .map(|f| format!("failure {}:{}", f.relative_path, f.line)),
    );
    lines.join("\n")
}

fn directive(d: &AnsibleDirective) -> String {
    format!(
        "    {:?} -> '{}' when='{}' in {}:{} dynamic={} expression={}",
        d.kind,
        d.target,
        d.condition,
        d.declared_in,
        d.line,
        d.is_dynamic(),
        d.is_expression()
    )
}
