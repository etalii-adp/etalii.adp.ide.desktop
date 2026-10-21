//! What one RDF document states (etalii.adp spec 030, task T083): its triples in document order, each with where in
//! the text it is stated, its prefix declarations, and its base IRI.
//!
//! Ported from the standalone host's `src/diagrams/rdf/backend/EtAlii.Adp.Diagram.Rdf/_Model` at `8b6ccfc`
//! (`RdfTerm`, `IriTerm`, `BlankTerm`, `LiteralTerm`, `RdfTriple`, `SourceSpan`, `PrefixDeclaration`, `RdfModel`).
//! Every reading of the family projects over this model; none parses again. Equality is the records' value equality:
//! an IRI term's spelling is part of it, as a triple's offsets are.

use crate::lines::LineRange;

/// An IRI-named term: its full IRI, and the exact spelling in the document (a prefixed name, a bracketed IRI, or `a`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IriTerm {
    pub iri: String,
    pub as_written: String,
}

impl IriTerm {
    pub fn new(iri: impl Into<String>, as_written: impl Into<String>) -> Self {
        IriTerm {
            iri: iri.into(),
            as_written: as_written.into(),
        }
    }
}

/// A blank node: the label of a `_:name` node, or none for an anonymous one, and its place in document order of first
/// appearance, which holds for one parse only (the blank-node identity boundary).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BlankTerm {
    pub label: Option<String>,
    pub ordinal: usize,
}

/// A literal: its value with escapes decoded, its datatype (none for a plain string), its language tag without `@`,
/// and its exact spelling.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LiteralTerm {
    pub lexical: String,
    pub datatype: Option<String>,
    pub language: Option<String>,
    pub as_written: String,
}

impl LiteralTerm {
    pub fn new(lexical: impl Into<String>, datatype: Option<String>, language: Option<String>) -> Self {
        LiteralTerm {
            lexical: lexical.into(),
            datatype,
            language,
            as_written: String::new(),
        }
    }
}

/// One node of a triple: an IRI, a blank node or a literal, and nothing else.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RdfTerm {
    Iri(IriTerm),
    Blank(BlankTerm),
    Literal(LiteralTerm),
}

impl RdfTerm {
    pub fn iri(iri: impl Into<String>, as_written: impl Into<String>) -> Self {
        RdfTerm::Iri(IriTerm::new(iri, as_written))
    }

    /// The full IRI of an IRI term.
    pub fn as_iri(&self) -> Option<&str> {
        match self {
            RdfTerm::Iri(term) => Some(&term.iri),
            _ => None,
        }
    }

    pub fn as_iri_term(&self) -> Option<&IriTerm> {
        match self {
            RdfTerm::Iri(term) => Some(term),
            _ => None,
        }
    }

    pub fn as_blank(&self) -> Option<&BlankTerm> {
        match self {
            RdfTerm::Blank(term) => Some(term),
            _ => None,
        }
    }

    pub fn as_literal(&self) -> Option<&LiteralTerm> {
        match self {
            RdfTerm::Literal(term) => Some(term),
            _ => None,
        }
    }

    pub fn is_blank(&self) -> bool {
        matches!(self, RdfTerm::Blank(_))
    }

    /// Whether this is the IRI `iri`.
    pub fn is_iri(&self, iri: &str) -> bool {
        self.as_iri() == Some(iri)
    }
}

/// Where one triple's own tokens sit: the lines they occupy and, where they share their single line with other
/// substantive content, the byte columns of the fragment within it (`SourceSpan`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceSpan {
    pub start_line: usize,
    pub end_line: usize,
    pub fragment_start: Option<usize>,
    pub fragment_end: Option<usize>,
}

impl SourceSpan {
    /// Whether the triple owns its lines outright.
    pub fn owns_lines(&self) -> bool {
        self.fragment_start.is_none()
    }
}

/// One statement of the graph, with where in the text it is stated (`RdfTriple`). The offsets are valid only for the
/// parse they came from.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RdfTriple {
    pub subject: RdfTerm,
    pub predicate: IriTerm,
    pub object: RdfTerm,
    pub span: SourceSpan,
    /// The lines of the whole statement, subject through terminating `.`.
    pub statement: LineRange,
    pub span_start: usize,
    pub span_end: usize,
    pub object_start: usize,
    pub object_end: usize,
    pub terminator_offset: usize,
}

impl RdfTriple {
    /// Whether this triple is a `,` continuation: its own tokens are the object alone.
    pub fn is_list_continuation(&self) -> bool {
        self.span_start == self.object_start
    }

    /// Whether its subject is the IRI `iri`.
    pub fn has_subject(&self, iri: &str) -> bool {
        self.subject.is_iri(iri)
    }

    /// The IRI of its subject, for an IRI subject.
    pub fn subject_iri(&self) -> Option<&str> {
        self.subject.as_iri()
    }

    /// The IRI of its object, for an IRI object.
    pub fn object_iri(&self) -> Option<&str> {
        self.object.as_iri()
    }
}

/// One prefix declaration, at the line that states it (re-declarations are all kept).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PrefixDeclaration {
    pub prefix: String,
    pub iri: String,
    pub line: usize,
}

/// What one RDF document states (`RdfModel`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RdfModel {
    pub triples: Vec<RdfTriple>,
    pub prefixes: Vec<PrefixDeclaration>,
    pub base_iri: Option<String>,
    /// The line of the base declaration; none when there is none.
    pub base_line: Option<usize>,
}

impl RdfModel {
    /// The model of a document that states nothing, or one that could not be parsed.
    pub fn empty() -> Self {
        RdfModel::default()
    }

    /// The winning expansion of `prefix`: the last declaration, or none where it is not declared.
    pub fn expansion(&self, prefix: &str) -> Option<&str> {
        self.prefixes
            .iter()
            .rev()
            .find(|p| p.prefix == prefix)
            .map(|p| p.iri.as_str())
    }

    /// Whether any triple names `iri` as its subject or object.
    pub fn names(&self, iri: &str) -> bool {
        self.triples
            .iter()
            .any(|t| t.subject.is_iri(iri) || t.object.is_iri(iri))
    }
}
