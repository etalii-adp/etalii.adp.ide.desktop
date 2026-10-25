//! What the Hierarchical Requirement Designer's description needs of ATD 0.1 (etalii.adp spec 035, T166; ATD §9.3,
//! §11.3, §12.3, §14.7): the widths of a hierarchy's columns, `move` with a place, rule names with hyphens, binding
//! types the type map maps as the document and as unreadable, and `move … to x, y` refused in a designer.
//!
//! These constructs are in the grammar of etalii.adp's amendment of 2026-10-25. Built from a specification before it
//! (the vendored one until it is refreshed), the tests are reported as skipped, with that reason.

use std::collections::BTreeMap;
use std::path::PathBuf;

use adp_atd_check::{FsFiles, Overlay, check};

/// The findings of one description, as `code line:column message`.
fn findings(text: &str) -> Vec<String> {
    let path = PathBuf::from("/virtual/hierarchy.atd");
    let files = Overlay {
        open: BTreeMap::from([(path.clone(), text.to_owned())]),
        under: &FsFiles,
    };
    check(&files, &path)
        .findings
        .iter()
        .map(|f| format!("{} {}:{} {}", f.code, f.line, f.column, f.message))
        .collect()
}

/// Whether the grammar this crate was built from has the amendment's constructs; when not, says why the test is
/// skipped.
fn amended() -> bool {
    let probe =
        "atd 0.1\ndesigner example/probe version \"0.1.0\" {\n  surface hierarchy {\n    widths Column\n  }\n}\n";
    let syntax = findings(probe).into_iter().any(|f| f.starts_with("ATD1001"));
    if syntax {
        eprintln!("skipped: the grammar this crate was built from predates ATD 0.1's amendment of 2026-10-25");
    }
    !syntax
}

const OUTLINE: &str = r#"atd 0.1
designer example/outline version "0.1.0" {
  label "Outline"
  model {
    document {
      attribute lastItem: string
    }
    element Item {
      attribute title: string required
      attribute tags: string many
      attribute collapsed: bool default false
      contains Item, Requirement ordered
    }
    element Requirement {
      attribute title: string required
      attribute tags: string many
      attribute collapsed: bool default false
    }
    element Tag {
      attribute name: string required
    }
    element Column {
      attribute level: string required
      attribute width: int required
    }
  }
  persistence {
    typeMap: { Marks: { as: document }, Unreadable: { as: unreadable } }
    ids: { types: { Item: { strategy: counter, prefix: "ITEM-", digits: 4, mark: lastItem } } }
    settings: { collapsed: { attribute: collapsed, of: [Item, Requirement] } }
  }
  surface hierarchy {
    levels Item { title: title }
    entries Requirement { title: title }
    labels tags { on: [Item, Requirement], options: { type: Tag, name: name } }
    widths Column { level: level, width: width }
    collapsed: collapsed
  }
  operation moveItem "Move" for Item {
    param item: Item required
    param after: Item
    if has(p.after) {
      move self into p.item after p.after
    }
    if !has(p.after) {
      move self into p.item
    }
  }
  binding outline family bullets {
    claims files "*.outline.md"
    section items "^Items$"
    element Marks as marks line r"^<!-- last: (?<lastItem>ITEM-\d+) -->$" {
      attribute lastItem from group "lastItem"
    }
    element Item as item line r"^(?<id>ITEM-\d+)[ \t]+(?<title>.+)$" {
      within items
      id from group "id"
      parent item
      attribute title from group "title" { empty: refuse }
    }
    element Requirement as requirement line r"^(?<id>REQ-\d+)[ \t]+(?<title>.+)$" {
      within items
      id from group "id"
      parent item
      attribute title from group "title" { empty: refuse }
    }
    element Unreadable as unreadable-entry line r"^.*$" {
      within items
    }
    registration { blocks: { collapsed: { order: ordinal, keys: ids, history: false } } }
  }
}
"#;

#[test]
fn a_hierarchy_with_widths_a_placed_move_dashed_rule_names_and_a_type_map_checks_without_findings() {
    if !amended() {
        return;
    }
    assert_eq!(findings(OUTLINE), Vec::<String>::new());
}

#[test]
fn a_move_to_a_position_is_refused_in_a_designer() {
    if !amended() {
        return;
    }
    let text = OUTLINE.replace("move self into p.item\n", "move self to 10, 20\n");
    let found = findings(&text);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].starts_with("ATD3002 46:7 "), "{found:?}");
}

#[test]
fn a_document_attribute_the_mark_line_does_not_have_is_found_at_the_attribute() {
    if !amended() {
        return;
    }
    let text = OUTLINE.replace(
        "      attribute lastItem from group \"lastItem\"\n",
        "      attribute lastItem from group \"lastItem\"\n      attribute lastPart from group \"lastItem\"\n",
    );
    let found = findings(&text);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].starts_with("ATD6001 54:17 "), "{found:?}");
}
