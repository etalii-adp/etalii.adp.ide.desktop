//! Parsing ATD with `etalii-lark`'s interactive LALR(1) parser over its contextual lexer (ATD §3.6; spec 035, research
//! R9 as amended on 2026-10-24, R10): a lossless tree, every byte of the file a token, and recovery from a syntax error
//! at the next construct, driven through the interactive parser.
//!
//! The parser is fed one token at a time, each lexed for the state it is in. On an error the parse is recovered as
//! research R10 says: a missing `}` at the end of the file is assumed where a state on the stack reads it; otherwise
//! the text is skipped to the next construct start (`}` or a keyword) that a state on the stack reads, and the stack
//! is popped to that state. What was popped and skipped stays in the file's tokens, as skipped text; the text between
//! the tokens is lexed as the ignored terminals (whitespace and comments), so that the tokens are the file.

use std::collections::HashMap;

use etalii_lark::{Error as LarkError, InteractiveParser, Node as LarkNode, Token as LarkToken, Tree as LarkTree};

use crate::tables::{Language, language, terminals};
use crate::text::LineIndex;

/// What a token is to the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    /// A token the parser read.
    Normal,
    /// Whitespace or a comment.
    Trivia,
    /// Text skipped while recovering from a syntax error.
    Skipped,
    /// A zero-width token the parser assumed while recovering (a missing `}` at the end of the file).
    Missing,
}

/// A token: its terminal (an index into [`crate::tables::terminals`], `u16::MAX` for text no terminal matches) and its
/// byte range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub terminal: u16,
    pub start: usize,
    pub end: usize,
}

impl Token {
    pub fn text<'a>(&self, source: &'a str) -> &'a str {
        &source[self.start..self.end]
    }

    pub fn name(&self) -> &'static str {
        terminals().get(self.terminal as usize).map(|t| t.name).unwrap_or("?")
    }

    pub fn is_keyword(&self) -> bool {
        terminals().get(self.terminal as usize).is_some_and(|t| t.keyword)
    }
}

/// A child of a node: a token, by its index in [`Parse::tokens`], or a node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Child {
    Token(usize),
    Node(Node),
}

/// A node of the lossless tree: the rule that built it (its alias, else its name) and its children.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub name: &'static str,
    /// Its kind, an index into the language's kinds.
    pub rule: usize,
    pub children: Vec<Child>,
    /// The byte range of its first and last token, trivia excluded.
    pub start: usize,
    pub end: usize,
}

impl Node {
    /// The node's direct child nodes.
    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        self.children.iter().filter_map(|c| match c {
            Child::Node(n) => Some(n),
            Child::Token(_) => None,
        })
    }

    /// The node's direct tokens, by index.
    pub fn tokens(&self) -> impl Iterator<Item = usize> + '_ {
        self.children.iter().filter_map(|c| match c {
            Child::Token(t) => Some(*t),
            Child::Node(_) => None,
        })
    }

    /// The node's name without the module's prefix (`core__attribute` is `attribute`).
    pub fn kind(&self) -> &'static str {
        self.name.rsplit("__").next().unwrap_or(self.name)
    }

    /// Whether the rule that built it is a `?rule`, which Lark inlines when it has one child.
    pub fn inline(&self) -> bool {
        language().kinds.get(self.rule).is_some_and(|k| k.inline)
    }
}

/// A syntax error: where, and what was found and expected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxError {
    pub start: usize,
    pub end: usize,
    pub message: String,
}

/// A parsed file.
#[derive(Debug, Clone)]
pub struct Parse {
    pub text: String,
    /// Every token of the file, trivia and skipped text included, in order: their texts are the file.
    pub tokens: Vec<Token>,
    /// The tree, when the file parsed far enough to have a description.
    pub root: Option<Node>,
    pub errors: Vec<SyntaxError>,
}

impl Parse {
    /// The texts of the tokens, joined: the file (lossless, research R10).
    pub fn source(&self) -> String {
        self.tokens.iter().map(|t| t.text(&self.text)).collect()
    }

    pub fn token_text(&self, index: usize) -> &str {
        self.tokens[index].text(&self.text)
    }
}

/// Parses a file. A file that does not start with a header of a version this reader knows is not parsed (ATD §5.4):
/// the caller checks the version first.
pub fn parse(text: &str) -> Parse {
    let mut run = Run::new(text, None);
    let root = run.run();
    run.finish(root)
}

