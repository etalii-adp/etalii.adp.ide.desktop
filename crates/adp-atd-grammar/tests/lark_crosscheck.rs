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
