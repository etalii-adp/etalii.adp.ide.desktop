//! The family's named splice operations (etalii.adp spec 030, task T083): the one writer every reading's edit goes
//! through. Each answers with an empty string on success, or a refusal sentence before any splice, and a refused
//! document is untouched to the byte.
//!
//! Ported from the standalone host's `RdfWriter.cs` at `8b6ccfc`. Every operation takes the model beside the document
//! because the model carries the offsets the parser recorded, valid only for the parse they came from, so a caller
//! performing several operations parses again between them. Terms are written with the document's own declared
//! prefixes where one matches, and as bracketed IRIs otherwise: a writer never invents a declaration.

use std::collections::HashMap;

use crate::iri;
use crate::lines::{LineDocument, LineRange};
use crate::model::{IriTerm, RdfModel, RdfTerm, RdfTriple};
use crate::parser::{self, TokenKind, Tokenizer};
use crate::vocabulary::rdf;

/// What an edit keyed to a blank node answers (the blank-node identity boundary).
pub const BLANK_REFUSAL: &str = "That element is rooted in a blank node, which has no identity that survives a reparse, so this edit cannot land in the file. Name the node with an IRI to edit it.";

/// What an edit of a triple the document no longer states answers.
pub const STALE_TRIPLE: &str =
    "That triple is not part of the document as it stands - the file may have changed since it was read.";

/// States one more triple: appended to the subject's existing statement as a `;` continuation matching its
/// indentation, or as a new statement at the end of the document.
pub fn add_triple(
    document: &mut LineDocument,
    model: &RdfModel,
    subject_iri: &str,
    predicate_iri: &str,
    object: &RdfTerm,
) -> String {
    if object.is_blank() {
        return BLANK_REFUSAL.to_owned();
    }
    let predicate = if predicate_iri == rdf::TYPE {
        "a".to_owned()
    } else {
        compress(model, predicate_iri)
    };
    let written = format!("{predicate} {}", write_term(model, object));
    let anchor = model
        .triples
        .iter()
        .filter(|t| t.subject.is_iri(subject_iri))
        .max_by_key(|t| t.terminator_offset);

    let Some(anchor) = anchor else {
        let statement = format!("{} {written} .", compress(model, subject_iri));
        let index = document.len();
        if index > 0 && !document.lines()[index - 1].is_blank() {
            document.insert(index, &[String::new(), statement]);
        } else {
            document.insert(index, &[statement]);
        }
        return String::new();
    };

    let line_index = document.index();
    let terminator_line = line_index.line_of(anchor.terminator_offset);
    let terminator_column = anchor.terminator_offset - line_index.start_of(terminator_line);
    let line_text = document.line(terminator_line).to_owned();
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
    document.insert(terminator_line + 1, &[format!("{indent}{written} .")]);
    String::new()
}

/// Removes one triple: its whole lines where it owns them, the exact fragment and its stranded separator where it
/// shares a line, and the whole statement where it is the statement's last triple.
pub fn remove_triple(document: &mut LineDocument, model: &RdfModel, triple: &RdfTriple) -> String {
    if triple.subject.is_blank() || triple.object.is_blank() {
        return BLANK_REFUSAL.to_owned();
    }
    remove_triple_anchored(document, model, triple)
}

/// Removes one triple without the blank-node refusal, for a caller that anchored the edit to an IRI-named subject.
pub fn remove_triple_anchored(document: &mut LineDocument, model: &RdfModel, triple: &RdfTriple) -> String {
    if !model.triples.contains(triple) {
        return STALE_TRIPLE.to_owned();
    }
    let mates: Vec<&RdfTriple> = model
        .triples
        .iter()
        .filter(|t| t.statement == triple.statement && *t != triple)
        .collect();
    if mates.is_empty() {
        document.remove(triple.statement);
        return String::new();
    }
    let continuations = mates
        .iter()
        .any(|m| m.is_list_continuation() && m.subject == triple.subject && m.predicate.iri == triple.predicate.iri);
    let text = document.text();
    let (start, end) = if triple.is_list_continuation() || continuations {
        extend_over_separator(&text, triple.object_start, triple.object_end, ',')
    } else {
        extend_over_separator(&text, triple.span_start, triple.span_end, ';')
    };
    remove_region(document, start, end);
    String::new()
}

