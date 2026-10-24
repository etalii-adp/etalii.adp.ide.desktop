//! A declared binding plans an add at a position and a move to another parent as a plugin that declares
//! `add.position` and `move` does (FBL sections 5.5 and 6.4), over the mind map's binding and the maps of its
//! conformance case `example-1`, whose bytes the mind map module wrote.

use std::path::Path;
use std::rc::Rc;

use adp_cel::{Map, Value};

use super::{ModelChange, PlanResult};
use crate::documents::{self, ProblemSeverity};
use crate::history::OpenBody;
use crate::model::FblOptions;
use crate::splice::Operation;

fn vendor(path: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor/etalii.adp")
        .join(path)
}

fn map_body(after: &str) -> OpenBody {
    let (problems, loaded) = documents::load_file(&vendor("specifications/fbl/mindmap.fbl"));
    assert!(
        !problems.iter().any(|p| p.severity == ProblemSeverity::Error),
        "{problems:?}"
    );
    let binding = Rc::new(loaded.unwrap().binding("freeplane").unwrap().clone());
    OpenBody::open(
        std::fs::read(example(after)).unwrap(),
        binding,
        Rc::new(FblOptions {
            file_name: "mindmap.mm".into(),
            ..FblOptions::default()
        }),
    )
}

fn example(file: &str) -> std::path::PathBuf {
    vendor("definitions/diagrams/conformance/mindmap/example-1").join(file)
}

fn expected(file: &str) -> String {
    String::from_utf8(std::fs::read(example(file)).unwrap()).unwrap()
}

fn planned(result: PlanResult) -> Vec<Operation> {
    let PlanResult::Planned(edit) = result else {
        panic!("{result:?}");
    };
    edit.splices.iter().map(|s| s.operation).collect()
}

fn add_node(id: &str, text: &str, parent: &str, index: i64) -> ModelChange {
    let mut attributes = Map::default();
    attributes.insert("text", Value::String(text.into()));
    ModelChange::Add {
        type_name: "Node".into(),
        id: Some(id.into()),
        attributes,
        parent_id: Some(parent.into()),
        index: Some(index),
        slot: None,
    }
}

fn move_node(id: &str, parent: &str, index: i64) -> ModelChange {
    ModelChange::Move {
        id: id.into(),
        new_parent_id: Some(parent.into()),
        index,
    }
}

#[test]
fn an_add_at_an_index_goes_directly_before_the_sibling_there() {
    let mut body = map_body("after-1.mm");

    // Physics after Scene graph, before Layout: the second child of the root.
    let result = body.change(&add_node(
        "ID_d0e6nrsnd00g33qyexmxh4uim",
        "Physics",
        "ID_5eb2r180uk2p4ys1q0tkiwil6",
        1,
    ));

    assert_eq!(planned(result), [Operation::InsertEntry]);
    assert_eq!(String::from_utf8_lossy(body.bytes()), expected("after-2.mm"));
}

#[test]
fn an_add_at_the_number_of_siblings_goes_after_the_last() {
    let mut body = map_body("after-1.mm");
    let before = expected("after-1.mm");

    let result = body.change(&add_node("ID_new", "Last", "ID_1guw8w5hp34usdnp25phle3s4", 6));

    assert_eq!(planned(result), [Operation::InsertEntry]);
    let culling = "<node TEXT=\"Culling helpers\" ID=\"ID_7w0itn6r5ibqgk8q7ifapvulm\"/>\n";
    assert_eq!(
        String::from_utf8_lossy(body.bytes()),
        before.replace(culling, &format!("{culling}<node TEXT=\"Last\" ID=\"ID_new\"/>\n"))
    );
}

#[test]
fn a_move_to_another_parent_carries_the_entry_before_the_sibling_at_its_index() {
    let mut body = map_body("after-8.mm");

    // Transforms from Scene graph to the first child of Physics, as the parity script's gesture moved it.
    let result = body.change(&move_node(
        "ID_a937863j5r3fjwlgwak0fgiqo",
        "ID_d0e6nrsnd00g33qyexmxh4uim",
        0,
    ));

    assert_eq!(planned(result), [Operation::RemoveEntry, Operation::InsertEntry]);
    assert_eq!(String::from_utf8_lossy(body.bytes()), expected("after-9.mm"));
}

#[test]
fn a_move_to_another_parent_past_its_children_goes_after_the_last() {
    let mut body = map_body("after-8.mm");
    let transforms = "<node TEXT=\"Transforms\" ID=\"ID_a937863j5r3fjwlgwak0fgiqo\"/>\n";
    let soft_bodies_end = "<p>Cloth and ropes.</p></body></html></richcontent>\n</node>\n";

    let result = body.change(&move_node(
        "ID_a937863j5r3fjwlgwak0fgiqo",
        "ID_d0e6nrsnd00g33qyexmxh4uim",
        -1,
    ));

    assert_eq!(planned(result), [Operation::RemoveEntry, Operation::InsertEntry]);
    assert_eq!(
        String::from_utf8_lossy(body.bytes()),
        expected("after-8.mm")
            .replace(transforms, "")
            .replace(soft_bodies_end, &format!("{soft_bodies_end}{transforms}"))
    );
}

#[test]
fn a_move_into_a_node_without_children_opens_it_and_keeps_the_branch() {
    let mut body = map_body("after-15.mm");

    // Autosave, with its child, from Storage into the self-closed Input handling.
    let result = body.change(&move_node(
        "ID_btdpge28u4joa6ni21k7cpe3c",
        "ID_1z8lmh0t5m59d4qdgnded2jtd",
        -1,
    ));

    planned(result);
    assert_eq!(String::from_utf8_lossy(body.bytes()), expected("after-16.mm"));
}

#[test]
fn a_move_into_its_own_branch_is_refused_and_writes_nothing() {
    let mut body = map_body("after-15.mm");

    // Storage into its own child Autosave.
    let result = body.change(&move_node(
        "ID_51ljf5yl355i4knxrpnzmbiar",
        "ID_btdpge28u4joa6ni21k7cpe3c",
        0,
    ));

    assert_eq!(
        result,
        PlanResult::Refused("A Node cannot be moved into its own branch.".into())
    );
    assert_eq!(String::from_utf8_lossy(body.bytes()), expected("after-15.mm"));
}
