//! What a knowledge file is told about itself (the standalone host's `KnowledgeFindings.Tests.cs` at 8b6ccfc;
//! knowledge-designer Requirements 8.2 and 8.5): one finding code per condition, each from a file that has the
//! condition, at the place it is about - and nothing that is reported is removed or changed by reporting it.

mod common;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use adp_designer_knowledge::body::ANOTHER_VERSION;
use adp_designer_knowledge::validator::*;
use adp_designer_knowledge::{KnowledgeSession, store};
use adp_desl::table::model::{TableFinding, TableFindingSeverity, TableGesture};
use adp_desl::table::session::DesignerSession;
use common::{EditingTable, copy_example};

/// A small table with nothing wrong in it: a title, a number, a selection, one view, two rows.
const SOUND: &str = "ded: \"0.1\"
designer: etalii/knowledge
name: Cities
properties:
  - id: p1
    name: Name
    type: text
    title: true
  - id: p2
    name: People
    type: number
  - id: p3
    name: Size
    type: selection
    options:
      - id: o1
        name: Large
views:
  - id: v1
    name: All
rows:
  - id: r1
    cells:
      - property: p1
        text: Amsterdam
      - property: p2
        number: 931298
      - property: p3
        option: o1
  - id: r2
    cells:
      - property: p1
        text: Antwerp

";

fn write(folder: &Path, content: &str) -> PathBuf {
    let path = folder.join("table.yaml");
    std::fs::write(&path, content.replace('\n', "\r\n")).expect("written");
    path
}

fn changed(old: &str, new: &str) -> String {
    assert!(SOUND.contains(old), "{old}");
    SOUND.replace(old, new)
}

fn findings_of(path: &Path) -> Vec<TableFinding> {
    KnowledgeSession::new(path).baseline().findings
}

#[test]
fn a_sound_file_is_told_nothing() {
    let folder = common::folder();
    assert!(findings_of(&write(folder.path(), SOUND)).is_empty());
}

/// Each condition as a change of the sound file, the code it is reported under, how serious it is, and where.
#[test]
fn a_condition_is_reported_under_its_code_at_its_place() {
    use TableFindingSeverity::{Error, Info, Warning};
    let conditions: [(&str, TableFindingSeverity, &str, &str, &str, &str); 11] = [
        (NO_TITLE, Warning, "", "", "    title: true\n", ""),
        (
            SEVERAL_TITLES,
            Warning,
            "",
            "p2",
            "    name: People\n    type: number\n",
            "    name: People\n    type: text\n    title: true\n",
        ),
        (
            TITLE_NOT_TEXT,
            Error,
            "",
            "p1",
            "    name: Name\n    type: text\n",
            "    name: Name\n    type: number\n",
        ),
        (
            DUPLICATE_PROPERTY_NAME,
            Error,
            "",
            "p2",
            "    name: People\n",
            "    name: name\n",
        ),
        (
            UNKNOWN_PROPERTY,
            Warning,
            "r2",
            "p9",
            "      - property: p1\n        text: Antwerp\n",
            "      - property: p1\n        text: Antwerp\n      - property: p9\n        text: stray\n",
        ),
        (
            VALUE_OF_ANOTHER_TYPE,
            Warning,
            "r2",
            "p2",
            "        text: Antwerp\n",
            "        text: Antwerp\n      - property: p2\n        text: a great many\n",
        ),
        (
            UNKNOWN_OPTION,
            Warning,
            "r1",
            "p3",
            "        option: o1\n",
            "        option: o9\n",
        ),
        (
            VIEW_NAMES_MISSING_PROPERTY,
            Warning,
            "",
            "",
            "    name: All\n",
            "    name: All\n    groupBy: p9\n",
        ),
        (
            VIEW_NAMES_MISSING_PROPERTY,
            Warning,
            "",
            "",
            "    name: All\n",
            "    name: All\n    sorts:\n      - property: p9\n",
        ),
        (
            UNKNOWN_TYPE,
            Warning,
            "",
            "p2",
            "    name: People\n    type: number\n",
            "    name: People\n    type: person\n",
        ),
        (
            MISSING_ID,
            Info,
            "row@30",
            "",
            "  - id: r2\n    cells:\n",
            "  - cells:\n",
        ),
    ];
    for (code, severity, row_id, column_id, old, new) in conditions {
        let folder = common::folder();
        let findings = findings_of(&write(folder.path(), &changed(old, new)));
        let found: Vec<&TableFinding> = findings.iter().filter(|f| f.code == code).collect();
        let [finding] = found.as_slice() else {
            panic!("{code}: {findings:?}")
        };
        assert_eq!(severity, finding.severity, "{code}");
        assert_eq!(
            (row_id, column_id),
            (finding.row_id.as_str(), finding.column_id.as_str()),
            "{code}"
        );
        assert!(!finding.message.is_empty());
    }
}

