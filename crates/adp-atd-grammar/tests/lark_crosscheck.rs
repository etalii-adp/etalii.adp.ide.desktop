//! The tables this crate builds from the vendored grammar equal the tables Lark builds, as
//! `atd-grammar.py --tables` wrote them into `atd.tables.json`, byte for byte (spec 035, T034; ATD §4.7).

use std::path::PathBuf;

/// The `specifications/atd/` to read: `ATD_SPEC_DIR` when set (an etalii.adp branch being built together with this
/// one), else the vendored copy.
fn spec_dir() -> PathBuf {
    match std::env::var_os("ATD_SPEC_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/specifications/atd"),
    }
}

#[test]
fn the_tables_of_the_language_equal_larks() {
    let dir = spec_dir();
    let built = adp_atd_grammar::build_dir(&dir.join("grammar"), "atd", "description").expect("the grammar builds");
    let ours = adp_atd_grammar::tables::to_json(&built.grammar, &built.tables, "specifications/atd/grammar/atd.lark");
    let larks = std::fs::read_to_string(dir.join("grammar/atd.tables.json")).expect("atd.tables.json is there");
    if ours != larks {
        let line = ours.lines().zip(larks.lines()).position(|(a, b)| a != b).unwrap_or(0);
        panic!(
            "the tables differ from Lark's at line {}:\n ours:  {}\n Lark's: {}",
            line + 1,
            ours.lines().nth(line).unwrap_or("(none)"),
            larks.lines().nth(line).unwrap_or("(none)")
        );
    }
}

/// The tables of a binding grammar, against Lark's in `grammar/tables/`, for the grammar family (FBL 0.9 section 4.9,
/// ATD §11.9; spec 035, T147).
fn assert_equal_to_larks(ours: &str, name: &str) {
    let larks = std::fs::read_to_string(spec_dir().join("grammar/tables").join(name))
        .unwrap_or_else(|e| panic!("{name} is there: {e}"));
    if ours != larks {
        let line = ours.lines().zip(larks.lines()).position(|(a, b)| a != b).unwrap_or(0);
        panic!(
            "{name}: the tables differ from Lark's at line {}:\n ours:  {}\n Lark's: {}",
            line + 1,
            ours.lines().nth(line).unwrap_or("(none)"),
            larks.lines().nth(line).unwrap_or("(none)")
        );
    }
}

#[test]
fn the_tables_of_every_binding_grammar_equal_larks() {
    let dir = spec_dir();
    // FBL's example names its grammar by file, beside it.
    let fbl = dir.join("../fbl");
    let load = |module: &str| std::fs::read_to_string(fbl.join(format!("{module}.lark"))).ok();
    let built =
        adp_atd_grammar::build_with_options("w3c-turtle", "turtle_doc", &load, adp_atd_grammar::Options::default())
            .expect("FBL's Turtle grammar builds");
    let ours = adp_atd_grammar::tables::to_json(&built.grammar, &built.tables, "specifications/fbl/w3c-turtle.lark");
    assert_equal_to_larks(&ours, "w3c-turtle-grammar.turtle.tables.json");

    // ATD's example carries its grammar inline, in the binding's `grammar { … }`, whose closing brace stands alone at
    // the block's indentation.
    let atd = std::fs::read_to_string(dir.join("binding-grammar.atd")).expect("binding-grammar.atd is there");
    let open = atd.find("    grammar {\n").expect("the binding has an inline grammar") + "    grammar {\n".len();
    let close = open + atd[open..].find("\n    }\n").expect("the grammar block closes");
    let built = adp_atd_grammar::build_single(&adp_atd_grammar::dedent(&atd[open..close]), "turtle_doc")
        .expect("the inline grammar builds");
    let ours = adp_atd_grammar::tables::to_json(
        &built.grammar,
        &built.tables,
        "specifications/atd/binding-grammar.atd#turtle",
    );
    assert_equal_to_larks(&ours, "binding-grammar.turtle.tables.json");
}
