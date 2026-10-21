//! A DED definition's envelope (DED sections 2, 4 and 6): read in every form from every vendored example, checked
//! against `ded.schema.json`, and what a host does with a later version, a designer type it lacks and a
//! registration that disagrees.

use std::path::{Path, PathBuf};

use adp_desl::{DedEnvelope, DedFormat};

fn vendor() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp")
}

fn read(relative: &str) -> Result<DedEnvelope, String> {
    let path = vendor().join(relative);
    let bytes = std::fs::read(&path).expect("the example is vendored");
    DedEnvelope::read(&bytes, path.file_name().and_then(|n| n.to_str()).expect("a name"))
}

#[test]
fn every_vendored_definition_has_its_envelope_and_the_schema_accepts_it() {
    let examples = [
        (
            "definitions/designers/examples/cities.yaml",
            DedFormat::Yaml,
            "0.1",
            "etalii/knowledge",
        ),
        (
            "definitions/designers/examples/cities.json",
            DedFormat::Json,
            "0.1",
            "etalii/knowledge",
        ),
        (
            "definitions/designers/examples/cities.xml",
            DedFormat::Xml,
            "0.1",
            "etalii/knowledge",
        ),
        (
            "definitions/designers/examples/web-shop.md",
            DedFormat::Markdown,
            "0.2",
            "etalii/hierarchical-requirement",
        ),
        (
            "specifications/ded/library.ded",
            DedFormat::Json,
            "0.1",
            "etalii/knowledge",
        ),
        (
            "specifications/ded/web-shop.md",
            DedFormat::Markdown,
            "0.2",
            "etalii/hierarchical-requirement",
        ),
    ];
    for (relative, format, ded, designer) in examples {
        let envelope = read(relative).unwrap_or_else(|e| panic!("{relative}: {e}"));
        assert_eq!(
            (format, ded, designer),
            (envelope.format, envelope.ded.as_str(), envelope.designer.as_str())
        );
        assert!(envelope.findings().is_empty(), "{relative}: {:?}", envelope.findings());
        assert_eq!(None, envelope.read_only_reason(|origin| origin == designer));
    }
}

#[test]
fn a_file_without_the_envelope_is_no_definition() {
    for (text, name) in [
        ("# Notes\n\nNothing here.\n", "notes.md"),
        ("name: Cities\nded: \"0.1\"\ndesigner: etalii/knowledge\n", "late.yaml"),
        (
            "{ \"designer\": \"etalii/knowledge\", \"ded\": \"0.1\" }",
            "swapped.json",
        ),
        ("<table version=\"0.1\" designer=\"etalii/knowledge\"/>", "other.xml"),
        ("ded: \"0.1\"\n", "half.yaml"),
        ("ded = 0.1", "cities.toml"),
    ] {
        assert!(DedEnvelope::read(text.as_bytes(), name).is_err(), "{name}");
    }
}

#[test]
fn the_envelope_is_read_around_what_its_form_allows() {
    let yaml = "\u{feff}# A comment before.\nded: '0.2' # quoted\n\ndesigner: etalii/knowledge\nname: x\n";
    let envelope = DedEnvelope::read(yaml.as_bytes(), "a.yml").expect("an envelope");
    assert_eq!(
        ("0.2", "etalii/knowledge"),
        (envelope.ded.as_str(), envelope.designer.as_str())
    );

    let xml = "<?xml version=\"1.0\"?>\n<!-- made by hand -->\n<ded version='0.1' designer='etalii/knowledge' name='x'></ded>";
    assert_eq!(
        "0.1",
        DedEnvelope::read(xml.as_bytes(), "a.xml").expect("an envelope").ded
    );

    let markdown = "<!--\tded:  0.2 designer: etalii/hierarchical-requirement -->  \n# Title\n";
    assert_eq!(
        "0.2",
        DedEnvelope::read(markdown.as_bytes(), "a.md").expect("an envelope").ded
    );
}

#[test]
fn an_envelope_the_schema_refuses_is_said_to_be_wrong() {
    let envelope = DedEnvelope::read(b"ded: \"0.1\"\ndesigner: Etalii Knowledge\n", "a.yaml").expect("an envelope");
    let findings = envelope.findings();
    assert_eq!(1, findings.len(), "{findings:?}");
    assert!(findings[0].contains("designer"), "{}", findings[0]);
}

#[test]
fn a_later_version_or_a_designer_type_not_installed_opens_read_only_with_the_reason() {
    let later = DedEnvelope::read(b"ded: \"9.0\"\ndesigner: etalii/knowledge\n", "a.yaml").expect("an envelope");
    let reason = later.read_only_reason(|_| true).expect("read-only");
    assert!(reason.contains("DED 9.0"), "{reason}");

    let foreign = read("definitions/designers/examples/web-shop.md").expect("an envelope");
    let reason = foreign
        .read_only_reason(|origin| origin == "etalii/knowledge")
        .expect("read-only");
    assert!(reason.contains("'etalii/hierarchical-requirement'"), "{reason}");
}

#[test]
fn a_registration_that_names_another_designer_type_is_a_finding_and_the_envelope_decides() {
    let envelope = read("definitions/designers/examples/cities.yaml").expect("an envelope");
    assert_eq!(None, envelope.registration_finding("etalii/knowledge"));
    let finding = envelope.registration_finding("etalii/other").expect("a finding");
    assert!(finding.ends_with("it is opened with 'etalii/knowledge'."), "{finding}");
}
