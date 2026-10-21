//! A knowledge file read through the vendored bindings (the standalone host's `KnowledgeBody.Tests.cs` at 8b6ccfc;
//! knowledge-designer Requirements 2.3, 2.4, 2.9 and 2.10): each example opens as the expected table in all three
//! formats, a file that is not well-formed says so with its position, one of another version opens read-only, and
//! what the designer does not know is reported and kept.

mod common;

use adp_designer_knowledge::body::ANOTHER_VERSION;
use adp_designer_knowledge::{KnowledgeBody, KnowledgeTable, definition};
use common::{EXTENSIONS, example_bytes, read_example};

#[test]
fn the_example_opens_as_the_expected_table() {
    for extension in EXTENSIONS {
        let body = read_example(&format!("cities{extension}"));
        let table = body.table();

        // Readable, editable, and nothing to report.
        assert_eq!("", body.unreadable(), "{extension}");
        assert_eq!("", body.read_only_reason());
        assert!(
            body.model().findings.is_empty(),
            "{extension}: {:?}",
            body.model().findings
        );

        // The table and its properties, with what each type has.
        assert_eq!("Cities", table.name);
        assert_eq!("v2", table.active_view_id);
        let first: Vec<String> = table
            .columns
            .iter()
            .take(6)
            .map(|p| format!("{} {} {}", p.id, p.name, p.value_type))
            .collect();
        assert_eq!(
            vec![
                "p1 Name text",
                "p2 Country selection",
                "p3 Population number",
                "p4 Tags multipleSelection",
                "p5 Province relation",
                "p6 Visited checkbox"
            ],
            first
        );
        let titles: Vec<&str> = table
            .columns
            .iter()
            .filter(|p| p.is_title)
            .map(|p| p.id.as_str())
            .collect();
        assert_eq!(vec!["p1"], titles);
        let options: Vec<String> = table.columns[1]
            .options
            .iter()
            .map(|o| format!("{} {} {}", o.id, o.name, o.colour))
            .collect();
        assert_eq!(
            vec![
                "o1 Netherlands blue",
                "o2 Belgium yellow",
                "b4ff5lo4rezgty4ge7siqndey Germany default",
                "dn1qx45l0fu88pip2l9ua7joz France default",
                "7h1eyx85f462uj3sb6jlxdxvu Finland default",
                "2xc52nefzictwc49wy8ldxh7t Italy default"
            ],
            options
        );
        // A colour the file does not give is the default, in every format.
        let colours: Vec<&str> = table.columns[3].options.iter().map(|o| o.colour.as_str()).collect();
        assert_eq!(vec!["default", "red"], colours);
        assert_eq!(
            ("provinces.yaml", "one"),
            (table.columns[4].target_file.as_str(), table.columns[4].limit.as_str())
        );

        // At least one view, the one the file was left in among them.
        assert!(!table.views.is_empty());
        assert!(table.views.iter().any(|v| v.id == table.active_view_id));
        assert_eq!(Some("v2"), table.view_or_default("").map(|v| v.id.as_str()));

        // Rows with their cells, by property.
        assert!(!table.rows.is_empty());
        assert!(table.rows.iter().all(|r| !r.id.is_empty()));
        assert!(
            table
                .rows
                .iter()
                .flat_map(|r| r.cells.iter())
                .all(|c| table.columns.iter().any(|p| p.id == c.column_id))
        );
    }
}

/// A table as what a format must not change.
fn shape(table: &KnowledgeTable) -> Vec<String> {
    let mut lines = vec![format!("table {} active {}", table.name, table.active_view_id)];
    for p in &table.columns {
        let options: Vec<String> = p
            .options
            .iter()
            .map(|o| format!("{}:{}:{}", o.id, o.name, o.colour))
            .collect();
        lines.push(format!(
            "property {} {} {} title={} target={} limit={} counterpart={} computed={} parent={} {}",
            p.id,
            p.name,
            p.value_type,
            p.is_title,
            p.target_file,
            p.limit,
            p.counterpart,
            p.is_computed,
            p.is_parent,
            options.join(",")
        ));
    }
    for v in &table.views {
        let columns: Vec<String> = v
            .columns
            .iter()
            .map(|c| format!("{}:{}:{}:{}", c.column_id, c.visible, c.width, c.wrap))
            .collect();
        let sorts: Vec<String> = v
            .sorts
            .iter()
            .map(|s| format!("{}:{}", s.column_id, s.descending))
            .collect();
        lines.push(format!(
            "view {} {} group={} hideEmpty={} any={} filters={} columns={} sorts={} collapsed={} order={} hidden={}",
            v.id,
            v.name,
            v.group_by,
            v.hide_empty_groups,
            v.filter.any,
            v.filter.items.len(),
            columns.join(","),
            sorts.join(","),
            v.collapsed.join(","),
            v.group_order.join(","),
            v.hidden_groups.join(",")
        ));
    }
    for r in &table.rows {
        let cells: Vec<String> = r
            .cells
            .iter()
            .map(|c| format!("{}=[{}]", c.column_id, c.values.join("|")))
            .collect();
        lines.push(format!("row {} {}", r.id, cells.join(" ")));
    }
    lines
}

