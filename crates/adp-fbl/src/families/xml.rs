//! The xml family (FBL section 4.5; the standalone host's `Files/Xml/XmlFamily`): a lossless reading of an XML 1.0
//! document over its bytes. The prolog, comments, processing instructions and a document type declaration are
//! unbound content; carriage returns are kept, never normalised; entity declarations are not processed.

use std::collections::HashMap;
use std::sync::LazyLock;

use adp_cel::{Map, Value};
use regex::Regex;

use crate::documents::types::{AttributeBinding, CreateChild, HeaderSettings, Slot};
use crate::families::{Entry, EntryId, EntryKind, Family, FamilyKind};
use crate::model::{FindingSeverity, codes};
use crate::planning::new_text;
use crate::planning::plan::{Plan, Planning, refuse};
use crate::rules::{Candidate, InsertRequest, Node, ReadElement, SlotChange, SlotRead, capture, selector};
use crate::splice::Operation;
use crate::text::Span;

/// An attribute as written (FBL section 4.5): its own span from name to closing quote, and its value span between
/// the quotes.
#[derive(Debug, Clone)]
pub(crate) struct XmlAttribute {
    pub name: String,
    pub own: Span,
    pub value: Span,
    pub text: String,
}

/// Text runs and child elements of an element, in document order.
#[derive(Debug, Clone)]
pub(crate) enum XmlContent {
    /// Character data, its references decoded.
    Text(String),
    Element(EntryId),
}

/// An element (FBL section 4.5): an entry whose own span runs from its start tag's `<` to its end tag's `>`.
#[derive(Debug, Clone, Default)]
pub(crate) struct XmlElement {
    pub is_root: bool,
    pub start_tag: Span,
    /// The end of the element's name in its start tag, where a first attribute is added.
    pub name_end: usize,
    /// The `>` that ends the start tag, or the `/>` of a self-closed tag.
    pub close: Span,
    pub self_closed: bool,
    pub end_tag: Option<Span>,
    pub attributes: Vec<XmlAttribute>,
    pub content: Vec<XmlContent>,
    /// A reference to an entity other than the five predefined ones: the element is an unreadable entry.
    pub unreadable: Option<String>,
}

impl XmlElement {
    pub fn content_start(&self) -> usize {
        self.start_tag.end
    }

    pub fn content_end(&self) -> usize {
        self.end_tag.map(|e| e.start).unwrap_or(self.start_tag.end)
    }

    pub fn elements(&self) -> impl Iterator<Item = EntryId> + '_ {
        self.content.iter().filter_map(|c| match c {
            XmlContent::Element(e) => Some(*e),
            XmlContent::Text(_) => None,
        })
    }

    pub fn attribute(&self, name: &str) -> Option<(usize, &XmlAttribute)> {
        self.attributes.iter().enumerate().find(|(_, a)| a.name == name)
    }
}

/// What an xml reading keeps besides its entries.
#[derive(Debug, Default)]
pub(crate) struct XmlData {
    pub root: Option<EntryId>,
    pub document: Option<EntryId>,
    pub comment_lines: HashMap<usize, Span>,
}

static WHITESPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").expect("the whitespace pattern compiles"));

struct XmlError {
    offset: usize,
    message: String,
}

fn fail<T>(offset: usize, message: impl Into<String>) -> Result<T, XmlError> {
    Err(XmlError {
        offset,
        message: message.into(),
    })
}

fn is_name_byte(b: u8, first: bool) -> bool {
    b >= 0x80
        || b.is_ascii_alphabetic()
        || b == b'_'
        || b == b':'
        || (!first && (b.is_ascii_digit() || b == b'-' || b == b'.'))
}

impl Family {
    fn xml_data(&self) -> &XmlData {
        match &self.kind {
            FamilyKind::Xml(x) => x,
            _ => panic!("the family is not xml"),
        }
    }

    fn xml_data_mut(&mut self) -> &mut XmlData {
        match &mut self.kind {
            FamilyKind::Xml(x) => x,
            _ => panic!("the family is not xml"),
        }
    }

    fn element(&self, id: EntryId) -> &XmlElement {
        self.entries[id].xml()
    }

    fn element_mut(&mut self, id: EntryId) -> &mut XmlElement {
        match &mut self.entries[id].kind {
            EntryKind::Xml(x) => x,
            _ => panic!("the entry is not an xml element"),
        }
    }

    // ---- reading ----

    pub(super) fn xml_parse(&mut self) {
        if let Err(error) = self.xml_parse_document() {
            self.unreadable = Some((
                error.offset,
                format!("The body is not well-formed XML: {}", error.message),
            ));
        }
    }