#[test]
fn a_key_the_designer_does_not_know_is_reported_wherever_it_stands_and_kept() {
    // A key of the file, of a property, of an option, of a view, of a row and of a cell that no rule reads.
    let folder = common::folder();
    let path = write(
        folder.path(),
        &SOUND
            .replace("name: Cities\n", "name: Cities\nowner: Peter\n")
            .replace("    name: People\n", "    name: People\n    unit: people\n")
            .replace("        name: Large\n", "        name: Large\n        icon: star\n")
            .replace("    name: All\n", "    name: All\n    density: compact\n")
            .replace("  - id: r2\n", "  - id: r2\n    weight: 3\n")
            .replace(
                "        text: Antwerp\n",
                "        text: Antwerp\n        note: a port\n",
            ),
    );
    let unknown: Vec<TableFinding> = findings_of(&path)
        .into_iter()
        .filter(|f| f.code == UNKNOWN_KEY)
        .collect();

    // Each is said once, as information, by its name and its line.
    let keys = ["owner", "unit", "icon", "density", "weight", "note"];
    assert_eq!(keys.len(), unknown.len(), "{unknown:?}");
    for key in keys {
        let named = format!("'{key}'");
        assert_eq!(
            1,
            unknown
                .iter()
                .filter(|f| f.message.contains(&named) && f.message.contains("(line "))
                .count(),
            "{key}"
        );
    }
    assert!(unknown.iter().all(|f| f.severity == TableFindingSeverity::Info));

    // An edit beside them: one line changed, and every key nobody reads is where it was.
    let before = std::fs::read_to_string(&path).expect("read");
    let mut table = EditingTable::new(path.clone());
    table.edit(
        TableGesture::new("setCell")
            .row("r2")
            .column("p1")
            .values(["Antwerpen"]),
    );
    assert_eq!(
        before.replace("text: Antwerp\r\n", "text: Antwerpen\r\n"),
        std::fs::read_to_string(&path).expect("read")
    );
}

#[test]
fn a_key_the_designer_does_not_know_is_reported_in_a_json_file_too() {
    let folder = common::folder();
    let path = copy_example("cities.json", folder.path());
    let text = std::fs::read_to_string(&path).expect("read");
    assert!(!findings_of(&path).iter().any(|f| f.code == UNKNOWN_KEY));
    std::fs::write(
        &path,
        text.replace("\"name\": \"Cities\",", "\"name\": \"Cities\", \"owner\": \"Peter\",")
            .replace("{ \"id\": \"p1\",", "{ \"id\": \"p1\", \"unit\": \"none\","),
    )
    .expect("written");
    let unknown: Vec<TableFinding> = findings_of(&path)
        .into_iter()
        .filter(|f| f.code == UNKNOWN_KEY)
        .collect();
    assert_eq!(2, unknown.len(), "{unknown:?}");
    assert_eq!(1, unknown.iter().filter(|f| f.message.contains("'owner'")).count());
    assert_eq!(1, unknown.iter().filter(|f| f.message.contains("'unit'")).count());
}

#[test]
fn an_attribute_or_element_the_designer_does_not_know_is_reported_in_an_xml_file() {
    let folder = common::folder();
    let path = copy_example("cities.xml", folder.path());
    let text = std::fs::read_to_string(&path).expect("read");
    assert!(!findings_of(&path).iter().any(|f| f.code == UNKNOWN_KEY));
    std::fs::write(
        &path,
        text.replace(" name=\"Cities\"", " name=\"Cities\" owner=\"Peter\"")
            .replace("<property id=\"p1\"", "<property id=\"p1\" unit=\"none\"")
            .replace("  <properties>", "  <notes>Kept by hand.</notes>\r\n  <properties>"),
    )
    .expect("written");
    let unknown: Vec<TableFinding> = findings_of(&path)
        .into_iter()
        .filter(|f| f.code == UNKNOWN_KEY)
        .collect();
    assert_eq!(3, unknown.len(), "{unknown:?}");
    assert_eq!(
        1,
        unknown
            .iter()
            .filter(|f| f.message.contains("attribute 'owner'"))
            .count()
    );
    assert_eq!(
        1,
        unknown
            .iter()
            .filter(|f| f.message.contains("attribute 'unit'"))
            .count()
    );
    assert_eq!(
        1,
        unknown.iter().filter(|f| f.message.contains("element 'notes'")).count()
    );
}

