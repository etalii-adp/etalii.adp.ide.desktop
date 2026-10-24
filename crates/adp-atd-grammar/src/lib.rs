//! ATD grammar notation and the tables built from it (etalii.adp spec 035, research R9 as amended on 2026-10-24; ATD
//! §3.6, §4).
//!
//! Reads the modules of a grammar in ATD grammar notation and refuses what the subset of §4.1 does not admit; then
//! hands them to `etalii-lark`, the Rust port of Lark, in `parser=lalr`, `lexer=contextual`, `strict` mode, so that
//! the LALR(1) tables, the conflicts and the terminal collisions are Lark's own verdict. The tables are printed in the
//! normalized form of ATD §4.7 (`lark_tables`), which equals what `atd-grammar.py --tables` prints from Python Lark.
//! Used for the language (by `adp-atd-syntax`) and at run time for the grammars bindings carry (ATD §11.9).

pub mod lark_tables;
pub mod lexer;
pub mod notation;
pub mod tables;

use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::sync::Arc;

use etalii_lark::{Error as LarkError, GrammarLoader, Lark, LexerKind, ParserKind};

pub use lexer::{Lexeme, Lexer, TerminalKind};

/// Why a grammar does not build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError {
    /// The notation is not ATD's, or Lark refuses the grammar for another reason than a conflict or a collision.
    Compose(ComposeError),
    Conflicts(Vec<Conflict>),
    Lexer(String),
    Collisions(Vec<Collision>),
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildError::Compose(error) => write!(f, "{error}"),
            BuildError::Conflicts(conflicts) => {
                for conflict in conflicts {
                    writeln!(f, "{conflict}")?;
                }
                Ok(())
            }
            BuildError::Lexer(message) => write!(f, "{message}"),
            BuildError::Collisions(found) => {
                for collision in found {
                    writeln!(f, "{collision}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for BuildError {}

/// Why a grammar cannot be read: the module, the line (0 when the whole grammar is meant) and the reason.
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

/// A shift/reduce or reduce/reduce conflict, as Lark reports it, its terminals named as the normalized tables name
/// them (§4.7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    /// `Shift/Reduce` or `Reduce/Reduce`, as Lark says it.
    pub kind: &'static str,
    pub terminal: String,
    /// The rules involved, as `lhs : rhs`.
    pub rules: Vec<String>,
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

/// Two terminals one parser state accepts that match the same text, with an example of it (Lark's strict mode).
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

/// The grammar as the tables name it: its start rule and the terminals its modules name.
#[derive(Debug, Clone)]
pub struct Grammar {
    pub start: String,
    /// The terminals a module defines by name; Lark's other terminals are the anonymous strings, named in the tables
    /// by their quoted value.
    pub named: BTreeSet<String>,
}

impl Grammar {
    /// Whether Lark's terminal `name` is one a module names (`cel__INT` is `INT` of the module `cel`).
    pub fn names(&self, name: &str) -> bool {
        let bare = name.rsplit("__").next().unwrap_or(name).trim_start_matches('_');
        self.named.contains(bare)
    }
}

/// Lark's automaton of the grammar.
#[derive(Debug, Clone)]
pub struct Tables {
    pub lark: Arc<Lark>,
}

/// A grammar built by Lark: the parser, its tables and its lexer.
#[derive(Debug, Clone)]
pub struct Built {
    pub lark: Arc<Lark>,
    pub grammar: Grammar,
    pub tables: Tables,
    pub lexer: Lexer,
}

/// How Lark builds a grammar's parser; the tables and the lexer are the same either way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// Lark's strict mode: a shift/reduce conflict and a terminal collision refuse the grammar (ATD §4.7). The checks
    /// take most of the build's time, so a parser of a grammar already checked, as ATD's own is by its tests and
    /// etalii.adp's grammar job, may leave them out: its tables are the same.
    pub strict: bool,
    /// Every token in the tree, the anonymous strings too, as a lossless tree needs (Lark's `keep_all_tokens`).
    pub keep_all_tokens: bool,
    /// A placeholder for an absent `[optional]` (Lark's `maybe_placeholders`).
    pub maybe_placeholders: bool,
}

impl Default for Options {
    /// ATD's rule, strict, with Lark's defaults for the trees.
    fn default() -> Self {
        Options {
            strict: true,
            keep_all_tokens: false,
            maybe_placeholders: true,
        }
    }
}

/// Builds the grammar whose modules are the `.lark` files of `dir`, from the root module `root`.
pub fn build_dir(dir: &Path, root: &str, start: &str) -> Result<Built, BuildError> {
    let load = |module: &str| std::fs::read_to_string(dir.join(format!("{module}.lark"))).ok();
    build_with(root, start, &load)
}

/// Builds a grammar whose modules `load` reads by name.
pub fn build_with(root: &str, start: &str, load: &dyn Fn(&str) -> Option<String>) -> Result<Built, BuildError> {
    build_with_options(root, start, load, Options::default())
}

/// Builds a grammar whose modules `load` reads by name, its trees built as `options` say.
pub fn build_with_options(
    root: &str,
    start: &str,
    load: &dyn Fn(&str) -> Option<String>,
    options: Options,
) -> Result<Built, BuildError> {
    let root_text = load(root).ok_or_else(|| {
        BuildError::Compose(ComposeError {
            module: root.into(),
            line: 0,
            message: "the module is not there".into(),
        })
    })?;
    let root_module = read(root, &root_text)?;
    let mut modules = HashMap::from([(root.to_owned(), root_text.clone())]);
    let mut named = terminals_of(&root_module);
    for import in &root_module.imports {
        if modules.contains_key(&import.module) {
            continue;
        }
        let text = load(&import.module).ok_or_else(|| {
            BuildError::Compose(ComposeError {
                module: root.into(),
                line: import.line,
                message: format!("the module `{}` is not there", import.module),
            })
        })?;
        let module = read(&import.module, &text)?;
        if !module.imports.is_empty() {
            return Err(BuildError::Compose(ComposeError {
                module: import.module.clone(),
                line: module.imports[0].line,
                message: "a module other than the root imports nothing; the root's import list names what it shares"
                    .into(),
            }));
        }
        named.extend(terminals_of(&module));
        modules.insert(import.module.clone(), text);
    }
    finish(root, &root_text, start, named, Some(Modules(modules)), options)
}

/// Builds a grammar of one module, as a binding carries one (ATD §11.9).
pub fn build_single(text: &str, start: &str) -> Result<Built, BuildError> {
    let module = read("grammar", text)?;
    finish("grammar", text, start, terminals_of(&module), None, Options::default())
}

/// The module `name`, read by ATD grammar notation, which refuses what Lark admits and ATD does not.
fn read(name: &str, text: &str) -> Result<notation::Module, BuildError> {
    notation::read_module(text).map_err(|e| {
        BuildError::Compose(ComposeError {
            module: name.into(),
            line: e.line,
            message: e.message,
        })
    })
}

fn terminals_of(module: &notation::Module) -> BTreeSet<String> {
    module
        .definitions
        .iter()
        .filter(|d| d.is_terminal())
        .map(|d| d.name.trim_start_matches('_').to_owned())
        .collect()
}

/// The modules of a grammar, held in memory, as Lark's imports load them.
#[derive(Clone)]
struct Modules(HashMap<String, String>);

impl GrammarLoader for Modules {
    fn load(&self, _base_path: Option<&str>, grammar_path: &str) -> Option<(String, String)> {
        let name = grammar_path.rsplit(['/', '\\']).next()?.trim_end_matches(".lark");
        self.0.get(name).map(|text| (format!("{name}.lark"), text.clone()))
    }
}

fn finish(
    root: &str,
    text: &str,
    start: &str,
    named: BTreeSet<String>,
    modules: Option<Modules>,
    options: Options,
) -> Result<Built, BuildError> {
    let builder = |parser: ParserKind| {
        let mut builder = Lark::builder(text)
            .parser(parser)
            .start(start)
            .regex_flags("a")
            .propagate_positions(true)
            .keep_all_tokens(options.keep_all_tokens)
            .maybe_placeholders(options.maybe_placeholders)
            .source_path(format!("{root}.lark"));
        if parser == ParserKind::Lalr {
            builder = builder.lexer(LexerKind::Contextual).strict(options.strict);
        }
        if let Some(modules) = &modules {
            builder = builder.import_path(modules.clone());
        }
        builder.build()
    };
    let grammar = Grammar {
        start: start.to_owned(),
        named,
    };
    match builder(ParserKind::Lalr) {
        Ok(lark) => {
            let lark = Arc::new(lark);
            let lexer = Lexer::of(&lark, &grammar);
            Ok(Built {
                tables: Tables {
                    lark: Arc::clone(&lark),
                },
                lark,
                grammar,
                lexer,
            })
        }
        Err(error) => {
            // Lark's messages name its terminals; the names of §4.7 are found in the grammar as Earley reads it, which
            // builds no table and so has no conflict.
            let terminals: HashMap<String, String> = builder(ParserKind::Earley)
                .map(|earley| {
                    earley
                        .terminals()
                        .iter()
                        .map(|t| (t.name.clone(), lexer::table_name(t, &grammar)))
                        .collect()
                })
                .unwrap_or_default();
            Err(refusal(root, error, &terminals))
        }
    }
}

/// Lark's refusal as this crate reports it.
fn refusal(root: &str, error: LarkError, terminals: &HashMap<String, String>) -> BuildError {
    let name = |n: &str| terminals.get(n).cloned().unwrap_or_else(|| n.to_owned());
    let rule = |text: &str| -> String {
        let inner = text.trim().trim_start_matches('<').trim_end_matches('>');
        let (lhs, rhs) = inner.split_once(" : ").unwrap_or((inner, ""));
        let rhs: Vec<String> = rhs.split_whitespace().map(name).collect();
        format!("{lhs} : {}", rhs.join(" "))
    };
    match error {
        LarkError::Grammar(message) if message.starts_with("Shift/Reduce conflict for terminal ") => {
            let rest = &message["Shift/Reduce conflict for terminal ".len()..];
            let terminal = rest.split(['.', ' ']).next().unwrap_or_default();
            let rules = rest
                .lines()
                .filter_map(|l| l.trim_start().strip_prefix("* "))
                .map(rule)
                .collect();
            BuildError::Conflicts(vec![Conflict {
                kind: "Shift/Reduce",
                terminal: name(terminal),
                rules,
            }])
        }
        LarkError::Grammar(message) if message.starts_with("Reduce/Reduce collision in ") => {
            let terminal = message
                .split_once("Terminal('")
                .and_then(|(_, rest)| rest.split_once("')"))
                .map(|(t, _)| t)
                .unwrap_or_default();
            let rules = message
                .lines()
                .filter_map(|l| l.trim_start().strip_prefix("- "))
                .map(rule)
                .collect();
            BuildError::Conflicts(vec![Conflict {
                kind: "Reduce/Reduce",
                terminal: name(terminal),
                rules,
            }])
        }
        LarkError::Lex(message) if message.starts_with("Collision between Terminals ") => {
            let rest = &message["Collision between Terminals ".len()..];
            let (pair, example) = rest.split_once(". ").unwrap_or((rest, ""));
            let (first, second) = pair.split_once(" and ").unwrap_or((pair, ""));
            let example = example
                .split_once(": ")
                .map(|(_, e)| e.trim().trim_matches('\'').to_owned())
                .unwrap_or_default();
            BuildError::Collisions(vec![Collision {
                first: name(first),
                second: name(second),
                example,
            }])
        }
        LarkError::Lex(message) => BuildError::Lexer(message),
        other => BuildError::Compose(ComposeError {
            module: root.into(),
            line: 0,
            message: other.to_string(),
        }),
    }
}
