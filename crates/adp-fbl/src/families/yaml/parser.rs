//! The lossless reading of a YAML body's first document (FBL section 4.3; the standalone host's
//! `Files/Yaml/YamlParser`): block mappings and sequences, plain, quoted and block scalars, flow collections as one
//! value whose items are also entries when it is a sequence (FBL 0.10), anchors and aliases, every node with its span as
//! written. The standalone host had YamlDotNet judge
//! well-formedness before this ran; here this parser's own errors are what makes a body unreadable.

use std::collections::{HashMap, HashSet};

use adp_cel::Value;

use super::{flow, scalars};
use crate::families::tree::{Reading, TreeEntry, TreeValue, ValueId, ValueKind, ValueStyle, cel_of};
use crate::families::{Entry, EntryId, EntryKind};
use crate::text::{BodyText, Span};

/// Where and why the body is not the YAML this host reads.
#[derive(Debug, Clone)]
pub(crate) struct YamlError {
    pub offset: usize,
    pub message: String,
}

fn fail<T>(offset: usize, message: impl Into<String>) -> Result<T, YamlError> {
    Err(YamlError {
        offset,
        message: message.into(),
    })
}

/// What the parser read: the root value, every entry and value it made, the leaves, and the duplicate keys (the
/// key spans of the second and later occurrences).
pub(crate) struct Parsed {
    pub root: ValueId,
    pub entries: Vec<Entry>,
    pub values: Vec<TreeValue>,
    pub leaves: Vec<Span>,
    pub duplicates: Vec<(String, Span)>,
}

pub(crate) fn parse(text: &BodyText) -> Result<Parsed, YamlError> {
    let mut parser = Parser {
        text,
        bytes: text.bytes(),
        anchors: HashMap::new(),
        line: 0,
        end: 0,
        entries: Vec::new(),
        values: Vec::new(),
        leaves: Vec::new(),
        duplicates: Vec::new(),
    };
    let root = parser.parse_document()?;
    Ok(Parsed {
        root,
        entries: parser.entries,
        values: parser.values,
        leaves: parser.leaves,
        duplicates: parser.duplicates,
    })
}

struct Key {
    name: String,
    span: Span,
    colon: usize,
}

struct Parser<'a> {
    text: &'a BodyText,
    bytes: &'a [u8],
    anchors: HashMap<String, ValueId>,
    line: usize,
    end: usize,
    entries: Vec<Entry>,
    values: Vec<TreeValue>,
    leaves: Vec<Span>,
    duplicates: Vec<(String, Span)>,
}

fn space(b: u8) -> bool {
    b == b' ' || b == b'\t'
}

