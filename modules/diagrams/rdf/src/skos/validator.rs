//! The scheme reading's findings (etalii.adp spec 030, task T083), ported from the standalone host's
//! `Skos/SkosValidator.cs` at `8b6ccfc`: the family's text facts first, then what makes a vocabulary lie, all by
//! assertion. The cycle finding reads the layout's own detection. The S27 check covers the direct case only: the
//! transitive case needs entailment, which the no-inference rule forbids.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

use adp_diagram::{DiagramProblem, DiagramProblemLocation, DiagramProblemSeverity};

use crate::model::{RdfModel, RdfTerm, RdfTriple};
use crate::projection::{DEFAULT_BUDGET, display};
use crate::skos::labels::{self, SkosLabelSource};
use crate::skos::layout;
use crate::skos::projection::{self, SkosEdgeKind, SkosProjectionResult, element_id, typed_ids};
use crate::skos::registration_language;
use crate::validator::{self as family, UNPARSEABLE_RULE_ID};
use crate::vocabulary::skos;

pub const CYCLE_RULE_ID: &str = "skos.hierarchy-cycle";
pub const DUPLICATE_PREF_LABEL_RULE_ID: &str = "skos.duplicate-preflabel";
pub const RELATED_OVERLAP_RULE_ID: &str = "skos.related-overlap";
pub const MISSING_PREF_LABEL_RULE_ID: &str = "skos.missing-preflabel";
pub const UNFILED_CONCEPT_RULE_ID: &str = "skos.unfiled-concept";
pub const XL_LABELS_RULE_ID: &str = "skos.xl-labels";
pub const NONCONCEPT_RULE_ID: &str = "skos.nonconcept-target";
pub const MISPLACED_HEADER_RULE_ID: &str = "skos.misplaced-header";

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

/// Its fragment or last path segment, without a `res:` prefix (`SkosValidator.Short`).
fn short(iri: &str) -> String {
    crate::projection::local_part(iri.strip_prefix("res:").unwrap_or(iri))
}

/// The findings of `text`, registered through `registration` (`SkosValidator.ValidateAsync`).
pub fn validate(text: &str, registration: Option<&Path>) -> Vec<DiagramProblem> {
    let mut problems = family::validate(text);
    if problems.iter().any(|p| p.rule_id == UNPARSEABLE_RULE_ID) {
        return problems;
    }
    let model = crate::parser::parse(&crate::lines::LineDocument::parse(text)).unwrap_or_default();
    let projection = projection::project(&model, DEFAULT_BUDGET);
    let laid_out = layout::layout(&projection);

    let display_of = |element_id: &str| -> String {
        match projection.concepts.iter().find(|c| c.id == element_id) {
            Some(concept) => labels::choose(&concept.labels, labels::DEFAULT_LANGUAGE, &short(&concept.iri)).text,
            None => short(element_id),
        }
    };

    for cycle in &laid_out.cycles {
        let names: Vec<String> = cycle.concept_ids.iter().map(|id| display_of(id)).collect();
        problems.push(problem(
            DiagramProblemSeverity::Error,
            format!(
                "The broader/narrower hierarchy runs in a circle through {}. The diagram draws every edge and breaks the circle for layering only.",
                names.join(", ")
            ),
            CYCLE_RULE_ID,
            cycle.triples.first(),
        ));
    }

    let concept_ids = typed_ids(&model, skos::CONCEPT);
    for concept in &projection.concepts {
        let mut groups: Vec<(String, Vec<&labels::SkosLabel>)> = Vec::new();
        for label in concept.labels.iter().filter(|l| l.source == SkosLabelSource::Preferred) {
            let key = label.language.clone().unwrap_or_default();
            match groups.iter_mut().find(|(k, _)| *k == key) {
                Some((_, members)) => members.push(label),
                None => groups.push((key, vec![label])),
            }
        }
        for (language, members) in groups.iter().filter(|(_, m)| m.len() > 1) {
            let tag = if language.is_empty() {
                "no language tag".to_owned()
            } else {
                format!("language '{language}'")
            };
            problems.push(problem(
                DiagramProblemSeverity::Error,
                format!(
                    "{} has {} preferred labels with {tag}; SKOS allows one (S14). The lexicographically first is drawn.",
                    display_of(&concept.id),
                    members.len()
                ),
                DUPLICATE_PREF_LABEL_RULE_ID,
                members[0].triple.as_ref(),
            ));
        }
        if concept.labels.iter().all(|l| l.source != SkosLabelSource::Preferred) {
            let fallback = if concept.labels.is_empty() {
                "its IRI"
            } else {
                "an alternate label"
            };
            problems.push(problem(
                DiagramProblemSeverity::Warning,
                format!(
                    "{} has no skos:prefLabel in any language, so the diagram falls back to {fallback}.",
                    display_of(&concept.id)
                ),
                MISSING_PREF_LABEL_RULE_ID,
                None,
            ));
        }
    }

    let unordered = |a: &str, b: &str| -> (String, String) {
        if a <= b {
            (a.to_owned(), b.to_owned())
        } else {
            (b.to_owned(), a.to_owned())
        }
    };
    let hierarchy_pairs: HashSet<(String, String)> = projection
        .edges
        .iter()
        .filter(|e| e.kind == SkosEdgeKind::Hierarchy)
        .map(|e| unordered(&e.from_id, &e.to_id))
        .collect();
    for related in projection.edges.iter().filter(|e| e.kind == SkosEdgeKind::Related) {
        if hierarchy_pairs.contains(&unordered(&related.from_id, &related.to_id)) {
            problems.push(problem(
                DiagramProblemSeverity::Warning,
                format!(
                    "{} and {} are both hierarchy-linked and skos:related, which SKOS S27 forbids. Only this direct case is checked: the transitive case would need inference, which this diagram never does.",
                    display_of(&related.from_id),
                    display_of(&related.to_id)
                ),
                RELATED_OVERLAP_RULE_ID,
                related.triples.first(),
            ));
        }
    }

    report_unfiled(&projection, &mut problems);
    report_non_concept_ends(&model, &concept_ids, &mut problems);

    if projection::has_xl_labels(&model) {
        problems.push(problem(
            DiagramProblemSeverity::Info,
            "This file labels concepts through SKOS-XL, which this reading does not resolve; the plain-SKOS fallbacks apply (Requirement 3.6).".to_owned(),
            XL_LABELS_RULE_ID,
            None,
        ));
    }

    let (_, misplaced_line) = registration_language::read(registration);
    if misplaced_line > 0 {
        problems.push(problem(
            DiagramProblemSeverity::Warning,
            format!(
                "The registration's language: header (line {misplaced_line}) sits above a body: or view: line, where core's header scan would stop before reaching them. It is ignored; move it below body:/view: and above layout:."
            ),
            MISPLACED_HEADER_RULE_ID,
            None,
        ));
    }
    problems
}