    fn xml_parse_document(&mut self) -> Result<(), XmlError> {
        let length = self.text.len();
        self.entries.push(Entry {
            own: Span::new(0, length),
            line_span: None,
            parent: None,
            children: Vec::new(),
            name: None,
            indent: 0,
            kind: EntryKind::Xml(Box::new(XmlElement {
                is_root: true,
                ..XmlElement::default()
            })),
        });
        let root = self.entries.len() - 1;
        self.xml_data_mut().root = Some(root);
        let mut stack: Vec<EntryId> = Vec::new();
        let mut position = self.text.bom_length();
        while position < length {
            if self.text.bytes()[position] != b'<' {
                position = self.read_text(position, stack.last().copied())?;
                continue;
            }
            if self.starts_with(position, "<?") {
                position = self.skip(position, "?>", "processing instruction")?;
            } else if self.starts_with(position, "<!--") {
                let start = position;
                position = self.skip(position, "-->", "comment")?;
                self.remember_comment(Span::new(start, position));
            } else if self.starts_with(position, "<![CDATA[") {
                let Some(&top) = stack.last() else {
                    return fail(position, "A CDATA section must be inside the document element.");
                };
                let start = position;
                position = self.skip(position, "]]>", "CDATA section")?;
                let content = self.text.text(start + 9, position - 3).to_owned();
                self.element_mut(top).content.push(XmlContent::Text(content));
            } else if self.starts_with(position, "<!") {
                if self.xml_data().document.is_some() {
                    return fail(
                        position,
                        "A document type declaration must come before the document element.",
                    );
                }
                position = self.skip_declaration(position)?;
            } else if self.starts_with(position, "</") {
                position = self.read_end_tag(position, &mut stack)?;
            } else {
                position = self.read_start_tag(position, &mut stack)?;
            }
        }
        if let Some(&top) = stack.last() {
            return fail(
                length,
                format!(
                    "The element '{}' is not closed.",
                    self.entries[top].name.as_deref().unwrap_or_default()
                ),
            );
        }
        if self.xml_data().document.is_none() {
            return fail(self.text.bom_length(), "The body has no document element.");
        }
        for i in 0..self.order.len() {
            let id = self.order[i];
            self.entries[id].line_span = self.xml_line_span_of(id);
        }
        Ok(())
    }

    fn starts_with(&self, position: usize, literal: &str) -> bool {
        self.text.bytes()[position..].starts_with(literal.as_bytes())
    }

    fn find(&self, position: usize, literal: &str) -> Option<usize> {
        (position..self.text.len()).find(|&i| self.starts_with(i, literal))
    }

    fn skip(&mut self, position: usize, terminator: &str, what: &str) -> Result<usize, XmlError> {
        let Some(end) = self.find(position + 2, terminator) else {
            return fail(position, format!("The {what} is not closed."));
        };
        let end = end + terminator.len();
        self.leaves.push(Span::new(position, end));
        Ok(end)
    }

    fn skip_declaration(&mut self, position: usize) -> Result<usize, XmlError> {
        let mut depth = 0i32;
        let mut quote = 0u8;
        for i in position + 2..self.text.len() {
            let b = self.text.bytes()[i];
            if quote != 0 {
                if b == quote {
                    quote = 0;
                }
                continue;
            }
            match b {
                b'"' | b'\'' => quote = b,
                b'[' => depth += 1,
                b']' => depth -= 1,
                b'>' if depth == 0 => {
                    self.leaves.push(Span::new(position, i + 1));
                    return Ok(i + 1);
                }
                _ => {}
            }
        }
        fail(position, "The declaration is not closed.")
    }

    fn remember_comment(&mut self, comment: Span) {
        let line = self.text.line_index_at(comment.start);
        if self.text.line_index_at(comment.end - 1) == line
            && self.only_whitespace_before(comment.start)
            && self.only_trivia_after(comment.end, None)
        {
            self.xml_data_mut().comment_lines.insert(line, comment);
        }
    }

    fn read_text(&mut self, position: usize, parent: Option<EntryId>) -> Result<usize, XmlError> {
        let mut end = position;
        while end < self.text.len() && self.text.bytes()[end] != b'<' {
            end += 1;
        }
        let span = Span::new(position, end);
        if self.is_whitespace(span) {
            if let Some(parent) = parent {
                let text = self.text.text_of(span).to_owned();
                self.element_mut(parent).content.push(XmlContent::Text(text));
            }
            return Ok(end);
        }
        let Some(parent) = parent else {
            return fail(position, "Text must be inside the document element.");
        };
        self.leaves.push(span);
        let decoded = self.decode(span, Some(parent))?;
        self.element_mut(parent).content.push(XmlContent::Text(decoded));
        Ok(end)
    }

