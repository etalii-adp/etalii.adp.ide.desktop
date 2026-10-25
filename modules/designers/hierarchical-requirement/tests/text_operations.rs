//! `setTitle` and `setDescription` make the bytes the conformance fixtures state (the standalone host's
//! `TextOperations.Tests.cs` at a7fdf03; spec 029 T066, FR-026, R17): a replace of the one value, and, in a document
//! holding placeholders, their numbers and the mark written by the same edit. Each step is the fixture's own, run
//! through the module's operation from the surface's id, not through the fixture runner's stand-in.

mod common;

use std::fs;

use adp_designer_hierarchical_requirement::gestures::DRIFTED;
use adp_designer_hierarchical_requirement::operations::text::{self, SET_DESCRIPTION, SET_TITLE};
use serde_json::Value as Json;

/// Every step of the corpus that changes one title or one description, by fixture and step.
const STEPS: [(&str, usize); 4] = [
    ("hierarchical-requirement-edits", 1),
    ("hierarchical-requirement-references", 0),
    ("hierarchical-requirement-references", 2),
    ("hierarchical-requirement-faults", 1),
];

#[test]
fn a_set_of_one_text_writes_the_fixtures_bytes() {
    for (fixture, step) in STEPS {
        // Arrange: the fixture's input, and its step's one changed value.
        let folder = common::fixture(fixture);
        let json: Json = serde_json::from_str(&fs::read_to_string(folder.join("fixture.json")).unwrap()).unwrap();
        let input_name = json["input"].as_str().unwrap();
        let input = fs::read(folder.join(input_name)).unwrap();
        let stated = &json["steps"][step];
        let set = &stated["edit"]["set"];
        let attributes = set["attributes"].as_object().unwrap();
        assert_eq!(attributes.len(), 1, "{fixture} step {step} sets one value");
        let (attribute, value) = attributes.iter().next().unwrap();
        let operation = if attribute == "title" {
            SET_TITLE
        } else {
            SET_DESCRIPTION
        };

        // Act.
        let bytes = text::apply(
            &input,
            input_name,
            operation,
            set["element"].as_str().unwrap(),
            value.as_str().unwrap(),
        );

        // Assert.
        let bytes = bytes.unwrap_or_else(|refusal| panic!("{fixture} step {step} is refused: {refusal}"));
        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            stated["expect"].as_str().unwrap(),
            "{fixture} step {step}"
        );
    }
}

#[test]
fn a_title_of_two_lines_is_refused_and_an_entry_the_file_no_longer_has_is_a_change_made_against_an_older_file() {
    let input = fs::read(common::fixture("hierarchical-requirement-edits/web-shop.md")).unwrap();

    assert_eq!(
        text::apply(&input, "web-shop.md", SET_TITLE, "REQ-0004", "One\nTwo"),
        Err("A title is one line; the description holds the lines below it.".to_owned())
    );
    assert_eq!(
        text::apply(&input, "web-shop.md", SET_TITLE, "REQ-0999", "Gone"),
        Err(DRIFTED.to_owned())
    );
}
