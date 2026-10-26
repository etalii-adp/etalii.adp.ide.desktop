//! A form for a base type that shows a subtype's attribute behind `self.isA('T') && …` (etalii.adp spec 035, batch
//! 5b of the descriptions; ATD §14.2): the guard makes `self` a `T` in what follows it, and nowhere else.

use std::collections::BTreeMap;
use std::path::PathBuf;

use adp_atd_check::{FsFiles, Overlay, check};

/// The findings of one description, as `code line:column message`.
fn findings(text: &str) -> Vec<String> {
    let path = PathBuf::from("/virtual/type-guard.atd");
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

/// A description whose one form field, in a section as the Azure DevOps pipeline's is, is visible when `visible` holds.
fn description(visible: &str) -> String {
    format!(
        r#"atd 0.1
diagram example/jobs version "0.1.0" {{
  model {{
    element Job {{
      abstract: true
      attribute name: string
    }}
    element DeploymentJob extends Job {{
      attribute environment: string
    }}
  }}

  form job for Job {{
    usage: [inspector]
    section {{
      label: "Deployment"
      field environment "Environment" {{
        widget: text
        visible: ({visible})
      }}
    }}
  }}
}}
"#
    )
}

#[test]
fn an_is_a_guard_lets_what_follows_it_read_the_subtypes_attributes() {
    let text = description("self.isA('DeploymentJob') && has(self.environment) && self.environment != ''");
    assert_eq!(findings(&text), Vec::<String>::new());
}

#[test]
fn a_subtypes_attribute_read_without_the_guard_is_still_found() {
    let found = findings(&description("has(self.environment) && self.isA('DeploymentJob')"));
    assert!(
        found
            .iter()
            .any(|f| f.starts_with("ATD2001") && f.contains("`environment`")),
        "{found:?}"
    );
}

#[test]
fn a_guard_in_a_disjunction_proves_nothing() {
    let found = findings(&description("self.isA('DeploymentJob') || self.environment != ''"));
    assert!(
        found
            .iter()
            .any(|f| f.starts_with("ATD2001") && f.contains("`environment`")),
        "{found:?}"
    );
}
