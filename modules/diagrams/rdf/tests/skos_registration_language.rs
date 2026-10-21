//! The `language:` header's band guard (etalii.adp spec 030, task T083), ported from the standalone host's
//! `SkosRegistrationLanguage.Tests.cs` at `8b6ccfc`. The standalone host's shared-read case is not ported: a read here
//! does not lock the file.

use std::fs;
use std::path::PathBuf;

use adp_diagram_rdf::skos::registration_language::read;

fn write(folder: &tempfile::TempDir, content: &str) -> PathBuf {
    let path = folder.path().join("scheme.adp");
    fs::write(&path, content).unwrap();
    path
}

#[test]
fn a_header_in_the_safe_band_is_honoured_and_lowercased() {
    let folder = tempfile::tempdir().unwrap();
    let path = write(
        &folder,
        "w3c/skos\nbody: scheme.ttl\nview: whatever\nlanguage: NL\nlayout:\n  res:x 1 2\n",
    );
    assert_eq!((Some("nl".to_owned()), 0), read(Some(&path)));
}

#[test]
fn a_header_above_a_body_line_is_ignored_and_its_line_reported() {
    let folder = tempfile::tempdir().unwrap();
    let path = write(&folder, "w3c/skos\nlanguage: nl\nbody: scheme.ttl\n");
    assert_eq!((None, 2), read(Some(&path)));
}

#[test]
fn a_header_with_no_core_header_below_is_in_the_band_by_construction() {
    let folder = tempfile::tempdir().unwrap();
    let path = write(&folder, "w3c/skos\nlanguage: de\n");
    assert_eq!((Some("de".to_owned()), 0), read(Some(&path)));
}

#[test]
fn other_readings_headers_are_left_alone_and_layout_ends_the_scan() {
    let folder = tempfile::tempdir().unwrap();
    let path = write(
        &folder,
        "w3c/skos\nbody: scheme.ttl\nresource: something\nlayout:\n  language: fr\n",
    );
    assert_eq!((None, 0), read(Some(&path)));
}

#[test]
fn no_registration_means_the_default_order() {
    let folder = tempfile::tempdir().unwrap();
    assert_eq!((None, 0), read(None));
    assert_eq!((None, 0), read(Some(&folder.path().join("absent.adp"))));
}
