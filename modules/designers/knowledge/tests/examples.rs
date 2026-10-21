//! The shipped examples (the standalone host's `Examples.Tests.cs` at 8b6ccfc; knowledge-designer Requirements 9.1 to
//! 9.4): every one reads with nothing to report and, written unchanged, keeps its bytes; the same table read from
//! each of the three formats is the same table, and is again after each kind of edit. The standalone host's tests of
//! its showcase project are not ported: the showcase is not part of this repository.

mod common;

use std::path::Path;
use std::rc::Rc;

use adp_designer_knowledge::relations::{self, KnowledgeTarget, SELF};
use adp_designer_knowledge::{KnowledgeBody, KnowledgeProperty, KnowledgeTable, validator};
use adp_desl::table::model::TableGesture;
use common::{EXTENSIONS, EditingTable, copy_example, describe, example_bytes, examples_folder, read_example};

/// What a relation of an example points at: the table itself, or the file beside it that it names.
fn target(path: &Path, own: &KnowledgeTable, property: &KnowledgeProperty) -> Option<Rc<KnowledgeTarget>> {
    if property.target_file == SELF || property.target_file.is_empty() {
        return Some(Rc::new(KnowledgeTarget {
            path: path.to_path_buf(),
            table: Some(own.clone()),
            problem: String::new(),
        }));
    }
    let at = relations::target_path(path, &property.target_file);
    let table = std::fs::read(&at)
        .ok()
        .map(|bytes| KnowledgeBody::read(bytes, &at.to_string_lossy()).table().clone());
    Some(Rc::new(KnowledgeTarget {
        path: at,
        table,
        problem: String::new(),
    }))
}

#[test]
fn an_example_reads_with_nothing_to_report_and_written_unchanged_keeps_its_bytes() {
    let mut files: Vec<_> = std::fs::read_dir(examples_folder())
        .expect("the examples")
        .map(|e| e.expect("an entry").path())
        .collect();
    files.sort();
    // The table in three formats and the table it relates to.
    assert_eq!(4, files.len());
    for path in files {
        let bytes = std::fs::read(&path).expect("read");
        let body = KnowledgeBody::read(bytes.clone(), &path.to_string_lossy());

        // Readable, editable, and neither the reading nor the designer's rules have anything to say.
        assert_eq!("", body.read_only_reason(), "{}", path.display());
        assert!(
            body.model().findings.is_empty(),
            "{}: {:?}",
            path.display(),
            body.model().findings
        );
        let target_of = |p: &KnowledgeProperty| target(&path, body.table(), p);
        let findings = validator::validate(body.table(), body.model(), Some(&target_of));
        assert!(findings.is_empty(), "{}: {findings:?}", path.display());
        assert!(!body.table().rows.is_empty());

        // Written with nothing changed, it is the bytes it was.
        assert_eq!(Ok(bytes), body.change(&[]));
    }
}

#[test]
fn the_same_table_read_from_each_format_is_the_same_table() {
    let tables: Vec<String> = EXTENSIONS
        .iter()
        .map(|e| describe(read_example(&format!("cities{e}")).table()))
        .collect();
    assert_eq!(tables[0], tables[1]);
    assert_eq!(tables[0], tables[2]);
    assert!(tables[0].contains("row r1"));
}

/// One edit of each kind that makes nothing new - what is made new is given an id that was never used, which
/// differs from file to file by design.
const EDITS: [&str; 24] = [
    "setCell",
    "clearCell",
    "setSeveral",
    "renameColumn",
    "setColumnType",
    "deleteColumn",
    "deleteRows",
    "renameOption",
    "recolourOption",
    "moveOption",
    "deleteOption",
    "renameView",
    "moveView",
    "deleteView",
    "hideColumn",
    "resizeColumn",
    "moveColumn",
    "addSort",
    "removeSort",
    "addFilter",
    "setFilter",
    "removeFilter",
    "groupBy",
    "toggleGroup",
];

fn gesture(edit: &str) -> TableGesture {
    match edit {
        "setCell" => TableGesture::new("setCell").row("r2").column("p3").values(["529247"]),
        "clearCell" => TableGesture::new("setCell").row("r1").column("p7"),
        "setSeveral" => TableGesture::new("setCell").row("r2").column("p4").values(["o4"]),
        "renameColumn" => TableGesture::new("renameColumn").column("p3").values(["Inhabitants"]),
        "setColumnType" => TableGesture::new("setColumnType").column("p3").setting("type", "text"),
        "deleteColumn" => TableGesture::new("deleteColumn").column("p2"),
        "deleteRows" => TableGesture::new("deleteRows").values(["r1"]),
        "renameOption" => TableGesture::new("renameOption")
            .column("p2")
            .target("o2")
            .values(["Flanders"]),
        "recolourOption" => TableGesture::new("recolourOption")
            .column("p4")
            .target("o3")
            .setting("colour", "green"),
        "moveOption" => TableGesture::new("moveOption").column("p2").target("o2").index(0),
        "deleteOption" => TableGesture::new("deleteOption").column("p4").target("o3"),
        "renameView" => TableGesture::new("renameView").view("v2").values(["Everything"]),
        "moveView" => TableGesture::new("moveView").view("v2").index(0),
        "deleteView" => TableGesture::new("deleteView").view("v1"),
        "hideColumn" => TableGesture::new("hideColumn").column("p5").view("v1"),
        "resizeColumn" => TableGesture::new("resizeColumn")
            .column("p1")
            .view("v2")
            .setting("width", "260"),
        "moveColumn" => TableGesture::new("moveColumn").column("p6").view("v1").index(0),
        "addSort" => TableGesture::new("addSort")
            .column("p1")
            .view("v1")
            .setting("direction", "descending"),
        "removeSort" => TableGesture::new("removeSort").column("p3").view("v1"),
        "addFilter" => TableGesture::new("addFilter").column("p6").view("v1"),
        "setFilter" => TableGesture::new("setFilter")
            .view("v1")
            .target("0")
            .values(["o2"])
            .setting("comparison", "is-not"),
        "removeFilter" => TableGesture::new("removeFilter").view("v1").target("0"),
        "groupBy" => TableGesture::new("groupBy").column("p6").view("v1"),
        "toggleGroup" => TableGesture::new("toggleGroup")
            .view("v1")
            .target("o2")
            .setting("collapsed", "true"),
        other => panic!("No gesture is written for '{other}'."),
    }
}

#[test]
fn the_same_edit_made_in_each_format_leaves_the_same_table() {
    let original = describe(KnowledgeBody::read(example_bytes("cities.yaml"), "cities.yaml").table());
    for edit in EDITS {
        let mut tables = Vec::new();
        for extension in EXTENSIONS {
            let folder = common::folder();
            let mut table = EditingTable::new(copy_example(&format!("cities{extension}"), folder.path()));
            table.edit(gesture(edit));
            tables.push(describe(&table.on_disk()));
        }
        // One table, three times - and not the table it was, or the edit proved nothing.
        assert_eq!(tables[0], tables[1], "{edit}: yaml and json differ");
        assert_eq!(tables[0], tables[2], "{edit}: yaml and xml differ");
        assert_ne!(original, tables[0], "{edit} changed nothing");
    }
}
