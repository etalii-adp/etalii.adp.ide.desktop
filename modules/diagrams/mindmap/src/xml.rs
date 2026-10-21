//! The XML a map is read from and written back as (etalii.adp spec 030, FR-031 and FR-032), ported from what the
//! standalone host's mind map module at `8b6ccfc` took from .NET: `XDocument.Parse` with
//! `LoadOptions.PreserveWhitespace | LoadOptions.SetLineInfo`, and the module's own `FreeplaneXmlWriter.cs`.
//!
//! The document is an arena of nodes, so an element keeps its identity while it is detached and put back: elements
//! with their attributes in the order they were read, text (whitespace included, exactly as loaded, a CDATA section as
//! text), comments, processing instructions and the document type's name. The writer is the module's: no XML
//! declaration, `/>` with no space before it, attributes in their order, every name by its local name, and only what
//! XML requires escaped plus what Freeplane escapes (`&quot;`, and a newline or a carriage return inside an attribute
//! as `&#xa;` and `&#xd;`). Reading follows XML 1.0 as .NET's reader does: a reference is expanded, a literal tab,
//! newline or carriage return in an attribute value reads as a space, and a lone carriage return in text as a newline.
//! The carriage returns of CRLF line breaks survive because the caller escapes them before parsing
//! ([`crate::document`]).

use std::fmt;

/// An index into a document's arena.
pub type NodeId = usize;

/// What a node of the arena is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Element {
        name: String,
        attributes: Vec<(String, String)>,
    },
    Text(String),
    Comment(String),
    Instruction {
        target: String,
        data: String,
    },
    DocType(String),
}

#[derive(Debug, Clone)]
struct Node {
    kind: Kind,
    parent: Option<NodeId>,
    children: Vec<NodeId>,
    /// The 1-based line the node starts on; 0 for a node made since the read.
    line: usize,
}

/// Why a text is not well-formed XML, and the 1-based line the reader stopped at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlError {
    pub message: String,
    pub line: usize,
}

impl fmt::Display for XmlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} Line {}.", self.message, self.line)
    }
}

/// A parsed XML document: its top-level nodes and every node beneath them.
#[derive(Debug, Clone, Default)]
pub struct XmlDocument {
    nodes: Vec<Node>,
    top: Vec<NodeId>,
}

impl XmlDocument {
    /// Parses `text`, or says why it is not well-formed.
    pub fn parse(text: &str) -> Result<XmlDocument, XmlError> {
        Parser::new(text).document()
    }

    /// Parses one element and what it holds, as a fragment.
    pub fn parse_fragment(text: &str) -> Result<(XmlDocument, NodeId), XmlError> {
        let document = Parser::new(text).document()?;
        let root = document.root().ok_or_else(|| XmlError {
            message: "Root element is missing.".into(),
            line: 1,
        })?;
        Ok((document, root))
    }

    /// An empty document.
    pub fn new() -> Self {
        Self::default()
    }

    fn push(&mut self, kind: Kind, line: usize) -> NodeId {
        self.nodes.push(Node {
            kind,
            parent: None,
            children: Vec::new(),
            line,
        });
        self.nodes.len() - 1
    }

    /// A new element, detached.
    pub fn new_element(&mut self, name: &str, attributes: Vec<(String, String)>) -> NodeId {
        self.push(
            Kind::Element {
                name: name.to_owned(),
                attributes,
            },
            0,
        )
    }

    /// A new text node, detached.
    pub fn new_text(&mut self, text: &str) -> NodeId {
        self.push(Kind::Text(text.to_owned()), 0)
    }

    /// The top-level nodes, in order.
    pub fn top(&self) -> &[NodeId] {
        &self.top
    }

    /// Appends `node` at the top level.
    pub fn push_top(&mut self, node: NodeId) {
        self.nodes[node].parent = None;
        self.top.push(node);
    }

    /// The document's element.
    pub fn root(&self) -> Option<NodeId> {
        self.top.iter().copied().find(|n| self.is_element(*n))
    }

    pub fn kind(&self, node: NodeId) -> &Kind {
        &self.nodes[node].kind
    }

    pub fn line(&self, node: NodeId) -> usize {
        self.nodes[node].line
    }

