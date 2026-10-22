//! Reads a document's text into the model every reading of the family shares (etalii.adp spec 030, task T083):
//! triples with source spans, prefix declarations and the base IRI. Turtle in full, and N-Triples, being a subset.
//!
//! Ported from the standalone host's `RdfTokenizer.cs`, `RdfToken.cs`, `RdfTokenKind.cs`, `RdfPendingTriple.cs`,
//! `RdfParseException.cs` and `RdfParser.cs` at `8b6ccfc`: recursive descent over hand-rolled tokens, each addressed by
//! its exact slice of the text, so a triple's lines and, on a shared line, its exact fragment are recorded for the
//! writers to splice. The parser never rewrites anything.

use std::collections::HashMap;

use crate::iri;
use crate::lines::{LineDocument, LineIndex, LineRange};
use crate::model::{BlankTerm, IriTerm, LiteralTerm, PrefixDeclaration, RdfModel, RdfTerm, RdfTriple, SourceSpan};
use crate::scanner::{self, TerminalProblem};
use crate::vocabulary::rdf;

/// Why a text is not Turtle or N-Triples this can read, and the 1-based line it stopped at (`RdfParseException`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
}

/// The kinds of token (`RdfTokenKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Iri,
    PrefixedName,
    BlankNodeLabel,
    String,
    LanguageTag,
    Number,
    Boolean,
    A,
    PrefixDirective,
    BaseDirective,
    SparqlPrefix,
    SparqlBase,
    Dot,
    Semicolon,
    Comma,
    DoubleCaret,
    OpenParen,
    CloseParen,
    OpenBracket,
    CloseBracket,
    EndOfFile,
}

/// One token: its kind, its byte offset and its raw text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub start: usize,
    /// How many bytes of the text the token spans, which a language tag's value (without its `@`) does not say.
    pub length: usize,
    pub value: String,
}

impl Token {
    /// The offset just past the token.
    pub fn end(&self) -> usize {
        self.start + self.length
    }
}

/// Cuts a text into Turtle tokens (`RdfTokenizer`).
pub struct Tokenizer<'a> {
    text: &'a str,
    lines: LineIndex,
    position: usize,
}

impl<'a> Tokenizer<'a> {
    pub fn new(text: &'a str) -> Self {
        Tokenizer {
            text,
            lines: LineIndex::new(text),
            position: 0,
        }
    }

    /// The zero-based line holding `offset`.
    pub fn line_of(&self, offset: usize) -> usize {
        self.lines.line_of(offset)
    }

    /// The offset at which the zero-based `line` starts.
    pub fn start_of_line(&self, line: usize) -> usize {
        self.lines.start_of(line)
    }

    fn error(&self, message: impl Into<String>, offset: usize) -> ParseError {
        ParseError {
            message: message.into(),
            line: self.line_of(offset) + 1,
        }
    }

    fn token(&mut self, kind: TokenKind, start: usize, end: usize) -> Token {
        self.position = end;
        Token {
            kind,
            start,
            length: end - start,
            value: self.text[start..end].to_owned(),
        }
    }

    fn fixed(&mut self, kind: TokenKind, start: usize, value: &str) -> Token {
        self.position = start + value.len();
        Token {
            kind,
            start,
            length: value.len(),
            value: value.to_owned(),
        }
    }

