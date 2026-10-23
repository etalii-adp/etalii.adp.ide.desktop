//! Terminal collisions (ATD §4.7, Lark's strict mode): two pattern terminals of the same priority that one parser
//! state accepts, ignored terminals included, and that match the same text fully. Each pair is found by walking the
//! product of their DFAs; an example of the text is given.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

use regex_automata::dfa::{Automaton, StartKind, dense};
use regex_automata::util::start;
use regex_automata::{Anchored, MatchKind};

use crate::grammar::{Grammar, TerminalKind};
use crate::lalr::Tables;
use crate::lexer::to_rust;

/// Two terminals that match the same text, with an example of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Collision {
    pub first: String,
    pub second: String,
    pub example: String,
}

impl std::fmt::Display for Collision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Collision between Terminals {} and {}. Example: {:?}",
            self.first, self.second, self.example
        )
    }
}

/// Every collision of the grammar, each pair once.
pub fn collisions(grammar: &Grammar, tables: &Tables) -> Vec<Collision> {
    let patterns: HashMap<&str, (i32, &str)> = grammar
        .terminals
        .iter()
        .filter(|t| t.kind == TerminalKind::Pattern)
        .map(|t| (t.name.as_str(), (t.priority, t.value.as_str())))
        .collect();
    let ignored: Vec<&str> = grammar
        .ignore
        .iter()
        .map(String::as_str)
        .filter(|n| patterns.contains_key(n))
        .collect();
    let mut groups: BTreeSet<Vec<&str>> = BTreeSet::new();
    for state in &tables.states {
        let mut group: Vec<&str> = state
            .actions
            .keys()
            .map(String::as_str)
            .filter(|n| patterns.contains_key(n))
            .collect();
        group.extend(ignored.iter().copied());
        group.sort();
        group.dedup();
        groups.insert(group);
    }
    let mut checked: HashSet<(&str, &str)> = HashSet::new();
    let mut dfas: HashMap<&str, Option<dense::DFA<Vec<u32>>>> = HashMap::new();
    let mut found = Vec::new();
    for group in &groups {
        let mut by_priority: BTreeMap<i32, Vec<&str>> = BTreeMap::new();
        for name in group {
            by_priority.entry(patterns[name].0).or_default().push(name);
        }
        for names in by_priority.values() {
            for (i, a) in names.iter().enumerate() {
                for b in &names[i + 1..] {
                    if !checked.insert((a, b)) {
                        continue;
                    }
                    for name in [*a, *b] {
                        dfas.entry(name).or_insert_with(|| dfa(patterns[name].1));
                    }
                    if let (Some(first), Some(second)) = (&dfas[a], &dfas[b])
                        && let Some(example) = overlap(first, second)
                    {
                        found.push(Collision {
                            first: (*a).to_owned(),
                            second: (*b).to_owned(),
                            example,
                        });
                    }
                }
            }
        }
    }
    found
}

/// The bytes a match of `pattern` can start with, from its automaton; none when it does not compile to one. A lexer
/// tries a pattern only where the text starts with one of them.
pub fn first_bytes(pattern: &str) -> Option<[bool; 256]> {
    let dfa = dfa(pattern)?;
    let start = dfa.start_state(&start::Config::new().anchored(Anchored::Yes)).ok()?;
    let mut bytes = [false; 256];
    for (byte, can) in bytes.iter_mut().enumerate() {
        *can = !dfa.is_dead_state(dfa.next_state(start, byte as u8));
    }
    Some(bytes)
}

fn dfa(pattern: &str) -> Option<dense::DFA<Vec<u32>>> {
    dense::Builder::new()
        .configure(
            dense::Config::new()
                .start_kind(StartKind::Anchored)
                .match_kind(MatchKind::All),
        )
        .build(&format!("^(?:{})$", to_rust(pattern)))
        .ok()
}

/// The shortest text both automata match fully, when there is one.
fn overlap(a: &dense::DFA<Vec<u32>>, b: &dense::DFA<Vec<u32>>) -> Option<String> {
    let config = start::Config::new().anchored(Anchored::Yes);
    let start = (a.start_state(&config).ok()?, b.start_state(&config).ok()?);
    let mut seen = HashSet::from([start]);
    let mut queue = VecDeque::from([(start, Vec::<u8>::new())]);
    while let Some(((sa, sb), text)) = queue.pop_front() {
        if !text.is_empty() {
            let (ea, eb) = (a.next_eoi_state(sa), b.next_eoi_state(sb));
            if a.is_match_state(ea) && b.is_match_state(eb) {
                return Some(String::from_utf8_lossy(&text).into_owned());
            }
        }
        if text.len() > 64 {
            continue;
        }
        for byte in 0..=255u8 {
            let next = (a.next_state(sa, byte), b.next_state(sb, byte));
            if a.is_dead_state(next.0) || b.is_dead_state(next.1) || !seen.insert(next) {
                continue;
            }
            let mut longer = text.clone();
            longer.push(byte);
            queue.push_back((next, longer));
        }
    }
    None
}
