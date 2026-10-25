//! The words of the designer's specification that this module implements, written out (the standalone host's
//! `HierarchicalRequirementVocabulary` at a7fdf03): its name, the element types and their attributes, the document's
//! attributes, the enumerations and their values, and the operations.
//!
//! **Written out rather than read from `hierarchical-requirement.des`, on purpose**, as the Knowledge module writes
//! its own: a module that read its vocabulary from the specification would agree with it by construction and
//! implement none of what was added to it. Written here, the two can differ, and `tests/definition_conformance.rs`
//! fails when they do, in either direction.

/// The designer type's name, as its specification labels it.
pub const NAME: &str = "Hierarchical Requirement Designer";

/// The element types and the attributes of each, in the specification's order.
pub const TYPES: [(&str, &[&str]); 8] = [
    ("Item", &["title", "description", "tags", "collapsed"]),
    (
        "Requirement",
        &["title", "description", "tags", "category", "state", "collapsed"],
    ),
    ("Assessment", &["kind", "about", "text", "detail", "resolved"]),
    ("Tag", &["name", "colour"]),
    ("Category", &["name", "colour"]),
    ("State", &["name", "colour"]),
    ("Filter", &["name", "query"]),
    ("Column", &["level", "width"]),
];

/// The document's own attributes: the marks of the two counters, on line 2 of the file.
pub const DOCUMENT_ATTRIBUTES: [&str; 2] = ["lastItem", "lastRequirement"];

/// The ten colour names a member of a list can have.
pub const COLOURS: [&str; 10] = [
    "default", "gray", "brown", "orange", "yellow", "green", "blue", "purple", "pink", "red",
];

/// The enumerations and their values, by key.
pub const ENUMERATIONS: [(&str, &[&str]); 2] = [
    ("Colour", &COLOURS),
    ("AssessmentKind", &["question", "remark", "warning", "idea"]),
];

/// The operations the designer offers, by name.
pub const OPERATIONS: [&str; 41] = [
    "addTopItem",
    "addItem",
    "addItemBelow",
    "addRequirement",
    "addRequirementBelow",
    "setTitle",
    "setDescription",
    "moveItem",
    "moveRequirement",
    "deleteEntry",
    "toggleCollapsed",
    "addTag",
    "removeTag",
    "renameTag",
    "recolourTag",
    "moveTag",
    "deleteTag",
    "setCategory",
    "clearCategory",
    "addCategory",
    "renameCategory",
    "recolourCategory",
    "moveCategory",
    "deleteCategory",
    "setState",
    "clearState",
    "addState",
    "renameState",
    "recolourState",
    "moveState",
    "deleteState",
    "addAssessment",
    "changeAssessment",
    "resolveAssessment",
    "removeAssessment",
    "storeFilter",
    "renameFilter",
    "removeFilter",
    "addColumnWidth",
    "setColumnWidth",
    "resetColumnWidth",
];
