//! The grammar of a `grammar` body (FBL 0.9 section 4.9), built by `adp-atd-grammar` on `etalii-lark` once per
//! grammar, and a body read with it into the family's tree: a node per rule, named by its rule or alias, a token per
//! named terminal, each with its own span, the comments between the tokens, and the text that does not parse, kept and
//! reported, when the binding reads in part (*(FBL 0.11)* inside its blocks too, with `recover.nested`).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use adp_atd_grammar::{BuildError, Built, Options};
use etalii_lark::{InteractiveParser, Node as LarkNode, Token as LarkToken, Tree as LarkTree};

use crate::documents::types::GrammarSettings;
use crate::text::Span;

/// A grammar built for a body: the parser of its start rule and, with `recover`, that of its unit.
pub(crate) struct Grammar {
    pub start: Arc<Built>,
    pub unit: Option<Arc<Built>>,
    /// The terminals reading skips to after text that does not parse, as indices of the lexer's terminals.
    sync: Vec<usize>,
    /// *(FBL 0.11)* The terminals that open and close a block, with `recover.nested`.
    nested: Option<Nested>,
    /// *(FBL 0.11)* `recover.unclosed`: each terminal, and the text a token of it must end with.
    unclosed: Vec<(usize, String)>,
}

/// `recover.nested`, as indices of the lexer's terminals.
#[derive(Debug, Clone, Copy)]
struct Nested {
    open: usize,
    close: usize,
}

/// Why a grammar cannot be built, as a binding's problem says it (FBL section 14.1).
pub(crate) fn describe(error: &BuildError) -> String {
    match error {
        BuildError::Conflicts(_) | BuildError::Collisions(_) => {
            format!("The grammar is not LALR(1): {}", error.to_string().trim())
        }
        other => format!("The grammar cannot be built: {}", other.to_string().trim()),
    }
}

/// The grammar of `settings`, built once for every body that has the same one.
pub(crate) fn build(settings: &GrammarSettings) -> Result<Arc<Grammar>, String> {
    static BUILT: OnceLock<Mutex<HashMap<String, Arc<Grammar>>>> = OnceLock::new();
    if settings.modules.is_empty() {
        return Err(format!(
            "The grammar file '{}' was not read beside the binding.",
            settings.file.as_deref().unwrap_or_default()
        ));
    }
    let key = format!("{settings:?}");
    let cache = BUILT.get_or_init(Mutex::default);
    if let Some(built) = cache.lock().ok().and_then(|c| c.get(&key).cloned()) {
        return Ok(built);
    }
    let load = |module: &str| settings.modules.get(module).cloned();
    let options = Options {
        strict: true,
        keep_all_tokens: false,
        maybe_placeholders: true,
    };
    let start = adp_atd_grammar::build_with_options(&settings.root, &settings.start, &load, options)
        .map_err(|e| describe(&e))?;
    // A terminal by its name, or by the text of an anonymous string terminal.
    let terminal = |name: &str| {
        start.lexer.matchers.iter().position(|m| {
            let d = m.definition();
            d.name == *name || (d.pattern.is_str() && d.pattern.value() == name)
        })
    };
    let (unit, sync, nested, unclosed) = match &settings.recover {
        None => (None, Vec::new(), None, Vec::new()),
        Some(recover) => {
            let unit = adp_atd_grammar::build_with_options(&settings.root, &recover.unit, &load, options)
                .map_err(|e| describe(&e))?;
            let sync = recover.sync.iter().filter_map(|s| terminal(s)).collect();
            let nested = recover.nested.as_ref().and_then(|n| {
                Some(Nested {
                    open: terminal(&n.open)?,
                    close: terminal(&n.close)?,
                })
            });
            let unclosed = recover
                .unclosed
                .iter()
                .filter_map(|(name, ending)| Some((terminal(name)?, ending.clone())))
                .collect();
            (Some(Arc::new(unit)), sync, nested, unclosed)
        }
    };
    let grammar = Arc::new(Grammar {
        start: Arc::new(start),
        unit,
        sync,
        nested,
        unclosed,
    });
    if let Ok(mut c) = cache.lock() {
        c.insert(key, Arc::clone(&grammar));
    }
    Ok(grammar)
}

/// A node or token of the family's tree.
#[derive(Debug, Clone)]
pub(crate) struct GNode {
    /// A node's rule or alias, a token's terminal.
    pub name: String,
    pub token: bool,
    /// The bytes from the first token its rule matched to the last; none for a node that matched no token.
    pub span: Option<Span>,
    /// Its children in order, an optional part that matched nothing being `None`.
    pub children: Vec<Option<usize>>,
}

