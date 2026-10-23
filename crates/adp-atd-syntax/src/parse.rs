//! Lexing and parsing with ATD's tables: a lossless tree, every byte of the file a token, and recovery from a syntax
//! error at the next construct (ATD §3.6; spec 035, research R10).

use crate::tables::{self, END, RULES, TERMINALS};

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

/// A token: its terminal (an index into [`tables::TERMINALS`], `u16::MAX` for text no terminal matches) and its byte
/// range.
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
        TERMINALS.get(self.terminal as usize).map(|t| t.name).unwrap_or("?")
    }

    pub fn is_keyword(&self) -> bool {
        TERMINALS.get(self.terminal as usize).is_some_and(|t| t.keyword)
    }
}

/// A child of a node: a token, by its index in [`Parse::tokens`], or a node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Child {
    Token(usize),
    Node(Node),
}

/// A node of the lossless tree: the rule that built it (its alias, else its left-hand side) and its children, the
/// items of the repetitions it holds spliced in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub name: &'static str,
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

    /// Whether the rule that built it is a `?rule` that has this one child node and no other tokens but filtered ones.
    pub fn inline(&self) -> bool {
        RULES[self.rule].inline
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

/// The construct starts recovery stops at: `}` and every keyword that starts a construct.
fn synchronising(terminal: u16) -> bool {
    let info = &TERMINALS[terminal as usize];
    info.name == "\"}\"" || info.keyword
}

/// Parses a file. A file that does not start with a header of a version this reader knows is not parsed (ATD §5.4):
/// the caller checks the version first.
pub fn parse(text: &str) -> Parse {
    let mut parser = Parser {
        text,
        position: if text.starts_with('\u{feff}') { 3 } else { 0 },
        tokens: Vec::new(),
        states: vec![0],
        values: Vec::new(),
        errors: Vec::new(),
    };
    if parser.position > 0 {
        parser.tokens.push(Token {
            kind: TokenKind::Trivia,
            terminal: u16::MAX,
            start: 0,
            end: 3,
        });
    }
    let root = parser.run();
    Parse {
        text: text.to_owned(),
        tokens: parser.tokens,
        root,
        errors: parser.errors,
    }
}

struct Parser<'a> {
    text: &'a str,
    position: usize,
    tokens: Vec<Token>,
    states: Vec<usize>,
    values: Vec<Child>,
    errors: Vec<SyntaxError>,
}

