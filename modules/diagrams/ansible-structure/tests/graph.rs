//! One test per edge kind, plus the three-state resolution that keeps a parameterised role from being reported as a
//! missing one (etalii.adp spec 030, task T079): the standalone host's `AnsibleGraph.Tests.cs` at `8b6ccfc`, with the
//! same expectations.

mod common;

use std::collections::HashSet;

use adp_diagram_ansible_structure::graph::{self, AnsibleGraph};
use adp_diagram_ansible_structure::model::{
    AnsibleEdge, AnsibleEdgeKind, AnsibleInventory, AnsibleInventoryGroup, AnsibleNodeKind, AnsibleTargetResolution,
};
use common::read;

fn graph_of(fixture: &str) -> AnsibleGraph {
    AnsibleGraph::derive(&read(fixture))
}

fn edges_of(graph: &AnsibleGraph, kind: AnsibleEdgeKind) -> Vec<AnsibleEdge> {
    graph.edges.iter().filter(|e| e.kind == kind).cloned().collect()
}

fn has(edges: &[AnsibleEdge], source: &str, target: &str) -> bool {
    edges.iter().any(|e| e.source_id == source && e.target_id == target)
}

// ---- the five edge kinds -----------------------------------------------------------------------------------------------

#[test]
fn a_plays_roles_list_runs_playbook_to_role() {
    let uses = edges_of(&graph_of("infrastructure"), AnsibleEdgeKind::UsesRole);
    assert!(has(&uses, "playbook:webservers.yml", "role:common"));
    assert!(has(&uses, "playbook:webservers.yml", "role:nginx"));
    assert!(has(&uses, "playbook:dbservers.yml", "role:postgres"));
    assert!(uses.iter().all(|e| e.resolution == AnsibleTargetResolution::Resolved));
}

#[test]
fn an_import_playbook_runs_playbook_to_playbook() {
    let imports = edges_of(&graph_of("infrastructure"), AnsibleEdgeKind::ImportsPlaybook);
    assert_eq!(imports.len(), 2);
    assert!(imports.iter().all(|e| e.source_id == "playbook:site.yml"));
    let targets: Vec<&str> = imports.iter().map(|e| e.target_id.as_str()).collect();
    assert_eq!(targets, ["playbook:webservers.yml", "playbook:dbservers.yml"]);
}

#[test]
fn an_include_tasks_runs_within_the_role_and_draws_the_task_file() {
    let graph = graph_of("infrastructure");
    let [include] = edges_of(&graph, AnsibleEdgeKind::IncludesTasks).try_into().unwrap();
    assert_eq!(include.source_id, "role:nginx");
    assert_eq!(include.target_id, "taskfile:roles/nginx/tasks/tls.yml");
    assert_eq!(
        graph
            .nodes
            .iter()
            .filter(|n| n.kind == AnsibleNodeKind::TaskFile)
            .count(),
        1
    );
}

#[test]
fn a_meta_dependency_runs_role_to_role() {
    let dependencies = edges_of(&graph_of("infrastructure"), AnsibleEdgeKind::DependsOn);
    assert_eq!(dependencies.len(), 2);
    assert!(dependencies.iter().all(|e| e.target_id == "role:common"));
    let mut sources: Vec<&str> = dependencies.iter().map(|e| e.source_id.as_str()).collect();
    sources.sort();
    assert_eq!(sources, ["role:nginx", "role:postgres"]);
}

#[test]
fn a_hosts_pattern_runs_to_every_inventory_that_defines_it() {
    let targets = edges_of(&graph_of("infrastructure"), AnsibleEdgeKind::Targets);
    assert_eq!(targets.len(), 4);
    assert!(has(
        &targets,
        "playbook:webservers.yml",
        "inventory:inventories/production"
    ));
    assert!(has(
        &targets,
        "playbook:webservers.yml",
        "inventory:inventories/staging"
    ));
    assert!(
        targets
            .iter()
            .filter(|e| e.source_id == "playbook:webservers.yml")
            .all(|e| e.directive.target == "web")
    );
    assert!(
        targets
            .iter()
            .filter(|e| e.source_id == "playbook:dbservers.yml")
            .all(|e| e.directive.target == "db")
    );
}