/// A body read by its grammar.
#[derive(Debug, Default)]
pub(crate) struct Reading {
    /// The nodes and tokens; the root is the first.
    pub nodes: Vec<GNode>,
    /// The text that did not parse, each with where reading stopped (FBL section 4.9).
    pub unparsed: Vec<(usize, Span)>,
    /// *(FBL 0.11)* Every token the tree was read from, those it leaves out included, in document order.
    pub tokens: Vec<Span>,
    /// *(FBL 0.11)* The comments: the ignored tokens between those, each holding more than whitespace.
    pub comments: Vec<Span>,
    /// *(FBL 0.11)* The `open` token of each block still open at the end of the body.
    pub unclosed_blocks: Vec<Span>,
    /// *(FBL 0.11)* The `close` tokens that closed no block.
    pub unmatched: Vec<Span>,
    /// *(FBL 0.11)* The tokens of a terminal of `recover.unclosed` that do not end with its text.
    pub unclosed_comments: Vec<Span>,
}

/// Why a body cannot be read as a whole: where, and what was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Unreadable {
    pub offset: usize,
    pub message: String,
}

impl Grammar {
    /// Reads `text`: as a whole, or unit by unit with `recover`.
    pub fn read(&self, text: &str) -> Result<Reading, Unreadable> {
        let mut reading = Reading::default();
        reading.nodes.push(GNode {
            name: self.start.grammar.start.clone(),
            token: false,
            span: Some(Span::new(0, text.len())),
            children: Vec::new(),
        });
        match &self.unit {
            None => {
                let root = match self.parse_unit(&self.start, text, 0, false, false, &mut reading)? {
                    UnitEnd::Accepted(root, _) => root,
                    UnitEnd::Stopped(offset, message) => return Err(Unreadable { offset, message }),
                    UnitEnd::Skipped(_) | UnitEnd::Recover => unreachable!("without recover nothing is skipped"),
                };
                // The start rule's node is the root itself: its children are the root's.
                if let LarkNode::Tree(tree) = root {
                    let children = tree.children.into_iter().map(|c| reading.add(c, 0)).collect();
                    reading.nodes[0].children = children;
                }
            }
            Some(unit) => {
                let mut position = self.skip_trivia(text, 0);
                while position < text.len() {
                    let mark = (reading.tokens.len(), reading.unmatched.len());
                    // A unit is first read without keeping the parser's state at each boundary, which holds all that
                    // was read so far: a body that parses keeps no copies (FBL 0.11 section 4.9, rule 4).
                    let mut end = self.parse_unit(unit, text, position, true, false, &mut reading);
                    if matches!(end, Ok(UnitEnd::Recover)) {
                        reading.tokens.truncate(mark.0);
                        reading.unmatched.truncate(mark.1);
                        end = self.parse_unit(unit, text, position, true, true, &mut reading);
                    }
                    match end {
                        Ok(UnitEnd::Accepted(node, end)) => {
                            let child = reading.add(node, position);
                            reading.nodes[0].children.push(child);
                            position = end;
                        }
                        Ok(UnitEnd::Skipped(end)) => position = end,
                        Ok(UnitEnd::Recover) => unreachable!("a unit read in recovery recovers"),
                        Ok(UnitEnd::Stopped(stopped, _)) | Err(Unreadable { offset: stopped, .. }) => {
                            reading.tokens.truncate(mark.0);
                            let end = self.skip(text, stopped, None, &mut reading);
                            reading.unparsed.push((stopped, Span::new(position, end)));
                            position = end;
                        }
                    }
                    position = self.skip_trivia(text, position);
                }
            }
        }
        self.read_comments(text, &mut reading);
        Ok(reading)
    }

    /// The position after the trivia at `position`: whitespace and the tokens the grammar ignores.
    fn skip_trivia(&self, text: &str, mut position: usize) -> usize {
        let lexer = &self.start.lexer;
        while position < text.len() {
            match lexer.next(text, position, &[]) {
                Some(l) if l.end > position && lexer.matchers[l.terminal].ignored => position = l.end,
                _ => {
                    let c = text[position..].chars().next().unwrap_or(' ');
                    if c.is_whitespace() {
                        position += c.len_utf8();
                    } else {
                        break;
                    }
                }
            }
        }
        position
    }

