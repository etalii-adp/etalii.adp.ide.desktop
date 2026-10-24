//! The normalized parse tables of ATD §4.7 (`atd.schema.json#/$defs/ParseTables`), written byte for byte as
//! `atd-grammar.py --tables` writes Lark's, so that the two can be compared.

use crate::{Grammar, Tables, lark_tables};

/// A string as Python's `json.dumps(…, ensure_ascii=False)` writes it.
pub fn json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The tables as JSON, in the layout `atd-grammar.py --tables` writes: Lark's automaton, normalized (§4.7).
pub fn to_json(grammar: &Grammar, tables: &Tables, grammar_path: &str) -> String {
    lark_tables::normalized(&tables.lark, grammar_path, &grammar.start, &|name| grammar.names(name))
}
