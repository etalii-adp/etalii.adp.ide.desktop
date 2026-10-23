//! A grammar in rules: the modules composed as ATD §4.3 says, expanded into rules as §4.4 says, and named as the
//! normalized tables of §4.7 name them, so that the rules, and the automaton built from them, are Lark's.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::notation::{self, Alternative, Definition, Item, Module, NotationError};

/// A terminal: by its name, or, when it is an anonymous string, by its quoted string (§4.7).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Terminal {
    pub name: String,
    pub kind: TerminalKind,
    pub value: String,
    pub priority: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TerminalKind {
    String,
    Pattern,
}

/// A rule after the expansion of §4.4.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rule {
    pub lhs: String,
    pub rhs: Vec<String>,
    pub alias: Option<String>,
}

/// A grammar ready to be built: its terminals, its rules and its start symbol.
#[derive(Debug, Clone)]
pub struct Grammar {
    /// Sorted by name.
    pub terminals: Vec<Terminal>,
    /// The ignored terminals, by name, sorted.
    pub ignore: Vec<String>,
    /// Sorted, without duplicates: the rule numbers of the tables.
    pub rules: Vec<Rule>,
    pub start: String,
    /// The definitions as written, by their composed name: what the tree is shaped by.
    pub inline: BTreeSet<String>,
}

impl Grammar {
    pub fn is_terminal(&self, symbol: &str) -> bool {
        self.terminals.binary_search_by(|t| t.name.as_str().cmp(symbol)).is_ok()
    }

    pub fn terminal(&self, symbol: &str) -> Option<&Terminal> {
        self.terminals
            .binary_search_by(|t| t.name.as_str().cmp(symbol))
            .ok()
            .map(|index| &self.terminals[index])
    }
}

/// Why a grammar cannot be composed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposeError {
    pub module: String,
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for ComposeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.lark:{}: {}", self.module, self.line, self.message)
    }
}

impl std::error::Error for ComposeError {}

fn compose_error(module: &str, error: NotationError) -> ComposeError {
    ComposeError {
        module: module.to_owned(),
        line: error.line,
        message: error.message,
    }
}

/// Composes the grammar whose root module is `root` (`atd` for ATD's grammar), reading each module's text with
/// `load`, and expands it for the start symbol `start`.
pub fn compose(
    root: &str,
    start: &str,
    load: &dyn Fn(&str) -> Option<String>,
) -> Result<Grammar, ComposeError> {
    let root_text = load(root).ok_or_else(|| ComposeError {
        module: root.into(),
        line: 0,
        message: "the module is not there".into(),
    })?;
    let root_module = notation::read_module(&root_text).map_err(|e| compose_error(root, e))?;
    // Every definition of the composed grammar, by its composed name.
    let mut definitions: BTreeMap<String, Definition> = BTreeMap::new();
    let mut seen_modules: Vec<String> = Vec::new();
    let mut aliases_by_module: Vec<(String, Vec<String>, usize)> = Vec::new();
    for import in &root_module.imports {
        if let Some(entry) = aliases_by_module.iter_mut().find(|(m, _, _)| *m == import.module) {
            entry.1.extend(import.names.iter().cloned());
        } else {
            aliases_by_module.push((import.module.clone(), import.names.clone(), import.line));
        }
    }
    for (module_name, aliases, line) in &aliases_by_module {
        let text = load(module_name).ok_or_else(|| ComposeError {
            module: root.into(),
            line: *line,
            message: format!("the module `{module_name}` is not there"),
        })?;
        let module = notation::read_module(&text).map_err(|e| compose_error(module_name, e))?;
        if !module.imports.is_empty() {
            return Err(ComposeError {
                module: module_name.clone(),
                line: module.imports[0].line,
                message: "a module other than the root imports nothing; the root's import list names what it shares".into(),
            });
        }
        let aliases: HashSet<&str> = aliases.iter().map(String::as_str).collect();
        let mangle = |name: &str| -> String {
            if aliases.contains(name) {
                name.to_owned()
            } else if let Some(rest) = name.strip_prefix('_') {
                format!("_{module_name}__{rest}")
            } else {
                format!("{module_name}__{name}")
            }
        };
        let own: HashMap<&str, &Definition> = module.definitions.iter().map(|d| (d.name.as_str(), d)).collect();
        // Lark keeps only what the imported names reach within the module.
        let mut reached: BTreeSet<String> = BTreeSet::new();
        let mut todo: Vec<String> = aliases.iter().filter(|a| own.contains_key(*a)).map(|a| (*a).to_owned()).collect();
        while let Some(name) = todo.pop() {
            if !reached.insert(name.clone()) {
                continue;
            }
            if let Some(definition) = own.get(name.as_str()) {
                for symbol in symbols_of(&definition.alternatives) {
                    if own.contains_key(symbol.as_str()) && !reached.contains(&symbol) {
                        todo.push(symbol);
                    }
                }
            }
        }
        for name in &reached {
            let definition = own[name.as_str()];
            let composed = mangle(name);
            if definitions.contains_key(&composed) {
                return Err(ComposeError {
                    module: module_name.clone(),
                    line: definition.line,
                    message: format!("`{composed}` is defined by two modules"),
                });
            }
            let renamed = rename(definition, &mangle);
            definitions.insert(composed.clone(), Definition { name: composed, ..renamed });
        }
        seen_modules.push(module_name.clone());
    }
    for definition in &root_module.definitions {
        if definitions.contains_key(&definition.name) {
            return Err(ComposeError {
                module: root.into(),
                line: definition.line,
                message: format!("`{}` is defined by the root and by a module", definition.name),
            });
        }
        definitions.insert(definition.name.clone(), definition.clone());
    }
    expand(&definitions, &root_module, start).map_err(|message| ComposeError {
        module: root.into(),
        line: 0,
        message,
    })
}