impl Parser<'_> {
    fn state(&self) -> usize {
        *self.states.last().expect("the stack holds the start state")
    }

    /// The next token the current state can read, the trivia before it pushed; `END` at the end of the file. Text
    /// no candidate matches is one skipped token, ended at the next whitespace.
    fn lex(&mut self) -> Option<usize> {
        loop {
            if self.position >= self.text.len() {
                return None;
            }
            let rest = &self.text[self.position..];
            let state = self.state();
            let mut best: Option<(u16, usize)> = None;
            let consider = |terminal: u16, best: &mut Option<(u16, usize)>| {
                let Some(len) = tables::match_len(terminal, rest) else {
                    return;
                };
                let info = &TERMINALS[terminal as usize];
                let better = match *best {
                    None => true,
                    Some((current, current_len)) => {
                        let other = &TERMINALS[current as usize];
                        len > current_len
                            || (len == current_len
                                && (info.priority > other.priority
                                    || (info.priority == other.priority && !info.pattern && other.pattern)))
                    }
                };
                if better {
                    *best = Some((terminal, len));
                }
            };
            for terminal in tables::acceptable(state) {
                if terminal != END {
                    consider(terminal, &mut best);
                }
            }
            for (index, info) in TERMINALS.iter().enumerate() {
                if info.ignored {
                    consider(index as u16, &mut best);
                }
            }
            match best {
                Some((terminal, len)) if TERMINALS[terminal as usize].ignored => {
                    self.tokens.push(Token {
                        kind: TokenKind::Trivia,
                        terminal,
                        start: self.position,
                        end: self.position + len,
                    });
                    self.position += len;
                }
                Some((terminal, len)) => {
                    self.tokens.push(Token {
                        kind: TokenKind::Normal,
                        terminal,
                        start: self.position,
                        end: self.position + len,
                    });
                    self.position += len;
                    return Some(self.tokens.len() - 1);
                }
                None => {
                    // Nothing this state reads: any terminal of the grammar, else the text to the next whitespace.
                    let any = (0..TERMINALS.len() as u16)
                        .filter_map(|t| tables::match_len(t, rest).map(|len| (t, len)))
                        .max_by_key(|(_, len)| *len);
                    let (terminal, len) = any.unwrap_or_else(|| {
                        let len = rest
                            .char_indices()
                            .find(|(_, c)| c.is_whitespace())
                            .map(|(i, _)| i)
                            .unwrap_or(rest.len())
                            .max(rest.chars().next().map(char::len_utf8).unwrap_or(1));
                        (u16::MAX, len)
                    });
                    self.tokens.push(Token {
                        kind: TokenKind::Normal,
                        terminal,
                        start: self.position,
                        end: self.position + len,
                    });
                    self.position += len;
                    return Some(self.tokens.len() - 1);
                }
            }
        }
    }

    fn run(&mut self) -> Option<Node> {
        let mut lookahead = self.lex();
        let mut steps = 0usize;
        loop {
            steps += 1;
            if steps > 50 * (self.text.len() + 16) {
                return None;
            }
            let terminal = lookahead.map(|t| self.tokens[t].terminal).unwrap_or(END);
            match tables::action(self.state(), terminal) {
                Some(i32::MIN) => {
                    return match self.values.pop() {
                        Some(Child::Node(node)) => Some(node),
                        _ => None,
                    };
                }
                Some(code) if code >= 0 => {
                    let token = lookahead.expect("only a token is shifted");
                    self.states.push(code as usize);
                    self.values.push(Child::Token(token));
                    lookahead = self.lex();
                }
                Some(code) => self.reduce((-code - 1) as usize),
                None => {
                    if !self.recover(&mut lookahead) {
                        return None;
                    }
                }
            }
        }
    }

    fn reduce(&mut self, rule: usize) {
        let info = &RULES[rule];
        let at = self.values.len() - info.len;
        let popped: Vec<Child> = self.values.drain(at..).collect();
        self.states.truncate(self.states.len() - info.len);
        let mut children = Vec::with_capacity(popped.len());
        for child in popped {
            match child {
                Child::Node(node) if RULES[node.rule].helper => children.extend(node.children),
                other => children.push(other),
            }
        }
        let (start, end) = self.extent(&children);
        self.values.push(Child::Node(Node {
            name: info.name,
            rule,
            children,
            start,
            end,
        }));
        let state = tables::goto(self.state(), info.lhs).expect("the tables have the goto");
        self.states.push(state);
    }

    fn extent(&self, children: &[Child]) -> (usize, usize) {
        let first = children.iter().find_map(|c| self.child_range(c));
        let last = children.iter().rev().find_map(|c| self.child_range(c));
        match (first, last) {
            (Some((s, _)), Some((_, e))) => (s, e),
            _ => {
                let at = self.tokens.last().map(|t| t.end).unwrap_or(0);
                (at, at)
            }
        }
    }

    fn child_range(&self, child: &Child) -> Option<(usize, usize)> {
        match child {
            Child::Token(t) => Some((self.tokens[*t].start, self.tokens[*t].end)),
            Child::Node(n) if n.start < n.end || !n.children.is_empty() => Some((n.start, n.end)),
            Child::Node(_) => None,
        }
    }

    fn expected(&self) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = tables::acceptable(self.state())
            .map(|t| if t == END { "the end of the file" } else { describe(t) })
            .collect();
        names.sort();
        names.dedup();
        names
    }

    /// Recovers from a syntax error at `lookahead` (research R10): records it, then assumes a missing `}` at the end of
    /// the file, or skips to the next construct start some state on the stack reads, popping to that state. The
    /// popped and skipped tokens are kept, as skipped text.
    fn recover(&mut self, lookahead: &mut Option<usize>) -> bool {
        let (start, end, found) = match *lookahead {
            Some(t) => (
                self.tokens[t].start,
                self.tokens[t].end,
                format!("`{}`", self.tokens[t].text(self.text)),
            ),
            None => (self.text.len(), self.text.len(), "the end of the file".to_owned()),
        };
        let expected = self.expected();
        let first_error_here = self.errors.last().map(|e| e.start != start).unwrap_or(true);
        if first_error_here {
            self.errors.push(SyntaxError {
                start,
                end,
                message: format!("Found {found} where {} expected.", list(&expected)),
            });
        }
        let close = tables::terminal("\"}\"").expect("the grammar has `}`");
        if lookahead.is_none() {
            // A missing `}`: assume it, where a state on the stack can read it.
            if let Some(depth) = (0..self.states.len())
                .rev()
                .find(|d| tables::action(self.states[*d], close).is_some())
            {
                self.discard_to(depth);
                self.tokens.push(Token {
                    kind: TokenKind::Missing,
                    terminal: close,
                    start: self.text.len(),
                    end: self.text.len(),
                });
                *lookahead = Some(self.tokens.len() - 1);
                return true;
            }
            return false;
        }
        // Skip to a construct start or `}` that a state on the stack reads.
        let mut current = *lookahead;
        let mut skipped_first = false;
        loop {
            let Some(token) = current else {
                *lookahead = None;
                return true;
            };
            let terminal = self.tokens[token].terminal;
            if terminal != u16::MAX
                && synchronising(terminal)
                && (skipped_first || !self.errors_at(token))
                && let Some(depth) = (0..self.states.len())
                    .rev()
                    .find(|d| tables::action(self.states[*d], terminal).is_some())
                && (depth + 1 < self.states.len() || skipped_first)
            {
                self.discard_to(depth);
                *lookahead = Some(token);
                return true;
            }
            self.tokens[token].kind = TokenKind::Skipped;
            skipped_first = true;
            current = self.lex_skipping();
        }
    }

    fn errors_at(&self, token: usize) -> bool {
        self.errors.last().is_some_and(|e| e.start == self.tokens[token].start)
    }

    /// The next token, lexed with every terminal of the grammar a candidate, while skipping.
    fn lex_skipping(&mut self) -> Option<usize> {
        let saved = self.states.clone();
        // A state that reads everything is not at hand; lex as the start of a construct, the outermost state.
        self.states = vec![*self.states.first().unwrap_or(&0)];
        let mut found = self.lex();
        if let Some(t) = found
            && self.tokens[t].terminal == u16::MAX
        {
            found = Some(t);
        }
        self.states = saved;
        let token = found?;
        // Relex a word as the keyword it spells, when it spells one.
        let text = self.tokens[token].text(self.text);
        if let Some(keyword) = TERMINALS.iter().position(|t| t.keyword && t.value == text) {
            self.tokens[token].terminal = keyword as u16;
        } else if text == "}" {
            self.tokens[token].terminal = tables::terminal("\"}\"").unwrap_or(u16::MAX);
        }
        Some(token)
    }

    /// Pops the stack to `depth`, keeping the popped tokens as skipped text.
    fn discard_to(&mut self, depth: usize) {
        while self.states.len() > depth + 1 {
            self.states.pop();
            if let Some(value) = self.values.pop() {
                self.mark_skipped(&value);
            }
        }
    }

    fn mark_skipped(&mut self, child: &Child) {
        match child {
            Child::Token(t) => self.tokens[*t].kind = TokenKind::Skipped,
            Child::Node(n) => {
                for c in &n.children {
                    self.mark_skipped(c);
                }
            }
        }
    }
}

/// How a finding names a terminal it expected.
pub fn describe(terminal: u16) -> &'static str {
    match TERMINALS[terminal as usize].name {
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
