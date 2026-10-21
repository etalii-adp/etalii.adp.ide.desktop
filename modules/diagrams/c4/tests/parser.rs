//! Reading the DSL subset ADP models: the model, the deployment model, relationships, views and styles (c4-diagrams
//! tasks 7 to 11). What a document means is not judged here, so an incomplete model still parses. Ported from the
//! standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/C4Parser.Tests.cs` at `8b6ccfc`.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use adp_diagram_c4::model::{C4ElementKind, C4ViewKind, C4Workspace};
use adp_diagram_c4::parser::parse_text;

fn parse_fixture(name: &str) -> C4Workspace {
    parse_text(&fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)).unwrap())
}

// ---- the static model -----------------------------------------------------------------------------------------------

#[test]
fn parse_reads_people_systems_and_containers_with_their_descriptions_and_technologies() {
    let workspace = parse_fixture("comments-everywhere.dsl");
    let user = workspace.find("u").unwrap();
    assert_eq!(user.kind, C4ElementKind::Person);
    assert_eq!(user.name, "User");
    assert_eq!(user.description, "A user.");
    let web = workspace.find("web").unwrap();
    assert_eq!(web.kind, C4ElementKind::Container);
    assert_eq!(web.description, "Serves pages.");
    assert_eq!(web.technology, "React");
    assert_eq!(web.parent_id.as_deref(), Some("s"));
}

#[test]
fn parse_is_not_fooled_by_comments_in_any_position() {
    let workspace = parse_fixture("comments-everywhere.dsl");
    assert_eq!(workspace.elements.len(), 4);
    assert_eq!(workspace.name, "Comments");
}

#[test]
fn parse_ignores_model_keywords_inside_a_block_comment() {
    let workspace = parse_text(
        "workspace \"W\" {\n  model {\n    /* person \"Ghost\"\n       softwareSystem \"Phantom\" */\n    u = person \"Real\"\n  }\n}\n",
    );
    assert_eq!(workspace.elements.len(), 1);
    assert_eq!(workspace.elements[0].name, "Real");
}

#[test]
fn parse_reads_an_element_with_no_identifier_by_generating_one() {
    let workspace = parse_text("workspace {\n  model {\n    person \"Anonymous\"\n  }\n}\n");
    assert_eq!(workspace.elements.len(), 1);
    let element = &workspace.elements[0];
    assert_eq!(element.name, "Anonymous");
    assert!(!element.id.is_empty());
}

#[test]
fn parse_treats_a_group_as_a_cluster_rather_than_an_abstraction_level() {
    let workspace =
        parse_text("workspace {\n  model {\n    group \"Team\" {\n      a = softwareSystem \"A\"\n    }\n  }\n}\n");
    assert_eq!(workspace.find("a").unwrap().parent_id, None);
}

#[test]
fn parse_reads_external_from_tags() {
    let workspace = parse_text(
        "workspace {\n  model {\n    a = softwareSystem \"A\" \"desc\" \"External\"\n    b = softwareSystem \"B\"\n  }\n}\n",
    );
    assert!(workspace.find("a").unwrap().is_external());
    assert!(!workspace.find("b").unwrap().is_external());
}

// ---- the deployment model -------------------------------------------------------------------------------------------

#[test]
fn parse_reads_nested_deployment_nodes_to_any_depth() {
    let workspace = parse_fixture("deployment-nested.dsl");
    let node = workspace.find("node").unwrap();
    assert_eq!(node.kind, C4ElementKind::DeploymentNode);
    assert_eq!(node.parent_id.as_deref(), Some("cluster"));
    assert_eq!(workspace.find("cluster").unwrap().parent_id.as_deref(), Some("region"));
}

#[test]
fn parse_reads_infrastructure_nodes_as_their_own_kind() {
    let workspace = parse_fixture("deployment-nested.dsl");
    assert_eq!(workspace.find("dns").unwrap().kind, C4ElementKind::InfrastructureNode);
    assert_eq!(workspace.find("lb").unwrap().kind, C4ElementKind::InfrastructureNode);
}

#[test]
fn parse_reads_a_container_deployed_twice_as_two_instances_of_one_container() {
    let workspace = parse_fixture("deployment-nested.dsl");
    let mut parents: Vec<String> = workspace
        .elements
        .iter()
        .filter(|e| e.kind == C4ElementKind::ContainerInstance && e.referenced_id.as_deref() == Some("db"))
        .map(|e| e.parent_id.clone().unwrap_or_default())
        .collect();
    parents.sort();
    assert_eq!(parents, ["primary", "replica"]);
}

// ---- relationships --------------------------------------------------------------------------------------------------

#[test]
fn parse_reads_relationships_with_description_and_technology() {
    let workspace = parse_fixture("comments-everywhere.dsl");
    let found: Vec<_> = workspace
        .relationships
        .iter()
        .filter(|r| r.source_id == "web" && r.destination_id == "db")
        .collect();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].description, "Reads from and writes to");
    assert_eq!(found[0].technology, "SQL/TCP");
}

#[test]
fn parse_reads_a_relationship_declared_inside_an_element_block() {
    let workspace = parse_text(
        "workspace {\n  model {\n    a = softwareSystem \"A\" {\n      -> b \"Calls\" \"HTTPS\"\n    }\n    b = softwareSystem \"B\"\n  }\n}\n",
    );
    assert_eq!(workspace.relationships.len(), 1);
    let relationship = &workspace.relationships[0];
    assert_eq!(relationship.source_id, "a");
    assert_eq!(relationship.destination_id, "b");
    assert_eq!(relationship.description, "Calls");
}

