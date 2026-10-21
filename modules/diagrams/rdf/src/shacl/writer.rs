//! The shapes reading's edit gestures (etalii.adp spec 030, task T083), ported from the standalone host's
//! `Shacl/ShaclWriter.cs` at `8b6ccfc`. Everything triple-shaped goes through the family writer; only the two
//! operations whose definitions speak SHACL rather than Turtle live here: a property row's block, and the removal of a
//! shape with the blank subtrees only it reaches.

use std::collections::{HashSet, VecDeque};

use crate::lines::{LineDocument, LineRange};
use crate::model::{LiteralTerm, RdfModel, RdfTerm, RdfTriple};
use crate::parser;
use crate::shacl::discovery::key_of;
use crate::shacl::refusals;
use crate::vocabulary::{rdf, shacl};
use crate::writer;

/// What a new property row states beyond its path, each omitted from the written block when absent
/// (`ShaclPropertyShapeOptions`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ShaclPropertyShapeOptions {
    /// The full datatype IRI for a `sh:datatype` constraint, empty for none.
    pub datatype_iri: String,
    pub min_count: Option<i64>,
    pub max_count: Option<i64>,
    /// The `sh:name`, empty for none.
    pub name: String,
}

/// Splices one `sh:property [ … ] ;` continuation into an IRI-named shape's block.
pub fn append_property_shape_block(
    document: &mut LineDocument,
    model: &RdfModel,
    shape_iri: &str,
    path_iri: &str,
    options: &ShaclPropertyShapeOptions,
) -> String {
    if shape_iri.trim().is_empty() {
        return refusals::BLANK_ROOTED.to_owned();
    }
    if path_iri.trim().is_empty() {
        return refusals::PATH_REQUIRED.to_owned();
    }
    let Some(anchor) = model
        .triples
        .iter()
        .filter(|t| t.subject.is_iri(shape_iri))
        .max_by_key(|t| t.terminator_offset)
    else {
        return refusals::NO_SUCH_SHAPE.to_owned();
    };
    let block = property_block(model, path_iri, options);
    let line_index = document.index();
    let terminator_line = line_index.line_of(anchor.terminator_offset);
    let terminator_column = anchor
        .terminator_offset
        .checked_sub(line_index.start_of(terminator_line));
    let line_text = document.line(terminator_line).to_owned();
    let Some(terminator_column) = terminator_column.filter(|&c| c < line_text.len() && line_text.as_bytes()[c] == b'.')
    else {
        return refusals::NO_SUCH_SHAPE.to_owned();
    };
    let indent = if anchor.statement.len() > 1 {
        indent_of(document.line(anchor.statement.start + 1)).to_owned()
    } else {
        format!("{}    ", indent_of(document.line(anchor.statement.start)))
    };
    document.replace(
        LineRange::new(terminator_line, terminator_line),
        &[format!(
            "{};{}",
            &line_text[..terminator_column],
            &line_text[terminator_column + 1..]
        )],
    );
    document.insert(terminator_line + 1, &[format!("{indent}{block} .")]);
    String::new()
}

/// Declares one more target on an IRI-named shape, through the family writer.
pub fn add_target(
    document: &mut LineDocument,
    model: &RdfModel,
    shape_iri: &str,
    predicate: &str,
    term: &RdfTerm,
) -> String {
    if shape_iri.trim().is_empty() {
        return refusals::BLANK_ROOTED.to_owned();
    }
    if !shacl::TARGET_PREDICATES.contains(&predicate) {
        return refusals::NO_SUCH_TARGET.to_owned();
    }
    writer::add_triple(document, model, shape_iri, predicate, term)
}

/// Removes one target declaration, addressed as its chip carries it.
pub fn remove_target(
    document: &mut LineDocument,
    model: &RdfModel,
    shape_iri: &str,
    predicate: &str,
    term_iri: &str,
) -> String {
    if shape_iri.trim().is_empty() {
        return refusals::BLANK_ROOTED.to_owned();
    }
    if predicate.trim().is_empty() {
        return refusals::IMPLICIT_TARGET.to_owned();
    }
    match model
        .triples
        .iter()
        .find(|t| t.subject.is_iri(shape_iri) && t.predicate.iri == predicate && t.object.is_iri(term_iri))
    {
        Some(triple) => writer::remove_triple(document, model, &triple.clone()),
        None => refusals::NO_SUCH_TARGET.to_owned(),
    }
}