    /// Decodes the five predefined entities and character references; any other reference makes `owner` an
    /// unreadable entry.
    fn decode(&mut self, span: Span, owner: Option<EntryId>) -> Result<String, XmlError> {
        let raw = self.text.text_of(span).to_owned();
        if !raw.contains('&') {
            return Ok(raw);
        }
        let mut out = String::with_capacity(raw.len());
        let mut i = 0;
        while i < raw.len() {
            let c = raw[i..].chars().next().expect("a character");
            if c != '&' {
                out.push(c);
                i += c.len_utf8();
                continue;
            }
            let Some(semicolon) = raw[i + 1..].find(';').map(|p| p + i + 1) else {
                return fail(span.start, "An '&' must start a reference ending in ';'.");
            };
            let name = &raw[i + 1..semicolon];
            match name {
                "amp" => out.push('&'),
                "lt" => out.push('<'),
                "gt" => out.push('>'),
                "quot" => out.push('"'),
                "apos" => out.push('\''),
                _ if name.starts_with('#') => {
                    let (digits, radix) = match name.strip_prefix("#x") {
                        Some(hex) => (hex, 16),
                        None => (&name[1..], 10),
                    };
                    let code = u32::from_str_radix(digits, radix).ok().filter(|_| {
                        !digits.is_empty()
                            && digits
                                .bytes()
                                .all(|b| b.is_ascii_hexdigit() && (radix == 16 || b.is_ascii_digit()))
                    });
                    match code
                        .filter(|c| (1..=0x10FFFF).contains(c) && !(0xD800..=0xDFFF).contains(c))
                        .and_then(char::from_u32)
                    {
                        Some(ch) => out.push(ch),
                        None => return fail(span.start, format!("'&{name};' is not a valid character reference.")),
                    }
                }
                _ => {
                    if let Some(owner) = owner
                        && self.element(owner).unreadable.is_none()
                    {
                        self.element_mut(owner).unreadable = Some(format!(
                            "It refers to the entity '&{name};', which is not one of XML's five predefined entities."
                        ));
                    }
                    out.push('&');
                    out.push_str(name);
                    out.push(';');
                }
            }
            i = semicolon + 1;
        }
        Ok(out)
    }

    fn read_name(&self, position: usize) -> Result<usize, XmlError> {
        let mut end = position;
        while end < self.text.len() && is_name_byte(self.text.bytes()[end], end == position) {
            end += 1;
        }
        if end == position {
            return fail(position, "A name was expected here.");
        }
        Ok(end)
    }

    fn skip_space(&self, mut position: usize) -> usize {
        while position < self.text.len() && matches!(self.text.bytes()[position], b' ' | b'\t' | b'\r' | b'\n') {
            position += 1;
        }
        position
    }

    fn read_start_tag(&mut self, start: usize, stack: &mut Vec<EntryId>) -> Result<usize, XmlError> {
        let parent = stack.last().copied();
        if parent.is_none() && self.xml_data().document.is_some() {
            return fail(start, "The body has a second document element.");
        }
        let name_end = self.read_name(start + 1)?;
        let name = self.text.text(start + 1, name_end).to_owned();
        let line_start = self.text.lines()[self.text.line_index_at(start)].start;
        let root = self.xml_data().root.expect("the root");
        self.entries.push(Entry {
            own: Span::new(start, name_end),
            line_span: None,
            parent: Some(parent.unwrap_or(root)),
            children: Vec::new(),
            name: Some(name.clone()),
            indent: start - line_start,
            kind: EntryKind::Xml(Box::new(XmlElement {
                name_end,
                ..XmlElement::default()
            })),
        });
        let element = self.entries.len() - 1;
        let mut position = name_end;
        loop {
            let spaced = self.skip_space(position);
            if spaced >= self.text.len() {
                return fail(start, format!("The start tag of '{name}' is not closed."));
            }
            if self.starts_with(spaced, "/>") {
                let e = self.element_mut(element);
                e.self_closed = true;
                e.close = Span::new(spaced, spaced + 2);
                position = spaced + 2;
                break;
            }
            if self.text.bytes()[spaced] == b'>' {
                self.element_mut(element).close = Span::new(spaced, spaced + 1);
                position = spaced + 1;
                break;
            }
            if spaced == position {
                return fail(spaced, "Attributes must be separated by whitespace.");
            }
            let attribute_end = self.read_name(spaced)?;
            let attribute = self.text.text(spaced, attribute_end).to_owned();
            let equals = self.skip_space(attribute_end);
            if equals >= self.text.len() || self.text.bytes()[equals] != b'=' {
                return fail(equals, format!("The attribute '{attribute}' has no value."));
            }
            let open = self.skip_space(equals + 1);
            if open >= self.text.len() || !matches!(self.text.bytes()[open], b'"' | b'\'') {
                return fail(open, format!("The value of '{attribute}' must be quoted."));
            }
            let quote = self.text.bytes()[open];
            let mut close = open + 1;
            while close < self.text.len() && self.text.bytes()[close] != quote {
                if self.text.bytes()[close] == b'<' {
                    return fail(close, "An attribute value cannot contain '<'.");
                }
                close += 1;
            }
            if close >= self.text.len() {
                return fail(open, format!("The value of '{attribute}' is not closed."));
            }
            if self.element(element).attribute(&attribute).is_some() {
                return fail(spaced, format!("The attribute '{attribute}' is written twice."));
            }
            let value = Span::new(open + 1, close);
            let text = self.decode(value, Some(element))?;
            self.element_mut(element).attributes.push(XmlAttribute {
                name: attribute,
                own: Span::new(spaced, close + 1),
                value,
                text,
            });
            position = close + 1;
        }
        let start_tag = Span::new(start, position);
        self.element_mut(element).start_tag = start_tag;
        self.entries[element].own = start_tag;
        self.leaves.push(start_tag);
        self.order.push(element);
        match parent {
            None => {
                self.xml_data_mut().document = Some(element);
                self.entries[root].children.push(element);
            }
            Some(parent) => {
                self.entries[parent].children.push(element);
                self.element_mut(parent).content.push(XmlContent::Element(element));
            }
        }
        if !self.element(element).self_closed {
            stack.push(element);
        }
        Ok(position)
    }