#[test]
fn a_hosts_pattern_reaching_two_inventories_is_two_edges_with_two_ids() {
    let targets = edges_of(&graph_of("infrastructure"), AnsibleEdgeKind::Targets);
    let ids: HashSet<String> = targets.iter().map(AnsibleEdge::id).collect();
    assert_eq!(targets.len(), 4);
    assert_eq!(ids.len(), targets.len());
}

#[test]
fn every_node_and_edge_has_an_id_nothing_else_in_the_graph_has() {
    for fixture in ["infrastructure", "broken", "ini-inventory", "unconventional"] {
        let graph = graph_of(fixture);
        let ids: Vec<String> = graph
            .nodes
            .iter()
            .map(|n| n.id.clone())
            .chain(graph.edges.iter().map(AnsibleEdge::id))
            .collect();
        let distinct: HashSet<&String> = ids.iter().collect();
        assert_eq!(distinct.len(), ids.len(), "{fixture}: {ids:?}");
    }
}

// ---- static versus dynamic, and what the file wrote --------------------------------------------------------------------

#[test]
fn static_and_dynamic_mechanisms_are_distinguishable() {
    let graph = graph_of("infrastructure");
    let [include] = edges_of(&graph, AnsibleEdgeKind::IncludesTasks).try_into().unwrap();
    assert!(include.is_dynamic());
    assert!(
        edges_of(&graph, AnsibleEdgeKind::ImportsPlaybook)
            .iter()
            .all(|e| !e.is_dynamic())
    );
    assert!(
        edges_of(&graph, AnsibleEdgeKind::UsesRole)
            .iter()
            .all(|e| !e.is_dynamic())
    );
}

#[test]
fn an_edge_carries_the_file_and_line_that_declared_it() {
    let [include] = edges_of(&graph_of("infrastructure"), AnsibleEdgeKind::IncludesTasks)
        .try_into()
        .unwrap();
    assert_eq!(include.directive.declared_in, "roles/nginx/tasks/main.yml");
    assert!(include.directive.line > 0);
    assert_eq!(include.directive.target, "tls.yml");
}

#[test]
fn a_when_condition_survives_as_written_and_is_never_evaluated() {
    let uses = edges_of(&graph_of("infrastructure"), AnsibleEdgeKind::UsesRole);
    let postgres = uses.iter().find(|e| e.target_id == "role:postgres").unwrap();
    assert_eq!(postgres.directive.condition, "postgres_enabled | default(true)");
}

// ---- the three resolution states ---------------------------------------------------------------------------------------

#[test]
fn a_role_with_no_folder_is_missing() {
    let uses = edges_of(&graph_of("broken"), AnsibleEdgeKind::UsesRole);
    let absent = uses.iter().find(|e| e.directive.target == "absent-role").unwrap();
    assert_eq!(absent.resolution, AnsibleTargetResolution::Missing);
    assert_eq!(absent.target_id, "");
}

#[test]
fn an_expression_target_is_unresolvable_not_missing() {
    let uses = edges_of(&graph_of("broken"), AnsibleEdgeKind::UsesRole);
    let expression: Vec<&AnsibleEdge> = uses.iter().filter(|e| e.directive.is_expression()).collect();
    assert_eq!(expression.len(), 1);
    assert_eq!(expression[0].resolution, AnsibleTargetResolution::Unresolvable);
    assert_eq!(expression[0].directive.target, "{{ role_name }}");
}

