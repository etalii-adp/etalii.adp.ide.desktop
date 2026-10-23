//! ATD grammar notation (ATD §4): reads one module, a `.lark` file, into its definitions, imports and ignores, and
//! refuses what the subset of §4.1 does not admit.

use std::fmt;

/// An item of an expansion.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Item {
    /// A rule or a terminal, by name.
    Symbol(String),
    /// An anonymous string terminal, its value unescaped as Lark does.
    Str(String),
    /// An anonymous regular expression terminal.
    Re(String),
    /// `( … )`.
    Group(Vec<Alternative>),
    /// `[ … ]`.
    Maybe(Vec<Alternative>),
    /// `x?`.
    Optional(Box<Item>),
    /// `x*`.
    Star(Box<Item>),
    /// `x+`.
    Plus(Box<Item>),
}

/// One alternative of a definition: its items and its alias.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Alternative {
    pub items: Vec<Item>,
    pub alias: Option<String>,
}

/// A rule or a terminal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Definition {
    /// Its name, with a leading `_` kept and a leading `?` taken off.
    pub name: String,
    /// Written `?name`: inlined in the tree when it has one child.
    pub inline: bool,
    pub priority: i32,
    pub alternatives: Vec<Alternative>,
    pub line: usize,
}

impl Definition {
    pub fn is_terminal(&self) -> bool {
        is_terminal_name(&self.name)
    }
}

/// `%import .module (names)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    pub module: String,
    pub names: Vec<String>,
    pub line: usize,
}

/// One module of a grammar.
#[derive(Debug, Clone, Default)]
pub struct Module {
    pub definitions: Vec<Definition>,
    pub imports: Vec<Import>,
    pub ignores: Vec<String>,
}

/// A construct outside ATD grammar notation, or text that is not notation at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotationError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for NotationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for NotationError {}

pub fn is_terminal_name(name: &str) -> bool {
    name.trim_start_matches('_')
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_uppercase())
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Name(String),
    Str(String),
    Re(String),
    Int(i32),
    Directive(String),
    Punct(&'static str),
}

/// Reads the text of one module.
pub fn read_module(text: &str) -> Result<Module, NotationError> {
    let mut module = Module::default();
    for (line, statement) in statements(text) {
        let tokens = tokenize(&statement, line)?;
        if tokens.is_empty() {
            continue;
        }
        let mut parser = Parser {
            tokens,
            position: 0,
            line,
        };
        parser.statement(&mut module)?;
    }
    Ok(module)
}

/// The statements of a module, each with its first line: a statement starts on a line that starts with neither
/// whitespace nor `|`, and runs to the next such line.
fn statements(text: &str) -> Vec<(usize, String)> {
    let mut out: Vec<(usize, String)> = Vec::new();
    for (index, raw) in text.split('\n').enumerate() {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        let content = strip_comment(line);
        if content.trim().is_empty() {
            continue;
        }
        let starts = !line.starts_with([' ', '\t', '|']);
        if starts || out.is_empty() {
            out.push((index + 1, content.to_owned()));
        } else if let Some(last) = out.last_mut() {
            last.1.push('\n');
            last.1.push_str(content);
        }
    }
    out
}

/// The line without its `//` comment, which never starts inside a string or a regular expression.
fn strip_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => i = skip_quoted(bytes, i, b'"'),
            b'/' if bytes.get(i + 1) == Some(&b'/') => return &line[..i],
            b'/' if regexp_can_start(bytes, i) => i = skip_quoted(bytes, i, b'/'),
            _ => i += 1,
        }
    }
    line
}

fn regexp_can_start(bytes: &[u8], i: usize) -> bool {
    !matches!(bytes.get(i + 1), Some(b'/') | Some(b'*') | None)
}

/// The index after the literal that starts at `start` with `quote`.
fn skip_quoted(bytes: &[u8], start: usize, quote: u8) -> usize {
    let mut i = start + 1;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] == quote {
            i += 1;
            while quote == b'/' && i < bytes.len() && bytes[i].is_ascii_lowercase() {
                i += 1;
            }
            return i;
        }
        i += 1;
    }
    bytes.len()
}

