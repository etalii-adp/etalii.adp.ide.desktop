//! A field of `self` its type does not have, and a style that names nothing (etalii.adp spec 035, batch 1 of the
//! descriptions; ATD §14.2): found wherever a construct is written for one type, at the field or the name, with the
//! closest names and their fixes.

use std::collections::BTreeMap;
use std::path::PathBuf;

use adp_atd_check::{FsFiles, Overlay, check};

/// The findings of one description, as `code line:column message`.
fn findings(text: &str) -> Vec<String> {
    let path = PathBuf::from("/virtual/members.atd");
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

const MODEL: &str = "
  model {
    element Task {
      attribute title: string required
      attribute done: bool default false
      contains Task
    }
  }";

#[test]
fn a_field_of_self_the_type_lacks_is_found_in_every_construct_written_for_the_type() {
    let text = format!(
        "atd 0.1\ndiagram example/tasks version \"0.1.0\" {{{MODEL}
  constraint titled for Task {{
    rule self.titel.trim() != \"\"
    message \"A task needs a title.\"
  }}
  operation finish \"Finish\" for Task {{
    enabled !self.don
    set self.done = true
  }}
  notation {{
    node Task {{ label (self.parent.tittle) as up }}
  }}
}}
"
    );
    assert_eq!(
        findings(&text),
        vec![
            "ATD2001 11:15 No attribute `titel` in element type `Task`. Did you mean `title`?",
            "ATD2001 15:19 No attribute `don` in element type `Task`. Did you mean `done`?",
            "ATD2001 19:36 No attribute `tittle` in element type `Task`. Did you mean `title`?",
        ]
    );
}

#[test]
fn built_in_fields_attributes_and_untyped_operands_are_never_findings() {
    let text = format!(
        "atd 0.1\ndiagram example/tasks version \"0.1.0\" {{{MODEL}
  constraint known for Task {{
    rule self.id != \"\" && self.title != \"\" && self.children.size() >= 0 && self.view.collapsed == self.view.collapsed && !self.derived
    message \"Known.\"
  }}
  operation any \"Any\" for Task {{
    param note: string
    set self.title = p.note + self.children.map(c, c.whatever).join(\"\")
  }}
}}
"
    );
    assert_eq!(findings(&text), Vec::<String>::new());
}

#[test]
fn a_style_a_notation_does_not_declare_is_found_at_its_name() {
    let text = format!(
        "atd 0.1\ndiagram example/tasks version \"0.1.0\" {{{MODEL}
  notation {{
    style card {{ fill: \"#FFFFFF\" }}
    node Task {{
      style: crad
      label @title
    }}
  }}
}}
"
    );
    assert_eq!(
        findings(&text),
        vec!["ATD2011 13:14 No style `crad`. Did you mean `card`?"]
    );
}
