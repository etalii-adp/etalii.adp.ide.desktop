//! The two view kinds that once drew nothing useful (etalii.adp spec 030, task T061): ported from the standalone
//! host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/C4DynamicAndDeploymentViews.Tests.cs` at `8b6ccfc`. A
//! dynamic view shows what takes part in its interactions, numbered in order; a deployment view draws each node as a
//! boundary around what it hosts, to any depth, outermost first, an instance as the container it instantiates, and a
//! container relationship between its instances.

use std::path::Path;

use adp_diagram::{DiagramElement, DiagramViewport};
use adp_diagram_c4::layout;
use adp_diagram_c4::mapper::{self, RELATIONSHIP_TYPE};
use adp_diagram_c4::metrics::C4Metrics;
use adp_diagram_c4::model::{C4View, C4Workspace};
use adp_diagram_c4::parser;
use adp_diagram_c4::rules::{self, codes};
use adp_layout::rect::Rect;

fn load_fixture(name: &str) -> (C4Workspace, C4View) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name);
    let workspace = parser::parse_text(&std::fs::read_to_string(path).unwrap());
    let view = workspace.views[0].clone();
    (workspace, view)
}

fn instance_of(workspace: &C4Workspace, container: &str, node: &str) -> String {
    let found: Vec<_> = workspace
        .elements
        .iter()
        .filter(|e| e.referenced_id.as_deref() == Some(container) && e.parent_id.as_deref() == Some(node))
        .collect();
    assert_eq!(found.len(), 1, "{container} on {node}");
    found[0].id.clone()
}

fn encloses(outer: Rect, inner: Rect) -> bool {
    inner.x > outer.x && inner.y > outer.y && inner.right() < outer.right() && inner.bottom() < outer.bottom()
}

fn visible(workspace: &C4Workspace, view: &C4View) -> Vec<DiagramElement> {
    mapper::visible(workspace, view, DiagramViewport::UNBOUNDED, &C4Metrics::default(), None)
}

fn compute(workspace: &C4Workspace, view: &C4View) -> layout::C4Layout {
    layout::compute(workspace, view, &C4Metrics::default(), None)
}

#[test]
fn a_dynamic_view_shows_what_takes_part_in_its_interactions_without_an_include() {
    let (workspace, view) = load_fixture("dynamic-interactions.dsl");
    assert!(view.includes.is_empty());
    assert!(!view.includes_everything);
    let mut ids: Vec<String> = compute(&workspace, &view)
        .boxes
        .iter()
        .map(|(id, _)| id.to_owned())
        .collect();
    ids.sort();
    assert_eq!(ids, ["api", "db", "u", "web"]);
}

#[test]
fn a_dynamic_view_delivers_its_interactions_numbered_in_order() {
    let (workspace, view) = load_fixture("dynamic-interactions.dsl");
    let mut lines: Vec<String> = visible(&workspace, &view)
        .iter()
        .filter(|e| e.element_type == RELATIONSHIP_TYPE)
        .map(|e| {
            format!(
                "{} {}->{}",
                e.attribute("interactionOrder"),
                e.source.as_deref().unwrap(),
                e.target.as_deref().unwrap()
            )
        })
        .collect();
    lines.sort();
    assert_eq!(lines, ["1 u->web", "2 web->api", "3 api->db", "4 api->web"]);
}

#[test]
fn a_dynamic_view_whose_interactions_name_nothing_declared_is_reported_empty() {
    let dsl = r#"workspace {
    model {
        s = softwareSystem "System" "Does it."
    }
    views {
        dynamic s "flow" {
            ghost -> phantom "Talks to"
        }
    }
}"#;
    let problems = rules::validate(&parser::parse_text(dsl));
    assert!(problems.iter().any(|p| p.rule_id == codes::EMPTY_VIEW));
}

