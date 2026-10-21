//! The YAML a `.skv` document is read as: its first document as a tree of nodes, each with where it starts and ends
//! (etalii.adp spec 030, task T077).
//!
//! The standalone host at `8b6ccfc` read `.skv` with YamlDotNet's representation model through the core's
//! `YamlAccess.cs` and `YamlNodeRange.cs`, and only read with it: every write is a splice of lines. Here saphyr's
//! event parser reads it, and the tree keeps what those two used of YamlDotNet's nodes: a scalar's value as written
//! (`1` is the text `1`; an empty value is the empty text), a mapping's entries in their order, a key written twice
//! refused as YamlDotNet refuses it, and each node's start and end, from which [`range_of`] takes the lines that
//! declare it as `YamlNodeRange.Of` took them.

use std::collections::HashMap;

use saphyr_parser::{Event, Parser};

use crate::lines::{LineDocument, LineRange};

/// A place in the text: a character index and its zero-based line, and whether it is the first character of its line
/// (YamlDotNet's column 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Mark {
    pub line: usize,
    pub at_line_start: bool,
}

/// One node of the document.
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

    /// The node under the scalar key `key`, of whatever kind (`YamlAccess.Node`).
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
        match self.get(key) {
            Some(YamlNode::Scalar { value, .. }) => Some(value),
            _ => None,
        }
    }

    /// The scalar keys of a mapping, each with its node.
    pub fn keys(&self) -> Vec<(&str, &YamlNode)> {
        match self {
            YamlNode::Mapping { entries, .. } => entries
                .iter()
                .filter_map(|(k, _)| match k {
                    YamlNode::Scalar { value, .. } => Some((value.as_str(), k)),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Every child: a mapping's keys and values, a sequence's items.
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

    /// The furthest end in the node's subtree, by line.
    fn extent(&self) -> Mark {
        let mut end = self.end();
        for child in self.children() {
            let child_end = child.extent();
            if child_end.line > end.line {
                end = child_end;
            }
        }
        end
    }
}

/// Why the text is not YAML: the zero-based line it was found on and the parser's sentence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlError {
    pub line: usize,
    pub message: String,
}

/// The first document of `text`: its root node, none for a text with no document, or why it is not YAML.
pub fn parse(text: &str) -> Result<Option<YamlNode>, YamlError> {
    // saphyr counts in characters.
    let mut line_of_char = Vec::with_capacity(text.len() + 1);
    let mut line_starts = Vec::with_capacity(text.len() + 1);
    let (mut line, mut at_start) = (0, true);
    for c in text.chars() {
        line_of_char.push(line);
        line_starts.push(at_start);
        at_start = c == '\n';
        if c == '\n' {
            line += 1;
        }
    }
    line_of_char.push(line);
    line_starts.push(at_start);
    let mark = |index: usize| {
        let index = index.min(line_of_char.len() - 1);
        Mark {
            line: line_of_char[index],
            at_line_start: line_starts[index],
        }
    };

    enum Frame {
        Mapping {
            start: Mark,
            anchor: usize,
            entries: Vec<(YamlNode, YamlNode)>,
            key: Option<YamlNode>,
        },
        Sequence {
            start: Mark,
            anchor: usize,
            items: Vec<YamlNode>,
        },
    }

    let mut stack: Vec<Frame> = Vec::new();
    let mut anchors: HashMap<usize, YamlNode> = HashMap::new();
    let mut root: Option<YamlNode> = None;
    let mut documents = 0;
    for result in Parser::new_from_str(text) {
        let (event, span) = result.map_err(|error| YamlError {
            line: mark(error.marker().index()).line,
            message: format!(
                "(Line: {}, Col: {}, Idx: {}): {}",
                mark(error.marker().index()).line + 1,
                error.marker().col() + 1,
                error.marker().index(),
                error.info()
            ),
        })?;
        let (start, end) = (mark(span.start.index()), mark(span.end.index()));
        let complete = match event {
            Event::DocumentStart(_) => {
                documents += 1;
                if documents > 1 {
                    // Only the first document is read, as `YamlStream.Documents[0]`.
                    break;
                }
                None
            }
            Event::Scalar(value, _, anchor, _) => {
                // An empty value is the empty text, as YamlDotNet reads it; saphyr reads it as `~`, with no extent.
                let value = if span.start.index() == span.end.index() && value == "~" {
                    String::new()
                } else {
                    value.into_owned()
                };
                let node = YamlNode::Scalar { value, start, end };
                if anchor > 0 {
                    anchors.insert(anchor, node.clone());
                }
                Some(node)
            }
            Event::Alias(anchor) => Some(anchors.get(&anchor).cloned().ok_or_else(|| YamlError {
                line: start.line,
                message: "An alias names no anchor set before it.".to_owned(),
            })?),
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
            Event::MappingEnd | Event::SequenceEnd => match stack.pop() {
                // A block collection has no end of its own: YamlDotNet's node ends where it starts, and its lines
                // run to the furthest end in it.
                Some(Frame::Mapping {
                    start, anchor, entries, ..
                }) => {
                    let node = YamlNode::Mapping {
                        entries,
                        start,
                        end: start,
                    };
                    if anchor > 0 {
                        anchors.insert(anchor, node.clone());
                    }
                    Some(node)
                }
                Some(Frame::Sequence { start, anchor, items }) => {
                    let node = YamlNode::Sequence {
                        items,
                        start,
                        end: start,
                    };
                    if anchor > 0 {
                        anchors.insert(anchor, node.clone());
                    }
                    Some(node)
                }
                None => None,
            },
            _ => None,
        };
        let Some(node) = complete else {
            continue;
        };
        match stack.last_mut() {
            Some(Frame::Mapping { entries, key, .. }) => match key.take() {
                None => {
                    if let YamlNode::Scalar { value, start, .. } = &node
                        && entries
                            .iter()
                            .any(|(k, _)| matches!(k, YamlNode::Scalar { value: v, .. } if v == value))
                    {
                        return Err(YamlError {
                            line: start.line,
                            message: format!("(Line: {}): Duplicate key", start.line + 1),
                        });
                    }
                    *key = Some(node);
                }
                Some(k) => entries.push((k, node)),
            },
            Some(Frame::Sequence { items, .. }) => items.push(node),
            None => {
                if root.is_none() {
                    root = Some(node);
                }
            }
        }
    }
    Ok(root)
}

/// The zero-based line `node` starts on, kept inside the document (`YamlAccess.LineOf`).
pub fn line_of(node: &YamlNode, document: &LineDocument) -> usize {
    node.start().line.min(document.lines().len().saturating_sub(1))
}

/// The lines that declare `node` (`YamlNodeRange.Of`): from its start to the furthest end in its subtree, stepped
/// back off an end at the start of a line, then without the blank and comment lines at its end.
pub fn range_of(node: &YamlNode, document: &LineDocument) -> LineRange {
    let last = document.lines().len().saturating_sub(1);
    let extent = node.extent();
    let start = node.start().line.min(last);
    let mut end = extent.line.clamp(start, last.max(start));
    if extent.at_line_start && end > start {
        end -= 1;
    }
    while end > start && says_nothing(document.line(end)) {
        end -= 1;
    }
    LineRange::new(start, end)
}

/// A blank line, or a YAML comment line: the first character that is not a space is `#`.
fn says_nothing(text: &str) -> bool {
    text.trim().is_empty() || text.trim_start_matches(' ').starts_with('#')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_entry_runs_from_its_first_key_to_its_last_value() {
        let text = "sankey: 1\nnodes:\n  - id: a\n    name: A\n\n  # the next one\n  - id: b\n";
        let document = LineDocument::parse(text);
        let root = parse(text).unwrap().unwrap();
        let Some(YamlNode::Sequence { items, .. }) = root.get("nodes") else {
            panic!("nodes is a sequence");
        };
        assert_eq!(range_of(&items[0], &document), LineRange::new(2, 3));
        assert_eq!(range_of(&items[1], &document), LineRange::new(6, 6));
        assert_eq!(root.scalar("sankey"), Some("1"));
    }

    #[test]
    fn an_empty_value_is_the_empty_text_and_a_key_twice_is_not_yaml() {
        let root = parse("a:\nb: ~\n").unwrap().unwrap();
        assert_eq!(root.scalar("a"), Some(""));
        assert_eq!(root.scalar("b"), Some("~"));
        assert!(parse("a: 1\na: 2\n").is_err());
        assert!(parse("a: [\n").is_err());
        assert_eq!(parse("").unwrap(), None);
    }
}
