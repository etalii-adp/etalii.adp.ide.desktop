//! A registered Ansible folder opened in a project, and what is asked of it (etalii.adp spec 030, task T079): the
//! standalone host's `AnsibleSession.Tests.cs`, `AnsibleContextPropertyProvider.Tests.cs` and
//! `AnsibleContextSourceResolver.Tests.cs` at `8b6ccfc`, with the same expectations. The standalone host's sessions
//! heard the folder through a watcher; here the project's reconciliation hands a changed path to the module.

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_context::PropertyRow;
use adp_core::catalogue::Catalogue;
use adp_diagram::{DiagramDelta, DiagramElement, DiagramSession, DiagramTools, DiagramViewport, DiagramWorkspace};
use adp_diagram_ansible_structure::graph::AnsibleGraph;
use adp_diagram_ansible_structure::model::AnsibleEdgeKind;
use adp_diagram_ansible_structure::view::EDGE_REFUSAL;
use adp_diagram_ansible_structure::{mapper, registration_layout};
use adp_disl::DislPluginFunctions;
use common::{fixture, read, registered};

fn tools() -> DiagramTools {
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
    adp_diagram_ansible_structure::register(DiagramTools::new(Catalogue::load(&vendor), |_| {
        DislPluginFunctions::none()
    }))
}

struct Opened {
    _folder: tempfile::TempDir,
    root: PathBuf,
    registration: PathBuf,
    workspace: DiagramWorkspace,
    session: DiagramSession,
}

fn open_registered(folder: tempfile::TempDir, registration: PathBuf) -> Opened {
    let root = folder.path().to_path_buf();
    let mut workspace = DiagramWorkspace::new(&root);
    let name = registration.file_name().unwrap().to_owned();
    let (opened, session) = workspace.open(&mut tools(), Path::new(&name)).unwrap();
    assert_eq!(opened.origin, "ansible/structure");
    assert_eq!(opened.body_path, registration);
    Opened {
        _folder: folder,
        root,
        registration,
        workspace,
        session,
    }
}

fn open(fixture_name: &str) -> Opened {
    let (folder, registration) = registered(fixture_name);
    open_registered(folder, registration)
}

impl Opened {
    fn delivered(&mut self) -> Vec<DiagramElement> {
        match self
            .session
            .baseline(self.workspace.documents())
            .unwrap()
            .into_iter()
            .next()
        {
            Some(DiagramDelta::Add(elements)) => elements,
            other => panic!("{other:?}"),
        }
    }

    fn rows(&self, id: &str) -> Vec<PropertyRow> {
        self.session.properties(self.workspace.documents(), &[id])
    }

    fn stored(&self) -> std::collections::BTreeMap<String, (f64, f64)> {
        registration_layout::read(&fs::read_to_string(&self.registration).unwrap())
    }

    fn snapshot(&self) -> Vec<(PathBuf, Vec<u8>)> {
        let mut files = Vec::new();
        let mut pending = vec![self.root.clone()];
        while let Some(folder) = pending.pop() {
            for entry in fs::read_dir(folder).unwrap().flatten() {
                if entry.path().is_dir() {
                    pending.push(entry.path());
                } else {
                    files.push((entry.path(), fs::read(entry.path()).unwrap()));
                }
            }
        }
        files.sort();
        files
    }
}

fn row<'a>(rows: &'a [PropertyRow], id: &str) -> &'a PropertyRow {
    rows.iter()
        .find(|r| r.id == id)
        .unwrap_or_else(|| panic!("no row {id}: {rows:?}"))
}

fn groups(rows: &[PropertyRow]) -> Vec<String> {
    let mut groups: Vec<String> = Vec::new();
    for row in rows {
        if !groups.contains(&row.group) {
            groups.push(row.group.clone());
        }
    }
    groups
}

// ---- the subject is the folder -----------------------------------------------------------------------------------------

#[test]
fn the_module_resolves_the_subject_as_the_folder_the_registration_sits_in() {
    let mut opened = open("infrastructure");
    let elements = opened.delivered();
    assert!(elements.iter().any(|e| e.id == "role:nginx"));
    assert!(elements.iter().any(|e| e.id == "playbook:site.yml"));
    assert!(opened.session.module_view().is_some());
}

