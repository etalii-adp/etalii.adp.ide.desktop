//! The palette a C4 view offers: exactly what it permits, and nothing it would then refuse (etalii.adp spec 030, task
//! T061).
//!
//! Ported from the standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/C4ToolboxProvider.Tests.cs`
//! at `8b6ccfc`.

use adp_diagram::ToolboxItem;
use adp_diagram_c4::model::C4ViewKind;
use adp_diagram_c4::{rules, toolbox};

fn items(kind: C4ViewKind) -> Vec<ToolboxItem> {
    toolbox::items_for(kind)
}

fn labels(kind: C4ViewKind) -> Vec<String> {
    items(kind).into_iter().map(|item| item.label).collect()
}

#[test]
fn a_context_view_offers_people_and_software_systems_only() {
    assert_eq!(labels(C4ViewKind::SystemContext), vec!["Person", "Software System"]);
}

#[test]
fn a_container_view_offers_containers_but_not_components() {
    let labels = labels(C4ViewKind::Container);
    assert!(labels.contains(&"Container".to_owned()));
    assert!(!labels.contains(&"Component".to_owned()));
}

#[test]
fn a_deployment_view_offers_nodes_and_instances_and_no_people() {
    let labels = labels(C4ViewKind::Deployment);
    for expected in ["Deployment Node", "Infrastructure Node", "Container Instance"] {
        assert!(labels.contains(&expected.to_owned()), "{expected}");
    }
    assert!(!labels.contains(&"Person".to_owned()));
}

#[test]
fn no_view_offers_a_kind_it_would_then_refuse() {
    for kind in [
        C4ViewKind::SystemLandscape,
        C4ViewKind::SystemContext,
        C4ViewKind::Container,
        C4ViewKind::Component,
        C4ViewKind::Dynamic,
        C4ViewKind::Deployment,
    ] {
        assert_eq!(rules::permitted_kinds(kind).len(), items(kind).len(), "{kind:?}");
    }
}

#[test]
fn every_item_is_backed_by_an_action_so_a_drop_does_what_the_menu_does() {
    for item in items(C4ViewKind::Container) {
        assert!(!item.drop_action_id.is_empty());
        assert!(item.id.starts_with("c4."), "{}", item.id);
    }
}

#[test]
fn a_containers_description_says_it_is_not_a_docker_container() {
    let container = items(C4ViewKind::Container)
        .into_iter()
        .find(|item| item.label == "Container")
        .unwrap();
    assert!(container.description.to_lowercase().contains("not a docker container"));
}
