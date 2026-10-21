//! The YAML editor's analysis (etalii.adp spec 030, FR-020, task T033), after the standalone host's YAML editor at
//! `8b6ccfc` (yaml-editor: `splitDocuments.ts`, `yamlReading.ts`, `yamlAnalysis.ts`).
//!
//! - **Documents**: a file may hold several, separated by `---` and ended by `...`, each read on its own.
//! - **The schema line** `# yaml-language-server: $schema=<reference>` before a document's first content names that
//!   document's schema, and only that document's; it is read from the project by the JSON editor's rules.
//! - **Findings**: a parser error, a tab used for indentation, a repeated key and an alias that names no anchor set
//!   before it (errors); aliases expanding past 100 nodes (error, and the document is not checked); a break of the
//!   schema (error); a schema that cannot be used (information, on the schema line).
//! - **Data**: plain values are typed as YAML 1.2's core schema reads them, merge keys (`<<`) are applied, and an
//!   application tag constructs nothing.
//!
//! Offsets are bytes into the whole text. The standalone host words the parser's findings as the `yaml` package does;
//! the wording here is saphyr's.

use std::collections::HashMap;
use std::ops::Range;

use adp_editor_json::{ReadFile, check_against_schema};
use saphyr_parser::{Event, Parser, ScalarStyle};
use serde_json::{Map, Number, Value};

/// The parser's bound on alias expansion, as in the standalone host.
pub const MAX_ALIAS_COUNT: usize = 100;
/// Above this many characters nothing is checked.
pub const LARGEST_CHECKED_TEXT: usize = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub span: Range<usize>,
    pub severity: Severity,
    pub message: String,
}

/// One document of a file: its byte range and the schema line found before its first content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    pub range: Range<usize>,
    pub schema_reference: Option<String>,
    pub schema_line: Option<Range<usize>>,
}

/// Splits a text into its documents at `---` and `...` markers at the start of a line; one inside a block scalar is
/// indented and is text. An empty text is one empty part.
pub fn split_documents(text: &str) -> Vec<Part> {
    let mut parts = Vec::new();
    let mut current = Part {
        range: 0..text.len(),
        schema_reference: None,
        schema_line: None,
    };
    let mut holds_content = false;
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let bare = line.trim_end_matches(['\n', '\r']);
        let is_start = bare == "---" || bare.starts_with("--- ") || bare.starts_with("---\t");
        let is_end = bare == "..." || bare.starts_with("... ");
        if is_start {
            if holds_content {
                current.range.end = offset;
                parts.push(current);
                current = Part {
                    range: offset..text.len(),
                    schema_reference: None,
                    schema_line: None,
                };
            }
            holds_content = true;
        } else if is_end {
            current.range.end = offset + line.len();
            parts.push(current);
            current = Part {
                range: offset + line.len()..text.len(),
                schema_reference: None,
                schema_line: None,
            };
            holds_content = false;
        } else if let Some(reference) = schema_line(bare) {
            if current.schema_reference.is_none() && !has_content_before(&text[current.range.start..offset]) {
                current.schema_reference = Some(reference);
                current.schema_line = Some(offset..offset + bare.len());
            }
        } else if !bare.trim().is_empty() && !bare.trim_start().starts_with('#') {
            holds_content = true;
        }
        offset += line.len();
    }
    if current.range.start < text.len() || parts.is_empty() {
        current.range.end = text.len();
        parts.push(current);
    }
    parts
}

fn has_content_before(text: &str) -> bool {
    text.lines().any(|l| {
        let t = l.trim();
        !t.is_empty() && !t.starts_with('#') && t != "---" && !t.starts_with("--- ")
    })
}

/// `# yaml-language-server: $schema=<reference>`, the reference running to the end of the comment.
fn schema_line(line: &str) -> Option<String> {
    let rest = line.trim_end().strip_prefix('#')?.trim_start();
    let rest = rest
        .strip_prefix("yaml-language-server")?
        .trim_start()
        .strip_prefix(':')?
        .trim_start();
    Some(rest.strip_prefix("$schema=")?.trim().to_owned())
}

