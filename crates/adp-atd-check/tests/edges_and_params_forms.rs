//! An edge drawn for a reference attribute, and a form that edits an operation's parameters (etalii.adp spec 035,
//! batch 5b of the descriptions): `edge <Type>.<attribute>` names an attribute of the type that references an element
//! type (DISL §6.10), and a field of a form an operation names as its `paramsForm` edits a parameter, not an attribute
//! (DISL §9.2).

use std::collections::BTreeMap;
use std::path::PathBuf;

use adp_atd_check::{FsFiles, Overlay, check};

/// The findings of one description, as `code line:column message`.
fn findings(text: &str) -> Vec<String> {
    let path = PathBuf::from("/virtual/edges.atd");
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
    element Component {
      attribute name: string required
    }
    element Evolution {
      attribute component: Component
      attribute note: string
    }
  }";

#[test]
fn an_edge_for_a_reference_attribute_is_drawn_and_reads_the_type_that_holds_it() {
    let text = format!(
        "atd 0.1\ndiagram example/maps version \"0.1.0\" {{{MODEL}
  notation {{
    edge Evolution.component {{ label (self.note) as note }}
  }}
}}
"
    );
    assert_eq!(findings(&text), Vec::<String>::new());
}

#[test]
fn an_edge_for_an_attribute_that_is_missing_or_not_a_reference_is_found_at_its_name() {
    let text = format!(
        "atd 0.1\ndiagram example/maps version \"0.1.0\" {{{MODEL}
  notation {{
    edge Evolution.target {{ }}
    edge Evolution.note {{ }}
  }}
}}
"
    );
    assert_eq!(
        findings(&text),
        vec![
            "ATD2002 13:10 `Evolution` has no reference attribute `target` for an edge to draw.",
            "ATD2002 14:10 `Evolution` has no reference attribute `note` for an edge to draw.",
        ]
    );
}

#[test]
fn a_field_of_a_params_form_edits_a_parameter_and_any_other_field_an_attribute() {
    let text = format!(
        "atd 0.1\ndiagram example/maps version \"0.1.0\" {{{MODEL}
  operation rename \"Rename\" for Component {{
    param label: string
    paramsForm: renameDialog
    set self.name = p.label
  }}
  form renameDialog \"Rename\" for Component {{
    field label \"Name\" {{ }}
  }}
  form inspector for Component {{
    field label \"Name\" {{ }}
  }}
}}
"
    );
    assert_eq!(
        findings(&text),
        vec!["ATD2001 21:11 No attribute `label` in element type `Component`."]
    );
}