fn tokenize(text: &str, first_line: usize) -> Result<Vec<Token>, NotationError> {
    let bytes = text.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    let mut line = first_line;
    let error = |line: usize, message: String| NotationError { line, message };
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'\n' {
            line += 1;
            i += 1;
            continue;
        }
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if c == b'"' {
            let end = skip_quoted(bytes, i, b'"');
            let raw = &text[i + 1..end - 1];
            if end > bytes.len() || !text[..end].ends_with('"') {
                return Err(error(line, "a string that does not end".into()));
            }
            if text[end..].starts_with(|c: char| c.is_ascii_lowercase()) {
                return Err(error(
                    line,
                    "a string with flags, which ATD grammar notation does not admit".into(),
                ));
            }
            tokens.push(Token::Str(string_value(raw, line)?));
            i = end;
            continue;
        }
        if c == b'/' {
            let end = skip_quoted(bytes, i, b'/');
            let literal = &text[i..end];
            let close = literal.rfind('/').unwrap_or(0);
            if close == 0 {
                return Err(error(line, "a regular expression that does not end".into()));
            }
            if close + 1 != literal.len() {
                return Err(error(
                    line,
                    format!(
                        "the flags `{}` of a regular expression, which ATD grammar notation does not admit",
                        &literal[close + 1..]
                    ),
                ));
            }
            tokens.push(Token::Re(eval_escaping(&literal[1..close], line)?));
            i = end;
            continue;
        }
        if c == b'%' {
            let end = i
                + 1
                + text[i + 1..]
                    .find(|c: char| !c.is_ascii_alphabetic())
                    .unwrap_or(text.len() - i - 1);
            tokens.push(Token::Directive(text[i..end].to_owned()));
            i = end;
            continue;
        }
        if c.is_ascii_digit() {
            let end = i + text[i..].find(|c: char| !c.is_ascii_digit()).unwrap_or(text.len() - i);
            let value = text[i..end]
                .parse()
                .map_err(|_| error(line, "a number too large".into()))?;
            tokens.push(Token::Int(value));
            i = end;
            continue;
        }
        if c.is_ascii_alphabetic() || c == b'_' || ((c == b'?' || c == b'!') && tokens.is_empty()) {
            let start = i;
            i += 1;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            tokens.push(Token::Name(text[start..i].to_owned()));
            continue;
        }
        let punct: &'static str = match (c, bytes.get(i + 1)) {
            (b'-', Some(b'>')) => "->",
            (b'.', Some(b'.')) => "..",
            (b':', _) => ":",
            (b'|', _) => "|",
            (b'(', _) => "(",
            (b')', _) => ")",
            (b'[', _) => "[",
            (b']', _) => "]",
            (b'?', _) => "?",
            (b'*', _) => "*",
            (b'+', _) => "+",
            (b'.', _) => ".",
            (b',', _) => ",",
            (b'~', _) => "~",
            (b'{', _) => "{",
            (b'}', _) => "}",
            _ => return Err(error(line, format!("`{}` is not ATD grammar notation", c as char))),
        };
        i += punct.len();
        tokens.push(Token::Punct(punct));
    }
    Ok(tokens)
}

/// A string literal's value, as Lark reads it: escapes evaluated, then `\\` made one backslash.
fn string_value(raw: &str, line: usize) -> Result<String, NotationError> {
    Ok(eval_escaping(raw, line)?.replace("\\\\", "\\"))
}

