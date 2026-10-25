//! A test `x.isA('T')` narrows `x` (etalii.adp spec 035, batch 2 of the descriptions; ATD §14.5): where the test
//! holds, on the right of `&&` and in the first branch of `?:`, an attribute of `T` is no finding, and elsewhere it
//! still is.

use std::collections::BTreeMap;
use std::path::PathBuf;

use adp_atd_check::{FsFiles, Overlay, check};

/// The findings of one description, as `code line:column message`.
fn findings(text: &str) -> Vec<String> {
    let path = PathBuf::from("/virtual/narrowing.atd");
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

/// A description whose constraint on the abstract `Element` reads `rule`.
fn reading(rule: &str) -> String {
    format!(
        r#"atd 0.1
diagram example/plans version "0.1.0" {{
  model {{
    element Element {{
      abstract: true
      attribute begin: string
    }}
    element Period extends Element {{
      attribute end: string
    }}
    element Moment extends Element {{
    }}
  }}
  constraints {{
    constraint ends for Element {{
      rule {rule}
      message "An element ends."
    }}
  }}
}}
"#
    )
}

#[test]
fn an_attribute_of_the_type_a_test_holds_is_read_where_the_test_holds() {
    assert_eq!(findings(&reading("!self.isA('Period') || self.end != ''")).len(), 1);
    assert_eq!(
        findings(&reading("self.isA('Period') && self.end != ''")),
        Vec::<String>::new()
    );
    assert_eq!(
        findings(&reading(
            "self.isA('Period') && self.begin != '' ? self.end != '' : true"
        )),
        Vec::<String>::new()
    );
}

#[test]
fn an_attribute_of_the_type_a_test_holds_is_still_found_where_the_test_does_not_hold() {
    let found = findings(&reading("self.isA('Period') ? true : self.end != ''"));
    assert_eq!(found.len(), 1);
    assert!(found[0].starts_with("ATD2001"), "{found:?}");
}
