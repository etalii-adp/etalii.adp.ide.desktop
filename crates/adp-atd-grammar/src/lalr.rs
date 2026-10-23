//! The LALR(1) automaton of a grammar (ATD §4.7): the LR(0) states, the lookaheads by propagation (the result
//! DeRemer and Pennello's relations compute, as Lark does), the actions and gotos, and every conflict, named as Lark
//! names it.

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

use crate::grammar::Grammar;

/// The end of the input.
pub const END: &str = "$END";

/// An action on a terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Shift(usize),
    /// By the index of a rule in [`Grammar::rules`].
    Reduce(usize),
    Accept,
}

/// One state: its actions by terminal and its gotos by nonterminal.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    pub actions: BTreeMap<String, Action>,
    pub gotos: BTreeMap<String, usize>,
    /// The items of the state's kernel, as `lhs: a b * c`, for reports.
    pub kernel: Vec<String>,
}

/// The automaton, its states numbered breadth first from the start state over the symbols in sorted order (§4.7).
#[derive(Debug, Clone)]
pub struct Tables {
    pub states: Vec<State>,
}

/// A shift/reduce or reduce/reduce conflict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    /// `Shift/Reduce` or `Reduce/Reduce`, as Lark says it.
    pub kind: &'static str,
    pub terminal: String,
    /// The rules involved, as `lhs: rhs`.
    pub rules: Vec<String>,
    /// The items of the state's kernel.
    pub items: Vec<String>,
}

impl std::fmt::Display for Conflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} conflict for terminal {}", self.kind, self.terminal)?;
        for rule in &self.rules {
            write!(f, "\n * <{rule}>")?;
        }
        Ok(())
    }
}

/// Symbols by number: the terminals first, `$END` last of them, then the nonterminals, the augmented start last.
struct Symbols {
    names: Vec<String>,
    index: HashMap<String, usize>,
    terminals: usize,
}

impl Symbols {
    fn is_terminal(&self, symbol: usize) -> bool {
        symbol < self.terminals
    }
}

type Item = (usize, usize);

struct Builder<'g> {
    grammar: &'g Grammar,
    symbols: Symbols,
    /// The rules as symbol numbers; the last one is the augmented `$root: start`.
    rules: Vec<(usize, Vec<usize>)>,
    by_lhs: Vec<Vec<usize>>,
    nullable: Vec<bool>,
    first: Vec<BitSet>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct BitSet(Vec<u64>);

impl BitSet {
    fn new(size: usize) -> Self {
        BitSet(vec![0; size.div_ceil(64)])
    }
    fn insert(&mut self, bit: usize) -> bool {
        let (word, mask) = (bit / 64, 1u64 << (bit % 64));
        let fresh = self.0[word] & mask == 0;
        self.0[word] |= mask;
        fresh
    }
    fn union(&mut self, other: &BitSet) -> bool {
        let mut changed = false;
        for (a, b) in self.0.iter_mut().zip(&other.0) {
            let merged = *a | *b;
            changed |= merged != *a;
            *a = merged;
        }
        changed
    }
    fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        self.0.iter().enumerate().flat_map(|(w, bits)| {
            (0..64)
                .filter(move |b| bits & (1u64 << b) != 0)
                .map(move |b| w * 64 + b)
        })
    }
}

/// Builds the automaton. Every conflict is returned; the tables are only returned when there is none.
pub fn build(grammar: &Grammar) -> Result<Tables, Vec<Conflict>> {
    let mut builder = Builder::new(grammar);
    builder.analyse();
    builder.automaton()
}

