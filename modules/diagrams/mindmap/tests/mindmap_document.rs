//! The document against the real corpus: what it reads, what it writes back, and what its edits do to the XML behind
//! the tree. Ported from the standalone host's `MindmapDocument.Tests.cs`, `MindmapEmptyDocument.Tests.cs` and
//! `Fixtures.Tests.cs` at `8b6ccfc`.

mod common;

use adp_diagram_mindmap::document::MindmapDocument;
use adp_diagram_mindmap::xml::{Kind, XmlDocument};
use common::{corpus, corpus_path, load};

fn texts(document: &MindmapDocument, nodes: &[usize]) -> Vec<String> {
    nodes.iter().map(|n| document.text(*n)).collect()
}

// ---- the corpus itself ---------------------------------------------------------------------------------------------

fn corpus_xml() -> XmlDocument {
    XmlDocument::parse(&corpus()).expect("the corpus is well-formed")
}

#[test]
fn the_map_is_beside_the_tests() {
    assert!(corpus_path().is_file(), "{} is missing", corpus_path().display());
}

#[test]
fn the_map_is_well_formed_freeplane() {
    let xml = corpus_xml();
    let map = xml.root().unwrap();
    assert_eq!(xml.local_name(map), "map");
    assert!(xml.attribute(map, "version").unwrap().starts_with("freeplane"));
    assert_eq!(xml.elements_named(map, "node").count(), 1, "exactly one root node");
}

#[test]
fn the_map_stays_complex_enough_to_be_worth_testing() {
    let xml = corpus_xml();
    let nodes = xml.descendants_named(xml.root().unwrap(), "node");
    assert!(nodes.len() >= 20, "the corpus has shrunk to {} nodes", nodes.len());
    assert!(nodes.iter().all(|n| xml.attribute(*n, "ID").is_some()));
    assert!(nodes.iter().any(|n| xml.attribute(*n, "FOLDED") == Some("true")));
    assert!(nodes.iter().any(|n| xml.attribute(*n, "TEXT") == Some("")));
    assert!(nodes.iter().any(|n| xml.attribute(*n, "LINK").is_some()));
}

#[test]
fn the_map_carries_links_in_both_forms_requirement_12_names() {
    let xml = corpus_xml();
    let links: Vec<&str> = xml
        .descendants_named(xml.root().unwrap(), "node")
        .into_iter()
        .filter_map(|n| xml.attribute(n, "LINK"))
        .collect();
    assert!(links.iter().any(|l| l.starts_with("..")));
    assert!(links.iter().any(|l| l.starts_with("https://")));
}

#[test]
fn the_map_carries_elements_adp_does_not_understand() {
    let xml = corpus_xml();
    let root = xml.root().unwrap();
    for name in [
        "hook",
        "map_styles",
        "stylenode",
        "font",
        "edge",
        "icon",
        "cloud",
        "arrowlink",
        "attribute",
    ] {
        assert!(
            !xml.descendants_named(root, name).is_empty(),
            "the corpus no longer contains a '{name}' element"
        );
    }
}

#[test]
fn the_map_carries_rich_content_for_both_a_note_and_a_nodes_text() {
    let xml = corpus_xml();
    let kinds: Vec<&str> = xml
        .descendants_named(xml.root().unwrap(), "richcontent")
        .into_iter()
        .filter_map(|n| xml.attribute(n, "TYPE"))
        .collect();
    assert!(kinds.contains(&"NOTE"));
    assert!(kinds.contains(&"NODE"));
}

// ---- round trip ----------------------------------------------------------------------------------------------------

#[test]
fn parse_then_write_reproduces_the_corpus_byte_for_byte() {
    assert_eq!(load(false).to_text(), corpus());
}

#[test]
fn parse_then_write_is_stable_across_a_second_pass() {
    let once = load(true).to_text();
    let twice = MindmapDocument::parse(&once, true).unwrap().to_text();
    assert_eq!(once, twice);
}

