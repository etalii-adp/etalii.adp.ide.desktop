//! The YAML of an Ansible project: the first document of a file as a tree of nodes, each with the line it starts on
//! (etalii.adp spec 030, task T079).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/ansible-structure/backend/
//! EtAlii.Adp.Diagram.AnsibleStructure/AnsibleYaml.cs`, which parses a file it is handed with YamlDotNet's
//! representation model, and the core's `YamlAccess.Scalar`; the events of `saphyr-parser` are built into the same
//! tree here, as the supply chain module builds them. The marks are YamlDotNet's: 1-based lines and columns. A key
//! a mapping holds twice makes the file unreadable, as YamlDotNet's loader refuses it. Nothing here writes.

use saphyr_parser::{Event, Parser, ScalarStyle};

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
                // An alias names a node declared earlier; it says nothing the reader reads.
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
