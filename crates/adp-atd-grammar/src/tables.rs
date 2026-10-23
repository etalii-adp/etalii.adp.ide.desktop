//! The normalized parse tables of ATD §4.7 (`atd.schema.json#/$defs/ParseTables`), written byte for byte as
//! `atd-grammar.py --tables` writes Lark's, so that the two can be compared.

use crate::grammar::{Grammar, TerminalKind};
use crate::lalr::{Action, Tables};

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

/// The tables as JSON, in the layout `atd-grammar.py --tables` writes.
pub fn to_json(grammar: &Grammar, tables: &Tables, grammar_path: &str) -> String {
    let mut lines = vec![
        "{".to_owned(),
        "  \"$schema\": \"https://etalii.net/adp/atd/schema/0.1/atd.schema.json#/$defs/ParseTables\",".to_owned(),
        format!("  \"grammar\": {},", json_string(grammar_path)),
        format!("  \"start\": {},", json_string(&grammar.start)),
        format!(
            "  \"ignore\": [{}],",
            grammar.ignore.iter().map(|i| json_string(i)).collect::<Vec<_>>().join(", ")
        ),
        "  \"terminals\": [".to_owned(),
    ];
    let terminals: Vec<String> = grammar
        .terminals
        .iter()
        .map(|t| {
            format!(
                "    {{\"name\": {}, \"kind\": {}, \"value\": {}, \"priority\": {}}}",
                json_string(&t.name),
                json_string(match t.kind {
                    TerminalKind::String => "string",
                    TerminalKind::Pattern => "pattern",
                }),
                json_string(&t.value),
                t.priority
            )
        })
        .collect();
    lines.push(terminals.join(",\n"));
    lines.push("  ],".to_owned());
    lines.push("  \"rules\": [".to_owned());
    let rules: Vec<String> = grammar
        .rules
        .iter()
        .map(|r| {
            let rhs: Vec<String> = r.rhs.iter().map(|s| json_string(s)).collect();
            let mut text = format!("    {{\"lhs\": {}, \"rhs\": [{}]", json_string(&r.lhs), rhs.join(", "));
            if let Some(alias) = &r.alias {
                text.push_str(&format!(", \"alias\": {}", json_string(alias)));
            }
            text.push('}');
            text
        })
        .collect();
    lines.push(rules.join(",\n"));
    lines.push("  ],".to_owned());
    lines.push("  \"states\": [".to_owned());
    let states: Vec<String> = tables
        .states
        .iter()
        .map(|state| {
            let actions: Vec<String> = state
                .actions
                .iter()
                .map(|(t, a)| {
                    let action = match a {
                        Action::Shift(target) => format!("s{target}"),
                        Action::Reduce(rule) => format!("r{rule}"),
                        Action::Accept => "accept".to_owned(),
                    };
                    format!("{}:{}", json_string(t), json_string(&action))
                })
                .collect();
            let gotos: Vec<String> = state
                .gotos
                .iter()
                .map(|(n, target)| format!("{}:{target}", json_string(n)))
                .collect();
            format!("    {{\"actions\":{{{}}},\"gotos\":{{{}}}}}", actions.join(","), gotos.join(","))
        })
        .collect();
    lines.push(states.join(",\n"));
    lines.push("  ]".to_owned());
    lines.push("}".to_owned());
    lines.join("\n") + "\n"
}
