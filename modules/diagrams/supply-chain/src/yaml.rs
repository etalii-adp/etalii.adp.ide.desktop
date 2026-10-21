//! The YAML a `.supply` document is read from: its first document as a tree of nodes, each with where it starts and
//! ends, and the lines an entry occupies (etalii.adp spec 030, task T076).
//!
//! Ported from the standalone host's core at `8b6ccfc`: `src/backend/EtAlii.Adp.Documents/YamlAccess.cs` and
//! `YamlNodeRange.cs`, which read through YamlDotNet's representation model; here the events of `saphyr-parser` are
//! built into the same tree. YAML is only ever read: every write is a splice of lines ([`crate::lines`]).
//!
//! The marks are those YamlDotNet's representation model gives: a scalar starts where it starts and ends after its
//! last character, and a collection's own end is the end of the event that opened it, so the lines an entry occupies
//! are found from the furthest end in its subtree ([`range_of`]). A key that a mapping holds twice makes the text
//! unreadable, as YamlDotNet's loader refuses it.

use saphyr_parser::{Event, Parser, ScalarStyle};

use crate::lines::{LineDocument, LineRange};

/// A place in the text: its 1-based line and column, as YamlDotNet's `Mark` counts them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Mark {
    pub line: usize,
    pub column: usize,
}

/// One node of the tree.
#[derive(Debug, Clone, PartialEq)]
pub enum YamlNode {
    Scalar {
        value: String,
        start: Mark,
        end: Mark,
    },
    Mapping {
        entries: Vec<(YamlNode, YamlNode)>,
        start: Mark,
        end: Mark,
    },
    Sequence {
        items: Vec<YamlNode>,
        start: Mark,
        end: Mark,
    },
}

impl YamlNode {
    pub fn start(&self) -> Mark {
        match self {
            YamlNode::Scalar { start, .. } | YamlNode::Mapping { start, .. } | YamlNode::Sequence { start, .. } => {
                *start
            }
        }
    }

    pub fn end(&self) -> Mark {
        match self {
            YamlNode::Scalar { end, .. } | YamlNode::Mapping { end, .. } | YamlNode::Sequence { end, .. } => *end,
        }
    }

    /// The node under `key` of a mapping, of whatever kind (`YamlAccess.Node`).
    pub fn get(&self, key: &str) -> Option<&YamlNode> {
        match self {
            YamlNode::Mapping { entries, .. } => entries
                .iter()
                .find(|(k, _)| matches!(k, YamlNode::Scalar { value, .. } if value == key))
                .map(|(_, v)| v),
            _ => None,
        }
    }

    /// The scalar under `key`, or none where there is none or it is not a scalar (`YamlAccess.Scalar`).
    pub fn scalar(&self, key: &str) -> Option<&str> {
        match self.get(key)? {
            YamlNode::Scalar { value, .. } => Some(value),
            _ => None,
        }
    }

    pub fn as_scalar(&self) -> Option<&str> {
        match self {
            YamlNode::Scalar { value, .. } => Some(value),
            _ => None,
        }
    }

    /// Every node directly below: a mapping's keys, then its values; a sequence's items.
    fn children(&self) -> Vec<&YamlNode> {
        match self {
            YamlNode::Mapping { entries, .. } => entries
                .iter()
                .map(|(k, _)| k)
                .chain(entries.iter().map(|(_, v)| v))
                .collect(),
            YamlNode::Sequence { items, .. } => items.iter().collect(),
            YamlNode::Scalar { .. } => Vec::new(),
        }
    }
}

/// Why a text is not YAML: the 1-based line the reader stopped on, and what it said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlError {
    pub line: usize,
    pub message: String,
}

/// What is still being built.
enum Open {
    Mapping {
        entries: Vec<(YamlNode, YamlNode)>,
        key: Option<YamlNode>,
        start: Mark,
        end: Mark,
    },
    Sequence {
        items: Vec<YamlNode>,
        start: Mark,
        end: Mark,
    },
}

fn mark(marker: saphyr_parser::Marker) -> Mark {
    Mark {
        line: marker.line(),
        column: marker.col() + 1,
    }
}

