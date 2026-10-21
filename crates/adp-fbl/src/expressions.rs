//! FBL's presets of the CEL engine (FBL section 2.4; the standalone host's `Expressions/FblCel` and `CelContext`):
//! the standard environment, which FBL shares with DISL, with the variables each context offers and nothing else.

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

thread_local! {
    // An environment holds its functions behind `Rc`, so each thread keeps its own presets.
    static ENVIRONMENTS: [Environment; 4] = [
        Environment::standard(&["entry", "parent", "path", "line", "registration"]),
        Environment::standard(&["entry", "parent", "groups", "line", "registration"]),
        Environment::standard(&["attributes"]),
        Environment::standard(&["entry", "text", "style", "parent", "path", "line", "registration"]),
    ];
}

/// Compiles `expression` for `context`; the error names what it cannot use.
pub fn compile(expression: &str, context: CelContext) -> Result<Program, String> {
    let index = match context {
        CelContext::Tree => 0,
        CelContext::Lines => 1,
        CelContext::Insert => 2,
        CelContext::Scalars => 3,
    };
    ENVIRONMENTS.with(|environments| environments[index].compile(expression))
}
