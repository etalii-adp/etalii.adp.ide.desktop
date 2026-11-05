//! DISL 0.9's worked example of `groupBy` (section 12.4; spec 075, T010), as vendored: the terms derived from the
//! keys of the statements grouped by subject, each labelled from a second grouping, and the type whose key fails,
//! which has no elements and one `std.derivedFailed` finding; with the findings of `grouping-terms.findings.json`.

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

#[test]
fn the_grouping_example_derives_its_terms_and_gives_the_findings_of_its_findings_file() {
    let definition: serde_json::Value = serde_json::from_str(&vendored("grouping-terms.did")).unwrap();
    let specification = loaded(&vendored("grouping-terms.dis"));
    let diagram = DislDiagram::new(&specification);
    for element in definition["elements"].as_array().unwrap() {
        let mut new = NewElement::new("Statement", element["id"].as_str().unwrap());
        let attribute = |name: &str| element["attributes"][name].as_str().unwrap().into();
        new.attributes = attrs([
            ("subject", attribute("subject")),
            ("predicate", attribute("predicate")),
            ("object", attribute("object")),
        ]);
        diagram.add(new).unwrap();
    }
    let derivation = builder::complete(&diagram);

    let terms: Vec<(String, String)> = diagram
        .nodes_of_type("Term")
        .iter()
        .map(|t| (t.id(), t.value_of("label").as_str().unwrap().to_owned()))
        .collect();
    assert_eq!(
        terms,
        [("alice", "Alice"), ("bob", "bob"), ("carol", "Carol")].map(|(i, l)| (i.to_owned(), l.to_owned()))
    );
    assert!(
        diagram.nodes_of_type("Weight").is_empty(),
        "a failing key fails the whole grouping"
    );

    let expected: serde_json::Value = serde_json::from_str(&vendored("grouping-terms.findings.json")).unwrap();
    let expected = expected["findings"].as_array().unwrap();
    assert_eq!(derivation.len(), 1, "{derivation:?}");
    assert_eq!(derivation[0].code, expected[0]["constraintId"].as_str().unwrap());
    assert_eq!(derivation[0].message, expected[0]["message"].as_str().unwrap());
    let findings = constraints::evaluate(&specification, &diagram, &DislConstraintOptions::default());
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0].constraint_id, expected[1]["constraintId"].as_str().unwrap());
    assert_eq!(findings[0].message, expected[1]["message"].as_str().unwrap());
    assert_eq!(
        findings[0].element_ids,
        vec![expected[1]["elementId"].as_str().unwrap().to_owned()]
    );
}
