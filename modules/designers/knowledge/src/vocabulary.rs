//! The words of the designer's specification that this module implements, written out (the standalone host's
//! `KnowledgeVocabulary` at 8b6ccfc): the value types, the key each is stored under, the comparisons each can be
//! filtered by, and what a view stores.
//!
//! **Written out rather than read from `knowledge.des`, on purpose.** A module that read its vocabulary from the
//! specification would agree with it by construction and implement none of what was added to it. Written here, the
//! two can differ - and a test fails when they do, in either direction (knowledge-designer Requirement 10.5).

/// The value types a property can have.
pub const VALUE_TYPES: [&str; 9] = [
    "text",
    "number",
    "checkbox",
    "date",
    "dateTime",
    "time",
    "selection",
    "multipleSelection",
    "relation",
];

/// The keys a cell or a condition holds its one value under.
pub const VALUE_KEYS: [&str; 7] = ["text", "number", "checked", "date", "dateTime", "time", "option"];

/// The colours an option can have, by name. Each is a token of the theme, never a colour value.
pub const COLOURS: [&str; 10] = [
    "default", "gray", "brown", "orange", "yellow", "green", "blue", "purple", "pink", "red",
];

/// The element types a view's settings are made of.
pub const VIEW_SETTINGS: [&str; 5] = ["Column", "Sort", "Condition", "FilterGroup", "GroupSetting"];

const MOMENTS: &[&str] = &["is", "is-before", "is-after", "is-on-or-before", "is-on-or-after"];

/// The comparisons a filter has for each value type, besides *is empty* and *is not empty*, which every type has.
pub fn comparisons(value_type: &str) -> Option<&'static [&'static str]> {
    Some(match value_type {
        "text" => &[
            "is",
            "is-not",
            "contains",
            "does-not-contain",
            "starts-with",
            "ends-with",
        ],
        "number" => &[
            "equals",
            "does-not-equal",
            "greater-than",
            "less-than",
            "at-least",
            "at-most",
        ],
        "checkbox" => &["is-checked", "is-not-checked"],
        "date" | "dateTime" | "time" => MOMENTS,
        "selection" => &["is", "is-not"],
        "multipleSelection" | "relation" => &["contains", "does-not-contain"],
        _ => return None,
    })
}

/// The value types that have comparisons, in their order.
pub fn compared_types() -> impl Iterator<Item = &'static str> {
    VALUE_TYPES.into_iter().filter(|t| comparisons(t).is_some())
}

/// The key a value of this type is stored under: an attribute of the cell for a type that holds one value, and of
/// each of the cell's items for a type that holds several.
pub fn stored_key(value_type: &str) -> &str {
    match value_type {
        "checkbox" => "checked",
        "selection" | "multipleSelection" => "option",
        "relation" => "row",
        other => other,
    }
}

/// The key a cell holds a value of this type under: one of a cell's attributes, or the name of its list of several
/// (`KnowledgeEdits.KeyOf`).
pub fn key_of(value_type: &str) -> &str {
    match value_type {
        "multipleSelection" => "options",
        "relation" => "rows",
        other => stored_key(other),
    }
}

/// Whether a filter can compare a value of this type in this way.
pub fn compares(value_type: &str, comparison: &str) -> bool {
    matches!(comparison, "is-empty" | "is-not-empty")
        || comparisons(value_type).is_some_and(|c| c.contains(&comparison))
}
