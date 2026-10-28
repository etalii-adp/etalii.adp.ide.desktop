//! The canonical layout of ATD (etalii.adp spec 035, FR-032, FR-033; ATD §16): two spaces per level of braces, one
//! member per line, a block of settings on one line when it holds no comment and fits in 120 columns, every comment
//! kept with its construct, blank lines between members kept as one, and machine edits as splices.

use std::collections::{BTreeMap, BTreeSet};

use adp_atd_syntax::{Child, Node, Parse, TokenKind, parse};

pub mod templates;

/// The widest line a block of settings is kept on.
pub const WIDTH: usize = 120;

/// The nodes whose braces hold settings, kept on one line when they fit (ATD §16.1).
const INLINE: &[&str] = &[
    "settings_map",
    "settings_block",
    "assignments",
    "attribute_binding_block",
    "cel_map",
];

/// One replacement of a range of bytes (FR-033).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Splice {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

/// Applies splices, given in byte offsets into `text`, that do not overlap.
pub fn apply(text: &str, splices: &[Splice]) -> String {
    let mut ordered: Vec<&Splice> = splices.iter().collect();
    ordered.sort_by_key(|s| (s.start, s.end));
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for splice in ordered {
        out.push_str(&text[at..splice.start]);
        out.push_str(&splice.text);
        at = splice.end;
    }
    out.push_str(&text[at..]);
    out
}

/// The one splice that turns `old` into `new`: what lies between their common start and their common end.
pub fn splice_between(old: &str, new: &str) -> Option<Splice> {
    if old == new {
        return None;
    }
    let prefix = old.bytes().zip(new.bytes()).take_while(|(a, b)| a == b).count();
    let prefix = (0..=prefix)
        .rev()
        .find(|p| old.is_char_boundary(*p) && new.is_char_boundary(*p))
        .unwrap_or(0);
    let max_suffix = (old.len() - prefix).min(new.len() - prefix);
    let suffix = old
        .bytes()
        .rev()
        .zip(new.bytes().rev())
        .take(max_suffix)
        .take_while(|(a, b)| a == b)
        .count();
    let suffix = (0..=suffix)
        .rev()
        .find(|s| old.is_char_boundary(old.len() - s) && new.is_char_boundary(new.len() - s))
        .unwrap_or(0);
    Some(Splice {
        start: prefix,
        end: old.len() - suffix,
        text: new[prefix..new.len() - suffix].to_owned(),
    })
}

/// The file in the canonical layout, or why it is not formatted: a file that does not parse is left as it is
/// (ATD §16.3).
pub fn format(text: &str) -> Result<String, String> {
    let parsed = parse(text);
    if let Some(error) = parsed.errors.first() {
        return Err(format!("it does not parse: {}", error.message));
    }
    let root = parsed.root.as_ref().ok_or("it does not parse")?;
    let plan = Plan::new(&parsed, root);
    Ok(Printer::new(&parsed, plan).print())
}

/// What the tree says about each token's layout.
#[derive(Default)]
struct Plan {
    /// The `{` of each block of settings, with its `}`.
    inline_open: BTreeMap<usize, usize>,
    /// The `{` of each block of members, with its `}`.
    block_open: BTreeMap<usize, usize>,
    /// The first token of every member of a block, which starts a line.
    member_start: BTreeSet<usize>,
    /// Tokens that are unary operators.
    unary: BTreeSet<usize>,
    /// Tokens of names, which an opening parenthesis follows without a space.
    names: BTreeSet<usize>,
    /// Tokens inside a type (`list<Concept>`), whose `<` and `>` have no spaces.
    types: BTreeSet<usize>,
    /// The `:` and `?` of a conditional, which have spaces.
    conditional: BTreeSet<usize>,
    /// The first and last token of each grammar a binding carries (`grammar { … }`, ATD §11.9), which is ATD grammar
    /// notation, not ATD, and keeps its own layout: only its indentation follows the block it stands in.
    verbatim: BTreeMap<usize, usize>,
}

