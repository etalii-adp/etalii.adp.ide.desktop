//! The `grammar` family (FBL 0.9 section 4.9): a body read by the grammar its binding carries, whose nodes and
//! tokens are the entries rules select by their names; a value is the text of a node, or the group `value` of its
//! `match`; new entries are written from `emit` and removed with their lines, and every write is read again before
//! it is made.

use std::collections::HashMap;
use std::sync::Arc;

use adp_cel::{Map, Value};

use super::{Entry, EntryId, EntryKind, Family, FamilyKind};
use crate::documents::types::{AttributeBinding, HeaderSettings, Slot};
use crate::grammar::{self, Grammar, Reading};
use crate::model::{FindingSeverity, codes};
use crate::planning::new_text;
use crate::planning::plan::{Plan, Planning, refuse};
use crate::rules::selector;
use crate::rules::{Candidate, InsertRequest, ReadElement, SlotChange, SlotRead};
use crate::splice::Operation;
use crate::text::Span;

/// A node or token of the tree, as an entry has it.
#[derive(Debug, Clone)]
pub(crate) struct GrammarEntry {
    pub token: bool,
    /// Every child in order, an optional part that matched nothing being `None`, as CEL sees them.
    pub all_children: Vec<Option<EntryId>>,
}

#[derive(Default)]
pub(crate) struct GrammarData {
    pub grammar: Option<Arc<Grammar>>,
    pub root: Option<EntryId>,
    /// The comment that stands alone on a line, by line index.
    pub comment_lines: HashMap<usize, Span>,
}

impl std::fmt::Debug for GrammarData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GrammarData").field("root", &self.root).finish_non_exhaustive()
    }
}

impl Family {
    fn grammar_data(&self) -> &GrammarData {
        match &self.kind {
            FamilyKind::Grammar(g) => g,
            _ => panic!("the family is not grammar"),
        }
    }

    pub(super) fn grammar_parse(&mut self) {
        let Some(settings) = self.binding.body.grammar.clone() else {
            self.unreadable = Some((0, "The binding has no grammar.".into()));
            return;
        };
        let built = match grammar::build(&settings) {
            Ok(built) => built,
            Err(reason) => {
                self.unreadable = Some((0, reason));
                return;
            }
        };
        let text = self.text.text(0, self.text.len()).to_owned();
        let reading = match built.read(&text) {
            Ok(reading) => reading,
            Err(unreadable) => {
                self.unreadable = Some((unreadable.offset, unreadable.message));
                return;
            }
        };
        let mut comment_lines = HashMap::new();
        for comment in built.comments(&text, 0, text.len()) {
            let line = self.text.line_index_at(comment.start);
            if self.only_whitespace_before(comment.start) {
                comment_lines.insert(line, comment);
            }
            self.leaves.push(comment);
        }
        if let FamilyKind::Grammar(data) = &mut self.kind {
            data.comment_lines = comment_lines;
        }
        self.grammar_entries(&reading);
        for (stopped, skipped) in &reading.unparsed {
            self.report(
                codes::UNPARSED_TEXT,
                FindingSeverity::Warning,
                "This text does not parse; it is kept and binds nothing.",
                Some(Span::new(*stopped, skipped.end.max(*stopped))),
            );
            self.leaves.push(*skipped);
        }
        if let FamilyKind::Grammar(data) = &mut self.kind {
            data.grammar = Some(built);
            data.root = Some(0);
        }
    }

    /// The entries of the tree, the root first, each with its line span.
    fn grammar_entries(&mut self, reading: &Reading) {
        let base = self.entries.len();
        for (index, node) in reading.nodes.iter().enumerate() {
            let own = node.span.unwrap_or_else(|| {
                let parent_start = reading
                    .nodes
                    .iter()
                    .position(|n| n.children.contains(&Some(index)))
                    .and_then(|p| reading.nodes[p].span)
                    .map(|s| s.start)
                    .unwrap_or(0);
                Span::new(parent_start, parent_start)
            });
            let line = self.text.lines()[self.text.line_index_at(own.start)];
            self.entries.push(Entry {
                own,
                line_span: None,
                parent: None,
                children: node.children.iter().flatten().map(|c| base + c).collect(),
                name: Some(node.name.clone()),
                indent: own.start - line.start,
                kind: EntryKind::Grammar(Box::new(GrammarEntry {
                    token: node.token,
                    all_children: node.children.iter().map(|c| c.map(|c| base + c)).collect(),
                })),
            });
            if node.token {
                self.leaves.push(own);
            }
        }
        for index in base..self.entries.len() {
            for child in self.entries[index].children.clone() {
                self.entries[child].parent = Some(index);
            }
        }
        for index in base + 1..self.entries.len() {
            if self.entries[index].own.start == self.entries[index].own.end {
                continue;
            }
            self.entries[index].line_span = self.grammar_line_span_of(index);
            self.order.push(index);
        }
    }