#[test]
fn a_folders_own_registration_named_dot_adp_opens_it() {
    let folder = tempfile::tempdir().unwrap();
    common::copy_tree(&fixture("infrastructure"), folder.path());
    let registration = folder.path().join(".adp");
    fs::write(&registration, "ansible/structure\n").unwrap();
    let mut opened = open_registered(folder, registration);
    assert_eq!(opened.delivered().len(), 25);
}

#[test]
fn every_node_and_relation_is_delivered_with_its_kind() {
    let mut opened = open("infrastructure");
    let elements = opened.delivered();
    let graph = AnsibleGraph::derive(&read("infrastructure"));
    assert_eq!(elements.len(), graph.nodes.len() + graph.edges.len());
    let nginx = elements.iter().find(|e| e.id == "role:nginx").unwrap();
    assert_eq!(nginx.element_type, mapper::ROLE_TYPE);
    assert_eq!(nginx.attribute("name"), "nginx");
    assert_eq!(nginx.attribute("contents.taskFiles"), "2");
    let include = elements
        .iter()
        .find(|e| e.id == "edge:role:nginx|IncludesTasks|tls.yml")
        .unwrap();
    assert_eq!(include.element_type, mapper::EDGE_TYPE);
    assert_eq!(include.attribute("directive"), "include_tasks");
    assert_eq!(include.attribute("dynamic"), "true");
    assert_eq!(include.target.as_deref(), Some("taskfile:roles/nginx/tasks/tls.yml"));
}

// ---- the viewport ------------------------------------------------------------------------------------------------------

#[test]
fn narrowing_the_viewport_removes_what_fell_out_of_it_and_widening_it_brings_it_back() {
    let mut opened = open("infrastructure");
    opened.delivered();
    let narrowed = opened
        .session
        .update_view(
            opened.workspace.documents(),
            DiagramViewport::new(-10.0, -10.0, 10.0, 10.0),
        )
        .unwrap();
    let left: Vec<String> = narrowed
        .iter()
        .filter_map(|d| match d {
            DiagramDelta::Remove(ids) => Some(ids.clone()),
            _ => None,
        })
        .flatten()
        .collect();
    assert!(!left.is_empty());
    // One hop: the entry playbook in view brings what it imports.
    assert!(!left.contains(&"playbook:webservers.yml".to_owned()));

    let widened = opened
        .session
        .update_view(opened.workspace.documents(), DiagramViewport::UNBOUNDED)
        .unwrap();
    let mut back: Vec<String> = widened
        .iter()
        .filter_map(|d| match d {
            DiagramDelta::Add(elements) => Some(elements.iter().map(|e| e.id.clone()).collect::<Vec<_>>()),
            _ => None,
        })
        .flatten()
        .collect();
    let mut left = left;
    left.sort();
    back.sort();
    assert_eq!(left, back);
}

// ---- following the folder ----------------------------------------------------------------------------------------------

#[test]
fn a_change_on_disk_reaches_the_view() {
    let mut opened = open("infrastructure");
    opened.delivered();
    let playbook = opened.root.join("dbservers.yml");
    fs::remove_file(&playbook).unwrap();
    let change = opened.workspace.reconcile(&playbook);
    assert_eq!(change, adp_diagram::DiskChange::Reloaded(opened.registration.clone()));
    assert_eq!(opened.workspace.take_changed(), [opened.registration.clone()]);
    let rendered = opened.session.render(opened.workspace.documents()).unwrap();
    assert!(!rendered.iter().any(|e| e.id == "playbook:dbservers.yml"));
    // A file nothing reads changes nothing.
    let note = opened.root.join("docs").join("runbook.md");
    fs::write(&note, "changed").unwrap();
    assert_eq!(opened.workspace.reconcile(&note), adp_diagram::DiskChange::Unchanged);
}

// ---- what it refuses ---------------------------------------------------------------------------------------------------

#[test]
fn a_reparenting_move_is_refused_with_a_reason_and_changes_nothing_on_disk() {
    let mut opened = open("infrastructure");
    opened.delivered();
    let before = opened.snapshot();
    let reason = opened
        .session
        .move_element(&mut opened.workspace, "role:nginx", Some("role:common"), 0)
        .unwrap_err();
    assert!(reason.to_lowercase().contains("folder"), "{reason}");
    assert_eq!(before, opened.snapshot());
}