/// A value of a document, with where it stands.
#[derive(Debug, Clone, PartialEq)]
enum Node {
    Map {
        span: Range<usize>,
        entries: Vec<(String, Range<usize>, Node)>,
    },
    Seq {
        span: Range<usize>,
        items: Vec<Node>,
    },
    Scalar {
        span: Range<usize>,
        value: Value,
    },
    /// An alias, resolved to the anchored node's value.
    Alias {
        span: Range<usize>,
        value: Value,
    },
}

impl Node {
    fn span(&self) -> Range<usize> {
        match self {
            Node::Map { span, .. } | Node::Seq { span, .. } | Node::Scalar { span, .. } | Node::Alias { span, .. } => {
                span.clone()
            }
        }
    }

    fn to_value(&self) -> Value {
        match self {
            Node::Map { entries, .. } => {
                let mut map = Map::new();
                // The mapping's own keys first, so they win over what a merge key brings in.
                for (key, _, value) in entries.iter().filter(|(k, _, _)| k != "<<") {
                    map.insert(key.clone(), value.to_value());
                }
                for (_, _, value) in entries.iter().filter(|(k, _, _)| k == "<<") {
                    let merged = match value.to_value() {
                        Value::Array(items) => items,
                        other => vec![other],
                    };
                    for object in merged
                        .into_iter()
                        .filter_map(|m| if let Value::Object(o) = m { Some(o) } else { None })
                    {
                        for (k, v) in object {
                            map.entry(k).or_insert(v);
                        }
                    }
                }
                Value::Object(map)
            }
            Node::Seq { items, .. } => Value::Array(items.iter().map(Node::to_value).collect()),
            Node::Scalar { value, .. } | Node::Alias { value, .. } => value.clone(),
        }
    }

    /// The node at a JSON pointer, with the key's span when the last step is a key.
    fn at_pointer(&self, pointer: &str) -> Option<(&Node, Option<Range<usize>>)> {
        let mut node = self;
        let mut key_span = None;
        for raw in pointer.split('/').skip(1) {
            let step = raw.replace("~1", "/").replace("~0", "~");
            match node {
                Node::Map { entries, .. } => {
                    let (_, span, value) = entries.iter().rev().find(|(k, _, _)| *k == step)?;
                    key_span = Some(span.clone());
                    node = value;
                }
                Node::Seq { items, .. } => {
                    node = items.get(step.parse::<usize>().ok()?)?;
                    key_span = None;
                }
                _ => return None,
            }
        }
        Some((node, key_span))
    }
}

/// What the editor knows about a YAML text.
#[derive(Debug, Clone, PartialEq)]
pub struct Analysis {
    pub findings: Vec<Finding>,
    pub status: String,
}

/// Analyses a YAML text; `document` is its path in the project, its file name last.
pub fn analyse(text: &str, document: &[String], read: ReadFile) -> Analysis {
    if text.len() > LARGEST_CHECKED_TEXT {
        return Analysis {
            findings: Vec::new(),
            status: "Too large to check".into(),
        };
    }
    let mut findings = tab_indentation(text);
    let mut statuses = Vec::new();
    for part in split_documents(text) {
        let reading = read_part(&text[part.range.clone()], part.range.start);
        let readable = reading.readable && !findings.iter().any(|f| part.range.contains(&f.span.start));
        findings.extend(reading.findings);
        let Some(reference) = &part.schema_reference else {
            continue;
        };
        let line = part.schema_line.clone().unwrap_or(part.range.start..part.range.start);
        let (true, Some(root)) = (readable, reading.root) else {
            continue;
        };
        let check = check_against_schema(reference, document, read.clone(), root.to_value(), false);
        for (pointer, message) in check.breaks {
            let span = match root.at_pointer(&pointer) {
                Some((_, Some(key))) => key,
                Some((Node::Map { span, entries }, None)) => message
                    .split('\'')
                    .skip(1)
                    .step_by(2)
                    .find_map(|name| entries.iter().find(|(k, _, _)| k == name).map(|(_, s, _)| s.clone()))
                    .unwrap_or(span.start..span.start + 1),
                Some((node, None)) => node.span(),
                None => root.span(),
            };
            findings.push(Finding {
                span,
                severity: Severity::Error,
                message,
            });
        }
        if let Some(reason) = check.refusal {
            findings.push(Finding {
                span: line,
                severity: Severity::Info,
                message: reason,
            });
        }
        statuses.push(check.status);
    }
    findings.sort_by_key(|f| (f.span.start, f.span.end));
    findings.dedup();
    let status = if statuses.is_empty() {
        "No schema".to_owned()
    } else {
        statuses.join(" · ")
    };
    Analysis { findings, status }
}

