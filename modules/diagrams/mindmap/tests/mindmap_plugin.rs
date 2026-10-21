//! The persistence plugin of `mindmap.fbl#mindmap` reads a Freeplane body into the elements the DISL model is built
//! from, one per node in document order, each under its parent, with an id of its own even where the file gives two
//! nodes one `ID` or a node none; and it plans a change as the one splice the standalone host's command and save came
//! to. Ported from the standalone host's `MindmapFreeplanePlugin.Tests.cs` at `8b6ccfc`.

mod common;

use std::rc::Rc;

use adp_cel::Value;
use adp_diagram_mindmap::plugin::{MindmapFreeplanePlugin, PLUGIN_ID, STORED_ID_ATTRIBUTE};
use adp_fbl::model::codes;
use adp_fbl::plugins::{PersistencePlugin, PluginBody, PluginFile, PluginReadRequest, PluginReadResult};
use adp_fbl::{ModelChange, PlanResult};

fn read(text: &str) -> PluginReadResult {
    MindmapFreeplanePlugin.read(&PluginReadRequest {
        files: vec![PluginFile::new("", text.as_bytes().to_vec())],
        supplementary_entries: Vec::new(),
    })
}

fn binding() -> Rc<adp_fbl::FblBinding> {
    let mut tools = common::tools();
    tools
        .get(adp_diagram_mindmap::ORIGIN)
        .expect("the mind map loads")
        .binding
        .clone()
}

#[test]
fn a_node_is_read_with_its_text_notes_link_fold_and_side_under_its_parent() {
    let text = "<map version=\"freeplane 1.12.15\">\n<node TEXT=\"root\" ID=\"ID_1\">\n<node TEXT=\"left\" ID=\"ID_2\" POSITION=\"left\" FOLDED=\"true\" LINK=\"notes.md\"><richcontent TYPE=\"NOTE\"><html><head/><body><p>A note</p></body></html></richcontent>\n<node TEXT=\"deep\" ID=\"ID_3\"/>\n</node>\n</node>\n</map>\n";
    let result = read(text);
    assert!(!result.unreadable);
    assert!(result.findings.is_empty());
    let ids: Vec<&str> = result.elements.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, ["ID_1", "ID_2", "ID_3"]);
    let left = &result.elements[1];
    assert!(left.id_is_stored);
    assert_eq!(left.parent_id.as_deref(), Some("ID_1"));
    assert_eq!(left.parent_slot.as_deref(), Some("children"));
    assert_eq!(left.attributes.get("text"), Some(&Value::String("left".into())));
    assert_eq!(left.attributes.get("notes"), Some(&Value::String("A note".into())));
    assert_eq!(left.attributes.get("link"), Some(&Value::String("notes.md".into())));
    assert_eq!(left.attributes.get("folded"), Some(&Value::Bool(true)));
    assert_eq!(left.attributes.get("position"), Some(&Value::String("left".into())));
    assert_eq!(
        left.attributes.get(STORED_ID_ATTRIBUTE),
        Some(&Value::String("ID_2".into()))
    );
    assert_eq!(left.line, 3);
    assert_eq!(result.elements[0].parent_id, None);
}

#[test]
fn a_link_is_read_only_when_the_file_has_one_even_an_empty_one() {
    let result = read(
        "<map version=\"freeplane 1.12.15\"><node TEXT=\"root\" ID=\"ID_1\" LINK=\"\"><node TEXT=\"plain\" ID=\"ID_2\"/></node></map>",
    );
    assert_eq!(
        result.elements[0].attributes.get("link"),
        Some(&Value::String(String::new()))
    );
    assert!(result.elements[1].attributes.get("link").is_none());
    assert!(result.elements[1].attributes.get("position").is_none());
}