#[test]
fn a_property_write_is_refused_whatever_it_is_given() {
    let mut opened = open("infrastructure");
    let reason = opened
        .session
        .set_property(&mut opened.workspace, &["role:nginx"], "ansible.name", "something-else")
        .unwrap_err();
    assert!(reason.contains("changes nothing"), "{reason}");
}

// ---- the one edit: a reposition ----------------------------------------------------------------------------------------

#[test]
fn a_move_writes_the_authored_position_into_the_registration() {
    let mut opened = open("infrastructure");
    opened.delivered();
    opened
        .session
        .move_element_to(&mut opened.workspace, "role:common", 321.0, 123.0)
        .unwrap();
    assert_eq!(opened.stored()["role:common"], (321.0, 123.0));
    let moved = opened
        .session
        .render(opened.workspace.documents())
        .unwrap()
        .into_iter()
        .find(|e| e.id == "role:common")
        .unwrap();
    assert_eq!((moved.x, moved.y, moved.placed), (321.0, 123.0, true));
}

#[test]
fn a_move_is_undone_back_to_the_registrations_previous_bytes() {
    let mut opened = open("infrastructure");
    let before = fs::read(&opened.registration).unwrap();
    opened
        .session
        .move_element_to(&mut opened.workspace, "role:common", 321.0, 123.0)
        .unwrap();
    assert!(opened.workspace.undo().is_done());
    assert_eq!(before, fs::read(&opened.registration).unwrap());
    assert!(opened.workspace.redo().is_done());
    assert_eq!(opened.stored()["role:common"], (321.0, 123.0));
}

#[test]
fn a_move_of_an_edge_is_refused_because_an_edge_follows_its_endpoints() {
    let mut opened = open("infrastructure");
    let reason = opened
        .session
        .move_element_to(
            &mut opened.workspace,
            "edge:playbook:webservers.yml|Role|nginx",
            321.0,
            123.0,
        )
        .unwrap_err();
    assert_eq!(reason, EDGE_REFUSAL);
    assert!(opened.stored().is_empty());
}

#[test]
fn an_authored_position_is_where_the_element_is_delivered_on_the_next_open() {
    let (folder, registration) = registered("infrastructure");
    let root = folder.path().to_path_buf();
    {
        let mut workspace = DiagramWorkspace::new(&root);
        let (_, session) = workspace.open(&mut tools(), Path::new("infrastructure.adp")).unwrap();
        session
            .move_element_to(&mut workspace, "role:common", 321.0, 123.0)
            .unwrap();
    }
    let mut opened = open_registered(folder, registration);
    let moved = opened.delivered().into_iter().find(|e| e.id == "role:common").unwrap();
    assert_eq!((moved.x, moved.y), (321.0, 123.0));
}

/// A stored position whose node is absent is not removed by a write, and is applied again when the node returns
/// (finding B9).
#[test]
fn a_stored_position_of_an_absent_node_survives_a_write_and_returns_with_its_node() {
    let folder = tempfile::tempdir().unwrap();
    let root = folder.path().to_path_buf();
    fs::write(root.join("half.yml"), "- name: half\n  hosts: [\n").unwrap();
    fs::write(
        root.join("other.yml"),
        "- name: other\n  hosts: localhost\n  tasks: []\n",
    )
    .unwrap();
    let registration = root.join(".adp");
    fs::write(
        &registration,
        "ansible/structure\nlayout:\n  playbook:half.yml: 100 200\n  playbook:other.yml: 300 400\n",
    )
    .unwrap();
    let mut opened = open_registered(folder, registration);
    assert!(!opened.delivered().iter().any(|e| e.id == "playbook:half.yml"));

    opened
        .session
        .move_element_to(&mut opened.workspace, "playbook:other.yml", 500.0, 600.0)
        .unwrap();
    let stored = opened.stored();
    assert_eq!(stored["playbook:other.yml"], (500.0, 600.0));
    assert_eq!(stored["playbook:half.yml"], (100.0, 200.0));

    fs::write(
        root.join("half.yml"),
        "- name: saved whole\n  hosts: localhost\n  tasks: []\n",
    )
    .unwrap();
    opened.workspace.reconcile(&root.join("half.yml"));
    let returned = opened
        .session
        .render(opened.workspace.documents())
        .unwrap()
        .into_iter()
        .find(|e| e.id == "playbook:half.yml")
        .unwrap();
    assert_eq!((returned.x, returned.y), (100.0, 200.0));
}

// ---- what a selection reads as -----------------------------------------------------------------------------------------

