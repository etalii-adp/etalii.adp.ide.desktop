//! A stored relation's end may resolve to a derived node (DISL 0.8, section 4.9), checked against etalii.adp's example as
//! vendored: a dependency on a task nobody declares has the placeholder for its source, stays as it was written, and a
//! rule tells that end from a stored one.

mod common;

use adp_disl::derivation::constraints;
use adp_disl::model::builder;
use adp_disl::{DislConstraintOptions, DislDiagram, NewElement};
use common::{attrs, loaded};

const VENDORED: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../vendor/etalii.adp/specifications/disl"
);

fn vendored(path: &str) -> String {
    std::fs::read_to_string(format!("{VENDORED}/{path}")).expect("the vendored document is there")
}

/// The definition of etalii.adp's `placeholder-tasks.did`, built on the vendored `placeholder-tasks.dis`, draws the
/// placeholder's edge and gives exactly the finding of `placeholder-tasks.findings.json`.
#[test]
fn the_placeholder_example_resolves_its_end_and_gives_the_finding_of_its_findings_file() {
    let definition: serde_json::Value = serde_json::from_str(&vendored("placeholder-tasks.did")).unwrap();
    let specification = loaded(&vendored("placeholder-tasks.dis"));
    let diagram = DislDiagram::new(&specification);
    for element in definition["elements"].as_array().unwrap() {
        let mut new = NewElement::new("Task", element["id"].as_str().unwrap());
        new.attributes = attrs([("name", element["attributes"]["name"].as_str().unwrap().into())]);
        diagram.add(new).unwrap();
    }
    for relation in definition["relations"].as_array().unwrap() {
        let mut new = NewElement::new("Dependency", relation["id"].as_str().unwrap());
        let end = |name: &str| relation[name].as_str().unwrap().to_owned();
        new.attributes = attrs([("upstream", relation["attributes"]["upstream"].as_str().unwrap().into())]);
        new.source = diagram.element_by_id(&end("source"));
        new.target = diagram.element_by_id(&end("target"));
        new.source_id = Some(end("source"));
        new.target_id = Some(end("target"));
        diagram.add(new).unwrap();
    }
    assert!(builder::complete(&diagram).is_empty());

    let relation = diagram.element_by_id("dep-2").unwrap();
    let source = relation.source().expect("the end names the placeholder");
    assert_eq!(source.id(), "ghost");
    assert!(source.is_derived());
    assert_eq!(relation.source_id().as_deref(), Some("ghost"));

    let expected: serde_json::Value = serde_json::from_str(&vendored("placeholder-tasks.findings.json")).unwrap();
    let want = &expected["findings"][0];
    let findings = constraints::evaluate(&specification, &diagram, &DislConstraintOptions::default());
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0].constraint_id, want["constraintId"].as_str().unwrap());
    assert_eq!(findings[0].message, want["message"].as_str().unwrap());
    assert_eq!(
        findings[0].element_ids,
        vec![want["elementId"].as_str().unwrap().to_owned()]
    );
}
