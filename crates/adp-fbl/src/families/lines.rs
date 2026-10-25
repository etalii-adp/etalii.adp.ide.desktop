//! The `lines` and `blocks` families (FBL sections 4.6 and 4.7; the standalone host's `Files/Lines/LinesFamily`): a
//! statement is one line, or, when it opens a block, the lines up to and including the one holding the matching
//! `}`. A rule's `line` expression matches a statement's first line, its named groups are the statement's slots,
//! and a group is read word by word where a slot names a `word`.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use adp_cel::Value;

use crate::documents::types::{AttributeBinding, HeaderSettings, Slot};
use crate::families::bullets::BulletsData;
use crate::families::bullets::parser::Bullet;
use crate::families::{Entry, EntryId, EntryKind, Family, FamilyKind};
use crate::model::{FindingSeverity, codes};
use crate::planning::new_text;
use crate::planning::plan::{Plan, Planning, refuse};
use crate::rules::reading::BodyReading;
use crate::rules::{Candidate, InsertRequest, Node, ReadElement, SlotChange, SlotRead, Word};
use crate::splice::Operation;
use crate::text::Span;

/// A statement of the `lines` or `blocks` family.
#[derive(Debug, Clone)]
pub(crate) struct Statement {
    pub first_line: usize,
    /// The statement's first line from its first non-whitespace byte, as the rules' `line` expressions see it.
    pub content: String,
    /// The byte offset of the content's first byte; a match's byte offsets are counted from it.
    pub content_start: usize,
    pub opens: bool,
    pub is_header: Cell<bool>,
    /// The span of the first line's content, without trailing whitespace: what `re-emit-line` replaces.
    pub first_line_span: Span,
    /// What a list item of the `bullets` family has besides (FBL 0.7 section 4.8); none for a statement.
    pub bullet: Option<Box<Bullet>>,
}

/// A named group of one statement as one rule's `line` matched it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Group {
    pub value: String,
    pub span: Span,
}

type Groups = Rc<Vec<(String, Group)>>;

/// What a lines or blocks reading keeps besides its entries.
#[derive(Debug, Default)]
pub(crate) struct LinesData {
    pub blocks: bool,
    /// Set for the `bullets` family, whose statements are its items and the lines outside them.
    pub bullets: Option<BulletsData>,
    pub statements: Vec<EntryId>,
    matches: RefCell<HashMap<(String, EntryId), Option<Groups>>>,
    match_order: RefCell<Vec<(String, EntryId)>>,
    comment_lines: HashSet<usize>,
}

impl LinesData {
    pub fn new(blocks: bool) -> Self {
        LinesData {
            blocks,
            ..LinesData::default()
        }
    }

    pub fn bullets() -> Self {
        LinesData {
            bullets: Some(BulletsData::default()),
            ..LinesData::default()
        }
    }
}

fn group<'a>(groups: &'a [(String, Group)], name: &str) -> Option<&'a Group> {
    groups.iter().find(|(n, _)| n == name).map(|(_, g)| g)
}

/// The items of a list value, which a group holds as its words (FBL section 4.6); none for any other value.
fn words_of(value: &Value) -> Option<&[Value]> {
    value.as_list()
}

/// Whether a value written as one word would read back as several (FBL section 4.6), so it is written in quotes.
fn has_space(text: &str) -> bool {
    text.chars().any(char::is_whitespace)
}

/// `text` as placeholder `name` of `template` writes it: between quotes where it holds a space and the template
/// does not quote it itself.
fn as_word(template: &str, name: &str, value: &Value, text: Option<String>) -> Option<String> {
    match text {
        Some(text)
            if words_of(value).is_none() && has_space(&text) && !template.contains(&format!("\"{{{name}}}\"")) =>
        {
            Some(format!("\"{text}\""))
        }
        other => other,
    }
}

/// The literal word a flag's `word` expression matches.
fn flag_word(slot: &Slot) -> String {
    let mut word = slot.word.as_deref().unwrap_or_default();
    word = word.strip_prefix('^').unwrap_or(word);
    word = word.strip_suffix('$').unwrap_or(word);
    unescape(word)
}

/// .NET's `Regex.Unescape` for what a flag's word holds: a backslash escape is the character it escapes.
fn unescape(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\\'
            && let Some(next) = chars.next()
        {
            out.push(match next {
                'n' => '\n',
                't' => '\t',
                'r' => '\r',
                other => other,
            });
            continue;
        }
        out.push(c);
    }
    out
}

