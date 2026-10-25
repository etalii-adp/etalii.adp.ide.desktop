//! The templates of a new description (etalii.adp spec 035, T133, FR-060): for each kind, a description that checks
//! without findings, in the canonical layout, with the origin and the name it was given.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use adp_atd_check::{FsFiles, Overlay, check};
use adp_atd_format::format;
use adp_atd_format::templates::{Kind, file_name, template};
use serde_json::Value;

/// A name with what a string must escape.
const NAME: &str = "Kim's \"widget\" board \\ v2";

fn checked(text: &str) -> adp_atd_check::Checked {
    let path = PathBuf::from("/nowhere/widgets.atd");
    let files = Overlay {
        open: BTreeMap::from([(path.clone(), text.to_owned())]),
        under: &FsFiles,
    };
    check(&files, Path::new(&path))
}

fn label(tool: &Value) -> Option<String> {
    tool["members"]
        .as_array()?
        .iter()
        .find(|m| m["kind"] == "label")
        .map(|m| m.to_string())
}

#[test]
fn each_kinds_template_checks_without_findings_and_has_its_origin_and_name() {
    for kind in Kind::ALL {
        let text = template(kind, "acme/widgets", NAME).unwrap();

        let checked = checked(&text);

        assert!(checked.findings.is_empty(), "{kind}: {:?}\n{text}", checked.findings);
        assert_eq!(checked.description.kind(), Some(kind.as_str()));
        assert_eq!(checked.description.origin(), Some("acme/widgets"));
        let tool = checked.description.tool().unwrap();
        let label = label(tool).unwrap_or_default();
        assert!(label.contains(r#"Kim's \"widget\" board \\ v2"#), "{kind}: {label}");
        assert!(text.starts_with(&format!("atd 0.1\n\n/// {NAME}:")), "{kind}");
    }
}

#[test]
fn each_kinds_template_is_in_the_canonical_layout() {
    for kind in Kind::ALL {
        let text = template(kind, "acme/widgets", "Widgets").unwrap();

        assert_eq!(format(&text).unwrap(), text, "{kind}");
    }
}

#[test]
fn an_origin_that_is_not_vendor_and_type_is_refused() {
    for origin in [
        "widgets",
        "acme/",
        "/widgets",
        "acme/wid gets",
        "acme/widgets/x",
        "-acme/widgets",
    ] {
        let refused = template(Kind::Diagram, origin, "Widgets").unwrap_err();

        assert_eq!(
            refused,
            format!("'{origin}' is not an origin of the form <vendor>/<type>.")
        );
        assert_eq!(file_name(origin), None);
    }
}

#[test]
fn a_name_must_be_one_line_of_text() {
    assert_eq!(
        template(Kind::Editor, "acme/notes", " ").unwrap_err(),
        "A description needs a name."
    );
    assert_eq!(
        template(Kind::Editor, "acme/notes", "Two\nlines").unwrap_err(),
        "A description's name is one line of text."
    );
}

#[test]
fn a_kind_is_read_by_its_keyword() {
    assert_eq!("designer".parse::<Kind>(), Ok(Kind::Designer));
    assert_eq!(
        "table".parse::<Kind>(),
        Err("'table' is not a kind of description: diagram, designer or editor.".to_owned())
    );
    assert_eq!(file_name("acme/widget-board"), Some("widget-board.atd".to_owned()));
}
