//! The tables of ATD's grammar, built by `build.rs`, and the terminals' matchers.

use std::sync::OnceLock;

use regex::Regex;

/// A terminal of the grammar.
#[derive(Debug, Clone, Copy)]
pub struct TerminalInfo {
    /// Its name: a named terminal's, or an anonymous string's quoted (ATD §4.7).
    pub name: &'static str,
    pub pattern: bool,
    pub value: &'static str,
    pub priority: i32,
    pub ignored: bool,
    /// A string terminal that spells a name (ATD §3.6).
    pub keyword: bool,
}

/// A rule: its left-hand side (an index into [`NONTERMINALS`]), its length, the name of the tree node it builds (its
/// alias, else its left-hand side), whether it is a repetition's rule, and whether its node is inlined when it has
/// one child (`?rule`).
#[derive(Debug, Clone, Copy)]
pub struct RuleInfo {
    pub lhs: u16,
    pub len: usize,
    pub name: &'static str,
    pub helper: bool,
    pub inline: bool,
}

include!(concat!(env!("OUT_DIR"), "/tables.rs"));

/// The action of `state` on `terminal`.
pub fn action(state: usize, terminal: u16) -> Option<i32> {
    let row = ACTIONS[state];
    row.binary_search_by_key(&terminal, |(t, _)| *t).ok().map(|i| row[i].1)
}

/// The state `state` goes to on `nonterminal`.
pub fn goto(state: usize, nonterminal: u16) -> Option<usize> {
    let row = GOTOS[state];
    row.binary_search_by_key(&nonterminal, |(n, _)| *n)
        .ok()
        .map(|i| row[i].1 as usize)
}

/// The terminals `state` has an action on.
pub fn acceptable(state: usize) -> impl Iterator<Item = u16> {
    ACTIONS[state].iter().map(|(t, _)| *t)
}

/// A terminal by name.
pub fn terminal(name: &str) -> Option<u16> {
    TERMINALS.iter().position(|t| t.name == name).map(|i| i as u16)
}

/// A nonterminal by name.
pub fn nonterminal(name: &str) -> Option<u16> {
    NONTERMINALS.iter().position(|n| *n == name).map(|i| i as u16)
}

/// The anchored matcher of each pattern terminal, by index; `None` for a string terminal.
pub fn matchers() -> &'static [Option<Regex>] {
    static MATCHERS: OnceLock<Vec<Option<Regex>>> = OnceLock::new();
    MATCHERS.get_or_init(|| {
        TERMINALS
            .iter()
            .map(|t| {
                t.pattern.then(|| {
                    Regex::new(&format!("^(?:{})", adp_atd_grammar::lexer::to_rust(t.value)))
                        .expect("a terminal of ATD's grammar compiles")
                })
            })
            .collect()
    })
}

/// The length of the match of `terminal` at the start of `text`.
pub fn match_len(terminal: u16, text: &str) -> Option<usize> {
    let info = &TERMINALS[terminal as usize];
    match &matchers()[terminal as usize] {
        None => text.starts_with(info.value).then_some(info.value.len()),
        Some(regex) => regex.find(text).map(|m| m.end()).filter(|len| *len > 0),
    }
}
