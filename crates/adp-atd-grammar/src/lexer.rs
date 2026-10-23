//! The contextual lexer of ATD §3.6: in a parser state, the candidates are the terminals the state can shift and the
//! ignored ones; of those that match, the longest match wins, then the higher priority, then a string over a pattern.

use regex::Regex;

use crate::grammar::{Grammar, TerminalKind};

/// A terminal ready to match.
#[derive(Debug, Clone)]
pub struct Matcher {
    pub name: String,
    pub kind: TerminalKind,
    pub priority: i32,
    pub ignored: bool,
    string: String,
    regex: Option<Regex>,
}

impl Matcher {
    /// The length in bytes of the match at the start of `text`, when there is one.
    pub fn match_len(&self, text: &str) -> Option<usize> {
        match &self.regex {
            None => text.starts_with(&self.string).then_some(self.string.len()),
            Some(regex) => regex.find(text).map(|m| m.end()).filter(|len| *len > 0),
        }
    }
}

/// The terminals of a grammar, compiled.
#[derive(Debug, Clone)]
pub struct Lexer {
    pub matchers: Vec<Matcher>,
}

/// A token: its terminal (an index into [`Lexer::matchers`]) and its byte range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lexeme {
    pub terminal: usize,
    pub start: usize,
    pub end: usize,
}

impl Lexer {
    pub fn new(grammar: &Grammar) -> Result<Lexer, String> {
        let mut matchers = Vec::new();
        for terminal in &grammar.terminals {
            let regex = match terminal.kind {
                TerminalKind::String => None,
                TerminalKind::Pattern => Some(
                    Regex::new(&format!("^(?:{})", to_rust(&terminal.value)))
                        .map_err(|e| format!("the terminal {} does not compile: {e}", terminal.name))?,
                ),
            };
            matchers.push(Matcher {
                name: terminal.name.clone(),
                kind: terminal.kind,
                priority: terminal.priority,
                ignored: grammar.ignore.contains(&terminal.name),
                string: terminal.value.clone(),
                regex,
            });
        }
        Ok(Lexer { matchers })
    }

    pub fn index(&self, name: &str) -> Option<usize> {
        self.matchers.iter().position(|m| m.name == name)
    }

    /// The token at `position` among the `candidates` (indexes of terminals) and the ignored terminals, by the rule
    /// of §3.6. `None` when nothing matches.
    pub fn next(&self, text: &str, position: usize, candidates: &[usize]) -> Option<Lexeme> {
        let rest = &text[position..];
        let mut best: Option<(usize, usize)> = None;
        let consider = |index: usize, best: &mut Option<(usize, usize)>| {
            let matcher = &self.matchers[index];
            let Some(len) = matcher.match_len(rest) else { return };
            let better = match *best {
                None => true,
                Some((current, current_len)) => {
                    let other = &self.matchers[current];
                    len > current_len
                        || (len == current_len
                            && (matcher.priority > other.priority
                                || (matcher.priority == other.priority
                                    && matcher.kind == TerminalKind::String
                                    && other.kind == TerminalKind::Pattern)))
                }
            };
            if better {
                *best = Some((index, len));
            }
        };
        for &index in candidates {
            consider(index, &mut best);
        }
        for (index, matcher) in self.matchers.iter().enumerate() {
            if matcher.ignored && !candidates.contains(&index) {
                consider(index, &mut best);
            }
        }
        best.map(|(terminal, len)| Lexeme {
            terminal,
            start: position,
            end: position + len,
        })
    }
}

/// A terminal's expression (ATD §3.5) as the `regex` crate reads it: `\d`, `\w` and `\s` and their negations are their
/// ASCII classes, and an escaped character that is not special in Rust is written plain.
pub fn to_rust(expression: &str) -> String {
    const SPACE: &str = " \\t\\n\\r\\x0C\\x0B";
    let class = |c: char| -> Option<(&'static str, bool)> {
        match c {
            'd' => Some(("0-9", false)),
            'D' => Some(("0-9", true)),
            'w' => Some(("A-Za-z0-9_", false)),
            'W' => Some(("A-Za-z0-9_", true)),
            's' => Some((SPACE, false)),
            'S' => Some((SPACE, true)),
            _ => None,
        }
    };
    let chars: Vec<char> = expression.chars().collect();
    let mut out = String::with_capacity(expression.len() + 16);
    let mut in_class = false;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' && i + 1 < chars.len() {
            let next = chars[i + 1];
            i += 2;
            if let Some((members, negated)) = class(next) {
                let inner = format!("[{}{members}]", if negated { "^" } else { "" });
                out.push_str(&inner);
                continue;
            }
            if next.is_ascii_punctuation() && !"\\.+*?()|[]{}^$#&-~".contains(next) {
                out.push(next);
            } else {
                out.push('\\');
                out.push(next);
            }
            continue;
        }
        if in_class {
            if c == ']' {
                in_class = false;
            }
            if c == '[' {
                out.push('\\');
            }
            out.push(c);
            i += 1;
            continue;
        }
        if c == '[' {
            in_class = true;
            out.push(c);
            if chars.get(i + 1) == Some(&'^') {
                out.push('^');
                i += 1;
            }
            if chars.get(i + 1) == Some(&']') {
                out.push_str("\\]");
                i += 1;
            }
            i += 1;
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classes_are_ascii_and_nest_inside_classes() {
        let regex = Regex::new(&format!("^(?:{})", to_rust(r"[\s\S]+"))).unwrap();
        assert_eq!(regex.find("a\nb").unwrap().end(), 3);
        let digits = Regex::new(&format!("^(?:{})", to_rust(r"\d+"))).unwrap();
        assert!(digits.find("٣").is_none());
        let slash = Regex::new(&format!("^(?:{})", to_rust(r"\/\*"))).unwrap();
        assert!(slash.is_match("/*"));
    }
}