/// The root of the first document of `text`, or none for a text with no document; why it is not YAML when it is not.
pub fn parse(text: &str) -> Result<Option<YamlNode>, YamlError> {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut parser = Parser::new_from_str(text);
    let mut stack: Vec<Open> = Vec::new();
    let mut root: Option<YamlNode> = None;
    let mut documents = 0;
    while let Some(event) = parser.next_event() {
        let (event, span) = event.map_err(|error| YamlError {
            line: error.marker().line(),
            message: error.info().to_owned(),
        })?;
        let (start, end) = (mark(span.start), mark(span.end));
        let node = match event {
            Event::DocumentStart(_) => {
                documents += 1;
                continue;
            }
            Event::DocumentEnd if documents >= 1 && root.is_some() => break,
            Event::Scalar(value, style, _, _) => {
                // An empty value is the parser's `~` at the indicator before it: YamlDotNet reads it as empty.
                let literal = style != ScalarStyle::Plain
                    || value != "~"
                    || lines
                        .get(start.line.saturating_sub(1))
                        .and_then(|l| l.chars().nth(start.column - 1))
                        == Some('~');
                YamlNode::Scalar {
                    value: if literal { value.into_owned() } else { String::new() },
                    start,
                    end,
                }
            }
            Event::MappingStart(..) => {
                stack.push(Open::Mapping {
                    entries: Vec::new(),
                    key: None,
                    start,
                    end,
                });
                continue;
            }
            Event::SequenceStart(..) => {
                stack.push(Open::Sequence {
                    items: Vec::new(),
                    start,
                    end,
                });
                continue;
            }
            Event::MappingEnd => match stack.pop() {
                Some(Open::Mapping {
                    entries, start, end, ..
                }) => YamlNode::Mapping { entries, start, end },
                _ => unreachable!("the parser closes what it opened"),
            },
            Event::SequenceEnd => match stack.pop() {
                Some(Open::Sequence { items, start, end }) => YamlNode::Sequence { items, start, end },
                _ => unreachable!("the parser closes what it opened"),
            },
            Event::Alias(_) => YamlNode::Scalar {
                // An alias names a node declared earlier; it says nothing a supply chain reads.
                value: String::new(),
                start,
                end,
            },
            _ => continue,
        };
        match stack.last_mut() {
            None => {
                if root.is_none() {
                    root = Some(node);
                }
            }
            Some(Open::Sequence { items, .. }) => items.push(node),
            Some(Open::Mapping { entries, key, .. }) => match key.take() {
                None => *key = Some(node),
                Some(k) => {
                    if let YamlNode::Scalar { value, start, .. } = &k
                        && entries.iter().any(|(e, _)| e.as_scalar() == Some(value.as_str()))
                    {
                        return Err(YamlError {
                            line: start.line,
                            message: format!("Duplicate key '{value}'."),
                        });
                    }
                    entries.push((k, node));
                }
            },
        }
    }
    Ok(root)
}

/// The furthest end mark in `node`'s subtree, the first of equals by line.
fn end_mark(node: &YamlNode) -> Mark {
    let mut end = node.end();
    for child in node.children() {
        let child_end = end_mark(child);
        if child_end.line > end.line {
            end = child_end;
        }
    }
    end
}

/// A blank line, or one whose first character after its spaces is `#`.
fn says_nothing(text: &str) -> bool {
    text.trim().is_empty() || text.trim_start_matches(' ').starts_with('#')
}

/// The lines `node` occupies in `document` (`YamlNodeRange.Of`): from its start to the furthest end in its subtree,
/// stepped back off a mark in column one, with trailing blank and comment lines left out.
pub fn range_of(node: &YamlNode, document: &LineDocument) -> LineRange {
    let last = document.lines().len().saturating_sub(1);
    let extent = end_mark(node);
    let start = node.start().line.saturating_sub(1).min(last);
    let mut end = extent.line.saturating_sub(1).clamp(start, last.max(start));
    if extent.column == 1 && end > start {
        end -= 1;
    }
    while end > start && says_nothing(document.line(end)) {
        end -= 1;
    }
    LineRange::new(start, end)
}

/// The 0-based line `node` starts on, kept inside the document (`YamlAccess.LineOf`).
pub fn line_of(node: &YamlNode, document: &LineDocument) -> usize {
    node.start()
        .line
        .saturating_sub(1)
        .min(document.lines().len().saturating_sub(1))
}