    fn read_end_tag(&mut self, start: usize, stack: &mut Vec<EntryId>) -> Result<usize, XmlError> {
        let name_end = self.read_name(start + 2)?;
        let name = self.text.text(start + 2, name_end).to_owned();
        let close = self.skip_space(name_end);
        if close >= self.text.len() || self.text.bytes()[close] != b'>' {
            return fail(start, format!("The end tag of '{name}' is not closed."));
        }
        match stack.last() {
            None => return fail(start, format!("The end tag '{name}' closes nothing.")),
            Some(&top) if self.entries[top].name.as_deref() != Some(name.as_str()) => {
                return fail(
                    start,
                    format!(
                        "The end tag '{name}' does not close '{}'.",
                        self.entries[top].name.as_deref().unwrap_or_default()
                    ),
                );
            }
            _ => {}
        }
        let element = stack.pop().expect("an open element");
        let end_tag = Span::new(start, close + 1);
        self.element_mut(element).end_tag = Some(end_tag);
        let start_tag = self.element(element).start_tag;
        self.entries[element].own = start_tag.with_end(close + 1);
        self.leaves.push(end_tag);
        Ok(close + 1)
    }

    /// The line span (FBL section 4.1.1): whole lines, extended upwards over comment lines at the same indentation.
    fn xml_line_span_of(&self, entry: EntryId) -> Option<Span> {
        let e = &self.entries[entry];
        if !self.only_whitespace_before(e.own.start) || !self.only_trivia_after(e.own.end, None) {
            return None;
        }
        let lines = self.text.lines();
        let mut first = self.text.line_index_at(e.own.start);
        let last = self.text.line_index_at(e.own.end - 1);
        while first > 0
            && let Some(comment) = self.xml_data().comment_lines.get(&(first - 1))
            && comment.start - lines[first - 1].start == e.indent
        {
            first -= 1;
        }
        Some(Span::new(lines[first].start, lines[last].end))
    }

    pub(super) fn xml_step(&self) -> usize {
        for &id in &self.order {
            let entry = &self.entries[id];
            if let Some(parent) = entry.parent {
                let p = &self.entries[parent];
                if !p.xml().is_root && p.line_span.is_some() && entry.line_span.is_some() && entry.indent >= p.indent {
                    return entry.indent - p.indent;
                }
            }
        }
        self.default_step()
    }

    fn attribute_equals(&self, entry: EntryId, attribute: &str, value: &str) -> bool {
        self.element(entry)
            .attribute(attribute)
            .is_some_and(|(_, a)| a.text == value)
    }

    fn child(&self, element: EntryId, selector_text: &str) -> Option<EntryId> {
        let equals = |e: EntryId, a: &str, v: &str| self.attribute_equals(e, a, v);
        selector::matches(&self.entries, element, selector_text, Some(&equals))
            .into_iter()
            .next()
            .map(|(e, _)| e)
    }

    pub(super) fn xml_candidates(&self, rule: usize) -> Vec<Candidate> {
        let (Some(at), Some(root)) = (self.rule(rule).at.as_deref(), self.xml_data().root) else {
            return Vec::new();
        };
        let equals = |e: EntryId, a: &str, v: &str| self.attribute_equals(e, a, v);
        let mut candidates = Vec::new();
        for (entry, captures) in selector::matches(&self.entries, root, at, Some(&equals)) {
            let element = self.element(entry);
            if let Some(reason) = &element.unreadable {
                let line = self.locate(self.entries[entry].own).line;
                let reported =
                    self.findings.borrow().iter().any(|f| {
                        f.code == codes::UNREADABLE_ENTRY && f.location.as_ref().is_some_and(|l| l.line == line)
                    });
                if !reported {
                    self.report(
                        codes::UNREADABLE_ENTRY,
                        FindingSeverity::Warning,
                        format!("This entry cannot be read: {reason}"),
                        Some(element.start_tag),
                    );
                }
                continue;
            }
            candidates.push(Candidate {
                rule: Some(rule),
                block: None,
                entry,
                captures,
            });
        }
        candidates
    }

    /// An element's own character data, references decoded.
    fn own_text(&self, element: EntryId) -> String {
        self.element(element)
            .content
            .iter()
            .filter_map(|c| match c {
                XmlContent::Text(t) => Some(t.as_str()),
                XmlContent::Element(_) => None,
            })
            .collect()
    }

    /// All character data inside an element, its descendants' included.
    fn all_text(&self, element: EntryId) -> String {
        self.element(element)
            .content
            .iter()
            .map(|c| match c {
                XmlContent::Text(t) => t.clone(),
                XmlContent::Element(e) => self.all_text(*e),
            })
            .collect()
    }

    pub(super) fn xml_cel_value(&self, entry: EntryId) -> Value {
        let mut map = Map::default();
        for attribute in &self.element(entry).attributes {
            map.insert(attribute.name.clone(), Value::from(attribute.text.as_str()));
        }
        map.insert("text", Value::from(self.own_text(entry)));
        Value::map(map)
    }