fn touching<'a>(model: &'a RdfModel, iri: &str) -> Vec<&'a RdfTriple> {
    model
        .triples
        .iter()
        .filter(|t| t.subject.is_iri(iri) || t.object.is_iri(iri))
        .collect()
}

/// Removes every triple the IRI-named resource is subject or object of, the bottom-most first.
pub fn remove_resource(document: &mut LineDocument, model: &RdfModel, iri: &str) -> String {
    let matches = touching(model, iri);
    if matches.is_empty() {
        return format!("Nothing in the document names {iri}, so there is nothing to remove.");
    }
    if matches.iter().any(|t| t.subject.is_blank() || t.object.is_blank()) {
        return BLANK_REFUSAL.to_owned();
    }
    let mut current = model.clone();
    loop {
        let Some(next) = touching(&current, iri)
            .into_iter()
            .rev()
            .max_by_key(|t| t.span_start)
            .cloned()
        else {
            return String::new();
        };
        let refusal = remove_triple(document, &current, &next);
        if !refusal.is_empty() {
            return refusal;
        }
        current = parser::parse(document).unwrap_or_default();
    }
}

/// Renames a term everywhere it occurs - subject, predicate, object, datatype, prefixed or full - in one operation;
/// collisions are refused first.
pub fn rename_term(document: &mut LineDocument, model: &RdfModel, old_iri: &str, new_iri: &str) -> String {
    if old_iri == new_iri {
        return "The new name is the same as the old one, so there is nothing to rename.".to_owned();
    }
    let names = |triple: &RdfTriple, iri: &str| {
        triple.subject.is_iri(iri)
            || triple.predicate.iri == iri
            || triple.object.is_iri(iri)
            || triple
                .object
                .as_literal()
                .is_some_and(|l| l.datatype.as_deref() == Some(iri))
    };
    if model.triples.iter().any(|t| names(t, new_iri)) {
        return format!(
            "{new_iri} already names something in this document, so renaming onto it would silently merge two resources. Pick an unused IRI."
        );
    }
    if !model.triples.iter().any(|t| names(t, old_iri)) {
        return format!("Nothing in the document names {old_iri}, so there is nothing to rename.");
    }
    let written = compress(model, new_iri);
    let mut occurrences = term_occurrences(&document.text(), old_iri);
    occurrences.sort_by_key(|o| std::cmp::Reverse(o.0));
    for (start, end) in occurrences {
        replace_region(document, start, end, &written);
    }
    String::new()
}

/// Rewrites exactly one literal object token in place, leaving every neighbouring byte untouched; a language wins over
/// a datatype where both arrive.
pub fn replace_object_literal(
    document: &mut LineDocument,
    model: &RdfModel,
    triple: &RdfTriple,
    lexical: &str,
    language: Option<&str>,
    datatype: Option<&str>,
) -> String {
    if !model.triples.contains(triple) {
        return STALE_TRIPLE.to_owned();
    }
    if triple.object.as_literal().is_none() {
        return "That value is not a literal, so it cannot be rewritten as one. Rename or reconnect the resource instead."
            .to_owned();
    }
    if triple.subject.is_blank() {
        return BLANK_REFUSAL.to_owned();
    }
    let mut written = format!("\"{}\"", escape_literal(lexical));
    match (language, datatype) {
        (Some(language), _) if !language.is_empty() => {
            written.push('@');
            written.push_str(language);
        }
        (_, Some(datatype)) if !datatype.is_empty() && datatype != rdf::XSD_STRING => {
            written.push_str("^^");
            written.push_str(&compress(model, datatype));
        }
        _ => {}
    }
    replace_region(document, triple.object_start, triple.object_end, &written);
    String::new()
}

