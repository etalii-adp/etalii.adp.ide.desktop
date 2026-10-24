//! ATD's grammar, built by `etalii-lark` the first time it is used (etalii.adp spec 035, T165): the parser, its
//! table, and its terminals and tree nodes as the parser's tree names them.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use adp_atd_grammar::{Built, Lexer, Options};
use etalii_lark::Lark;

include!(concat!(env!("OUT_DIR"), "/grammar.rs"));

/// A terminal of the grammar.
#[derive(Debug, Clone, Copy)]
pub struct TerminalInfo {
    /// Its name: a named terminal's, or an anonymous string's quoted (ATD §4.7).
    pub name: &'static str,
    pub pattern: bool,
    pub value: &'static str,
    pub priority: i64,
    pub ignored: bool,
    /// A string terminal that spells a name (ATD §3.6).
    pub keyword: bool,
    /// Its name in Lark's parser.
    pub lark: &'static str,
}

/// A kind of tree node: its name (a rule's alias, else its name) and whether it is a `?rule`'s, which Lark inlines
/// when it has one child.
#[derive(Debug, Clone, Copy)]
pub struct KindInfo {
    pub name: &'static str,
    pub inline: bool,
}

/// The grammar as the parser uses it.
pub struct Language {
    pub lark: Arc<Lark>,
    /// Lark's lexer over the grammar's terminals, for the text between tokens and the text recovery skips.
    pub lexer: Lexer,
    pub terminals: Vec<TerminalInfo>,
    by_lark: HashMap<&'static str, u16>,
    pub kinds: Vec<KindInfo>,
    by_kind: HashMap<&'static str, usize>,
    /// The state the parser starts in.
    pub start_state: usize,
}

fn leak(text: &str) -> &'static str {
    Box::leak(text.to_owned().into_boxed_str())
}

/// The grammar, built once.
pub fn language() -> &'static Language {
    static LANGUAGE: OnceLock<Language> = OnceLock::new();
    LANGUAGE.get_or_init(|| {
        let load = |module: &str| {
            MODULES
                .iter()
                .find(|(m, _)| *m == module)
                .map(|(_, text)| (*text).to_owned())
        };
        // The grammar's strictness is checked where its tables are (adp-atd-grammar's tests and etalii.adp's grammar
        // job); left out here, the parser is built in a tenth of the time, with the same tables.
        let options = Options {
            strict: false,
            keep_all_tokens: true,
            maybe_placeholders: false,
        };
        let Built { lark, lexer, .. } = adp_atd_grammar::build_with_options("atd", "description", &load, options)
            .unwrap_or_else(|e| panic!("ATD's grammar does not build: {e}"));
        let ignored = lark.ignore_tokens();
        let terminals: Vec<TerminalInfo> = lexer
            .matchers
            .iter()
            .map(|m| {
                let value = leak(m.definition().pattern.value());
                TerminalInfo {
                    name: leak(&m.name),
                    pattern: !m.definition().pattern.is_str(),
                    value,
                    priority: m.priority,
                    ignored: ignored.contains(&m.definition().name),
                    keyword: m.definition().pattern.is_str()
                        && value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
                    lark: leak(&m.definition().name),
                }
            })
            .collect();
        let by_lark = terminals.iter().enumerate().map(|(i, t)| (t.lark, i as u16)).collect();
        let mut kinds: Vec<KindInfo> = Vec::new();
        for rule in lark.rules() {
            let (name, inline) = match &rule.alias {
                Some(alias) => (alias.as_str(), false),
                None => (&*rule.origin.name, rule.options.expand1),
            };
            if !kinds.iter().any(|k| k.name == name) {
                kinds.push(KindInfo {
                    name: leak(name),
                    inline,
                });
            }
        }
        let by_kind = kinds.iter().enumerate().map(|(i, k)| (k.name, i)).collect();
        let start_state = lark
            .parse_table()
            .and_then(|t| t.start_states.get("description").copied())
            .expect("a LALR parser starts somewhere");
        Language {
            lark,
            lexer,
            terminals,
            by_lark,
            kinds,
            by_kind,
            start_state,
        }
    })
}

impl Language {
    /// The terminal Lark names `name`.
    pub fn terminal_of_lark(&self, name: &str) -> Option<u16> {
        self.by_lark.get(name).copied()
    }

    /// A terminal by its name in the tables (`NAME`, `"}"`).
    pub fn terminal(&self, name: &str) -> Option<u16> {
        self.terminals.iter().position(|t| t.name == name).map(|i| i as u16)
    }

    /// A kind of tree node by its name.
    pub fn kind(&self, name: &str) -> Option<usize> {
        self.by_kind.get(name).copied()
    }

    /// Whether `state` has an action on Lark's terminal `name`.
    pub fn reads(&self, state: usize, name: &str) -> bool {
        self.lark
            .parse_table()
            .is_some_and(|t| t.states.get(state).is_some_and(|row| row.contains_key(name)))
    }
}

/// The grammar's terminals.
pub fn terminals() -> &'static [TerminalInfo] {
    &language().terminals
}