    /// The next token, consuming it; whitespace and comments are skipped.
    pub fn next_token(&mut self) -> Result<Token, ParseError> {
        self.position = scanner::skip_trivia(self.text, self.position);
        let start = self.position;
        let Some(character) = scanner::char_at(self.text, start) else {
            return Ok(Token {
                kind: TokenKind::EndOfFile,
                start: self.text.len(),
                length: 0,
                value: String::new(),
            });
        };
        let next = scanner::char_at(self.text, start + character.len_utf8());
        match character {
            '<' => return self.read_iri(start),
            '"' | '\'' => return self.read_string(start),
            '@' => return self.read_at_word(start),
            '.' => {
                if next.is_some_and(|c| c.is_ascii_digit()) {
                    let end = scanner::number(self.text, start);
                    return Ok(self.token(TokenKind::Number, start, end));
                }
                return Ok(self.fixed(TokenKind::Dot, start, "."));
            }
            ';' => return Ok(self.fixed(TokenKind::Semicolon, start, ";")),
            ',' => return Ok(self.fixed(TokenKind::Comma, start, ",")),
            '(' => return Ok(self.fixed(TokenKind::OpenParen, start, "(")),
            ')' => return Ok(self.fixed(TokenKind::CloseParen, start, ")")),
            '[' => return Ok(self.fixed(TokenKind::OpenBracket, start, "[")),
            ']' => return Ok(self.fixed(TokenKind::CloseBracket, start, "]")),
            '^' => {
                if next == Some('^') {
                    return Ok(self.fixed(TokenKind::DoubleCaret, start, "^^"));
                }
                return Err(self.error("A lone '^' is not a Turtle token; the datatype marker is '^^'.", start));
            }
            '_' => {
                if next == Some(':') {
                    let end = scanner::local_name(self.text, start + 2);
                    return Ok(self.token(TokenKind::BlankNodeLabel, start, end));
                }
                return Err(self.error("A blank node label starts with '_:'.", start));
            }
            _ => {}
        }
        if character == '+' || character == '-' || character.is_ascii_digit() {
            let end = scanner::number(self.text, start);
            return Ok(self.token(TokenKind::Number, start, end));
        }
        if scanner::is_name_char(character) || character == ':' {
            return self.read_name(start);
        }
        Err(self.error(format!("Unexpected character '{character}'."), start))
    }

    fn read_iri(&mut self, start: usize) -> Result<Token, ParseError> {
        let scan = scanner::iri(self.text, start);
        match scan.problem {
            TerminalProblem::None => Ok(self.token(TokenKind::Iri, start, scan.end)),
            TerminalProblem::LineEnded => {
                Err(self.error("An IRI ran to the end of its line without its closing '>'.", start))
            }
            TerminalProblem::FileEnded => {
                Err(self.error("An IRI ran to the end of the file without its closing '>'.", start))
            }
        }
    }

    fn read_string(&mut self, start: usize) -> Result<Token, ParseError> {
        let scan = scanner::string(self.text, start);
        let bytes = self.text.as_bytes();
        let is_long = start + 2 < bytes.len() && bytes[start + 1] == bytes[start] && bytes[start + 2] == bytes[start];
        match scan.problem {
            TerminalProblem::None => Ok(self.token(TokenKind::String, start, scan.end)),
            TerminalProblem::LineEnded => {
                Err(self.error("A string ran to the end of its line without its closing quote.", start))
            }
            TerminalProblem::FileEnded if is_long => Err(self.error(
                "A long string ran to the end of the file without its closing quotes.",
                start,
            )),
            TerminalProblem::FileEnded => {
                Err(self.error("A string ran to the end of the file without its closing quote.", start))
            }
        }
    }

    fn read_at_word(&mut self, start: usize) -> Result<Token, ParseError> {
        let bytes = self.text.as_bytes();
        let mut position = start + 1;
        while position < bytes.len() && (bytes[position].is_ascii_alphabetic() || bytes[position] == b'-') {
            position += 1;
        }
        let word = self.text[start + 1..position].to_owned();
        self.position = position;
        match word.as_str() {
            "prefix" => Ok(Token {
                kind: TokenKind::PrefixDirective,
                start,
                length: position - start,
                value: "@prefix".to_owned(),
            }),
            "base" => Ok(Token {
                kind: TokenKind::BaseDirective,
                start,
                length: position - start,
                value: "@base".to_owned(),
            }),
            "" => Err(self.error("A lone '@' is not a Turtle token.", start)),
            _ => Ok(Token {
                kind: TokenKind::LanguageTag,
                start,
                length: position - start,
                value: word,
            }),
        }
    }