impl Family {
    /// Every statement of the reading, in document order: for the `bullets` family, its items too.
    pub(super) fn lines_statements(&self) -> &[EntryId] {
        &self.lines_data().statements
    }

    fn lines_data(&self) -> &LinesData {
        match &self.kind {
            FamilyKind::Lines(l) => l,
            _ => panic!("the family is not lines or blocks"),
        }
    }

    fn statement(&self, id: EntryId) -> &Statement {
        self.entries[id].statement()
    }

    pub(super) fn lines_parse(&mut self) {
        let comment = self.binding.comment.clone().map(|c| self.regex(&c, false));
        let blocks = self.lines_data().blocks;
        let mut stack: Vec<EntryId> = Vec::new();
        let line_count = self.text.lines().len();
        let mut comment_lines: HashSet<usize> = HashSet::new();
        let mut statements: Vec<EntryId> = Vec::new();
        for index in 0..line_count {
            let line = self.text.lines()[index];
            let start = if index == 0 {
                line.start.max(self.text.bom_length())
            } else {
                line.start
            };
            let mut first = start;
            while first < line.content_end && matches!(self.text.bytes()[first], b' ' | b'\t') {
                first += 1;
            }
            if first == line.content_end {
                continue;
            }
            let content = self.text.text(first, line.content_end).to_owned();
            if let Some(comment) = &comment
                && comment.is_match(self.text.text(start, line.content_end))
            {
                comment_lines.insert(index);
                self.leaves.push(Span::new(first, line.content_end));
                continue;
            }
            let trimmed = content.trim_end_matches([' ', '\t']);
            if blocks && trimmed == "}" {
                let Some(opener) = stack.pop() else {
                    self.unreadable = Some((
                        first,
                        "The braces of the body do not balance: this '}' closes no block.".into(),
                    ));
                    return;
                };
                let o = &mut self.entries[opener];
                o.line_span = Some(Span::new(o.line_span.expect("a statement's line span").start, line.end));
                o.own = o.own.with_end(first + 1);
                continue;
            }
            let own = Span::new(first, first + trimmed.len());
            let mut leading = line.start;
            for above in (0..index).rev() {
                if !comment_lines.contains(&above) {
                    break;
                }
                leading = self.text.lines()[above].start;
            }
            let parent = stack.last().copied();
            let (net, ends_with_open) = braces(&content);
            self.entries.push(Entry {
                own,
                line_span: Some(Span::new(leading, line.end)),
                parent,
                children: Vec::new(),
                name: None,
                indent: first - line.start,
                kind: EntryKind::Statement(Box::new(Statement {
                    first_line: index,
                    content,
                    content_start: first,
                    opens: false,
                    is_header: Cell::new(false),
                    first_line_span: own,
                    bullet: None,
                })),
            });
            let statement = self.entries.len() - 1;
            if let Some(parent) = parent {
                self.entries[parent].children.push(statement);
            }
            statements.push(statement);
            self.order.push(statement);
            self.leaves.push(Span::new(first, line.content_end));
            if blocks {
                if ends_with_open && net == 1 {
                    if let EntryKind::Statement(s) = &mut self.entries[statement].kind {
                        s.opens = true;
                    }
                    stack.push(statement);
                } else if net != 0 {
                    self.unreadable = Some((first, "The braces of the body do not balance on this line.".into()));
                    return;
                }
            }
        }
        if let FamilyKind::Lines(data) = &mut self.kind {
            data.statements = statements;
            data.comment_lines = comment_lines;
        }
        if let Some(&open) = stack.last() {
            self.unreadable = Some((
                self.entries[open].own.start,
                "The braces of the body do not balance: this block is never closed.".into(),
            ));
        }
    }