    pub(super) fn xml_read(&self, candidate: &Candidate, slot: &Slot, html_paragraphs: bool) -> SlotRead {
        let element = candidate.entry;
        if let Some(name) = &slot.capture {
            return match capture(&candidate.captures, name) {
                Some(key) => SlotRead::new(Value::from(key), None, true, false)
                    .with_reason("An element's name is not rewritten in this file."),
                None => SlotRead::absent(),
            };
        }
        let target = match &slot.child {
            None => Some(element),
            Some(child) => self.child(element, child),
        };
        if let Some(name) = &slot.xml_attribute {
            return match target {
                None => SlotRead::absent(),
                Some(target) => self.read_attribute(target, name),
            };
        }
        if slot.text {
            let Some(target) = target else {
                return SlotRead::absent();
            };
            let t = self.element(target);
            let writable = t.unreadable.is_none();
            if html_paragraphs {
                let body = self.child(target, "html/body").unwrap_or(target);
                let b = self.element(body);
                let span = (!b.self_closed).then(|| Span::new(b.content_start(), b.content_end()));
                let mut read = SlotRead::new(
                    Value::from(self.paragraphs(body)),
                    span,
                    true,
                    writable && !b.self_closed,
                );
                read.node = Node::XmlText {
                    element: target,
                    span,
                    html: true,
                };
                return read;
            }
            if t.self_closed {
                let mut read = SlotRead::new(Value::from(""), None, true, writable);
                read.node = Node::XmlText {
                    element: target,
                    span: None,
                    html: false,
                };
                return read;
            }
            let first_child = t.elements().next();
            let span = Span::new(
                t.content_start(),
                first_child
                    .map(|c| self.entries[c].own.start)
                    .unwrap_or(t.content_end()),
            );
            let value: String = t
                .content
                .iter()
                .map_while(|c| match c {
                    XmlContent::Text(text) => Some(text.as_str()),
                    XmlContent::Element(_) => None,
                })
                .collect();
            let mut read = SlotRead::new(Value::from(value.as_str()), Some(span), true, writable);
            read.node = Node::XmlText {
                element: target,
                span: Some(span),
                html: false,
            };
            read.wire = Some(value);
            return read;
        }
        SlotRead::read_only_absent(format!("An xml entry has no {}.", slot.describe()))
    }

    fn read_attribute(&self, element: EntryId, name: &str) -> SlotRead {
        let e = self.element(element);
        if let Some((index, attribute)) = e.attribute(name) {
            let mut read = SlotRead::new(
                Value::from(attribute.text.as_str()),
                Some(attribute.value),
                true,
                e.unreadable.is_none(),
            );
            read.node = Node::XmlAttribute(element, index);
            read.wire = Some(attribute.text.clone());
            return read;
        }
        let mut read = SlotRead::absent();
        read.node = Node::XmlElement(element);
        read
    }