#[test]
fn the_three_formats_are_one_table() {
    let tables: Vec<Vec<String>> = EXTENSIONS
        .iter()
        .map(|e| shape(read_example(&format!("cities{e}")).table()))
        .collect();
    assert!(!tables[0].is_empty());
    assert_eq!(tables[0], tables[1]);
    assert_eq!(tables[0], tables[2]);
}

#[test]
fn a_file_that_is_not_well_formed_says_so_with_its_position_and_holds_no_table() {
    let position = regex::Regex::new(r"\(line \d+, column \d+\)$").expect("the pattern compiles");
    for (extension, text) in [
        (".yaml", "ded: \"0.1\"\ndesigner: etalii/knowledge\nname: [unclosed\n"),
        (
            ".json",
            "{ \"ded\": \"0.1\", \"designer\": \"etalii/knowledge\", \"name\": ",
        ),
        (
            ".xml",
            "<ded version=\"0.1\" designer=\"etalii/knowledge\" name=\"Cities\"><properties>",
        ),
    ] {
        let body = KnowledgeBody::read(text.as_bytes().to_vec(), &format!("broken{extension}"));

        // The format's own error, with where; nothing guessed out of the wreck.
        assert!(
            body.unreadable().starts_with("This file cannot be read: "),
            "{extension}: {}",
            body.unreadable()
        );
        assert!(
            position.is_match(body.unreadable()),
            "{extension}: {}",
            body.unreadable()
        );
        assert_ne!(ANOTHER_VERSION, body.unreadable());
        assert_eq!(body.unreadable(), body.read_only_reason());
        assert_eq!(&KnowledgeTable::default(), body.table());
    }
}

#[test]
fn a_file_of_another_version_is_not_read_and_says_why() {
    let text = String::from_utf8(example_bytes("cities.yaml"))
        .expect("UTF-8")
        .replace("ded: \"0.1\"", "ded: \"9.0\"");
    let body = KnowledgeBody::read(text.into_bytes(), "cities.yaml");

    // The bindings require the version's mark, so the file is not read - a later version may hold what this one
    // would misread - and the reason names the version, not a parse error.
    assert_eq!(ANOTHER_VERSION, body.unreadable());
    assert_eq!(ANOTHER_VERSION, body.read_only_reason());
    assert_eq!(&KnowledgeTable::default(), body.table());
}

#[test]
fn an_extension_no_binding_reads_is_refused_by_name() {
    let body = KnowledgeBody::read(b"name: Cities\n".to_vec(), "cities.toml");
    assert_eq!(
        "A knowledge file is YAML, JSON or XML; '.toml' is none of them.",
        body.unreadable()
    );
}

#[test]
fn a_cell_whose_value_does_not_fit_its_type_is_a_finding_and_the_rest_is_read() {
    // A population that is not a number.
    let text = "ded: \"0.1\"\ndesigner: etalii/knowledge\nname: Cities\nproperties:\n  - id: p1\n    name: Name\n    type: text\n    title: true\n  - id: p3\n    name: Population\n    type: number\nviews:\n  - id: v1\n    name: All\nrows:\n  - id: r1\n    cells:\n      - property: p1\n        text: Antwerp\n      - property: p3\n        number: about half a million\n";
    let body = KnowledgeBody::read(text.as_bytes().to_vec(), "cities.yaml");

    // Reported where it is; the row and its good cell are there, the bad cell is not.
    let [finding] = body.model().findings.as_slice() else {
        panic!("{:?}", body.model().findings)
    };
    assert_eq!(Some(20), finding.location.as_ref().map(|l| l.line));
    let [row] = body.table().rows.as_slice() else {
        panic!("one row")
    };
    let cells: Vec<&str> = row.cells.iter().map(|c| c.column_id.as_str()).collect();
    assert_eq!(vec!["p1"], cells);
    assert_eq!("", body.read_only_reason());
}

#[test]
fn the_vendored_definition_is_read_for_its_types() {
    // Without them a mistyped value is no finding (the standalone host's `TheBundledDefinition_IsWhatItsProvenanceSays`:
    // the definition here is vendored at the commit `vendor/etalii.adp.lock` records, not bundled with a provenance).
    assert!(
        definition::typed_attribute_count() > 20,
        "Only {} attribute types were read from knowledge.des.",
        definition::typed_attribute_count()
    );
    assert_eq!(Some("number".to_owned()), definition::attribute_type("Cell", "number"));
}