#[test]
fn a_later_holder_of_an_id_and_a_node_without_one_get_ids_of_their_own_and_keep_what_the_file_writes() {
    let elements = read(
        "<map version=\"freeplane 1.12.15\"><node TEXT=\"root\" ID=\"ID_1\"><node TEXT=\"a\" ID=\"ID_A\"><node TEXT=\"under the first\" ID=\"ID_X\"/></node><node TEXT=\"a again\" ID=\"ID_A\"><node TEXT=\"under the second\" ID=\"ID_Y\"/></node><node TEXT=\"none\"/></node></map>",
    )
    .elements;
    let mut ids: Vec<&str> = elements.iter().map(|e| e.id.as_str()).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), elements.len());
    assert_eq!(elements[1].id, "ID_A");
    assert!(elements[1].id_is_stored);
    assert_ne!(elements[3].id, "ID_A");
    assert!(!elements[3].id_is_stored);
    assert_eq!(
        elements[3].attributes.get(STORED_ID_ATTRIBUTE),
        Some(&Value::String("ID_A".into()))
    );
    assert_eq!(elements[4].parent_id.as_deref(), Some(elements[3].id.as_str()));
    assert!(!elements[5].id_is_stored);
    assert_eq!(
        elements[5].attributes.get(STORED_ID_ATTRIBUTE),
        Some(&Value::String(String::new()))
    );
}

#[test]
fn a_body_that_is_not_a_map_is_unreadable_with_the_readers_reason() {
    let result = read(
        "<map version=\"freeplane 1.12.15\">\n<node TEXT=\"one\" ID=\"ID_1\"/>\n<node TEXT=\"two\" ID=\"ID_2\"/>\n</map>\n",
    );
    assert!(result.unreadable);
    assert!(result.elements.is_empty());
    assert_eq!(result.findings.len(), 1);
    assert_eq!(result.findings[0].code, codes::UNPARSEABLE);
    assert_eq!(
        result.findings[0].message,
        "A map has exactly one root node; this one has 2."
    );
}

#[test]
fn the_binding_names_this_plugin_as_its_reader() {
    assert_eq!(binding().plugin.as_ref().unwrap().plugin, PLUGIN_ID);
    assert_eq!(MindmapFreeplanePlugin.id(), PLUGIN_ID);
}

/// Where the desktop host departs from the standalone host, whose plugin refused every change because its commands
/// edited the XML in place: the change is planned as the one splice that writes what the command and the save wrote.
#[test]
fn a_change_is_planned_as_the_splice_the_command_wrote() {
    let text = "<map version=\"freeplane 1.12.15\">\n<node TEXT=\"root\" ID=\"ID_1\">\n<node TEXT=\"leaf\" ID=\"ID_2\"/>\n</node>\n</map>\n";
    let mut body = PluginBody::open(
        text.as_bytes().to_vec(),
        binding(),
        Some(Rc::new(MindmapFreeplanePlugin)),
        "map.mm",
    )
    .unwrap();
    let PlanResult::Planned(edit) = body.change(&ModelChange::Remove { id: "ID_2".into() }) else {
        panic!("the removal was not planned");
    };
    assert_eq!(edit.splices.len(), 1);
    assert_eq!(
        String::from_utf8_lossy(body.bytes()),
        "<map version=\"freeplane 1.12.15\">\n<node TEXT=\"root\" ID=\"ID_1\">\n</node>\n</map>\n"
    );
    assert!(matches!(
        body.change(&ModelChange::Remove { id: "ID_1".into() }),
        PlanResult::Refused(_)
    ));
}

#[test]
fn a_change_writes_the_ids_the_file_lacked_as_the_standalone_hosts_store_did() {
    let text = "<map version=\"freeplane 1.11.5\">\n<node TEXT=\"root\">\n<node TEXT=\"leaf\" ID=\"ID_2\"/>\n</node>\n</map>\n";
    let mut body = PluginBody::open(
        text.as_bytes().to_vec(),
        binding(),
        Some(Rc::new(MindmapFreeplanePlugin)),
        "map.mm",
    )
    .unwrap();
    let mut attributes = adp_cel::Map::default();
    attributes.insert("text", Value::String("changed".into()));
    assert!(matches!(
        body.change(&ModelChange::Set {
            id: "ID_2".into(),
            attributes
        }),
        PlanResult::Planned(_)
    ));
    let written = String::from_utf8_lossy(body.bytes()).into_owned();
    assert!(written.contains("<node TEXT=\"root\" ID=\"ID_"), "{written}");
    assert!(written.contains("<node TEXT=\"changed\" ID=\"ID_2\"/>"), "{written}");
}
