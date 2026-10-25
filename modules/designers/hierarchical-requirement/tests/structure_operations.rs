//! The adds, the moves, the delete and giving a duplicate a number make the bytes the conformance fixtures state (the
//! standalone host's `StructureOperations.Tests.cs` at a7fdf03; spec 029 T073, FR-026, R17): each step is the fixture's
//! own, run through the module's operation from the surface's ids, against the bytes the step before it left.

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use adp_designer_hierarchical_requirement::body::RequirementBody;
use adp_designer_hierarchical_requirement::gestures::DRIFTED;
use adp_designer_hierarchical_requirement::operations::structure::{self, *};
use serde_json::Value as Json;

/// Every step of the corpus that shapes the hierarchy: the fixture, the step, and the gesture that makes it.
const STEPS: [(&str, usize, &str, &str, &str); 9] = [
    ("hierarchical-requirement-structure", 0, ADD_REQUIREMENT_BELOW, "REQ-0004", ""),
    ("hierarchical-requirement-structure", 1, ADD_ITEM, "ITEM-0001", ""),
    ("hierarchical-requirement-structure", 2, MOVE_ITEM, "ITEM-0003", "item=ITEM-0004"),
    ("hierarchical-requirement-structure", 3, DELETE_ENTRY, "ITEM-0002", ""),
    ("hierarchical-requirement-structure", 4, ADD_REQUIREMENT, "ITEM-0001", ""),
    ("hierarchical-requirement-structure/no-section", 0, ADD_TOP_ITEM, "", ""),
    ("hierarchical-requirement-new", 1, ADD_TOP_ITEM, "", ""),
    ("hierarchical-requirement-edits/no-mark", 2, ADD_TOP_ITEM, "", ""),
    ("hierarchical-requirement-edits", 3, RENUMBER_DUPLICATE, "line/14", ""),
];

fn expected(folder: &Path, step: &Json) -> String {
    match step["expect"].as_str() {
        Some(expect) => expect.to_owned(),
        None => fs::read_to_string(folder.join(step["expectFile"].as_str().unwrap())).unwrap(),
    }
}

fn params(written: &str) -> BTreeMap<String, String> {
    written
        .split(';')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (key, value) = pair.split_once('=').unwrap();
            (key.to_owned(), value.to_owned())
        })
        .collect()
}

fn catalogue() -> Vec<u8> {
    fs::read(common::fixture("hierarchical-requirement-structure/catalogue.md")).unwrap()
}

fn numbering() -> Vec<u8> {
    fs::read(common::fixture("hierarchical-requirement-edits/numbering/web-shop.md")).unwrap()
}

/// The numbers of the entries directly in `parent`, in their order.
fn children(body: &[u8], parent: &str) -> Vec<String> {
    RequirementBody::read(body.to_vec(), "catalogue.md", None)
        .document
        .entries
        .into_iter()
        .filter(|e| e.parent_id.as_deref() == Some(parent))
        .map(|e| e.id)
        .collect()
}

fn lf(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).unwrap().replace("\r\n", "\n")
}

#[test]
fn a_gesture_writes_the_fixtures_bytes() {
    for (fixture, step, operation, target, written) in STEPS {
        // Arrange: the bytes the step starts from, the input's or those the step before it states.
        let folder = common::fixture(fixture);
        let json: Json = serde_json::from_str(&fs::read_to_string(folder.join("fixture.json")).unwrap()).unwrap();
        let input_name = json["input"].as_str().unwrap();
        let steps = &json["steps"];
        let input = if step == 0 {
            fs::read(folder.join(input_name)).unwrap()
        } else {
            expected(&folder, &steps[step - 1]).into_bytes()
        };

        // Act.
        let bytes = structure::apply(&input, input_name, operation, target, &params(written));

        // Assert.
        let bytes = bytes.unwrap_or_else(|refusal| panic!("{fixture} step {step} is refused: {refusal}"));
        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            expected(&folder, &steps[step]),
            "{fixture} step {step}"
        );
    }
}