/// A tab in a line's indentation is an error: YAML indents with spaces. Lines inside a block scalar are text.
fn tab_indentation(text: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut offset = 0;
    let mut block_scalar_parent: Option<usize> = None;
    for line in text.split_inclusive('\n') {
        let indent: String = line.chars().take_while(|c| *c == ' ' || *c == '\t').collect();
        let spaces = indent.chars().take_while(|c| *c == ' ').count();
        if let Some(parent) = block_scalar_parent {
            if line.trim().is_empty() || spaces > parent {
                offset += line.len();
                continue;
            }
            block_scalar_parent = None;
        }
        if let Some(at) = indent.find('\t')
            && !line.trim().is_empty()
        {
            findings.push(Finding {
                span: offset + at..offset + at + 1,
                severity: Severity::Error,
                message: "Tabs are not allowed as indentation; indent with spaces.".into(),
            });
        }
        let bare = line.trim_end();
        if bare.ends_with('|') || bare.ends_with('>') || bare.ends_with("|-") || bare.ends_with(">-") {
            block_scalar_parent = Some(spaces);
        }
        offset += line.len();
    }
    findings
}

struct PartReading {
    root: Option<Node>,
    findings: Vec<Finding>,
    readable: bool,
}

enum Frame {
    Map {
        start: usize,
        anchor: usize,
        entries: Vec<(String, Range<usize>, Node)>,
        key: Option<(String, Range<usize>)>,
    },
    Seq {
        start: usize,
        anchor: usize,
        items: Vec<Node>,
    },
}