    pub fn is_element(&self, node: NodeId) -> bool {
        matches!(self.nodes[node].kind, Kind::Element { .. })
    }

    /// An element's name as written, prefix included; empty for any other node.
    pub fn name(&self, node: NodeId) -> &str {
        match &self.nodes[node].kind {
            Kind::Element { name, .. } => name,
            _ => "",
        }
    }

    /// An element's local name: its name without the prefix.
    pub fn local_name(&self, node: NodeId) -> &str {
        local(self.name(node))
    }

    /// The node's parent element; none at the top level or detached.
    pub fn parent(&self, node: NodeId) -> Option<NodeId> {
        self.nodes[node].parent
    }

    /// The nodes an element holds, in order.
    pub fn children(&self, node: NodeId) -> &[NodeId] {
        &self.nodes[node].children
    }

    /// The child elements named `name`, in order.
    pub fn elements_named<'a>(&'a self, node: NodeId, name: &'a str) -> impl Iterator<Item = NodeId> + 'a {
        self.children(node)
            .iter()
            .copied()
            .filter(move |c| self.is_element(*c) && self.name(*c) == name)
    }

    /// Every element named `name` beneath `node`, in document order.
    pub fn descendants_named(&self, node: NodeId, name: &str) -> Vec<NodeId> {
        let mut found = Vec::new();
        self.collect(node, name, &mut found);
        found
    }

    fn collect(&self, node: NodeId, name: &str, found: &mut Vec<NodeId>) {
        for child in self.children(node) {
            if self.is_element(*child) {
                if self.name(*child) == name {
                    found.push(*child);
                }
                self.collect(*child, name, found);
            }
        }
    }

    /// Every element beneath `node`, in document order.
    pub fn descendant_elements(&self, node: NodeId) -> Vec<NodeId> {
        let mut found = Vec::new();
        let mut stack: Vec<NodeId> = self.children(node).iter().rev().copied().collect();
        while let Some(next) = stack.pop() {
            if self.is_element(next) {
                found.push(next);
                stack.extend(self.children(next).iter().rev().copied());
            }
        }
        found
    }

