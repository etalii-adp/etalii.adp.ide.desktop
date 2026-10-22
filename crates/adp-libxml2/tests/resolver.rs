//! Every file libxml2 reads comes through the caller, by a project path: references resolve against the folder of the
//! file that names them, and one that is not a file of the project is refused with its reason and never read
//! (after the standalone host's `ProjectXmlResolver.Tests.cs` at `8b6ccfc`).

use std::cell::RefCell;

use adp_libxml2::{Refusal, address_of, check_against_dtd, path_of};

fn path(text: &str) -> Vec<String> {
    text.split('/').map(str::to_owned).collect()
}

#[test]
fn an_address_names_its_project_path_and_back() {
    for name in ["app.xml", "settings/app.xml", "a folder/ünïcode #1.xsd"] {
        assert_eq!(path_of(&address_of(&path(name))), Ok(path(name)));
    }
}

#[test]
fn a_reference_resolves_against_the_folder_of_the_file_that_names_it() {
    let asked = RefCell::new(Vec::new());
    let files = |p: &[String]| {
        asked.borrow_mut().push(p.to_vec());
        (p == path("schemas/app.dtd")).then(|| "<!ELEMENT app EMPTY>".to_owned())
    };
    let outcome = check_against_dtd(
        "<!DOCTYPE app SYSTEM \"../schemas/app.dtd\">\n<app/>",
        &path("settings/app.xml"),
        &files,
    );

    assert_eq!(outcome.read, [path("schemas/app.dtd")]);
    assert!(outcome.refused.is_empty(), "{:?}", outcome.refused);
    assert!(outcome.issues.is_empty(), "{:?}", outcome.issues);
}

#[test]
fn a_reference_that_is_not_a_file_of_the_project_is_refused_with_its_reason_and_never_read() {
    for (reference, reason) in [
        ("../../outside.dtd", "leaves the project"),
        (
            "http://example.com/app.dtd",
            "is a web address; ADP reads schemas from the project",
        ),
        ("file:///etc/passwd", "is not a file of the project"),
    ] {
        let asked = RefCell::new(Vec::new());
        let files = |p: &[String]| {
            asked.borrow_mut().push(p.to_vec());
            Some("<!ENTITY leak \"outside\">".to_owned())
        };
        let outcome = check_against_dtd(
            &format!("<!DOCTYPE app SYSTEM \"{reference}\">\n<app>&leak;</app>"),
            &path("app.xml"),
            &files,
        );

        assert_eq!(
            outcome
                .refused
                .iter()
                .map(|r: &Refusal| r.reason.as_str())
                .collect::<Vec<_>>(),
            [reason],
            "{reference}"
        );
        assert!(asked.borrow().is_empty(), "{reference}: {:?}", asked.borrow());
        assert!(outcome.read.is_empty());
    }
}
