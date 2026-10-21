//! The YAML a Databricks file is read from: its first document as a tree of nodes, each with where it starts and
//! ends, and the lines a node occupies (etalii.adp spec 030, task T081).
//!
//! Ported from what the standalone host's Databricks module read through at `8b6ccfc`: YamlDotNet's representation
//! model (`YamlStream`, whose aliases resolve to the node their anchor names), the core's
//! `src/backend/EtAlii.Adp.Documents/YamlAccess.cs` and `YamlNodeRange.cs`. The events of `saphyr-parser` are built into
//! the same tree, as the Azure DevOps pipeline module builds its own. JSON is a subset of YAML 1.2, so a pipeline's
//! settings `.json` is read through the same door. YAML is only ever read: every write is a splice of lines
//! ([`crate::lines`]).
//!
//! The marks are those YamlDotNet's representation model gives: a scalar starts where it starts and ends after its
//! last character, and a collection's own end is the end of the event that opened it, so the lines a node occupies are
//! found from the furthest end in its subtree ([`range_of`]). An alias is the node its anchor named, declared earlier
//! in the text, so following one never stretches a range past where the node holding it ends. A key that a mapping
//! holds twice makes the text unreadable, as YamlDotNet's loader refuses it.

use std::collections::HashMap;

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

/// What is still being built, with the anchor it will be kept under (0 for none).
enum Open {
    Mapping {
        entries: Vec<(YamlNode, YamlNode)>,
        key: Option<YamlNode>,
        start: Mark,
        end: Mark,
        anchor: usize,
    },
    Sequence {
        items: Vec<YamlNode>,
        start: Mark,
        end: Mark,
        anchor: usize,
    },
}

fn mark(marker: saphyr_parser::Marker) -> Mark {
    Mark {
        line: marker.line(),
        column: marker.col() + 1,
    }
}

/// The root of the first document of `text`, or none for a text with no document; why it is not YAML when it is not,
/// in any of its documents, as `YamlStream.Load` reads them all.
pub fn parse(text: &str) -> Result<Option<YamlNode>, YamlError> {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut parser = Parser::new_from_str(text);
    let mut stack: Vec<Open> = Vec::new();
    let mut anchors: HashMap<usize, YamlNode> = HashMap::new();
    let mut root: Option<YamlNode> = None;
    let mut documents = 0;
    while let Some(event) = parser.next_event() {
        let (event, span) = event.map_err(|error| YamlError {
            line: error.marker().line(),
            message: error.info().to_owned(),
        })?;
        let (start, end) = (mark(span.start), mark(span.end));
        let (node, anchor) = match event {
            Event::DocumentStart(_) => {
                documents += 1;
                continue;
            }
            Event::Scalar(value, style, anchor, _) => {
                // An empty value is the parser's `~` at the indicator before it: YamlDotNet reads it as empty.
                let literal = style != ScalarStyle::Plain
                    || value != "~"
                    || lines
                        .get(start.line.saturating_sub(1))
                        .and_then(|l| l.chars().nth(start.column - 1))
                        == Some('~');
                (
                    YamlNode::Scalar {
                        value: if literal { value.into_owned() } else { String::new() },
                        start,
                        end,
                    },
                    anchor,
                )
            }
            Event::MappingStart(anchor, _) => {
                stack.push(Open::Mapping {
                    entries: Vec::new(),
                    key: None,
                    start,
                    end,
                    anchor,
                });
                continue;
            }
            Event::SequenceStart(anchor, _) => {
                stack.push(Open::Sequence {
                    items: Vec::new(),
                    start,
                    end,
                    anchor,
                });
                continue;
            }
            Event::MappingEnd => match stack.pop() {
                Some(Open::Mapping {
                    entries,
                    start,
                    end,
                    anchor,
                    ..
                }) => (YamlNode::Mapping { entries, start, end }, anchor),
                _ => unreachable!("the parser closes what it opened"),
            },
            Event::SequenceEnd => match stack.pop() {
                Some(Open::Sequence {
                    items,
                    start,
                    end,
                    anchor,
                }) => (YamlNode::Sequence { items, start, end }, anchor),
                _ => unreachable!("the parser closes what it opened"),
            },
            Event::Alias(id) => match anchors.get(&id) {
                // The node its anchor named, as the representation model resolves it.
                Some(node) => (node.clone(), 0),
                None => {
                    return Err(YamlError {
                        line: start.line,
                        message: "An alias names an anchor that was not declared before it.".to_owned(),
                    });
                }
            },
            _ => continue,
        };
        if anchor != 0 {
            anchors.insert(anchor, node.clone());
        }
        match stack.last_mut() {
            None => {
                if documents == 1 && root.is_none() {
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

/// The lines a mapping entry occupies, its key and its value together (`YamlNodeRange.Of(key, value)`).
pub fn entry_range(key: &YamlNode, value: &YamlNode, document: &LineDocument) -> LineRange {
    let (k, v) = (range_of(key, document), range_of(value, document));
    LineRange::new(k.start.min(v.start), k.end.max(v.end))
}

impl YamlNode {
    /// A mapping's entries; none for any other node.
    pub fn entries(&self) -> &[(YamlNode, YamlNode)] {
        match self {
            YamlNode::Mapping { entries, .. } => entries,
            _ => &[],
        }
    }

    /// Whether the node is a mapping.
    pub fn is_mapping(&self) -> bool {
        matches!(self, YamlNode::Mapping { .. })
    }

    /// The node under the scalar key `key` of a mapping, of whatever kind (`YamlAccess.Node`).
    pub fn get(&self, key: &str) -> Option<&YamlNode> {
        self.entries()
            .iter()
            .find(|(k, _)| k.as_scalar() == Some(key))
            .map(|(_, v)| v)
    }

    /// Whether a mapping holds the key `key`.
    pub fn contains_key(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// The mapping under `key`, or none where there is none (`YamlAccess.Mapping`).
    pub fn mapping(&self, key: &str) -> Option<&YamlNode> {
        self.get(key).filter(|n| n.is_mapping())
    }

    /// The items of the sequence under `key`, none where there is none (`YamlAccess.Sequence`).
    pub fn sequence(&self, key: &str) -> &[YamlNode] {
        match self.get(key) {
            Some(YamlNode::Sequence { items, .. }) => items,
            _ => &[],
        }
    }

    /// The scalar under `key`, or none where there is none or it is not a scalar (`YamlAccess.Scalar`).
    pub fn scalar(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(YamlNode::as_scalar)
    }

    /// A boolean scalar read leniently, as `bool.TryParse` reads it: only a value that says so is true
    /// (`YamlAccess.Flag`).
    pub fn flag(&self, key: &str) -> bool {
        self.scalar(key).is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))
    }
}
