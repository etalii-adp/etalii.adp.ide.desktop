//! What is wrong with a query file (etalii.adp spec 030, task T084): facts about the text, and nothing else. It never
//! executes the query, never contacts an endpoint, and never resolves a `SERVICE` clause.
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/sparql/backend/EtAlii.Adp.Diagram.Sparql/
//! SparqlValidator.cs`.

use adp_diagram::{DiagramProblem, DiagramProblemLocation, DiagramProblemSeverity};

use crate::model::SparqlQueryModel;
use crate::parser;

/// Reported when the file is not a SPARQL query this can read, SPARQL Update included.
pub const UNPARSEABLE_RULE_ID: &str = "sparql.unparseable";
/// Reported when a projected variable never appears in the where clause.
pub const UNBOUND_PROJECTION_RULE_ID: &str = "sparql.unbound-projection";
/// Reported when a prefix is declared but never used.
pub const UNUSED_PREFIX_RULE_ID: &str = "sparql.unused-prefix";

/// The findings of the query `text` (`SparqlValidator.ValidateAsync`): one error when it does not parse, else the
/// findings over what it says.
pub fn validate(text: &str) -> Vec<DiagramProblem> {
    match parser::parse(text) {
        // One finding, not a list of consequences: running the rules over an empty model would bury the reason.
        Err(error) => vec![DiagramProblem {
            severity: DiagramProblemSeverity::Error,
            message: format!("This is not a SPARQL query that can be read: {}", error.message),
            rule_id: UNPARSEABLE_RULE_ID.to_owned(),
            location: Some(DiagramProblemLocation::Line(
                u32::try_from(error.line.max(1)).unwrap_or(u32::MAX),
            )),
        }],
        Ok(model) => judge(&model, text),
    }
}

/// The findings over a parsed query.
pub fn judge(model: &SparqlQueryModel, text: &str) -> Vec<DiagramProblem> {
    let mut problems = Vec::new();
    // A projected variable the pattern never binds yields a silently empty column.
    for item in &model.projection {
        let Some(usage) = model.variable(&item.name) else {
            continue;
        };
        if usage.pattern_occurrences == 0 && usage.defining_expression.is_empty() {
            problems.push(DiagramProblem {
                severity: DiagramProblemSeverity::Warning,
                message: format!(
                    "?{} is selected but never appears in the where clause, so its column will always be unbound.",
                    item.name
                ),
                rule_id: UNBOUND_PROJECTION_RULE_ID.to_owned(),
                location: Some(DiagramProblemLocation::Line(1)),
            });
        }
    }
    // An unused prefix declaration is harmless but misleading: info, at its own line.
    for prefix in &model.prefixes {
        if !is_used(&prefix.prefix, text) {
            problems.push(DiagramProblem {
                severity: DiagramProblemSeverity::Info,
                message: format!("The prefix '{}:' is declared but never used.", prefix.prefix),
                rule_id: UNUSED_PREFIX_RULE_ID.to_owned(),
                location: Some(DiagramProblemLocation::Line(
                    u32::try_from(prefix.line.max(1)).unwrap_or(u32::MAX),
                )),
            });
        }
    }
    problems
}

/// Whether the query uses `prefix` anywhere outside its own declarations, asked of the text, since a prefixed name can
/// stand in an expression or a solution modifier.
fn is_used(prefix: &str, text: &str) -> bool {
    let needle = format!("{prefix}:");
    let mut from = 0;
    while let Some(found) = text[from..].find(&needle) {
        let at = from + found;
        from = at + needle.len();
        // The declaration itself is not a use: skip an occurrence whose line starts with PREFIX before it.
        let line_start = text[..at].rfind('\n').map_or(0, |n| n + 1);
        let before = text[line_start..at].trim_start();
        if before.get(..6).is_some_and(|word| word.eq_ignore_ascii_case("PREFIX")) {
            continue;
        }
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_declaration_is_not_a_use() {
        assert!(!is_used("ex", "PREFIX ex: <x>\nSELECT * WHERE { ?s ?p ?o }"));
        assert!(is_used("ex", "PREFIX ex: <x>\nSELECT * WHERE { ?s ex:p ?o }"));
        assert!(is_used("", "PREFIX : <x>\nSELECT * WHERE { ?s :p ?o }"));
    }
}
