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

/// The terminals the lexer skips between tokens: whitespace and comments.
pub fn ignored() -> &'static [u16] {
    static IGNORED: OnceLock<Vec<u16>> = OnceLock::new();
    IGNORED.get_or_init(|| {
        (0..TERMINALS.len() as u16)
            .filter(|t| TERMINALS[*t as usize].ignored)
            .collect()
    })
}

/// Whether a match of `terminal` can start with `byte`.
fn can_start(terminal: u16, byte: u8) -> bool {
    let info = &TERMINALS[terminal as usize];
    if info.pattern {
        first_bytes()[terminal as usize].is_none_or(|bytes| bytes[byte as usize])
    } else {
        info.value.as_bytes().first() == Some(&byte)
    }
}

/// The terminals the lexer tries in `state` where the text starts with `byte`: those the state reads and the ignored
/// ones, without the end of the file, that can start with it. Each state's lists are made the first time the lexer
/// is in it, so that a token costs the few terminals that can start there rather than every one the state reads.
pub fn candidates(state: usize, byte: u8) -> &'static [u16] {
    static BY_STATE: OnceLock<Vec<OnceLock<Vec<Vec<u16>>>>> = OnceLock::new();
    let states = BY_STATE.get_or_init(|| (0..ACTIONS.len()).map(|_| OnceLock::new()).collect());
    let lists = states[state].get_or_init(|| {
        let all: Vec<u16> = acceptable(state)
            .filter(|t| *t != END)
            .chain(ignored().iter().copied())
            .collect();
        (0..=255u8)
            .map(|byte| all.iter().copied().filter(|t| can_start(*t, byte)).collect())
            .collect()
    });
    &lists[byte as usize]
}

/// The bytes each pattern terminal's match can start with, by index; `None` for a string terminal, or a pattern
/// whose automaton was not built (then every byte).
fn first_bytes() -> &'static [Option<[bool; 256]>] {
    static FIRST: OnceLock<Vec<Option<[bool; 256]>>> = OnceLock::new();
    FIRST.get_or_init(|| {
        TERMINALS
            .iter()
            .map(|t| {
                t.pattern
                    .then(|| adp_atd_grammar::collisions::first_bytes(t.value))
                    .flatten()
            })
            .collect()
    })
}

/// The length of the match of `terminal` at the start of `text`. A pattern is tried only where the text's first byte
/// can start it: most of a state's patterns cannot, and trying none of them is what keeps lexing linear and fast
/// (SC-006).
pub fn match_len(terminal: u16, text: &str) -> Option<usize> {
    let info = &TERMINALS[terminal as usize];
    match &matchers()[terminal as usize] {
        None => {
            // The first byte rules out nearly every string terminal before a comparison is made.
            (text.as_bytes().first() == info.value.as_bytes().first() && text.starts_with(info.value))
                .then_some(info.value.len())
        }
        Some(regex) => {
            let first = *text.as_bytes().first()?;
            if first_bytes()[terminal as usize].is_some_and(|bytes| !bytes[first as usize]) {
                return None;
            }
            regex.find(text).map(|m| m.end()).filter(|len| *len > 0)
        }
    }
}
