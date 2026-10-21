//! Parses CEL against an environment (the standalone host's `CelParser`): a call resolves to one of its functions
//! and a comprehension to one of its macros while parsing, so anything the environment does not offer is refused
//! here, naming it.

use crate::Environment;
use crate::eval::{Node, arity, is_macro};
use crate::value::Value;

#[derive(Debug, Clone, PartialEq)]
enum Kind {
    Int,
    Double,
    Str,
    Ident,
    Op,
}

pub(crate) fn parse(source: &str, environment: &Environment) -> Result<Node, String> {
    let mut parser = Parser {
        source,
        environment,
        tokens: tokenize(source)?,
        position: 0,
    };
    let node = parser.expression()?;
    if parser.position < parser.tokens.len() {
        return Err(format!(
            "Unexpected '{}' in '{source}'.",
            parser.tokens[parser.position].1
        ));
    }
    Ok(node)
}

struct Parser<'a> {
    source: &'a str,
    environment: &'a Environment,
    tokens: Vec<(Kind, String)>,
    position: usize,
}

impl Parser<'_> {
    fn expression(&mut self) -> Result<Node, String> {
        let condition = self.or()?;
        if self.peek("?") {
            self.position += 1;
            let then = self.or()?;
            self.expect(":")?;
            let otherwise = self.expression()?;
            return Ok(Node::Conditional(
                Box::new(condition),
                Box::new(then),
                Box::new(otherwise),
            ));
        }
        Ok(condition)
    }

    fn or(&mut self) -> Result<Node, String> {
        let mut left = self.and()?;
        while self.peek("||") {
            self.position += 1;
            left = Node::Binary("||", Box::new(left), Box::new(self.and()?));
        }
        Ok(left)
    }

    fn and(&mut self) -> Result<Node, String> {
        let mut left = self.relation()?;
        while self.peek("&&") {
            self.position += 1;
            left = Node::Binary("&&", Box::new(left), Box::new(self.relation()?));
        }
        Ok(left)
    }

    fn relation(&mut self) -> Result<Node, String> {
        let mut left = self.addition()?;
        while let Some((Kind::Op, text)) = self.tokens.get(self.position) {
            let op = match text.as_str() {
                "==" => "==",
                "!=" => "!=",
                "<" => "<",
                "<=" => "<=",
                ">" => ">",
                ">=" => ">=",
                "in" => "in",
                _ => break,
            };
            self.position += 1;
            left = Node::Binary(op, Box::new(left), Box::new(self.addition()?));
        }
        Ok(left)
    }

    fn addition(&mut self) -> Result<Node, String> {
        let mut left = self.multiplication()?;
        loop {
            let op = if self.peek("+") {
                "+"
            } else if self.peek("-") {
                "-"
            } else {
                break;
            };
            self.position += 1;
            left = Node::Binary(op, Box::new(left), Box::new(self.multiplication()?));
        }
        Ok(left)
    }

    fn multiplication(&mut self) -> Result<Node, String> {
        let mut left = self.unary()?;
        loop {
            let op = if self.peek("*") {
                "*"
            } else if self.peek("/") {
                "/"
            } else if self.peek("%") {
                "%"
            } else {
                break;
            };
            self.position += 1;
            left = Node::Binary(op, Box::new(left), Box::new(self.unary()?));
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Node, String> {
        if self.peek("!") || self.peek("-") {
            let op = if self.peek("!") { "!" } else { "-" };
            self.position += 1;
            return Ok(Node::Unary(op, Box::new(self.unary()?)));
        }
        let primary = self.primary()?;
        self.postfix(primary)
    }

    fn postfix(&mut self, mut node: Node) -> Result<Node, String> {
        loop {
            if self.peek(".?") || self.peek("[?") {
                return Err(format!(
                    "Optional values are not supported by this CEL evaluator, in '{}'.",
                    self.source
                ));
            }
            if self.peek(".") {
                self.position += 1;
                let name = self.expect_ident()?;
                if self.peek("(") {
                    self.position += 1;
                    if is_macro(&name) {
                        let variable = self.expect_ident()?;
                        self.expect(",")?;
                        let body = self.expression()?;
                        self.expect(")")?;
                        node = Node::Comprehension(name, Box::new(node), variable, Box::new(body));
                        continue;
                    }
                    let arguments = self.arguments()?;
                    node = self.resolve(&name, Some(node), arguments)?;
                    continue;
                }
                node = Node::Member(Box::new(node), name);
                continue;
            }
            if self.peek("[") {
                self.position += 1;
                let key = self.expression()?;
                self.expect("]")?;
                node = Node::Index(Box::new(node), Box::new(key));
                continue;
            }
            return Ok(node);
        }
    }

    fn resolve(&self, name: &str, receiver: Option<Node>, arguments: Vec<Node>) -> Result<Node, String> {
        let Some((min, max)) = arity(name, receiver.is_some()) else {
            return Err(format!("The function '{name}' is not supported by this CEL evaluator."));
        };
        if arguments.len() < min || arguments.len() > max {
            let range = if min == max {
                min.to_string()
            } else {
                format!("{min} to {max}")
            };
            let plural = if min == 1 && max == 1 { "" } else { "s" };
            return Err(format!(
                "The function '{name}' takes {range} argument{plural}, not {}, in '{}'.",
                arguments.len(),
                self.source
            ));
        }
        Ok(Node::Call(name.to_owned(), receiver.map(Box::new), arguments))
    }

    fn arguments(&mut self) -> Result<Vec<Node>, String> {
        let mut arguments = Vec::new();
        if !self.peek(")") {
            arguments.push(self.expression()?);
            while self.peek(",") {
                self.position += 1;
                arguments.push(self.expression()?);
            }
        }
        self.expect(")")?;
        Ok(arguments)
    }

    fn primary(&mut self) -> Result<Node, String> {
        let Some((kind, text)) = self.tokens.get(self.position).cloned() else {
            return Err(format!("'{}' ends too early.", self.source));
        };
        self.position += 1;
        match kind {
            Kind::Int => text
                .parse::<i64>()
                .map(|n| Node::Literal(Value::Int(n)))
                .map_err(|_| format!("The int {text} is out of range, in '{}'.", self.source)),
            Kind::Double => Ok(Node::Literal(Value::Double(text.parse().unwrap_or(0.0)))),
            Kind::Str => Ok(Node::Literal(Value::String(text))),
            Kind::Ident => {
                match text.as_str() {
                    "true" => return Ok(Node::Literal(Value::Bool(true))),
                    "false" => return Ok(Node::Literal(Value::Bool(false))),
                    "null" => return Ok(Node::Literal(Value::Null)),
                    "has" if self.peek("(") => {
                        self.expect("(")?;
                        let primary = self.primary()?;
                        let target = self.postfix(primary)?;
                        self.expect(")")?;
                        return match target {
                            Node::Member(target, name) => Ok(Node::Has(target, name)),
                            _ => Err("has() needs a field selection such as has(entry.end).".into()),
                        };
                    }
                    _ => {}
                }
                if self.peek("(") {
                    self.position += 1;
                    let arguments = self.arguments()?;
                    return self.resolve(&text, None, arguments);
                }
                if let Some(qualified) = self.qualified_call(&text)? {
                    return Ok(qualified);
                }
                Ok(Node::Ident(text))
            }
            Kind::Op => match text.as_str() {
                "(" => {
                    let inner = self.expression()?;
                    self.expect(")")?;
                    Ok(inner)
                }
                "[" => {
                    let mut items = Vec::new();
                    if !self.peek("]") {
                        items.push(self.expression()?);
                        while self.peek(",") {
                            self.position += 1;
                            if self.peek("]") {
                                break;
                            }
                            items.push(self.expression()?);
                        }
                    }
                    self.expect("]")?;
                    Ok(Node::List(items))
                }
                "{" => {
                    let mut entries = Vec::new();
                    if !self.peek("}") {
                        loop {
                            if self.peek(",") {
                                self.position += 1;
                            }
                            if self.peek("}") {
                                break;
                            }
                            let key = self.expression()?;
                            self.expect(":")?;
                            entries.push((key, self.expression()?));
                            if !self.peek(",") {
                                break;
                            }
                        }
                    }
                    self.expect("}")?;
                    Ok(Node::MapLiteral(entries))
                }
                _ => Err(format!("Unexpected '{text}' in '{}'.", self.source)),
            },
        }
    }

    /// `cel.bind(…)`, or a global function with a dotted name such as `lists.range(…)`, when `head` is not a
    /// variable; none when the tokens are an ordinary selection.
    fn qualified_call(&mut self, head: &str) -> Result<Option<Node>, String> {
        let p = self.position;
        if !self.peek(".")
            || p + 2 >= self.tokens.len()
            || self.tokens[p + 1].0 != Kind::Ident
            || self.tokens[p + 2] != (Kind::Op, "(".to_owned())
        {
            return Ok(None);
        }
        if self.environment.is_variable(head) {
            return Ok(None);
        }
        let name = format!("{head}.{}", self.tokens[p + 1].1);
        if name == "cel.bind" {
            self.position += 3;
            let variable = self.expect_ident()?;
            self.expect(",")?;
            let init = self.expression()?;
            self.expect(",")?;
            let body = self.expression()?;
            self.expect(")")?;
            return Ok(Some(Node::Bind(variable, Box::new(init), Box::new(body))));
        }
        if arity(&name, false).is_none() {
            return Ok(None);
        }
        self.position += 3;
        let arguments = self.arguments()?;
        self.resolve(&name, None, arguments).map(Some)
    }

    fn peek(&self, text: &str) -> bool {
        matches!(self.tokens.get(self.position), Some((Kind::Op, t)) if t == text)
    }

    fn expect(&mut self, text: &str) -> Result<(), String> {
        if !self.peek(text) {
            return Err(format!("Expected '{text}' in '{}'.", self.source));
        }
        self.position += 1;
        Ok(())
    }

    fn expect_ident(&mut self) -> Result<String, String> {
        match self.tokens.get(self.position) {
            Some((Kind::Ident, name)) => {
                let name = name.clone();
                self.position += 1;
                Ok(name)
            }
            _ => Err(format!("Expected a name in '{}'.", self.source)),
        }
    }
}

fn tokenize(source: &str) -> Result<Vec<(Kind, String)>, String> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let mut is_double = false;
            if i + 1 < chars.len() && chars[i] == '.' && chars[i + 1].is_ascii_digit() {
                is_double = true;
                i += 1;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            if i < chars.len() && matches!(chars[i], 'e' | 'E') {
                let mut exponent = i + 1;
                if exponent < chars.len() && matches!(chars[exponent], '+' | '-') {
                    exponent += 1;
                }
                if exponent < chars.len() && chars[exponent].is_ascii_digit() {
                    is_double = true;
                    i = exponent;
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                }
            }
            if i < chars.len() && matches!(chars[i], 'u' | 'U') {
                return Err("Unsigned integers are not supported by this CEL evaluator.".into());
            }
            let text: String = chars[start..i].iter().collect();
            tokens.push((if is_double { Kind::Double } else { Kind::Int }, text));
            continue;
        }
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            // 'in' is an operator spelled as a name.
            tokens.push((if text == "in" { Kind::Op } else { Kind::Ident }, text));
            continue;
        }
        if c == '\'' || c == '"' {
            let mut text = String::new();
            i += 1;
            while i < chars.len() && chars[i] != c {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    i += 1;
                    text.push(match chars[i] {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        other => other,
                    });
                } else {
                    text.push(chars[i]);
                }
                i += 1;
            }
            if i >= chars.len() {
                return Err(format!("A string is not closed in '{source}'."));
            }
            i += 1;
            tokens.push((Kind::Str, text));
            continue;
        }
        if i + 1 < chars.len() {
            let two: String = chars[i..i + 2].iter().collect();
            // '.?' (optional selection) and '[?' (optional index) are taken before '.', '[' and '?'.
            if matches!(two.as_str(), "&&" | "||" | "==" | "!=" | "<=" | ">=" | ".?" | "[?") {
                tokens.push((Kind::Op, two));
                i += 2;
                continue;
            }
        }
        if "!-+*/%<>?:.,()[]{}".contains(c) {
            tokens.push((Kind::Op, c.to_string()));
            i += 1;
            continue;
        }
        return Err(format!("'{c}' is not supported by this CEL evaluator, in '{source}'."));
    }
    Ok(tokens)
}