    fn read_name(&mut self, start: usize) -> Result<Token, ParseError> {
        let mut position = start;
        while let Some(character) = scanner::char_at(self.text, position) {
            if !scanner::is_name_char(character) {
                break;
            }
            position += character.len_utf8();
        }
        if scanner::char_at(self.text, position) != Some(':') {
            let word = self.text[start..position].to_owned();
            self.position = position;
            return match word.as_str() {
                "a" => Ok(Token {
                    kind: TokenKind::A,
                    start,
                    length: word.len(),
                    value: word,
                }),
                "true" | "false" => Ok(Token {
                    kind: TokenKind::Boolean,
                    start,
                    length: word.len(),
                    value: word,
                }),
                _ if word.eq_ignore_ascii_case("PREFIX") => Ok(Token {
                    kind: TokenKind::SparqlPrefix,
                    start,
                    length: word.len(),
                    value: word,
                }),
                _ if word.eq_ignore_ascii_case("BASE") => Ok(Token {
                    kind: TokenKind::SparqlBase,
                    start,
                    length: word.len(),
                    value: word,
                }),
                _ => Err(self.error(
                    format!("'{word}' is not a Turtle keyword, and a prefixed name needs its ':'."),
                    start,
                )),
            };
        }
        let end = scanner::local_name(self.text, position + 1);
        Ok(self.token(TokenKind::PrefixedName, start, end))
    }
}

/// A triple read but not yet placed: its terms and the offsets of its own tokens (`RdfPendingTriple`).
struct Pending {
    subject: RdfTerm,
    predicate: IriTerm,
    object: RdfTerm,
    start: usize,
    end: usize,
    object_start: usize,
}

/// What a document states (`RdfParser`).
pub struct Parser<'a> {
    document: &'a LineDocument,
    text: &'a str,
    tokenizer: Tokenizer<'a>,
    triples: Vec<RdfTriple>,
    declarations: Vec<PrefixDeclaration>,
    prefixes: HashMap<String, String>,
    labeled_blanks: HashMap<String, BlankTerm>,
    current: Token,
    base: Option<String>,
    base_line: Option<usize>,
    blank_ordinal: usize,
}

/// What `document` states, or why it is not valid Turtle and where (`RdfParser.Parse`).
pub fn parse(document: &LineDocument) -> Result<RdfModel, ParseError> {
    let text = document.text();
    let mut parser = Parser::start(document, &text)?;
    parser.parse_document()
}

/// The outcome the store and the validators keep: the model, or the empty model with the line and sentence the parser
/// stopped at (`RdfParser.Outcome`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseOutcome {
    pub model: RdfModel,
    /// The 0-based line and the sentence of the failure, for a text that does not parse.
    pub problem: Option<(usize, String)>,
    /// Why nothing may be written to it: "it does not parse at line N. ...".
    pub unreadable: Option<String>,
}

impl ParseOutcome {
    pub fn is_usable(&self) -> bool {
        self.unreadable.is_none()
    }
}

/// What `document` states, never failing: a text that is not Turtle reads as the empty model and its reason.
pub fn outcome(document: &LineDocument) -> ParseOutcome {
    match parse(document) {
        Ok(model) => ParseOutcome {
            model,
            problem: None,
            unreadable: None,
        },
        Err(error) => ParseOutcome {
            model: RdfModel::empty(),
            problem: Some((error.line.max(1) - 1, error.message.clone())),
            unreadable: Some(format!("it does not parse at line {}. {}", error.line, error.message)),
        },
    }
}

impl<'a> Parser<'a> {
    fn start(document: &'a LineDocument, text: &'a str) -> Result<Self, ParseError> {
        let mut tokenizer = Tokenizer::new(text);
        let current = tokenizer.next_token()?;
        Ok(Parser {
            document,
            text,
            tokenizer,
            triples: Vec::new(),
            declarations: Vec::new(),
            prefixes: HashMap::new(),
            labeled_blanks: HashMap::new(),
            current,
            base: None,
            base_line: None,
            blank_ordinal: 0,
        })
    }