#[test]
fn a_file_of_another_version_is_told_so_in_one_finding_and_is_not_changed() {
    let folder = common::folder();
    let path = write(folder.path(), &changed("ded: \"0.1\"", "ded: \"9.0\""));
    let before = std::fs::read(&path).expect("read");
    let baseline = KnowledgeSession::new(&path).baseline();

    // The one finding the designer's definition names for it, and a table nobody can edit.
    let [finding] = baseline.findings.as_slice() else {
        panic!("{:?}", baseline.findings)
    };
    assert_eq!(
        (NEWER_VERSION, TableFindingSeverity::Warning, ANOTHER_VERSION),
        (finding.code.as_str(), finding.severity, finding.message.as_str())
    );
    assert_ne!("", baseline.read_only_reason);
    assert_eq!(before, std::fs::read(&path).expect("read"));
}

#[test]
fn entries_without_an_id_are_given_one_by_the_first_edit_which_may_be_an_edit_of_one_of_them() {
    // A property, an option, a view and a row written by hand, each without an id.
    let folder = common::folder();
    let path = write(
        folder.path(),
        &SOUND
            .replace("  - id: p2\n    name: People\n", "  - name: People\n")
            .replace("      - id: o1\n        name: Large\n", "      - name: Large\n")
            .replace("  - id: v1\n    name: All\n", "  - name: All\n")
            .replace("  - id: r2\n    cells:\n", "  - cells:\n")
            .replace("      - property: p2\n        number: 931298\n", "")
            .replace("        option: o1\n", "        text: kept\n"),
    );
    let mut table = EditingTable::new(path.clone());
    let missing: Vec<TableFinding> = table
        .session
        .baseline()
        .findings
        .into_iter()
        .filter(|f| f.code == MISSING_ID)
        .collect();
    assert_eq!(4, missing.len(), "{missing:?}");
    let unnamed: Vec<&TableFinding> = missing.iter().filter(|f| !f.row_id.is_empty()).collect();
    let [unnamed] = unnamed.as_slice() else {
        panic!("{missing:?}")
    };

    // The first edit, of the row that has no id.
    table.edit(
        TableGesture::new("setCell")
            .row(unnamed.row_id.clone())
            .column("p1")
            .values(["Antwerpen"]),
    );

    // The edit is made, every such entry has an id in the file, and nothing is said of them any more.
    let after = store::read(&path).body.expect("it reads");
    for element in after
        .model()
        .elements
        .iter()
        .filter(|e| matches!(e.type_name.as_str(), "Property" | "Option" | "View" | "Row"))
    {
        assert!(
            element.id_is_stored,
            "The {} on line {} still has no id.",
            element.type_name, element.line
        );
    }
    assert_eq!(
        vec!["Antwerpen"],
        after.table().rows[1].cell("p1").expect("a name").values
    );
    assert!(!after.table().rows[1].id.contains('@'));
    assert!(!table.session.baseline().findings.iter().any(|f| f.code == MISSING_ID));
    let names: Vec<&str> = after.table().columns.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(vec!["Name", "People", "Size"], names);
    assert_eq!("All", after.table().views[0].name);

    // One undo: the ids go with the edit that gave them.
    table.session.undo().expect("undone");
    let again = store::read(&path).body.expect("it reads");
    assert_eq!(
        4,
        again
            .model()
            .elements
            .iter()
            .filter(|e| !e.id_is_stored && matches!(e.type_name.as_str(), "Property" | "Option" | "View" | "Row"))
            .count()
    );
}

#[test]
fn entries_without_an_id_are_given_one_in_the_other_formats_too() {
    for (example, row, row_without, view, view_without) in [
        ("cities.json", "{ \"id\": \"r2\", ", "{ ", "{ \"id\": \"v2\", ", "{ "),
        ("cities.xml", "<row id=\"r2\">", "<row>", "<view id=\"v2\" ", "<view "),
    ] {
        let folder = common::folder();
        let path = copy_example(example, folder.path());
        let text = std::fs::read_to_string(&path).expect("read");
        assert!(text.contains(row) && text.contains(view), "{example}");
        std::fs::write(&path, text.replace(row, row_without).replace(view, view_without)).expect("written");
        let mut table = EditingTable::new(path.clone());
        assert_eq!(
            2,
            table
                .session
                .baseline()
                .findings
                .iter()
                .filter(|f| f.code == MISSING_ID)
                .count(),
            "{example}"
        );

        // An edit of another row altogether.
        table.edit(TableGesture::new("setCell").row("r1").column("p1").values(["Mokum"]));

        // Both have an id, the file still reads without a complaint, and the rest of it is as it was.
        let after = store::read(&path).body.expect("it reads");
        for element in after
            .model()
            .elements
            .iter()
            .filter(|e| matches!(e.type_name.as_str(), "View" | "Row"))
        {
            assert!(
                element.id_is_stored,
                "{example}: the {} on line {} still has no id.",
                element.type_name, element.line
            );
        }
        assert!(
            !table
                .session
                .baseline()
                .findings
                .iter()
                .any(|f| f.code == MISSING_ID || f.severity == TableFindingSeverity::Error),
            "{example}"
        );
        assert_eq!("Map", after.table().views[1].name);
        assert_eq!(
            vec!["Antwerp"],
            after.table().rows[1].cell("p1").expect("a name").values
        );
    }
}