/// What the grammar admits at `offset`: the terminals the parser's state there reads, the text before it parsed with
/// recovery (completion, spec 035 research R10). A word the offset stands in or at the end of is not read.
pub fn expected_at(text: &str, offset: usize) -> Vec<&'static str> {
    let mut start = offset.min(text.len());
    while start > 0 {
        let c = text[..start].chars().next_back().unwrap_or(' ');
        if c.is_alphanumeric() || c == '_' {
            start -= c.len_utf8();
        } else {
            break;
        }
    }
    let mut run = Run::new(text, Some(start));
    let _ = run.run();
    let language = language();
    let mut names: Vec<&'static str> = run
        .accepts
        .iter()
        .filter_map(|name| language.terminal_of_lark(name))
        .filter(|t| !terminals()[*t as usize].ignored)
        .map(|t| terminals()[t as usize].name)
        .collect();
    names.sort();
    names.dedup();
    names
}

/// One run of the parser over a file.
struct Run<'a> {
    text: &'a str,
    language: &'static Language,
    lines: LineIndex,
    /// Where to stop reading, for [`expected_at`]; the terminals the state there accepts are kept.
    stop: Option<usize>,
    accepts: Vec<String>,
    tokens: Vec<Token>,
    errors: Vec<SyntaxError>,
    /// Tokens to read after a recovery before another error is recorded, so that one error gives one finding.
    quiet: usize,
}

/// A token the recovery skips or stops at: its terminal (`u16::MAX` when none matches) and its range.
#[derive(Debug, Clone, Copy)]
struct Seen {
    terminal: u16,
    start: usize,
    end: usize,
}

impl<'a> Run<'a> {
    fn new(text: &'a str, stop: Option<usize>) -> Self {
        Run {
            text,
            language: language(),
            lines: LineIndex::new(text),
            stop,
            accepts: Vec::new(),
            tokens: Vec::new(),
            errors: Vec::new(),
            quiet: 0,
        }
    }

    fn run(&mut self) -> Option<LarkNode> {
        let language = self.language;
        let mut parser = language.lark.parse_interactive(self.text, None).ok()?;
        if self.text.starts_with('\u{feff}') {
            parser.skip_chars(1);
            self.push(TokenKind::Trivia, u16::MAX, 0, 3);
        }
        let mut steps = 0usize;
        loop {
            steps += 1;
            if steps > 50 * (self.text.len() + 16) {
                return None;
            }
            match parser.next_token() {
                Ok(Some(token)) => {
                    let (start, end) = (token.start_pos.unwrap_or(0), token.end_pos.unwrap_or(0));
                    if self.stop.is_some_and(|stop| start >= stop) {
                        self.accepts = parser.accepts().into_iter().collect();
                        return None;
                    }
                    let terminal = language.terminal_of_lark(&token.kind).unwrap_or(u16::MAX);
                    match parser.feed_token(token) {
                        Ok(_) => {
                            self.push(TokenKind::Normal, terminal, start, end);
                            self.quiet = self.quiet.saturating_sub(1);
                        }
                        Err(_) => {
                            let found = Seen { terminal, start, end };
                            if !self.recover(&mut parser, Some(found)) {
                                return None;
                            }
                        }
                    }
                }
                Ok(None) => {
                    if self.stop.is_some() {
                        self.accepts = parser.accepts().into_iter().collect();
                        return None;
                    }
                    match parser.feed_eof(None) {
                        Ok(root) => return root,
                        Err(_) => {
                            if !self.recover(&mut parser, None) {
                                return None;
                            }
                        }
                    }
                }
                Err(error) => {
                    // Text this state does not read: the token the grammar would read there, or else the text to the
                    // next whitespace. The lexer has read it, or it has not, as Lark's lexer goes.
                    let at = match &error {
                        LarkError::UnexpectedToken(e) => e.token.start_pos.unwrap_or(0),
                        LarkError::UnexpectedCharacters(e) => e.pos_in_stream,
                        _ => return None,
                    };
                    if self.stop.is_some_and(|stop| at >= stop) {
                        self.accepts = parser.accepts().into_iter().collect();
                        return None;
                    }
                    let found = self.lex_skipping(at).unwrap_or(Seen {
                        terminal: u16::MAX,
                        start: at,
                        end: self.text.len(),
                    });
                    self.advance(&mut parser, found.end);
                    if !self.recover(&mut parser, Some(found)) {
                        return None;
                    }
                }
            }
        }
    }