    /// The end of the first token of a synchronising terminal that ends after `stopped`, lexing from there with
    /// every terminal and passing a character no terminal matches; the end of the body when none follows. *(FBL 0.11)*
    /// Inside a block, the start of a `close` token at the block's own depth when that comes first.
    fn skip(&self, text: &str, stopped: usize, nested: Option<Nested>, reading: &mut Reading) -> usize {
        let lexer = &self.start.lexer;
        let all: Vec<usize> = (0..lexer.matchers.len()).collect();
        let mut position = stopped;
        let mut depth = 0usize;
        while position < text.len() {
            match lexer.next(text, position, &all) {
                Some(l) if l.end > position => {
                    self.check_unclosed(text, l.terminal, Span::new(l.start, l.end), reading);
                    if let Some(nested) = nested {
                        if l.terminal == nested.close {
                            if depth == 0 {
                                return l.start;
                            }
                            depth -= 1;
                        } else if l.terminal == nested.open {
                            depth += 1;
                        }
                    }
                    if self.sync.contains(&l.terminal) {
                        return l.end;
                    }
                    position = l.end;
                }
                _ => position += text[position..].chars().next().map(char::len_utf8).unwrap_or(1),
            }
        }
        text.len()
    }

    /// Reports a token of a terminal of `recover.unclosed` that does not end with its text (FBL 0.11 section 4.9).
    fn check_unclosed(&self, text: &str, terminal: usize, token: Span, reading: &mut Reading) {
        if self
            .unclosed
            .iter()
            .any(|(t, ending)| *t == terminal && !text[token.start..token.end].ends_with(ending.as_str()))
        {
            reading.unclosed_comments.push(token);
        }
    }

    /// The comments of the trivia: the ignored tokens between the tokens read that hold more than whitespace. Only
    /// what the parser did not read is looked at, so that a comment's opening inside a token is the token's (FBL
    /// 0.11 section 4.9); the text that does not parse holds none.
    fn read_comments(&self, text: &str, reading: &mut Reading) {
        let lexer = &self.start.lexer;
        let mut taken: Vec<Span> = reading.tokens.clone();
        taken.extend(reading.unparsed.iter().map(|(_, skipped)| *skipped));
        taken.extend(reading.unmatched.iter().copied());
        taken.sort_by_key(|s| s.start);
        let mut comments = Vec::new();
        let mut gap_start = 0;
        for span in taken
            .iter()
            .copied()
            .chain(std::iter::once(Span::new(text.len(), text.len())))
        {
            let mut start = gap_start;
            while start < span.start {
                match lexer.next(&text[..span.start], start, &[]) {
                    Some(l) if l.end > start && lexer.matchers[l.terminal].ignored => {
                        if text[start..l.end].chars().any(|c| !c.is_whitespace()) {
                            comments.push(Span::new(start, l.end));
                            self.check_unclosed(text, l.terminal, Span::new(start, l.end), reading);
                        }
                        start = l.end;
                    }
                    _ => start += text[start..].chars().next().map(char::len_utf8).unwrap_or(1),
                }
            }
            gap_start = gap_start.max(span.end);
        }
        reading.comments = comments;
    }

