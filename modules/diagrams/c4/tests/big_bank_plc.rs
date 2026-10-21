//! The parser against the C4 worked example (etalii.adp spec 030, task T061), ported from the standalone host's
//! `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/BigBankPlc.Tests.cs` at `8b6ccfc`: all four static levels, a
//! dynamic view, a deployment environment and a styles block in one document certified valid by the Structurizr CLI;
//! every view lays out without overlaps, and the rules report what Structurizr's inspector also reports.

use std::path::Path;

use adp_diagram::DiagramProblemSeverity;
use adp_diagram_c4::metrics::C4Metrics;
use adp_diagram_c4::model::{C4ElementKind, C4ViewKind, C4Workspace};
use adp_diagram_c4::rules::{self, codes};
use adp_diagram_c4::{layout, parser, theme};

fn workspace() -> C4Workspace {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/big-bank-plc.dsl");
    parser::parse_text(&std::fs::read_to_string(path).unwrap())
}

#[test]
fn the_four_static_levels_all_parse() {
    let w = workspace();
    assert_eq!(w.find("customer").unwrap().kind, C4ElementKind::Person);
    assert_eq!(
        w.find("internetBankingSystem").unwrap().kind,
        C4ElementKind::SoftwareSystem
    );
    assert_eq!(w.find("apiApplication").unwrap().kind, C4ElementKind::Container);
    assert_eq!(w.find("securityComponent").unwrap().kind, C4ElementKind::Component);
}

#[test]
fn the_hierarchy_is_nested_three_levels_deep() {
    let w = workspace();
    assert_eq!(
        w.find("apiApplication").unwrap().parent_id.as_deref(),
        Some("internetBankingSystem")
    );
    assert_eq!(
        w.find("securityComponent").unwrap().parent_id.as_deref(),
        Some("apiApplication")
    );
}

#[test]
fn existing_systems_are_external_and_the_subject_is_not() {
    let w = workspace();
    assert!(w.find("mainframeBankingSystem").unwrap().is_external());
    assert!(w.find("emailSystem").unwrap().is_external());
    assert!(!w.find("internetBankingSystem").unwrap().is_external());
}

#[test]
fn containers_keep_their_technology_and_their_tags() {
    let w = workspace();
    let database = w.find("database").unwrap();
    assert_eq!(database.technology, "Oracle Database Schema");
    assert!(database.tags.iter().any(|t| t == "Database"));
    assert_eq!(theme::shape_of(database), "Cylinder");
}

#[test]
fn a_deployment_node_with_an_instance_count_does_not_mistake_the_count_for_a_tag() {
    let w = workspace();
    let nodes: Vec<_> = w
        .elements
        .iter()
        .filter(|e| e.kind == C4ElementKind::DeploymentNode && e.name == "bigbank-web***")
        .collect();
    assert_eq!(nodes.len(), 1);
    let web_server = nodes[0];
    assert_eq!(web_server.technology, "Ubuntu 16.04 LTS");
    assert!(!web_server.tags.iter().any(|t| t == "4"));
    assert!(web_server.tags.is_empty());
}

#[test]
fn deployment_nodes_nest_and_instances_resolve_to_their_containers() {
    let w = workspace();
    let tomcat = w
        .elements
        .iter()
        .find(|e| e.kind == C4ElementKind::DeploymentNode && e.name == "Apache Tomcat")
        .unwrap();
    assert!(tomcat.parent_id.is_some());
    let instances: Vec<_> = w
        .elements
        .iter()
        .filter(|e| e.kind == C4ElementKind::ContainerInstance)
        .collect();
    assert!(instances.iter().any(|i| i.referenced_id.as_deref() == Some("database")));
    assert_eq!(
        instances
            .iter()
            .filter(|i| i.referenced_id.as_deref() == Some("database"))
            .count(),
        2
    );
}

#[test]
fn every_declared_view_is_read() {
    let keys: Vec<String> = workspace().views.iter().map(|v| v.key.clone()).collect();
    assert_eq!(
        keys,
        [
            "SystemLandscape",
            "SystemContext",
            "Containers",
            "Components",
            "SignIn",
            "LiveDeployment"
        ]
    );
}

#[test]
fn the_dynamic_view_keeps_its_interactions_in_order() {
    let w = workspace();
    let sign_in = w.find_view("SignIn").unwrap();
    assert_eq!(sign_in.kind, C4ViewKind::Dynamic);
    assert_eq!(sign_in.interactions.len(), 6);
    assert_eq!(sign_in.interactions[0].source_id, "singlePageApplication");
    assert_eq!(sign_in.interactions[0].description, "Submits credentials to");
    let orders: Vec<&str> = sign_in.interactions.iter().map(|i| i.order.as_str()).collect();
    assert_eq!(orders, ["1", "2", "3", "4", "5", "6"]);
}

#[test]
fn auto_layout_with_no_direction_is_still_read() {
    assert!(workspace().find_view("Containers").unwrap().auto_layout.is_some());
}

#[test]
fn the_styles_block_is_read_so_the_documents_palette_wins() {
    let w = workspace();
    for tag in ["Person", "Existing System", "Database"] {
        assert!(w.styles.iter().any(|s| s.tag == tag), "{tag}");
    }
}

#[test]
fn relationships_across_all_levels_are_read() {
    let w = workspace();
    assert!(
        w.relationships
            .iter()
            .any(|r| r.source_id == "customer" && r.destination_id == "internetBankingSystem")
    );
    assert!(
        w.relationships
            .iter()
            .any(|r| r.source_id == "securityComponent" && r.destination_id == "database" && r.technology == "SQL/TCP")
    );
    assert!(
        w.relationships
            .iter()
            .any(|r| r.source_id == "primaryDatabaseServer" && r.destination_id == "secondaryDatabaseServer")
    );
}

#[test]
fn every_view_lays_out_without_overlaps() {
    let w = workspace();
    assert!(w.views.len() >= 4, "only {} views were parsed", w.views.len());
    let mut comparable_views = 0;
    for view in &w.views {
        let layout = layout::compute(&w, view, &C4Metrics::default(), None);
        let boxes: Vec<_> = layout.boxes.iter().collect();
        if boxes.len() >= 2 {
            comparable_views += 1;
        }
        for i in 0..boxes.len() {
            for j in i + 1..boxes.len() {
                assert!(
                    !boxes[i].1.overlaps(&boxes[j].1),
                    "'{}': '{}' {:?} overlaps '{}' {:?}",
                    view.key,
                    boxes[i].0,
                    boxes[i].1,
                    boxes[j].0,
                    boxes[j].1
                );
            }
        }
    }
    assert!(comparable_views > 0, "no view laid out two or more boxes");
}

#[test]
fn the_worked_example_reports_what_structurizr_also_reports() {
    let mut reported: Vec<String> = rules::validate(&workspace())
        .into_iter()
        .filter(|p| p.rule_id != codes::EMPTY_VIEW)
        .map(|p| p.rule_id)
        .collect();
    reported.sort();
    reported.dedup();
    assert_eq!(
        reported,
        [codes::MISSING_DEPLOYMENT_DESCRIPTION, codes::MISSING_PROTOCOL]
    );
}

#[test]
fn every_problem_on_the_worked_example_is_a_warning_rather_than_an_error() {
    assert!(
        rules::validate(&workspace())
            .iter()
            .all(|p| p.severity == DiagramProblemSeverity::Warning)
    );
}
