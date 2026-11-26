//! What a knowledge relation's *Create* adds to a mind map (etalii.adp spec 092, FR-025): the map offers a node, and
//! adds one with the typed text as the last child of the central topic, under an id of the map's own form, `ID_` and 8
//! characters. A linked node stands for another document's element and is not offered.

mod common;

use adp_diagram::element_creation::{NOTHING_TO_CREATE, create_in, diagrams_under};

const PLAIN: &str = "<map version=\"freeplane 1.11.5\">\n<node TEXT=\"Shop relaunch\" ID=\"ID_1\">\n<attribute NAME=\"adp-id\" VALUE=\"p4r8t1zc\"/>\n<node TEXT=\"Pricing\" ID=\"ID_2\"/>\n</node>\n</map>\n";

fn written(text: &str) -> std::path::PathBuf {
    let folder = tempfile::tempdir().unwrap().keep();
    let path = folder.join("ideas.mm");
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn the_map_offers_a_node_and_no_node_that_holds_a_link() {
    let linked = PLAIN.replace(
        "<node TEXT=\"Pricing\" ID=\"ID_2\"/>\n",
        "<node TEXT=\"Pricing\" ID=\"ID_2\"/>\n<node TEXT=\"Launch\" ID=\"ID_3\">\n<attribute NAME=\"adp-link\" VALUE=\"$r6d3w8ne@roadmap.tml#launch=Launch\"/>\n</node>\n",
    );
    let path = written(&linked);
    let found = diagrams_under(&mut common::tools(), path.parent().unwrap());
    let [map] = found.as_slice() else {
        panic!("one map: {found:?}")
    };
    let kinds: Vec<(&str, &str)> = map
        .kinds
        .iter()
        .map(|k| (k.type_name.as_str(), k.label.as_str()))
        .collect();
    assert_eq!(kinds, [("Node", "node")]);
    let ids: Vec<&str> = map.indexed.elements.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, ["ID_1", "ID_2"]);
}

#[test]
fn a_node_is_added_as_the_last_child_of_the_central_topic() {
    let created = create_in(&mut common::tools(), &written(PLAIN), "Node", "Payments").expect("added");
    let rest = created
        .id
        .strip_prefix("ID_")
        .unwrap_or_else(|| panic!("{}", created.id));
    assert!(
        rest.len() == 8 && rest.chars().all(|c| c.is_ascii_digit() || c.is_ascii_lowercase()),
        "{}",
        created.id
    );
    assert_eq!(
        String::from_utf8(created.body).unwrap(),
        PLAIN.replace(
            "<node TEXT=\"Pricing\" ID=\"ID_2\"/>\n",
            &format!(
                "<node TEXT=\"Pricing\" ID=\"ID_2\"/>\n<node TEXT=\"Payments\" ID=\"{}\"/>\n",
                created.id
            )
        )
    );
}

#[test]
fn a_kind_the_map_does_not_offer_adds_nothing() {
    assert_eq!(
        create_in(&mut common::tools(), &written(PLAIN), "Moment", "Payments"),
        Err(NOTHING_TO_CREATE.to_owned())
    );
}