impl Plan {
    fn new(parsed: &Parse, root: &Node) -> Plan {
        let mut plan = Plan::default();
        plan.visit(parsed, root);
        // The tool or part, and the members of a part, start lines.
        for child in root.nodes() {
            if let Some(first) = first_token(child) {
                plan.member_start.insert(first);
            }
            if child.kind() == "part" {
                for member in child.nodes() {
                    if let Some(first) = first_token(member) {
                        plan.member_start.insert(first);
                    }
                }
            }
        }
        plan
    }

    fn visit(&mut self, parsed: &Parse, node: &Node) {
        let kind = node.kind();
        let open = node.tokens().find(|t| parsed.token_text(*t) == "{");
        let close = node.tokens().filter(|t| parsed.token_text(*t) == "}").last();
        if let (Some(open), Some(close)) = (open, close) {
            if INLINE.contains(&kind) && !has_member_constructs(node) {
                self.inline_open.insert(open, close);
            } else {
                self.block_open.insert(open, close);
                for child in node.nodes() {
                    if let Some(first) = first_token(child)
                        && first > open
                        && first < close
                    {
                        self.member_start.insert(first);
                    }
                }
            }
        }
        match kind {
            "cel_not" | "cel_negate" | "negative_literal" => {
                if let Some(op) = node.tokens().next() {
                    self.unary.insert(op);
                }
            }
            "name" | "word" | "cel_ident" => self.names.extend(all_tokens(node)),
            "type_ref" => self.types.extend(all_tokens(node)),
            "grammar_text" => {
                if let (Some(first), Some(last)) = (first_token(node), all_tokens(node).last().copied()) {
                    self.verbatim.insert(first, last);
                }
                return;
            }
            "cel_conditional" => {
                for t in node.tokens() {
                    if matches!(parsed.token_text(t), ":" | "?") {
                        self.conditional.insert(t);
                    }
                }
            }
            _ => {}
        }
        for child in node.nodes() {
            self.visit(parsed, child);
        }
    }
}

/// A settings block that holds a `references`, `label` or `doc` member is laid out as a block of members.
fn has_member_constructs(node: &Node) -> bool {
    node.nodes()
        .any(|n| matches!(n.kind(), "references" | "label_member" | "doc_member"))
}

fn first_token(node: &Node) -> Option<usize> {
    for child in &node.children {
        match child {
            Child::Token(t) => return Some(*t),
            Child::Node(n) => {
                if let Some(t) = first_token(n) {
                    return Some(t);
                }
            }
        }
    }
    None
}

fn all_tokens(node: &Node) -> Vec<usize> {
    let mut out = Vec::new();
    for child in &node.children {
        match child {
            Child::Token(t) => out.push(*t),
            Child::Node(n) => out.extend(all_tokens(n)),
        }
    }
    out
}

fn is_comment(text: &str) -> bool {
    text.starts_with("//") || text.starts_with("/*")
}

struct Printer<'a> {
    parsed: &'a Parse,
    plan: Plan,
    lines: Vec<String>,
    line: String,
    indent: usize,
    /// The token last written, by index.
    previous: Option<usize>,
    /// The `}` of the block of settings being written on one line.
    inline_until: Option<usize>,
    newline: &'static str,
}

impl<'a> Printer<'a> {
    fn new(parsed: &'a Parse, plan: Plan) -> Self {
        let first_line = parsed.text.split('\n').next().unwrap_or("");
        let newline = if first_line.ends_with('\r') { "\r\n" } else { "\n" };
        Printer {
            parsed,
            plan,
            lines: Vec::new(),
            line: String::new(),
            indent: 0,
            previous: None,
            inline_until: None,
            newline,
        }
    }