    /// The value of an element's attribute `name`.
    pub fn attribute(&self, node: NodeId, name: &str) -> Option<&str> {
        match &self.nodes[node].kind {
            Kind::Element { attributes, .. } => attributes.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str()),
            _ => None,
        }
    }

    /// Sets an element's attribute, as `XElement.SetAttributeValue` does: an existing one keeps its place, a new one
    /// goes last, and none removes it.
    pub fn set_attribute(&mut self, node: NodeId, name: &str, value: Option<&str>) {
        if let Kind::Element { attributes, .. } = &mut self.nodes[node].kind {
            match (attributes.iter().position(|(n, _)| n == name), value) {
                (Some(at), Some(value)) => attributes[at].1 = value.to_owned(),
                (Some(at), None) => {
                    attributes.remove(at);
                }
                (None, Some(value)) => attributes.push((name.to_owned(), value.to_owned())),
                (None, None) => {}
            }
        }
    }

    /// The text of a text node; none for any other node.
    pub fn text(&self, node: NodeId) -> Option<&str> {
        match &self.nodes[node].kind {
            Kind::Text(text) => Some(text),
            _ => None,
        }
    }

    /// Whether `node` is a text node of whitespace alone.
    pub fn is_whitespace(&self, node: NodeId) -> bool {
        self.text(node).is_some_and(|t| t.chars().all(char::is_whitespace))
    }

    /// The concatenated text beneath an element, as `XElement.Value` reads it.
    pub fn value(&self, node: NodeId) -> String {
        let mut value = String::new();
        self.append_value(node, &mut value);
        value
    }

    fn append_value(&self, node: NodeId, value: &mut String) {
        for child in self.children(node) {
            match &self.nodes[*child].kind {
                Kind::Text(text) => value.push_str(text),
                Kind::Element { .. } => self.append_value(*child, value),
                _ => {}
            }
        }
    }

    /// The node after `node` among its parent's, when there is one.
    pub fn next_sibling(&self, node: NodeId) -> Option<NodeId> {
        let siblings = self.siblings(node)?;
        let at = siblings.iter().position(|n| *n == node)?;
        siblings.get(at + 1).copied()
    }

    /// The last node an element holds.
    pub fn last_child(&self, node: NodeId) -> Option<NodeId> {
        self.children(node).last().copied()
    }

    fn siblings(&self, node: NodeId) -> Option<&Vec<NodeId>> {
        match self.nodes[node].parent {
            Some(parent) => Some(&self.nodes[parent].children),
            None => self.top.contains(&node).then_some(&self.top),
        }
    }

    /// Detaches `node` from its parent; it stays in the arena and can be put back.
    pub fn remove(&mut self, node: NodeId) {
        match self.nodes[node].parent.take() {
            Some(parent) => self.nodes[parent].children.retain(|n| *n != node),
            None => self.top.retain(|n| *n != node),
        }
    }

    /// Detaches everything an element holds.
    pub fn remove_children(&mut self, node: NodeId) {
        for child in std::mem::take(&mut self.nodes[node].children) {
            self.nodes[child].parent = None;
        }
    }

    /// Appends `child` to an element's content.
    pub fn append(&mut self, parent: NodeId, child: NodeId) {
        self.nodes[child].parent = Some(parent);
        self.nodes[parent].children.push(child);
    }

    /// Puts `nodes` right before `before`, under the same parent.
    pub fn insert_before(&mut self, before: NodeId, nodes: &[NodeId]) {
        let parent = self.nodes[before].parent.expect("an attached node");
        let at = self.nodes[parent]
            .children
            .iter()
            .position(|n| *n == before)
            .expect("a child");
        for (offset, node) in nodes.iter().enumerate() {
            self.nodes[*node].parent = Some(parent);
            self.nodes[parent].children.insert(at + offset, *node);
        }
    }

    /// Puts `nodes` right after `after`, under the same parent.
    pub fn insert_after(&mut self, after: NodeId, nodes: &[NodeId]) {
        let parent = self.nodes[after].parent.expect("an attached node");
        let at = self.nodes[parent]
            .children
            .iter()
            .position(|n| *n == after)
            .expect("a child");
        for (offset, node) in nodes.iter().enumerate() {
            self.nodes[*node].parent = Some(parent);
            self.nodes[parent].children.insert(at + 1 + offset, *node);
        }
    }

    /// Copies `node` of `other`, and everything it holds, into this document, detached.
    pub fn import(&mut self, other: &XmlDocument, node: NodeId) -> NodeId {
        let copy = self.push(other.nodes[node].kind.clone(), other.nodes[node].line);
        for child in other.children(node).to_vec() {
            let imported = self.import(other, child);
            self.append(copy, imported);
        }
        copy
    }

    // ---- writing --------------------------------------------------------------------------------------------------

    /// The document as Freeplane writes it.
    pub fn write(&self) -> String {
        let mut output = String::new();
        for node in &self.top {
            self.write_node(*node, &mut output);
        }
        output
    }

    /// One node as Freeplane text.
    pub fn write_node_text(&self, node: NodeId) -> String {
        let mut output = String::new();
        self.write_node(node, &mut output);
        output
    }

    fn write_node(&self, node: NodeId, output: &mut String) {
        match &self.nodes[node].kind {
            Kind::Element { name, attributes } => {
                output.push('<');
                output.push_str(local(name));
                for (attribute, value) in attributes {
                    output.push(' ');
                    output.push_str(local(attribute));
                    output.push_str("=\"");
                    escape_attribute(value, output);
                    output.push('"');
                }
                // An element with no content, or whose content is one empty text, has no nodes to .NET.
                let children: Vec<NodeId> = self.nodes[node]
                    .children
                    .iter()
                    .copied()
                    .filter(|c| self.text(*c) != Some(""))
                    .collect();
                if children.is_empty() {
                    output.push_str("/>");
                    return;
                }
                output.push('>');
                for child in children {
                    self.write_node(child, output);
                }
                output.push_str("</");
                output.push_str(local(name));
                output.push('>');
            }
            Kind::Text(text) => escape_text(text, output),
            Kind::Comment(comment) => {
                output.push_str("<!--");
                output.push_str(comment);
                output.push_str("-->");
            }
            Kind::Instruction { target, data } => {
                output.push_str("<?");
                output.push_str(target);
                output.push(' ');
                output.push_str(data);
                output.push_str("?>");
            }
            Kind::DocType(name) => {
                output.push_str("<!DOCTYPE ");
                output.push_str(name);
                output.push('>');
            }
        }
    }
}