#[test]
fn a_relationships_id_is_stable_and_distinguishes_parallel_edges() {
    let workspace = parse_text(
        "workspace {\n  model {\n    a = softwareSystem \"A\"\n    b = softwareSystem \"B\"\n    a -> b \"One\"\n    a -> b \"Two\"\n  }\n}\n",
    );
    let ids: HashSet<String> = workspace.relationships.iter().map(|r| r.id()).collect();
    assert_eq!(ids.len(), 2);
}

// ---- views ----------------------------------------------------------------------------------------------------------

#[test]
fn parse_reads_each_view_kind_with_its_scope() {
    let workspace = parse_fixture("comments-everywhere.dsl");
    let context = workspace.find_view("context").unwrap();
    assert_eq!(context.kind, C4ViewKind::SystemContext);
    assert_eq!(context.scope_id.as_deref(), Some("s"));
    assert!(context.includes_everything);
    assert_eq!(workspace.find_view("containers").unwrap().kind, C4ViewKind::Container);
}

#[test]
fn parse_reads_a_deployment_views_environment() {
    let workspace = parse_fixture("deployment-nested.dsl");
    let view = workspace.find_view("production").unwrap();
    assert_eq!(view.kind, C4ViewKind::Deployment);
    assert_eq!(view.scope_id.as_deref(), Some("s"));
    assert_eq!(view.environment.as_deref(), Some("Production"));
}

#[test]
fn parse_reads_a_landscape_view_with_no_scope() {
    let workspace = parse_text(
        "workspace {\n  model {\n  }\n  views {\n    systemLandscape \"all\" {\n      include *\n    }\n  }\n}\n",
    );
    assert_eq!(workspace.views.len(), 1);
    assert_eq!(workspace.views[0].kind, C4ViewKind::SystemLandscape);
    assert_eq!(workspace.views[0].scope_id, None);
}

#[test]
fn parse_reads_a_dynamic_views_interactions_in_order() {
    let workspace = parse_fixture("dynamic-interactions.dsl");
    let view = workspace.find_view("signup").unwrap();
    assert_eq!(view.kind, C4ViewKind::Dynamic);
    assert_eq!(view.interactions.len(), 4);
    let orders: Vec<&str> = view.interactions.iter().map(|i| i.order.as_str()).collect();
    assert_eq!(orders, ["1", "2", "3", "4"]);
    assert_eq!(view.interactions[0].description, "Submits the form");
    assert_eq!(view.interactions[2].source_id, "api");
}

#[test]
fn parse_reads_auto_layout_with_its_direction() {
    let workspace = parse_fixture("deployment-nested.dsl");
    let view = workspace.find_view("production").unwrap();
    assert_eq!(view.auto_layout.as_ref().unwrap().direction, "lr");
}

#[test]
fn parse_leaves_auto_layout_null_when_the_document_declares_none() {
    let workspace = parse_fixture("comments-everywhere.dsl");
    assert_eq!(workspace.find_view("context").unwrap().auto_layout, None);
}

#[test]
fn the_first_view_is_what_a_bare_document_opens() {
    assert_eq!(parse_fixture("comments-everywhere.dsl").views[0].key, "context");
}

// ---- styles ---------------------------------------------------------------------------------------------------------

#[test]
fn parse_reads_tag_based_element_styles() {
    let workspace = parse_fixture("unmodelled-constructs.dsl");
    let tags: Vec<&str> = workspace.styles.iter().map(|s| s.tag.as_str()).collect();
    assert_eq!(tags, ["Person", "External"]);
    let lines: Vec<u32> = workspace.styles.iter().map(|s| s.line).collect();
    assert_eq!(lines, [21, 26]);
}

#[test]
fn parse_reads_no_styles_when_the_document_declares_none() {
    assert!(parse_fixture("minimal.dsl").styles.is_empty());
}

// ---- what it does not model -----------------------------------------------------------------------------------------

#[test]
fn parse_ignores_constructs_it_does_not_model_without_losing_the_ones_it_does() {
    let workspace = parse_fixture("unmodelled-constructs.dsl");
    assert_eq!(workspace.find("u").unwrap().id, "u");
    assert_eq!(workspace.find("web").unwrap().parent_id.as_deref(), Some("s"));
    assert_eq!(workspace.views.len(), 1);
}

#[test]
fn parse_survives_unusual_indentation() {
    let workspace = parse_fixture("unusual-indentation.dsl");
    assert_eq!(workspace.find("api").unwrap().parent_id.as_deref(), Some("s"));
    assert_eq!(workspace.views.len(), 1);
}

#[test]
fn parse_of_an_empty_document_is_an_empty_workspace() {
    let workspace = parse_text("");
    assert!(workspace.elements.is_empty());
    assert!(workspace.views.is_empty());
}

#[test]
fn parse_reads_the_same_model_whatever_the_line_endings() {
    for name in [
        "minimal.dsl",
        "crlf-line-endings.dsl",
        "lf-line-endings.dsl",
        "no-trailing-newline.dsl",
    ] {
        let workspace = parse_fixture(name);
        assert_eq!(workspace.elements.len(), 2, "{name}");
        assert_eq!(workspace.relationships.len(), 1, "{name}");
    }
}