    fn text(&self, token: usize) -> &'a str {
        self.parsed.tokens[token].text(&self.parsed.text)
    }

    fn break_line(&mut self) {
        if self.line.trim().is_empty() {
            self.line.clear();
            return;
        }
        let line = std::mem::take(&mut self.line);
        self.lines.push(line.trim_end().to_owned());
    }

    /// One blank line, unless a block has just opened.
    fn blank_line(&mut self) {
        self.break_line();
        if self.lines.last().is_some_and(|l| !l.is_empty() && !l.ends_with('{')) {
            self.lines.push(String::new());
        }
    }

    fn start_line(&mut self) {
        if self.line.is_empty() {
            self.line.push_str(&"  ".repeat(self.indent));
        }
    }

    fn print(mut self) -> String {
        let count = self.parsed.tokens.len();
        let mut newlines_before = 0usize;
        let mut after_comment = false;
        let mut skip_until = None;
        for index in 0..count {
            if let Some(last) = skip_until {
                if index <= last {
                    continue;
                }
                skip_until = None;
            }
            if let Some(&last) = self.plan.verbatim.get(&index) {
                self.verbatim(index, last);
                self.previous = Some(last);
                skip_until = Some(last);
                newlines_before = 0;
                continue;
            }
            let token = self.parsed.tokens[index];
            if token.kind == TokenKind::Trivia {
                let text = self.text(index);
                if is_comment(text) {
                    self.comment(index, newlines_before);
                    after_comment = true;
                    newlines_before = 0;
                } else {
                    newlines_before += text.matches('\n').count();
                }
                continue;
            }
            if after_comment {
                self.break_line();
                after_comment = false;
            }
            self.token(index, newlines_before);
            newlines_before = 0;
        }
        self.break_line();
        let mut out = self.lines.join(self.newline);
        out.push_str(self.newline);
        out
    }

    /// A binding's grammar from token `first` to `last`, line by line as written, its lines without the indentation
    /// they share and with the block's own.
    fn verbatim(&mut self, first: usize, last: usize) {
        let text = &self.parsed.text;
        let start = self.parsed.tokens[first].start;
        let end = self.parsed.tokens[last].end;
        let line_start = text[..start].rfind('\n').map_or(0, |n| n + 1);
        let written = &text[line_start..end];
        let lines: Vec<&str> = written.lines().map(|l| l.trim_end_matches('\r')).collect();
        let shared = lines
            .iter()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.len() - l.trim_start().len())
            .min()
            .unwrap_or(0);
        self.break_line();
        for line in lines {
            if line.trim().is_empty() {
                self.lines.push(String::new());
            } else {
                self.lines
                    .push(format!("{}{}", "  ".repeat(self.indent), &line[shared..]));
            }
        }
    }

    fn comment(&mut self, index: usize, newlines_before: usize) {
        let text = self.text(index).trim_end().to_owned();
        if newlines_before == 0 && self.previous.is_some() && !self.line.trim().is_empty() {
            // At the end of a line: it stays there.
            self.line.push_str("  ");
            self.line.push_str(&text);
            return;
        }
        if newlines_before > 1 && self.previous.is_some() {
            self.blank_line();
        } else {
            self.break_line();
        }
        self.start_line();
        self.line.push_str(&text);
        self.break_line();
    }

    fn token(&mut self, index: usize, newlines_before: usize) {
        if let Some(until) = self.inline_until {
            // Written already, with the block of settings it belongs to.
            if index == until {
                self.inline_until = None;
            }
            self.previous = Some(index);
            return;
        }
        let closes_block = self.plan.block_open.values().any(|c| *c == index);
        if closes_block {
            self.indent = self.indent.saturating_sub(1);
            self.break_line();
        } else if self.plan.member_start.contains(&index) {
            let after_open = self.previous.is_some_and(|p| self.text(p) == "{");
            if newlines_before > 1 && !after_open {
                self.blank_line();
            } else {
                self.break_line();
            }
        }
        self.start_line();
        if !self.line.trim().is_empty() && self.previous.is_some_and(|p| self.space(p, index)) {
            self.line.push(' ');
        }
        if let Some(&close) = self.plan.inline_open.get(&index) {
            let rendered = self.render_inline(index, close);
            if !rendered.contains('\n') && self.line.chars().count() + rendered.chars().count() <= WIDTH {
                self.line.push_str(&rendered);
                self.previous = Some(index);
                self.inline_until = Some(close);
                return;
            }
            // Too wide, or holding a comment: its entries one per line.
            self.plan.block_open.insert(index, close);
            self.mark_entries(index, close);
        }
        let text = self.text(index);
        self.line.push_str(text);
        self.previous = Some(index);
        if self.plan.block_open.contains_key(&index) {
            self.indent += 1;
            self.break_line();
        } else if closes_block {
            let next_is_else = self.next_significant(index).is_some_and(|n| self.text(n) == "else");
            if !next_is_else {
                self.break_line();
            }
        }
    }

    fn next_significant(&self, index: usize) -> Option<usize> {
        (index + 1..self.parsed.tokens.len()).find(|i| self.parsed.tokens[*i].kind != TokenKind::Trivia)
    }

    /// Marks the first token of each entry of a block of settings laid out over lines: a key, or a string key.
    fn mark_entries(&mut self, open: usize, close: usize) {
        let mut depth = 0i32;
        for i in open + 1..close {
            if self.parsed.tokens[i].kind == TokenKind::Trivia {
                continue;
            }
            let text = self.text(i);
            let name = self.parsed.tokens[i].name();
            let quoted_key = name == "STRING" && self.next_significant(i).is_some_and(|n| self.text(n) == ":");
            if depth == 0 && (name == "KEY" || quoted_key) {
                self.plan.member_start.insert(i);
            }
            match text {
                "{" | "(" | "[" => depth += 1,
                "}" | ")" | "]" => depth -= 1,
                _ => {}
            }
        }
    }

    /// The tokens from `open` to `close` on one line, spaced as this layout spaces them; a line end where a comment
    /// is among them, so that such a block is never kept on one line.
    fn render_inline(&self, open: usize, close: usize) -> String {
        let mut out = String::new();
        let mut previous: Option<usize> = None;
        for i in open..=close {
            if self.parsed.tokens[i].kind == TokenKind::Trivia {
                if is_comment(self.text(i)) {
                    out.push('\n');
                }
                continue;
            }
            if let Some(p) = previous
                && self.space(p, i)
            {
                out.push(' ');
            }
            out.push_str(self.text(i));
            previous = Some(i);
        }
        out
    }

    /// Whether a space stands between two tokens on one line (ATD §16.1).
    fn space(&self, previous: usize, next: usize) -> bool {
        let p = self.text(previous);
        let n = self.text(next);
        if self.plan.unary.contains(&previous) {
            return false;
        }
        if self.plan.conditional.contains(&next) || self.plan.conditional.contains(&previous) {
            return true;
        }
        if self.plan.types.contains(&previous) && self.plan.types.contains(&next) && (n == "<" || n == ">" || p == "<")
        {
            return false;
        }
        if p == "{" {
            return n != "}";
        }
        if n == "}" {
            return true;
        }
        if matches!(n, "," | ")" | "]" | "." | ":") || self.parsed.tokens[next].name() == "CAPABILITY_RANGE" {
            return false;
        }
        if n == "?" && (p == "." || p == "[") {
            return false;
        }
        if matches!(p, "(" | "[" | ".") {
            return false;
        }
        if p == "?"
            && previous > 0
            && self
                .previous_significant(previous)
                .is_some_and(|q| matches!(self.text(q), "." | "["))
        {
            return false;
        }
        if n == "(" && (self.plan.names.contains(&previous) || self.parsed.tokens[previous].name() == "NAME") {
            return false;
        }
        if n == "[" && (self.plan.names.contains(&previous) || matches!(p, ")" | "]")) {
            return false;
        }
        true
    }

    fn previous_significant(&self, index: usize) -> Option<usize> {
        (0..index)
            .rev()
            .find(|i| self.parsed.tokens[*i].kind != TokenKind::Trivia)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_splice_covers_only_what_changed() {
        let splice = splice_between("label \"Mind map\"", "label \"Mind maps\"").unwrap();
        assert_eq!(
            splice,
            Splice {
                start: 15,
                end: 15,
                text: "s".into()
            }
        );
        assert_eq!(apply("label \"Mind map\"", &[splice]), "label \"Mind maps\"");
    }
}