/// Lark's `eval_escaping`: `\n`, `\t`, `\r`, `\f`, `\xHH`, `\uHHHH` and `\UHHHHHHHH` become their characters,
/// `\"` a quote, and every other escape is kept as written, `\\` included.
pub fn eval_escaping(raw: &str, line: usize) -> Result<String, NotationError> {
    let chars: Vec<char> = raw.chars().collect();
    let mut out = String::with_capacity(raw.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c != '\\' {
            out.push(c);
            i += 1;
            continue;
        }
        let Some(&next) = chars.get(i + 1) else {
            return Err(NotationError {
                line,
                message: "a literal that ends with a lone backslash".into(),
            });
        };
        i += 2;
        match next {
            'n' => out.push('\n'),
            't' => out.push('\t'),
            'r' => out.push('\r'),
            'f' => out.push('\u{c}'),
            '"' => out.push('"'),
            'x' | 'u' | 'U' => {
                let digits = match next {
                    'x' => 2,
                    'u' => 4,
                    _ => 8,
                };
                let hex: String = chars.iter().skip(i).take(digits).collect();
                let code = u32::from_str_radix(&hex, 16)
                    .ok()
                    .and_then(char::from_u32)
                    .ok_or(NotationError {
                        line,
                        message: format!("the escape `\\{next}{hex}` is not a character"),
                    })?;
                out.push(code);
                i += digits;
            }
            other => {
                out.push('\\');
                out.push(other);
            }
        }
    }
    Ok(out)
}

struct Parser {
    tokens: Vec<Token>,
    position: usize,
    line: usize,
}

