//! The surface knows no designer type (etalii.adp spec 036, FR-012): no name of the Hierarchical Requirement
//! Designer's vocabulary appears in its sources.

const SOURCES: [(&str, &str); 3] = [
    ("mod.rs", include_str!("mod.rs")),
    ("model.rs", include_str!("model.rs")),
    ("session.rs", include_str!("session.rs")),
];

/// Words only a designer type has: its element types, its lists, its operations and its numbering.
const DESIGNER_WORDS: [&str; 12] = [
    "Requirement",
    "requirement",
    "ITEM-",
    "REQ-",
    "Assessment",
    "assessment",
    "category",
    "addTopItem",
    "setTitle",
    "deleteEntry",
    "Question",
    "Remark",
];

#[test]
fn no_designer_type_is_named_in_the_surface() {
    let mut found = Vec::new();
    for (file, text) in SOURCES {
        for word in DESIGNER_WORDS {
            if text.contains(word) {
                found.push(format!("{file}: {word}"));
            }
        }
    }
    assert!(found.is_empty(), "the surface names a designer type: {found:?}");
}