#[test]
fn a_dangling_import_and_include_are_both_missing() {
    let graph = graph_of("broken");
    let import = edges_of(&graph, AnsibleEdgeKind::ImportsPlaybook)
        .into_iter()
        .find(|e| e.directive.target == "does-not-exist.yml")
        .unwrap();
    assert_eq!(import.resolution, AnsibleTargetResolution::Missing);
    let [include] = edges_of(&graph, AnsibleEdgeKind::IncludesTasks).try_into().unwrap();
    assert_eq!(include.resolution, AnsibleTargetResolution::Missing);
    assert_eq!(include.directive.target, "tls.yml");
}

#[test]
fn a_dependency_on_a_missing_role_is_missing() {
    let [dependency] = edges_of(&graph_of("broken"), AnsibleEdgeKind::DependsOn)
        .try_into()
        .unwrap();
    assert_eq!(dependency.directive.target, "base");
    assert_eq!(dependency.resolution, AnsibleTargetResolution::Missing);
}

#[test]
fn a_hosts_pattern_no_inventory_defines_yields_no_edge_at_all() {
    let targets = edges_of(&graph_of("broken"), AnsibleEdgeKind::Targets);
    assert!(targets.iter().all(|e| e.directive.target != "cache"));
}

// ---- multi-play playbooks, and what is deliberately not drawn ----------------------------------------------------------

#[test]
fn a_playbook_with_several_plays_draws_each_play_and_edges_run_from_the_play() {
    let graph = graph_of("broken");
    let plays: Vec<_> = graph.nodes.iter().filter(|n| n.kind == AnsibleNodeKind::Play).collect();
    assert_eq!(plays.len(), 3);
    assert!(plays.iter().all(|p| p.id.starts_with("play:playbooks/deploy.yml#")));
    assert!(
        edges_of(&graph, AnsibleEdgeKind::UsesRole)
            .iter()
            .any(|e| e.source_id.starts_with("play:"))
    );
}

#[test]
fn each_play_gets_its_own_colour_index() {
    let graph = graph_of("broken");
    let plays: Vec<_> = graph.nodes.iter().filter(|n| n.kind == AnsibleNodeKind::Play).collect();
    let indices: HashSet<i64> = plays.iter().map(|p| p.play_index).collect();
    assert_eq!(indices.len(), plays.len());
    assert!(plays.iter().all(|p| p.play_index >= 0));
}

#[test]
fn variable_folders_are_nodes_but_nothing_draws_an_edge_to_a_variable() {
    let graph = graph_of("infrastructure");
    assert_eq!(
        graph
            .nodes
            .iter()
            .filter(|n| n.kind == AnsibleNodeKind::VariableFolder)
            .count(),
        3
    );
    assert!(graph.edges.iter().all(|e| !e.target_id.contains("vars:")));
}

#[test]
fn an_unrecognised_layout_has_no_nodes_and_no_edges() {
    let graph = graph_of("unconventional");
    assert!(graph.nodes.is_empty());
    assert!(graph.edges.is_empty());
}

// ---- host pattern matching ---------------------------------------------------------------------------------------------

#[test]
fn host_patterns_match_the_subset_the_module_claims() {
    let inventory = AnsibleInventory {
        name: "production".into(),
        relative_path: "inventories/production".into(),
        groups: vec![
            AnsibleInventoryGroup {
                name: "web".into(),
                host_count: 2,
            },
            AnsibleInventoryGroup {
                name: "db".into(),
                host_count: 1,
            },
        ],
        variable_folders: Vec::new(),
    };
    for (pattern, expected) in [
        ("web", true),
        ("db", true),
        ("cache", false),
        ("all", true),
        ("*", true),
        ("web:db", true),
        ("cache:web", true),
        ("we*", true),
        ("xy*", false),
        ("!web", false),
        ("&web", false),
        ("", false),
    ] {
        assert_eq!(graph::matches(Some(pattern), &inventory), expected, "{pattern}");
    }
}

#[test]
fn the_same_project_derives_the_same_graph_twice() {
    assert_eq!(graph_of("infrastructure"), graph_of("infrastructure"));
}
