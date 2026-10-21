//! The ontology reading's findings (etalii.adp spec 030, task T083), ported from the standalone host's
//! `Owl/OwlValidator.cs` at `8b6ccfc`: structural facts about the asserted OWL, no inference, no network. The family's
//! file-level rules are not restated: they run first, through [`crate::validator`], and a body that does not parse
//! gets their finding alone.

use std::collections::HashSet;

use adp_diagram::{DiagramProblem, DiagramProblemLocation, DiagramProblemSeverity};

use crate::lines::LineDocument;
use crate::model::RdfModel;
use crate::owl::hierarchy::{self, Supers};
use crate::owl::index::OwlModelIndex;
use crate::owl::projection;
use crate::parser;
use crate::projection::display;
use crate::validator;
use crate::vocabulary::{owl, rdf};

/// The rule reported when classes assert each other as subclasses in a cycle.
pub const SUBCLASS_CYCLE_RULE_ID: &str = "owl.subclass-cycle";
/// The rule reported for a class expression whose structure is broken.
pub const MALFORMED_EXPRESSION_RULE_ID: &str = "owl.malformed-expression";
/// The rule reported for an IRI used in a property's axiom position but declared nowhere in the file.
pub const UNDECLARED_PROPERTY_RULE_ID: &str = "owl.undeclared-property";
/// The rule reported when a non-deprecated axiom references a deprecated entity.
pub const DEPRECATED_REFERENCE_RULE_ID: &str = "owl.deprecated-reference";
/// The rule reported - once per import - naming what is deliberately not fetched.
pub const IMPORTS_NOT_FETCHED_RULE_ID: &str = "owl.imports-not-fetched";

fn problem(severity: DiagramProblemSeverity, message: String, rule_id: &str, line: usize) -> DiagramProblem {
    DiagramProblem {
        severity,
        message,
        rule_id: rule_id.to_owned(),
        location: Some(DiagramProblemLocation::Line(line as u32 + 1)),
    }
}

/// The findings of the document `text` (`OwlValidator.ValidateAsync`): the family's, then this reading's.
pub fn validate(text: &str) -> Vec<DiagramProblem> {
    let mut problems = validator::validate(text);
    let outcome = parser::outcome(&LineDocument::parse(text));
    if outcome.is_usable() {
        problems.extend(judge(&outcome.model));
    }
    problems
}

/// This reading's findings over a parsed model (`OwlValidator.Judge`).
pub fn judge(model: &RdfModel) -> Vec<DiagramProblem> {
    let mut problems = Vec::new();
    let index = OwlModelIndex::build(model);

    // Subclass cycles: legal OWL, almost always a mistake.
    let mut supers = Supers::new();
    for triple in &model.triples {
        if triple.predicate.iri == owl::SUB_CLASS_OF
            && let (Some(child), Some(parent)) = (triple.subject.as_iri(), triple.object.as_iri())
        {
            supers.entry(child.to_owned()).or_default().push(parent.to_owned());
            supers.entry(parent.to_owned()).or_default();
        }
    }
    for cycle in hierarchy::cycles(&supers) {
        let names: Vec<String> = cycle.iter().map(|iri| display(model, iri)).collect();
        let line = model
            .triples
            .iter()
            .find(|t| {
                t.predicate.iri == owl::SUB_CLASS_OF && t.subject.as_iri().is_some_and(|s| cycle.iter().any(|c| c == s))
            })
            .map_or(0, |t| t.span.start_line);
        problems.push(problem(
            DiagramProblemSeverity::Warning,
            format!(
                "These classes assert each other as subclasses in a cycle: {}. That is legal OWL - it makes them mutually equivalent - and it is almost always a mistake.",
                names.join(", ")
            ),
            SUBCLASS_CYCLE_RULE_ID,
            line,
        ));
    }

    // Malformed class expressions, from the projection's own judgment.
    let graph = projection::project(model, usize::MAX);
    for node in graph.nodes.iter().filter(|n| n.malformed) {
        let line = node.expression_root.as_ref().map_or(0, |root| {
            model
                .triples
                .iter()
                .find(|t| t.subject.as_blank().is_some_and(|b| b.ordinal == root.ordinal))
                .map_or(0, |t| t.span.start_line)
        });
        problems.push(problem(
            DiagramProblemSeverity::Error,
            format!(
                "This class expression is not well-formed: it renders as '{}', with ? standing for what the structure is missing.",
                node.display
            ),
            MALFORMED_EXPRESSION_RULE_ID,
            line,
        ));
    }

    // An IRI used in a property's axiom position but declared nowhere here.
    let declared: HashSet<&String> = index
        .object_properties
        .iter()
        .chain(index.data_properties.iter())
        .chain(index.annotation_properties.iter())
        .collect();
    let mut reported: HashSet<String> = HashSet::new();
    for triple in &model.triples {
        let candidate = match triple.predicate.iri.as_str() {
            owl::DOMAIN | owl::RANGE => triple.subject.as_iri(),
            owl::ON_PROPERTY => triple.object.as_iri(),
            _ => None,
        };
        let Some(candidate) = candidate else {
            continue;
        };
        if owl::is_built_in(candidate)
            || declared.contains(&candidate.to_owned())
            || !reported.insert(candidate.to_owned())
        {
            continue;
        }
        problems.push(problem(
            DiagramProblemSeverity::Info,
            format!(
                "{} is used as a property but declared nowhere in this file - it may live in an import, which is deliberately not fetched.",
                display(model, candidate)
            ),
            UNDECLARED_PROPERTY_RULE_ID,
            triple.span.start_line,
        ));
    }

    // A non-deprecated axiom referencing a deprecated entity.
    let mut deprecated_reported: HashSet<String> = HashSet::new();
    for triple in &model.triples {
        if triple.predicate.iri == owl::DEPRECATED || triple.predicate.iri == rdf::TYPE {
            continue;
        }
        if let Some(target) = triple.object.as_iri()
            && index.deprecated.contains(target)
            && triple.subject.as_iri().is_none_or(|s| !index.deprecated.contains(s))
            && deprecated_reported.insert(target.to_owned())
        {
            problems.push(problem(
                DiagramProblemSeverity::Info,
                format!(
                    "{} is deprecated, and something not deprecated still references it.",
                    display(model, target)
                ),
                DEPRECATED_REFERENCE_RULE_ID,
                triple.span.start_line,
            ));
        }
    }

    // Imports are named, never fetched.
    for triple in model.triples.iter().filter(|t| t.predicate.iri == owl::IMPORTS) {
        if let Some(import) = triple.object.as_iri() {
            problems.push(problem(
                DiagramProblemSeverity::Info,
                format!(
                    "This ontology imports {import}, which is not fetched: the diagram shows what this file asserts, not the import closure a reasoner would load."
                ),
                IMPORTS_NOT_FETCHED_RULE_ID,
                triple.span.start_line,
            ));
        }
    }
    problems
}
