//! A map's document id and its linked nodes (etalii.adp spec 089; mindmap.md section 3.8): the plugin reads the
//! central topic's `adp-id` as the root's `documentId` and a node with an `adp-link` as a `LinkedNode`, and plans the
//! load-time step's document id, a refreshed link and a new linked node as the bytes the section says.

mod common;

use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_cel::Value;
use adp_diagram_mindmap::document::new_id;
use adp_diagram_mindmap::plugin::{LINKED_ELEMENT_ATTRIBUTE, LINKED_NODE_TYPE, MindmapFreeplanePlugin, NODE_TYPE};
use adp_fbl::plugins::PluginBody;
use adp_fbl::{ModelChange, PlanResult};

fn ideas() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../vendor/etalii.adp/specifications/fbl/fixtures/links-workspace/ideas/ideas.mm")
}

fn body(text: &str) -> PluginBody {
    let mut tools = common::tools();
    let binding = tools
        .get(adp_diagram_mindmap::ORIGIN)
        .expect("the mind map loads")
        .binding
        .clone();
    PluginBody::open(
        text.as_bytes().to_vec(),
        binding,
        Some(Rc::new(MindmapFreeplanePlugin)),
        "ideas.mm",
    )
    .unwrap()
}

fn planned(body: &mut PluginBody, change: ModelChange) -> String {
    let planned = body.change(&change);
    assert!(matches!(planned, PlanResult::Planned(_)), "{planned:?}");
    String::from_utf8_lossy(body.bytes()).into_owned()
}

fn set_link(id: &str, link: &str) -> ModelChange {
    let mut attributes = adp_cel::Map::default();
    attributes.insert(LINKED_ELEMENT_ATTRIBUTE, Value::String(link.into()));
    ModelChange::Set {
        id: id.into(),
        attributes,
    }
}

#[test]
fn the_fixture_is_read_with_its_document_id_and_its_linked_nodes() {
    let model = body(&std::fs::read_to_string(ideas()).unwrap()).model();
    assert_eq!(model.document_id.as_deref(), Some("m8c1u4wa"));
    let root = &model.elements[0];
    assert_eq!(root.type_name, NODE_TYPE);
    assert!(root.attributes.get(LINKED_ELEMENT_ATTRIBUTE).is_none());
    let launch = model.find("ID_2").unwrap();
    assert_eq!(launch.type_name, LINKED_NODE_TYPE);
    assert_eq!(
        launch.attributes.get(LINKED_ELEMENT_ATTRIBUTE),
        Some(&Value::String("$t5h8w1rz@release.tml#launch.text=Launch".into()))
    );
    assert_eq!(launch.attributes.get("text"), Some(&Value::String("Launch".into())));
    // Every node under the central topic holds a link.
    assert!(model.elements[1..].iter().all(|e| e.type_name == LINKED_NODE_TYPE));
}

#[test]
fn a_map_without_an_id_is_given_one_as_the_central_topics_first_child_on_a_line_of_its_own() {
    let mut map = body(
        "<map version=\"freeplane 1.11.5\">\r\n<node TEXT=\"root\" ID=\"ID_1\">\r\n<node TEXT=\"a\" ID=\"ID_2\"/>\r\n</node>\r\n</map>\r\n",
    );
    assert_eq!(map.model().document_id, None);
    let written = planned(&mut map, ModelChange::SetDocumentId { id: "a1b2c3d4".into() });
    assert_eq!(
        written,
        "<map version=\"freeplane 1.11.5\">\r\n<node TEXT=\"root\" ID=\"ID_1\">\r\n<attribute NAME=\"adp-id\" VALUE=\"a1b2c3d4\"/>\r\n<node TEXT=\"a\" ID=\"ID_2\"/>\r\n</node>\r\n</map>\r\n"
    );
    assert_eq!(map.model().document_id.as_deref(), Some("a1b2c3d4"));
}

#[test]
fn the_document_id_goes_after_the_central_topics_formatted_text_and_into_an_empty_map_too() {
    let mut rich = body(
        "<map version=\"freeplane 1.11.5\">\n<node ID=\"ID_1\"><richcontent TYPE=\"NODE\"><html><head/><body><p>root</p></body></html></richcontent>\n<node TEXT=\"a\" ID=\"ID_2\"/>\n</node>\n</map>\n",
    );
    assert_eq!(
        planned(&mut rich, ModelChange::SetDocumentId { id: "a1b2c3d4".into() }),
        "<map version=\"freeplane 1.11.5\">\n<node ID=\"ID_1\"><richcontent TYPE=\"NODE\"><html><head/><body><p>root</p></body></html></richcontent>\n<attribute NAME=\"adp-id\" VALUE=\"a1b2c3d4\"/>\n<node TEXT=\"a\" ID=\"ID_2\"/>\n</node>\n</map>\n"
    );
    let mut empty = body("<map version=\"freeplane 1.11.5\">\n<node TEXT=\"root\" ID=\"ID_1\"/>\n</map>\n");
    assert_eq!(
        planned(&mut empty, ModelChange::SetDocumentId { id: "a1b2c3d4".into() }),
        "<map version=\"freeplane 1.11.5\">\n<node TEXT=\"root\" ID=\"ID_1\">\n<attribute NAME=\"adp-id\" VALUE=\"a1b2c3d4\"/>\n</node>\n</map>\n"
    );
}