#[test]
fn a_node_resolves_to_its_own_path_and_name() {
    let opened = open("infrastructure");
    let selected = opened
        .session
        .select(opened.workspace.documents(), "role:nginx")
        .unwrap();
    assert_eq!(selected.path, ["roles", "nginx"]);
    assert_eq!(selected.text, "nginx");
    assert_eq!(selected.element.element_type, mapper::ROLE_TYPE);
}

#[test]
fn an_edge_resolves_as_its_declaring_side() {
    let opened = open("infrastructure");
    let edge = AnsibleGraph::derive(&read("infrastructure"))
        .edges
        .into_iter()
        .find(|e| e.kind == AnsibleEdgeKind::IncludesTasks)
        .unwrap();
    let selected = opened.session.select(opened.workspace.documents(), &edge.id()).unwrap();
    assert_eq!(selected.path, ["roles", "nginx", "tasks", "main.yml"]);
    assert!(selected.text.contains("include_tasks:"), "{}", selected.text);
}

#[test]
fn an_unknown_element_resolves_to_nothing() {
    let opened = open("infrastructure");
    assert!(
        opened
            .session
            .select(opened.workspace.documents(), "role:no-such-role")
            .is_none()
    );
}

// ---- the property rows -------------------------------------------------------------------------------------------------

#[test]
fn every_property_of_every_node_kind_is_read_only_with_a_reason_naming_a_file_that_exists() {
    let opened = open("infrastructure");
    for id in [
        "playbook:site.yml",
        "playbook:webservers.yml",
        "role:nginx",
        "role:common",
        "taskfile:roles/nginx/tasks/tls.yml",
        "inventory:inventories/production",
        "vars:inventories/production/group_vars",
    ] {
        let rows = opened.rows(id);
        assert!(!rows.is_empty(), "{id}");
        for row in rows {
            assert!(row.is_read_only(), "{} is editable.", row.id);
            let named = row
                .read_only_reason
                .strip_prefix("Defined in ")
                .unwrap_or_else(|| panic!("{}: {}", row.id, row.read_only_reason))
                .split(';')
                .next()
                .unwrap()
                .to_owned();
            assert!(
                opened.root.join(&named).exists(),
                "{} names '{named}', which does not exist.",
                row.id
            );
        }
    }
}

#[test]
fn every_property_id_is_module_prefixed() {
    let opened = open("infrastructure");
    assert!(opened.rows("role:nginx").iter().all(|r| r.id.starts_with("ansible.")));
}

#[test]
fn a_role_shows_identity_contents_and_relationships() {
    let opened = open("infrastructure");
    let rows = opened.rows("role:nginx");
    assert_eq!(groups(&rows), ["Identity", "Contents", "Relationships"]);
    assert_eq!(row(&rows, "ansible.name").value, "nginx");
    assert_eq!(row(&rows, "ansible.tasks").value, "2");
    assert_eq!(row(&rows, "ansible.handlers").value, "1");
    assert_eq!(row(&rows, "ansible.dependencies").value, "common");
    assert!(row(&rows, "ansible.used-by").value.contains("webservers.yml"));
    assert!(
        row(&rows, "ansible.dependencies")
            .read_only_reason
            .contains("roles/nginx/meta/main.yml")
    );
}

#[test]
fn a_playbook_shows_identity_runs_and_targets() {
    let opened = open("infrastructure");
    let rows = opened.rows("playbook:webservers.yml");
    assert_eq!(groups(&rows), ["Identity", "Runs", "Targets"]);
    assert_eq!(row(&rows, "ansible.roles").value, "common, nginx");
    assert_eq!(row(&rows, "ansible.hosts").value, "web");
    assert!(row(&rows, "ansible.inventories").value.contains("production"));
    assert_eq!(
        row(&opened.rows("playbook:site.yml"), "ansible.imports").value,
        "webservers.yml, dbservers.yml"
    );
}

#[test]
fn an_inventory_shows_its_groups_and_variable_folders() {
    let opened = open("infrastructure");
    let rows = opened.rows("inventory:inventories/production");
    assert_eq!(groups(&rows), ["Identity", "Groups", "Variables"]);
    assert_eq!(row(&rows, "ansible.group.web").value, "2 hosts");
    assert_eq!(row(&rows, "ansible.group.db").value, "1 host");
    assert_eq!(row(&rows, "ansible.vars.group_vars").value, "2 files");
}