/// A single module as a grammar of its own, as a binding's grammar is (ATD §11.9).
pub fn from_module(module: &Module, start: &str) -> Result<Grammar, String> {
    let definitions: BTreeMap<String, Definition> =
        module.definitions.iter().map(|d| (d.name.clone(), d.clone())).collect();
    expand(&definitions, module, start)
}

fn symbols_of(alternatives: &[Alternative]) -> Vec<String> {
    let mut out = Vec::new();
    for alternative in alternatives {
        for item in &alternative.items {
            collect_symbols(item, &mut out);
        }
    }
    out
}

fn collect_symbols(item: &Item, out: &mut Vec<String>) {
    match item {
        Item::Symbol(name) => out.push(name.clone()),
        Item::Str(_) | Item::Re(_) => {}
        Item::Group(inner) | Item::Maybe(inner) => out.extend(symbols_of(inner)),
        Item::Optional(inner) | Item::Star(inner) | Item::Plus(inner) => collect_symbols(inner, out),
    }
}

fn rename(definition: &Definition, mangle: &dyn Fn(&str) -> String) -> Definition {
    Definition {
        name: definition.name.clone(),
        inline: definition.inline,
        priority: definition.priority,
        alternatives: rename_alternatives(&definition.alternatives, mangle),
        line: definition.line,
    }
}

fn rename_alternatives(alternatives: &[Alternative], mangle: &dyn Fn(&str) -> String) -> Vec<Alternative> {
    alternatives
        .iter()
        .map(|a| Alternative {
            items: a.items.iter().map(|i| rename_item(i, mangle)).collect(),
            alias: a.alias.as_deref().map(mangle),
        })
        .collect()
}

fn rename_item(item: &Item, mangle: &dyn Fn(&str) -> String) -> Item {
    match item {
        Item::Symbol(name) => Item::Symbol(mangle(name)),
        Item::Str(_) | Item::Re(_) => item.clone(),
        Item::Group(inner) => Item::Group(rename_alternatives(inner, mangle)),
        Item::Maybe(inner) => Item::Maybe(rename_alternatives(inner, mangle)),
        Item::Optional(inner) => Item::Optional(Box::new(rename_item(inner, mangle))),
        Item::Star(inner) => Item::Star(Box::new(rename_item(inner, mangle))),
        Item::Plus(inner) => Item::Plus(Box::new(rename_item(inner, mangle))),
    }
}