    /// The named groups of `statement` as `expression` matches them, cached by expression and statement.
    pub(super) fn match_groups(&self, expression: &str, case_insensitive: bool, statement: EntryId) -> Option<Groups> {
        let data = self.lines_data();
        let key = (expression.to_owned(), statement);
        if let Some(cached) = data.matches.borrow().get(&key) {
            return cached.clone();
        }
        let regex = self.regex(expression, case_insensitive);
        let s = self.statement(statement);
        let groups = regex.captures(&s.content).map(|captures| {
            let mut groups = Vec::new();
            for name in regex.regex.capture_names().flatten() {
                if let Some(m) = captures.name(name) {
                    groups.push((
                        name.to_owned(),
                        Group {
                            value: m.as_str().to_owned(),
                            span: Span::new(s.content_start + m.start(), s.content_start + m.end()),
                        },
                    ));
                }
            }
            Rc::new(groups)
        });
        data.matches.borrow_mut().insert(key.clone(), groups.clone());
        data.match_order.borrow_mut().push(key);
        groups
    }

    pub(super) fn lines_header_holds(&self, header: &HeaderSettings) -> bool {
        let Some(line) = &header.line else { return true };
        let Some(&first) = self.lines_data().statements.first() else {
            return false;
        };
        if self.match_groups(line, false, first).is_none() {
            return false;
        }
        self.statement(first).is_header.set(true);
        true
    }

    pub(super) fn to_candidate(
        rule: Option<usize>,
        block: Option<usize>,
        entry: EntryId,
        groups: &Groups,
    ) -> Candidate {
        Candidate {
            rule,
            block,
            entry,
            captures: Rc::new(groups.iter().map(|(n, g)| (n.clone(), g.value.clone())).collect()),
        }
    }

    pub(super) fn lines_candidates(&self, rule: usize) -> Vec<Candidate> {
        let r = self.rule(rule);
        let Some(line) = &r.line else { return Vec::new() };
        let mut candidates = Vec::new();
        for &statement in &self.lines_data().statements {
            if self.statement(statement).is_header.get() {
                continue;
            }
            if let Some(groups) = self.match_groups(line, r.case_insensitive, statement) {
                candidates.push(Family::to_candidate(Some(rule), None, statement, &groups));
            }
        }
        candidates
    }

    pub(super) fn lines_block_candidates(&self, block: usize) -> Vec<Candidate> {
        let b = self.block(block);
        let mut candidates = Vec::new();
        for &statement in &self.lines_data().statements {
            if self.statement(statement).is_header.get() {
                continue;
            }
            if let Some(groups) = self.match_groups(&b.line, b.case_insensitive, statement) {
                candidates.push(Family::to_candidate(None, Some(block), statement, &groups));
            }
        }
        candidates
    }

    fn groups(&self, candidate: &Candidate) -> Groups {
        let (expression, insensitive) = match (candidate.rule, candidate.block) {
            (Some(r), _) => (
                self.rule(r).line.clone().unwrap_or_default(),
                self.rule(r).case_insensitive,
            ),
            (None, Some(b)) => (self.block(b).line.clone(), self.block(b).case_insensitive),
            _ => (String::new(), false),
        };
        self.match_groups(&expression, insensitive, candidate.entry)
            .unwrap_or_default()
    }

    pub(super) fn lines_read(&self, candidate: &Candidate, slot: &Slot) -> SlotRead {
        let Some(group_name) = &slot.group else {
            return SlotRead::read_only_absent(format!(
                "A {} statement has no {}.",
                self.family_name(),
                slot.describe()
            ));
        };
        let groups = self.groups(candidate);
        let Some(g) = group(&groups, group_name) else {
            return if slot.flag {
                SlotRead::new(Value::Bool(false), None, false, true)
            } else {
                SlotRead::absent()
            };
        };
        let Some(word) = &slot.word else {
            let mut read = SlotRead::new(Value::from(g.value.as_str()), Some(g.span), true, true);
            read.node = Node::Group;
            read.words = Some(Rc::new(self.words(g.span)));
            read.wire = Some(g.value.clone());
            return read;
        };
        let expression = self.regex(word, false);
        for w in self.words(g.span) {
            let raw = self.text.text_of(w.span).to_owned();
            let Some(m) = expression.captures(&raw) else { continue };
            if slot.flag {
                let mut read = SlotRead::new(Value::Bool(true), Some(w.span), true, true);
                read.node = Node::Word(w);
                read.wire = Some(raw);
                return read;
            }
            if let Some(value) = m.name("value") {
                let offset = w.span.start + value.start();
                let span = Span::new(offset, offset + value.as_str().len());
                let mut read = SlotRead::new(Value::from(value.as_str()), Some(span), true, true);
                read.wire = Some(value.as_str().to_owned());
                read.node = Node::Word(w);
                return read;
            }
            let mut read = SlotRead::new(Value::from(w.text.as_str()), Some(w.span), true, true);
            read.quote = w.quoted.then(|| "\"".to_owned());
            read.wire = Some(w.text.clone());
            read.node = Node::Word(w);
            return read;
        }
        if slot.flag {
            SlotRead::new(Value::Bool(false), None, false, true)
        } else {
            SlotRead::absent()
        }
    }

