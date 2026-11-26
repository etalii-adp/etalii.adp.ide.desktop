//! The functional decomposition graph description against its case, recorded from the module (etalii.adp spec 035,
//! T132): the elements drawn at the sizes their attributes keep, a new element written with its type (FBL 0.8
//! `constant`), and a connect gesture known by its wire id, apart from the differences listed here by name.
//! `ETALII_ADP` runs it on another clone.

mod listed;

/// The differences of the case, by case.
const LISTED: &[(&str, &[&str])] = &[(
    "field-service",
    &[
        // Step 6 moves Task detail by a distance and step 7 Step row; the module adds the first move's distance to the
        // second (Step row goes to 750, 434 for a move of 20, 10 from 660, 400), so the bytes and the drawing differ
        // from step 7 on. And at steps 11, 13, 14 and 15 the case records the recorder's note that the standalone host
        // generated no id, not a refusal of the tool's: the description makes two of those connections and refuses
        // the other two in its own words.
        "10:bytes",
        "10:drawing",
        "11:bytes",
        "11:drawing",
        "11:refused",
        "12:bytes",
        "12:drawing",
        "13:bytes",
        "13:drawing",
        "13:refused",
        "14:bytes",
        "14:drawing",
        "14:refused",
        "15:bytes",
        "15:drawing",
        "15:refused",
        "16:bytes",
        "16:drawing",
        "17:bytes",
        "17:drawing",
        "18:bytes",
        "18:drawing",
        "19:bytes",
        "19:drawing",
        "20:bytes",
        "20:drawing",
        "21:bytes",
        "21:drawing",
        "22:bytes",
        "22:drawing",
        "23:bytes",
        "23:drawing",
        "24:bytes",
        "24:drawing",
        "25:bytes",
        "25:drawing",
        "26:bytes",
        "26:drawing",
        "27:bytes",
        "27:drawing",
        "28:bytes",
        "28:drawing",
        "29:bytes",
        "29:drawing",
        "30:bytes",
        "30:drawing",
        "31:bytes",
        "31:drawing",
        "7:bytes",
        "7:drawing",
        "8:bytes",
        "8:drawing",
        "9:bytes",
        "9:drawing",
    ],
)];

#[test]
fn the_functional_decomposition_case_passes_but_for_what_is_listed() {
    let found = listed::differences(
        &listed::definitions("diagrams").join("functional-decomposition-graph.atd"),
        1,
    );
    listed::assert_listed(&found, LISTED);
}

/// SC-005: a name taken off the list fails the test above, and so does a difference not listed.
#[test]
fn its_list_is_held_both_ways() {
    listed::held_both_ways(LISTED, "field-service");
}