/// States a new node shape: one subject with one `rdf:type sh:NodeShape` triple.
pub fn create_node_shape(document: &mut LineDocument, model: &RdfModel, shape_iri: &str) -> String {
    if shape_iri.trim().is_empty() {
        return refusals::NO_SUCH_SHAPE.to_owned();
    }
    writer::add_triple(
        document,
        model,
        shape_iri,
        rdf::TYPE,
        &RdfTerm::iri(shacl::NODE_SHAPE, "sh:NodeShape"),
    )
}

/// Switches a shape off or back on: adds or removes its `sh:deactivated true` triple.
pub fn set_deactivated(document: &mut LineDocument, model: &RdfModel, shape_iri: &str, deactivated: bool) -> String {
    if shape_iri.trim().is_empty() {
        return refusals::BLANK_ROOTED.to_owned();
    }
    let existing = model
        .triples
        .iter()
        .find(|t| t.subject.is_iri(shape_iri) && t.predicate.iri == shacl::DEACTIVATED)
        .cloned();
    if deactivated {
        return match existing {
            Some(_) => String::new(),
            None => writer::add_triple(
                document,
                model,
                shape_iri,
                shacl::DEACTIVATED,
                &RdfTerm::Literal(LiteralTerm {
                    lexical: "true".to_owned(),
                    datatype: Some(rdf::XSD_BOOLEAN.to_owned()),
                    language: None,
                    as_written: "true".to_owned(),
                }),
            ),
        };
    }
    match existing {
        None => String::new(),
        Some(triple) => writer::remove_triple(document, model, &triple),
    }
}

/// Sets a shape's `sh:name` or `sh:description`: rewritten where one is stated, added where none is.
pub fn set_literal(
    document: &mut LineDocument,
    model: &RdfModel,
    shape_iri: &str,
    predicate: &str,
    value: &str,
) -> String {
    if shape_iri.trim().is_empty() {
        return refusals::BLANK_ROOTED.to_owned();
    }
    let existing = model
        .triples
        .iter()
        .find(|t| t.subject.is_iri(shape_iri) && t.predicate.iri == predicate && t.object.as_literal().is_some())
        .cloned();
    match existing {
        Some(triple) => writer::replace_object_literal(document, model, &triple, value, None, None),
        None => writer::add_triple(
            document,
            model,
            shape_iri,
            predicate,
            &RdfTerm::Literal(LiteralTerm {
                lexical: value.to_owned(),
                datatype: None,
                language: None,
                as_written: format!("\"{value}\""),
            }),
        ),
    }
}

/// How many triples [`remove_shape_with_subtrees`] would take.
pub fn count_shape_removal(model: &RdfModel, shape_iri: &str) -> usize {
    if shape_iri.trim().is_empty() {
        return 0;
    }
    let exclusive = exclusive_blanks(model, shape_iri);
    model.triples.iter().filter(|t| owned(t, shape_iri, &exclusive)).count()
}

/// Removes an IRI-named shape: the separately stated exclusive subtrees first, then its own triples, which carry every
/// inline subtree with them, the model read again after each removal.
pub fn remove_shape_with_subtrees(document: &mut LineDocument, model: &RdfModel, shape_iri: &str) -> String {
    if shape_iri.trim().is_empty() {
        return refusals::BLANK_ROOTED.to_owned();
    }
    if !model.triples.iter().any(|t| t.subject.is_iri(shape_iri)) {
        return refusals::NO_SUCH_SHAPE.to_owned();
    }
    let mut current = model.clone();
    while let Some(next) = separately_stated_subtree_triples(&current, shape_iri)
        .into_iter()
        .rev()
        .max_by_key(|t| t.span_start)
    {
        let refusal = writer::remove_triple_anchored(document, &current, &next);
        if !refusal.is_empty() {
            return refusal;
        }
        current = parser::parse(document).unwrap_or_default();
    }
    loop {
        let Some(next) = current
            .triples
            .iter()
            .filter(|t| t.subject.is_iri(shape_iri))
            .rev()
            .max_by_key(|t| t.span_start)
            .cloned()
        else {
            return String::new();
        };
        let refusal = writer::remove_triple_anchored(document, &current, &next);
        if !refusal.is_empty() {
            return refusal;
        }
        current = parser::parse(document).unwrap_or_default();
    }
}

