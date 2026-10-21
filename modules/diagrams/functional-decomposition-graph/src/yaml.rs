//! The YAML a `.fdg` is read with: its nodes, each with where it starts and ends, and the lines a node occupies
//! (etalii.adp spec 030, task T072).
//!
//! Ported from the standalone host's core at `8b6ccfc`: `src/backend/EtAlii.Adp.Documents/YamlAccess.cs` and
//! `YamlNodeRange.cs`, which read through YamlDotNet's representation model. Here saphyr's event parser builds the
//! same tree; YAML is only ever read, never written. A node's range runs from the line it starts on to the last line
//! its subtree says something on, trailing blank and comment lines trimmed, because an edit has no business
//! rewriting a comment that merely follows an entry. A node's end is the last character of its subtree's content
//! rather than YamlDotNet's end mark, which is the same line once the standalone host's step back off a mark in
//! column one is taken.

use std::collections::HashMap;

use saphyr_parser::{Event, Parser, ScalarStyle};

use crate::lines::{LineDocument, LineRange};

/// One node of a document: a scalar with its text, a mapping with its entries in order, or a sequence.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Scalar {
        value: String,
        /// The character offset of its first character and one past its last.
        start: usize,
        end: usize,
    },
    Mapping {
        start: usize,
        entries: Vec<(Node, Node)>,
    },
    Sequence {
        start: usize,
        items: Vec<Node>,
    },
}

impl Node {
    /// The character offset the node starts at.
    pub fn start(&self) -> usize {
        match self {
            Node::Scalar { start, .. } | Node::Mapping { start, .. } | Node::Sequence { start, .. } => *start,
        }
    }

    /// The node under `key` of a mapping, of whatever kind.
    pub fn get(&self, key: &str) -> Option<&Node> {
        match self {
            Node::Mapping { entries, .. } => entries.iter().find_map(|(k, v)| match k {
                Node::Scalar { value, .. } if value == key => Some(v),
                _ => None,
            }),
            _ => None,
        }
    }

    /// The scalar under `key`, or none where there is none or it is not a scalar.
    pub fn scalar(&self, key: &str) -> Option<&str> {
        match self.get(key)? {
            Node::Scalar { value, .. } => Some(value),
            _ => None,
        }
    }

    /// The items of the sequence under `key`, empty where there is none.
    pub fn sequence(&self, key: &str) -> &[Node] {
        match self.get(key) {
            Some(Node::Sequence { items, .. }) => items,
            _ => &[],
        }
    }

    /// The furthest character offset the node's subtree has content at, as one past it.
    fn content_end(&self) -> usize {
        match self {
            Node::Scalar { start, end, .. } => (*end).max(*start),
            Node::Mapping { start, entries } => entries
                .iter()
                .map(|(k, v)| k.content_end().max(v.content_end()))
                .max()
                .unwrap_or(*start),
            Node::Sequence { start, items } => items.iter().map(Node::content_end).max().unwrap_or(*start),
        }
    }
}

/// Why a text is not YAML: the parser's sentence and the character offset it stopped at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlError {
    pub message: String,
    pub at: usize,
}

/// Where each line of a document starts, in characters, to turn an offset into its zero-based line.
pub struct LineIndex {
    starts: Vec<usize>,
    chars: Vec<char>,
}

impl LineIndex {
    pub fn of(document: &LineDocument) -> LineIndex {
        let mut starts = Vec::with_capacity(document.lines().len());
        let mut chars = Vec::new();
        for line in document.lines() {
            starts.push(chars.len());
            chars.extend(line.text.chars());
            chars.extend(line.ending.chars());
        }
        LineIndex { starts, chars }
    }

    /// The zero-based line holding character `offset`, kept inside the document.
    pub fn line_of(&self, offset: usize) -> usize {
        match self.starts.binary_search(&offset) {
            Ok(line) => line,
            Err(next) => next.saturating_sub(1),
        }
        .min(self.starts.len().saturating_sub(1))
    }

    /// The line `node` starts on: where a problem found at it is reported.
    pub fn start_line(&self, node: &Node) -> usize {
        self.line_of(node.start())
    }

