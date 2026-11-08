//! The Knowledge designer's description against its five cases, recorded from the module (etalii.adp spec 076, T075,
//! FR-004 to FR-006): each format's script, the module's refusals and the findings of a file as it is read, each read
//! with the binding that claims its body. A regression or a correction shows here. `ETALII_ADP` runs it on another
//! clone of etalii.adp.

mod listed;

/// The differences of the cases that Peter has ruled on, by case, with his ruling (FR-007). Nothing awaits a ruling:
/// the `findings` case's are listed by name as module behaviour (Peter, 2026-11-07, spec 076's Q8).
const LISTED: &[(&str, &[&str])] = &[(
    "findings",
    &[
        // The findings the host gives and no constraint can (knowledge.md, section 7): the FBL reader's dangling
        // references and unknown keys in the module's wording, an entry without an id, an unknown type, an unresolved
        // target or row, a parent cycle; and the constraints' sentences, which the module words with the raw id a
        // dangling reference holds.
        "read:findings",
        // A cell naming no property: the module names it by the id it stores, `r1/p94`; the description's derived id
        // reads `self.property.id`, which a dangling reference does not have, so the runtime names it by its place.
        "read:reading",
    ],
)];

#[test]
fn the_knowledge_cases_pass_but_for_what_is_listed() {
    let found = listed::differences(&listed::definitions("designers").join("knowledge.atd"), 5);
    listed::assert_listed(&found, LISTED);
}

/// SC-005: a name taken off the list fails the test above, and so does a difference not listed.
#[test]
fn its_list_is_held_both_ways() {
    listed::held_both_ways(LISTED, "cities-yaml");
}