#[test]
fn parse_then_write_keeps_an_element_and_an_attribute_it_does_not_understand() {
    let original = corpus()
        .replace("<node TEXT=\"Backend\"", "<node TEXT=\"Backend\" FUTURE_ATTR=\"kept\"")
        .replace(
            "<edge COLOR=\"#ff0000\" WIDTH=\"2\"/>",
            "<edge COLOR=\"#ff0000\" WIDTH=\"2\"/>\n<future_element answer=\"42\"><inner/></future_element>",
        );
    assert_ne!(original, corpus(), "both replacements found something to change");
    let written = MindmapDocument::parse(&original, false).unwrap().to_text();
    assert_eq!(written, original);
}

#[test]
fn parse_then_write_keeps_mixed_line_endings_and_an_escaped_carriage_return_in_an_attribute() {
    let mixed = "<map version=\"freeplane 1.12.15\">\r\n<node TEXT=\"a&#xd;b\" ID=\"ID_1\">\r\n<node TEXT=\"lf line\" ID=\"ID_2\"/>\n</node>\r\n</map>\r\n";
    assert_eq!(MindmapDocument::parse(mixed, false).unwrap().to_text(), mixed);
}

#[test]
fn assigning_missing_ids_changes_only_the_nodes_that_had_none() {
    let hand_written = "<map version=\"freeplane 1.11.5\">\n<node TEXT=\"root\" ID=\"ID_1\">\n<node TEXT=\"no id yet\">\n<node TEXT=\"child without one either\"/>\n</node>\n</node>\n</map>";
    let document = MindmapDocument::parse(hand_written, true).unwrap();
    assert!(document.nodes().iter().all(|n| !document.id(*n).is_empty()));
    let assigned = document
        .nodes()
        .into_iter()
        .find(|n| document.text(*n) == "no id yet")
        .unwrap();
    assert!(document.id(assigned).starts_with("ID_"));
    assert_eq!(document.id(document.parent(assigned).unwrap()), "ID_1");
    assert!(
        document
            .to_text()
            .contains(&format!("ID=\"{}\"", document.id(assigned)))
    );
}

// ---- reading -------------------------------------------------------------------------------------------------------

#[test]
fn parse_reads_the_tree() {
    let document = load(true);
    let root = document.root();
    assert_eq!(document.text(root), "ADP architecture");
    assert_eq!(
        texts(&document, &document.children(root)),
        ["Backend", "Client", "Diagram types", "Deliberately deep"]
    );
    assert_eq!(document.nodes().len(), 25);
}

#[test]
fn parse_reads_text_held_as_rich_content() {
    let document = load(true);
    let rich = document.find("ID_411002940").unwrap();
    assert_eq!(
        document.text(rich),
        "A node whose text is rich content rather than a TEXT attribute"
    );
}

#[test]
fn parse_reads_notes_with_and_without_a_content_type() {
    let document = load(true);
    assert!(
        document
            .notes(document.root())
            .starts_with("The map ADP's round-trip tests read.")
    );
    assert!(
        document
            .notes(document.find("ID_88117420").unwrap())
            .contains("<node> is not a node.")
    );
    assert_eq!(document.notes(document.find("ID_88117422").unwrap()), "");
}

#[test]
fn parse_reads_fold_link_and_empty_text() {
    let document = load(true);
    assert!(document.folded(document.find("ID_88117425").unwrap()));
    assert_eq!(
        document.link(document.find("ID_88117420").unwrap()),
        Some("../../../../backend/EtAlii.Adp.Backend/Context/ContextServiceImpl.cs")
    );
    assert_eq!(document.link(document.find("ID_88117422").unwrap()), None);
    assert_eq!(document.text(document.find("ID_411002939").unwrap()), "");
}

#[test]
fn parse_a_malformed_file_fails_naming_the_problem() {
    let error = MindmapDocument::parse("<map><node TEXT='unclosed'></map>", true).unwrap_err();
    assert!(error.message.contains("well-formed"), "{}", error.message);
}

#[test]
fn parse_a_file_that_is_not_a_map_fails() {
    assert!(MindmapDocument::parse("<html><body/></html>", true).is_err());
}