    /// The line span (FBL section 4.1.1): whole lines, the trailing comment on the last included, extended upwards
    /// over comment lines at the same indentation.
    fn grammar_line_span_of(&self, entry: EntryId) -> Option<Span> {
        let e = &self.entries[entry];
        if !self.only_whitespace_before(e.own.start) || !self.grammar_only_trivia_after(e.own.end) {
            return None;
        }
        let lines = self.text.lines();
        let mut first = self.text.line_index_at(e.own.start);
        let last = self.text.line_index_at(e.own.end.saturating_sub(1).max(e.own.start));
        while first > 0
            && let Some(comment) = self.grammar_data().comment_lines.get(&(first - 1))
            && comment.start - lines[first - 1].start == e.indent
        {
            first -= 1;
        }
        Some(Span::new(lines[first].start, lines[last].end))
    }

    /// Whether only whitespace and comments follow `offset` on its line.
    fn grammar_only_trivia_after(&self, offset: usize) -> bool {
        let line = self.text.lines()[self.text.line_index_at(offset)];
        let comments: Vec<Span> = self
            .leaves
            .iter()
            .copied()
            .filter(|l| l.start >= offset && l.start < line.content_end && !self.is_leaf_token(*l))
            .collect();
        let mut i = offset;
        while i < line.content_end {
            if let Some(c) = comments.iter().find(|c| c.start == i) {
                i = c.end.min(line.content_end);
                continue;
            }
            if !matches!(self.text.bytes()[i], b' ' | b'\t') {
                return false;
            }
            i += 1;
        }
        true
    }

    fn is_leaf_token(&self, span: Span) -> bool {
        self.entries
            .iter()
            .any(|e| e.own == span && matches!(&e.kind, EntryKind::Grammar(g) if g.token))
    }

    /// The text of a node or token.
    fn node_text(&self, entry: EntryId) -> &str {
        self.text.text_of(self.entries[entry].own)
    }

    /// Whether a node or token that the relative selector `path` reaches from `entry` has the text `value`.
    fn path_equals(&self, entry: EntryId, path: &str, value: &str) -> bool {
        selector::matches(&self.entries, entry, path, None)
            .into_iter()
            .any(|(e, _)| self.node_text(e) == value)
    }

    /// The first node or token, in document order, that the relative selector `node` reaches from `entry`.
    fn grammar_reach(&self, entry: EntryId, node: &str) -> Option<EntryId> {
        if node == "." {
            return Some(entry);
        }
        let equals = |e: EntryId, p: &str, v: &str| self.path_equals(e, p, v);
        selector::matches(&self.entries, entry, node, Some(&equals))
            .into_iter()
            .next()
            .map(|(e, _)| e)
    }

    pub(super) fn grammar_candidates(&self, rule: usize) -> Vec<Candidate> {
        let (Some(at), Some(root)) = (self.rule(rule).at.as_deref(), self.grammar_data().root) else {
            return Vec::new();
        };
        let equals = |e: EntryId, p: &str, v: &str| self.path_equals(e, p, v);
        selector::matches(&self.entries, root, at, Some(&equals))
            .into_iter()
            .filter(|(e, _)| *e != root || at == "/")
            .map(|(entry, captures)| Candidate {
                rule: Some(rule),
                block: None,
                entry,
                captures,
            })
            .collect()
    }

    /// A node as CEL sees it (FBL 0.9 section 4.1.4): `type`, `text` and `children`, an empty optional part null.
    pub(super) fn grammar_cel_value(&self, entry: EntryId) -> Value {
        let mut map = Map::default();
        map.insert("type", Value::from(self.entries[entry].name.clone().unwrap_or_default()));
        map.insert("text", Value::from(self.node_text(entry)));
        let children = match &self.entries[entry].kind {
            EntryKind::Grammar(g) => g
                .all_children
                .iter()
                .map(|c| c.map(|c| self.grammar_cel_value(c)).unwrap_or(Value::Null))
                .collect(),
            _ => Vec::new(),
        };
        map.insert("children", Value::list(children));
        Value::map(map)
    }