/// Declares one more prefix, beside the existing declaration run.
pub fn add_prefix(document: &mut LineDocument, model: &RdfModel, prefix: &str, iri: &str) -> String {
    if let Some(existing) = model.expansion(prefix) {
        return if existing == iri {
            format!("The prefix '{prefix}:' already expands to {iri}, so there is nothing to add.")
        } else {
            format!(
                "The prefix '{prefix}:' is already declared as {existing}. Redeclaring it would change what every use of it means."
            )
        };
    }
    let line = match (model.prefixes.iter().map(|p| p.line).max(), model.base_line) {
        (Some(last), _) => last + 1,
        (None, Some(base)) => base + 1,
        (None, None) => 0,
    };
    document.insert(line, &[format!("@prefix {prefix}: <{iri}> .")]);
    String::new()
}

/// The IRI written with the model's own declared prefixes where the longest-matching one yields a clean local name,
/// and bracketed otherwise (`RdfWriter.Compress`).
pub fn compress(model: &RdfModel, iri: &str) -> String {
    let mut best: Option<&crate::model::PrefixDeclaration> = None;
    for declaration in &model.prefixes {
        if !iri.starts_with(declaration.iri.as_str()) || iri.len() == declaration.iri.len() {
            continue;
        }
        if best.is_none_or(|b| declaration.iri.len() >= b.iri.len()) {
            best = Some(declaration);
        }
    }
    let Some(best) = best else {
        return format!("<{iri}>");
    };
    let local = &iri[best.iri.len()..];
    if is_clean_local_name(local) {
        format!("{}:{local}", best.prefix)
    } else {
        format!("<{iri}>")
    }
}

fn is_clean_local_name(local: &str) -> bool {
    !local.is_empty()
        && !local.starts_with('.')
        && !local.ends_with('.')
        && local
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
}

/// A term as the writer states it.
pub fn write_term(model: &RdfModel, term: &RdfTerm) -> String {
    match term {
        RdfTerm::Iri(iri) => compress(model, &iri.iri),
        RdfTerm::Literal(literal) => {
            if let Some(language) = literal.language.as_deref().filter(|l| !l.is_empty()) {
                format!("\"{}\"@{language}", escape_literal(&literal.lexical))
            } else if let Some(datatype) = literal
                .datatype
                .as_deref()
                .filter(|d| !d.is_empty() && *d != rdf::XSD_STRING)
            {
                format!(
                    "\"{}\"^^{}",
                    escape_literal(&literal.lexical),
                    compress(model, datatype)
                )
            } else {
                format!("\"{}\"", escape_literal(&literal.lexical))
            }
        }
        RdfTerm::Blank(_) => unreachable!("blank nodes are refused before writing begins"),
    }
}

/// A literal's value escaped for a short string.
pub fn escape_literal(lexical: &str) -> String {
    let mut builder = String::with_capacity(lexical.len());
    for character in lexical.chars() {
        match character {
            '"' => builder.push_str("\\\""),
            '\\' => builder.push_str("\\\\"),
            '\n' => builder.push_str("\\n"),
            '\r' => builder.push_str("\\r"),
            '\t' => builder.push_str("\\t"),
            other => builder.push(other),
        }
    }
    builder
}

/// Grows a removal region over one adjacent list separator and the whitespace binding it: the preceding one where
/// there is one, else the following one.
fn extend_over_separator(text: &str, start: usize, end: usize, separator: char) -> (usize, usize) {
    let before = text[..start].trim_end_matches(char::is_whitespace);
    if before.ends_with(separator) {
        return (before.len() - separator.len_utf8(), end);
    }
    let horizontal = |c: char| c.is_whitespace() && c != '\n' && c != '\r';
    let rest = &text[end..];
    let after_space = rest.len() - rest.trim_start_matches(horizontal).len();
    let after = end + after_space;
    if text[after..].starts_with(separator) {
        let past = after + separator.len_utf8();
        let rest = &text[past..];
        let trailing = rest.len() - rest.trim_start_matches(horizontal).len();
        return (start, past + trailing);
    }
    (start, end)
}

