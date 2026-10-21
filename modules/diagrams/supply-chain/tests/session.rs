//! A session over the automotive example: what it delivers, and a drag of a group (the standalone host's
//! `SupplyChainSessionTests` and `SupplyChainContextSourceResolverTests`). The standalone host's sessions traced the
//! selection their connection made; here the canvas knows its selection and traces what it was given
//! ([`adp_diagram_supply_chain::trace::TraceGraph::of_elements`]), so every element arrives untraced.

mod common;

use std::fs;
use std::path::Path;

use adp_diagram::{DiagramElement, DiagramWorkspace};
use adp_diagram_supply_chain::layout::SupplyChainLayout;
use adp_diagram_supply_chain::mapper::{FLOW_TYPE, GROUP_TYPE};
use adp_diagram_supply_chain::view::PARENT_REFUSAL;
use common::{AUTOMOTIVE, fixtures, read, read_text, tools};

struct Example {
    _folder: tempfile::TempDir,
    body: std::path::PathBuf,
    workspace: DiagramWorkspace,
    session: adp_diagram::DiagramSession,
}

/// The automotive example, opened through its registration.
fn automotive() -> Example {
    let folder = tempfile::tempdir().unwrap();
    for name in ["automotive.adp", "automotive.supply"] {
        fs::copy(
            fixtures().join("examples/automotive").join(name),
            folder.path().join(name),
        )
        .unwrap();
    }
    let mut workspace = DiagramWorkspace::new(folder.path());
    let (opened, session) = workspace.open(&mut tools(), Path::new("automotive.adp")).unwrap();
    assert_eq!("etalii/supply-chain", opened.origin);
    Example {
        body: folder.path().join("automotive.supply"),
        _folder: folder,
        workspace,
        session,
    }
}

impl Example {
    fn delivered(&mut self) -> Vec<DiagramElement> {
        let deltas = self.session.baseline(self.workspace.documents()).unwrap();
        match deltas.into_iter().next() {
            Some(adp_diagram::DiagramDelta::Add(elements)) => elements,
            other => panic!("{other:?}"),
        }
    }
}

#[test]
fn a_fresh_session_delivers_every_group_node_and_flow_groups_first() {
    let model = read(AUTOMOTIVE);
    let mut example = automotive();
    let delivered = example.delivered();

    let mut expected: Vec<&str> = model
        .groups
        .iter()
        .map(|g| g.id.as_str())
        .chain(model.nodes.iter().map(|n| n.id.as_str()))
        .chain(model.flows.iter().map(|f| f.id.as_str()))
        .collect();
    expected.sort_unstable();
    let mut ids: Vec<&str> = delivered.iter().map(|e| e.id.as_str()).collect();
    ids.sort_unstable();
    assert_eq!(expected, ids);
    assert!(
        delivered[..model.groups.len()]
            .iter()
            .all(|e| e.element_type == GROUP_TYPE)
    );
    assert!(
        delivered
            .iter()
            .filter(|e| e.element_type != FLOW_TYPE)
            .all(|e| e.attribute("trace").is_empty())
    );
}

#[test]
fn a_node_arrives_with_its_stage_type_and_quantity_and_a_flow_with_its_weight() {
    let delivered = automotive().delivered();
    let plant = delivered.iter().find(|e| e.id == "vehicle-plant").unwrap();
    assert_eq!("etalii/supply-chain+integrator", plant.element_type);
    assert_eq!(
        ("Vehicle assembly plant", "2.8", "M vehicles", "europe"),
        (
            plant.attribute("name"),
            plant.attribute("quantity"),
            plant.attribute("unit"),
            plant.attribute("groupId")
        )
    );
    // The heaviest flow in its own unit is drawn at full width, a lighter one narrower.
    let weight = |id: &str| -> f64 {
        delivered
            .iter()
            .find(|e| e.id == id)
            .unwrap()
            .attribute("weight")
            .parse()
            .unwrap()
    };
    assert_eq!(1.0, weight("vehicle-plant-to-port-logistics"));
    assert!((weight("port-logistics-to-fleet-leasing") - 0.7 / 2.6).abs() < 1e-6);
}

#[test]
fn dragging_a_group_moves_every_member_by_the_same_distance() {
    let mut example = automotive();
    let before = SupplyChainLayout::of(&read(AUTOMOTIVE));
    let frame = before.group_boxes["central-africa"];

    example
        .session
        .move_element_to(&mut example.workspace, "central-africa", frame.x + 40.0, frame.y - 30.0)
        .unwrap();

    let after = read_text(&fs::read_to_string(&example.body).unwrap());
    let cobalt = after.node_of("kolwezi-cobalt").unwrap();
    assert_eq!(Some(before.node_boxes["kolwezi-cobalt"].x + 40.0), cobalt.x);
    assert_eq!(Some(before.node_boxes["kolwezi-cobalt"].y - 30.0), cobalt.y);
}

#[test]
fn a_selection_reads_as_the_name_of_what_it_names() {
    let example = automotive();
    let selected = example
        .session
        .select(example.workspace.documents(), "pack-assembly")
        .unwrap();
    assert_eq!(vec!["Battery pack assembly".to_owned()], selected.path);
    assert_eq!("etalii/supply-chain+integrator", selected.element.element_type);
    assert!(
        example
            .session
            .select(example.workspace.documents(), "nowhere")
            .is_none()
    );
}

#[test]
fn a_node_joins_a_group_by_being_dragged_into_it_never_by_a_re_parent() {
    let mut example = automotive();
    assert_eq!(
        Err(PARENT_REFUSAL.to_owned()),
        example
            .session
            .move_element(&mut example.workspace, "pack-assembly", Some("europe"), -1)
    );
}
