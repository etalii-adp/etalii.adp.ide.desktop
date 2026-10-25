//! The module against the designer's specification (the standalone host's `DefinitionConformance.Tests.cs` at
//! a7fdf03): its name, its element types and their attributes, the document's attributes, the enumerations and the
//! operations are those of `hierarchical-requirement.des`, in both directions. A difference is reported as what the
//! module is *missing* or as what is *unknown to the definition*, so the failure says which side has to change. The
//! specification is read here from the file itself, not through the module's loader, which would compare the module
//! with itself.
//!
//! The standalone host's checks of its bundled copy against its provenance have no counterpart here: the module
//! reads the vendored files at build time, and `tools/sync-etalii-adp.py --check` holds the vendored folder to its
//! commit.

mod common;

use std::collections::BTreeSet;

use adp_designer_hierarchical_requirement::{ORIGIN, definition, vocabulary};
use serde_json::Value;

fn specification() -> Value {
    let text = std::fs::read_to_string(common::definition_folder().join("hierarchical-requirement.des")).unwrap();
    serde_json::from_str(text.trim_start_matches('\u{feff}')).unwrap()
}

fn names(value: &Value) -> Vec<String> {
    value
        .as_object()
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default()
}

/// What one side has and the other does not, as the sentences a failure is made of.
fn differences<'a>(
    what: &str,
    defined: impl IntoIterator<Item = &'a str>,
    implemented: impl IntoIterator<Item = &'a str>,
) -> Vec<String> {
    let defined: BTreeSet<&str> = defined.into_iter().collect();
    let implemented: BTreeSet<&str> = implemented.into_iter().collect();
    defined
        .difference(&implemented)
        .map(|name| format!("{what} '{name}' is missing from the module."))
        .chain(
            implemented
                .difference(&defined)
                .map(|name| format!("{what} '{name}' is unknown to the definition.")),
        )
        .collect()
}

#[test]
fn the_modules_name_is_the_definitions() {
    let specification = specification();
    assert_eq!(Some(vocabulary::NAME), specification["language"]["label"].as_str());
    assert_eq!(Some(ORIGIN), specification["language"]["origin"].as_str());
}

#[test]
fn the_modules_types_and_attributes_are_the_definitions() {
    let specification = specification();
    let types = &specification["metamodel"]["types"];
    let defined = names(types);
    assert_eq!(8, defined.len());
    let mut differing = differences(
        "The type",
        defined.iter().map(String::as_str),
        vocabulary::TYPES.iter().map(|(name, _)| *name),
    );
    for type_name in &defined {
        let implemented = vocabulary::TYPES
            .iter()
            .find(|(name, _)| name == type_name)
            .map_or(&[][..], |(_, attributes)| attributes);
        let attributes = names(&types[type_name]["attributes"]);
        differing.extend(differences(
            &format!("{type_name}'s attribute"),
            attributes.iter().map(String::as_str),
            implemented.iter().copied(),
        ));
    }
    let document = names(&specification["metamodel"]["document"]["attributes"]);
    differing.extend(differences(
        "The document's attribute",
        document.iter().map(String::as_str),
        vocabulary::DOCUMENT_ATTRIBUTES,
    ));
    assert!(differing.is_empty(), "{differing:#?}");
}

#[test]
fn the_modules_enumerations_are_the_definitions() {
    let specification = specification();
    let enums = &specification["metamodel"]["enums"];
    let defined = names(enums);
    assert_eq!(2, defined.len());
    let mut differing = differences(
        "The enumeration",
        defined.iter().map(String::as_str),
        vocabulary::ENUMERATIONS.iter().map(|(name, _)| *name),
    );
    for enumeration in &defined {
        let implemented = vocabulary::ENUMERATIONS
            .iter()
            .find(|(name, _)| name == enumeration)
            .map_or(&[][..], |(_, values)| values);
        let values = names(&enums[enumeration]["values"]);
        differing.extend(differences(
            &format!("{enumeration}'s value"),
            values.iter().map(String::as_str),
            implemented.iter().copied(),
        ));
    }
    assert!(differing.is_empty(), "{differing:#?}");
    assert_eq!(10, vocabulary::COLOURS.len());
}

#[test]
fn the_modules_operations_are_the_definitions() {
    let operations = names(&specification()["operations"]);
    assert_eq!(41, operations.len());
    let differing = differences(
        "The operation",
        operations.iter().map(String::as_str),
        vocabulary::OPERATIONS,
    );
    assert!(differing.is_empty(), "{differing:#?}");
}

#[test]
fn a_difference_is_named_in_its_direction() {
    // The guard's own control: what the module lacks, and what only the module has, are reported as such.
    assert_eq!(
        vec![
            "The operation 'addTag' is missing from the module.",
            "The operation 'addLink' is unknown to the definition.",
        ],
        differences("The operation", ["setTitle", "addTag"], ["setTitle", "addLink"])
    );
    assert!(differences("The operation", ["a", "b"], ["b", "a"]).is_empty());
}

#[test]
fn the_specification_was_read() {
    // Without its types a resolved checkbox is text, and without many the tags are one word.
    assert!(
        definition::typed_attribute_count() > 20,
        "only {} attribute types were read",
        definition::typed_attribute_count()
    );
    assert!(definition::is_many("Requirement", "tags"));
    assert!(definition::mentions("Requirement", "description").is_some());
    assert_eq!(3, definition::initial("categories").len());
}
