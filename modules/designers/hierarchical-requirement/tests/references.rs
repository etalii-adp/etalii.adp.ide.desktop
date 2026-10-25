//! A reference is text, so the module writes one as it writes any description, and keeps it when its target goes (the
//! standalone host's `References.Tests.cs` at a7fdf03; spec 029 T089, FR-026; 028's User Story 6): the steps of
//! `hierarchical-requirement-references` run through the module's operations make the fixture's bytes, and the
//! document its delete leaves shows each broken reference as one that names nothing, with the finding of
//! `references/deleted` at its line.

mod common;

use std::fs;

use adp_designer_hierarchical_requirement::body::RequirementBody;
use adp_designer_hierarchical_requirement::mapper;
use adp_designer_hierarchical_requirement::operations::structure::{self, DELETE_ENTRY, MOVE_REQUIREMENT};
use adp_designer_hierarchical_requirement::operations::text::{self, SET_DESCRIPTION};
use adp_desl::hierarchy::model::SurfaceNode;
use common::params;
use serde_json::Value as Json;

const FIXTURE: &str = "hierarchical-requirement-references";

/// Every step of the fixture that is not an undo: the step, the gesture, its target and its params.
const STEPS: [(usize, &str, &str, &str); 4] = [
    (
        0,
        SET_DESCRIPTION,
        "REQ-0003",
        "Only once [REQ-0002] has confirmed the payment.",
    ),
    (
        2,
        SET_DESCRIPTION,
        "REQ-0001",
        "The provider's terms are on [REQ-0099](https://example.com/terms), a link and no reference.\nThe card rules are in [REQ-0002].",
    ),
    (4, MOVE_REQUIREMENT, "REQ-0002", "item=ITEM-0002|after=REQ-0003"),
    (6, DELETE_ENTRY, "REQ-0002", ""),
];

#[test]
fn a_gesture_writes_the_fixtures_bytes() {
    let folder = common::fixture(FIXTURE);
    let json: Json = serde_json::from_str(&fs::read_to_string(folder.join("fixture.json")).unwrap()).unwrap();
    let input = fs::read(folder.join("requirements.md")).unwrap();
    for (step, operation, target, value) in STEPS {
        // Act.
        let bytes = if text::handles(operation) {
            text::apply(&input, "requirements.md", operation, target, value)
        } else {
            structure::apply(&input, "requirements.md", operation, target, &params(value))
        };

        // Assert.
        let written = String::from_utf8(bytes.unwrap_or_else(|r| panic!("step {step} is refused: {r}"))).unwrap();
        let expected = &json["steps"][step];
        let expected = match expected["expectFile"].as_str() {
            Some(file) => fs::read_to_string(folder.join(file)).unwrap(),
            None => expected["expect"].as_str().unwrap().to_owned(),
        };
        assert_eq!(expected, written, "step {step}");
    }
}

#[test]
fn a_deleted_target_leaves_its_references_as_written_naming_nothing_and_a_finding_at_each_line() {
    // Arrange: the document the delete of REQ-0002 leaves.
    let folder = common::fixture(&format!("{FIXTURE}/deleted"));
    let json: Json = serde_json::from_str(&fs::read_to_string(folder.join("fixture.json")).unwrap()).unwrap();
    let bytes = fs::read(folder.join("requirements.md")).unwrap();

    // Act.
    let document = RequirementBody::read(bytes, "requirements.md", None).document;
    let model = mapper::map(&document, "Payments", "open", &|_| None);

    // Assert: the fixture's std.mentions findings, at its lines, and nothing else.
    let findings: Vec<String> = json["read"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| format!("{}@{}", f["rule"].as_str().unwrap(), f["line"]))
        .collect();
    let found: Vec<String> = model
        .findings
        .iter()
        .map(|f| format!("{}@{}", f.code, f.line))
        .collect();
    assert_eq!(findings, found);
    assert!(model.findings.iter().all(|f| f.severity == "warning"));

    // The box in ITEM-0001's description names nothing; the assessments' other references still name their entries.
    let node = |number: &str| -> &SurfaceNode { model.nodes.iter().find(|n| n.number == number).unwrap() };
    let checkout = node("ITEM-0001");
    let [broken] = checkout.references.as_slice() else {
        panic!("one reference: {:?}", checkout.references);
    };
    let units: Vec<u16> = checkout
        .description
        .encode_utf16()
        .skip(broken.start)
        .take(broken.length)
        .collect();
    let shown = String::from_utf16(&units).unwrap();
    assert_eq!(("[REQ-0002]", ""), (shown.as_str(), broken.target_id.as_str()));
    // Its finding stands at ITEM-0001, whose description holds it, so selecting it shows that box (FR-021).
    assert!(model.findings.iter().any(|f| f.node_id == checkout.id));
    // A link that writes a number is no reference: REQ-0001's description holds no box.
    assert!(node("REQ-0001").references.is_empty());
    let named: Vec<&str> = model
        .notes
        .iter()
        .flat_map(|n| &n.references)
        .map(|r| r.target_id.as_str())
        .collect();
    assert!(named.contains(&node("REQ-0003").id.as_str()));
    assert!(named.contains(&node("ITEM-0003").id.as_str()));
    assert!(named.contains(&""));
}