#[test]
fn a_deployment_view_draws_each_node_around_what_it_hosts_to_any_depth() {
    let (workspace, view) = load_fixture("deployment-nested.dsl");
    let web = instance_of(&workspace, "web", "node");
    let primary_db = instance_of(&workspace, "db", "primary");
    let layout = compute(&workspace, &view);
    let boundary = |id: &str| layout.boundaries.iter().find(|b| b.id == id).unwrap().rect;
    let at = |id: &str| layout.boxes.get(id).unwrap();
    assert!(
        encloses(boundary("boundary:node"), at(&web)),
        "the web instance is not inside its Fargate node"
    );
    assert!(
        encloses(boundary("boundary:cluster"), boundary("boundary:node")),
        "the Fargate node is not inside its cluster"
    );
    assert!(
        encloses(boundary("boundary:region"), boundary("boundary:cluster")),
        "the cluster is not inside its region"
    );
    assert!(
        encloses(boundary("boundary:primary"), at(&primary_db)),
        "the database instance is not inside its primary node"
    );
    assert!(
        encloses(boundary("boundary:region"), at("dns")),
        "the infrastructure node is not inside its region"
    );
    assert!(!layout.boxes.contains("region"));
}

#[test]
fn a_deployment_view_lays_out_without_overlaps() {
    let (workspace, view) = load_fixture("deployment-nested.dsl");
    let layout = compute(&workspace, &view);
    let boxes: Vec<Rect> = layout.boxes.iter().map(|(_, r)| r).collect();
    for i in 0..boxes.len() {
        for j in i + 1..boxes.len() {
            assert!(!boxes[i].overlaps(&boxes[j]), "{:?} overlaps {:?}", boxes[i], boxes[j]);
        }
    }
}

#[test]
fn a_deployment_view_boundaries_come_outermost_first_so_the_inner_ones_draw_on_top() {
    let (workspace, view) = load_fixture("deployment-nested.dsl");
    let ids: Vec<String> = compute(&workspace, &view)
        .boundaries
        .into_iter()
        .map(|b| b.id)
        .collect();
    let index = |id: &str| ids.iter().position(|i| i == id).unwrap();
    assert!(index("boundary:region") < index("boundary:cluster"));
    assert!(index("boundary:cluster") < index("boundary:node"));
}

#[test]
fn a_container_instance_shows_the_container_it_instantiates() {
    let (workspace, view) = load_fixture("deployment-nested.dsl");
    let web = instance_of(&workspace, "web", "node");
    let elements = visible(&workspace, &view);
    let node = elements.iter().find(|e| e.id == web).unwrap();
    assert_eq!(node.attribute("name"), "Web App");
    assert_eq!(node.attribute("typeLine"), "[Container: React]");
}

const DEPLOYED: &str = r#"workspace {
    model {
        s = softwareSystem "System" "Does it." {
            web = container "Web App" "Serves pages." "React"
            api = container "API" "Answers." "Kotlin"
            db = container "Database" "Stores." "PostgreSQL"
            web -> api "Posts to" "JSON/HTTPS"
            api -> db "Writes" "SQL/TCP"
        }
        production = deploymentEnvironment "Production" {
            region = deploymentNode "eu-west-1" "A region." "Region" {
                dns = infrastructureNode "Route 53" "Resolves." "DNS"
                lb = infrastructureNode "Load Balancer" "Balances." "ELB"
                node = deploymentNode "Service" "Runs it." "Fargate" {
                    containerInstance web
                    containerInstance api
                }
                primary = deploymentNode "Primary" "Writes." "PostgreSQL" {
                    containerInstance db
                }
                replica = deploymentNode "Replica" "Reads." "PostgreSQL" {
                    containerInstance db
                }
                dns -> lb "Forwards to" "HTTPS"
            }
        }
    }
    views {
        deployment s "Production" "production" {
            include *
        }
    }
}"#;

#[test]
fn a_deployment_view_draws_container_relationships_between_their_instances() {
    let workspace = parser::parse_text(DEPLOYED);
    let view = workspace.views[0].clone();
    let web = instance_of(&workspace, "web", "node");
    let api = instance_of(&workspace, "api", "node");
    let primary = instance_of(&workspace, "db", "primary");
    let replica = instance_of(&workspace, "db", "replica");
    let lines: Vec<(String, String)> = visible(&workspace, &view)
        .iter()
        .filter(|e| e.element_type == RELATIONSHIP_TYPE)
        .map(|e| (e.source.clone().unwrap(), e.target.clone().unwrap()))
        .collect();
    assert!(lines.contains(&(web, api.clone())), "{lines:?}");
    assert!(lines.contains(&(api.clone(), primary)), "{lines:?}");
    assert!(lines.contains(&(api, replica)), "{lines:?}");
    assert!(lines.contains(&("dns".to_owned(), "lb".to_owned())), "{lines:?}");
}
