//! A tolerant JSON reader that keeps where everything is: every value's byte span and every key's, the mistakes of
//! JSON itself (a syntax error, a comment, a trailing comma) and a repeated property name. A mistake never stops the
//! reading; what can be read is read, so a schema can still be checked around it.

use std::ops::Range;

use serde_json::{Map, Number, Value};

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Object { span: Range<usize>, members: Vec<Member> },
    Array { span: Range<usize>, items: Vec<Node> },
    String { span: Range<usize>, value: String },
    Number { span: Range<usize>, text: String },
    Bool { span: Range<usize>, value: bool },
    Null { span: Range<usize> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Member {
    pub key: String,
    pub key_span: Range<usize>,
    /// `None` when the member has a key but no value yet (`"a":` or `"a"`).
    pub value: Option<Node>,
}

impl Node {
    pub fn span(&self) -> Range<usize> {
        match self {
            Node::Object { span, .. }
            | Node::Array { span, .. }
            | Node::String { span, .. }
            | Node::Number { span, .. }
            | Node::Bool { span, .. }
            | Node::Null { span } => span.clone(),
        }
    }

    /// The value as serde_json reads it, for checking against a schema; members without a value are left out.
    pub fn to_value(&self) -> Value {
        match self {
            Node::Object { members, .. } => Value::Object(
                members
                    .iter()
                    .filter_map(|m| m.value.as_ref().map(|v| (m.key.clone(), v.to_value())))
                    .collect::<Map<_, _>>(),
            ),
            Node::Array { items, .. } => Value::Array(items.iter().map(Node::to_value).collect()),
            Node::String { value, .. } => Value::String(value.clone()),
            Node::Number { text, .. } => text.parse::<Number>().map(Value::Number).unwrap_or(Value::Null),
            Node::Bool { value, .. } => Value::Bool(*value),
            Node::Null { .. } => Value::Null,
        }
    }

    /// The node a JSON pointer (`/properties/0/name`) leads to, with the key's span when the last step is a key.
    pub fn at_pointer(&self, pointer: &str) -> Option<(&Node, Option<Range<usize>>)> {
        let mut node = self;
        let mut key_span = None;
        for raw in pointer.split('/').skip(1) {
            let step = raw.replace("~1", "/").replace("~0", "~");
            match node {
                Node::Object { members, .. } => {
                    let member = members.iter().rev().find(|m| m.key == step)?;
                    key_span = Some(member.key_span.clone());
                    node = member.value.as_ref()?;
                }
                Node::Array { items, .. } => {
                    node = items.get(step.parse::<usize>().ok()?)?;
                    key_span = None;
                }
                _ => return None,
            }
        }
        Some((node, key_span))
    }
}

/// What reading found wrong, by byte span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mistake {
    pub span: Range<usize>,
    pub message: String,
    /// A repeated property name is a warning; everything else is an error.
    pub warning: bool,
}

pub struct Parsed {
    pub root: Option<Node>,
    pub mistakes: Vec<Mistake>,
}

pub fn parse(text: &str) -> Parsed {
    let mut reader = Reader {
        text,
        bytes: text.as_bytes(),
        at: 0,
        mistakes: Vec::new(),
    };
    reader.skip_space();
    let root = if reader.at < reader.bytes.len() {
        reader.value()
    } else {
        None
    };
    reader.skip_space();
    if reader.at < reader.bytes.len() {
        let start = reader.at;
        reader.mistake(start..reader.bytes.len(), "End of file expected.");
    }
    Parsed {
        root,
        mistakes: reader.mistakes,
    }
}

struct Reader<'t> {
    text: &'t str,
    bytes: &'t [u8],
    at: usize,
    mistakes: Vec<Mistake>,
}

