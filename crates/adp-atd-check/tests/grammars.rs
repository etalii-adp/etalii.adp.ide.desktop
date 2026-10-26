//! The grammar of a `grammar` binding (FBL 0.9 section 4.9, ATD §11.9; spec 035, T148): one that is not LALR(1) is
//! `ATD8001` at the head of the definition of the rule in conflict defined first, naming the conflict; a file
//! `grammar from` names that is not there is `ATD2009`; a grammar that builds gives nothing.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use adp_atd_check::{FsFiles, Overlay, check};

const DESCRIPTION: &str = r#"atd 0.1

diagram acme/items version "0.1.0" {
  label "Items"

  model {
    element Item {
      attribute name: string
    }
  }

  binding items family grammar {
    body { kind: file, start: doc }
    GRAMMAR
    element Item as item at "/item" {
      attribute name from node "NAME"
    }
  }
}
"#;

const CONFLICT: &str = "grammar {\n      doc: item*\n      item: NAME \";\"\n          | other\n      other: NAME \";\"\n      NAME: /[a-z]+/\n      WS: / +/\n      %ignore WS\n    }";

fn findings(files: &[(&str, &str)]) -> Vec<(String, String, usize, usize, String)> {
    let folder = PathBuf::from("/nowhere/items");
    let open: BTreeMap<PathBuf, String> = files.iter().map(|(n, t)| (folder.join(n), (*t).to_owned())).collect();
    let overlay = Overlay { open, under: &FsFiles };
    check(&overlay, Path::new(&folder.join(files[0].0)))
        .findings
        .iter()
        .map(|f| (f.code.to_owned(), f.file.clone(), f.line, f.column, f.message.clone()))
        .collect()
}

#[test]
fn an_inline_grammar_with_a_conflict_is_atd8001_at_the_rule_in_conflict() {
    let found = findings(&[("items.atd", &DESCRIPTION.replace("GRAMMAR", CONFLICT))]);

    assert_eq!(found.len(), 1, "{found:?}");
    let (code, _, line, column, message) = &found[0];
    assert_eq!(code, "ATD8001");
    // `item`, defined before `other`.
    assert_eq!((*line, *column), (16, 7));
    assert!(message.contains("Reduce/Reduce"), "{message}");
    assert!(message.contains("`item : NAME \";\"`") && message.contains("`other : NAME \";\"`"), "{message}");
}

#[test]
fn a_grammar_file_with_a_conflict_is_atd8001_and_a_missing_one_atd2009() {
    let lark = "doc: item*\nitem: NAME \";\" | other\nother: NAME \";\"\nNAME: /[a-z]+/\nWS: / +/\n%ignore WS\n";
    let description = DESCRIPTION.replace("GRAMMAR", "grammar from \"items.lark\"");

    let conflict = findings(&[("items.atd", &description), ("items.lark", lark)]);
    let missing = findings(&[("items.atd", &description)]);

    assert_eq!(conflict.iter().map(|f| f.0.as_str()).collect::<Vec<_>>(), ["ATD8001"], "{conflict:?}");
    assert_eq!(missing.iter().map(|f| f.0.as_str()).collect::<Vec<_>>(), ["ATD2009"], "{missing:?}");
}

#[test]
fn a_grammar_that_builds_gives_nothing() {
    let lark = "doc: item*\nitem: NAME \";\"\nNAME: /[a-z]+/\nWS: / +/\n%ignore WS\n";
    let description = DESCRIPTION.replace("GRAMMAR", "grammar from \"items.lark\"");

    assert_eq!(findings(&[("items.atd", &description), ("items.lark", lark)]), []);
}