#[test]
fn a_refreshed_link_writes_its_value_and_the_name_it_caches_as_the_nodes_text() {
    let mut map = body(&std::fs::read_to_string(ideas()).unwrap());
    let written = planned(&mut map, set_link("ID_7", "$m8c1u4wa@ideas.mm#ID_2.text=Launch"));
    assert!(
        written.contains(
            "<node TEXT=\"Launch\" ID=\"ID_7\">\n<attribute NAME=\"adp-link\" VALUE=\"$m8c1u4wa@ideas.mm#ID_2.text=Launch\"/>\n</node>"
        ),
        "{written}"
    );
    // Nothing else of the map changed.
    let before = std::fs::read_to_string(ideas()).unwrap();
    assert_eq!(
        written
            .replace(
                "$m8c1u4wa@ideas.mm#ID_2.text=Launch",
                "$m8c1u4wa@ideas.mm#ID_2.text=Start"
            )
            .replacen(
                "<node TEXT=\"Launch\" ID=\"ID_7\">",
                "<node TEXT=\"Start\" ID=\"ID_7\">",
                1
            ),
        before
    );
}

#[test]
fn a_cached_name_is_escaped_in_both_attributes_and_formatted_text_gives_way_to_it() {
    let mut map = body(
        "<map version=\"freeplane 1.11.5\">\n<node TEXT=\"root\" ID=\"ID_1\">\n<node ID=\"ID_2\"><richcontent TYPE=\"NODE\"><html><head/><body><p>old</p></body></html></richcontent>\n<attribute NAME=\"adp-link\" VALUE=\"$a1b2c3d4@x.tml#e=old\"/>\n</node>\n</node>\n</map>\n",
    );
    let written = planned(&mut map, set_link("ID_2", "$a1b2c3d4@x.tml#e=R&D \"<new>\""));
    assert!(
        written.contains(
            "<node ID=\"ID_2\" TEXT=\"R&amp;D &quot;&lt;new&gt;&quot;\">\n<attribute NAME=\"adp-link\" VALUE=\"$a1b2c3d4@x.tml#e=R&amp;D &quot;&lt;new&gt;&quot;\"/>"
        ),
        "{written}"
    );
}

#[test]
fn a_linked_node_is_added_as_the_last_child_holding_its_link_under_the_name_it_caches() {
    let mut map = body(
        "<map version=\"freeplane 1.11.5\">\n<node TEXT=\"root\" ID=\"ID_1\">\n<node TEXT=\"a\" ID=\"ID_2\"/>\n</node>\n</map>\n",
    );
    let mut attributes = adp_cel::Map::default();
    attributes.insert(
        LINKED_ELEMENT_ATTRIBUTE,
        Value::String("$t5h8w1rz@release.tml#launch.text=Launch".into()),
    );
    let written = planned(
        &mut map,
        ModelChange::Add {
            id: Some("ID_new00001".into()),
            type_name: LINKED_NODE_TYPE.into(),
            attributes,
            parent_id: Some("ID_1".into()),
            slot: None,
            index: None,
        },
    );
    assert_eq!(
        written,
        "<map version=\"freeplane 1.11.5\">\n<node TEXT=\"root\" ID=\"ID_1\">\n<node TEXT=\"a\" ID=\"ID_2\"/>\n<node TEXT=\"Launch\" ID=\"ID_new00001\">\n<attribute NAME=\"adp-link\" VALUE=\"$t5h8w1rz@release.tml#launch.text=Launch\"/>\n</node>\n</node>\n</map>\n"
    );
    assert_eq!(map.model().find("ID_new00001").unwrap().type_name, LINKED_NODE_TYPE);
}

#[test]
fn a_new_node_id_is_id_and_eight_characters_of_base_36() {
    let id = new_id();
    assert_eq!(id.len(), 11, "{id}");
    assert!(id.starts_with("ID_"));
    assert!(
        id[3..].chars().all(|c| c.is_ascii_digit() || c.is_ascii_lowercase()),
        "{id}"
    );
}