    /// The words of a group (FBL section 4.6): runs of non-whitespace, where a run starting with `"` extends to the
    /// next `"` and includes both quotes.
    fn words(&self, span: Span) -> Vec<Word> {
        let bytes = self.text.bytes();
        let mut words = Vec::new();
        let (mut i, end) = (span.start, span.end);
        while i < end {
            while i < end && matches!(bytes[i], b' ' | b'\t') {
                i += 1;
            }
            if i >= end {
                break;
            }
            let start = i;
            if bytes[i] == b'"' {
                i += 1;
                while i < end && bytes[i] != b'"' {
                    i += 1;
                }
                if i < end {
                    i += 1;
                }
                words.push(Word {
                    text: self.text.text(start + 1, (start + 1).max(i - 1)).to_owned(),
                    span: Span::new(start, i),
                    quoted: true,
                });
                continue;
            }
            while i < end && !matches!(bytes[i], b' ' | b'\t') {
                i += 1;
            }
            words.push(Word {
                text: self.text.text(start, i).to_owned(),
                span: Span::new(start, i),
                quoted: false,
            });
        }
        words
    }

    pub(super) fn lines_read_raw(&self, entry: EntryId, name: &str) -> SlotRead {
        let data = self.lines_data();
        let matches = data.matches.borrow();
        for key in data.match_order.borrow().iter() {
            if key.1 != entry {
                continue;
            }
            if let Some(Some(groups)) = matches.get(key)
                && let Some(g) = group(groups, name)
            {
                let mut read = SlotRead::new(Value::from(g.value.as_str()), Some(g.span), true, true);
                read.node = Node::Group;
                return read;
            }
        }
        SlotRead::absent()
    }

    pub(super) fn lines_after_read(&self, claimed_by: &dyn Fn(EntryId) -> Option<String>) {
        if !self.binding.report_unmatched {
            return;
        }
        for &statement in &self.lines_data().statements {
            let s = self.statement(statement);
            if s.is_header.get() || claimed_by(statement).is_some() {
                continue;
            }
            self.report(
                codes::UNBOUND_STATEMENT,
                FindingSeverity::Warning,
                "No rule of the binding reads this statement; it is kept as it is.",
                Some(s.first_line_span),
            );
        }
    }

    // ---- writing ----

    pub(super) fn lines_format(&self, read: &SlotRead, binding: Option<&AttributeBinding>, value: &Value) -> String {
        if let Some(words) = words_of(value) {
            let mut plain = read.clone();
            plain.quote = None;
            plain.wire = None;
            return words
                .iter()
                .map(|word| self.lines_format(&plain, binding, word))
                .collect::<Vec<_>>()
                .join(" ");
        }
        let written = binding
            .and_then(|b| new_text::wire(b, value, read.wire.as_deref()))
            .unwrap_or_else(|| new_text::plain(value, binding));
        if let Some(quote) = &read.quote {
            return format!("{quote}{written}{quote}");
        }
        if matches!(read.node, Node::Word(_)) && has_space(&written) {
            format!("\"{written}\"")
        } else {
            written
        }
    }

    /// The written form of a value for an emit placeholder: a map's first key, a flag's word, a number by FBL 6.3.
    fn emitted(&self, binding: Option<&AttributeBinding>, value: &Value) -> String {
        if let Some(words) = words_of(value) {
            return words
                .iter()
                .map(|word| self.emitted(binding, word))
                .collect::<Vec<_>>()
                .join(" ");
        }
        let Some(binding) = binding else {
            return new_text::plain(value, None);
        };
        if binding.flag {
            return if matches!(value, Value::Bool(true)) {
                flag_word(binding)
            } else {
                String::new()
            };
        }
        new_text::wire(binding, value, None).unwrap_or_else(|| new_text::plain(value, Some(binding)))
    }

