//! The terminals of a grammar and Lark's lexer over them (ATD §3.6, as amended on 2026-11-01): among the candidates a
//! parser state reads and the ignored terminals, a string that a pattern of its priority matches whole is that
//! pattern's keyword; the rest are tried by priority, then by the longest text they can match, and the first that
//! matches wins, retyped to a keyword its text is. `etalii-lark`'s `BasicLexer` decides it, as Lark's does.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use etalii_lark::lexer::{BasicLexer, LexerConf, LexerState, LineCounter};
use etalii_lark::{Lark, TerminalDef};

use crate::Grammar;
use crate::tables::json_string;

/// Whether a terminal is a string or a pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TerminalKind {
    String,
    Pattern,
}

/// A terminal, named as the normalized tables name it (§4.7).
#[derive(Debug, Clone)]
pub struct Matcher {
    pub name: String,
    pub kind: TerminalKind,
    pub priority: i64,
    pub ignored: bool,
    definition: TerminalDef,
}

impl Matcher {
    /// The terminal as Lark defines it.
    pub fn definition(&self) -> &TerminalDef {
        &self.definition
    }
}

/// The terminals of a grammar, sorted by name.
#[derive(Clone)]
pub struct Lexer {
    pub matchers: Vec<Matcher>,
    /// Lark's lexer over each set of candidates asked for, built once: building one compiles its terminals.
    lexers: Arc<Mutex<HashMap<Vec<usize>, Arc<BasicLexer>>>>,
}

/// A token: its terminal (an index into [`Lexer::matchers`]) and its byte range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lexeme {
    pub terminal: usize,
    pub start: usize,
    pub end: usize,
}

/// The name of Lark's terminal `t` in the normalized tables: an anonymous string by its quoted value.
pub fn table_name(t: &TerminalDef, grammar: &Grammar) -> String {
    if t.pattern.is_str() && !grammar.names(&t.name) {
        json_string(t.pattern.value())
    } else {
        t.name.clone()
    }
}

impl std::fmt::Debug for Lexer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Lexer")
            .field("matchers", &self.matchers)
            .finish_non_exhaustive()
    }
}

impl Lexer {
    pub(crate) fn of(lark: &Lark, grammar: &Grammar) -> Lexer {
        let ignored = lark.ignore_tokens();
        let mut matchers: Vec<Matcher> = lark
            .terminals()
            .iter()
            .map(|t| Matcher {
                name: table_name(t, grammar),
                kind: if t.pattern.is_str() {
                    TerminalKind::String
                } else {
                    TerminalKind::Pattern
                },
                priority: t.priority,
                ignored: ignored.contains(&t.name),
                definition: t.clone(),
            })
            .collect();
        matchers.sort_by(|a, b| a.name.cmp(&b.name));
        Lexer {
            matchers,
            lexers: Arc::default(),
        }
    }

    /// Lark's lexer over `candidates` and the ignored terminals.
    fn lexer(&self, candidates: &[usize]) -> Option<Arc<BasicLexer>> {
        let mut key: Vec<usize> = candidates.to_vec();
        key.sort_unstable();
        key.dedup();
        let mut lexers = self.lexers.lock().ok()?;
        if let Some(lexer) = lexers.get(&key) {
            return Some(Arc::clone(lexer));
        }
        let terminals: Vec<TerminalDef> = self
            .matchers
            .iter()
            .enumerate()
            .filter(|(i, m)| m.ignored || key.binary_search(i).is_ok())
            .map(|(_, m)| m.definition.clone())
            .collect();
        let lexer = Arc::new(BasicLexer::new(&LexerConf::new(terminals, Vec::new())).ok()?);
        lexers.insert(key, Arc::clone(&lexer));
        Some(lexer)
    }

    pub fn index(&self, name: &str) -> Option<usize> {
        self.matchers.iter().position(|m| m.name == name)
    }

    /// The token at `position` among the `candidates` (indexes of terminals) and the ignored terminals, as Lark's
    /// lexer reads it. `None` when nothing matches.
    pub fn next(&self, text: &str, position: usize, candidates: &[usize]) -> Option<Lexeme> {
        let lexer = self.lexer(candidates)?;
        // Started at `position` by its byte offset alone: `LexerState::new` counts the lines of all the text before
        // it, which made reading a file's trivia token by token quadratic in its length. Lines and columns are not
        // read here, and ATD's terminals have no anchors (ATD §3), so what matches is the same.
        let mut state = LexerState {
            text,
            end: text.len(),
            line_ctr: LineCounter {
                pos: position,
                ..LineCounter::default()
            },
            last_token: None,
        };
        let token = lexer.next_token(&mut state, None).ok()??;
        let terminal = self.matchers.iter().position(|m| m.definition.name == token.kind)?;
        Some(Lexeme {
            terminal,
            start: token.start_pos.unwrap_or(position),
            end: token.end_pos.unwrap_or(position),
        })
    }
}
