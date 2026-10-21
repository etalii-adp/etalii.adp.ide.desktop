//! The parser against a realistic deployment model (etalii.adp spec 030, task T061), ported from the standalone host's
//! `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/AwsDeployment.Tests.cs` at `8b6ccfc`: infrastructure nodes,
//! four levels of nesting, an instance count on a node that also has a body, relationships declared inside a
//! deployment environment, the findings Structurizr's inspector also reports, and a layout without overlaps.

use std::path::Path;

use adp_diagram_c4::metrics::C4Metrics;
use adp_diagram_c4::model::{C4Element, C4ElementKind, C4ViewKind, C4Workspace};
use adp_diagram_c4::rules::{self, codes};
use adp_diagram_c4::{layout, parser};

fn workspace() -> C4Workspace {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/aws-deployment.dsl");
    parser::parse_text(&std::fs::read_to_string(path).unwrap())
}

fn node<'a>(w: &'a C4Workspace, name: &str) -> &'a C4Element {
    let found: Vec<_> = w
        .elements
        .iter()
        .filter(|e| {
            matches!(
                e.kind,
                C4ElementKind::DeploymentNode | C4ElementKind::InfrastructureNode
            ) && e.name == name
        })
        .collect();
    assert_eq!(found.len(), 1, "{name}");
    found[0]
}

#[test]
fn infrastructure_nodes_are_their_own_kind_not_deployment_nodes() {
    let w = workspace();
    assert_eq!(node(&w, "Route 53").kind, C4ElementKind::InfrastructureNode);
    assert_eq!(
        node(&w, "Elastic Load Balancer").kind,
        C4ElementKind::InfrastructureNode
    );
    assert_eq!(node(&w, "Amazon RDS").kind, C4ElementKind::DeploymentNode);
}

#[test]
fn deployment_nodes_nest_four_deep() {
    let w = workspace();
    let ec2 = node(&w, "Amazon EC2");
    let autoscaling = w.find(ec2.parent_id.as_deref().unwrap()).unwrap();
    let region = w.find(autoscaling.parent_id.as_deref().unwrap()).unwrap();
    let cloud = w.find(region.parent_id.as_deref().unwrap()).unwrap();
    assert_eq!(autoscaling.name, "Autoscaling group");
    assert_eq!(region.name, "US-East-1");
    assert_eq!(cloud.name, "Amazon Web Services");
    assert_eq!(cloud.parent_id, None);
}

#[test]
fn an_instance_count_on_a_node_that_also_has_a_body_is_not_mistaken_for_something_else() {
    let w = workspace();
    let ec2 = node(&w, "Amazon EC2");
    assert!(ec2.description.is_empty());
    assert!(ec2.technology.is_empty());
    assert!(!ec2.tags.iter().any(|t| t == "2"));
    // The tags come from the nested `tags` line, not from the argument list.
    assert!(ec2.tags.iter().any(|t| t == "Amazon Web Services - EC2"));
}

#[test]
fn tags_declared_inside_a_block_reach_the_element() {
    let w = workspace();
    assert!(
        node(&w, "Route 53")
            .tags
            .iter()
            .any(|t| t == "Amazon Web Services - Route 53")
    );
    assert!(
        node(&w, "US-East-1")
            .tags
            .iter()
            .any(|t| t == "Amazon Web Services - Region")
    );
}

#[test]
fn container_instances_resolve_to_the_containers_they_deploy() {
    let w = workspace();
    let instances: Vec<_> = w
        .elements
        .iter()
        .filter(|e| e.kind == C4ElementKind::ContainerInstance)
        .collect();
    assert_eq!(instances.len(), 2);
    assert!(
        instances
            .iter()
            .any(|i| i.referenced_id.as_deref() == Some("webApplication"))
    );
    assert!(instances.iter().any(|i| i.referenced_id.as_deref() == Some("database")));
}

#[test]
fn relationships_declared_in_the_environment_reach_infrastructure_and_instances_alike() {
    let w = workspace();
    assert!(
        w.relationships
            .iter()
            .any(|r| r.source_id == "route53" && r.destination_id == "elb" && r.technology == "HTTPS")
    );
    assert!(
        w.relationships
            .iter()
            .any(|r| r.source_id == "elb" && r.destination_id == "webApplicationInstance")
    );
}

#[test]
fn the_deployment_view_knows_its_environment_and_its_scope() {
    let w = workspace();
    assert_eq!(w.views.len(), 1);
    let view = &w.views[0];
    assert_eq!(view.kind, C4ViewKind::Deployment);
    assert_eq!(view.key, "AmazonWebServicesDeployment");
    assert_eq!(view.environment.as_deref(), Some("Live"));
    assert_eq!(view.scope_id.as_deref(), Some("springPetClinic"));
}

#[test]
fn the_aws_example_reports_its_bare_nodes() {
    let mut reported: Vec<String> = rules::validate(&workspace())
        .into_iter()
        .filter(|p| p.rule_id != codes::EMPTY_VIEW)
        .map(|p| p.rule_id)
        .collect();
    reported.sort();
    reported.dedup();
    assert_eq!(
        reported,
        [
            codes::DISCONNECTED_ELEMENT,
            codes::ELEMENT_NOT_ON_ANY_VIEW,
            codes::MISSING_DEPLOYMENT_DESCRIPTION,
            codes::MISSING_DEPLOYMENT_TECHNOLOGY,
            codes::MISSING_INFRASTRUCTURE_TECHNOLOGY,
        ]
    );
}

#[test]
fn the_deployment_view_lays_out_without_overlaps() {
    let w = workspace();
    let layout = layout::compute(&w, &w.views[0], &C4Metrics::default(), None);
    let boxes: Vec<_> = layout.boxes.iter().collect();
    for i in 0..boxes.len() {
        for j in i + 1..boxes.len() {
            assert!(
                !boxes[i].1.overlaps(&boxes[j].1),
                "'{}' {:?} overlaps '{}' {:?}",
                boxes[i].0,
                boxes[i].1,
                boxes[j].0,
                boxes[j].1
            );
        }
    }
}
