//! ATD grammar notation and the tables built from it (etalii.adp spec 035, research R9; ATD §3.6, §4).
//!
//! Reads the modules of a grammar in ATD grammar notation, composes them as Lark composes them, expands them into
//! rules, builds the LALR(1) automaton by DeRemer–Pennello's lookaheads with every conflict reported in Lark's terms,
//! and writes the normalized tables of ATD §4.7, which equal Lark's for the same grammar. The contextual lexer of ATD
//! §3.6 lexes over the same terminals. Used at build time for the language (by `adp-atd-syntax`'s `build.rs`) and at
//! run time for the grammars bindings carry.

pub mod collisions;
pub mod grammar;
pub mod lalr;
pub mod lexer;
pub mod notation;
pub mod tables;

use std::path::Path;

pub use grammar::{Grammar, Rule, Terminal, TerminalKind};
pub use lalr::{Action, Conflict, State, Tables};
pub use lexer::{Lexeme, Lexer};

/// Why a grammar does not build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError {
    Compose(grammar::ComposeError),
    Conflicts(Vec<Conflict>),
    Lexer(String),
    Collisions(Vec<collisions::Collision>),
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

/// A grammar with its tables and lexer.
#[derive(Debug, Clone)]
pub struct Built {
    pub grammar: Grammar,
    pub tables: Tables,
    pub lexer: Lexer,
}

/// Builds the grammar whose modules are the `.lark` files of `dir`, from the root module `root`.
pub fn build_dir(dir: &Path, root: &str, start: &str) -> Result<Built, BuildError> {
    let load = |module: &str| std::fs::read_to_string(dir.join(format!("{module}.lark"))).ok();
    build_with(root, start, &load)
}

/// Builds a grammar whose modules `load` reads by name.
pub fn build_with(root: &str, start: &str, load: &dyn Fn(&str) -> Option<String>) -> Result<Built, BuildError> {
    let grammar = grammar::compose(root, start, load).map_err(BuildError::Compose)?;
    finish(grammar)
}

/// Builds a grammar of one module, as a binding carries one (ATD §11.9).
pub fn build_single(text: &str, start: &str) -> Result<Built, BuildError> {
    let module = notation::read_module(text).map_err(|e| {
        BuildError::Compose(grammar::ComposeError {
            module: "grammar".into(),
            line: e.line,
            message: e.message,
        })
    })?;
    let grammar = grammar::from_module(&module, start).map_err(|message| {
        BuildError::Compose(grammar::ComposeError {
            module: "grammar".into(),
            line: 0,
            message,
        })
    })?;
    finish(grammar)
}

fn finish(grammar: Grammar) -> Result<Built, BuildError> {
    let tables = lalr::build(&grammar).map_err(BuildError::Conflicts)?;
    let found = collisions::collisions(&grammar, &tables);
    if !found.is_empty() {
        return Err(BuildError::Collisions(found));
    }
    let lexer = Lexer::new(&grammar).map_err(BuildError::Lexer)?;
    Ok(Built { grammar, tables, lexer })
}
