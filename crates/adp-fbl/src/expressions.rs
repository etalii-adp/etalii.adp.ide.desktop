//! FBL's presets of the CEL engine (FBL section 2.4; the standalone host's `Expressions/FblCel` and `CelContext`):
//! the standard environment, which FBL shares with DISL, with the variables each context offers and nothing else.

use std::sync::OnceLock;

use adp_cel::{Environment, Program};

/// Where a CEL expression appears, which decides the variables it may use (FBL section 2.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CelContext {
    /// A rule of an xml binding: entry, parent, path, line, registration.
    Tree,
    /// A rule of a lines or blocks binding: entry, parent, groups, line, registration.
    Lines,
    /// `insert.when`: attributes.
    Insert,
    /// A rule of a yaml or json binding (FBL 0.5): entry, text, style, parent, path, line, registration.
    Scalars,
}

fn environment(context: CelContext) -> &'static Environment {
    static TREE: OnceLock<Environment> = OnceLock::new();
    static LINES: OnceLock<Environment> = OnceLock::new();
    static INSERT: OnceLock<Environment> = OnceLock::new();
    static SCALARS: OnceLock<Environment> = OnceLock::new();
    match context {
        CelContext::Tree => {
            TREE.get_or_init(|| Environment::standard(&["entry", "parent", "path", "line", "registration"]))
        }
        CelContext::Lines => {
            LINES.get_or_init(|| Environment::standard(&["entry", "parent", "groups", "line", "registration"]))
        }
        CelContext::Insert => INSERT.get_or_init(|| Environment::standard(&["attributes"])),
        CelContext::Scalars => SCALARS.get_or_init(|| {
            Environment::standard(&["entry", "text", "style", "parent", "path", "line", "registration"])
        }),
    }
}

/// Compiles `expression` for `context`; the error names what it cannot use.
pub fn compile(expression: &str, context: CelContext) -> Result<Program, String> {
    environment(context).compile(expression)
}