    /// The lines `node` occupies: from its first line to the last its subtree says something on, trailing blank
    /// and comment lines trimmed.
    pub fn range(&self, node: &Node, document: &LineDocument) -> LineRange {
        let start = self.start_line(node);
        let mut end_offset = node.content_end();
        // Back off the whitespace and line breaks a block scalar's end takes in, to its last character.
        while end_offset > node.start() && self.chars.get(end_offset - 1).is_some_and(|c| c.is_whitespace()) {
            end_offset -= 1;
        }
        let mut end = self.line_of(end_offset.saturating_sub(1).max(node.start())).max(start);
        let lines = document.lines();
        while end > start && says_nothing(&lines[end].text) {
            end -= 1;
        }
        LineRange::new(start, end)
    }
}

/// A blank line, or a YAML comment line: its first character that is not a space is `#`.
fn says_nothing(text: &str) -> bool {
    text.trim().is_empty() || text.trim_start_matches(' ').starts_with('#')
}

enum Frame {
    Mapping {
        start: usize,
        anchor: usize,
        entries: Vec<(Node, Node)>,
        key: Option<Node>,
    },
    Sequence {
        start: usize,
        anchor: usize,
        items: Vec<Node>,
    },
}

/// Reads the first document of `text`: its root node, none for an empty text, or why it is not YAML. A mapping that
/// states a key twice is not YAML either, as YamlDotNet's representation model refused it.
pub fn read(text: &str) -> Result<Option<Node>, YamlError> {
    let mut stack: Vec<Frame> = Vec::new();
    let mut root: Option<Node> = None;
    let mut anchors: HashMap<usize, Node> = HashMap::new();
    for result in Parser::new_from_str(text) {
        let (event, span) = result.map_err(|error| YamlError {
            message: sentence(error.info()),
            at: error.marker().index(),
        })?;
        let (start, end) = (span.start.index(), span.end.index());
        let node = match event {
            Event::Scalar(value, style, anchor, _) => {
                // An empty value reads as the empty text, as YamlDotNet's scalar of nothing does.
                let value = if style == ScalarStyle::Plain && start == end {
                    String::new()
                } else {
                    value.to_string()
                };
                let node = Node::Scalar { value, start, end };
                if anchor > 0 {
                    anchors.insert(anchor, node.clone());
                }
                Some(node)
            }
            Event::Alias(anchor) => Some(anchors.get(&anchor).cloned().unwrap_or(Node::Scalar {
                value: String::new(),
                start,
                end,
            })),
            Event::MappingStart(anchor, _) => {
                stack.push(Frame::Mapping {
                    start,
                    anchor,
                    entries: Vec::new(),
                    key: None,
                });
                None
            }
            Event::SequenceStart(anchor, _) => {
                stack.push(Frame::Sequence {
                    start,
                    anchor,
                    items: Vec::new(),
                });
                None
            }
            Event::MappingEnd | Event::SequenceEnd => stack.pop().map(|frame| {
                let (node, anchor) = match frame {
                    Frame::Mapping {
                        start, anchor, entries, ..
                    } => (Node::Mapping { start, entries }, anchor),
                    Frame::Sequence { start, anchor, items } => (Node::Sequence { start, items }, anchor),
                };
                if anchor > 0 {
                    anchors.insert(anchor, node.clone());
                }
                node
            }),
            Event::DocumentEnd if root.is_some() => break,
            _ => None,
        };
        let Some(node) = node else { continue };
        match stack.last_mut() {
            None => {
                if root.is_none() {
                    root = Some(node);
                }
            }
            Some(Frame::Sequence { items, .. }) => items.push(node),
            Some(Frame::Mapping { entries, key, .. }) => match key.take() {
                None => {
                    if let Node::Scalar { value, start, .. } = &node
                        && entries
                            .iter()
                            .any(|(k, _)| matches!(k, Node::Scalar { value: v, .. } if v == value))
                    {
                        return Err(YamlError {
                            message: "Duplicate key".to_owned(),
                            at: *start,
                        });
                    }
                    *key = Some(node);
                }
                Some(k) => entries.push((k, node)),
            },
        }
    }
    Ok(root)
}

/// The parser's wording as a sentence.
fn sentence(info: &str) -> String {
    let mut chars = info.chars();
    let mut text = match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
        None => String::new(),
    };
    if !text.ends_with('.') {
        text.push('.');
    }
    text
}
