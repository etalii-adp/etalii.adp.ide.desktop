//! The grammar of a `grammar` body (FBL 0.9 section 4.9), built by `adp-atd-grammar` on `etalii-lark` once per
//! grammar, and a body read with it into the family's tree: a node per rule, named by its rule or alias, a token per
//! named terminal, each with its own span, and the text that does not parse, kept and reported, when the binding
//! reads in part.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use adp_atd_grammar::{BuildError, Built, Options};
use etalii_lark::{Node as LarkNode, Token as LarkToken, Tree as LarkTree};

use crate::documents::types::GrammarSettings;
use crate::text::Span;

/// A grammar built for a body: the parser of its start rule and, with `recover`, that of its unit.
pub(crate) struct Grammar {
    pub start: Arc<Built>,
    pub unit: Option<Arc<Built>>,
    /// The terminals reading skips to after text that does not parse, as indices of the lexer's terminals.
    sync: Vec<usize>,
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
    let (unit, sync) = match &settings.recover {
        None => (None, Vec::new()),
        Some(recover) => {
            let unit = adp_atd_grammar::build_with_options(&settings.root, &recover.unit, &load, options)
                .map_err(|e| describe(&e))?;
            let sync = recover
                .sync
                .iter()
                .filter_map(|s| {
                    start.lexer.matchers.iter().position(|m| {
                        let d = m.definition();
                        d.name == *s || (d.pattern.is_str() && d.pattern.value() == s)
                    })
                })
                .collect();
            (Some(Arc::new(unit)), sync)
        }
    };
    let grammar = Arc::new(Grammar {
        start: Arc::new(start),
        unit,
        sync,
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
                let root = match parse_unit(&self.start, text, 0, false)? {
                    UnitEnd::Accepted(root, _) => root,
                    UnitEnd::Stopped(offset, message) => return Err(Unreadable { offset, message }),
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
                    match parse_unit(unit, text, position, true) {
                        Ok(UnitEnd::Accepted(node, end)) => {
                            let child = reading.add(node, position);
                            reading.nodes[0].children.push(child);
                            position = end;
                        }
                        Ok(UnitEnd::Stopped(stopped, _)) | Err(Unreadable { offset: stopped, .. }) => {
                            let end = self.skip_to_sync(text, stopped);
                            reading.unparsed.push((stopped, Span::new(position, end)));
                            position = end;
                        }
                    }
                    position = self.skip_trivia(text, position);
                }
            }
        }
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
    /// every terminal and passing a character no terminal matches; the end of the body when none follows.
    fn skip_to_sync(&self, text: &str, stopped: usize) -> usize {
        let lexer = &self.start.lexer;
        let all: Vec<usize> = (0..lexer.matchers.len()).collect();
        let mut position = stopped;
        while position < text.len() {
            match lexer.next(text, position, &all) {
                Some(l) if l.end > position => {
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

    /// The comments of `text` between `start` and `end`: the ignored tokens that hold more than whitespace.
    pub fn comments(&self, text: &str, mut start: usize, end: usize) -> Vec<Span> {
        let lexer = &self.start.lexer;
        let mut comments = Vec::new();
        while start < end {
            match lexer.next(&text[..end], start, &[]) {
                Some(l) if l.end > start && lexer.matchers[l.terminal].ignored => {
                    if text[start..l.end].chars().any(|c| !c.is_whitespace()) {
                        comments.push(Span::new(start, l.end));
                    }
                    start = l.end;
                }
                _ => start += text[start..].chars().next().map(char::len_utf8).unwrap_or(1),
            }
        }
        comments
    }

    /// Whether `value` is a whole token of the terminal Lark names `terminal`.
    pub fn is_token_of(&self, terminal: &str, value: &str) -> bool {
        let lexer = &self.start.lexer;
        let Some(index) = lexer.matchers.iter().position(|m| m.definition().name == terminal) else {
            return true;
        };
        lexer
            .next(value, 0, &[index])
            .is_some_and(|l| l.terminal == index && l.end == value.len())
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

/// How reading a unit ended: with its node and the end of its last token, or where it stopped.
enum UnitEnd {
    Accepted(LarkNode, usize),
    Stopped(usize, String),
}

/// Reads one unit of `built`'s start rule from `position`, positions counted from there: tokens while the parser
/// can shift them; then the unit ends when the parser can accept there, or with `partial` when what follows gives
/// no token it can shift (FBL section 4.9). Without `partial`, all of the text must be the unit.
fn parse_unit(built: &Built, text: &str, position: usize, partial: bool) -> Result<UnitEnd, Unreadable> {
    let slice = &text[position..];
    let lark = &built.lark;
    let mut parser = lark.parse_interactive(slice, None).map_err(|e| Unreadable {
        offset: position,
        message: e.to_string(),
    })?;
    let reads = |parser: &etalii_lark::InteractiveParser<'_>, kind: &str| {
        lark.parse_table().is_some_and(|t| {
            t.states
                .get(parser.parser_state().position())
                .is_some_and(|row| row.contains_key(kind))
        })
    };
    let mut last_end = 0;
    loop {
        let stopped_at = match parser.next_token() {
            Ok(Some(token)) => {
                let start = token.start_pos.unwrap_or(0);
                if reads(&parser, &token.kind) {
                    let end = token.end_pos.unwrap_or(start);
                    if parser.feed_token(token).is_ok() {
                        last_end = end;
                        continue;
                    }
                }
                start
            }
            Ok(None) => {
                return match parser.feed_eof(None) {
                    Ok(Some(node)) => Ok(UnitEnd::Accepted(node, position + last_end)),
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
        if partial && let Ok(Some(node)) = parser.copy().feed_eof(None) {
            return Ok(UnitEnd::Accepted(node, position + last_end));
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
