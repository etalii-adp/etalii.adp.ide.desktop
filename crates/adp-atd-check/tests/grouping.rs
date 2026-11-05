//! A grouping in a description (DISL 0.9, §12.4; ATD §6.3; spec 075, T018, T021 and T022): `atd check` gives the
//! vendored descriptions that group a list, and that still filter one per item, exactly the findings of their
//! findings files, which name no cost, since no host estimates one; and a group of a typed list holds that type.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use adp_atd_check::{FsFiles, Overlay, check};
use serde_json::Value;

fn spec_dir() -> PathBuf {
    match std::env::var_os("ATD_SPEC_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/specifications/atd"),
    }
}

/// The findings of `path`, and those its findings file states, each as `code line:column`.
fn both(path: &Path) -> (Vec<String>, Vec<String>) {
    let expected: Value =
        serde_json::from_str(&std::fs::read_to_string(path.with_extension("findings.json")).unwrap()).unwrap();
    let expected = expected["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| format!("{} {}:{}", f["code"].as_str().unwrap(), f["line"], f["column"]))
        .collect();
    let actual = check(&FsFiles, path)
        .findings
        .iter()
        .map(|f| format!("{} {}:{}", f.code, f.line, f.column))
        .collect();
    (actual, expected)
}

#[test]
fn a_description_that_groups_a_list_of_the_limits_size_gives_its_findings() {
    let (actual, expected) = both(&spec_dir().join("fixtures/grouping-at-limit/grouping-at-limit.atd"));
    assert_eq!(actual, expected);
    assert!(actual.is_empty(), "no finding names the grouping's cost");
}

#[test]
fn a_description_that_filters_per_item_gives_what_it_gave_before() {
    let (actual, expected) = both(&spec_dir().join("fixtures/filtering-per-item/filtering-per-item.atd"));
    assert_eq!(actual, expected);
}

/// The findings of a description of boxes that hold items, with a rule reading `label`.
fn findings(label: &str) -> Vec<String> {
    let path = PathBuf::from("/virtual/grouping.atd");
    let text = format!(
        r#"atd 0.1
diagram example/boxes version "0.1.0" {{
  model {{
    element Item {{
      attribute name: string
      attribute shelf: string
    }}
    element Box {{
      attribute name: string
      contains Item
    }}
  }}

  constraint first for Box {{
    rule {label} != ''
    severity info
    message "The first item has no name."
  }}
}}
"#
    );
    let files = Overlay {
        open: BTreeMap::from([(path.clone(), text)]),
        under: &FsFiles,
    };
    check(&files, &path)
        .findings
        .iter()
        .map(|f| format!("{} {}", f.code, f.message))
        .collect()
}

#[test]
fn a_group_of_a_typed_list_holds_items_of_that_type() {
    assert_eq!(
        findings("self.children.groupBy(c, c.shelf).get('top')[0].name"),
        Vec::<String>::new()
    );
    let refused = findings("self.children.groupBy(c, c.shelf).get('top')[0].colour");
    assert_eq!(refused.len(), 1, "{refused:?}");
    assert!(refused[0].starts_with("ATD2001"), "{refused:?}");
    assert_eq!(
        findings("self.children.groupBy(c, c.shelf).keys().size() > 0 ? 'many' : 'none'"),
        Vec::<String>::new()
    );
}