impl Parser {
    fn error(&self, message: impl Into<String>) -> NotationError {
        NotationError {
            line: self.line,
            message: message.into(),
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    fn next(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.position).cloned();
        self.position += 1;
        token
    }

    fn eat(&mut self, punct: &str) -> bool {
        if matches!(self.peek(), Some(Token::Punct(p)) if *p == punct) {
            self.position += 1;
            return true;
        }
        false
    }

    fn expect(&mut self, punct: &str) -> Result<(), NotationError> {
        if self.eat(punct) {
            Ok(())
        } else {
            Err(self.error(format!("`{punct}` expected")))
        }
    }

    fn name(&mut self) -> Result<String, NotationError> {
        match self.next() {
            Some(Token::Name(name)) => Ok(name),
            _ => Err(self.error("a name expected")),
        }
    }

    fn statement(&mut self, module: &mut Module) -> Result<(), NotationError> {
        match self.next() {
            Some(Token::Directive(directive)) => match directive.as_str() {
                "%ignore" => {
                    let name = self.name()?;
                    module.ignores.push(name);
                }
                "%import" => {
                    self.expect(".")?;
                    let mut path = vec![self.name()?];
                    while self.eat(".") {
                        path.push(self.name()?);
                    }
                    if path.len() != 1 {
                        return Err(self.error("an import from outside the language's modules"));
                    }
                    self.expect("(")?;
                    let mut names = vec![self.name()?];
                    while self.eat(",") {
                        names.push(self.name()?);
                    }
                    self.expect(")")?;
                    module.imports.push(Import {
                        module: path.remove(0),
                        names,
                        line: self.line,
                    });
                }
                other => {
                    return Err(self.error(format!("`{other}` is not admitted (only `%import` and `%ignore` are)")));
                }
            },
            Some(Token::Name(head)) => {
                let (inline, name) = match head.strip_prefix('?') {
                    Some(rest) => (true, rest.to_owned()),
                    None if head.starts_with('!') => {
                        return Err(self.error(format!("the keep-all-tokens rule `{head}` is not admitted")));
                    }
                    None => (false, head),
                };
                let mut priority = 0;
                if self.eat(".") {
                    match self.next() {
                        Some(Token::Int(value)) if is_terminal_name(&name) => priority = value,
                        Some(Token::Int(_)) => {
                            return Err(self.error(format!("the rule priority of `{name}` is not admitted")));
                        }
                        _ => return Err(self.error("a priority expected")),
                    }
                }
                if matches!(self.peek(), Some(Token::Punct("{"))) {
                    return Err(self.error(format!("the template `{name}{{…}}` is not admitted")));
                }
                self.expect(":")?;
                let alternatives = self.alternatives()?;
                if self.position < self.tokens.len() {
                    return Err(self.error("text after the definition"));
                }
                module.definitions.push(Definition {
                    name,
                    inline,
                    priority,
                    alternatives,
                    line: self.line,
                });
            }
            _ => return Err(self.error("a definition or a directive expected")),
        }
        if self.position < self.tokens.len() {
            return Err(self.error("text after the statement"));
        }
        Ok(())
    }

    fn alternatives(&mut self) -> Result<Vec<Alternative>, NotationError> {
        let mut alternatives = vec![self.alternative()?];
        while self.eat("|") {
            alternatives.push(self.alternative()?);
        }
        Ok(alternatives)
    }

    fn alternative(&mut self) -> Result<Alternative, NotationError> {
        let mut items = Vec::new();
        while let Some(Token::Name(_) | Token::Str(_) | Token::Re(_) | Token::Punct("(" | "[")) = self.peek() {
            items.push(self.item()?);
        }
        let alias = if self.eat("->") { Some(self.name()?) } else { None };
        Ok(Alternative { items, alias })
    }

    fn item(&mut self) -> Result<Item, NotationError> {
        let atom = match self.next() {
            Some(Token::Name(name)) => Item::Symbol(name),
            Some(Token::Str(value)) => {
                if self.eat("..") {
                    return Err(self.error("the range `..` is not admitted"));
                }
                Item::Str(value)
            }
            Some(Token::Re(value)) => Item::Re(value),
            Some(Token::Punct("(")) => {
                let inner = self.alternatives()?;
                self.expect(")")?;
                Item::Group(inner)
            }
            Some(Token::Punct("[")) => {
                let inner = self.alternatives()?;
                self.expect("]")?;
                Item::Maybe(inner)
            }
            _ => return Err(self.error("an item expected")),
        };
        Ok(match self.peek() {
            Some(Token::Punct("?")) => {
                self.position += 1;
                Item::Optional(Box::new(atom))
            }
            Some(Token::Punct("*")) => {
                self.position += 1;
                Item::Star(Box::new(atom))
            }
            Some(Token::Punct("+")) => {
                self.position += 1;
                Item::Plus(Box::new(atom))
            }
            Some(Token::Punct("~")) => return Err(self.error("the repetition `~` is not admitted")),
            _ => atom,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_rules_terminals_imports_and_ignores() {
        let module = read_module(
            "// a comment\n?start: a b* [c] -> s\n     | \"x\" (d | e)+\nA.2: /[a-z]+/\n%import .core (a, b)\n%ignore WS\n",
        )
        .unwrap();
        assert_eq!(module.definitions.len(), 2);
        assert!(module.definitions[0].inline);
        assert_eq!(module.definitions[0].alternatives.len(), 2);
        assert_eq!(module.definitions[0].alternatives[0].alias.as_deref(), Some("s"));
        assert_eq!(module.definitions[1].priority, 2);
        assert_eq!(module.imports[0].names, vec!["a", "b"]);
        assert_eq!(module.ignores, vec!["WS"]);
    }

    #[test]
    fn escapes_are_read_as_lark_reads_them() {
        assert_eq!(eval_escaping(r"[ \t]\.\\", 1).unwrap(), "[ \t]\\.\\\\");
        assert_eq!(string_value(r"\\", 1).unwrap(), "\\");
    }

    #[test]
    fn constructs_outside_the_subset_are_refused() {
        for (text, expected) in [
            ("a.2: b\n", "rule priority"),
            ("!a: b\n", "keep-all-tokens"),
            ("a: \"x\"i\n", "flags"),
            ("A: /x/i\n", "flags"),
            ("%declare A\n", "not admitted"),
            ("%import common.WS\n", "`.` expected"),
            ("a: \"a\"..\"z\"\n", "range"),
            ("a: b ~ 3\n", "repetition"),
            ("a{x}: x\n", "template"),
        ] {
            let error = read_module(text).unwrap_err();
            assert!(error.message.contains(expected), "{text}: {error}");
        }
    }
}