    pub(super) fn grammar_read(&self, candidate: &Candidate, slot: &Slot) -> SlotRead {
        let Some(node) = &slot.node else {
            return SlotRead::read_only_absent(format!("A grammar entry has no {}.", slot.describe()));
        };
        let Some(target) = self.grammar_reach(candidate.entry, node) else {
            return SlotRead::absent();
        };
        let own = self.entries[target].own;
        let text = self.node_text(target);
        let (value, span) = match &slot.node_match {
            None => (text.to_owned(), own),
            Some(expression) => {
                let regex = self.regex(&format!(r"\A(?:{expression})\z"), false);
                let Some(group) = regex.regex.captures(text).and_then(|c| c.name("value")) else {
                    return SlotRead::read_only_absent(format!("'{text}' does not match the slot's match."));
                };
                (
                    group.as_str().to_owned(),
                    Span::new(own.start + group.start(), own.start + group.end()),
                )
            }
        };
        let mut read = SlotRead::new(Value::from(value.as_str()), Some(span), true, true);
        read.wire = Some(value);
        read
    }

    pub(super) fn grammar_header_holds(&self, header: &HeaderSettings) -> bool {
        let Some(line) = &header.line else { return true };
        let first = self.text.lines().first().map(|l| self.text.text(l.start, l.content_end)).unwrap_or("");
        self.regex(line, false).is_match(first)
    }

    // ---- writing (FBL 0.9 section 6.3) ----

    pub(super) fn grammar_format(&self, binding: Option<&AttributeBinding>, value: &Value) -> String {
        new_text::plain(value, binding)
    }