    fn push(&mut self, kind: TokenKind, terminal: u16, start: usize, end: usize) {
        self.tokens.push(Token {
            kind,
            terminal,
            start,
            end,
        });
    }

    /// Moves the lexer to `offset`, when it is before it.
    fn advance(&self, parser: &mut InteractiveParser<'_>, offset: usize) {
        let at = parser.lexer_position();
        if offset > at {
            parser.skip_chars(self.text[at..offset].chars().count());
        }
    }

    fn expected(&self, parser: &InteractiveParser<'_>) -> Vec<&'static str> {
        let language = self.language;
        let mut names: Vec<&'static str> = parser
            .choices()
            .keys()
            .filter_map(|name| {
                if name == "$END" {
                    Some("the end of the file")
                } else {
                    language.terminal_of_lark(name).map(describe)
                }
            })
            .collect();
        names.sort();
        names.dedup();
        names
    }

    /// Recovers from a syntax error at `found`, or at the end of the file (research R10): records it, then assumes a
    /// missing `}` at the end of the file, or skips to the next construct start some state on the stack reads, popping
    /// to that state. The popped and skipped tokens are kept, as skipped text.
    fn recover(&mut self, parser: &mut InteractiveParser<'_>, found: Option<Seen>) -> bool {
        let (start, end, text) = match found {
            Some(seen) => (seen.start, seen.end, format!("`{}`", &self.text[seen.start..seen.end])),
            None => (self.text.len(), self.text.len(), "the end of the file".to_owned()),
        };
        let expected = self.expected(parser);
        let first_error_here = self.errors.last().is_none_or(|e| e.start != start);
        if first_error_here && self.quiet == 0 {
            self.errors.push(SyntaxError {
                start,
                end,
                message: format!("Found {text} where {} expected.", list(&expected)),
            });
        }
        self.quiet = 3;
        let language = self.language;
        let close = language.terminal("\"}\"").expect("the grammar has `}`");
        let close_lark = terminals()[close as usize].lark;
        let Some(mut current) = found else {
            // A missing `}`: assume it, where a state on the stack can read it.
            let Some(depth) = self.deepest_reading(parser, close_lark) else {
                return false;
            };
            self.discard_to(parser, depth);
            let at = self.text.len();
            if parser.feed_token(self.lark_token(close_lark, at, at)).is_err() {
                return false;
            }
            self.push(TokenKind::Missing, close, at, at);
            return true;
        };
        // Skip to a construct start or `}` that a state on the stack reads.
        let mut skipped_first = false;
        // Braces opened in the skipped text close in it too: their `}` is never taken for the construct's own.
        let mut braces = 0usize;
        loop {
            let text = &self.text[current.start..current.end];
            if braces > 0 || text == "{" {
                if text == "{" {
                    braces += 1;
                } else if text == "}" {
                    braces -= 1;
                }
            } else if current.terminal != u16::MAX && synchronising(current.terminal) {
                let lark = terminals()[current.terminal as usize].lark;
                let errors_here = self.errors.last().is_some_and(|e| e.start == current.start);
                if (skipped_first || !errors_here)
                    && let Some(depth) = self.deepest_reading(parser, lark)
                    && (depth + 1 < parser.parser_state().depth() || skipped_first)
                {
                    self.discard_to(parser, depth);
                    self.advance(parser, current.end);
                    if parser
                        .feed_token(self.lark_token(lark, current.start, current.end))
                        .is_ok()
                    {
                        self.push(TokenKind::Normal, current.terminal, current.start, current.end);
                        return true;
                    }
                    return self.recover(parser, Some(current));
                }
            }
            self.push(TokenKind::Skipped, current.terminal, current.start, current.end);
            skipped_first = true;
            match self.lex_skipping(current.end) {
                Some(next) => {
                    self.advance(parser, next.end);
                    current = next;
                }
                None => {
                    self.advance(parser, self.text.len());
                    return true;
                }
            }
        }
    }

    /// The deepest index on the parser's stack whose state reads Lark's terminal `name`.
    fn deepest_reading(&self, parser: &InteractiveParser<'_>, name: &str) -> Option<usize> {
        let stack = parser.parser_state().state_stack();
        (0..stack.len()).rev().find(|d| self.language.reads(stack[*d], name))
    }

    /// Pops the stack to `depth`, keeping the popped tokens as skipped text.
    fn discard_to(&mut self, parser: &mut InteractiveParser<'_>, depth: usize) {
        let popped = parser.parser_state_mut().truncate(depth + 1);
        for value in &popped {
            self.mark_skipped(value);
        }
    }

    fn mark_skipped(&mut self, node: &LarkNode) {
        match node {
            LarkNode::Token(token) => {
                let start = token.start_pos.unwrap_or(0);
                if let Some(t) = self
                    .tokens
                    .iter_mut()
                    .rev()
                    .find(|t| t.start == start && t.kind == TokenKind::Normal)
                {
                    t.kind = TokenKind::Skipped;
                }
            }
            LarkNode::Tree(tree) => {
                for child in &tree.children {
                    self.mark_skipped(child);
                }
            }
            LarkNode::Missing => {}
        }
    }

    /// The next token at or after `offset`, lexed as at the start of a construct while skipping, the whitespace and
    /// comments before it kept as trivia; a word that spells a keyword is that keyword. None at the end of the file.
    fn lex_skipping(&mut self, mut offset: usize) -> Option<Seen> {
        let language = self.language;
        let lexer = &language.lexer;
        let starts: Vec<usize> = (0..terminals().len())
            .filter(|t| language.reads(language.start_state, terminals()[*t].lark))
            .collect();
        loop {
            if offset >= self.text.len() {
                return None;
            }
            let lexeme = lexer.next(self.text, offset, &starts).or_else(|| {
                let all: Vec<usize> = (0..terminals().len()).collect();
                lexer.next(self.text, offset, &all)
            });
            let (terminal, end) = match lexeme {
                Some(lexeme) if lexeme.end > offset => (lexeme.terminal as u16, lexeme.end),
                _ => {
                    let rest = &self.text[offset..];
                    let len = rest
                        .char_indices()
                        .find(|(_, c)| c.is_whitespace())
                        .map(|(i, _)| i)
                        .unwrap_or(rest.len())
                        .max(rest.chars().next().map(char::len_utf8).unwrap_or(1));
                    (u16::MAX, offset + len)
                }
            };
            if terminal != u16::MAX && terminals()[terminal as usize].ignored {
                self.push(TokenKind::Trivia, terminal, offset, end);
                offset = end;
                continue;
            }
            let text = &self.text[offset..end];
            let terminal = terminals()
                .iter()
                .position(|t| t.keyword && t.value == text)
                .map(|k| k as u16)
                .or_else(|| (text == "}").then(|| language.terminal("\"}\"")).flatten())
                .unwrap_or(terminal);
            return Some(Seen {
                terminal,
                start: offset,
                end,
            });
        }
    }

    /// A token of Lark's terminal `name` over the text from `start` to `end`, positioned as Lark positions it.
    fn lark_token(&self, name: &str, start: usize, end: usize) -> LarkToken {
        let (line, column) = self.lines.position(self.text, start);
        let (end_line, end_column) = self.lines.position(self.text, end);
        let mut token = LarkToken::new(name, &self.text[start..end]);
        token.start_pos = Some(start);
        token.end_pos = Some(end);
        token.line = Some(line);
        token.column = Some(column);
        token.end_line = Some(end_line);
        token.end_column = Some(end_column);
        token
    }

    /// The parse: the tokens in order with the text between them lexed as trivia, and the tree.
    fn finish(mut self, root: Option<LarkNode>) -> Parse {
        let mut tokens = std::mem::take(&mut self.tokens);
        tokens.sort_by_key(|t| (t.start, t.end));
        let mut all: Vec<Token> = Vec::with_capacity(tokens.len() * 2);
        let mut at = 0;
        for token in tokens {
            if token.start > at {
                self.trivia(at, token.start, &mut all);
            }
            at = at.max(token.end);
            all.push(token);
        }
        if at < self.text.len() {
            self.trivia(at, self.text.len(), &mut all);
        }
        let index: HashMap<(usize, usize), usize> = all
            .iter()
            .enumerate()
            .filter(|(_, t)| matches!(t.kind, TokenKind::Normal | TokenKind::Missing))
            .map(|(i, t)| ((t.start, t.end), i))
            .collect();
        let root = match root {
            Some(LarkNode::Tree(tree)) => Some(node(&tree, &index, &all)),
            _ => None,
        };
        Parse {
            text: self.text.to_owned(),
            tokens: all,
            root,
            errors: self.errors,
        }
    }

    /// The text from `start` to `end`, between tokens, as the ignored terminals lex it; what they do not lex is
    /// skipped text.
    fn trivia(&self, mut start: usize, end: usize, out: &mut Vec<Token>) {
        let lexer = &self.language.lexer;
        while start < end {
            match lexer.next(&self.text[..end], start, &[]) {
                Some(lexeme) if lexeme.end > start && terminals()[lexeme.terminal].ignored => {
                    out.push(Token {
                        kind: TokenKind::Trivia,
                        terminal: lexeme.terminal as u16,
                        start,
                        end: lexeme.end,
                    });
                    start = lexeme.end;
                }
                _ => {
                    let next = (start + 1..=end)
                        .find(|i| self.text.is_char_boundary(*i))
                        .unwrap_or(end);
                    match out.last_mut() {
                        Some(last) if last.kind == TokenKind::Skipped && last.end == start => last.end = next,
                        _ => out.push(Token {
                            kind: TokenKind::Skipped,
                            terminal: u16::MAX,
                            start,
                            end: next,
                        }),
                    }
                    start = next;
                }
            }
        }
    }
}

/// Lark's tree as the lossless tree: each token by its index in the file's tokens.
fn node(tree: &LarkTree, index: &HashMap<(usize, usize), usize>, tokens: &[Token]) -> Node {
    let language = language();
    let children: Vec<Child> = tree
        .children
        .iter()
        .filter_map(|child| match child {
            LarkNode::Tree(t) => Some(Child::Node(node(t, index, tokens))),
            LarkNode::Token(t) => index
                .get(&(t.start_pos.unwrap_or(0), t.end_pos.unwrap_or(0)))
                .map(|i| Child::Token(*i)),
            LarkNode::Missing => None,
        })
        .collect();
    let range = |child: &Child| match child {
        Child::Token(t) => Some((tokens[*t].start, tokens[*t].end)),
        Child::Node(n) if n.start < n.end || !n.children.is_empty() => Some((n.start, n.end)),
        Child::Node(_) => None,
    };
    let first = children.iter().find_map(range);
    let last = children.iter().rev().find_map(range);
    let (start, end) = match (first, last) {
        (Some((s, _)), Some((_, e))) => (s, e),
        _ => (0, 0),
    };
    let rule = language.kind(&tree.data).unwrap_or(usize::MAX);
    let name = language
        .kinds
        .get(rule)
        .map(|k| k.name)
        .unwrap_or_else(|| Box::leak(tree.data.clone().into_boxed_str()));
    Node {
        name,
        rule,
        children,
        start,
        end,
    }
}

/// The construct starts recovery stops at: `}` and every keyword that starts a construct.
fn synchronising(terminal: u16) -> bool {
    let info = &terminals()[terminal as usize];
    info.name == "\"}\"" || info.keyword
}

/// How a finding names a terminal it expected.
pub fn describe(terminal: u16) -> &'static str {
    match terminals()[terminal as usize].name {
        "NAME" => "a name",
        "KEY" => "a setting (`key:`)",
        "ASSIGN" => "an assignment (`name =`)",
        "STRING" => "a string",
        "INT" => "a whole number",
        "UINT" => "an unsigned number",
        "DOUBLE" => "a number",
        "LENGTH" => "a length",
        "ATTR_REF" => "an attribute reference (`@name`)",
        "TOKEN_REF" => "a token reference (`$name`)",
        "PARAM_REF" => "a parameter reference (`#name`)",
        "ORIGIN" => "an origin (`vendor/type`)",
        "CAPABILITY_RANGE" => "a version range (`@^1`)",
        "VERSION" => "a version (`0.1`)",
        "DASHED_WORD" => "a word",
        "BYTES" => "bytes",
        name => name,
    }
}

fn list(expected: &[&str]) -> String {
    let shown: Vec<&str> = expected.iter().copied().take(8).collect();
    let mut text = match shown.as_slice() {
        [] => "nothing".to_owned(),
        [one] => format!("{one} was"),
        [rest @ .., last] => format!("{} or {last} was", rest.join(", ")),
    };
    if expected.len() > shown.len() {
        text = text.replace(
            " was",
            &format!(" (or one of {} more) was", expected.len() - shown.len()),
        );
    }
    text.replace("\"", "`").replace("`` ", "` ")
}
