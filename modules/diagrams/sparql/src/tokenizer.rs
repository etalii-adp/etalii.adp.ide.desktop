//! Cuts a query's text into tokens, each addressed by its exact character slice (etalii.adp spec 030, task T084).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/sparql/backend/EtAlii.Adp.Diagram.Sparql/
//! SparqlTokenizer.cs`, `_Model/SparqlToken.cs` and `_Model/SparqlTokenKind.cs`. Positions serve two purposes only,
//! reporting error lines and slicing expression text as written, because nothing in this module ever writes the query
//! back. An IRI is the one terminal read here rather than in [`crate::terminals`]: in a query `<` is also less-than,
//! so an IRI ends at the first whitespace or quote.

use crate::parser::ParseError;
use crate::terminals::{self, LineIndex, TerminalProblem};

/// The kinds of token a query is made of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    /// A bracketed IRI, `<...>`.
    Iri,
    /// A prefixed name, `foaf:name` or `:thing`.
    PrefixedName,
    /// `?name` or `$name`.
    Variable,
    /// `_:name`.
    BlankNodeLabel,
    String,
    /// `@en`.
    LanguageTag,
    Number,
    /// `true` or `false`.
    Boolean,
    /// A bare word: a keyword, the verb `a`, or a built-in call's name.
    Name,
    /// Punctuation and operators, exactly as written.
    Punct,
    EndOfFile,
}

/// One token: its kind, its slice of the text and its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub start: usize,
    pub length: usize,
    pub value: String,
}

impl Token {
    /// Offset just past the token's last character.
    pub fn end(&self) -> usize {
        self.start + self.length
    }

    /// Whether this token is the bare word `keyword`, compared without regard to case.
    pub fn is_keyword(&self, keyword: &str) -> bool {
        self.kind == TokenKind::Name && self.value.eq_ignore_ascii_case(keyword)
    }

    /// Whether this token is the punctuation `punctuation`.
    pub fn is_punct(&self, punctuation: &str) -> bool {
        self.kind == TokenKind::Punct && self.value == punctuation
    }
}

/// The tokenizer over one text.
pub struct Tokenizer<'a> {
    text: &'a [char],
    lines: LineIndex,
    position: usize,
}

fn is_name_start_char(character: char) -> bool {
    terminals::is_letter(character) || character == '_'
}