impl Reader<'_> {
    fn mistake(&mut self, span: Range<usize>, message: &str) {
        let end = span.end.max(span.start + 1).min(self.bytes.len().max(span.start));
        self.mistakes.push(Mistake {
            span: span.start..end,
            message: message.into(),
            warning: false,
        });
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    fn skip_space(&mut self) {
        loop {
            while self.peek().is_some_and(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\n')) {
                self.at += 1;
            }
            let start = self.at;
            if self.bytes[self.at..].starts_with(b"//") {
                while self.peek().is_some_and(|b| b != b'\n') {
                    self.at += 1;
                }
                self.mistake(start..self.at, "Comments are not permitted in JSON.");
            } else if self.bytes[self.at..].starts_with(b"/*") {
                match self.text[self.at + 2..].find("*/") {
                    Some(end) => self.at += end + 4,
                    None => self.at = self.bytes.len(),
                }
                self.mistake(start..self.at, "Comments are not permitted in JSON.");
            } else {
                return;
            }
        }
    }

    fn value(&mut self) -> Option<Node> {
        self.skip_space();
        let start = self.at;
        match self.peek()? {
            b'{' => Some(self.object()),
            b'[' => Some(self.array()),
            b'"' => {
                let (value, span) = self.string();
                Some(Node::String { span, value })
            }
            b'-' | b'0'..=b'9' => Some(self.number()),
            b't' | b'f' | b'n' => {
                for (word, node) in [
                    (
                        "true",
                        Node::Bool {
                            span: start..start + 4,
                            value: true,
                        },
                    ),
                    (
                        "false",
                        Node::Bool {
                            span: start..start + 5,
                            value: false,
                        },
                    ),
                    ("null", Node::Null { span: start..start + 4 }),
                ] {
                    if self.text[start..].starts_with(word) {
                        self.at += word.len();
                        return Some(node);
                    }
                }
                self.unexpected()
            }
            _ => self.unexpected(),
        }
    }

    fn unexpected(&mut self) -> Option<Node> {
        let start = self.at;
        while self
            .peek()
            .is_some_and(|b| !matches!(b, b',' | b'}' | b']' | b'\n' | b':'))
        {
            self.at += 1;
        }
        if self.at == start {
            self.at += 1;
        }
        self.mistake(start..self.at, "Value expected.");
        None
    }

    fn object(&mut self) -> Node {
        let start = self.at;
        self.at += 1;
        let mut members: Vec<Member> = Vec::new();
        loop {
            self.skip_space();
            match self.peek() {
                None => {
                    self.mistake(start..start + 1, "Expected '}' to close this object.");
                    break;
                }
                Some(b'}') => {
                    self.at += 1;
                    break;
                }
                Some(b'"') => {
                    let (key, key_span) = self.string();
                    if members.iter().any(|m| m.key == key) {
                        self.mistakes.push(Mistake {
                            span: key_span.clone(),
                            message: "Duplicate object key.".into(),
                            warning: true,
                        });
                    }
                    self.skip_space();
                    let value = if self.peek() == Some(b':') {
                        self.at += 1;
                        self.skip_space();
                        if matches!(self.peek(), Some(b',' | b'}') | None) {
                            let at = self.at;
                            self.mistake(at..at + 1, "Value expected.");
                            None
                        } else {
                            self.value()
                        }
                    } else {
                        self.mistake(key_span.clone(), "Colon expected.");
                        None
                    };
                    members.push(Member { key, key_span, value });
                    self.skip_space();
                    match self.peek() {
                        Some(b',') => {
                            let comma = self.at;
                            self.at += 1;
                            self.skip_space();
                            if self.peek() == Some(b'}') {
                                self.mistake(comma..comma + 1, "Trailing comma.");
                            }
                        }
                        Some(b'}') | None => {}
                        Some(_) => {
                            let at = self.at;
                            self.mistake(at..at + 1, "Expected comma.");
                            if !matches!(self.peek(), Some(b'"')) {
                                self.unexpected();
                            }
                        }
                    }
                }
                Some(_) => {
                    let at = self.at;
                    self.mistake(at..at + 1, "Property expected.");
                    self.unexpected();
                    if self.peek() == Some(b',') {
                        self.at += 1;
                    }
                }
            }
        }
        Node::Object {
            span: start..self.at,
            members,
        }
    }

    fn array(&mut self) -> Node {
        let start = self.at;
        self.at += 1;
        let mut items = Vec::new();
        loop {
            self.skip_space();
            match self.peek() {
                None => {
                    self.mistake(start..start + 1, "Expected ']' to close this array.");
                    break;
                }
                Some(b']') => {
                    self.at += 1;
                    break;
                }
                _ => {
                    if let Some(item) = self.value() {
                        items.push(item);
                    }
                    self.skip_space();
                    match self.peek() {
                        Some(b',') => {
                            let comma = self.at;
                            self.at += 1;
                            self.skip_space();
                            if self.peek() == Some(b']') {
                                self.mistake(comma..comma + 1, "Trailing comma.");
                            }
                        }
                        Some(b']') | None => {}
                        Some(_) => {
                            let at = self.at;
                            self.mistake(at..at + 1, "Expected comma.");
                        }
                    }
                }
            }
        }
        Node::Array {
            span: start..self.at,
            items,
        }
    }

    fn string(&mut self) -> (String, Range<usize>) {
        let start = self.at;
        self.at += 1;
        let mut value = String::new();
        loop {
            match self.peek() {
                None | Some(b'\n') => {
                    self.mistake(start..self.at, "Unexpected end of string.");
                    break;
                }
                Some(b'"') => {
                    self.at += 1;
                    break;
                }
                Some(b'\\') => {
                    let escape_at = self.at;
                    self.at += 1;
                    match self.peek() {
                        Some(b'"') => value.push('"'),
                        Some(b'\\') => value.push('\\'),
                        Some(b'/') => value.push('/'),
                        Some(b'b') => value.push('\u{8}'),
                        Some(b'f') => value.push('\u{c}'),
                        Some(b'n') => value.push('\n'),
                        Some(b'r') => value.push('\r'),
                        Some(b't') => value.push('\t'),
                        Some(b'u') => {
                            let hex = self.text.get(self.at + 1..self.at + 5).unwrap_or("");
                            match u32::from_str_radix(hex, 16).ok().and_then(char::from_u32) {
                                Some(c) => {
                                    value.push(c);
                                    self.at += 4;
                                }
                                None => self.mistake(escape_at..self.at + 1, "Invalid unicode sequence in string."),
                            }
                        }
                        _ => self.mistake(escape_at..self.at + 1, "Invalid escape character in string."),
                    }
                    self.at += 1;
                }
                Some(_) => {
                    let c = self.text[self.at..].chars().next().unwrap_or('\0');
                    value.push(c);
                    self.at += c.len_utf8();
                }
            }
        }
        (value, start..self.at)
    }

    fn number(&mut self) -> Node {
        let start = self.at;
        while self
            .peek()
            .is_some_and(|b| matches!(b, b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9'))
        {
            self.at += 1;
        }
        let text = self.text[start..self.at].to_owned();
        if text.parse::<Number>().is_err() {
            self.mistake(start..self.at, "Invalid number format.");
        }
        Node::Number {
            span: start..self.at,
            text,
        }
    }
}