    /// An html body as plain text (FBL section 4.5): one line per `p` element, whitespace inside a paragraph
    /// collapsed.
    fn paragraphs(&self, body: EntryId) -> String {
        let paragraphs: Vec<EntryId> = self
            .descendants(body)
            .into_iter()
            .filter(|e| self.entries[*e].name.as_deref() == Some("p"))
            .collect();
        if paragraphs.is_empty() {
            return collapse(&self.all_text(body));
        }
        paragraphs
            .iter()
            .map(|p| collapse(&self.all_text(*p)))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn descendants(&self, element: EntryId) -> Vec<EntryId> {
        let mut result = Vec::new();
        for child in self.element(element).elements() {
            result.push(child);
            result.extend(self.descendants(child));
        }
        result
    }

    pub(super) fn xml_read_raw(&self, entry: EntryId, name: &str) -> SlotRead {
        match &self.entries[entry].kind {
            EntryKind::Xml(_) => self.read_attribute(entry, name),
            _ => SlotRead::absent(),
        }
    }

    pub(super) fn xml_header_holds(&self, header: &HeaderSettings) -> bool {
        let Some(line) = &header.line else { return true };
        let Some(document) = self.xml_data().document else {
            return false;
        };
        self.regex(line, false)
            .is_match(self.text.text_of(self.element(document).start_tag))
    }

    // ---- writing (FBL section 6) ----

    pub(super) fn xml_format(&self, read: &SlotRead, binding: Option<&AttributeBinding>, value: &Value) -> String {
        let plain = match binding {
            None => new_text::plain(value, None),
            Some(b) => {
                new_text::wire(b, value, read.wire.as_deref()).unwrap_or_else(|| new_text::plain(value, Some(b)))
            }
        };
        match read.node.inner() {
            Node::XmlText { html: true, .. } => html_paragraphs(&plain),
            Node::XmlText { .. } => escape_text(&plain),
            _ => escape_attribute(&plain),
        }
    }

    pub(super) fn xml_write(&self, plan: &mut Plan<'_>, element: &ReadElement, changes: &[SlotChange<'_>]) -> Planning {
        let owner = element.entry();
        for change in changes {
            let read = &change.read;
            let binding = change.binding;
            match read.node.inner() {
                Node::XmlAttribute(e, index) if read.present => {
                    let attribute = &self.element(*e).attributes[*index];
                    if change.is_empty && binding.empty.as_deref() == Some("remove") {
                        plan.add(
                            Operation::RemoveKey,
                            self.whitespace_before(attribute.own.start),
                            attribute.own.end,
                            "",
                        );
                    } else {
                        let written = self.xml_format(read, Some(binding), &change.value);
                        if written != self.text.text_of(attribute.value) {
                            plan.add_span(Operation::ReplaceValue, attribute.value, written);
                        }
                    }
                    continue;
                }
                Node::XmlText {
                    element: text_element,
                    span,
                    ..
                } if read.present => {
                    let removes_child =
                        *text_element != owner && (binding.create.is_some() || matches!(read.node, Node::Override(_)));
                    if change.is_empty && binding.empty.as_deref() == Some("remove") && removes_child {
                        let parent = self.entries[*text_element]
                            .parent
                            .expect("a child element has a parent");
                        self.remove_child(plan, parent, *text_element, Operation::RemoveKey);
                    } else if let Some(span) = span {
                        let written = self.xml_format(read, Some(binding), &change.value);
                        if written != self.text.text_of(*span) {
                            plan.add_span(Operation::ReplaceValue, *span, written);
                        }
                    } else {
                        let content = self.xml_format(read, Some(binding), &change.value);
                        self.open(plan, *text_element, &content, Operation::ReplaceValue, true);
                    }
                    continue;
                }
                _ => {}
            }
            if let Some(name) = &binding.xml_attribute {
                let Node::XmlElement(target) = read.node.inner() else {
                    return refuse(format!(
                        "The {} file has no {} to write \"{name}\" in.",
                        self.binding.name,
                        binding.child.as_deref().unwrap_or_default()
                    ));
                };
                // After the nearest attribute before it in the rule's own order, where the order places it before
                // another that is there; else after the last attribute (FBL section 6.3).
                let t = self.element(*target);
                let order = self.rule(change.rule).wire_order();
                let place = order.iter().position(|n| n == name);
                let before = place.and_then(|place| {
                    t.attributes
                        .iter()
                        .rfind(|a| order.iter().position(|n| *n == a.name).is_some_and(|at| at < place))
                });
                let offset = match (before, t.attributes.last()) {
                    (Some(before), _) => before.own.end,
                    (None, Some(last)) => last.own.end,
                    (None, None) => t.name_end,
                };
                plan.add(
                    Operation::InsertKey,
                    offset,
                    offset,
                    format!(" {name}=\"{}\"", self.xml_format(read, Some(binding), &change.value)),
                );
                continue;
            }
            if binding.text
                && let Some(create) = &binding.create
            {
                let mut text_read = read.clone();
                text_read.node = Node::XmlText {
                    element: owner,
                    span: None,
                    html: binding.html_paragraphs,
                };
                let content = self.xml_format(&text_read, Some(binding), &change.value);
                let rendered = new_text::render(&create.emit, &|p| (p == "value").then(|| content.clone()));
                self.create_child(plan, owner, create, &rendered);
                continue;
            }
            return refuse(format!(
                "The {} file has no {} to write the {} in.",
                self.binding.name,
                binding.child.as_deref().unwrap_or("element"),
                change.attribute
            ));
        }
        Ok(())
    }

    /// The start of the whitespace directly before `offset`.
    fn whitespace_before(&self, mut offset: usize) -> usize {
        while offset > 0 && matches!(self.text.bytes()[offset - 1], b' ' | b'\t' | b'\r' | b'\n') {
            offset -= 1;
        }
        offset
    }

    /// The bytes from the start of an element's line to its first byte.
    fn indent_of(&self, entry: EntryId) -> String {
        let start = self.entries[entry].own.start;
        self.text
            .text(self.text.lines()[self.text.line_index_at(start)].start, start)
            .to_owned()
    }

    fn child_indent(&self, parent: EntryId) -> String {
        self.indent_of(parent) + &self.indentation(self.step())
    }

    /// Opens a self-closed element to hold `content` (FBL section 6.1 `self-close`): its `/>` becomes `>`, the
    /// content follows on its own line, and a new end tag closes it on the line after, at the element's indentation.
    fn open(&self, plan: &mut Plan<'_>, element: EntryId, content: &str, operation: Operation, inline: bool) {
        let e = self.element(element);
        let name = self.entries[element].name.as_deref().unwrap_or_default();
        let at = e.close.end;
        plan.add_span(Operation::SelfClose, e.close, ">");
        if inline || self.entries[element].line_span.is_none() {
            plan.add(operation, at, at, content);
            plan.add(Operation::SelfClose, at, at, format!("</{name}>"));
            return;
        }
        let newline = self.newline_at(e.close.start);
        plan.add(
            operation,
            at,
            at,
            format!("{newline}{}{content}", self.child_indent(element)),
        );
        plan.add(
            Operation::SelfClose,
            at,
            at,
            format!("{newline}{}</{name}>", self.indent_of(element)),
        );
    }