    fn advance(&mut self) -> Result<(), ParseError> {
        self.current = self.tokenizer.next_token()?;
        Ok(())
    }

    fn error(&self, message: impl Into<String>) -> ParseError {
        let offset = if self.current.kind == TokenKind::EndOfFile {
            self.text.len().saturating_sub(1)
        } else {
            self.current.start
        };
        ParseError {
            message: message.into(),
            line: self.tokenizer.line_of(offset) + 1,
        }
    }

    fn expect(&mut self, kind: TokenKind, expectation: &str) -> Result<(), ParseError> {
        if self.current.kind != kind {
            let found = if self.current.kind == TokenKind::EndOfFile {
                "end of file".to_owned()
            } else {
                self.current.value.clone()
            };
            return Err(self.error(format!("Expected {expectation}, found '{found}'.")));
        }
        self.advance()
    }

    fn parse_document(&mut self) -> Result<RdfModel, ParseError> {
        while self.current.kind != TokenKind::EndOfFile {
            match self.current.kind {
                TokenKind::PrefixDirective => self.parse_prefix_directive(true)?,
                TokenKind::SparqlPrefix => self.parse_prefix_directive(false)?,
                TokenKind::BaseDirective => self.parse_base_directive(true)?,
                TokenKind::SparqlBase => self.parse_base_directive(false)?,
                _ => self.parse_statement()?,
            }
        }
        Ok(RdfModel {
            triples: std::mem::take(&mut self.triples),
            prefixes: std::mem::take(&mut self.declarations),
            base_iri: self.base.clone(),
            base_line: self.base_line,
        })
    }

    fn parse_prefix_directive(&mut self, expect_dot: bool) -> Result<(), ParseError> {
        let directive = self.current.clone();
        self.advance()?;
        if self.current.kind != TokenKind::PrefixedName || !self.current.value.ends_with(':') {
            return Err(self.error(format!("{} expects a prefix ending in ':'.", directive.value)));
        }
        let prefix = self.current.value[..self.current.value.len() - 1].to_owned();
        self.advance()?;
        if self.current.kind != TokenKind::Iri {
            return Err(self.error(format!("{} expects a '<...>' IRI after the prefix.", directive.value)));
        }
        let iri = self.resolve(&scanner::iri_value(&self.current.value));
        self.advance()?;
        if expect_dot {
            self.expect(TokenKind::Dot, &format!("{} ends with a '.'", directive.value))?;
        }
        self.prefixes.insert(prefix.clone(), iri.clone());
        self.declarations.push(PrefixDeclaration {
            prefix,
            iri,
            line: self.tokenizer.line_of(directive.start),
        });
        Ok(())
    }

    fn parse_base_directive(&mut self, expect_dot: bool) -> Result<(), ParseError> {
        let directive = self.current.clone();
        self.advance()?;
        if self.current.kind != TokenKind::Iri {
            return Err(self.error(format!("{} expects a '<...>' IRI.", directive.value)));
        }
        self.base = Some(self.resolve(&scanner::iri_value(&self.current.value)));
        self.base_line = Some(self.tokenizer.line_of(directive.start));
        self.advance()?;
        if expect_dot {
            self.expect(TokenKind::Dot, &format!("{} ends with a '.'", directive.value))?;
        }
        Ok(())
    }

    fn parse_statement(&mut self) -> Result<(), ParseError> {
        let statement_start = self.current.start;
        let mut pending = Vec::new();
        let subject_is_property_list = self.current.kind == TokenKind::OpenBracket;
        let subject = self.parse_subject(&mut pending)?;
        if !subject_is_property_list || self.current.kind != TokenKind::Dot {
            self.parse_predicate_object_list(&subject, &mut pending)?;
        }
        let dot = self.current.clone();
        self.expect(TokenKind::Dot, "a statement ends with a '.'")?;
        let statement = LineRange::new(
            self.tokenizer.line_of(statement_start),
            self.tokenizer.line_of(dot.start),
        );
        for triple in pending {
            let span = self.span_of(triple.start, triple.end);
            self.triples.push(RdfTriple {
                subject: triple.subject,
                predicate: triple.predicate,
                object: triple.object,
                span,
                statement,
                span_start: triple.start,
                span_end: triple.end,
                object_start: triple.object_start,
                object_end: triple.end,
                terminator_offset: dot.start,
            });
        }
        Ok(())
    }