#[test]
fn parse_a_map_with_two_roots_fails() {
    assert!(MindmapDocument::parse("<map><node TEXT=\"a\"/><node TEXT=\"b\"/></map>", true).is_err());
}

#[test]
fn outcome_a_malformed_file_is_unusable_with_the_line_the_reader_stopped_at() {
    let outcome = MindmapDocument::outcome("<map>\n<node TEXT='unclosed'>\n</map>\n", true);
    assert!(!outcome.is_usable());
    let reason = outcome.unreadable.clone().unwrap();
    assert!(reason.contains("well-formed"), "{reason}");
    assert_eq!(outcome.problems, [(2, reason)]);
    assert!(outcome.document.children(outcome.document.root()).is_empty());
}

#[test]
fn outcome_a_map_with_two_roots_is_unusable_at_the_first_line() {
    let outcome = MindmapDocument::outcome("<map><node TEXT=\"a\"/><node TEXT=\"b\"/></map>", true);
    assert_eq!(
        outcome.problems,
        [(0, "A map has exactly one root node; this one has 2.".to_owned())]
    );
    assert_eq!(Some(outcome.problems[0].1.clone()), outcome.unreadable);
}

#[test]
fn outcome_a_map_is_usable_with_no_problems() {
    let outcome = MindmapDocument::outcome("<map><node TEXT=\"a\"/></map>", true);
    assert!(outcome.is_usable());
    assert!(outcome.problems.is_empty());
    assert_eq!(outcome.document.text(outcome.document.root()), "a");
}

// ---- structural edits ----------------------------------------------------------------------------------------------

#[test]
fn add_child_appends_as_the_last_child_and_writes_one_per_line() {
    let mut document = load(true);
    let backend = document.find("ID_411002937").unwrap();
    let added = document.add_child(backend, "Logging");
    assert_eq!(document.text(*document.children(backend).last().unwrap()), "Logging");
    assert!(document.id(added).starts_with("ID_"));
    // The corpus is a Windows save, so a synthesized structural newline matches its CRLF.
    let expected = format!("<node TEXT=\"Logging\" ID=\"{}\"/>\r\n</node>", document.id(added));
    assert!(document.to_text().contains(&expected));
}

#[test]
fn add_sibling_inserts_right_after_the_node() {
    let mut document = load(true);
    let context_service = document.find("ID_88117420").unwrap();
    document.add_sibling(context_service, "Projects").unwrap();
    let parent = document.parent(context_service).unwrap();
    assert_eq!(
        texts(&document, &document.children(parent)),
        ["Context service", "Projects", "Commands & history", "Hierarchy"]
    );
}

#[test]
fn move_carries_the_whole_subtree_to_the_given_parent_and_index() {
    let mut document = load(true);
    let hierarchy = document.find("ID_88117425").unwrap();
    let client = document.find("ID_411002938").unwrap();
    document.move_node(hierarchy, client, 0).unwrap();
    let first = document.children(client)[0];
    assert_eq!(document.text(first), "Hierarchy");
    assert_eq!(document.children(first).len(), 2, "its children came along");
    let backend = document.find("ID_411002937").unwrap();
    assert!(
        document
            .children(backend)
            .iter()
            .all(|c| document.id(*c) != "ID_88117425")
    );
}

#[test]
fn move_into_its_own_subtree_is_refused() {
    let mut document = load(true);
    let backend = document.find("ID_411002937").unwrap();
    let grandchild = document.find("ID_88117422").unwrap();
    assert!(document.move_node(backend, grandchild, 0).is_err());
}

#[test]
fn move_the_root_is_refused() {
    let mut document = load(true);
    let backend = document.find("ID_411002937").unwrap();
    assert!(document.move_node(document.root(), backend, 0).is_err());
}

#[test]
fn remove_then_restore_puts_the_subtree_back_exactly() {
    let mut document = load(true);
    let before = document.to_text();
    let hierarchy = document.find("ID_88117425").unwrap();
    let parent = document.parent(hierarchy).unwrap();
    let index = document.index_in_parent(hierarchy);
    let subtree = document.remove(hierarchy).unwrap();
    assert!(document.find("ID_88117425").is_none());
    assert!(document.find("ID_88117428").is_none(), "a grandchild went with it");
    document.restore(subtree, parent, index as i64);
    assert_eq!(document.to_text(), before);
}

