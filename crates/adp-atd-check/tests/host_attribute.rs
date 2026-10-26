//! `hostAttribute(element, name)` (etalii.adp spec 035, batch 2 of the descriptions; ATD §12.8): a call is one of
//! the library's, and a name written as a string is a host attribute of an entry of `persistence`'s `typeMap`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use adp_atd_check::{FsFiles, Overlay, check};

/// The findings of one description, as `code line:column message`.
fn findings(text: &str) -> Vec<String> {
    let path = PathBuf::from("/virtual/host-attribute.atd");
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

/// A description whose constraint reads the host attribute `name`.
fn reading(name: &str) -> String {
    format!(
        r#"atd 0.1
diagram example/tasks version "0.1.0" {{
  model {{
    element Task {{
      attribute title: string
    }}
  }}
  fn stored(task: Task) -> string {{
    let written = hostAttribute(task, "{name}")
    return written == null ? "" : string(written)
  }}
  constraints {{
    constraint stored for Task {{
      rule stored(self) != ""
      message "A task is read with an id."
    }}
  }}
  persistence {{
    typeMap: {{ Task: {{ as: Task, hostAttributes: [storedId] }} }}
  }}
  binding tasks family yaml {{
    claims {{ extensions: [".tasks"] }}
    element Task as task at "/tasks/*" {{
      id from key "id"
      attribute storedId from key "id"
      attribute title from key "title"
    }}
  }}
}}
"#
    )
}

#[test]
fn a_host_attribute_the_type_map_declares_is_read_by_name() {
    assert_eq!(findings(&reading("storedId")), Vec::<String>::new());
}

#[test]
fn a_host_attribute_no_entry_declares_is_refused_at_its_name() {
    assert_eq!(
        findings(&reading("storedID")),
        ["ATD2001 9:39 No host attribute `storedID` in `persistence`'s `typeMap`. Did you mean `storedId`?"]
    );
}
