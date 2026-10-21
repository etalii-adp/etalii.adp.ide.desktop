//! The Manchester-style rendering of a blank-node-rooted class expression (etalii.adp spec 030, task T083), ported
//! from the standalone host's `Owl/ExpressionRenderer.cs` at `8b6ccfc`: a pure recursive walk over the structure's
//! triples, depth-capped for canvas labels and uncapped for the property grid's read-only text. A malformed structure
//! short-circuits to a label showing what it has, with `?` standing for what it lacks.

use std::collections::HashSet;

use crate::model::{RdfModel, RdfTerm, RdfTriple};
use crate::owl::index::OwlModelIndex;
use crate::projection;
use crate::vocabulary::{owl, rdf};

/// The canvas label's depth cap: structure nested deeper than two expression levels elides to `…`.
pub const CANVAS_DEPTH: usize = 2;

/// What a rendered class expression came out as (`OwlExpressionText`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpressionText {
    pub text: String,
    /// Whether structure deeper than the depth cap was cut to `…`.
    pub elided: bool,
    /// Whether the structure references itself, rendered `…` at the revisit.
    pub cyclic: bool,
}

/// Renders `root` against `model`, capped at `max_depth` expression levels, none for uncapped
/// (`ExpressionRenderer.Render`).
pub fn render(root: &RdfTerm, model: &RdfModel, max_depth: Option<usize>) -> ExpressionText {
    render_with(root, &OwlModelIndex::build(model), model, max_depth)
}

/// Renders `root` with an index already built.
pub fn render_with(
    root: &RdfTerm,
    index: &OwlModelIndex,
    model: &RdfModel,
    max_depth: Option<usize>,
) -> ExpressionText {
    let mut state = State {
        index,
        model,
        max_depth: max_depth.unwrap_or(usize::MAX),
        path: HashSet::new(),
        elided: false,
        cyclic: false,
    };
    let text = state.term(root, 1);
    ExpressionText {
        text,
        elided: state.elided,
        cyclic: state.cyclic,
    }
}

struct State<'a> {
    index: &'a OwlModelIndex,
    model: &'a RdfModel,
    max_depth: usize,
    path: HashSet<usize>,
    elided: bool,
    cyclic: bool,
}

impl State<'_> {
    fn term(&mut self, term: &RdfTerm, depth: usize) -> String {
        match term {
            RdfTerm::Iri(iri) => self.display(&iri.iri),
            RdfTerm::Literal(literal) => literal.lexical.clone(),
            RdfTerm::Blank(_) if depth > self.max_depth => {
                self.elided = true;
                "…".to_owned()
            }
            RdfTerm::Blank(blank) => {
                if !self.path.insert(blank.ordinal) {
                    self.cyclic = true;
                    return "…".to_owned();
                }
                let text = self.structure(blank, depth);
                self.path.remove(&blank.ordinal);
                text
            }
        }
    }

    fn structure(&mut self, blank: &crate::model::BlankTerm, depth: usize) -> String {
        let about: Vec<RdfTriple> = self.index.triples_of(blank).to_vec();
        for triple in &about {
            match triple.predicate.iri.as_str() {
                owl::UNION_OF => return format!("({})", self.members(&triple.object, depth, " ∪ ")),
                owl::INTERSECTION_OF => return format!("({})", self.members(&triple.object, depth, " ∩ ")),
                owl::COMPLEMENT_OF => return format!("¬{}", self.term(&triple.object, depth + 1)),
                owl::ONE_OF => return format!("{{{}}}", self.members(&triple.object, depth, ", ")),
                _ => {}
            }
        }

        // Not an operator: a restriction, well-formed or otherwise.
        let property = about
            .iter()
            .find(|t| t.predicate.iri == owl::ON_PROPERTY)
            .map(|t| &t.object);
        let property_text = match property.and_then(RdfTerm::as_iri) {
            Some(iri) => self.display(iri),
            None => "?".to_owned(),
        };
        for triple in &about {
            match triple.predicate.iri.as_str() {
                owl::SOME_VALUES_FROM => return format!("∃ {property_text}.{}", self.term(&triple.object, depth + 1)),
                owl::ALL_VALUES_FROM => return format!("∀ {property_text}.{}", self.term(&triple.object, depth + 1)),
                owl::HAS_VALUE => return format!("∋ {property_text}.{}", self.term(&triple.object, depth + 1)),
                owl::CARDINALITY => return self.cardinality("=", &triple.object, &property_text, None, depth),
                owl::MIN_CARDINALITY => return self.cardinality("≥", &triple.object, &property_text, None, depth),
                owl::MAX_CARDINALITY => return self.cardinality("≤", &triple.object, &property_text, None, depth),
                owl::QUALIFIED_CARDINALITY => {
                    return self.cardinality("=", &triple.object, &property_text, filler(&about), depth);
                }
                owl::MIN_QUALIFIED_CARDINALITY => {
                    return self.cardinality("≥", &triple.object, &property_text, filler(&about), depth);
                }
                owl::MAX_QUALIFIED_CARDINALITY => {
                    return self.cardinality("≤", &triple.object, &property_text, filler(&about), depth);
                }
                _ => {}
            }
        }

        // No quantifier at all: show what there is, ? standing for what is missing.
        format!("? {property_text}")
    }

    fn cardinality(
        &mut self,
        comparator: &str,
        count: &RdfTerm,
        property_text: &str,
        filler: Option<RdfTerm>,
        depth: usize,
    ) -> String {
        let count_text = count.as_literal().map_or("?", |l| l.lexical.as_str());
        let head = format!("{comparator} {count_text} {property_text}");
        match filler {
            None => head,
            Some(filler) => format!("{head}.{}", self.term(&filler, depth + 1)),
        }
    }

    fn members(&mut self, head: &RdfTerm, depth: usize, separator: &str) -> String {
        let mut parts = String::new();
        let mut cursor = head.clone();
        while let RdfTerm::Blank(cell) = &cursor {
            let Some(member) = self.index.firsts.get(&cell.ordinal).cloned() else {
                break;
            };
            if !parts.is_empty() {
                parts.push_str(separator);
            }
            let rendered = self.term(&member, depth + 1);
            parts.push_str(&rendered);
            cursor = self
                .index
                .rests
                .get(&cell.ordinal)
                .cloned()
                .unwrap_or_else(|| RdfTerm::iri(rdf::NIL, "rdf:nil"));
        }
        if parts.is_empty() { "?".to_owned() } else { parts }
    }

    fn display(&self, iri: &str) -> String {
        display(self.index, self.model, iri)
    }
}

fn filler(about: &[RdfTriple]) -> Option<RdfTerm> {
    about
        .iter()
        .find(|t| t.predicate.iri == owl::ON_CLASS || t.predicate.iri == owl::ON_DATA_RANGE)
        .map(|t| t.object.clone())
}

/// A term's display in the ontology reading: its `rdfs:label`, else the family's display.
pub fn display(index: &OwlModelIndex, model: &RdfModel, iri: &str) -> String {
    match index.labels.get(iri) {
        Some(label) => label.clone(),
        None => projection::display(model, iri),
    }
}