#[test]
fn an_entry_holding_placeholders_is_deleted_and_moved_after_they_are_numbered() {
    // Arrange: ITEM-NEW on line 10 holds a REQ-NEW; both are numbered by the edit that deletes or moves them.
    let input = numbering();

    // Act.
    let deleted = structure::apply(&input, "web-shop.md", DELETE_ENTRY, "line/10", &params(""));
    let moved = structure::apply(&input, "web-shop.md", MOVE_ITEM, "line/10", &params("after=ITEM-0002"));

    // Assert: the numbers are given and kept by the marks, the item goes with what it holds.
    let head = "<!-- ded: 0.2 designer: etalii/hierarchical-requirement -->\n<!-- last: ITEM-0003 REQ-0005 -->\n# Web shop\n\n## Requirements\n\n- ITEM-0001 Backend #core\n  - REQ-0001 A session ends after 30 minutes without activity. #security\n  - REQ-0004 Tokens are refreshed without the user noticing.\n- ITEM-0002 Data\n  - REQ-0002 Personal data is kept no longer than it is needed.\n";
    assert_eq!(lf(deleted.unwrap()), head);
    assert_eq!(
        lf(moved.unwrap()),
        format!(
            "{head}- ITEM-0003 Payments\n  - REQ-0005 A customer pays by card or by bank transfer. category:functional\n"
        )
    );
}

#[test]
fn a_move_within_its_parent_goes_after_the_sibling_named_or_to_the_front_of_its_kind() {
    let input = catalogue();

    let first = structure::apply(&input, "catalogue.md", MOVE_REQUIREMENT, "REQ-0006", &params("item=ITEM-0003")).unwrap();
    let last = structure::apply(
        &input,
        "catalogue.md",
        MOVE_REQUIREMENT,
        "REQ-0004",
        &params("item=ITEM-0003;after=REQ-0006"),
    )
    .unwrap();
    let same = structure::apply(&input, "catalogue.md", MOVE_ITEM, "ITEM-0001", &params("")).unwrap();

    assert_eq!(children(&first, "ITEM-0003"), ["REQ-0006", "REQ-0004"]);
    assert_eq!(children(&last, "ITEM-0003"), ["REQ-0006", "REQ-0004"]);
    assert_eq!(same, input);
}

#[test]
fn a_requirement_moved_into_another_item_goes_where_a_new_requirement_would() {
    let input = catalogue();

    let after_requirements =
        structure::apply(&input, "catalogue.md", MOVE_REQUIREMENT, "REQ-0007", &params("item=ITEM-0001")).unwrap();
    let into_empty = structure::apply(
        &input,
        "catalogue.md",
        MOVE_REQUIREMENT,
        "REQ-0001",
        &params("item=ITEM-0004;after=REQ-0007"),
    )
    .unwrap();

    assert_eq!(
        children(&after_requirements, "ITEM-0001"),
        ["REQ-0001", "REQ-0002", "REQ-0007", "ITEM-0002"]
    );
    assert_eq!(children(&into_empty, "ITEM-0004"), ["REQ-0007", "REQ-0001"]);
}

#[test]
fn what_the_specification_rolls_back_or_makes_unavailable_is_refused_and_nothing_is_written() {
    let catalogue = catalogue();
    let top_level = b"<!-- ded: 0.2 designer: etalii/hierarchical-requirement -->\r\n# Alone\r\n\r\n## Requirements\r\n\r\n- REQ-0001 Stands in no item.\r\n";
    let refused = |body: &[u8], name: &str, operation: &str, target: &str, written: &str| {
        structure::apply(body, name, operation, target, &params(written)).unwrap_err()
    };
    let into_itself = "ITEM-0001 cannot be moved into itself or into anything it holds.";

    assert_eq!(refused(top_level, "alone.md", ADD_REQUIREMENT_BELOW, "REQ-0001", ""), NO_ITEM);
    assert_eq!(refused(&catalogue, "catalogue.md", MOVE_ITEM, "ITEM-0001", "item=ITEM-0001"), into_itself);
    assert_eq!(refused(&catalogue, "catalogue.md", MOVE_ITEM, "ITEM-0001", "item=ITEM-0002"), into_itself);
    assert_eq!(
        refused(&catalogue, "catalogue.md", MOVE_ITEM, "ITEM-0003", "item=REQ-0001"),
        "REQ-0001 is a requirement, which holds nothing."
    );
    assert_eq!(
        refused(&catalogue, "catalogue.md", RENUMBER_DUPLICATE, "REQ-0001", ""),
        "REQ-0001 is the only entry with its number, which it keeps."
    );
    assert_eq!(refused(&catalogue, "catalogue.md", DELETE_ENTRY, "ITEM-0099", ""), DRIFTED);
    assert_eq!(refused(&catalogue, "catalogue.md", MOVE_ITEM, "ITEM-0003", "after=ITEM-0099"), DRIFTED);
}