    fn parse_subject(&mut self, pending: &mut Vec<Pending>) -> Result<RdfTerm, ParseError> {
        match self.current.kind {
            TokenKind::Iri | TokenKind::PrefixedName => Ok(RdfTerm::Iri(self.parse_iri_term()?)),
            TokenKind::BlankNodeLabel => Ok(RdfTerm::Blank(self.parse_labeled_blank()?)),
            TokenKind::OpenBracket => Ok(self.parse_blank_property_list(pending)?.0),
            TokenKind::OpenParen => Ok(self.parse_collection(pending)?.0),
            _ => Err(self.error(format!("Expected a subject, found '{}'.", self.current.value))),
        }
    }

    fn parse_predicate_object_list(&mut self, subject: &RdfTerm, pending: &mut Vec<Pending>) -> Result<(), ParseError> {
        loop {
            let verb_start = self.current.start;
            let verb = self.parse_verb()?;
            self.parse_object_list(subject, &verb, verb_start, pending)?;
            if self.current.kind != TokenKind::Semicolon {
                return Ok(());
            }
            while self.current.kind == TokenKind::Semicolon {
                self.advance()?;
            }
            if matches!(self.current.kind, TokenKind::Dot | TokenKind::CloseBracket) {
                return Ok(());
            }
        }
    }

    fn parse_verb(&mut self) -> Result<IriTerm, ParseError> {
        match self.current.kind {
            TokenKind::A => {
                self.advance()?;
                Ok(IriTerm::new(rdf::TYPE, "a"))
            }
            TokenKind::Iri | TokenKind::PrefixedName => self.parse_iri_term(),
            _ => Err(self.error(format!("Expected a predicate, found '{}'.", self.current.value))),
        }
    }

    fn parse_object_list(
        &mut self,
        subject: &RdfTerm,
        verb: &IriTerm,
        pair_start: usize,
        pending: &mut Vec<Pending>,
    ) -> Result<(), ParseError> {
        let mut first = true;
        loop {
            let object_start = self.current.start;
            let (term, end) = self.parse_object(pending)?;
            pending.push(Pending {
                subject: subject.clone(),
                predicate: verb.clone(),
                object: term,
                start: if first { pair_start } else { object_start },
                end,
                object_start,
            });
            if self.current.kind != TokenKind::Comma {
                return Ok(());
            }
            self.advance()?;
            first = false;
        }
    }

    fn parse_object(&mut self, pending: &mut Vec<Pending>) -> Result<(RdfTerm, usize), ParseError> {
        match self.current.kind {
            TokenKind::Iri | TokenKind::PrefixedName => {
                let end = self.current.end();
                Ok((RdfTerm::Iri(self.parse_iri_term()?), end))
            }
            TokenKind::BlankNodeLabel => {
                let end = self.current.end();
                Ok((RdfTerm::Blank(self.parse_labeled_blank()?), end))
            }
            TokenKind::String => self.parse_literal(),
            TokenKind::Number => {
                let token = self.current.clone();
                self.advance()?;
                let datatype = scanner::number_datatype(&token.value).to_owned();
                Ok((
                    RdfTerm::Literal(LiteralTerm {
                        lexical: token.value.clone(),
                        datatype: Some(datatype),
                        language: None,
                        as_written: token.value.clone(),
                    }),
                    token.end(),
                ))
            }
            TokenKind::Boolean => {
                let token = self.current.clone();
                self.advance()?;
                Ok((
                    RdfTerm::Literal(LiteralTerm {
                        lexical: token.value.clone(),
                        datatype: Some(rdf::XSD_BOOLEAN.to_owned()),
                        language: None,
                        as_written: token.value.clone(),
                    }),
                    token.end(),
                ))
            }
            TokenKind::OpenBracket => self.parse_blank_property_list(pending),
            TokenKind::OpenParen => self.parse_collection(pending),
            _ => Err(self.error(format!("Expected an object, found '{}'.", self.current.value))),
        }
    }