fn report_unfiled(projection: &SkosProjectionResult, problems: &mut Vec<DiagramProblem>) {
    let mut children_of: HashMap<&str, Vec<&str>> = HashMap::new();
    for edge in projection.edges.iter().filter(|e| e.kind == SkosEdgeKind::Hierarchy) {
        children_of
            .entry(edge.from_id.as_str())
            .or_default()
            .push(edge.to_id.as_str());
    }
    let mut reached: HashSet<&str> = HashSet::new();
    let mut frontier: VecDeque<&str> = projection
        .schemes
        .iter()
        .flat_map(|s| s.top_concept_ids.iter().map(String::as_str))
        .collect();
    while let Some(id) = frontier.pop_front() {
        if !reached.insert(id) {
            continue;
        }
        if let Some(children) = children_of.get(id) {
            frontier.extend(children.iter().copied());
        }
    }
    for concept in &projection.concepts {
        if concept.scheme_iris.is_empty() && !reached.contains(concept.id.as_str()) {
            problems.push(problem(
                DiagramProblemSeverity::Info,
                format!(
                    "{} is in no concept scheme and under no top concept - legal SKOS, and usually a mistake. It draws in the unfiled band.",
                    short(&concept.iri)
                ),
                UNFILED_CONCEPT_RULE_ID,
                None,
            ));
        }
    }
}

fn report_non_concept_ends(model: &RdfModel, concept_ids: &HashSet<String>, problems: &mut Vec<DiagramProblem>) {
    for triple in &model.triples {
        let check: Vec<&RdfTerm> = match triple.predicate.iri.as_str() {
            skos::BROADER | skos::NARROWER | skos::RELATED => vec![&triple.subject, &triple.object],
            skos::IN_SCHEME | skos::TOP_CONCEPT_OF => vec![&triple.subject],
            skos::HAS_TOP_CONCEPT => vec![&triple.object],
            _ => continue,
        };
        for term in check {
            if let Some(id) = element_id(term)
                && !concept_ids.contains(&id)
            {
                problems.push(problem(
                    DiagramProblemSeverity::Warning,
                    format!(
                        "{} takes part in a {} assertion but is not typed skos:Concept. Typing is never inferred, so only the file's own silence is reported.",
                        short(&id),
                        display(model, &triple.predicate.iri)
                    ),
                    NONCONCEPT_RULE_ID,
                    Some(triple),
                ));
            }
        }
    }
}