    pub(super) fn grammar_write(&self, plan: &mut Plan<'_>, element: &ReadElement, changes: &[SlotChange<'_>]) -> Planning {
        for change in changes {
            let read = &change.read;
            let Some(span) = read.span.filter(|_| read.present) else {
                return refuse(format!(
                    "The {} has no {} in this file to set.",
                    self.rule(change.rule).type_name,
                    change.attribute
                ));
            };
            let written = self.grammar_format(Some(change.binding), &change.value);
            if !self.reads_back(element, change, span, &written) {
                return refuse(format!(
                    "The new {} would not read back from the text.",
                    change.attribute
                ));
            }
            plan.add_span(Operation::ReplaceValue, span, written);
        }
        Ok(())
    }

    /// Whether the node holding `span` reads `written` as its value once it is written there: its text still a
    /// token of its terminal, and its `match` still giving that value (FBL 0.9 section 4.9).
    fn reads_back(&self, element: &ReadElement, change: &SlotChange<'_>, span: Span, written: &str) -> bool {
        let entry = element.entry();
        let slot = &change.binding.slot;
        let Some(target) = slot.node.as_deref().and_then(|n| self.grammar_reach(entry, n)) else {
            return false;
        };
        let own = self.entries[target].own;
        let text = self.node_text(target);
        let new_text = format!(
            "{}{written}{}",
            &text[..span.start - own.start],
            &text[span.end - own.start..]
        );
        if let Some(expression) = &slot.node_match {
            let regex = self.regex(&format!(r"\A(?:{expression})\z"), false);
            if regex.regex.captures(&new_text).and_then(|c| c.name("value")).map(|g| g.as_str()) != Some(written) {
                return false;
            }
        }
        let is_token = matches!(&self.entries[target].kind, EntryKind::Grammar(g) if g.token);
        let grammar = self.grammar_data().grammar.as_ref();
        !is_token || grammar.is_none_or(|g| g.is_token_of(self.entries[target].name.as_deref().unwrap_or(""), &new_text))
    }

    pub(super) fn grammar_insert(&self, plan: &mut Plan<'_>, request: &InsertRequest) -> Planning {
        let reading = plan.reading;
        let rule = self.rule(request.rule);
        let insert = rule.insert.as_ref().expect("an insertable rule");
        let Some(emit) = &insert.emit else {
            return refuse(format!("The binding gives no form to write a new {} in.", rule.type_name));
        };
        let text = new_text::render(emit, &|name| match name {
            "id" => request.id.clone(),
            "source" => request.source.map(|s| reading.element(s).key.clone()),
            "target" => request.target.map(|t| reading.element(t).key.clone()),
            _ => request
                .values
                .get(name)
                .filter(|v| !v.is_null())
                .map(|v| new_text::plain(v, rule.attribute(name))),
        });
        let root = self.grammar_data().root.unwrap_or(0);
        let container = insert
            .container
            .as_deref()
            .and_then(|c| selector::matches(&self.entries, root, c, None).into_iter().next().map(|(e, _)| e))
            .unwrap_or(root);
        // The entries the rule read, its `when` holding: what `after-last` follows.
        let of_rule: Vec<EntryId> = reading
            .element_indices()
            .iter()
            .map(|&e| reading.element(e))
            .filter(|e| e.rule == request.rule)
            .map(|e| e.entry())
            .collect();
        let count = self.candidates(request.rule).len();
        let children = &self.entries[container].children;
        let (offset, new) = match insert.place.as_str() {
            "start" => match children.first() {
                Some(&first) => self.before_entry(first, &text),
                None => self.at_end_of_body(&text),
            },
            "end-of-document" => self.at_end_of_body(&text),
            "end" => match children.last() {
                Some(&last) => self.after_entry(last, &text),
                None => self.at_end_of_body(&text),
            },
            _ => match of_rule.last().or(children.last()) {
                Some(&last) => self.after_entry(last, &text),
                None => self.at_end_of_body(&text),
            },
        };
        if !self.adds_one(request.rule, count, offset, &new) {
            return refuse(format!(
                "The new {} would not read back from the text.",
                rule.type_name
            ));
        }
        plan.add(Operation::InsertEntry, offset, offset, new);
        Ok(())
    }

    /// `text` as a new entry's lines with `indent`: each line feed in it starts a new line with the indentation.
    fn indented(&self, text: &str, indent: &str, newline: &str) -> String {
        text.split('\n').collect::<Vec<_>>().join(&format!("{newline}{indent}"))
    }

    fn indent_text(&self, entry: EntryId) -> String {
        let own = self.entries[entry].own;
        let line = self.text.lines()[self.text.line_index_at(own.start)];
        self.text.text(line.start, own.start).to_owned()
    }

    /// A new entry after `entry`: after its line span on a line of its own, or, when it does not end its line,
    /// right after it on a new line.
    fn after_entry(&self, entry: EntryId, text: &str) -> (usize, String) {
        let e = &self.entries[entry];
        let indent = self.indent_text(entry);
        match e.line_span {
            Some(line) => {
                let newline = self.newline_at(line.end);
                let lines = self.indented(text, &indent, &newline);
                if line.end == self.text.len() && !self.text.text(0, line.end).ends_with('\n') {
                    return (line.end, format!("{newline}{indent}{lines}"));
                }
                (line.end, format!("{indent}{lines}{newline}"))
            }
            None => {
                let line = self.text.lines()[self.text.line_index_at(e.own.end.saturating_sub(1))];
                let line_indent: String = self
                    .text
                    .text(line.start, line.content_end)
                    .chars()
                    .take_while(|c| *c == ' ' || *c == '\t')
                    .collect();
                let newline = self.newline_at(e.own.end);
                (
                    e.own.end,
                    format!("{newline}{line_indent}{}", self.indented(text, &line_indent, &newline)),
                )
            }
        }
    }

    /// A new entry before `entry`, at the start of its line span.
    fn before_entry(&self, entry: EntryId, text: &str) -> (usize, String) {
        let indent = self.indent_text(entry);
        let start = self.entries[entry]
            .line_span
            .map(|l| l.start)
            .unwrap_or(self.entries[entry].own.start);
        let newline = self.newline_at(start);
        (start, format!("{indent}{}{newline}", self.indented(text, &indent, &newline)))
    }

    /// A new entry after the last byte of the body.
    fn at_end_of_body(&self, text: &str) -> (usize, String) {
        let end = self.text.len();
        let newline = self.newline_at(end);
        let body = self.text.text(0, end);
        let line = self.indented(text, "", &newline);
        if body.is_empty() || body.ends_with('\n') {
            (end, format!("{line}{newline}"))
        } else {
            (end, format!("{newline}{line}"))
        }
    }

    /// Whether the body with `new` inserted at `offset` reads with one more entry of `rule` (FBL 0.9 section 4.9).
    fn adds_one(&self, rule: usize, count: usize, offset: usize, new: &str) -> bool {
        let Some(grammar) = self.grammar_data().grammar.clone() else {
            return false;
        };
        let body = self.text.text(0, self.text.len());
        let changed = format!("{}{new}{}", &body[..offset], &body[offset..]);
        let Ok(reading) = grammar.read(&changed) else {
            return false;
        };
        if reading.unparsed.len() > self.findings.borrow().iter().filter(|f| f.code == codes::UNPARSED_TEXT).count() {
            return false;
        }
        let mut after = Family::create(
            crate::text::BodyText::new(changed.into_bytes()),
            self.binding.clone(),
            self.options.clone(),
        );
        after.parse();
        after.unreadable.is_none() && after.candidates(rule).len() == count + 1
    }

    pub(super) fn grammar_remove(&self, plan: &mut Plan<'_>, element: usize) -> Planning {
        let entry = plan.reading.element(element).entry();
        plan.add_span(Operation::RemoveEntry, self.entries[entry].removal_span(), "");
        Ok(())
    }
}