/// Reads one document with saphyr's event parser, building its values with spans.
fn read_part(text: &str, base: usize) -> PartReading {
    let byte_of: Vec<usize> = text
        .char_indices()
        .map(|(b, _)| b)
        .chain(std::iter::once(text.len()))
        .collect();
    let at = |chars: usize| base + byte_of.get(chars).copied().unwrap_or(text.len());
    let mut findings = Vec::new();
    let mut readable = true;
    let mut stack: Vec<Frame> = Vec::new();
    let mut root: Option<Node> = None;
    let mut anchors: HashMap<usize, Value> = HashMap::new();
    let mut aliases = 0usize;

    for result in Parser::new_from_str(text) {
        let (event, span) = match result {
            Ok(pair) => pair,
            Err(error) => {
                let start = at(error.marker().index()).min((base + text.len()).saturating_sub(1).max(base));
                findings.push(Finding {
                    span: start..start + 1,
                    severity: Severity::Error,
                    message: sentence(error.info()),
                });
                readable = false;
                break;
            }
        };
        let limit = base + text.len();
        let start = at(span.start.index()).min(limit.saturating_sub(1).max(base));
        let range = start..at(span.end.index()).max(start + 1).min(limit.max(start + 1));
        let node = match event {
            Event::Scalar(value, style, anchor, tag) => {
                let typed = if style == ScalarStyle::Plain && tag.is_none() {
                    core_schema(&value)
                } else {
                    Value::String(value.to_string())
                };
                if anchor > 0 {
                    anchors.insert(anchor, typed.clone());
                }
                Some(Node::Scalar {
                    span: range,
                    value: typed,
                })
            }
            Event::Alias(anchor) => {
                aliases += 1;
                if aliases == MAX_ALIAS_COUNT + 1 {
                    findings.push(Finding {
                        span: range.clone(),
                        severity: Severity::Error,
                        message: format!("The aliases of this document expand past {MAX_ALIAS_COUNT} nodes."),
                    });
                    readable = false;
                }
                match anchors.get(&anchor) {
                    Some(value) => Some(Node::Alias {
                        span: range,
                        value: value.clone(),
                    }),
                    None => {
                        let source = text.get(range.start - base..range.end - base).unwrap_or_default();
                        let name = source.trim_start_matches('*').trim();
                        findings.push(Finding {
                            span: range.clone(),
                            severity: Severity::Error,
                            message: format!("The alias *{name} names no anchor set before it."),
                        });
                        Some(Node::Alias {
                            span: range,
                            value: Value::Null,
                        })
                    }
                }
            }
            Event::MappingStart(anchor, _) => {
                stack.push(Frame::Map {
                    start,
                    anchor,
                    entries: Vec::new(),
                    key: None,
                });
                None
            }
            Event::SequenceStart(anchor, _) => {
                stack.push(Frame::Seq {
                    start,
                    anchor,
                    items: Vec::new(),
                });
                None
            }
            Event::MappingEnd | Event::SequenceEnd => {
                let end = range.end;
                stack.pop().map(|frame| {
                    let (node, anchor) = match frame {
                        Frame::Map {
                            start, anchor, entries, ..
                        } => (
                            Node::Map {
                                span: start..end.max(start + 1),
                                entries,
                            },
                            anchor,
                        ),
                        Frame::Seq { start, anchor, items } => (
                            Node::Seq {
                                span: start..end.max(start + 1),
                                items,
                            },
                            anchor,
                        ),
                    };
                    if anchor > 0 {
                        anchors.insert(anchor, node.to_value());
                    }
                    node
                })
            }
            _ => None,
        };
        let Some(node) = node else { continue };
        match stack.last_mut() {
            None => root = Some(node),
            Some(Frame::Seq { items, .. }) => items.push(node),
            Some(Frame::Map { entries, key, .. }) => match key.take() {
                None => {
                    let name = match &node {
                        Node::Scalar {
                            value: Value::String(s),
                            ..
                        } => s.clone(),
                        Node::Scalar { value, .. } | Node::Alias { value, .. } => value.to_string(),
                        _ => String::new(),
                    };
                    if name != "<<" && entries.iter().any(|(k, _, _)| *k == name) {
                        findings.push(Finding {
                            span: node.span(),
                            severity: Severity::Error,
                            message: "Map keys must be unique".into(),
                        });
                    }
                    *key = Some((name, node.span()));
                }
                Some((name, span)) => entries.push((name, span, node)),
            },
        }
    }
    PartReading {
        root,
        findings,
        readable,
    }
}

/// A plain value as YAML 1.2's core schema types it.
fn core_schema(text: &str) -> Value {
    match text {
        "" | "~" | "null" | "Null" | "NULL" => return Value::Null,
        "true" | "True" | "TRUE" => return Value::Bool(true),
        "false" | "False" | "FALSE" => return Value::Bool(false),
        _ => {}
    }
    let unsigned = text.trim_start_matches(['+', '-']);
    let integer = if let Some(octal) = text.strip_prefix("0o") {
        i64::from_str_radix(octal, 8).ok()
    } else if let Some(hex) = text.strip_prefix("0x") {
        i64::from_str_radix(hex, 16).ok()
    } else if !unsigned.is_empty() && unsigned.chars().all(|c| c.is_ascii_digit()) {
        text.parse::<i64>().ok()
    } else {
        None
    };
    if let Some(n) = integer {
        return Value::Number(n.into());
    }
    let looks_float = unsigned.chars().next().is_some_and(|c| c.is_ascii_digit() || c == '.')
        && unsigned
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | '+' | '-'));
    if looks_float && let Some(n) = text.parse::<f64>().ok().and_then(Number::from_f64) {
        return Value::Number(n);
    }
    Value::String(text.to_owned())
}

/// The parser's message as a sentence.
fn sentence(info: &str) -> String {
    let mut text = info.trim().to_owned();
    if let Some(first) = text.get(..1) {
        let upper = first.to_uppercase();
        text.replace_range(..1, &upper);
    }
    if !text.ends_with('.') {
        text.push('.');
    }
    text
}