/// The expansion of §4.4, from the start symbol: rules reachable from it, terminals they use and the ignored ones.
fn expand(definitions: &BTreeMap<String, Definition>, root: &Module, start: &str) -> Result<Grammar, String> {
    // Terminals by name, their pattern from their definition.
    let mut named_terminals: BTreeMap<String, Terminal> = BTreeMap::new();
    for definition in definitions.values().filter(|d| d.is_terminal()) {
        let (kind, value) = terminal_pattern(&definition.alternatives, definitions)
            .ok_or_else(|| format!("the terminal `{}` is neither a string nor a regular expression", definition.name))?;
        named_terminals.insert(
            definition.name.clone(),
            Terminal {
                name: definition.name.clone(),
                kind,
                value,
                priority: definition.priority,
            },
        );
    }
    for definition in definitions.values() {
        let filtered = definition.name.starts_with('_');
        if filtered && !definition.is_terminal() && definition.alternatives.iter().any(|a| a.alias.is_some()) {
            return Err(format!(
                "the rule `{}` is filtered from the tree (it starts with `_`) and so has no alias",
                definition.name
            ));
        }
    }
    let mut expander = Expander {
        definitions,
        named_terminals: &named_terminals,
        anonymous: BTreeMap::new(),
        rules: BTreeSet::new(),
        helpers: BTreeSet::new(),
    };
    let mut done: BTreeSet<String> = BTreeSet::new();
    let mut todo = vec![start.to_owned()];
    if !definitions.contains_key(start) {
        return Err(format!("the start symbol `{start}` is not defined"));
    }
    while let Some(name) = todo.pop() {
        if !done.insert(name.clone()) {
            continue;
        }
        let definition = definitions
            .get(&name)
            .ok_or_else(|| format!("`{name}` is used but not defined"))?;
        let mut referenced = Vec::new();
        for alternative in &definition.alternatives {
            for sequence in expander.sequence(&alternative.items, &mut referenced)? {
                expander.rules.insert(Rule {
                    lhs: name.clone(),
                    rhs: sequence,
                    alias: alternative.alias.clone(),
                });
            }
        }
        for symbol in referenced {
            if !done.contains(&symbol) {
                todo.push(symbol);
            }
        }
    }
    let rules: Vec<Rule> = expander.rules.into_iter().collect();
    let mut used: BTreeSet<String> = BTreeSet::new();
    for rule in &rules {
        for symbol in &rule.rhs {
            used.insert(symbol.clone());
        }
    }
    let mut terminals: Vec<Terminal> = Vec::new();
    for terminal in named_terminals.values() {
        if used.contains(&terminal.name) || root.ignores.contains(&terminal.name) {
            terminals.push(terminal.clone());
        }
    }
    terminals.extend(expander.anonymous.into_values());
    terminals.sort();
    let mut ignore = root.ignores.clone();
    ignore.sort();
    let inline = definitions.values().filter(|d| d.inline).map(|d| d.name.clone()).collect();
    Ok(Grammar {
        terminals,
        ignore,
        rules,
        start: start.to_owned(),
        inline,
    })
}

/// A terminal's pattern: one string, or one regular expression, or a choice and sequence of them joined into one
/// regular expression as Lark joins them.
fn terminal_pattern(alternatives: &[Alternative], definitions: &BTreeMap<String, Definition>) -> Option<(TerminalKind, String)> {
    if let [Alternative { items, .. }] = alternatives
        && let [item] = items.as_slice()
    {
        match item {
            Item::Str(value) => return Some((TerminalKind::String, value.clone())),
            Item::Re(value) => return Some((TerminalKind::Pattern, value.clone())),
            Item::Symbol(name) => {
                return definitions
                    .get(name)
                    .and_then(|d| terminal_pattern(&d.alternatives, definitions));
            }
            _ => {}
        }
    }
    let mut choices = Vec::new();
    for alternative in alternatives {
        let mut sequence = String::new();
        for item in &alternative.items {
            sequence.push_str(&item_regex(item, definitions)?);
        }
        choices.push(sequence);
    }
    Some((TerminalKind::Pattern, format!("(?:{})", choices.join("|"))))
}

fn item_regex(item: &Item, definitions: &BTreeMap<String, Definition>) -> Option<String> {
    Some(match item {
        Item::Str(value) => regex_escape(value),
        Item::Re(value) => format!("(?:{value})"),
        Item::Symbol(name) => {
            let (kind, value) = terminal_pattern(&definitions.get(name)?.alternatives, definitions)?;
            if kind == TerminalKind::String {
                regex_escape(&value)
            } else {
                format!("(?:{value})")
            }
        }
        Item::Group(inner) => terminal_pattern(inner, definitions)?.1,
        Item::Maybe(inner) => format!("(?:{})?", terminal_pattern(inner, definitions)?.1),
        Item::Optional(inner) => format!("(?:{})?", item_regex(inner, definitions)?),
        Item::Star(inner) => format!("(?:{})*", item_regex(inner, definitions)?),
        Item::Plus(inner) => format!("(?:{})+", item_regex(inner, definitions)?),
    })
}

