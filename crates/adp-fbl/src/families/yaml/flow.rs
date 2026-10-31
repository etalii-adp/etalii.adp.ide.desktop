//! Reads a YAML flow collection (`[a, b]`, `{k: v}`) into the values CEL sees (FBL section 4.3; the standalone
//! host's `Files/Yaml/FlowReader`): its members are readable, while its only writable span is the whole collection.
//! One pass gives three readings of the same shape: each member's typed value, its written text (FBL section
//! 4.1.5) and the name of the style it is written in (FBL section 2.4).

use adp_cel::{Map, Value};

use super::scalars;

/// A member of a flow collection, or the collection itself, read three ways: typed, as written, and by style.
#[derive(Debug, Clone)]
pub(crate) struct FlowValue {
    pub typed: Value,
    pub text: Value,
    pub style: Value,
}

impl FlowValue {
    fn empty() -> Self {
        FlowValue {
            typed: Value::Null,
            text: Value::from(""),
            style: Value::from("plain"),
        }
    }
}

pub(crate) fn read(text: &str) -> FlowValue {
    let mut reader = FlowReader {
        text: text.chars().collect(),
        position: 0,
    };
    reader.value()
}

struct FlowReader {
    text: Vec<char>,
    position: usize,
}

impl FlowReader {
    fn at(&self) -> Option<char> {
        self.text.get(self.position).copied()
    }

    fn skip(&mut self) {
        while let Some(c) = self.at() {
            if matches!(c, ' ' | '\t' | '\r' | '\n') {
                self.position += 1;
            } else if c == '#' && (self.position == 0 || matches!(self.text[self.position - 1], ' ' | '\t' | '\n')) {
                while self.at().is_some_and(|c| c != '\n') {
                    self.position += 1;
                }
            } else {
                break;
            }
        }
    }

    fn value(&mut self) -> FlowValue {
        self.skip();
        match self.at() {
            None => FlowValue::empty(),
            Some('[') => {
                self.position += 1;
                let (mut typed, mut texts, mut styles) = (Vec::new(), Vec::new(), Vec::new());
                loop {
                    self.skip();
                    if self.at().is_none_or(|c| c == ']') {
                        if self.at().is_some() {
                            self.position += 1;
                        }
                        return FlowValue {
                            typed: Value::list(typed),
                            text: Value::list(texts),
                            style: Value::list(styles),
                        };
                    }
                    let at = self.position;
                    let item = self.value();
                    // A value that takes nothing (a stray `:`) is stepped over, so that the walk ends.
                    if self.position == at {
                        self.position += 1;
                    }
                    typed.push(item.typed);
                    texts.push(item.text);
                    styles.push(item.style);
                    self.skip();
                    if self.at() == Some(',') {
                        self.position += 1;
                    }
                }
            }
            Some('{') => {
                self.position += 1;
                let (mut typed, mut texts, mut styles) = (Map::default(), Map::default(), Map::default());
                loop {
                    self.skip();
                    if self.at().is_none_or(|c| c == '}') {
                        if self.at().is_some() {
                            self.position += 1;
                        }
                        return FlowValue {
                            typed: Value::map(typed),
                            text: Value::map(texts),
                            style: Value::map(styles),
                        };
                    }
                    let at = self.position;
                    let key = self.scalar(true);
                    if self.position == at && self.at() != Some(':') {
                        self.position += 1;
                    }
                    self.skip();
                    let mut value = FlowValue::empty();
                    if self.at() == Some(':') {
                        self.position += 1;
                        value = self.value();
                    }
                    let name = crate::planning::new_text::dotnet_string(&key.typed);
                    typed.insert(name.clone(), value.typed);
                    texts.insert(key.text.as_str().unwrap_or_default().to_owned(), value.text);
                    styles.insert(name, value.style);
                    self.skip();
                    if self.at() == Some(',') {
                        self.position += 1;
                    }
                }
            }
            Some(_) => self.scalar(false),
        }
    }

    fn scalar(&mut self, key: bool) -> FlowValue {
        let c = self.text[self.position];
        if c == '"' || c == '\'' {
            self.position += 1;
            let start = self.position;
            while self.position < self.text.len() {
                if c == '"' && self.text[self.position] == '\\' {
                    self.position += 2;
                    continue;
                }
                if self.text[self.position] == c {
                    if c == '\'' && self.text.get(self.position + 1) == Some(&'\'') {
                        self.position += 2;
                        continue;
                    }
                    break;
                }
                self.position += 1;
            }
            let end = self.position.min(self.text.len());
            let inner: String = self.text[start..end].iter().collect();
            self.position += 1;
            let decoded = if c == '"' {
                scalars::decode_double(&inner)
            } else {
                scalars::decode_single(&inner)
            };
            return FlowValue {
                typed: Value::from(decoded.as_str()),
                text: Value::from(decoded),
                style: Value::from(if c == '"' { "double" } else { "single" }),
            };
        }
        let begin = self.position;
        while self.position < self.text.len() {
            let d = self.text[self.position];
            if matches!(d, ',' | ']' | '}') {
                break;
            }
            if d == ':'
                && (key
                    || self.position + 1 >= self.text.len()
                    || matches!(self.text[self.position + 1], ' ' | ',' | ']' | '}'))
            {
                break;
            }
            self.position += 1;
        }
        let raw: String = self.text[begin..self.position].iter().collect();
        let plain = scalars::fold(raw.trim());
        FlowValue {
            typed: scalars::typed(&plain),
            text: Value::from(plain),
            style: Value::from("plain"),
        }
    }
}