fn separately_stated_subtree_triples(model: &RdfModel, shape_iri: &str) -> Vec<RdfTriple> {
    let exclusive = exclusive_blanks(model, shape_iri);
    if exclusive.is_empty() {
        return Vec::new();
    }
    let own: Vec<LineRange> = model
        .triples
        .iter()
        .filter(|t| t.subject.is_iri(shape_iri))
        .map(|t| t.statement)
        .collect();
    model
        .triples
        .iter()
        .filter(|t| {
            key_of(&t.subject).is_some_and(|k| exclusive.contains(&k))
                && !own
                    .iter()
                    .any(|s| t.statement.start >= s.start && t.statement.start <= s.end)
        })
        .cloned()
        .collect()
}

fn owned(triple: &RdfTriple, shape_iri: &str, exclusive: &HashSet<String>) -> bool {
    triple.subject.is_iri(shape_iri) || key_of(&triple.subject).is_some_and(|k| exclusive.contains(&k))
}

/// The blank nodes reachable from the shape that nothing else reaches, as a fixed point.
fn exclusive_blanks(model: &RdfModel, shape_iri: &str) -> HashSet<String> {
    let mut candidates = HashSet::new();
    let mut queue = VecDeque::new();
    for triple in &model.triples {
        if triple.subject.is_iri(shape_iri)
            && triple.object.is_blank()
            && let Some(key) = key_of(&triple.object)
            && candidates.insert(key.clone())
        {
            queue.push_back(key);
        }
    }
    while let Some(key) = queue.pop_front() {
        for triple in &model.triples {
            if key_of(&triple.subject).as_deref() == Some(key.as_str())
                && triple.object.is_blank()
                && let Some(child) = key_of(&triple.object)
                && candidates.insert(child.clone())
            {
                queue.push_back(child);
            }
        }
    }
    let mut exclusive = candidates;
    loop {
        let mut changed = false;
        let keys: Vec<String> = exclusive.iter().cloned().collect();
        for key in keys {
            for triple in &model.triples {
                if key_of(&triple.object).as_deref() != Some(key.as_str()) {
                    continue;
                }
                let from_shape = triple.subject.is_iri(shape_iri);
                let from_removed = key_of(&triple.subject).is_some_and(|owner| exclusive.contains(&owner));
                if !from_shape && !from_removed {
                    exclusive.remove(&key);
                    changed = true;
                    break;
                }
            }
        }
        if !changed {
            return exclusive;
        }
    }
}

/// The `sh:property [ … ]` block a row is written as (`ShaclWriter.PropertyBlock`).
fn property_block(model: &RdfModel, path_iri: &str, options: &ShaclPropertyShapeOptions) -> String {
    let term = |iri: &str| writer::compress(model, iri);
    let mut parts = vec![format!("{} {}", term(shacl::PATH), term(path_iri))];
    if !options.datatype_iri.is_empty() {
        parts.push(format!("{} {}", term(shacl::DATATYPE), term(&options.datatype_iri)));
    }
    if let Some(min) = options.min_count {
        parts.push(format!("{} {min}", term(shacl::MIN_COUNT)));
    }
    if let Some(max) = options.max_count {
        parts.push(format!("{} {max}", term(shacl::MAX_COUNT)));
    }
    if !options.name.is_empty() {
        let escaped = options.name.replace('\\', "\\\\").replace('"', "\\\"");
        parts.push(format!("{} \"{escaped}\"", term(shacl::NAME)));
    }
    format!("{} [ {} ]", term(shacl::PROPERTY), parts.join(" ; "))
}

fn indent_of(line: &str) -> &str {
    &line[..line.len() - line.trim_start_matches([' ', '\t']).len()]
}