/// A name without its prefix, as `XName.LocalName` gives it.
fn local(name: &str) -> &str {
    name.split_once(':').map_or(name, |(_, local)| local)
}

fn escape_text(value: &str, output: &mut String) {
    for c in value.chars() {
        match c {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            other => output.push(other),
        }
    }
}

fn escape_attribute(value: &str, output: &mut String) {
    for c in value.chars() {
        match c {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\n' => output.push_str("&#xa;"),
            // A raw CR in an attribute would be read as a space by the next parser, so it leaves escaped.
            '\r' => output.push_str("&#xd;"),
            other => output.push(other),
        }
    }
}

// ---- reading -------------------------------------------------------------------------------------------------------

struct Parser<'a> {
    text: &'a str,
    position: usize,
    line: usize,
    document: XmlDocument,
}

fn is_name_start(c: char) -> bool {
    c.is_alphabetic() || c == '_' || c == ':' || (!c.is_ascii() && !c.is_whitespace())
}

fn is_name_char(c: char) -> bool {
    is_name_start(c) || c.is_ascii_digit() || c == '-' || c == '.' || c == '\u{B7}'
}

fn is_xml_whitespace(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r')
}

impl<'a> Parser<'a> {
    fn new(text: &'a str) -> Self {
        Parser {
            text,
            position: 0,
            line: 1,
            document: XmlDocument::new(),
        }
    }