    fn parse_literal(&mut self) -> Result<(RdfTerm, usize), ParseError> {
        let string = self.current.clone();
        self.advance()?;
        if let Some((offset, message)) = scanner::escape_refusal(&string.value) {
            return Err(ParseError {
                message,
                line: self.tokenizer.line_of(string.start + offset) + 1,
            });
        }
        let lexical = scanner::string_value(&string.value);
        if self.current.kind == TokenKind::LanguageTag {
            let tag = self.current.clone();
            self.advance()?;
            let written = self.text[string.start..tag.end()].to_owned();
            return Ok((
                RdfTerm::Literal(LiteralTerm {
                    lexical,
                    datatype: Some(rdf::LANG_STRING.to_owned()),
                    language: Some(tag.value.clone()),
                    as_written: written,
                }),
                tag.end(),
            ));
        }
        if self.current.kind == TokenKind::DoubleCaret {
            self.advance()?;
            if !matches!(self.current.kind, TokenKind::Iri | TokenKind::PrefixedName) {
                return Err(self.error("'^^' expects a datatype IRI."));
            }
            let datatype_end = self.current.end();
            let datatype = self.parse_iri_term()?;
            let written = self.text[string.start..datatype_end].to_owned();
            return Ok((
                RdfTerm::Literal(LiteralTerm {
                    lexical,
                    datatype: Some(datatype.iri),
                    language: None,
                    as_written: written,
                }),
                datatype_end,
            ));
        }
        let end = string.end();
        Ok((
            RdfTerm::Literal(LiteralTerm {
                lexical,
                datatype: None,
                language: None,
                as_written: string.value,
            }),
            end,
        ))
    }

    fn parse_iri_term(&mut self) -> Result<IriTerm, ParseError> {
        let token = self.current.clone();
        if token.kind == TokenKind::Iri {
            self.advance()?;
            return Ok(IriTerm::new(
                self.resolve(&scanner::iri_value(&token.value)),
                token.value,
            ));
        }
        let colon = token.value.find(':').unwrap_or(token.value.len());
        let prefix = &token.value[..colon];
        let Some(expansion) = self.prefixes.get(prefix).cloned() else {
            return Err(self.error(format!("The prefix '{prefix}:' is not declared.")));
        };
        self.advance()?;
        let local = scanner::local_value(&token.value[colon + 1..]);
        Ok(IriTerm::new(format!("{expansion}{local}"), token.value))
    }

    fn parse_labeled_blank(&mut self) -> Result<BlankTerm, ParseError> {
        let label = self.current.value[2..].to_owned();
        self.advance()?;
        if let Some(existing) = self.labeled_blanks.get(&label) {
            return Ok(existing.clone());
        }
        let term = BlankTerm {
            label: Some(label.clone()),
            ordinal: self.blank_ordinal,
        };
        self.blank_ordinal += 1;
        self.labeled_blanks.insert(label, term.clone());
        Ok(term)
    }

    fn fresh_blank(&mut self) -> BlankTerm {
        let blank = BlankTerm {
            label: None,
            ordinal: self.blank_ordinal,
        };
        self.blank_ordinal += 1;
        blank
    }

    fn parse_blank_property_list(&mut self, pending: &mut Vec<Pending>) -> Result<(RdfTerm, usize), ParseError> {
        self.advance()?;
        let blank = RdfTerm::Blank(self.fresh_blank());
        if self.current.kind == TokenKind::CloseBracket {
            let end = self.current.end();
            self.advance()?;
            return Ok((blank, end));
        }
        self.parse_predicate_object_list(&blank, pending)?;
        let end = self.current.end();
        self.expect(TokenKind::CloseBracket, "a '[' property list ends with ']'")?;
        Ok((blank, end))
    }