    /// Reads one unit of `built`'s start rule from `position`, positions counted from there: tokens while the parser
    /// can shift them; then the unit ends when the parser can accept there, or with `partial` when what follows gives
    /// no token it can shift (FBL section 4.9). Without `partial`, all of the text must be the unit. *(FBL 0.11)* With
    /// `recover.nested`, reading stopped or ended inside a block asks for `Recover`, unless `recover` already keeps the
    /// boundaries that it continues from (rules 4 to 7).
    fn parse_unit(
        &self,
        built: &Built,
        text: &str,
        position: usize,
        partial: bool,
        recover: bool,
        reading: &mut Reading,
    ) -> Result<UnitEnd, Unreadable> {
        let slice = &text[position..];
        let lark = &built.lark;
        let mut parser = lark.parse_interactive(slice, None).map_err(|e| Unreadable {
            offset: position,
            message: e.to_string(),
        })?;
        let reads = |parser: &InteractiveParser<'_>, kind: &str| {
            lark.parse_table().is_some_and(|t| {
                t.states
                    .get(parser.parser_state().position())
                    .is_some_and(|row| row.contains_key(kind))
            })
        };
        let lexer = &self.start.lexer;
        let kind_of = |terminal: usize| lexer.matchers[terminal].definition().name.as_str();
        let nested = self.nested.filter(|_| partial);
        let sync_kinds: Vec<&str> = self.sync.iter().map(|&s| kind_of(s)).collect();
        let mut blocks: Vec<OpenBlock<'_>> = Vec::new();
        let mut depth = 0usize;
        let mut last_end = 0;
        // Where the next unit starts: after the unit's last token, or the text skipped after it.
        let mut consumed = 0;
        let mut any = false;
        loop {
            let stopped_at = match parser.next_token() {
                Ok(Some(token)) => {
                    let start = token.start_pos.unwrap_or(0);
                    if reads(&parser, &token.kind) {
                        let end = token.end_pos.unwrap_or(start);
                        let kind = token.kind.clone();
                        if parser.feed_token(token).is_ok() {
                            last_end = end;
                            consumed = end;
                            any = true;
                            let span = Span::new(position + start, position + end);
                            reading.tokens.push(span);
                            if nested.is_some_and(|n| kind == kind_of(n.open)) {
                                depth += 1;
                                if recover {
                                    blocks.push(OpenBlock {
                                        open: span,
                                        boundary: parser.copy(),
                                        boundary_end: end,
                                        tokens: reading.tokens.len(),
                                    });
                                }
                            } else if nested.is_some_and(|n| kind == kind_of(n.close)) {
                                depth = depth.saturating_sub(1);
                                blocks.pop();
                            } else if sync_kinds.contains(&kind.as_str())
                                && let Some(block) = blocks.last_mut()
                            {
                                block.boundary = parser.copy();
                                block.boundary_end = end;
                                block.tokens = reading.tokens.len();
                            }
                            continue;
                        }
                    }
                    start
                }
                Ok(None) => {
                    if depth > 0 && !recover {
                        return Ok(UnitEnd::Recover);
                    }
                    if !self.close_blocks(&mut parser, &mut blocks, last_end, slice.len(), position, reading) {
                        return Ok(UnitEnd::Stopped(
                            position + slice.len(),
                            "The text ends too early.".into(),
                        ));
                    }
                    return match parser.feed_eof(None) {
                        Ok(Some(node)) => Ok(UnitEnd::Accepted(
                            node,
                            position + if depth > 0 { slice.len() } else { consumed },
                        )),
                        _ => Ok(UnitEnd::Stopped(
                            position + slice.len(),
                            "The text ends too early.".into(),
                        )),
                    };
                }
                Err(error) => match &error {
                    etalii_lark::Error::UnexpectedToken(e) => e.token.start_pos.unwrap_or(last_end),
                    etalii_lark::Error::UnexpectedCharacters(e) => e.pos_in_stream,
                    _ => last_end,
                },
            };
            if depth > 0 {
                if !recover {
                    return Ok(UnitEnd::Recover);
                }
                let block = blocks.last().expect("a block is open at this depth");
                let stopped = position + stopped_at;
                let from = position + block.boundary_end;
                let mut end = self.skip(text, stopped, nested, reading);
                // A close token the parser cannot shift at the boundary itself closes nothing: it goes alone.
                if end == from {
                    end = lexer
                        .next(text, end, &[nested.expect("a block is nested").close])
                        .map_or(end + 1, |l| l.end);
                    reading.unmatched.push(Span::new(from, end));
                } else {
                    reading.unparsed.push((stopped, Span::new(from, end)));
                }
                reading.tokens.truncate(block.tokens);
                last_end = block.boundary_end;
                consumed = end - position;
                parser = block.boundary.copy();
                parser.skip_chars(text[from..end].chars().count());
                continue;
            }
            if partial && let Ok(Some(node)) = parser.copy().feed_eof(None) {
                return Ok(UnitEnd::Accepted(node, position + consumed));
            }
            // A close token outside every block closes nothing, and is skipped alone (FBL 0.11 section 4.9, rule 7).
            if let Some(nested) = nested
                && let Some(l) = lexer.next(text, position + stopped_at, &[nested.close])
                && l.terminal == nested.close
                && l.start == position + stopped_at
            {
                reading.unmatched.push(Span::new(l.start, l.end));
                if !any {
                    return Ok(UnitEnd::Skipped(l.end));
                }
                consumed = l.end - position;
                let at = position + parser.lexer_position();
                if at < l.end {
                    parser.skip_chars(text[at..l.end].chars().count());
                }
                continue;
            }
            let found: String = slice[stopped_at..]
                .chars()
                .take_while(|c| !c.is_whitespace())
                .take(20)
                .collect();
            return Ok(UnitEnd::Stopped(
                position + stopped_at,
                if found.is_empty() {
                    "The text ends too early.".into()
                } else {
                    format!("Unexpected '{found}'.")
                },
            ));
        }
    }

    /// Ends each block still open at the end of the body (FBL 0.11 section 4.9, rule 6): closed without text, with a
    /// `sync` token first where the grammar needs one, and reported; a statement the body ends in that does not let the
    /// block close is skipped from the block's boundary. False when the parser still cannot close it.
    fn close_blocks<'a>(
        &self,
        parser: &mut InteractiveParser<'a>,
        blocks: &mut Vec<OpenBlock<'a>>,
        last_end: usize,
        length: usize,
        position: usize,
        reading: &mut Reading,
    ) -> bool {
        while let Some(block) = blocks.pop() {
            let closed = match self.closed(parser, last_end) {
                Some(closed) => closed,
                None => {
                    reading.unparsed.push((
                        position + length,
                        Span::new(position + block.boundary_end, position + length),
                    ));
                    reading.tokens.truncate(block.tokens);
                    match self.closed(&block.boundary, block.boundary_end) {
                        Some(closed) => closed,
                        None => return false,
                    }
                }
            };
            *parser = closed;
            reading.unclosed_blocks.push(block.open);
        }
        true
    }

    /// `parser` with its block closed without text, its tokens at `at`, when the grammar lets it close there.
    fn closed<'a>(&self, parser: &InteractiveParser<'a>, at: usize) -> Option<InteractiveParser<'a>> {
        let lexer = &self.start.lexer;
        let kind_of = |terminal: usize| lexer.matchers[terminal].definition().name.clone();
        let close = kind_of(self.nested?.close);
        let without_text = |kind: String| LarkToken {
            kind,
            value: String::new(),
            start_pos: Some(at),
            end_pos: Some(at),
            ..LarkToken::default()
        };
        let mut attempt = parser.copy();
        if attempt.feed_token(without_text(close.clone())).is_ok() {
            return Some(attempt);
        }
        self.sync.iter().find_map(|&sync| {
            let mut attempt = parser.copy();
            let fed = attempt.feed_token(without_text(kind_of(sync))).is_ok()
                && attempt.feed_token(without_text(close.clone())).is_ok();
            fed.then_some(attempt)
        })
    }
}

