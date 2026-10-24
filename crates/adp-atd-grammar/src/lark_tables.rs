//! Lark's LALR(1) automaton in the normalized form of ATD §4.7 (`atd.schema.json#/$defs/ParseTables`), written byte
//! for byte as `atd-grammar.py --tables` writes it from Python Lark: anonymous string terminals by their quoted value,
//! the helper rules of repetitions by what they repeat, the states numbered breadth first from the start state.

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

use etalii_lark::grammar::{Rule, Symbol};
use etalii_lark::{LalrAction, Lark};

use crate::tables::json_string;

/// A string as Python's `json.dumps(…)` writes it, non-ASCII escaped.
fn json_ascii(value: &str) -> String {
    let mut out = String::new();
    for c in json_string(value).chars() {
        if c.is_ascii() {
            out.push(c);
        } else {
            let mut units = [0u16; 2];
            for unit in c.encode_utf16(&mut units) {
                out.push_str(&format!("\\u{unit:04x}"));
            }
        }
    }
    out
}

struct Names<'a> {
    lark: &'a Lark,
    anonymous: BTreeSet<String>,
    helpers: std::cell::RefCell<HashMap<String, Option<String>>>,
}

impl Names<'_> {
    fn terminal(&self, name: &str) -> String {
        if name == "$END" {
            return name.to_owned();
        }
        match self.lark.get_terminal(name) {
            Some(t) if self.anonymous.contains(name) => json_ascii(t.pattern.value()),
            _ => name.to_owned(),
        }
    }

    fn helper(&self, origin: &str) -> String {
        if let Some(Some(known)) = self.helpers.borrow().get(origin) {
            return known.clone();
        }
        if !is_helper(origin) {
            return origin.to_owned();
        }
        self.helpers.borrow_mut().insert(origin.to_owned(), None);
        let mut alternatives: Vec<String> = self
            .lark
            .rules()
            .filter(|r| &*r.origin.name == origin && r.expansion.first().is_none_or(|s| &*s.name != origin))
            .map(|r| r.expansion.iter().map(|s| self.symbol(s)).collect::<Vec<_>>().join(" "))
            .collect();
        alternatives.sort();
        let name = format!("({})+", alternatives.join(" | "));
        self.helpers.borrow_mut().insert(origin.to_owned(), Some(name.clone()));
        name
    }

    fn symbol(&self, symbol: &Symbol) -> String {
        if symbol.is_term {
            self.terminal(&symbol.name)
        } else {
            self.helper(&symbol.name)
        }
    }

    /// A key of a state's actions: a terminal's name, or a rule's.
    fn key(&self, symbol: &str) -> String {
        if is_upper(symbol) || symbol.starts_with("$END") || symbol.starts_with("__ANON") {
            self.terminal(symbol)
        } else {
            self.helper(symbol)
        }
    }

    fn rule(&self, rule: &Rule) -> (String, Vec<String>) {
        (
            self.symbol(&rule.origin),
            rule.expansion.iter().map(|s| self.symbol(s)).collect(),
        )
    }
}

/// `__…_star_N` and `__…_plus_N`, the rules Lark makes for a repetition.
fn is_helper(name: &str) -> bool {
    name.starts_with("__")
        && ["_star_", "_plus_"].iter().any(|k| {
            name.rfind(k).is_some_and(|i| {
                !name[i + k.len()..].is_empty() && name[i + k.len()..].bytes().all(|b| b.is_ascii_digit())
            })
        })
}

/// Python's `str.isupper()`: a cased character, and no lower-case one.
fn is_upper(name: &str) -> bool {
    name.chars().any(char::is_uppercase) && !name.chars().any(char::is_lowercase)
}

