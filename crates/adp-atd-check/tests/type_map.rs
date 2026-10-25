//! A binding read through `persistence`'s `typeMap` (etalii.adp spec 035, batch 5b of the descriptions; ATD §14.7,
//! DISL §11.2): an entry keyed by a model type still maps it, its `hostAttributes` are the binding's to read, a rename
//! to an end or to the id names no attribute, and a header's rule may read what the header states. A context tool set
//! opens on DISL's reserved `diagram` and `connection` (ATD §8.6, DISL §7.3).

use std::collections::BTreeMap;
use std::path::PathBuf;

use adp_atd_check::{FsFiles, Overlay, check};

/// The findings of one description, as `code line:column message`.
fn findings(text: &str) -> Vec<String> {
    let path = PathBuf::from("/virtual/type-map.atd");
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

const MODEL: &str = r#"atd 0.1
diagram example/trends version "0.1.0" {
  model {
    element Trend {
      attribute name: string
    }
    relation Influence from Trend to Trend {
      attribute note: string
    }
  }
  persistence {
    typeMap: {
      Graph: { as: header }
      Trend: { as: Trend, hostAttributes: [storedId, unknownKeys] }
      Influence: { as: Influence, attributes: { from: source, to: target, key: id, legacy: null }, hostAttributes: [storedId] }
      Unreadable: { as: unreadable }
    }
  }
"#;

#[test]
fn a_type_map_entry_keyed_by_a_model_type_gives_its_rule_the_host_attributes_and_the_renames() {
    let text = format!(
        r#"{MODEL}  binding trends family yaml {{
    claims {{ extensions: [".trends"] }}
    element Graph as graph at "/" {{
      attribute version from key "trends"
    }}
    element Trend as trend at "/trends/*" {{
      id from key "id"
      attribute storedId from key "id"
      attribute unknownKeys from value (entry)
      attribute name from key "name"
    }}
    element Influence as influence at "/influences/*" {{
      attribute storedId from key "id"
      attribute key from key "id"
      attribute from from key "from"
      attribute to from key "to"
      attribute legacy from key "legacy"
      attribute note from key "note"
    }}
  }}
}}
"#
    );
    assert_eq!(findings(&text), Vec::<String>::new());
}

#[test]
fn an_attribute_neither_the_type_nor_its_host_attributes_has_is_still_found() {
    let text = format!(
        r#"{MODEL}  binding trends family yaml {{
    claims {{ extensions: [".trends"] }}
    element Trend as trend at "/trends/*" {{
      attribute storedId from key "id"
      attribute colour from key "colour"
    }}
    element Unreadable as trend-not-a-mapping at "/trends/*" {{
      attribute name from key "name"
    }}
  }}
}}
"#
    );
    assert_eq!(
        findings(&text),
        vec![
            "ATD6001 23:17 The binding reads `colour`, which element type `Trend` does not declare.",
            "ATD6001 26:17 `Unreadable` is mapped as unreadable, so its rule `trend-not-a-mapping` binds no attribute; it binds `name`.",
        ]
    );
}

#[test]
fn a_context_tool_set_opens_on_the_empty_canvas_and_on_a_pending_connection() {
    let text = format!(
        r#"{MODEL}  toolbox {{
    context for diagram {{
      tool "Add trend here" {{ kind: delete }}
    }}
    context for connection {{
      tool "influences" {{ kind: connect, via: Influence }}
    }}
    context for canvas {{
      tool "Nothing" {{ kind: delete }}
    }}
  }}
}}
"#
    );
    assert_eq!(findings(&text), vec!["ATD2002 26:17 No element type `canvas`."]);
}