impl Reading {
    /// Adds a node of Lark's tree, read from text that starts at `offset`, with its descendants; its index.
    fn add(&mut self, node: LarkNode, offset: usize) -> Option<usize> {
        let added = match node {
            LarkNode::Missing => return None,
            LarkNode::Token(token) => GNode {
                name: token.kind.clone(),
                token: true,
                span: token_span(&token, offset),
                children: Vec::new(),
            },
            LarkNode::Tree(tree) => {
                let span = tree_span(&tree, offset);
                let index = self.nodes.len();
                self.nodes.push(GNode {
                    name: tree.data.clone(),
                    token: false,
                    span,
                    children: Vec::new(),
                });
                let children = tree.children.into_iter().map(|c| self.add(c, offset)).collect();
                self.nodes[index].children = children;
                return Some(index);
            }
        };
        self.nodes.push(added);
        Some(self.nodes.len() - 1)
    }
}

fn token_span(token: &LarkToken, offset: usize) -> Option<Span> {
    Some(Span::new(token.start_pos? + offset, token.end_pos? + offset))
}

fn tree_span(tree: &LarkTree, offset: usize) -> Option<Span> {
    let meta = tree.meta();
    Some(Span::new(meta.start_pos? + offset, meta.end_pos? + offset))
}

/// How reading a unit ended: with its node and the end of its last token, where it stopped, *(FBL 0.11)* with a `close`
/// token that closes nothing and nothing before it, or stopped in a block, to read again keeping its boundaries.
enum UnitEnd {
    Accepted(LarkNode, usize),
    Stopped(usize, String),
    Skipped(usize),
    Recover,
}

/// A block the reader is inside (FBL 0.11 section 4.9, rule 4): its `open` token, the parser's state at its last
/// boundary, where that boundary is counted from the unit's start, and how many tokens were read by then.
struct OpenBlock<'a> {
    open: Span,
    boundary: InteractiveParser<'a>,
    boundary_end: usize,
    tokens: usize,
}