    fn parse_collection(&mut self, pending: &mut Vec<Pending>) -> Result<(RdfTerm, usize), ParseError> {
        self.advance()?;
        if self.current.kind == TokenKind::CloseParen {
            let end = self.current.end();
            self.advance()?;
            return Ok((RdfTerm::iri(rdf::NIL, "()"), end));
        }
        let first = IriTerm::new(rdf::FIRST, "rdf:first");
        let rest = IriTerm::new(rdf::REST, "rdf:rest");
        let mut head: Option<BlankTerm> = None;
        let mut previous: Option<BlankTerm> = None;
        let mut previous_span = (0, 0);
        while self.current.kind != TokenKind::CloseParen {
            let node = self.fresh_blank();
            if head.is_none() {
                head = Some(node.clone());
            }
            let element_start = self.current.start;
            let (element, element_end) = self.parse_object(pending)?;
            if let Some(previous) = &previous {
                pending.push(Pending {
                    subject: RdfTerm::Blank(previous.clone()),
                    predicate: rest.clone(),
                    object: RdfTerm::Blank(node.clone()),
                    start: previous_span.0,
                    end: previous_span.1,
                    object_start: previous_span.0,
                });
            }
            pending.push(Pending {
                subject: RdfTerm::Blank(node.clone()),
                predicate: first.clone(),
                object: element,
                start: element_start,
                end: element_end,
                object_start: element_start,
            });
            previous = Some(node);
            previous_span = (element_start, element_end);
        }
        if let Some(previous) = &previous {
            pending.push(Pending {
                subject: RdfTerm::Blank(previous.clone()),
                predicate: rest,
                object: RdfTerm::iri(rdf::NIL, "rdf:nil"),
                start: previous_span.0,
                end: previous_span.1,
                object_start: previous_span.0,
            });
        }
        let end = self.current.end();
        self.advance()?;
        Ok((
            RdfTerm::Blank(head.expect("a collection with a member has a head")),
            end,
        ))
    }

    fn span_of(&self, start: usize, end: usize) -> SourceSpan {
        let start_line = self.tokenizer.line_of(start);
        let end_line = self.tokenizer.line_of(end.saturating_sub(1));
        if start_line != end_line {
            return SourceSpan {
                start_line,
                end_line,
                fragment_start: None,
                fragment_end: None,
            };
        }
        let line_start = self.tokenizer.start_of_line(start_line);
        let line_text = &self.document.lines()[start_line].text;
        let start_column = start - line_start;
        let end_column = (end - line_start).min(line_text.len());
        let owns = !substantive_before(&line_text[..start_column]) && !substantive_after(&line_text[end_column..]);
        if owns {
            SourceSpan {
                start_line,
                end_line,
                fragment_start: None,
                fragment_end: None,
            }
        } else {
            SourceSpan {
                start_line,
                end_line,
                fragment_start: Some(start_column),
                fragment_end: Some(end_column),
            }
        }
    }

    fn resolve(&self, iri: &str) -> String {
        if iri.is_empty() {
            return self.base.clone().unwrap_or_default();
        }
        let Some(base) = &self.base else {
            return iri.to_owned();
        };
        if iri::is_absolute(iri) {
            return iri.to_owned();
        }
        iri::resolve(base, iri).unwrap_or_else(|| iri.to_owned())
    }
}

/// Whether the text before a triple's tokens on its line is more than whitespace and list separators.
fn substantive_before(before: &str) -> bool {
    before.chars().any(|c| !c.is_whitespace() && c != ',' && c != ';')
}

/// Whether the text after a triple's tokens on its line is more than whitespace, separators, the terminator and a
/// trailing comment.
fn substantive_after(after: &str) -> bool {
    let mut remaining = after.trim_start();
    while let Some(rest) = remaining.strip_prefix([',', ';', '.']) {
        remaining = rest.trim_start();
    }
    !remaining.is_empty() && !remaining.starts_with('#')
}