    /// Appends `content` as the element's last child, on its own line when the last child is on one (FBL 6.3).
    fn append(&self, plan: &mut Plan<'_>, element: EntryId, content: &str, operation: Operation) {
        let e = self.element(element);
        if e.self_closed {
            self.open(plan, element, content, operation, false);
            return;
        }
        if let Some(last) = e.elements().last() {
            let l = &self.entries[last];
            match l.line_span {
                Some(line) => {
                    let line_of_last = self.text.lines()[self.text.line_index_at(l.own.end - 1)];
                    let ending = if line_of_last.ending.is_empty() {
                        self.newline_at(l.own.end)
                    } else {
                        line_of_last.ending.to_owned()
                    };
                    plan.add(
                        operation,
                        line.end,
                        line.end,
                        format!("{}{content}{ending}", self.indent_of(last)),
                    );
                }
                None => plan.add(operation, l.own.end, l.own.end, content),
            }
            return;
        }
        let end = e.end_tag.expect("an element that is not self-closed has an end tag");
        if self.only_whitespace_before(end.start)
            && self.text.line_index_at(end.start) != self.text.line_index_at(e.start_tag.end)
        {
            let line_start = self.text.lines()[self.text.line_index_at(end.start)].start;
            plan.add(
                operation,
                line_start,
                line_start,
                format!("{}{content}{}", self.child_indent(element), self.newline_at(line_start)),
            );
            return;
        }
        plan.add(operation, end.start, end.start, content);
    }

    /// A child element holding a value (FBL section 5.2 `create`), placed first, last or before the first child of
    /// a name.
    fn create_child(&self, plan: &mut Plan<'_>, element: EntryId, create: &CreateChild, content: &str) {
        let e = self.element(element);
        if e.self_closed {
            self.open(plan, element, content, Operation::InsertKey, false);
            return;
        }
        let before = match create.place.as_str() {
            "first" => e.elements().next(),
            "before" => e
                .elements()
                .find(|c| self.entries[*c].name.as_deref() == create.before.as_deref()),
            _ => None,
        };
        let Some(before) = before else {
            self.append(plan, element, content, Operation::InsertKey);
            return;
        };
        let b = &self.entries[before];
        if b.line_span.is_some() {
            let offset = self.text.lines()[self.text.line_index_at(b.own.start) - 1].content_end;
            plan.add(
                Operation::InsertKey,
                offset,
                offset,
                format!("{}{}{content}", self.newline_at(offset), self.indent_of(before)),
            );
            return;
        }
        plan.add(Operation::InsertKey, b.own.start, b.own.start, content);
    }

    /// Removes a child element holding a value with the line break before it, and closes the parent again when
    /// only whitespace is left in it (`self-close`).
    fn remove_child(&self, plan: &mut Plan<'_>, parent: EntryId, child: EntryId, operation: Operation) {
        let c = &self.entries[child];
        let p = self.element(parent);
        let line = self.text.line_index_at(c.own.start);
        let mut span = if c.line_span.is_some() && line > 0 {
            c.own.with_start(self.text.lines()[line - 1].content_end)
        } else {
            c.own
        };
        if span.start < p.content_start() {
            span = span.with_start(p.content_start());
        }
        let closes = p.end_tag.is_some_and(|end| {
            self.is_whitespace(Span::new(p.content_start(), span.start))
                && self.is_whitespace(Span::new(span.end, end.start))
        });
        if !closes {
            plan.add_span(operation, span, "");
            return;
        }
        plan.add(Operation::SelfClose, p.close.start, span.start, "/>");
        plan.add_span(operation, span, "");
        plan.add(Operation::SelfClose, span.end, p.end_tag.expect("an end tag").end, "");
    }

    pub(super) fn xml_insert(&self, plan: &mut Plan<'_>, request: &InsertRequest) -> Planning {
        let reading = plan.reading;
        let rule = self.rule(request.rule);
        let insert = rule.insert.as_ref().expect("an insertable rule");
        // FBL section 5's place: {before: key} is not implemented here yet.
        if insert.place == "before" {
            return refuse(format!(
                "A {} body cannot place a new entry '{}'.",
                self.family_name(),
                insert.place
            ));
        }
        let equals = |e: EntryId, a: &str, v: &str| self.attribute_equals(e, a, v);
        let from_parent = request
            .parent
            .map(|p| reading.element(p).entry())
            .filter(|e| matches!(self.entries[*e].kind, EntryKind::Xml(_)));
        let from_container = || {
            let container = insert.container.as_ref()?;
            let root = self.xml_data().root?;
            selector::matches(
                &self.entries,
                selector::start(container, root, None),
                container,
                Some(&equals),
            )
            .into_iter()
            .next()
            .map(|(e, _)| e)
        };
        let Some(parent) = from_parent.or_else(from_container).or(self.xml_data().document) else {
            return refuse(format!(
                "The {} file has no element to add the {} to.",
                self.binding.name, rule.type_name
            ));
        };
        let text = match &insert.emit {
            Some(emit) => new_text::render(emit, &|name| match name {
                "id" => request.id.as_ref().map(|id| escape_attribute(id)),
                "source" => request.source.map(|s| escape_attribute(&reading.element(s).key)),
                "target" => request.target.map(|t| escape_attribute(&reading.element(t).key)),
                _ => request
                    .values
                    .get(name)
                    .filter(|v| !v.is_null())
                    .map(|v| escape_attribute(&new_text::plain(v, rule.attribute(name)))),
            }),
            None => self.new_element(plan, request),
        };
        if insert.place == "start"
            && let Some(first) = self.element(parent).elements().next()
        {
            let f = &self.entries[first];
            if f.line_span.is_some() {
                let line_start = self.text.lines()[self.text.line_index_at(f.own.start)].start;
                plan.add(
                    Operation::InsertEntry,
                    line_start,
                    line_start,
                    format!("{}{text}{}", self.indent_of(first), self.newline_at(line_start)),
                );
            } else {
                plan.add(Operation::InsertEntry, f.own.start, f.own.start, text);
            }
            return Ok(());
        }
        self.append(plan, parent, &text, Operation::InsertEntry);
        Ok(())
    }