#[test]
fn an_edge_says_which_file_and_directive_declared_it() {
    let opened = open("infrastructure");
    let rows = opened.rows("edge:role:nginx|IncludesTasks|tls.yml");
    assert_eq!(groups(&rows), ["Declaration"]);
    assert!(
        row(&rows, "ansible.declared-in")
            .value
            .starts_with("roles/nginx/tasks/main.yml:")
    );
    assert_eq!(row(&rows, "ansible.directive").value, "include_tasks");
    assert_eq!(row(&rows, "ansible.target").value, "tls.yml");
    assert_eq!(
        row(&rows, "ansible.condition").value,
        "nginx_tls_enabled | default(false)"
    );
    let import = opened.rows("edge:playbook:site.yml|ImportsPlaybook|webservers.yml");
    assert!(!import.iter().any(|r| r.id == "ansible.condition"));
}

#[test]
fn a_property_absent_from_the_files_is_not_contributed_at_all() {
    let opened = open("infrastructure");
    let rows = opened.rows("role:common");
    for absent in [
        "ansible.meta",
        "ansible.templates",
        "ansible.handlers",
        "ansible.dependencies",
    ] {
        assert!(!rows.iter().any(|r| r.id == absent), "{absent}");
    }
    assert_eq!(row(&rows, "ansible.tasks").value, "1");
    assert_eq!(row(&rows, "ansible.defaults").value, "1");
}

#[test]
fn a_hollow_role_and_an_unresolved_edge_say_so() {
    let opened = open("broken");
    assert!(
        row(&opened.rows("role:hollow"), "ansible.hollow")
            .value
            .to_lowercase()
            .contains("nothing to run")
    );
    let edges = AnsibleGraph::derive(&read("broken")).edges;
    let missing = edges.iter().find(|e| e.directive.target == "absent-role").unwrap();
    let expression = edges.iter().find(|e| e.directive.is_expression()).unwrap();
    assert_eq!(
        row(&opened.rows(&missing.id()), "ansible.resolution").value,
        "nothing in this folder"
    );
    assert!(
        row(&opened.rows(&expression.id()), "ansible.resolution")
            .value
            .contains("not knowable")
    );
}

#[test]
fn an_element_that_is_gone_describes_nothing() {
    let opened = open("infrastructure");
    assert!(opened.rows("role:never-existed").is_empty());
}

#[test]
fn the_palette_is_empty_and_no_action_is_offered() {
    let opened = open("infrastructure");
    assert!(opened.session.toolbox(opened.workspace.documents()).is_empty());
    let view = Rc::clone(opened.session.module_view().unwrap());
    assert!(
        view.actions(
            opened.workspace.documents(),
            &adp_diagram::ActionTarget::Element("role:nginx".into())
        )
        .is_empty()
    );
}

#[test]
fn the_problems_are_the_definitions_findings() {
    let opened = open("broken");
    let mut rules: Vec<String> = opened
        .session
        .problems(opened.workspace.documents())
        .into_iter()
        .map(|p| p.rule_id)
        .collect();
    rules.dedup();
    assert!(rules.contains(&"ansible.role-missing".to_owned()), "{rules:?}");
    let clean = open("infrastructure");
    assert!(clean.session.problems(clean.workspace.documents()).is_empty());
}

/// Reading, drawing, describing and judging the folder write nothing into it (the standalone host's
/// `ZeroWrites.Tests.cs`): only a move writes, and only the registration.
#[test]
fn nothing_but_a_move_writes_and_a_move_writes_only_the_registration() {
    let mut opened = open("infrastructure");
    let before = opened.snapshot();
    opened.delivered();
    opened.session.problems(opened.workspace.documents());
    for id in ["role:nginx", "playbook:site.yml", "inventory:inventories/production"] {
        opened.rows(id);
        opened.session.select(opened.workspace.documents(), id);
    }
    assert_eq!(before, opened.snapshot());

    opened
        .session
        .move_element_to(&mut opened.workspace, "role:nginx", 1.0, 2.0)
        .unwrap();
    let after = opened.snapshot();
    let changed: Vec<&PathBuf> = before
        .iter()
        .zip(&after)
        .filter(|(b, a)| b != a)
        .map(|(b, _)| &b.0)
        .collect();
    assert_eq!(changed, [&opened.registration]);
}