impl<'g> Builder<'g> {
    fn new(grammar: &'g Grammar) -> Self {
        let mut names: Vec<String> = grammar.terminals.iter().map(|t| t.name.clone()).collect();
        names.push(END.to_owned());
        let terminals = names.len();
        let mut nonterminals: BTreeSet<String> = BTreeSet::new();
        for rule in &grammar.rules {
            nonterminals.insert(rule.lhs.clone());
        }
        names.extend(nonterminals);
        names.push("$root".to_owned());
        let index: HashMap<String, usize> = names.iter().enumerate().map(|(i, n)| (n.clone(), i)).collect();
        let mut rules: Vec<(usize, Vec<usize>)> = grammar
            .rules
            .iter()
            .map(|r| (index[&r.lhs], r.rhs.iter().map(|s| index[s]).collect()))
            .collect();
        let root = index["$root"];
        rules.push((root, vec![index[&grammar.start]]));
        let mut by_lhs = vec![Vec::new(); names.len()];
        for (i, (lhs, _)) in rules.iter().enumerate() {
            by_lhs[*lhs].push(i);
        }
        let count = names.len();
        Builder {
            grammar,
            symbols: Symbols {
                names,
                index,
                terminals,
            },
            rules,
            by_lhs,
            nullable: vec![false; count],
            first: vec![BitSet::new(terminals); count],
        }
    }

    /// The nullable symbols and the FIRST sets.
    fn analyse(&mut self) {
        for t in 0..self.symbols.terminals {
            self.first[t].insert(t);
        }
        let mut changed = true;
        while changed {
            changed = false;
            for (lhs, rhs) in &self.rules {
                if !self.nullable[*lhs] && rhs.iter().all(|s| self.nullable[*s]) {
                    self.nullable[*lhs] = true;
                    changed = true;
                }
                for symbol in rhs {
                    let first = self.first[*symbol].clone();
                    changed |= self.first[*lhs].union(&first);
                    if !self.nullable[*symbol] {
                        break;
                    }
                }
            }
        }
    }

    fn closure(&self, kernel: &[Item]) -> Vec<Item> {
        let mut items: Vec<Item> = kernel.to_vec();
        let mut seen: BTreeSet<Item> = kernel.iter().copied().collect();
        let mut added_for: Vec<bool> = vec![false; self.symbols.names.len()];
        let mut i = 0;
        while i < items.len() {
            let (rule, dot) = items[i];
            if let Some(&symbol) = self.rules[rule].1.get(dot)
                && !self.symbols.is_terminal(symbol)
                && !added_for[symbol]
            {
                added_for[symbol] = true;
                for &r in &self.by_lhs[symbol] {
                    if seen.insert((r, 0)) {
                        items.push((r, 0));
                    }
                }
            }
            i += 1;
        }
        items
    }

    /// The LR(1) closure of one kernel item with the lookahead `#` (numbered `terminals`, one past `$END`'s bits),
    /// for the propagation of lookaheads.
    fn closure1(&self, item: Item, hash: usize) -> Vec<(Item, BitSet)> {
        let width = self.symbols.terminals + 1;
        let mut lookaheads: BTreeMap<Item, BitSet> = BTreeMap::new();
        let mut start = BitSet::new(width);
        start.insert(hash);
        lookaheads.insert(item, start);
        let mut todo: VecDeque<Item> = VecDeque::from([item]);
        while let Some(current) = todo.pop_front() {
            let (rule, dot) = current;
            let rhs = &self.rules[rule].1;
            let Some(&symbol) = rhs.get(dot) else { continue };
            if self.symbols.is_terminal(symbol) {
                continue;
            }
            // FIRST(β L) for the item's lookaheads L.
            let mut follow = BitSet::new(width);
            let mut rest_nullable = true;
            for &next in &rhs[dot + 1..] {
                for t in self.first[next].iter() {
                    follow.insert(t);
                }
                if !self.nullable[next] {
                    rest_nullable = false;
                    break;
                }
            }
            if rest_nullable {
                let own = lookaheads[&current].clone();
                follow.union(&own);
            }
            for &r in &self.by_lhs[symbol] {
                let entry = lookaheads.entry((r, 0)).or_insert_with(|| BitSet::new(width));
                if entry.union(&follow) {
                    todo.push_back((r, 0));
                }
            }
        }
        lookaheads.into_iter().collect()
    }

