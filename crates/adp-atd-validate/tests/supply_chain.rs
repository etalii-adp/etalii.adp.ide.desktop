//! The supply chain description against its case, recorded from the module (etalii.adp spec 035, T132): the cards laid
//! out by `layout.bandedLayers` and the groups drawn around them, a key set later written right after the entry's id
//! (FBL 0.24 `newKeys`), and a stage the module records by its label compared as its key (ATD §2.3), apart from the
//! differences listed here by name. `ETALII_ADP` runs it on another clone.

mod listed;

/// The differences of the case, by case.
const LISTED: &[(&str, &[&str])] = &[(
    "automotive",
    &[
        // The case records the standalone host's missing id as a refusal, where the description refuses a second flow
        // between the same two cards in its own words: the sentence is the recorder's note, not a refusal of the tool.
        "17:refused",
        // Dragging a card into another group's frame moves it into that group (supply-chain.md, finding A13): no hook
        // can state it, since a hook sees no frame of another element (the description's gap 8). The card keeps its
        // group, and the steps after it differ in its group's line and in where the bands are drawn, until the undo
        // and redo that end the case.
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
    ],
)];

#[test]
fn the_supply_chain_case_passes_but_for_what_is_listed() {
    let found = listed::differences(&listed::definitions("diagrams").join("supply-chain.atd"), 1);
    listed::assert_listed(&found, LISTED);
}

/// SC-005: a name taken off the list fails the test above, and so does a difference not listed.
#[test]
fn its_list_is_held_both_ways() {
    listed::held_both_ways(LISTED, "automotive");
}