#[test]
fn a_table_without_a_view_is_shown_as_if_it_had_one_and_says_so() {
    let folder = common::folder();
    let path = write(folder.path(), &changed("views:\n  - id: v1\n    name: All\n", ""));
    assert!(findings_of(&path).iter().any(|f| f.code == NO_VIEW));
}

#[test]
fn a_relation_to_one_row_holding_several_is_reported_and_keeps_them() {
    let folder = common::folder();
    let path = write(
        folder.path(),
        &changed(
            "        text: Antwerp\n",
            "        text: Antwerp\n      - property: p4\n        rows:\n          - row: r1\n          - row: r2\n",
        )
        .replace(
            "views:\n",
            "  - id: p4\n    name: Near\n    type: relation\n    target: .\n    limit: one\nviews:\n",
        ),
    );
    let findings = findings_of(&path);
    let over: Vec<&TableFinding> = findings.iter().filter(|f| f.code == RELATION_OVER_LIMIT).collect();
    let [finding] = over.as_slice() else {
        panic!("{findings:?}")
    };
    assert_eq!(("r2", "p4"), (finding.row_id.as_str(), finding.column_id.as_str()));
    let body = store::read(&path).body.expect("it reads");
    assert_eq!(
        vec!["r1", "r2"],
        body.table().row("r2").expect("r2").cell("p4").expect("p4").values
    );
}

#[test]
fn rows_that_are_each_others_parents_are_each_reported() {
    // r1 under r2 and r2 under r1.
    let folder = common::folder();
    let path = write(
        folder.path(),
        &SOUND
            .replace(
                "views:\n",
                "  - id: p4\n    name: Under\n    type: relation\n    target: .\n    limit: one\n    parent: true\nviews:\n",
            )
            .replace("        option: o1\n", "        option: o1\n      - property: p4\n        rows:\n          - row: r2\n")
            .replace("        text: Antwerp\n", "        text: Antwerp\n      - property: p4\n        rows:\n          - row: r1\n"),
    );
    let cycles: Vec<TableFinding> = findings_of(&path)
        .into_iter()
        .filter(|f| f.code == PARENT_CYCLE)
        .collect();
    let mut rows: Vec<&str> = cycles.iter().map(|f| f.row_id.as_str()).collect();
    rows.sort_unstable();
    assert_eq!(vec!["r1", "r2"], rows);
    assert!(cycles.iter().all(|f| f.severity == TableFindingSeverity::Error));
}

#[test]
fn a_file_with_several_things_wrong_is_told_all_of_them() {
    let folder = common::folder();
    let path = write(
        folder.path(),
        &SOUND
            .replace("    title: true\n", "")
            .replace("        option: o1\n", "        option: o9\n")
            .replace("    name: All\n", "    name: All\n    groupBy: p9\n"),
    );
    let codes: HashSet<String> = findings_of(&path).into_iter().map(|f| f.code).collect();
    for code in [NO_TITLE, UNKNOWN_OPTION, VIEW_NAMES_MISSING_PROPERTY] {
        assert!(codes.contains(code), "{code}: {codes:?}");
    }
}

#[test]
fn what_is_reported_is_still_in_the_file_after_an_edit_of_something_else() {
    // A word where a number belongs, an option that is not there, and a cell of a property that is not there.
    let folder = common::folder();
    let path = write(
        folder.path(),
        &SOUND
            .replace("        number: 931298\n", "        number: a great many\n")
            .replace("        option: o1\n", "        option: o9\n")
            .replace(
                "        text: Antwerp\n",
                "        text: Antwerp\n      - property: p9\n        text: stray\n",
            ),
    );
    let before = std::fs::read_to_string(&path).expect("read");
    let mut table = EditingTable::new(path.clone());
    table.edit(
        TableGesture::new("setCell")
            .row("r2")
            .column("p1")
            .values(["Antwerpen"]),
    );

    // One line changed, and every line that was reported is the line it was.
    let after = std::fs::read_to_string(&path).expect("read");
    assert_eq!(before.replace("text: Antwerp\r\n", "text: Antwerpen\r\n"), after);
    assert!(after.contains("number: a great many"));
    assert!(after.contains("option: o9"));
    assert!(after.contains("- property: p9"));
}