    fn automaton(&self) -> Result<Tables, Vec<Conflict>> {
        let root_rule = self.rules.len() - 1;
        // LR(0) states by kernel.
        let mut kernels: Vec<Vec<Item>> = vec![vec![(root_rule, 0)]];
        let mut by_kernel: HashMap<Vec<Item>, usize> = HashMap::from([(vec![(root_rule, 0)], 0)]);
        let mut transitions: Vec<BTreeMap<usize, usize>> = Vec::new();
        let mut index = 0;
        while index < kernels.len() {
            let closure = self.closure(&kernels[index]);
            let mut next: BTreeMap<usize, Vec<Item>> = BTreeMap::new();
            for (rule, dot) in closure {
                if let Some(&symbol) = self.rules[rule].1.get(dot) {
                    next.entry(symbol).or_default().push((rule, dot + 1));
                }
            }
            let mut moves = BTreeMap::new();
            for (symbol, mut kernel) in next {
                kernel.sort();
                kernel.dedup();
                let target = *by_kernel.entry(kernel.clone()).or_insert_with(|| {
                    kernels.push(kernel);
                    kernels.len() - 1
                });
                moves.insert(symbol, target);
            }
            transitions.push(moves);
            index += 1;
        }
        // Lookaheads of kernel items: spontaneous, and propagated.
        let width = self.symbols.terminals + 1;
        let hash = self.symbols.terminals;
        let end = self.symbols.index[END];
        let mut lookaheads: Vec<Vec<BitSet>> = kernels.iter().map(|k| vec![BitSet::new(width); k.len()]).collect();
        lookaheads[0][0].insert(end);
        let mut propagate: Vec<Vec<Vec<(usize, usize)>>> = kernels.iter().map(|k| vec![Vec::new(); k.len()]).collect();
        let position = |state: usize, item: Item, kernels: &Vec<Vec<Item>>| -> usize {
            kernels[state].binary_search(&item).expect("a kernel item")
        };
        for (state, kernel) in kernels.iter().enumerate() {
            for (k, &item) in kernel.iter().enumerate() {
                for ((rule, dot), set) in self.closure1(item, hash) {
                    let Some(&symbol) = self.rules[rule].1.get(dot) else {
                        continue;
                    };
                    let target = transitions[state][&symbol];
                    let at = position(target, (rule, dot + 1), &kernels);
                    for t in set.iter() {
                        if t == hash {
                            propagate[state][k].push((target, at));
                        } else {
                            lookaheads[target][at].insert(t);
                        }
                    }
                }
            }
        }
        let mut changed = true;
        while changed {
            changed = false;
            for state in 0..kernels.len() {
                for k in 0..kernels[state].len() {
                    let set = lookaheads[state][k].clone();
                    for &(target, at) in &propagate[state][k] {
                        changed |= lookaheads[target][at].union(&set);
                    }
                }
            }
        }
        // Actions, gotos and conflicts, by the states' own numbers.
        let mut conflicts = Vec::new();
        let mut raw: Vec<State> = Vec::with_capacity(kernels.len());
        for (state, kernel) in kernels.iter().enumerate() {
            let mut actions: BTreeMap<usize, Action> = BTreeMap::new();
            let mut gotos = BTreeMap::new();
            for (&symbol, &target) in &transitions[state] {
                if self.symbols.is_terminal(symbol) {
                    actions.insert(symbol, Action::Shift(target));
                } else if symbol != self.symbols.index["$root"] {
                    gotos.insert(self.symbols.names[symbol].clone(), target);
                }
            }
            // Completed items: of the kernel, and the empty rules of the closure, with their lookaheads.
            let mut completed: Vec<(usize, BitSet)> = Vec::new();
            for (k, &(rule, dot)) in kernel.iter().enumerate() {
                if dot == self.rules[rule].1.len() {
                    completed.push((rule, lookaheads[state][k].clone()));
                }
                if self.rules[rule]
                    .1
                    .iter()
                    .skip(dot)
                    .any(|s| !self.symbols.is_terminal(*s))
                {
                    for ((r, d), set) in self.closure1((rule, dot), hash) {
                        if d == 0 && self.rules[r].1.is_empty() {
                            let mut resolved = BitSet::new(width);
                            for t in set.iter() {
                                if t == hash {
                                    resolved.union(&lookaheads[state][k]);
                                } else {
                                    resolved.insert(t);
                                }
                            }
                            completed.push((r, resolved));
                        }
                    }
                }
            }
            let mut merged: BTreeMap<usize, BitSet> = BTreeMap::new();
            for (rule, set) in completed {
                merged.entry(rule).or_insert_with(|| BitSet::new(width)).union(&set);
            }
            for (rule, set) in merged {
                for t in set.iter() {
                    let action = if rule == root_rule {
                        Action::Accept
                    } else {
                        Action::Reduce(rule)
                    };
                    match actions.get(&t) {
                        None => {
                            actions.insert(t, action);
                        }
                        Some(Action::Shift(_)) => conflicts.push(self.conflict("Shift/Reduce", t, &[rule], kernel)),
                        Some(Action::Reduce(other)) if *other != rule => {
                            conflicts.push(self.conflict("Reduce/Reduce", t, &[*other, rule], kernel))
                        }
                        _ => {}
                    }
                }
            }
            raw.push(State {
                actions: actions
                    .into_iter()
                    .map(|(t, a)| (self.symbols.names[t].clone(), a))
                    .collect(),
                gotos,
                kernel: kernel.iter().map(|i| self.item_text(*i)).collect(),
            });
        }
        if !conflicts.is_empty() {
            return Err(conflicts);
        }
        Ok(renumber(raw))
    }

