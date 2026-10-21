//! The module against the designer's specification (the standalone host's `DefinitionConformance.Tests.cs` at
//! 8b6ccfc; knowledge-designer Requirement 10.5): its value types, the key each is stored under, the comparisons of
//! each and what a view stores are those of `knowledge.des`, in both directions. The specification is read here from
//! the file itself, not through the module's own loader: a guard that asked the module what the specification says
//! would compare the module with itself.

mod common;

use std::collections::BTreeSet;

use adp_designer_knowledge::vocabulary;
use serde_json::Value;

fn definition() -> Value {
    let text = std::fs::read_to_string(common::vendor().join("definitions/designers/knowledge.des")).expect("vendored");
    serde_json::from_str(text.trim_start_matches('\u{feff}')).expect("JSON")
}

fn names(value: &Value) -> Vec<String> {
    value
        .as_object()
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default()
}

/// What one side has and the other does not, as the two sentences a failure is made of.
fn differences<'a, 'b>(
    what: &str,
    defined: impl IntoIterator<Item = &'a str>,
    implemented: impl IntoIterator<Item = &'b str>,
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
fn the_modules_value_types_are_the_definitions() {
    let definition = definition();
    let defined = names(&definition["metamodel"]["enums"]["ValueType"]["values"]);
    assert_eq!(9, defined.len());
    assert!(
        differences(
            "The value type",
            defined.iter().map(String::as_str),
            vocabulary::VALUE_TYPES
        )
        .is_empty()
    );
    // And each is shown: a type the surface does not describe has no key, no editor and no comparison.
    let shown = names(&definition["surface"]["valueTypes"]);
    assert!(
        differences(
            "The shown value type",
            shown.iter().map(String::as_str),
            vocabulary::VALUE_TYPES
        )
        .is_empty()
    );
}

#[test]
fn each_value_type_is_stored_under_the_key_the_definition_gives_it() {
    let definition = definition();
    let surface = definition["surface"]["valueTypes"].as_object().expect("value types");
    let differing: Vec<String> = surface
        .iter()
        .filter(|(name, value)| value["key"].as_str() != Some(vocabulary::stored_key(name)))
        .map(|(name, value)| {
            format!(
                "{name}: the definition stores it under '{}', the module under '{}'.",
                value["key"],
                vocabulary::stored_key(name)
            )
        })
        .collect();
    assert!(differing.is_empty(), "{differing:?}");

    // The keys of a cell that holds one value are the cell's value attributes in the metamodel.
    let attributes: Vec<String> = names(&definition["metamodel"]["types"]["Cell"]["attributes"])
        .into_iter()
        .filter(|n| n != "property")
        .collect();
    assert!(
        differences(
            "The cell's value key",
            attributes.iter().map(String::as_str),
            vocabulary::VALUE_KEYS
        )
        .is_empty()
    );
}

#[test]
fn each_value_types_comparisons_are_the_definitions_in_its_order() {
    let definition = definition();
    let surface = definition["surface"]["valueTypes"].as_object().expect("value types");
    let mut differing = Vec::new();
    for (name, value) in surface {
        let defined: Vec<&str> = value["comparisons"]
            .as_array()
            .expect("comparisons")
            .iter()
            .filter_map(Value::as_str)
            .collect();
        let implemented: Vec<&str> = vocabulary::comparisons(name).map(<[&str]>::to_vec).unwrap_or_default();
        differing.extend(differences(
            &format!("For {name}, the comparison"),
            defined.iter().copied(),
            implemented.iter().copied(),
        ));
        if differing.is_empty() && defined != implemented {
            differing.push(format!(
                "For {name}, the comparisons are in another order: the first is what a new condition starts as."
            ));
        }
    }
    assert!(differing.is_empty(), "{differing:?}");
    let typed = names(&definition["surface"]["valueTypes"]);
    assert!(
        differences(
            "The type with comparisons",
            typed.iter().map(String::as_str),
            vocabulary::compared_types().collect::<Vec<_>>()
        )
        .is_empty()
    );
}

#[test]
fn what_a_view_stores_is_what_the_definition_says_it_does() {
    let definition = definition();
    let settings: Vec<&str> = definition["surface"]["views"]["settings"]
        .as_object()
        .expect("settings")
        .values()
        .filter_map(Value::as_str)
        .collect();
    assert!(!settings.is_empty());
    assert!(differences("The view setting", settings, vocabulary::VIEW_SETTINGS).is_empty());
}

#[test]
fn an_options_colours_are_the_definitions() {
    let definition = definition();
    let colours = names(&definition["metamodel"]["enums"]["Colour"]["values"]);
    assert_eq!(10, colours.len());
    assert!(differences("The colour", colours.iter().map(String::as_str), vocabulary::COLOURS).is_empty());
}

#[test]
fn a_difference_is_named_in_its_direction() {
    // The guard's own control: what a type removed from the module, and one added only to it, are reported as.
    assert_eq!(
        vec![
            "The value type 'time' is missing from the module.",
            "The value type 'person' is unknown to the definition."
        ],
        differences("The value type", ["text", "time"], ["text", "person"])
    );
    assert!(differences("The value type", ["text", "time"], ["time", "text"]).is_empty());
}