#[test]
fn remove_the_root_is_refused() {
    let mut document = load(true);
    assert!(document.remove(document.root()).is_err());
}

// ---- content edits -------------------------------------------------------------------------------------------------

#[test]
fn set_text_on_a_rich_content_node_replaces_the_markup_with_plain_text() {
    let mut document = load(true);
    let rich = document.find("ID_411002940").unwrap();
    document.set_text(rich, "plain now");
    assert_eq!(document.text(rich), "plain now");
    assert!(!document.to_text().contains("rich content"));
}

#[test]
fn set_text_to_empty_is_kept() {
    let mut document = load(true);
    let node = document.find("ID_88117422").unwrap();
    document.set_text(node, "");
    assert_eq!(document.text(node), "");
    assert!(document.to_text().contains("TEXT=\"\" ID=\"ID_88117422\""));
}

#[test]
fn set_notes_adds_then_updates_then_removes_the_note() {
    let mut document = load(true);
    let node = document.find("ID_88117422").unwrap();
    let rich_contents = |document: &MindmapDocument| document.xml().elements_named(node, "richcontent").count();

    document.set_notes(node, "first line\nsecond line");
    assert_eq!(document.notes(node), "first line\nsecond line");

    document.set_notes(node, "changed");
    assert_eq!(document.notes(node), "changed");
    assert_eq!(rich_contents(&document), 1);

    document.set_notes(node, "");
    assert_eq!(document.notes(node), "");
    assert_eq!(rich_contents(&document), 0);
}

#[test]
fn set_link_sets_and_clears_the_attribute() {
    let mut document = load(true);
    let node = document.find("ID_88117422").unwrap();
    document.set_link(node, Some("../file.cs"));
    assert_eq!(document.link(node), Some("../file.cs"));
    document.set_link(node, None);
    assert_eq!(document.link(node), None);
}

#[test]
fn edits_escape_what_xml_requires_and_nothing_else() {
    let mut document = load(true);
    let node = document.find("ID_88117422").unwrap();
    document.set_text(node, "a & b < c > \"d\" — ü");
    let written = document.to_text();
    assert!(written.contains("TEXT=\"a &amp; b &lt; c &gt; &quot;d&quot; — ü\""));
    let read = MindmapDocument::parse(&written, true).unwrap();
    assert_eq!(read.text(read.find("ID_88117422").unwrap()), "a & b < c > \"d\" — ü");
}

// ---- an empty document ---------------------------------------------------------------------------------------------

#[test]
fn create_empty_document_has_one_root_named_after_the_file_and_nothing_else() {
    let document = MindmapDocument::parse(&MindmapDocument::empty_document("domain"), true).unwrap();
    assert_eq!(document.text(document.root()), "domain");
    assert!(document.children(document.root()).is_empty());
    assert_eq!(document.nodes().len(), 1);
    assert!(document.id(document.root()).starts_with("ID_"));
}

#[test]
fn create_empty_document_round_trips_byte_for_byte() {
    let text = MindmapDocument::empty_document("domain");
    assert_eq!(MindmapDocument::parse(&text, false).unwrap().to_text(), text);
}

#[test]
fn create_empty_document_escapes_the_name() {
    let text = MindmapDocument::empty_document("a & \"b\"");
    assert!(text.contains("TEXT=\"a &amp; &quot;b&quot;\""));
    let document = MindmapDocument::parse(&text, true).unwrap();
    assert_eq!(document.text(document.root()), "a & \"b\"");
}

#[test]
fn a_comment_and_an_unknown_element_are_nodes_of_the_xml() {
    let xml = corpus_xml();
    let map = xml.root().unwrap();
    assert!(
        xml.children(map)
            .iter()
            .any(|c| matches!(xml.kind(*c), Kind::Comment(_)))
    );
}