impl<'a> Tokenizer<'a> {
    pub fn new(text: &'a [char]) -> Self {
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

    /// Every token in the document, in order, ending with [`TokenKind::EndOfFile`].
    pub fn tokenize(&mut self) -> Result<Vec<Token>, ParseError> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next()?;
            let end = token.kind == TokenKind::EndOfFile;
            tokens.push(token);
            if end {
                return Ok(tokens);
            }
        }
    }

    fn at(&self, offset: usize) -> Option<char> {
        self.text.get(offset).copied()
    }

    fn next(&mut self) -> Result<Token, ParseError> {
        self.position = terminals::skip_trivia(self.text, self.position);
        if self.position >= self.text.len() {
            return Ok(Token {
                kind: TokenKind::EndOfFile,
                start: self.position,
                length: 0,
                value: String::new(),
            });
        }
        let start = self.position;
        let character = self.text[start];
        match character {
            '<' => {
                return Ok(match self.try_read_iri(start) {
                    Some(iri) => iri,
                    None => self.read_operator(start)?,
                });
            }
            '"' | '\'' => return self.read_string(start),
            '?' | '$' => {
                if self.at(start + 1).is_some_and(terminals::is_name_char) {
                    return Ok(self.read_variable(start));
                }
                self.position += 1;
                return Ok(self.slice(TokenKind::Punct, start));
            }
            '@' => return self.read_language_tag(start),
            '_' => {
                if self.at(start + 1) == Some(':') {
                    self.position = terminals::local_name(self.text, start + 2);
                    return Ok(self.slice(TokenKind::BlankNodeLabel, start));
                }
                return Ok(self.read_name_or_prefixed_name(start));
            }
            '.' => {
                if self.at(start + 1).is_some_and(|c| c.is_ascii_digit()) {
                    self.position = terminals::number(self.text, start);
                    return Ok(self.slice(TokenKind::Number, start));
                }
                self.position += 1;
                return Ok(self.slice(TokenKind::Punct, start));
            }
            _ => {}
        }
        if character.is_ascii_digit() {
            // A sign is never read here: in a query '+' and '-' are operators first.
            self.position = terminals::number(self.text, start);
            return Ok(self.slice(TokenKind::Number, start));
        }
        if is_name_start_char(character) {
            Ok(self.read_name_or_prefixed_name(start))
        } else {
            self.read_operator(start)
        }
    }

    fn try_read_iri(&mut self, start: usize) -> Option<Token> {
        let mut position = start + 1;
        while position < self.text.len() {
            let character = self.text[position];
            if character == '>' {
                self.position = position + 1;
                return Some(self.slice(TokenKind::Iri, start));
            }
            if matches!(character, '<' | '"' | '\'' | '{' | '}' | '|' | '^' | '`') || terminals::is_white_space(character)
            {
                break;
            }
            position += 1;
        }
        None
    }

    fn read_string(&mut self, start: usize) -> Result<Token, ParseError> {
        let (end, problem) = terminals::string(self.text, start);
        self.position = end;
        match problem {
            TerminalProblem::None => Ok(self.slice(TokenKind::String, start)),
            TerminalProblem::LineEnded => Err(self.error("The string that starts here is not closed on its line.", start)),
            TerminalProblem::FileEnded => Err(self.error("The string that starts here is never closed.", start)),
        }
    }

    fn read_variable(&mut self, start: usize) -> Token {
        self.position += 1;
        while self.at(self.position).is_some_and(terminals::is_name_char) {
            self.position += 1;
        }
        self.slice(TokenKind::Variable, start)
    }

    fn read_language_tag(&mut self, start: usize) -> Result<Token, ParseError> {
        self.position += 1;
        while self
            .at(self.position)
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            self.position += 1;
        }
        if self.position == start + 1 {
            Err(self.error(
                "A lone '@' is not a SPARQL token; a language tag is '@' followed by letters.",
                start,
            ))
        } else {
            Ok(self.slice(TokenKind::LanguageTag, start))
        }
    }

    fn read_name_or_prefixed_name(&mut self, start: usize) -> Token {
        while self.at(self.position).is_some_and(terminals::is_name_char) {
            self.position += 1;
        }
        if self.at(self.position) == Some(':') {
            self.position = terminals::local_name(self.text, self.position + 1);
            return self.slice(TokenKind::PrefixedName, start);
        }
        let token = self.slice(TokenKind::Name, start);
        if token.value == "true" || token.value == "false" {
            Token {
                kind: TokenKind::Boolean,
                ..token
            }
        } else {
            token
        }
    }

    fn read_operator(&mut self, start: usize) -> Result<Token, ParseError> {
        let character = self.text[self.position];
        if character == ':' {
            // The default prefix's bare form, ':thing', or a lone ':' the parser will refuse in context.
            self.position = terminals::local_name(self.text, self.position + 1);
            return Ok(self.slice(TokenKind::PrefixedName, start));
        }
        // Two-character operators first, so '<=' never tokenizes as '<' '='.
        if self.position + 1 < self.text.len() {
            let pair: String = self.text[self.position..self.position + 2].iter().collect();
            if matches!(pair.as_str(), "^^" | "<=" | ">=" | "!=" | "&&" | "||") {
                self.position += 2;
                return Ok(self.slice(TokenKind::Punct, start));
            }
        }
        if matches!(
            character,
            '{' | '}'
                | '('
                | ')'
                | '['
                | ']'
                | ';'
                | ','
                | '|'
                | '/'
                | '^'
                | '*'
                | '+'
                | '!'
                | '='
                | '<'
                | '>'
                | '-'
                | '?'
                | '$'
        ) {
            self.position += 1;
            return Ok(self.slice(TokenKind::Punct, start));
        }
        Err(self.error(&format!("'{character}' is not a SPARQL token."), start))
    }

    fn slice(&self, kind: TokenKind, start: usize) -> Token {
        Token {
            kind,
            start,
            length: self.position - start,
            value: self.text[start..self.position].iter().collect(),
        }
    }

    fn error(&self, message: &str, offset: usize) -> ParseError {
        ParseError {
            message: message.to_owned(),
            line: self.line_of(offset) + 1,
        }
    }
}
