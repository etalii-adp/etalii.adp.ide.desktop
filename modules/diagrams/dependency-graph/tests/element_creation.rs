//! What a knowledge relation's *Create* adds to a dependency graph (etalii.adp spec 092, FR-025): the graph offers a
//! node, and adds one named as typed at the left, on a row of its own below every node, with the rest of the file as it
//! was. A linked node stands for another document's element and is not offered.

mod common;

use adp_diagram::element_creation::{NOTHING_TO_CREATE, create_in, diagrams_under};

const PLAIN: &str = "dependencies: 1\nid: s2v5b9hu\nelements:\n  - id: pay\n    label: Payments\n    x: 0\n    row: 0\n  - id: shop\n    label: Web shop\n    x: 240\n    row: 2\n";

fn written(text: &str) -> std::path::PathBuf {
    let folder = tempfile::tempdir().unwrap().keep();
    let path = folder.join("services.dgr");
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn the_graph_offers_a_node_and_no_node_that_holds_a_link() {
    let linked =
        format!("{PLAIN}  - id: elsewhere\n    link: $r6d3w8ne@roadmap.tml#launch=Launch\n    x: 0\n    row: 4\n");
    let path = written(&linked);
    let found = diagrams_under(&mut common::tools(), path.parent().unwrap());
    let [graph] = found.as_slice() else {
        panic!("one graph: {found:?}")
    };
    let kinds: Vec<(&str, &str)> = graph
        .kinds
        .iter()
        .map(|k| (k.type_name.as_str(), k.label.as_str()))
        .collect();
    assert_eq!(kinds, [("Node", "node")]);
    let ids: Vec<&str> = graph.indexed.elements.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, ["pay", "shop"]);
}

#[test]
fn a_node_is_added_at_the_left_on_the_row_below_the_last() {
    let created = create_in(&mut common::tools(), &written(PLAIN), "Node", "Billing").expect("added");
    assert_eq!(created.id.len(), 8, "{}", created.id);
    assert_eq!(
        String::from_utf8(created.body).unwrap(),
        format!(
            "{PLAIN}  - id: {}\n    label: Billing\n    x: 0\n    row: 3\n",
            created.id
        )
    );
}

#[test]
fn a_kind_the_graph_does_not_offer_adds_nothing() {
    assert_eq!(
        create_in(&mut common::tools(), &written(PLAIN), "Period", "Billing"),
        Err(NOTHING_TO_CREATE.to_owned())
    );
}