/// Removes the bytes of `start..end`, joining what remains of the first and last affected lines and removing lines
/// left holding only whitespace.
fn remove_region(document: &mut LineDocument, start: usize, end: usize) {
    let line_index = document.index();
    let first_line = line_index.line_of(start);
    let last_line = line_index.line_of(start.max(end.saturating_sub(1)));
    let mut before = document.line(first_line)[..start - line_index.start_of(first_line)].to_owned();
    let after = document.line(last_line)[end - line_index.start_of(last_line)..].to_owned();
    if !before.trim_end().is_empty()
        && !after.is_empty()
        && before.chars().last().is_some_and(char::is_whitespace)
        && after.chars().next().is_some_and(char::is_whitespace)
    {
        before = before.trim_end().to_owned();
    }
    let joined = format!("{before}{after}");
    if joined.trim().is_empty() {
        document.remove(LineRange::new(first_line, last_line));
    } else {
        document.replace(LineRange::new(first_line, last_line), &[joined]);
    }
}

/// Replaces the bytes of `start..end` with `replacement`, as one line.
fn replace_region(document: &mut LineDocument, start: usize, end: usize, replacement: &str) {
    let line_index = document.index();
    let first_line = line_index.line_of(start);
    let last_line = line_index.line_of(start.max(end.saturating_sub(1)));
    let before = document.line(first_line)[..start - line_index.start_of(first_line)].to_owned();
    let after = document.line(last_line)[end - line_index.start_of(last_line)..].to_owned();
    document.replace(
        LineRange::new(first_line, last_line),
        &[format!("{before}{replacement}{after}")],
    );
}

/// Every token range in `text` naming `iri` in any spelling: bracketed, prefixed under the declarations in force at
/// that point, or as a literal's datatype; the keyword `a` is left alone.
fn term_occurrences(text: &str, iri: &str) -> Vec<(usize, usize)> {
    let mut occurrences = Vec::new();
    let mut tokenizer = Tokenizer::new(text);
    let mut prefixes: HashMap<String, String> = HashMap::new();
    let mut base: Option<String> = None;
    let resolve = |base: &Option<String>, candidate: &str| -> String {
        match base {
            Some(base)
                if !candidate.contains("://")
                    && !candidate.starts_with("urn:")
                    && !candidate.starts_with("mailto:") =>
            {
                iri::resolve(base, candidate).unwrap_or_else(|| candidate.to_owned())
            }
            _ => candidate.to_owned(),
        }
    };
    let inner = |value: &str| value[1..value.len() - 1].to_owned();
    loop {
        let Ok(token) = tokenizer.next_token() else {
            return occurrences;
        };
        match token.kind {
            TokenKind::EndOfFile => return occurrences,
            TokenKind::PrefixDirective | TokenKind::SparqlPrefix => {
                let (Ok(name), Ok(value)) = (tokenizer.next_token(), tokenizer.next_token()) else {
                    return occurrences;
                };
                if name.kind == TokenKind::PrefixedName && value.kind == TokenKind::Iri {
                    let prefix = name.value[..name.value.len() - 1].to_owned();
                    prefixes.insert(prefix, resolve(&base, &inner(&value.value)));
                }
            }
            TokenKind::BaseDirective | TokenKind::SparqlBase => {
                let Ok(value) = tokenizer.next_token() else {
                    return occurrences;
                };
                if value.kind == TokenKind::Iri {
                    base = Some(resolve(&base, &inner(&value.value)));
                }
            }
            TokenKind::Iri if resolve(&base, &inner(&token.value)) == iri => {
                occurrences.push((token.start, token.end()));
            }
            TokenKind::PrefixedName => {
                let colon = token.value.find(':').unwrap_or(token.value.len());
                if let Some(expansion) = prefixes.get(&token.value[..colon])
                    && format!("{expansion}{}", &token.value[colon + 1..]) == iri
                {
                    occurrences.push((token.start, token.end()));
                }
            }
            _ => {}
        }
    }
}

fn indent_of(line: &str) -> &str {
    &line[..line.len() - line.trim_start_matches(char::is_whitespace).len()]
}

/// The IRI term of `iri`, as a command carries one: its spelling unknown.
pub fn iri_term(iri: &str) -> RdfTerm {
    RdfTerm::Iri(IriTerm::new(iri, ""))
}