    /// A new element without an `emit`: the rule's element name with its attributes in `insert.keys` order, then
    /// binding order (FBL section 6.3).
    fn new_element(&self, plan: &Plan<'_>, request: &InsertRequest) -> String {
        let rule = self.rule(request.rule);
        let reading = plan.reading;
        let at = rule.at.as_deref().unwrap_or_default();
        let last = at.split('/').rfind(|s| !s.is_empty()).unwrap_or_default();
        let name = match last.find('[') {
            Some(bracket) if bracket > 0 => &last[..bracket],
            _ => last,
        };
        let mut written: Vec<(String, String)> = Vec::new();
        if let Some(id_attribute) = rule
            .id
            .as_ref()
            .and_then(|i| i.from.as_ref())
            .and_then(|f| f.xml_attribute.clone())
            && let Some(id) = &request.id
        {
            written.push((id_attribute, id.clone()));
        }
        for (attribute, binding) in &rule.attributes {
            if let Some(xml) = &binding.xml_attribute
                && binding.child.is_none()
                && let Some(value) = request.values.get(attribute)
                && !value.is_null()
            {
                written.push((xml.clone(), new_text::plain(value, Some(binding))));
            }
        }
        if let Some(source) = rule.source.as_ref().and_then(|s| s.xml_attribute.clone())
            && let Some(s) = request.source
        {
            written.push((source, reading.element(s).key.clone()));
        }
        if let Some(target) = rule.target.as_ref().and_then(|t| t.xml_attribute.clone())
            && let Some(t) = request.target
        {
            written.push((target, reading.element(t).key.clone()));
        }
        let keys = &rule.insert.as_ref().expect("an insertable rule").keys;
        written.sort_by_key(|(n, _)| keys.iter().position(|k| k == n).unwrap_or(keys.len()));
        let attributes: String = written
            .iter()
            .map(|(n, v)| format!(" {n}=\"{}\"", escape_attribute(v)))
            .collect();
        format!("<{name}{attributes}/>")
    }

    pub(super) fn xml_remove(&self, plan: &mut Plan<'_>, element: usize) -> Planning {
        let reading = plan.reading;
        let read_element = reading.element(element);
        let entry = read_element.entry();
        if Some(entry) == self.xml_data().document {
            return refuse(format!(
                "The document element of the {} file cannot be removed.",
                self.binding.name
            ));
        }
        // The only child of an element that an insert may have opened for it (`last-child`, FBL section 6.2), or
        // of one that goes when it is left empty: the parent is closed again, so that adding a child and removing
        // it leaves the bytes it started from.
        let rule = self.rule(read_element.rule);
        if (rule.remove.as_ref().is_some_and(|r| r.remove_container_when_empty)
            || rule.insert.as_ref().is_some_and(|i| i.place == "last-child"))
            && let Some(parent) = self.entries[entry].parent
            && let EntryKind::Xml(p) = &self.entries[parent].kind
            && !p.is_root
            && !p.self_closed
            && Some(parent) != self.xml_data().document
            && p.elements().count() == 1
        {
            self.remove_child(plan, parent, entry, Operation::RemoveEntry);
            return Ok(());
        }
        plan.add_span(Operation::RemoveEntry, self.entries[entry].removal_span(), "");
        Ok(())
    }
}

fn collapse(text: &str) -> String {
    WHITESPACE.replace_all(text, " ").trim().to_owned()
}

/// Text escaping (FBL section 4.5): `&amp;`, `&lt;` and `&gt;`.
pub(crate) fn escape_text(value: &str) -> String {
    value.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Attribute escaping (FBL section 4.5): the text escapes, and `"`, LF and CR.
pub(crate) fn escape_attribute(value: &str) -> String {
    escape_text(value)
        .replace('"', "&quot;")
        .replace('\n', "&#xa;")
        .replace('\r', "&#xd;")
}

/// Plain text as html paragraphs (FBL section 4.5): one `<p>line</p>` per line.
pub(crate) fn html_paragraphs(value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }
    value
        .replace("\r\n", "\n")
        .split('\n')
        .map(|line| format!("<p>{}</p>", escape_text(line)))
        .collect()
}