    fn rest(&self) -> &'a str {
        &self.text[self.position..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn starts_with(&self, prefix: &str) -> bool {
        self.rest().starts_with(prefix)
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.position += c.len_utf8();
        if c == '\n' || (c == '\r' && self.peek() != Some('\n')) {
            self.line += 1;
        }
        Some(c)
    }

    fn advance(&mut self, count: usize) {
        for _ in 0..count {
            self.bump();
        }
    }

    fn error<T>(&self, message: impl Into<String>) -> Result<T, XmlError> {
        Err(XmlError {
            message: message.into(),
            line: self.line,
        })
    }

    fn skip_whitespace(&mut self) -> bool {
        let mut any = false;
        while self.peek().is_some_and(is_xml_whitespace) {
            self.bump();
            any = true;
        }
        any
    }

    fn document(mut self) -> Result<XmlDocument, XmlError> {
        if self.starts_with("<?xml")
            && self.rest()[5..]
                .chars()
                .next()
                .is_some_and(|c| is_xml_whitespace(c) || c == '?')
        {
            // The declaration is not one of the document's nodes, so the writer never writes it.
            match self.rest().find("?>") {
                Some(end) => {
                    let length = self.rest()[..end + 2].chars().count();
                    self.advance(length);
                }
                None => return self.error("Unexpected end of file while parsing the XML declaration."),
            }
        }
        let mut root_seen = false;
        while let Some(c) = self.peek() {
            if is_xml_whitespace(c) {
                let line = self.line;
                let start = self.position;
                self.skip_whitespace();
                let text = self.text[start..self.position].to_owned();
                let node = self.document.push(Kind::Text(text), line);
                self.document.push_top(node);
            } else if self.starts_with("<!--") {
                let node = self.comment()?;
                self.document.push_top(node);
            } else if self.starts_with("<?") {
                let node = self.instruction()?;
                self.document.push_top(node);
            } else if self.starts_with("<!DOCTYPE") {
                if root_seen {
                    return self.error("Unexpected DTD declaration.");
                }
                let node = self.doctype()?;
                self.document.push_top(node);
            } else if c == '<' {
                if root_seen {
                    return self.error("There are multiple root elements.");
                }
                let node = self.element()?;
                self.document.push_top(node);
                root_seen = true;
            } else {
                return self.error("Data at the root level is invalid.");
            }
        }
        if !root_seen {
            return self.error("Root element is missing.");
        }
        Ok(self.document)
    }

    fn comment(&mut self) -> Result<NodeId, XmlError> {
        let line = self.line;
        self.advance(4);
        let Some(end) = self.rest().find("-->") else {
            return self.error("Unexpected end of file while parsing Comment has occurred.");
        };
        let value = self.rest()[..end].to_owned();
        if value.contains("--") || value.ends_with('-') {
            return self.error("An XML comment cannot contain '--', and '-' cannot be the last character.");
        }
        let length = self.rest()[..end + 3].chars().count();
        self.advance(length);
        Ok(self.document.push(Kind::Comment(value), line))
    }

    fn instruction(&mut self) -> Result<NodeId, XmlError> {
        let line = self.line;
        self.advance(2);
        let target = self.name()?;
        if target.eq_ignore_ascii_case("xml") {
            return self
                .error("Unexpected XML declaration. The XML declaration must be the first node in the document.");
        }
        self.skip_whitespace();
        let Some(end) = self.rest().find("?>") else {
            return self.error("Unexpected end of file while parsing PI has occurred.");
        };
        let data = self.rest()[..end].to_owned();
        let length = self.rest()[..end + 2].chars().count();
        self.advance(length);
        Ok(self.document.push(Kind::Instruction { target, data }, line))
    }

    fn doctype(&mut self) -> Result<NodeId, XmlError> {
        let line = self.line;
        self.advance("<!DOCTYPE".len());
        if !self.skip_whitespace() {
            return self.error("Expecting whitespace after DOCTYPE.");
        }
        let name = self.name()?;
        let mut depth = 0;
        let mut quote: Option<char> = None;
        loop {
            let Some(c) = self.bump() else {
                return self.error("Unexpected end of file while parsing DOCTYPE has occurred.");
            };
            match (quote, c) {
                (Some(q), c) if c == q => quote = None,
                (Some(_), _) => {}
                (None, '"' | '\'') => quote = Some(c),
                (None, '[') => depth += 1,
                (None, ']') => depth -= 1,
                (None, '>') if depth == 0 => break,
                _ => {}
            }
        }
        Ok(self.document.push(Kind::DocType(name), line))
    }

    fn name(&mut self) -> Result<String, XmlError> {
        let start = self.position;
        match self.peek() {
            Some(c) if is_name_start(c) => {
                self.bump();
            }
            Some(c) => return self.error(format!("Name cannot begin with the '{c}' character.")),
            None => return self.error("Unexpected end of file has occurred."),
        }
        while self.peek().is_some_and(is_name_char) {
            self.bump();
        }
        Ok(self.text[start..self.position].to_owned())
    }

    fn element(&mut self) -> Result<NodeId, XmlError> {
        let line = self.line;
        self.bump();
        let name = self.name()?;
        let mut attributes: Vec<(String, String)> = Vec::new();
        loop {
            let spaced = self.skip_whitespace();
            match self.peek() {
                Some('/') if self.starts_with("/>") => {
                    self.advance(2);
                    return Ok(self.document.push(Kind::Element { name, attributes }, line));
                }
                Some('>') => {
                    self.bump();
                    break;
                }
                Some(c) if is_name_start(c) && spaced => {
                    let attribute = self.name()?;
                    self.skip_whitespace();
                    if self.peek() != Some('=') {
                        return self.error(format!("The '{attribute}' attribute has no value."));
                    }
                    self.bump();
                    self.skip_whitespace();
                    let value = self.attribute_value()?;
                    if attributes.iter().any(|(n, _)| *n == attribute) {
                        return self.error(format!("'{attribute}' is a duplicate attribute name."));
                    }
                    attributes.push((attribute, value));
                }
                Some(c) => return self.error(format!("'{c}' is an unexpected token in the '{name}' start tag.")),
                None => return self.error("Unexpected end of file has occurred."),
            }
        }
        let element = self.document.push(
            Kind::Element {
                name: name.clone(),
                attributes,
            },
            line,
        );
        loop {
            if self.starts_with("</") {
                let close_line = self.line;
                self.advance(2);
                let close = self.name()?;
                self.skip_whitespace();
                if self.peek() != Some('>') {
                    return self.error(format!("The end tag of '{close}' is not closed."));
                }
                self.bump();
                if close != name {
                    return Err(XmlError {
                        message: format!(
                            "The '{name}' start tag on line {line} does not match the end tag of '{close}'."
                        ),
                        line: close_line,
                    });
                }
                return Ok(element);
            }
            let child = if self.starts_with("<!--") {
                self.comment()?
            } else if self.starts_with("<![CDATA[") {
                let line = self.line;
                self.advance(9);
                let Some(end) = self.rest().find("]]>") else {
                    return self.error("Unexpected end of file while parsing CDATA has occurred.");
                };
                let value = self.rest()[..end].replace("\r\n", "\n").replace('\r', "\n");
                let length = self.rest()[..end + 3].chars().count();
                self.advance(length);
                self.document.push(Kind::Text(value), line)
            } else if self.starts_with("<?") {
                self.instruction()?
            } else if self.starts_with("<") {
                self.element()?
            } else if self.peek().is_none() {
                return self.error(format!(
                    "Unexpected end of file has occurred. The following elements are not closed: {name}."
                ));
            } else {
                self.text_run()?
            };
            self.document.append(element, child);
        }
    }

    fn reference(&mut self) -> Result<char, XmlError> {
        self.bump();
        let Some(end) = self.rest().find(';') else {
            return self.error("An entity reference is not closed.");
        };
        let name = self.rest()[..end].to_owned();
        let c = match name.as_str() {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            _ if name.starts_with("#x") => u32::from_str_radix(&name[2..], 16)
                .ok()
                .and_then(char::from_u32)
                .map_or_else(|| self.error(format!("Invalid character reference '&{name};'.")), Ok)?,
            _ if name.starts_with('#') => name[1..]
                .parse::<u32>()
                .ok()
                .and_then(char::from_u32)
                .map_or_else(|| self.error(format!("Invalid character reference '&{name};'.")), Ok)?,
            _ => return self.error(format!("Reference to undeclared entity '{name}'.")),
        };
        let length = name.chars().count() + 1;
        self.advance(length);
        Ok(c)
    }

    fn attribute_value(&mut self) -> Result<String, XmlError> {
        let quote = match self.peek() {
            Some(q @ ('"' | '\'')) => q,
            _ => return self.error("An attribute value must be quoted."),
        };
        self.bump();
        let mut value = String::new();
        loop {
            match self.peek() {
                None => return self.error("Unexpected end of file while parsing an attribute value."),
                Some(c) if c == quote => {
                    self.bump();
                    return Ok(value);
                }
                Some('<') => return self.error("'<', hexadecimal value 0x3C, is an invalid attribute character."),
                Some('&') => value.push(self.reference()?),
                Some(c) => {
                    self.bump();
                    value.push(if is_xml_whitespace(c) { ' ' } else { c });
                }
            }
        }
    }

    fn text_run(&mut self) -> Result<NodeId, XmlError> {
        let line = self.line;
        let mut value = String::new();
        while let Some(c) = self.peek() {
            match c {
                '<' => break,
                '&' => value.push(self.reference()?),
                '\r' => {
                    self.bump();
                    value.push('\n');
                }
                c => {
                    self.bump();
                    value.push(c);
                }
            }
        }
        Ok(self.document.push(Kind::Text(value), line))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(text: &str) -> String {
        XmlDocument::parse(text).expect("well-formed").write()
    }

    #[test]
    fn a_document_is_written_back_as_it_was_read() {
        let text = "<map version=\"1\">\n<node TEXT=\"a &amp; b\" ID=\"x\"/>\n<!-- note -->\n</map>";
        assert_eq!(round_trip(text), text);
    }

    #[test]
    fn a_single_quoted_attribute_is_written_double_quoted() {
        assert_eq!(round_trip("<a b='c'/>"), "<a b=\"c\"/>");
    }

    #[test]
    fn an_empty_pair_of_tags_is_written_self_closing() {
        assert_eq!(round_trip("<a><b></b></a>"), "<a><b/></a>");
    }

    #[test]
    fn a_newline_in_an_attribute_reads_as_a_space_and_a_reference_as_itself() {
        assert_eq!(round_trip("<a b=\"x\ny&#xa;z\"/>"), "<a b=\"x y&#xa;z\"/>");
    }

    #[test]
    fn a_mismatched_end_tag_is_not_well_formed_on_its_line() {
        let error = XmlDocument::parse("<map>\n<node>\n</map>").unwrap_err();
        assert_eq!(error.line, 3);
    }

    #[test]
    fn two_root_elements_are_not_well_formed() {
        assert!(XmlDocument::parse("<a/><b/>").is_err());
    }
}