    pub(super) fn lines_write(
        &self,
        plan: &mut Plan<'_>,
        element: &ReadElement,
        changes: &[SlotChange<'_>],
    ) -> Planning {
        let statement = element.entry();
        let mut re_emit = false;
        let mut pending: Vec<(Operation, Span, String)> = Vec::new();
        for change in changes {
            let read = &change.read;
            let binding = change.binding;
            if binding.continuation {
                pending.extend(self.continuation_splice(element, change));
                continue;
            }
            if binding.flag {
                if matches!(change.value, Value::Bool(true)) == read.present {
                    continue;
                }
                if read.present {
                    pending.push((
                        Operation::RemoveKey,
                        self.with_space_before(read.span.expect("a present flag has a span")),
                        String::new(),
                    ));
                } else if let Some((offset, text)) = self.insert_point(element, change) {
                    pending.push((Operation::InsertKey, Span::new(offset, offset), text));
                } else {
                    re_emit = true;
                }
                continue;
            }
            if change.is_empty && binding.empty.as_deref() == Some("remove") {
                if read.present {
                    pending.push((Operation::RemoveKey, self.removable(read), String::new()));
                }
                continue;
            }
            if read.present
                && let Some(read_span) = read.span
            {
                if same_number(read, &change.value) || same_words(read, &change.value) {
                    continue;
                }
                let span = self.after_leading_space(read, read_span);
                let text = self.lines_format(read, Some(binding), &change.value);
                if text != self.text.text_of(span) {
                    pending.push((Operation::ReplaceValue, span, text));
                }
                continue;
            }
            if let Some((offset, text)) = self.insert_point(element, change) {
                pending.push((Operation::InsertKey, Span::new(offset, offset), text));
            } else {
                re_emit = true;
            }
        }
        if !re_emit && self.removals_read_back(element, changes, &pending) {
            for (operation, span, text) in pending {
                plan.add_span(operation, span, text);
            }
            return Ok(());
        }
        let rule = self.rule(element.rule);
        let Some(emit) = rule.insert.as_ref().and_then(|i| i.emit.as_ref()) else {
            return refuse(format!(
                "This {} has no place in its line for the new value, and its rule gives no form to write the line in.",
                rule.type_name
            ));
        };
        let mut values = element.attributes.clone();
        for change in changes {
            values.insert(
                change.attribute.clone(),
                if change.is_empty {
                    Value::Null
                } else {
                    change.value.clone()
                },
            );
        }
        let id = element
            .id_read
            .as_ref()
            .and_then(|r| r.value.as_str().map(str::to_owned))
            .unwrap_or_else(|| element.id.clone());
        let source = element
            .source_read
            .as_ref()
            .and_then(|r| r.value.as_str().map(str::to_owned));
        let target = element
            .target_read
            .as_ref()
            .and_then(|r| r.value.as_str().map(str::to_owned));
        let line = self.render_line(
            emit,
            element.rule,
            &values,
            Some(&id),
            source.as_deref(),
            target.as_deref(),
        );
        plan.add_span(Operation::ReEmitLine, self.re_emit_span(statement), line);
        Ok(())
    }

    /// Whether the line, with `pending` applied, still reads as the change says where a value is removed: the rule
    /// still matches it, every group the change does not write reads as it did, and a value that has a group of its
    /// own no longer reads one. A removal can fail that, and the line is then re-emitted instead.
    fn removals_read_back(
        &self,
        element: &ReadElement,
        changes: &[SlotChange<'_>],
        pending: &[(Operation, Span, String)],
    ) -> bool {
        let rule = self.rule(element.rule);
        let Some(expression) = &rule.line else { return true };
        if pending.iter().all(|(op, _, _)| *op != Operation::RemoveKey) {
            return true;
        }
        let statement = self.statement(element.entry());
        let start = statement.content_start;
        let end = start + statement.content.len();
        let mut after = String::new();
        let mut at = start;
        let mut ordered: Vec<&(Operation, Span, String)> = pending.iter().collect();
        ordered.sort_by_key(|(_, span, _)| span.start);
        for (_, span, text) in ordered {
            if span.start < at || span.end > end {
                return true;
            }
            after.push_str(self.text.text(at, span.start));
            after.push_str(text);
            at = span.end;
        }
        after.push_str(self.text.text(at, end));
        let regex = self.regex(expression, rule.case_insensitive);
        let Some(m) = regex.captures(&after) else { return false };
        let changed: HashSet<&str> = changes.iter().filter_map(|c| c.binding.group.as_deref()).collect();
        let held = self
            .match_groups(expression, rule.case_insensitive, element.entry())
            .unwrap_or_default();
        for (name, group) in held.iter() {
            if !changed.contains(name.as_str()) && m.name(name).map(|g| g.as_str()).unwrap_or("") != group.value {
                return false;
            }
        }
        for change in changes {
            let Some(group) = &change.binding.group else { continue };
            if !change.is_empty {
                continue;
            }
            let own = rule
                .attributes
                .iter()
                .filter(|(_, a)| a.group.as_deref() == Some(group.as_str()))
                .count()
                == 1;
            if own && m.name(group).is_some() {
                return false;
            }
        }
        true
    }

    fn re_emit_span(&self, statement: EntryId) -> Span {
        let s = self.statement(statement);
        if !s.opens {
            return s.first_line_span;
        }
        let span = s.first_line_span;
        let mut end = span.end - 1;
        while end > span.start && matches!(self.text.bytes()[end - 1], b' ' | b'\t') {
            end -= 1;
        }
        span.with_end(end)
    }

    fn placeholder(
        &self,
        rule: usize,
        name: &str,
        values: &adp_cel::Map,
        id: Option<&str>,
        source: Option<&str>,
        target: Option<&str>,
    ) -> Option<String> {
        match name {
            "id" => return id.map(str::to_owned),
            "source" => return source.map(str::to_owned),
            "target" => return target.map(str::to_owned),
            _ => {}
        }
        let binding = self.rule(rule).attribute(name);
        values
            .get(name)
            .filter(|v| !v.is_null())
            .map(|v| self.emitted(binding, v))
    }

    fn with_space_before(&self, span: Span) -> Span {
        let mut start = span.start;
        while start > 0 && matches!(self.text.bytes()[start - 1], b' ' | b'\t') {
            start -= 1;
        }
        span.with_start(start)
    }

    /// The span a new value of a whole group replaces: the group without the whitespace it opens with, which
    /// separates it from what comes before and is the line's, not the value's.
    fn after_leading_space(&self, read: &SlotRead, span: Span) -> Span {
        if !matches!(read.node, Node::Group) {
            return span;
        }
        let mut start = span.start;
        while start < span.end && matches!(self.text.bytes()[start], b' ' | b'\t') {
            start += 1;
        }
        span.with_start(start)
    }

    /// A removed value takes its quotes and the whitespace before it with it (FBL section 6.1 `remove-key`), and
    /// the whole word when it was read from part of one, as `size=(?<value>.+)` reads it.
    fn removable(&self, read: &SlotRead) -> Span {
        let mut span = match &read.node {
            Node::Word(word) => word.span,
            _ => read.span.expect("a present value has a span"),
        };
        let bytes = self.text.bytes();
        if span.start > 0 && span.end < bytes.len() && bytes[span.start - 1] == b'"' && bytes[span.end] == b'"' {
            span = Span::new(span.start - 1, span.end + 1);
        }
        self.with_space_before(span)
    }

    /// Where an absent value can be written by `insert-key` (FBL section 6.3), which a host must prefer to
    /// re-emitting the line because it keeps every word no rule binds: when the rule's emit places it right after a
    /// value the statement has, and every value the emit places after it stands after that point. None when the
    /// line must be re-emitted instead.
    fn insert_point(&self, element: &ReadElement, change: &SlotChange<'_>) -> Option<(usize, String)> {
        let emit = self.rule(element.rule).insert.as_ref()?.emit.as_ref()?;
        let parts = new_text::parts(emit);
        let index = parts
            .iter()
            .position(|p| p.placeholder.as_deref() == Some(change.attribute.as_str()))?;
        let segment = parts[index].segment?;
        let first = parts.iter().position(|p| p.segment == Some(segment))?;
        let previous = (0..first).rev().find(|i| parts[*i].placeholder.is_some())?;
        let after = self.span_of(element, parts[previous].placeholder.as_deref()?)?;
        for later in parts.iter().skip(index + 1) {
            if let Some(later) = &later.placeholder
                && let Some(span) = self.span_of(element, later)
                && span.start < after.end
            {
                return None;
            }
        }
        let mut offset = after.end;
        // What the emit writes between that value and the new one, outside the optional parts left out.
        let own = parts[previous].segment;
        let between: String = parts[previous + 1..first]
            .iter()
            .filter(|p| p.segment.is_none() || p.segment == own)
            .map(|p| p.literal.clone().unwrap_or_default())
            .collect();
        if !between.is_empty()
            && offset + between.len() <= self.text.len()
            && self.text.text(offset, offset + between.len()) == between
        {
            offset += between.len();
        }
        let template: String = std::iter::once("[".to_owned())
            .chain(
                parts
                    .iter()
                    .filter(|p| p.segment == Some(segment))
                    .map(|p| match &p.literal {
                        Some(literal) => literal.replace('[', "\\[").replace(']', "\\]"),
                        None => format!("{{{}}}", p.placeholder.as_deref().unwrap_or_default()),
                    }),
            )
            .chain(std::iter::once("]".to_owned()))
            .collect();
        let segment_text = new_text::render(&template, &|name| {
            (name == change.attribute).then(|| {
                self.quoted(
                    change.rule,
                    &template,
                    name,
                    &change.value,
                    Some(self.emitted(Some(change.binding), &change.value)),
                )
            })?
        });
        (!segment_text.is_empty()).then_some((offset, segment_text))
    }

    fn span_of(&self, element: &ReadElement, placeholder: &str) -> Option<Span> {
        match placeholder {
            "id" => element.id_read.as_ref()?.span,
            "source" => element.source_read.as_ref()?.span,
            "target" => element.target_read.as_ref()?.span,
            _ => element.slot(placeholder).filter(|r| r.present)?.span,
        }
    }

    pub(super) fn lines_insert(&self, plan: &mut Plan<'_>, request: &InsertRequest) -> Planning {
        let reading: &BodyReading = plan.reading;
        let rule = self.rule(request.rule);
        let Some(emit) = rule.insert.as_ref().and_then(|i| i.emit.as_ref()) else {
            return refuse(format!(
                "The binding gives no form to write a new {} in.",
                rule.type_name
            ));
        };
        let source = request.source.map(|s| reading.element(s).key.clone());
        let target = request.target.map(|t| reading.element(t).key.clone());
        let line = self.render_line(
            emit,
            request.rule,
            &request.values,
            request.id.as_deref(),
            source.as_deref(),
            target.as_deref(),
        );
        let insert = rule.insert.as_ref().expect("an insertable rule");
        let data = self.lines_data();
        let mut container: Option<EntryId> = None;
        if data.blocks {
            if let Some(parent) = request.parent {
                container = Some(reading.element(parent).entry());
            } else if let Some(name) = &insert.container {
                container = data
                    .statements
                    .iter()
                    .copied()
                    .find(|s| reading.claimed_by(*s) == Some(name.as_str()));
            }
            if container.is_none() && (request.parent.is_some() || insert.container.is_some()) {
                return refuse(format!("The file has no block to add the {} to.", rule.type_name));
            }
        }
        let siblings: Vec<EntryId> = match container {
            None => data
                .statements
                .iter()
                .copied()
                .filter(|s| self.entries[*s].parent.is_none() && !self.statement(*s).is_header.get())
                .collect(),
            Some(c) => self.entries[c].children.clone(),
        };
        let previous = match insert.place.as_str() {
            "after-last" => siblings
                .iter()
                .copied()
                .rfind(|s| reading.claimed_by(*s) == Some(rule.name.as_str()))
                .or_else(|| siblings.last().copied()),
            "end" | "last-child" => siblings.last().copied(),
            "start" => None,
            "end-of-document" => data.statements.last().copied(),
            _ => {
                return refuse(format!(
                    "A {} body cannot place a new entry '{}'.",
                    self.family_name(),
                    insert.place
                ));
            }
        };
        if let Some(c) = container
            && !self.statement(c).opens
        {
            self.open_block(plan, c, &line);
            return Ok(());
        }
        let offset = match (previous, container) {
            (Some(previous), _) => self.entries[previous].line_span.expect("a statement's line span").end,
            (None, Some(c)) => self.text.lines()[self.statement(c).first_line].end,
            (None, None) => siblings
                .first()
                .and_then(|s| self.entries[*s].line_span)
                .map(|l| l.start)
                .unwrap_or(self.text.len()),
        };
        let indent = previous
            .map(|p| self.entries[p].indent)
            .or_else(|| siblings.first().map(|s| self.entries[*s].indent))
            .unwrap_or_else(|| container.map(|c| self.entries[c].indent + self.step()).unwrap_or(0));
        let text = self.lines_new_line(offset, &format!("{}{line}", self.indentation(indent)));
        plan.add(Operation::InsertEntry, offset, offset, text);
        Ok(())
    }

    /// The line `emit` writes for an entry of `rule` with `values` (FBL section 6.3).
    pub(super) fn render_line(
        &self,
        emit: &str,
        rule: usize,
        values: &adp_cel::Map,
        id: Option<&str>,
        source: Option<&str>,
        target: Option<&str>,
    ) -> String {
        new_text::render(emit, &|name| {
            let value = values.get(name).cloned().unwrap_or_default();
            self.quoted(
                rule,
                emit,
                name,
                &value,
                self.placeholder(rule, name, values, id, source, target),
            )
        })
    }

    /// `text` as placeholder `name` of `template` writes it: between quotes where it holds a space and the
    /// placeholder's values are read as words (FBL section 4.6), which is every placeholder of a statement and, of an
    /// item of the `bullets` family, only one of a word slot, so that an item's text is written as it is.
    fn quoted(&self, rule: usize, template: &str, name: &str, value: &Value, text: Option<String>) -> Option<String> {
        let quotes = !self.is_bullets() || self.rule(rule).attribute(name).is_some_and(|a| a.word.is_some());
        if quotes {
            as_word(template, name, value, text)
        } else {
            text
        }
    }

    /// The text of a new line at `offset`, a line start or the end of a body without a final newline (FBL 6.3).
    fn lines_new_line(&self, offset: usize, line: &str) -> String {
        let newline = self.newline_at(offset);
        if offset == self.text.len()
            && !self.text.is_empty()
            && self.text.lines().last().is_some_and(|l| l.ending.is_empty())
        {
            return format!("{newline}{line}");
        }
        format!("{line}{newline}")
    }

    fn open_block(&self, plan: &mut Plan<'_>, parent: EntryId, line: &str) {
        let s = self.statement(parent);
        let first = self.text.lines()[s.first_line];
        let newline = self.newline_at(first.start);
        plan.add(Operation::OpenBlock, s.first_line_span.end, s.first_line_span.end, " {");
        let indent = self.entries[parent].indent;
        let child_indent = self.indentation(indent + self.step());
        let close = format!("{}}}", self.indentation(indent));
        if first.ending.is_empty() {
            plan.add(
                Operation::InsertEntry,
                first.end,
                first.end,
                format!("{newline}{child_indent}{line}"),
            );
            plan.add(Operation::OpenBlock, first.end, first.end, format!("{newline}{close}"));
            return;
        }
        plan.add(
            Operation::InsertEntry,
            first.end,
            first.end,
            format!("{child_indent}{line}{newline}"),
        );
        plan.add(Operation::OpenBlock, first.end, first.end, format!("{close}{newline}"));
    }
}

/// Whether `value` is the number `read` already holds, however it is spelled there.
fn same_number(read: &SlotRead, value: &Value) -> bool {
    let Some(number) = value.as_number() else { return false };
    read.wire
        .as_deref()
        .and_then(|w| w.trim().parse::<f64>().ok())
        .is_some_and(|held| held == number)
}

/// Whether `value` is the list of words `read` already holds, however they are spaced there.
fn same_words(read: &SlotRead, value: &Value) -> bool {
    let (Some(items), Some(words)) = (words_of(value), &read.words) else {
        return false;
    };
    items.len() == words.len()
        && items
            .iter()
            .zip(words.iter())
            .all(|(item, word)| new_text::plain(item, None) == word.text)
}

/// The net braces of a line and whether it ends with an opening one, outside strings.
fn braces(content: &str) -> (i32, bool) {
    let mut net = 0;
    let mut in_string = false;
    let mut last = '\0';
    let chars: Vec<char> = content.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if in_string {
            if c == '\\' && i + 1 < chars.len() {
                i += 1;
            } else if c == '"' {
                in_string = false;
            }
            last = c;
            i += 1;
            continue;
        }
        if c == '"' {
            in_string = true;
        } else if c == '{' {
            net += 1;
        } else if c == '}' {
            net -= 1;
        }
        if c != ' ' && c != '\t' {
            last = c;
        }
        i += 1;
    }
    (net, !in_string && last == '{')
}