fn regex_escape(value: &str) -> String {
    let mut out = String::new();
    for c in value.chars() {
        if "\\.+*?()|[]{}^$#&-~/".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

struct Expander<'a> {
    definitions: &'a BTreeMap<String, Definition>,
    named_terminals: &'a BTreeMap<String, Terminal>,
    anonymous: BTreeMap<String, Terminal>,
    rules: BTreeSet<Rule>,
    helpers: BTreeSet<String>,
}

impl Expander<'_> {
    /// The sequences a list of items expands into: §4.4 steps 1 to 4.
    fn sequence(&mut self, items: &[Item], referenced: &mut Vec<String>) -> Result<Vec<Vec<String>>, String> {
        let mut sequences: Vec<Vec<String>> = vec![Vec::new()];
        for item in items {
            let choices = self.item(item, referenced)?;
            let mut next = Vec::new();
            for prefix in &sequences {
                for choice in &choices {
                    let mut joined = prefix.clone();
                    joined.extend(choice.iter().cloned());
                    next.push(joined);
                }
            }
            sequences = next;
        }
        let mut unique = Vec::new();
        for sequence in sequences {
            if !unique.contains(&sequence) {
                unique.push(sequence);
            }
        }
        Ok(unique)
    }

    fn alternatives(&mut self, alternatives: &[Alternative], referenced: &mut Vec<String>) -> Result<Vec<Vec<String>>, String> {
        let mut out = Vec::new();
        for alternative in alternatives {
            for sequence in self.sequence(&alternative.items, referenced)? {
                if !out.contains(&sequence) {
                    out.push(sequence);
                }
            }
        }
        Ok(out)
    }

    fn item(&mut self, item: &Item, referenced: &mut Vec<String>) -> Result<Vec<Vec<String>>, String> {
        Ok(match item {
            Item::Symbol(name) => {
                if notation::is_terminal_name(name) {
                    if !self.named_terminals.contains_key(name) {
                        return Err(format!("the terminal `{name}` is used but not defined"));
                    }
                } else {
                    if !self.definitions.contains_key(name) {
                        return Err(format!("the rule `{name}` is used but not defined"));
                    }
                    referenced.push(name.clone());
                }
                vec![vec![name.clone()]]
            }
            Item::Str(value) => {
                let name = quoted(value);
                self.anonymous.entry(name.clone()).or_insert(Terminal {
                    name: name.clone(),
                    kind: TerminalKind::String,
                    value: value.clone(),
                    priority: 0,
                });
                vec![vec![name]]
            }
            Item::Re(value) => {
                let name = format!("/{value}/");
                self.anonymous.entry(name.clone()).or_insert(Terminal {
                    name: name.clone(),
                    kind: TerminalKind::Pattern,
                    value: value.clone(),
                    priority: 0,
                });
                vec![vec![name]]
            }
            Item::Group(inner) => self.alternatives(inner, referenced)?,
            Item::Maybe(inner) => {
                let mut out = self.alternatives(inner, referenced)?;
                out.push(Vec::new());
                out
            }
            Item::Optional(inner) => {
                let mut out = self.item(inner, referenced)?;
                out.push(Vec::new());
                out
            }
            Item::Plus(inner) => vec![vec![self.helper(inner, referenced)?]],
            Item::Star(inner) => vec![vec![self.helper(inner, referenced)?], Vec::new()],
        })
    }

    /// The rule `H: x | H x` of a repetition, named by its alternatives (§4.4 step 3, §4.7).
    fn helper(&mut self, inner: &Item, referenced: &mut Vec<String>) -> Result<String, String> {
        let alternatives = self.item(inner, referenced)?;
        let mut printed: Vec<String> = alternatives.iter().map(|a| a.join(" ")).collect();
        printed.sort();
        let name = format!("({})+", printed.join(" | "));
        if self.helpers.insert(name.clone()) {
            for alternative in alternatives {
                let mut recursive = vec![name.clone()];
                recursive.extend(alternative.iter().cloned());
                self.rules.insert(Rule {
                    lhs: name.clone(),
                    rhs: alternative,
                    alias: None,
                });
                self.rules.insert(Rule {
                    lhs: name.clone(),
                    rhs: recursive,
                    alias: None,
                });
            }
        }
        Ok(name)
    }
}

/// An anonymous string terminal's name: the string as JSON writes it (§4.7).
pub fn quoted(value: &str) -> String {
    crate::tables::json_string(value)
}
