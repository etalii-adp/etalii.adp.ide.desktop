//! The family's findings (etalii.adp spec 030, task T083): facts about the text, no inference, no network. A file
//! that will not parse is one finding naming its line, since running the rules over an empty model would bury that
//! under a list of consequences. Reading-specific rules stay with the readings, which run these first.
//!
//! Ported from the standalone host's `RdfValidator.cs` at `8b6ccfc`.

use adp_diagram::{DiagramProblem, DiagramProblemLocation, DiagramProblemSeverity};

use crate::iri;
use crate::lines::LineDocument;
use crate::model::{RdfModel, RdfTerm, RdfTriple};
use crate::parser;

/// The rule reported when the file is not Turtle or N-Triples this can read.
pub const UNPARSEABLE_RULE_ID: &str = "rdf.unparseable";

/// The rule reported when a prefix is declared more than once.
pub const DUPLICATE_PREFIX_RULE_ID: &str = "rdf.duplicate-prefix";

/// The rule reported - once - when relative IRIs appear in a document with no base.
pub const RELATIVE_IRI_RULE_ID: &str = "rdf.relative-iri";

/// The rule reported for a language tag no speaker could have meant.
pub const LANGUAGE_TAG_RULE_ID: &str = "rdf.language-tag";

/// The rule reported for a datatype in the XSD namespace that XSD does not define.
pub const UNKNOWN_DATATYPE_RULE_ID: &str = "rdf.unknown-datatype";

const XSD_NAMESPACE: &str = "http://www.w3.org/2001/XMLSchema#";

const XSD_TYPES: [&str; 34] = [
    "string",
    "boolean",
    "decimal",
    "integer",
    "double",
    "float",
    "date",
    "time",
    "dateTime",
    "dateTimeStamp",
    "duration",
    "gYear",
    "gMonth",
    "gDay",
    "gYearMonth",
    "gMonthDay",
    "hexBinary",
    "base64Binary",
    "anyURI",
    "language",
    "normalizedString",
    "token",
    "long",
    "int",
    "short",
    "byte",
    "nonNegativeInteger",
    "positiveInteger",
    "negativeInteger",
    "nonPositiveInteger",
    "unsignedLong",
    "unsignedInt",
    "unsignedShort",
    "unsignedByte",
];

fn warning(message: String, rule_id: &str, line: usize) -> DiagramProblem {
    DiagramProblem {
        severity: DiagramProblemSeverity::Warning,
        message,
        rule_id: rule_id.to_owned(),
        location: Some(DiagramProblemLocation::Line(line as u32 + 1)),
    }
}

/// The findings of the document `text` (`RdfValidator.ValidateAsync`).
pub fn validate(text: &str) -> Vec<DiagramProblem> {
    let document = LineDocument::parse(text);
    let outcome = parser::outcome(&document);
    if let Some((line, message)) = outcome.problem {
        return vec![DiagramProblem {
            severity: DiagramProblemSeverity::Error,
            message: format!("This is not RDF that can be read: {message}"),
            rule_id: UNPARSEABLE_RULE_ID.to_owned(),
            location: Some(DiagramProblemLocation::Line(line as u32 + 1)),
        }];
    }
    judge(&outcome.model)
}

/// Whether `tag` is a well-formed language tag: letters, then optional `-` groups of letters and digits.
pub fn is_language_tag(tag: &str) -> bool {
    let mut groups = tag.split('-');
    let first = groups.next().unwrap_or_default();
    !first.is_empty()
        && first.chars().all(|c| c.is_ascii_alphabetic())
        && groups.all(|g| !g.is_empty() && g.chars().all(|c| c.is_ascii_alphanumeric()))
}

/// The findings over a parsed model (`RdfValidator.Judge`).
pub fn judge(model: &RdfModel) -> Vec<DiagramProblem> {
    let mut problems = Vec::new();

    // A prefix declared twice: every declaration after its first, grouped in order of first declaration.
    let mut seen: Vec<&str> = Vec::new();
    for declaration in &model.prefixes {
        if !seen.contains(&declaration.prefix.as_str()) {
            seen.push(&declaration.prefix);
        }
    }
    for prefix in seen {
        for redeclaration in model.prefixes.iter().filter(|p| p.prefix == prefix).skip(1) {
            problems.push(warning(
                format!(
                    "The prefix '{}:' is declared more than once; from here on it means {}, which changes what every earlier-styled use would say.",
                    redeclaration.prefix, redeclaration.iri
                ),
                DUPLICATE_PREFIX_RULE_ID,
                redeclaration.line,
            ));
        }
    }

    if model.base_iri.is_none() {
        let is_relative = |term: &RdfTerm| term.as_iri().is_some_and(|i| !iri::is_absolute(i));
        let relative = model.triples.iter().find(|t: &&RdfTriple| {
            is_relative(&t.subject) || !iri::is_absolute(&t.predicate.iri) || is_relative(&t.object)
        });
        if let Some(triple) = relative {
            problems.push(warning(
                "This document uses relative IRIs but declares no base, so their meaning depends on where the file happens to sit. Declare an @base.".to_owned(),
                RELATIVE_IRI_RULE_ID,
                triple.span.start_line,
            ));
        }
    }

    for triple in &model.triples {
        let Some(literal) = triple.object.as_literal() else {
            continue;
        };
        if let Some(tag) = literal.language.as_deref().filter(|t| !t.is_empty())
            && !is_language_tag(tag)
        {
            problems.push(warning(
                format!("'@{tag}' is not a well-formed language tag (letters, then optional '-' groups of letters and digits)."),
                LANGUAGE_TAG_RULE_ID,
                triple.span.start_line,
            ));
        }
        if let Some(local) = literal.datatype.as_deref().and_then(|d| d.strip_prefix(XSD_NAMESPACE))
            && !XSD_TYPES.contains(&local)
        {
            problems.push(warning(
                format!("xsd:{local} is not a datatype XSD defines; tools that check will refuse the value."),
                UNKNOWN_DATATYPE_RULE_ID,
                triple.span.start_line,
            ));
        }
    }
    problems
}