/// The normalized tables of `lark`, built from start rule `start`; `named` says whether a terminal of Lark's is one the
/// grammar names (the others, its anonymous strings, are named by their value).
pub fn normalized(lark: &Lark, grammar_path: &str, start: &str, named: &dyn Fn(&str) -> bool) -> String {
    let anonymous: BTreeSet<String> = lark
        .terminals()
        .iter()
        .filter(|t| t.pattern.is_str() && !named(&t.name))
        .map(|t| t.name.clone())
        .collect();
    let names = Names {
        lark,
        anonymous,
        helpers: Default::default(),
    };
    let rules: BTreeSet<(String, Vec<String>, String)> = lark
        .rules()
        .map(|r| {
            let (lhs, rhs) = names.rule(r);
            (lhs, rhs, r.alias.clone().unwrap_or_default())
        })
        .collect();
    let rules: Vec<(String, Vec<String>, String)> = rules.into_iter().collect();
    let index: HashMap<(String, Vec<String>), usize> = rules
        .iter()
        .enumerate()
        .map(|(i, (lhs, rhs, _))| ((lhs.clone(), rhs.clone()), i))
        .collect();
    let table = lark.parse_table().expect("a LALR parser");
    let terminal_names: BTreeSet<&str> = lark.terminals().iter().map(|t| t.name.as_str()).collect();
    let is_terminal = |symbol: &str| symbol == "$END" || terminal_names.contains(symbol);

    let first = table.start_states[start];
    let mut order: HashMap<usize, usize> = HashMap::from([(first, 0)]);
    let mut queue = VecDeque::from([first]);
    while let Some(state) = queue.pop_front() {
        let mut targets: Vec<(String, usize)> = table.states[state]
            .iter()
            .filter_map(|(symbol, action)| match action {
                LalrAction::Shift(target) => Some((names.key(symbol), *target)),
                LalrAction::Reduce(_) => None,
            })
            .collect();
        targets.sort();
        for (_, target) in targets {
            if !order.contains_key(&target) {
                order.insert(target, order.len());
                queue.push_back(target);
            }
        }
    }
    let mut numbered: Vec<(usize, usize)> = order.iter().map(|(s, n)| (*s, *n)).collect();
    numbered.sort_by_key(|(_, n)| *n);
    let states: Vec<String> = numbered
        .iter()
        .map(|(state, _)| {
            let mut actions: BTreeMap<String, String> = BTreeMap::new();
            let mut gotos: BTreeMap<String, usize> = BTreeMap::new();
            for (symbol, action) in &table.states[*state] {
                let key = names.key(symbol);
                match action {
                    LalrAction::Shift(target) if is_terminal(symbol) => {
                        actions.insert(key, format!("s{}", order[target]));
                    }
                    LalrAction::Shift(target) => {
                        gotos.insert(key, order[target]);
                    }
                    LalrAction::Reduce(rule) => {
                        let (lhs, rhs) = names.rule(rule);
                        let value = if lhs.starts_with("$root") && key == "$END" {
                            "accept".to_owned()
                        } else {
                            format!("r{}", index[&(lhs, rhs)])
                        };
                        actions.insert(key, value);
                    }
                }
            }
            if *state == table.end_states[start] {
                actions.insert("$END".into(), "accept".into());
            }
            let actions: Vec<String> = actions
                .iter()
                .map(|(k, v)| format!("{}:{}", json_string(k), json_string(v)))
                .collect();
            let gotos: Vec<String> = gotos.iter().map(|(k, v)| format!("{}:{v}", json_string(k))).collect();
            format!(
                "    {{\"actions\":{{{}}},\"gotos\":{{{}}}}}",
                actions.join(","),
                gotos.join(",")
            )
        })
        .collect();

    let mut terminals: Vec<(String, String)> = lark
        .terminals()
        .iter()
        .map(|t| {
            let name = names.terminal(&t.name);
            let line = format!(
                "    {{\"name\": {}, \"kind\": {}, \"value\": {}, \"priority\": {}}}",
                json_string(&name),
                json_string(if t.pattern.is_str() { "string" } else { "pattern" }),
                json_string(t.pattern.value()),
                t.priority
            );
            (name, line)
        })
        .collect();
    terminals.sort();
    let mut ignore: Vec<String> = lark.ignore_tokens().to_vec();
    ignore.sort();
    let rule_lines: Vec<String> = rules
        .iter()
        .map(|(lhs, rhs, alias)| {
            let rhs: Vec<String> = rhs.iter().map(|s| json_string(s)).collect();
            let mut line = format!("    {{\"lhs\": {}, \"rhs\": [{}]", json_string(lhs), rhs.join(", "));
            if !alias.is_empty() {
                line.push_str(&format!(", \"alias\": {}", json_string(alias)));
            }
            line + "}"
        })
        .collect();
    let mut lines = vec![
        "{".to_owned(),
        "  \"$schema\": \"https://etalii.net/adp/atd/schema/0.1/atd.schema.json#/$defs/ParseTables\",".to_owned(),
        format!("  \"grammar\": {},", json_string(grammar_path)),
        format!("  \"start\": {},", json_string(start)),
        format!(
            "  \"ignore\": [{}],",
            ignore.iter().map(|i| json_string(i)).collect::<Vec<_>>().join(", ")
        ),
        "  \"terminals\": [".to_owned(),
        terminals.into_iter().map(|(_, l)| l).collect::<Vec<_>>().join(",\n"),
        "  ],".to_owned(),
        "  \"rules\": [".to_owned(),
        rule_lines.join(",\n"),
        "  ],".to_owned(),
        "  \"states\": [".to_owned(),
        states.join(",\n"),
        "  ]".to_owned(),
        "}".to_owned(),
    ];
    lines.push(String::new());
    lines.join("\n")
}