    fn conflict(&self, kind: &'static str, terminal: usize, rules: &[usize], kernel: &[Item]) -> Conflict {
        Conflict {
            kind,
            terminal: self.symbols.names[terminal].clone(),
            rules: rules.iter().map(|r| self.rule_text(*r)).collect(),
            items: kernel.iter().map(|i| self.item_text(*i)).collect(),
        }
    }

    fn rule_text(&self, rule: usize) -> String {
        let (lhs, rhs) = &self.rules[rule];
        let names: Vec<&str> = rhs.iter().map(|s| self.symbols.names[*s].as_str()).collect();
        format!("{} : {}", self.symbols.names[*lhs], names.join(" "))
    }

    fn item_text(&self, (rule, dot): Item) -> String {
        let (lhs, rhs) = &self.rules[rule];
        let mut parts: Vec<&str> = rhs.iter().map(|s| self.symbols.names[*s].as_str()).collect();
        parts.insert(dot, "*");
        format!("{} : {}", self.symbols.names[*lhs], parts.join(" "))
    }

    #[allow(dead_code)]
    fn grammar(&self) -> &Grammar {
        self.grammar
    }
}

/// Numbers the states breadth first from the start state, visiting the targets of a state's shifts and gotos in the
/// sorted order of their symbols (§4.7).
fn renumber(raw: Vec<State>) -> Tables {
    let mut order: Vec<usize> = vec![0];
    let mut number: HashMap<usize, usize> = HashMap::from([(0, 0)]);
    let mut queue: VecDeque<usize> = VecDeque::from([0]);
    while let Some(state) = queue.pop_front() {
        let mut targets: Vec<(&String, usize)> = Vec::new();
        for (symbol, action) in &raw[state].actions {
            if let Action::Shift(target) = action {
                targets.push((symbol, *target));
            }
        }
        for (symbol, target) in &raw[state].gotos {
            targets.push((symbol, *target));
        }
        targets.sort();
        for (_, target) in targets {
            if let std::collections::hash_map::Entry::Vacant(entry) = number.entry(target) {
                entry.insert(order.len());
                order.push(target);
                queue.push_back(target);
            }
        }
    }
    let states = order
        .iter()
        .map(|&old| {
            let state = &raw[old];
            State {
                actions: state
                    .actions
                    .iter()
                    .map(|(t, a)| {
                        (
                            t.clone(),
                            match a {
                                Action::Shift(target) => Action::Shift(number[target]),
                                other => *other,
                            },
                        )
                    })
                    .collect(),
                gotos: state
                    .gotos
                    .iter()
                    .map(|(n, target)| (n.clone(), number[target]))
                    .collect(),
                kernel: state.kernel.clone(),
            }
        })
        .collect();
    Tables { states }
}