impl Parser<'_> {
    fn parse_document(&mut self) -> Result<ValueId, YamlError> {
        let count = self.text.lines().len();
        self.line = 0;
        while self.line < count {
            let p = self.first_non_space(self.line);
            if p < self.content_end(self.line) && self.bytes[p] == b'%' {
                self.line += 1;
                continue;
            }
            if self.is_marker(self.line, "---") {
                self.line += 1;
                break;
            }
            if self.is_significant(self.line) {
                break;
            }
            self.line += 1;
        }
        self.end = count;
        for i in self.line..count {
            if self.is_marker(i, "---") || self.is_marker(i, "...") {
                self.end = i;
                break;
            }
        }
        let document_end = if self.end < count {
            self.text.lines()[self.end].start
        } else {
            self.text.len()
        };
        if document_end < self.text.len() {
            self.leaves.push(Span::new(document_end, self.text.len()));
        }
        let root = match self.parse_block_node(-1, false)? {
            Some(root) => root,
            None => self.empty(self.text.bom_length()),
        };
        let rest = self.next_significant(self.line);
        if rest < self.end {
            return fail(
                self.first_non_space(rest),
                "This line does not continue the structure above it.",
            );
        }
        Ok(root)
    }

    // ---- lines ----

    fn line_start(&self, line: usize) -> usize {
        if line == 0 {
            self.text.lines()[0].start.max(self.text.bom_length())
        } else {
            self.text.lines()[line].start
        }
    }

    fn content_end(&self, line: usize) -> usize {
        self.text.lines()[line].content_end
    }

    fn first_non_space(&self, line: usize) -> usize {
        let mut p = self.line_start(line);
        let end = self.content_end(line);
        while p < end && space(self.bytes[p]) {
            p += 1;
        }
        p
    }

    fn is_significant(&self, line: usize) -> bool {
        let p = self.first_non_space(line);
        p < self.content_end(line) && self.bytes[p] != b'#'
    }

    fn next_significant(&self, from: usize) -> usize {
        let mut line = from;
        while line < self.end && !self.is_significant(line) {
            line += 1;
        }
        line
    }

    fn is_marker(&self, line: usize, marker: &str) -> bool {
        let start = self.line_start(line);
        let end = self.content_end(line);
        if end - start < 3 || &self.bytes[start..start + 3] != marker.as_bytes() {
            return false;
        }
        start + 3 == end || space(self.bytes[start + 3])
    }

    fn column(&self, line: usize, offset: usize) -> i64 {
        (offset - self.line_start(line)) as i64
    }

    fn is_dash(&self, line: usize, p: usize) -> bool {
        self.bytes[p] == b'-' && (p + 1 == self.content_end(line) || space(self.bytes[p + 1]))
    }

    fn at_line_end(&self, line: usize, mut p: usize) -> bool {
        while p < self.content_end(line) && space(self.bytes[p]) {
            p += 1;
        }
        p >= self.content_end(line)
            || (self.bytes[p] == b'#' && (p == self.line_start(line) || space(self.bytes[p - 1])))
    }

    fn skip_spaces(&self, line: usize, mut p: usize) -> usize {
        while p < self.content_end(line) && space(self.bytes[p]) {
            p += 1;
        }
        p
    }

    fn add_value(&mut self, value: TreeValue) -> ValueId {
        self.values.push(value);
        self.values.len() - 1
    }

    fn add_entry(
        &mut self,
        own: Span,
        indent: usize,
        name: Option<String>,
        key_span: Option<Span>,
        value: ValueId,
    ) -> EntryId {
        self.entries.push(Entry {
            own,
            line_span: None,
            parent: None,
            children: Vec::new(),
            name,
            indent,
            kind: EntryKind::Tree(TreeEntry {
                is_root: false,
                key_span,
                value,
                separator: None,
            }),
        });
        self.entries.len() - 1
    }

    fn empty(&mut self, offset: usize) -> ValueId {
        self.add_value(TreeValue::scalar(
            ValueStyle::Empty,
            Span::new(offset, offset),
            String::new(),
            Value::Null,
        ))
    }

    fn end_of(&self, value: ValueId, fallback: usize) -> usize {
        let v = &self.values[value];
        if v.style == ValueStyle::Empty {
            fallback
        } else {
            v.span.end
        }
    }

    // ---- block structure ----

    fn parse_block_node(&mut self, parent_indent: i64, sequence_at_parent: bool) -> Result<Option<ValueId>, YamlError> {
        let line = self.next_significant(self.line);
        if line >= self.end {
            return Ok(None);
        }
        let p = self.first_non_space(line);
        let column = self.column(line, p);
        if column < parent_indent {
            return Ok(None);
        }
        if column == parent_indent && !(sequence_at_parent && self.is_dash(line, p)) {
            return Ok(None);
        }
        self.line = line;
        if self.is_dash(line, p) {
            return self.parse_sequence(line, p, column).map(Some);
        }
        if self.key_at(line, p)?.is_some() {
            return self.parse_mapping(line, p, column).map(Some);
        }
        self.parse_inline(line, p, parent_indent).map(Some)
    }

    fn parse_sequence(&mut self, mut line: usize, mut p: usize, column: i64) -> Result<ValueId, YamlError> {
        let value = self.add_value(TreeValue::new(ValueKind::Sequence, ValueStyle::Block, Span::new(p, p)));
        loop {
            let dash = p;
            let q = self.skip_spaces(line, p + 1);
            let item = if self.at_line_end(line, q) {
                self.line = line + 1;
                match self.parse_block_node(column, false)? {
                    Some(item) => item,
                    None => self.empty(dash + 1),
                }
            } else {
                self.line = line;
                self.parse_content_after_indicator(line, q, column)?
            };
            let own = Span::new(dash, self.end_of(item, dash + 1));
            let entry = self.add_entry(own, column as usize, None, None, item);
            self.values[value].entries.push(entry);
            let next = self.next_significant(self.line);
            if next >= self.end {
                break;
            }
            let np = self.first_non_space(next);
            if self.column(next, np) != column || !self.is_dash(next, np) {
                break;
            }
            line = next;
            p = np;
            self.line = next;
        }
        let first = self.values[value].entries[0];
        let last = *self.values[value].entries.last().expect("a sequence has an item");
        self.values[value].span = Span::new(self.entries[first].own.start, self.entries[last].own.end);
        Ok(value)
    }

    fn parse_content_after_indicator(
        &mut self,
        line: usize,
        q: usize,
        parent_indent: i64,
    ) -> Result<ValueId, YamlError> {
        let column = self.column(line, q);
        if self.is_dash(line, q) {
            return self.parse_sequence(line, q, column);
        }
        if self.key_at(line, q)?.is_some() {
            return self.parse_mapping(line, q, column);
        }
        self.parse_inline(line, q, parent_indent)
    }

    fn parse_mapping(&mut self, mut line: usize, mut p: usize, column: i64) -> Result<ValueId, YamlError> {
        let value = self.add_value(TreeValue::new(ValueKind::Mapping, ValueStyle::Block, Span::new(p, p)));
        let mut names: HashSet<String> = HashSet::new();
        loop {
            let Some(key) = self.key_at(line, p)? else {
                return fail(p, "A mapping key was expected here.");
            };
            self.leaves.push(key.span);
            let q = self.skip_spaces(line, key.colon + 1);
            let member = if self.at_line_end(line, q) {
                self.line = line + 1;
                match self.parse_block_node(column, true)? {
                    Some(member) => member,
                    None => self.empty(key.colon + 1),
                }
            } else {
                self.line = line;
                self.parse_inline(line, q, column)?
            };
            let own = key.span.with_end(self.end_of(member, key.colon + 1));
            let entry = self.add_entry(own, column as usize, Some(key.name.clone()), Some(key.span), member);
            if key.name == "<<" && !self.values[member].merged.is_empty() {
                let merged = self.values[member].merged.clone();
                self.values[value].merged.extend(merged);
            }
            if names.insert(key.name.clone()) {
                self.values[value].entries.push(entry);
            } else {
                self.duplicates.push((key.name, key.span));
            }
            let next = self.next_significant(self.line);
            if next >= self.end {
                break;
            }
            let np = self.first_non_space(next);
            let next_column = self.column(next, np);
            if next_column > column {
                return fail(np, "This line is indented deeper than the mapping it is in allows.");
            }
            if next_column < column || self.key_at(next, np)?.is_none() {
                break;
            }
            line = next;
            p = np;
            self.line = next;
        }
        let entries = &self.values[value].entries;
        let span = match (entries.first(), entries.last()) {
            (Some(first), Some(last)) => Span::new(self.entries[*first].own.start, self.entries[*last].own.end),
            _ => Span::new(p, p),
        };
        self.values[value].span = span;
        Ok(value)
    }

    /// A mapping key at `p`: plain, or quoted on one line, followed by `:` and a space or the line's end.
    fn key_at(&self, line: usize, p: usize) -> Result<Option<Key>, YamlError> {
        let end = self.content_end(line);
        let b = self.bytes[p];
        if b == b'"' || b == b'\'' {
            let mut k = p + 1;
            while k < end {
                if self.bytes[k] == b'\\' && b == b'"' {
                    k += 2;
                    continue;
                }
                if self.bytes[k] == b {
                    if b == b'\'' && k + 1 < end && self.bytes[k + 1] == b'\'' {
                        k += 2;
                        continue;
                    }
                    break;
                }
                k += 1;
            }
            if k >= end {
                return Ok(None);
            }
            let colon = self.skip_spaces(line, k + 1);
            if colon >= end || self.bytes[colon] != b':' || !(colon + 1 == end || space(self.bytes[colon + 1])) {
                return Ok(None);
            }
            let inner = self.text.text(p + 1, k);
            let name = if b == b'"' {
                scalars::decode_double(inner)
            } else {
                scalars::decode_single(inner)
            };
            return Ok(Some(Key {
                name,
                span: Span::new(p, k + 1),
                colon,
            }));
        }
        if matches!(
            b,
            b'[' | b'{' | b'#' | b'&' | b'*' | b'!' | b'|' | b'>' | b'%' | b'@' | b'`'
        ) {
            return Ok(None);
        }
        if b == b'?' && (p + 1 == end || space(self.bytes[p + 1])) {
            return fail(p, "Explicit keys ('? ') are not read by this host.");
        }
        if self.is_dash(line, p) {
            return Ok(None);
        }
        for k in p..end {
            let c = self.bytes[k];
            if c == b'#' && k > p && space(self.bytes[k - 1]) {
                return Ok(None);
            }
            if c != b':' || !(k + 1 == end || space(self.bytes[k + 1])) {
                continue;
            }
            let mut key_end = k;
            while key_end > p && space(self.bytes[key_end - 1]) {
                key_end -= 1;
            }
            if key_end == p {
                return Ok(None);
            }
            return Ok(Some(Key {
                name: self.text.text(p, key_end).to_owned(),
                span: Span::new(p, key_end),
                colon: k,
            }));
        }
        Ok(None)
    }

    // ---- values ----

    fn parse_inline(&mut self, line: usize, mut q: usize, parent_indent: i64) -> Result<ValueId, YamlError> {
        let mut anchor: Option<String> = None;
        while matches!(self.bytes[q], b'&' | b'!') {
            let start = q;
            while q < self.content_end(line) && !space(self.bytes[q]) {
                q += 1;
            }
            if self.bytes[start] == b'&' {
                anchor = Some(self.text.text(start + 1, q).to_owned());
            }
            q = self.skip_spaces(line, q);
            if self.at_line_end(line, q) {
                self.line = line + 1;
                let block = match self.parse_block_node(parent_indent, true)? {
                    Some(block) => block,
                    None => self.empty(q),
                };
                if let Some(anchor) = anchor {
                    self.anchors.insert(anchor, block);
                }
                return Ok(block);
            }
        }
        let value = match self.bytes[q] {
            b'*' => self.parse_alias(line, q)?,
            b'"' | b'\'' => self.parse_quoted(line, q)?,
            b'|' | b'>' => self.parse_block_scalar(line, q, parent_indent)?,
            b'[' | b'{' => self.parse_flow(line, q)?,
            _ => self.parse_plain(line, q, parent_indent)?,
        };
        if let Some(anchor) = anchor {
            self.anchors.insert(anchor, value);
        }
        Ok(value)
    }

    fn expect_line_end(&self, line: usize, p: usize) -> Result<(), YamlError> {
        if !self.at_line_end(line, p) {
            return fail(p, "Unexpected content after a value.");
        }
        Ok(())
    }

    fn parse_alias(&mut self, line: usize, q: usize) -> Result<ValueId, YamlError> {
        let mut p = q + 1;
        while p < self.content_end(line) && !matches!(self.bytes[p], b' ' | b'\t' | b',' | b']' | b'}') {
            p += 1;
        }
        let name = self.text.text(q + 1, p).to_owned();
        let Some(&target) = self.anchors.get(&name) else {
            return fail(q, format!("The alias *{name} names no anchor."));
        };
        self.expect_line_end(line, p)?;
        self.line = line + 1;
        let span = Span::new(q, p);
        self.leaves.push(span);
        let t = &self.values[target];
        let scalar = t.kind == ValueKind::Scalar;
        let mut value = TreeValue::scalar(ValueStyle::Plain, span, t.text.clone(), t.typed.clone());
        value.flow = if scalar {
            t.flow.clone()
        } else {
            Some(cel_of(&self.values, &self.entries, target, Reading::Typed))
        };
        value.flow_text = if scalar { t.flow_text.clone() } else { None };
        value.flow_style = if scalar { t.flow_style.clone() } else { None };
        value.via_alias = true;
        if t.kind == ValueKind::Mapping {
            value.merged.push(target);
        }
        Ok(self.add_value(value))
    }

    fn parse_quoted(&mut self, line: usize, q: usize) -> Result<ValueId, YamlError> {
        let quote = self.bytes[q];
        let mut p = q + 1;
        let mut current = line;
        loop {
            if p >= self.content_end(current) {
                current += 1;
                if current >= self.end {
                    return fail(q, "A quoted scalar is never closed.");
                }
                p = self.line_start(current);
                continue;
            }
            let b = self.bytes[p];
            if quote == b'"' && b == b'\\' {
                p += 2;
                continue;
            }
            if b == quote {
                if quote == b'\'' && p + 1 < self.content_end(current) && self.bytes[p + 1] == b'\'' {
                    p += 2;
                    continue;
                }
                break;
            }
            p += 1;
        }
        let span = Span::new(q, p + 1);
        self.expect_line_end(current, p + 1)?;
        self.line = current + 1;
        self.leaves.push(span);
        let inner = self.text.text(q + 1, p);
        let decoded = if quote == b'"' {
            scalars::decode_double(inner)
        } else {
            scalars::decode_single(inner)
        };
        let style = if quote == b'"' {
            ValueStyle::Double
        } else {
            ValueStyle::Single
        };
        Ok(self.add_value(TreeValue::scalar(style, span, decoded.clone(), Value::from(decoded))))
    }

    fn parse_plain(&mut self, line: usize, q: usize, parent_indent: i64) -> Result<ValueId, YamlError> {
        let end = self.plain_end(line, q);
        let mut plain = self.text.text(q, end).to_owned();
        let mut last = end;
        let mut current = line;
        let mut blank = 0;
        for next in line + 1..self.end {
            let p = self.first_non_space(next);
            if p >= self.content_end(next) {
                blank += 1;
                continue;
            }
            if self.bytes[p] == b'#' || self.column(next, p) <= parent_indent {
                break;
            }
            if self.key_at(next, p)?.is_some() || (self.is_dash(next, p) && self.column(next, p) <= parent_indent + 1) {
                break;
            }
            let part_end = self.plain_end(next, p);
            if blank > 0 {
                plain.push_str(&"\n".repeat(blank));
            } else {
                plain.push(' ');
            }
            plain.push_str(self.text.text(p, part_end));
            blank = 0;
            last = part_end;
            current = next;
        }
        self.line = current + 1;
        let span = Span::new(q, last);
        self.leaves.push(span);
        let typed = scalars::typed(&plain);
        Ok(self.add_value(TreeValue::scalar(ValueStyle::Plain, span, plain, typed)))
    }

    fn plain_end(&self, line: usize, q: usize) -> usize {
        let end = self.content_end(line);
        let mut p = q;
        while p < end {
            if self.bytes[p] == b'#' && p > q && space(self.bytes[p - 1]) {
                break;
            }
            p += 1;
        }
        while p > q && space(self.bytes[p - 1]) {
            p -= 1;
        }
        p
    }

    fn parse_block_scalar(&mut self, line: usize, q: usize, parent_indent: i64) -> Result<ValueId, YamlError> {
        let literal = self.bytes[q] == b'|';
        let mut p = q + 1;
        let mut chomp = 'c';
        let mut explicit_indent = 0i64;
        while p < self.content_end(line) && !space(self.bytes[p]) {
            let c = self.bytes[p] as char;
            if c == '+' || c == '-' {
                chomp = c;
            } else if c.is_ascii_digit() {
                explicit_indent = i64::from(self.bytes[p] - b'0');
            } else {
                return fail(p, "A block scalar's header holds an unknown indicator.");
            }
            p += 1;
        }
        self.expect_line_end(line, p)?;
        let header_end = p;
        let mut indent = if explicit_indent > 0 {
            parent_indent.max(0) + explicit_indent
        } else {
            -1
        };
        let mut content_lines = Vec::new();
        let mut trailing_blank = 0;
        let mut last_content: Option<usize> = None;
        for next in line + 1..self.end {
            let first = self.first_non_space(next);
            let is_blank = first >= self.content_end(next);
            let column = self.column(next, first);
            if is_blank {
                content_lines.push(next);
                trailing_blank += 1;
                continue;
            }
            if indent < 0 {
                if column <= parent_indent {
                    break;
                }
                indent = column;
            }
            if column < indent {
                break;
            }
            content_lines.push(next);
            trailing_blank = 0;
            last_content = Some(next);
        }
        let style = if literal {
            ValueStyle::Literal
        } else {
            ValueStyle::Folded
        };
        let Some(last_content) = last_content else {
            self.line = line + 1;
            let empty = Span::new(q, header_end);
            self.leaves.push(empty);
            return Ok(self.add_value(TreeValue::scalar(style, empty, String::new(), Value::from(""))));
        };
        let texts: Vec<String> = content_lines
            .iter()
            .filter(|l| **l <= last_content)
            .map(|&l| {
                let start = (self.line_start(l) + indent as usize).min(self.content_end(l));
                self.text.text(start, self.content_end(l)).to_owned()
            })
            .collect();
        let mut body = if literal { texts.join("\n") } else { fold_block(&texts) };
        match chomp {
            '-' => {}
            '+' => {
                body.push('\n');
                body.push_str(&"\n".repeat(trailing_blank));
            }
            _ => body.push('\n'),
        }
        self.line = last_content + 1;
        let span = Span::new(q, self.content_end(last_content));
        self.leaves.push(span);
        Ok(self.add_value(TreeValue::scalar(style, span, body.clone(), Value::from(body))))
    }

    fn parse_flow(&mut self, line: usize, q: usize) -> Result<ValueId, YamlError> {
        let mut depth = 0;
        let mut current = line;
        let mut p = q;
        loop {
            if p >= self.content_end(current) {
                current += 1;
                if current >= self.end {
                    return fail(q, "A flow collection is never closed.");
                }
                p = self.line_start(current);
                continue;
            }
            let b = self.bytes[p];
            if b == b'"' || b == b'\'' {
                let quote = b;
                p += 1;
                loop {
                    if p >= self.content_end(current) {
                        current += 1;
                        if current >= self.end {
                            return fail(q, "A quoted scalar in a flow collection is never closed.");
                        }
                        p = self.line_start(current);
                        continue;
                    }
                    if quote == b'"' && self.bytes[p] == b'\\' {
                        p += 2;
                        continue;
                    }
                    if self.bytes[p] == quote {
                        if quote == b'\'' && p + 1 < self.content_end(current) && self.bytes[p + 1] == b'\'' {
                            p += 2;
                            continue;
                        }
                        break;
                    }
                    p += 1;
                }
                p += 1;
                continue;
            }
            if b == b'#' && p > q && space(self.bytes[p - 1]) {
                p = self.content_end(current);
                continue;
            }
            if b == b'[' || b == b'{' {
                depth += 1;
            }
            if b == b']' || b == b'}' {
                depth -= 1;
                if depth == 0 {
                    p += 1;
                    break;
                }
            }
            p += 1;
        }
        self.expect_line_end(current, p)?;
        self.line = current + 1;
        let span = Span::new(q, p);
        self.leaves.push(span);
        let raw = self.text.text_of(span).to_owned();
        let flow = flow::read(&raw);
        let style = if self.bytes[q] == b'[' {
            ValueStyle::FlowSequence
        } else {
            ValueStyle::FlowMapping
        };
        let mut value = TreeValue::scalar(style, span, raw, Value::Null);
        value.flow = Some(flow.typed);
        value.flow_text = Some(flow.text);
        value.flow_style = Some(flow.style);
        if style == ValueStyle::FlowSequence {
            value.entries = self.flow_items(q, p).0;
        }
        Ok(self.add_value(value))
    }

    // ---- the entries of a flow sequence (FBL 0.10 section 4.3) ----

    /// The offset of the next thing in a flow collection that is not whitespace or a comment.
    fn flow_skip(&self, mut p: usize, end: usize) -> usize {
        while p < end {
            match self.bytes[p] {
                b' ' | b'\t' | b'\r' | b'\n' => p += 1,
                b'#' if space(self.bytes[p - 1]) || self.bytes[p - 1] == b'\n' => {
                    while p < end && self.bytes[p] != b'\n' {
                        p += 1;
                    }
                }
                _ => break,
            }
        }
        p
    }

    fn flow_column(&self, offset: usize) -> usize {
        offset - self.line_start(self.text.line_index_at(offset))
    }

    /// The `,` at `p` that follows the entry just read, when there is one, as that entry's separator (FBL section
    /// 4.4), and the offset after it.
    fn flow_separator(&mut self, entry: EntryId, p: usize, end: usize) -> usize {
        let p = self.flow_skip(p, end);
        if p >= end || self.bytes[p] != b',' {
            return p;
        }
        if let EntryKind::Tree(tree) = &mut self.entries[entry].kind {
            tree.separator = Some(p);
        }
        p + 1
    }

    /// The items of the flow sequence opening at `open`, each an entry with the span of a json item, and the offset
    /// after its `]`. `end` bounds it: `parse_flow` found the brackets balanced, not that they match.
    fn flow_items(&mut self, open: usize, end: usize) -> (Vec<EntryId>, usize) {
        let mut items = Vec::new();
        let mut p = open + 1;
        loop {
            p = self.flow_skip(p, end);
            if p >= end {
                return (items, end);
            }
            match self.bytes[p] {
                b']' | b'}' => return (items, p + 1),
                b',' => {
                    p += 1;
                    continue;
                }
                _ => {}
            }
            let (value, next) = self.flow_value(p, end);
            let span = self.values[value].span;
            let item = self.add_entry(span, self.flow_column(span.start), None, None, value);
            items.push(item);
            // A value that takes nothing (a stray `:`) is stepped over.
            p = self.flow_separator(item, next.max(p + 1), end);
        }
    }

    /// The members of the flow mapping opening at `open`, as `flow_items` reads items.
    fn flow_members(&mut self, open: usize, end: usize) -> (Vec<EntryId>, usize) {
        let mut members = Vec::new();
        let mut p = open + 1;
        loop {
            p = self.flow_skip(p, end);
            if p >= end {
                return (members, end);
            }
            match self.bytes[p] {
                b']' | b'}' => return (members, p + 1),
                b',' => {
                    p += 1;
                    continue;
                }
                _ => {}
            }
            let (key, next) = self.flow_scalar(p, end, true);
            let key_span = key.span;
            let mut after = self.flow_skip(next.max(p + 1), end);
            let mut value = None;
            if after < end && self.bytes[after] == b':' {
                after = self.flow_skip(after + 1, end);
                if after < end && !matches!(self.bytes[after], b',' | b']' | b'}') {
                    let (read, next) = self.flow_value(after, end);
                    after = next.max(after + 1);
                    value = Some(read);
                }
            }
            let value = value.unwrap_or_else(|| self.empty(key_span.end));
            let own = key_span.with_end(self.end_of(value, key_span.end));
            let member = self.add_entry(own, self.flow_column(own.start), Some(key.text), Some(key_span), value);
            members.push(member);
            p = self.flow_separator(member, after, end);
        }
    }

    /// A value inside a flow sequence, and the offset after it.
    fn flow_value(&mut self, p: usize, end: usize) -> (ValueId, usize) {
        let (value, next) = match self.bytes[p] {
            b'[' => {
                let (entries, next) = self.flow_items(p, end);
                let span = Span::new(p, next);
                (
                    TreeValue {
                        entries,
                        ..TreeValue::new(ValueKind::Sequence, ValueStyle::FlowSequence, span)
                    },
                    next,
                )
            }
            b'{' => {
                let (entries, next) = self.flow_members(p, end);
                let span = Span::new(p, next);
                (
                    TreeValue {
                        entries,
                        ..TreeValue::new(ValueKind::Mapping, ValueStyle::FlowMapping, span)
                    },
                    next,
                )
            }
            _ => self.flow_scalar(p, end, false),
        };
        (self.add_value(value), next)
    }

    /// A scalar inside a flow collection, read as `flow` reads one, and the offset after it.
    fn flow_scalar(&self, p: usize, end: usize, key: bool) -> (TreeValue, usize) {
        let quote = self.bytes[p];
        if quote == b'"' || quote == b'\'' {
            let mut close = p + 1;
            while close < end {
                if quote == b'"' && self.bytes[close] == b'\\' {
                    close += 2;
                    continue;
                }
                if self.bytes[close] == quote {
                    if quote == b'\'' && close + 1 < end && self.bytes[close + 1] == b'\'' {
                        close += 2;
                        continue;
                    }
                    break;
                }
                close += 1;
            }
            let close = close.min(end);
            let inner = self.text.text(p + 1, close);
            let (decoded, style) = if quote == b'"' {
                (scalars::decode_double(inner), ValueStyle::Double)
            } else {
                (scalars::decode_single(inner), ValueStyle::Single)
            };
            let span = Span::new(p, (close + 1).min(end));
            let typed = Value::from(decoded.as_str());
            return (TreeValue::scalar(style, span, decoded, typed), span.end);
        }
        let mut q = p;
        while q < end {
            let b = self.bytes[q];
            if matches!(b, b',' | b']' | b'}') {
                break;
            }
            if b == b':' && (key || q + 1 >= end || matches!(self.bytes[q + 1], b' ' | b',' | b']' | b'}')) {
                break;
            }
            q += 1;
        }
        let raw = self.text.text(p, q);
        let trimmed = raw.trim();
        let plain = scalars::fold(trimmed);
        let span = Span::new(p, p + trimmed.len());
        let typed = scalars::typed(&plain);
        (TreeValue::scalar(ValueStyle::Plain, span, plain, typed), q)
    }
}

fn fold_block(lines: &[String]) -> String {
    let mut out = String::new();
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            let previous = &lines[i - 1];
            let more_indented = line.starts_with(' ') || previous.starts_with(' ');
            out.push(if line.is_empty() || previous.is_empty() || more_indented {
                '\n'
            } else {
                ' '
            });
        }
        out.push_str(line);
    }
    out
}
