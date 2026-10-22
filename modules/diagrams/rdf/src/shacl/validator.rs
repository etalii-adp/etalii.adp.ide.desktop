//! The shapes reading's findings (etalii.adp spec 030, task T083), ported from the standalone host's
//! `Shacl/ShaclValidator.cs` at `8b6ccfc`: the family's text facts first, composed rather than restated, then what makes
//! a shapes file broken or self-contradictory - and never whether any data conforms, because nothing here runs a shape.
//! A target naming a term the file does not describe is the medium working, and silent; a shape reference leaving the
//! file is an info, because one file cannot tell "missing" from "described next door".

use std::collections::HashSet;

use adp_diagram::{DiagramProblem, DiagramProblemLocation, DiagramProblemSeverity};

use crate::lines::LineDocument;
use crate::model::{RdfModel, RdfTriple};
use crate::parser;
use crate::projection::display;
use crate::shacl::discovery::{self, ShaclShape, key_of};
use crate::validator::{self as family, UNPARSEABLE_RULE_ID};
use crate::vocabulary::shacl;

/// A property shape without exactly one `sh:path`.
pub const PATH_CARDINALITY_RULE_ID: &str = "shacl.path-cardinality";

/// `sh:minCount` above `sh:maxCount`: nothing can conform.
pub const IMPOSSIBLE_COUNTS_RULE_ID: &str = "shacl.impossible-counts";

/// `sh:datatype` and `sh:class` on one shape: no value is both literal and instance.
pub const DATATYPE_AND_CLASS_RULE_ID: &str = "shacl.datatype-and-class";

/// A term in the `sh:` namespace the recommendation does not define.
pub const UNKNOWN_TERM_RULE_ID: &str = "shacl.unknown-term";

/// A `sh:node` or `sh:property` reference leaving this file.
pub const REFERENCE_LEAVES_FILE_RULE_ID: &str = "shacl.reference-leaves-file";

fn problem(
    severity: DiagramProblemSeverity,
    message: String,
    rule_id: &str,
    triple: Option<&RdfTriple>,
) -> DiagramProblem {
    DiagramProblem {
        severity,
        message,
        rule_id: rule_id.to_owned(),
        location: triple.map(|t| DiagramProblemLocation::Line(t.span.start_line as u32 + 1)),
    }
}

fn triple<'a>(model: &'a RdfModel, shape_key: &str, predicate: &str) -> Option<&'a RdfTriple> {
    model
        .triples
        .iter()
        .find(|t| key_of(&t.subject).as_deref() == Some(shape_key) && t.predicate.iri == predicate)
}

fn number(model: &RdfModel, shape_key: &str, predicate: &str) -> Option<i32> {
    triple(model, shape_key, predicate)?
        .object
        .as_literal()?
        .lexical
        .trim()
        .parse()
        .ok()
}

fn display_of(model: &RdfModel, shape: &ShaclShape) -> String {
    match shape.term.as_iri() {
        Some(iri) => display(model, iri),
        None => "An anonymous shape".to_owned(),
    }
}

/// The findings of the shapes document `text` (`ShaclValidator.ValidateAsync`).
pub fn validate(text: &str) -> Vec<DiagramProblem> {
    let mut problems = family::validate(text);
    if problems.iter().any(|p| p.rule_id == UNPARSEABLE_RULE_ID) {
        return problems;
    }
    let model = parser::parse(&LineDocument::parse(text)).unwrap_or_default();
    let shapes = discovery::discover(&model);

    for shape in &shapes {
        let paths: Vec<&RdfTriple> = model
            .triples
            .iter()
            .filter(|t| key_of(&t.subject).as_deref() == Some(shape.key.as_str()) && t.predicate.iri == shacl::PATH)
            .collect();
        let used_as_property = model
            .triples
            .iter()
            .any(|t| t.predicate.iri == shacl::PROPERTY && key_of(&t.object).as_deref() == Some(shape.key.as_str()));
        if used_as_property && paths.len() != 1 {
            let message = if paths.is_empty() {
                format!(
                    "{} is used as a property shape but states no sh:path; SHACL requires exactly one, and the row is drawn with an empty path.",
                    display_of(&model, shape)
                )
            } else {
                format!(
                    "{} states {} sh:path values; SHACL requires exactly one, so which values it constrains is undefined.",
                    display_of(&model, shape),
                    paths.len()
                )
            };
            let at = paths
                .first()
                .copied()
                .or_else(|| model.triples.get(shape.first_triple_index));
            problems.push(problem(
                DiagramProblemSeverity::Error,
                message,
                PATH_CARDINALITY_RULE_ID,
                at,
            ));
        }

        if let (Some(min), Some(max)) = (
            number(&model, &shape.key, shacl::MIN_COUNT),
            number(&model, &shape.key, shacl::MAX_COUNT),
        ) && min > max
        {
            problems.push(problem(
                DiagramProblemSeverity::Warning,
                format!(
                    "{} requires at least {min} values and at most {max}, so nothing can ever conform to it.",
                    display_of(&model, shape)
                ),
                IMPOSSIBLE_COUNTS_RULE_ID,
                triple(&model, &shape.key, shacl::MIN_COUNT),
            ));
        }

        let datatype = triple(&model, &shape.key, shacl::DATATYPE);
        if datatype.is_some() && triple(&model, &shape.key, shacl::CLASS).is_some() {
            problems.push(problem(
                DiagramProblemSeverity::Warning,
                format!(
                    "{} constrains its values by both sh:datatype and sh:class, and no value is both a literal and an instance of a class, so nothing can conform.",
                    display_of(&model, shape)
                ),
                DATATYPE_AND_CLASS_RULE_ID,
                datatype,
            ));
        }
    }

    let described: HashSet<String> = model.triples.iter().filter_map(|t| key_of(&t.subject)).collect();
    for t in &model.triples {
        if t.predicate.iri != shacl::NODE && t.predicate.iri != shacl::PROPERTY {
            continue;
        }
        let Some(reference) = t.object.as_iri() else {
            continue;
        };
        if key_of(&t.object).is_some_and(|k| described.contains(&k)) {
            continue;
        }
        problems.push(problem(
            DiagramProblemSeverity::Info,
            format!(
                "{} is referenced as a shape but not described in this file - it is defined elsewhere, or missing. Nothing outside the file is read to find out.",
                display(&model, reference)
            ),
            REFERENCE_LEAVES_FILE_RULE_ID,
            Some(t),
        ));
    }

    for t in &model.triples {
        for term in [Some(t.predicate.iri.as_str()), t.object.as_iri()]
            .into_iter()
            .flatten()
        {
            let Some(local) = term.strip_prefix(shacl::NAMESPACE) else {
                continue;
            };
            if shacl::is_known_term(term) {
                continue;
            }
            problems.push(problem(
                DiagramProblemSeverity::Warning,
                format!("sh:{local} is not a term SHACL defines, so no processor will act on it - check the spelling."),
                UNKNOWN_TERM_RULE_ID,
                Some(t),
            ));
        }
    }
    problems
}
