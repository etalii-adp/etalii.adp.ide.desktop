//! The `grammar` family (FBL 0.9 section 4.9): a body read by the grammar its binding carries, whose nodes and
//! tokens are the entries rules select by their names; a value is the text of a node, or the group `value` of its
//! `match`, with its escapes resolved (FBL 0.11); new entries are written from `emit` and removed with their lines, an
//! absent part from its binding's `create`, a block around a first child from its rule's `opens`, and every write is
//! read again before it is made.

use std::collections::HashMap;
use std::sync::Arc;

use adp_cel::{Map, Value};

use super::{Entry, EntryId, EntryKind, Family, FamilyKind};
use crate::documents::types::{AttributeBinding, HeaderSettings, InsertSettings, OpensText, Slot};
use crate::grammar::{self, Grammar, Reading};
use crate::model::{FindingSeverity, codes};
use crate::planning::new_text;
use crate::planning::plan::{Plan, Planning, refuse};
use crate::rules::selector;
use crate::rules::{Candidate, InsertRequest, ReadElement, SlotChange, SlotRead, Word, WordItem};
use crate::splice::Operation;
use crate::text::Span;

/// A node or token of the tree, as an entry has it.
#[derive(Debug, Clone)]
pub(crate) struct GrammarEntry {
    /// Every child in order, an optional part that matched nothing being `None`, as CEL sees them.
    pub all_children: Vec<Option<EntryId>>,
}

#[derive(Default)]
pub(crate) struct GrammarData {
    pub grammar: Option<Arc<Grammar>>,
    pub root: Option<EntryId>,
    /// *(FBL 0.11)* The comments, in document order.
    pub comments: Vec<Span>,
    /// The comment that holds a whole line but for trivia, with the comments before it there, by its last line's index.
    pub comment_lines: HashMap<usize, Span>,
    /// *(FBL 0.11)* Every token read, those the tree leaves out included, in document order.
    pub tokens: Vec<Span>,
    /// *(FBL 0.11)* The entries whose block is still open at the end of the body.
    pub unclosed: Vec<EntryId>,
    /// How many pieces of text do not parse.
    pub unparsed: usize,
}

impl std::fmt::Debug for GrammarData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GrammarData")
            .field("root", &self.root)
            .finish_non_exhaustive()
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
        if let FamilyKind::Grammar(data) = &mut self.kind {
            data.comments = reading.comments.clone();
            data.tokens = reading.tokens.clone();
            data.unparsed = reading.unparsed.len();
        }
        self.leaves.extend(reading.comments.iter().copied());
        let comment_lines = self.grammar_comment_lines();
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
        for close in &reading.unmatched {
            self.report(
                codes::UNMATCHED_CLOSE,
                FindingSeverity::Warning,
                "This closes no block; it is kept and closes nothing.",
                Some(*close),
            );
            self.leaves.push(*close);
        }
        for comment in &reading.unclosed_comments {
            let line = self.text.lines()[self.text.line_index_at(comment.start)];
            self.report(
                codes::UNCLOSED_COMMENT,
                FindingSeverity::Warning,
                "This comment is never closed; it runs to the end of the file.",
                Some(Span::new(comment.start, line.content_end.max(comment.start))),
            );
        }
        let mut unclosed = Vec::new();
        for open in &reading.unclosed_blocks {
            self.report(
                codes::UNCLOSED_BLOCK,
                FindingSeverity::Warning,
                "This block is never closed; it ends at the end of the file.",
                Some(*open),
            );
            unclosed.push(self.owner_of(open.start));
        }
        if let FamilyKind::Grammar(data) = &mut self.kind {
            data.grammar = Some(built);
            data.root = Some(0);
            data.unclosed = unclosed;
        }
    }

    /// The innermost entry whose own span holds the token at `offset`.
    fn owner_of(&self, offset: usize) -> EntryId {
        let mut owner = 0;
        while let Some(&child) = self.entries[owner].children.iter().find(|&&c| {
            let own = self.entries[c].own;
            own.start <= offset && offset < own.end
        }) {
            owner = child;
        }
        owner
    }

    /// The comments that are alone on their lines, but for other comments, by the index of the line they end on: the
    /// lines a leading comment is found on (FBL 0.11 section 4.9, comment rules 1 and 3).
    fn grammar_comment_lines(&self) -> HashMap<usize, Span> {
        let mut comment_lines: HashMap<usize, Span> = HashMap::new();
        for &comment in &self.grammar_data().comments {
            if self.grammar_only_trivia_before(comment.start) && self.grammar_last_trivia_line(comment.end).is_some() {
                let last = self.text.line_index_at(comment.end.saturating_sub(1));
                comment_lines.entry(last).or_insert(comment);
            }
        }
        comment_lines
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
            // A comment before an entry on its line is trivia: the entry's indentation is the line's whitespace.
            let indent = if self.grammar_only_trivia_before(own.start) {
                self.text.text(line.start, own.start).len()
                    - self
                        .text
                        .text(line.start, own.start)
                        .trim_start_matches([' ', '\t'])
                        .len()
            } else {
                own.start - line.start
            };
            self.entries.push(Entry {
                own,
                line_span: None,
                parent: None,
                children: node.children.iter().flatten().map(|c| base + c).collect(),
                name: Some(node.name.clone()),
                indent,
                kind: EntryKind::Grammar(Box::new(GrammarEntry {
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

    /// The line span (FBL section 4.1.1): whole lines, the trailing comment on the last included to the end of its own
    /// last line, extended upwards over comment lines at the same indentation.
    fn grammar_line_span_of(&self, entry: EntryId) -> Option<Span> {
        let e = &self.entries[entry];
        if !self.grammar_only_trivia_before(e.own.start) {
            return None;
        }
        let last = self.grammar_last_trivia_line(e.own.end)?;
        let lines = self.text.lines();
        let mut first = self.text.line_index_at(e.own.start);
        while first > 0
            && let Some(comment) = self.grammar_data().comment_lines.get(&(first - 1))
        {
            let comment_first = self.text.line_index_at(comment.start);
            if comment.start - lines[comment_first].start != e.indent {
                break;
            }
            first = comment_first;
        }
        Some(Span::new(lines[first].start, lines[last].end))
    }

    /// Whether only whitespace and comments stand before `offset` on its line.
    fn grammar_only_trivia_before(&self, offset: usize) -> bool {
        let line = self.text.lines()[self.text.line_index_at(offset)];
        let comments = &self.grammar_data().comments;
        let mut i = line.start.max(self.text.bom_length());
        while i < offset {
            if let Ok(found) = comments.binary_search_by_key(&i, |c| c.start) {
                i = comments[found].end;
                continue;
            }
            if !matches!(self.text.bytes()[i], b' ' | b'\t') {
                return false;
            }
            i += 1;
        }
        true
    }

    /// The index of the line that ends the trivia at `offset`, a comment that runs over lines included; none when
    /// something else follows on the line.
    fn grammar_last_trivia_line(&self, offset: usize) -> Option<usize> {
        let comments = &self.grammar_data().comments;
        let mut i = offset;
        loop {
            let line = self.text.lines()[self.text.line_index_at(i)];
            if i >= line.content_end {
                return Some(self.text.line_index_at(i));
            }
            if let Ok(found) = comments.binary_search_by_key(&i, |c| c.start) {
                // The last byte of a comment that holds its line ending as a token's end stays on that line.
                i = comments[found].end;
                if i >= self.text.len() {
                    return Some(self.text.line_index_at(i.saturating_sub(1)));
                }
                continue;
            }
            if !matches!(self.text.bytes()[i], b' ' | b'\t') {
                return None;
            }
            i += 1;
        }
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

    /// The entries a block rule's `at` selects (FBL 0.11 section 4.9).
    pub(super) fn grammar_block_candidates(&self, block: usize) -> Vec<Candidate> {
        let (Some(at), Some(root)) = (self.block(block).at.as_deref(), self.grammar_data().root) else {
            return Vec::new();
        };
        let equals = |e: EntryId, p: &str, v: &str| self.path_equals(e, p, v);
        selector::matches(&self.entries, root, at, Some(&equals))
            .into_iter()
            .filter(|(e, _)| *e != root)
            .map(|(entry, captures)| Candidate {
                rule: None,
                block: Some(block),
                entry,
                captures,
            })
            .collect()
    }

    /// A node as CEL sees it (FBL 0.9 section 4.1.4): `type`, `text` and `children`, an empty optional part null.
    pub(super) fn grammar_cel_value(&self, entry: EntryId) -> Value {
        let mut map = Map::default();
        map.insert(
            "type",
            Value::from(self.entries[entry].name.clone().unwrap_or_default()),
        );
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
        let Some((value, span)) = self.node_value(target, slot) else {
            let text = self.node_text(target);
            return SlotRead::read_only_absent(format!("'{text}' does not match the slot's match."));
        };
        let value = match &slot.escape {
            Some(escape) => escape.read(&value),
            None => value,
        };
        let mut read = SlotRead::new(Value::from(value.as_str()), Some(span), true, true);
        read.wire = Some(value);
        read
    }

    /// The value of a node a slot reaches, before its escapes are resolved, and its span: the node's text, or the
    /// group `value` of the slot's `match`, which the whole text must match.
    fn node_value(&self, node: EntryId, slot: &Slot) -> Option<(String, Span)> {
        let own = self.entries[node].own;
        let text = self.node_text(node);
        let Some(expression) = &slot.node_match else {
            return Some((text.to_owned(), own));
        };
        let regex = self.regex(&format!(r"\A(?:{expression})\z"), false);
        let group = regex.regex.captures(text).and_then(|c| c.name("value"))?;
        Some((
            group.as_str().to_owned(),
            Span::new(own.start + group.start(), own.start + group.end()),
        ))
    }

    /// Every item of a `many` attribute's slot and of its `gather`, in order (FBL 0.11 section 5.2, FBL 0.13 section
    /// 4.9): each node a slot reaches, or the pieces of it `split` cuts, with the escapes resolved per item. The
    /// attribute is present with no item too: it holds the empty list.
    pub(super) fn grammar_read_many(&self, candidate: &Candidate, attribute: &AttributeBinding) -> SlotRead {
        let equals = |e: EntryId, p: &str, v: &str| self.path_equals(e, p, v);
        let mut items = Vec::new();
        for slot in std::iter::once(&attribute.slot).chain(&attribute.gather) {
            let Some(node) = &slot.node else { continue };
            for (target, _) in selector::matches(&self.entries, candidate.entry, node, Some(&equals)) {
                let Some((value, span)) = self.node_value(target, slot) else {
                    continue;
                };
                let pieces = match &slot.split {
                    None => vec![(value.as_str(), span)],
                    Some(split) => split_items(&value, span, split),
                };
                for (piece, piece_span) in pieces {
                    let read = match &slot.escape {
                        Some(escape) => escape.read(piece),
                        None => piece.to_owned(),
                    };
                    items.push(WordItem {
                        value_span: piece_span,
                        word: Word {
                            text: read.clone(),
                            span: piece_span,
                            quoted: false,
                        },
                        value: read,
                    });
                }
            }
        }
        let mut read = SlotRead::new(
            Value::list(items.iter().map(|item| Value::from(item.value.as_str())).collect()),
            None,
            true,
            true,
        );
        read.items = Some(std::rc::Rc::new(items));
        read
    }

    pub(super) fn grammar_header_holds(&self, header: &HeaderSettings) -> bool {
        let Some(line) = &header.line else { return true };
        let first = self
            .text
            .lines()
            .first()
            .map(|l| self.text.text(l.start, l.content_end))
            .unwrap_or("");
        self.regex(line, false).is_match(first)
    }

    // ---- writing (FBL 0.9 section 6.3) ----

    /// A value as the slot writes it: as text, with every quote and escape character escaped by the slot's `escape`
    /// (FBL 0.11 section 6.3).
    pub(super) fn grammar_format(&self, binding: Option<&AttributeBinding>, value: &Value) -> String {
        let text = new_text::plain(value, binding);
        match binding.and_then(|b| b.slot.escape.as_ref()) {
            Some(escape) => escape.write(&text),
            None => text,
        }
    }

    pub(super) fn grammar_write(
        &self,
        plan: &mut Plan<'_>,
        element: &ReadElement,
        changes: &[SlotChange<'_>],
    ) -> Planning {
        // What padding writes for another change of this edit is that change's value, and it is written once.
        let padded: Vec<&str> = changes
            .iter()
            .filter(|c| !c.read.present)
            .flat_map(|c| self.padded_parts(element, c.binding))
            .collect();
        for change in changes {
            if change.read.items.is_some() {
                return refuse(format!(
                    "The {} of a {} holds several values, which this file's binding cannot change yet.",
                    change.attribute,
                    self.rule(change.rule).type_name
                ));
            }
            if !change.read.present {
                if padded.contains(&change.attribute.as_str()) {
                    continue;
                }
                self.grammar_write_absent(plan, element, change, changes)?;
                continue;
            }
            let Some(span) = change.read.span else {
                return refuse(format!(
                    "The {} has no {} in this file to set.",
                    self.rule(change.rule).type_name,
                    change.attribute
                ));
            };
            let slot = &change.binding.slot;
            if change.is_empty && change.binding.empty.as_deref() == Some("remove") {
                let part = self.part_span(element.entry(), slot, span);
                if !self.reads_back(element, change, part.start, part.end, "", None) {
                    return refuse(format!(
                        "The {} would not read back from the text without its {}.",
                        self.rule(change.rule).type_name,
                        change.attribute
                    ));
                }
                plan.add_span(Operation::RemoveKey, part, "");
                continue;
            }
            let written = self.grammar_format(Some(change.binding), &change.value);
            let value = new_text::plain(&change.value, Some(change.binding));
            if !self.reads_back(element, change, span.start, span.end, &written, Some(&value)) {
                return refuse(format!(
                    "The new {} would not read back from the text.",
                    change.attribute
                ));
            }
            plan.add_span(Operation::ReplaceValue, span, written);
        }
        Ok(())
    }

    /// An absent part (FBL 0.11 section 4.9, *Writing*): one `insert-key` right after the last token of the first
    /// attribute of `after` that is present, of the attributes before it that `pad` writes, nearest last, and the part.
    fn grammar_write_absent(
        &self,
        plan: &mut Plan<'_>,
        element: &ReadElement,
        change: &SlotChange<'_>,
        changes: &[SlotChange<'_>],
    ) -> Planning {
        let rule = self.rule(change.rule);
        let cannot = || {
            refuse(format!(
                "The {} has no {} in this file to set.",
                rule.type_name, change.attribute
            ))
        };
        let Some(create) = &change.binding.create else {
            return cannot();
        };
        let Some(offset) = create.after.iter().find_map(|name| self.end_of_part(element, name)) else {
            return cannot();
        };
        let mut text = String::new();
        for name in self.padded_parts(element, change.binding).iter().rev() {
            let Some(binding) = rule.attribute(name) else {
                return cannot();
            };
            let Some(emit) = binding.create.as_ref().map(|c| c.emit.as_str()) else {
                return cannot();
            };
            let padded = changes.iter().find(|c| c.attribute == *name);
            let value = padded.map(|c| self.grammar_format(Some(binding), &c.value));
            text.push_str(&create_text(emit, value.as_deref().unwrap_or_default()));
        }
        let written = self.grammar_format(Some(change.binding), &change.value);
        text.push_str(&create_text(&create.emit, &written));
        let value = new_text::plain(&change.value, Some(change.binding));
        if !self.reads_back(element, change, offset, offset, &text, Some(&value)) {
            return refuse(format!(
                "The new {} would not read back from the text.",
                change.attribute
            ));
        }
        plan.add(Operation::InsertKey, offset, offset, text);
        Ok(())
    }

    /// The attributes of `after`, nearest first, that a part with `pad` writes too: those before the first present.
    fn padded_parts<'b>(&self, element: &ReadElement, binding: &'b AttributeBinding) -> Vec<&'b str> {
        let Some(create) = binding.create.as_ref().filter(|c| c.pad) else {
            return Vec::new();
        };
        create
            .after
            .iter()
            .map(String::as_str)
            .take_while(|name| self.end_of_part(element, name).is_none())
            .collect()
    }

    /// The end of the last token of the part attribute `name` reads, when it is present.
    fn end_of_part(&self, element: &ReadElement, name: &str) -> Option<usize> {
        let binding = self.rule(element.rule).attribute(name)?;
        let node = binding.slot.node.as_deref()?;
        let target = self.grammar_reach(element.entry(), node)?;
        Some(self.entries[target].own.end)
    }

    /// The span of a part a slot reads, with the whitespace between it and the token before it (FBL 0.11 section 6.1).
    fn part_span(&self, entry: EntryId, slot: &Slot, value: Span) -> Span {
        let own = slot
            .node
            .as_deref()
            .and_then(|n| self.grammar_reach(entry, n))
            .map_or(value, |target| self.entries[target].own);
        let bytes = self.text.bytes();
        let mut start = own.start;
        while start > 0 && matches!(bytes[start - 1], b' ' | b'\t') {
            start -= 1;
        }
        Span::new(start, own.end)
    }

    /// Whether the body reads as it did with `start` to `end` replaced by `written`, and the changed entry holds
    /// `value` in the slot, or no longer has the slot without a value (FBL 0.9 section 4.9): nothing more fails to
    /// parse, its rule holds as many entries, and the entry at its place reads back.
    fn reads_back(
        &self,
        element: &ReadElement,
        change: &SlotChange<'_>,
        start: usize,
        end: usize,
        written: &str,
        value: Option<&str>,
    ) -> bool {
        let candidates = self.candidates(element.rule);
        let Some(place) = candidates.iter().position(|c| c.entry == element.entry()) else {
            return false;
        };
        let body = self.text.text(0, self.text.len());
        let changed = format!("{}{written}{}", &body[..start], &body[end..]);
        let Some(after) = self.grammar_read_again(changed) else {
            return false;
        };
        let again = after.candidates(element.rule);
        if again.len() != candidates.len() {
            return false;
        }
        let read = after.grammar_read(&again[place], &change.binding.slot);
        match value {
            Some(value) => read.present && read.value.as_str() == Some(value),
            None => !read.present,
        }
    }

    /// The family's reading of `changed`, when nothing more fails to parse than now and it can be read at all.
    fn grammar_read_again(&self, changed: String) -> Option<Family> {
        let unparsed = self.grammar_data().unparsed;
        let mut after = Family::create(
            crate::text::BodyText::new(changed.into_bytes()),
            self.binding.clone(),
            self.options.clone(),
        );
        after.parse();
        (after.unreadable.is_none() && after.grammar_data().unparsed <= unparsed).then_some(after)
    }

    pub(super) fn grammar_insert(&self, plan: &mut Plan<'_>, request: &InsertRequest) -> Planning {
        let reading = plan.reading;
        let rule = self.rule(request.rule);
        let insert = rule.insert.as_ref().expect("an insertable rule");
        let Some(emit) = &insert.emit else {
            return refuse(format!(
                "The binding gives no form to write a new {} in.",
                rule.type_name
            ));
        };
        let text = new_text::render(emit, &|name| match name {
            "id" => request.id.clone(),
            "source" => request.source.map(|s| reading.element(s).key.clone()),
            "target" => request.target.map(|t| reading.element(t).key.clone()),
            _ => request
                .values
                .get(name)
                .filter(|v| !v.is_null())
                .map(|v| self.grammar_format(rule.attribute(name), v)),
        });
        // The entries the rule read, its `when` holding: what `after-last` follows.
        let of_rule: Vec<EntryId> = reading
            .element_indices()
            .iter()
            .map(|&e| reading.element(e))
            .filter(|e| e.rule == request.rule)
            .map(|e| e.entry())
            .collect();
        let count = self.candidates(request.rule).len();
        // A placeholder's value is read back from the new entry, as the add gave it (FBL 0.9 section 4.9).
        let holds_values = |after: &Family, new: Span| {
            after
                .candidates(request.rule)
                .iter()
                .find(|c| new.start <= after.entries[c.entry].own.start && after.entries[c.entry].own.start < new.end)
                .is_some_and(|c| {
                    request.values.iter().all(|(name, value)| {
                        let Some(binding) = rule.attribute(name) else {
                            return true;
                        };
                        !emit.contains(&format!("{{{name}}}"))
                            || binding.slot.node.is_none()
                            || after.grammar_read(c, &binding.slot).value.as_str()
                                == Some(new_text::plain(value, Some(binding)).as_str())
                    })
                })
        };
        let check =
            |after: &Family, new: Span| after.candidates(request.rule).len() == count + 1 && holds_values(after, new);
        let parent = request.parent.map(|p| {
            let parent = reading.element(p);
            (parent.entry(), self.rule(parent.rule).opens_text.as_ref())
        });
        let Some(splices) = self.place_new_entry(insert, &text, &of_rule, parent, &check)? else {
            return refuse(format!("The new {} would not read back from the text.", rule.type_name));
        };
        for (operation, offset, new) in splices {
            plan.add(operation, offset, offset, new);
        }
        Ok(())
    }

    /// Adds one of a block rule's entries from its `emit`, planned as an element's insert (FBL 0.11 section 4.9).
    pub(crate) fn grammar_insert_block(&self, plan: &mut Plan<'_>, block: usize, values: &adp_cel::Map) -> Planning {
        let rule = self.block(block);
        if let Some(origin) = &self.options.origin
            && !rule.origins.is_empty()
            && !rule.origins.contains(origin)
        {
            return refuse(format!("A {} cannot be added to this reading.", rule.name));
        }
        let Some(insert) = &rule.insert else {
            return refuse(format!("A {} cannot be added to this file.", rule.name));
        };
        let Some(emit) = &insert.emit else {
            return refuse(format!("The binding gives no form to write a new {} in.", rule.name));
        };
        // The value that names the view is written as its slot reads it.
        let view = rule.view_slot.as_ref().filter(|slot| slot.escape.is_some());
        let text = new_text::render(emit, &|name| {
            values.get(name).filter(|v| !v.is_null()).map(|v| {
                let text = new_text::plain(v, None);
                match view
                    .filter(|slot| slot.node.as_deref() == Some(name))
                    .and_then(|slot| slot.escape)
                {
                    Some(escape) => escape.write(&text),
                    None => text,
                }
            })
        });
        let count = self.grammar_block_candidates(block).len();
        let check = |after: &Family, _: Span| after.grammar_block_candidates(block).len() == count + 1;
        let Some(splices) = self.place_new_entry(insert, &text, &[], None, &check)? else {
            return refuse(format!("The new {} would not read back from the text.", rule.name));
        };
        for (operation, offset, new) in splices {
            plan.add(operation, offset, offset, new);
        }
        Ok(())
    }

    /// Where a new entry goes (FBL section 6.3, *Where it goes*), as splices at their offsets, none when the body with
    /// them does not read as `check` says: the body read again, and the bytes of the entry in it.
    fn place_new_entry(
        &self,
        insert: &InsertSettings,
        text: &str,
        of_rule: &[EntryId],
        parent: Option<(EntryId, Option<&OpensText>)>,
        check: &dyn Fn(&Family, Span) -> bool,
    ) -> Planning<Option<Vec<(Operation, usize, String)>>> {
        let root = self.grammar_data().root.unwrap_or(0);
        let last_child = insert.place == "last-child";
        let container = match parent.filter(|_| last_child) {
            Some((entry, _)) => entry,
            None => insert
                .container
                .as_deref()
                .and_then(|c| {
                    selector::matches(&self.entries, root, c, None)
                        .into_iter()
                        .next()
                        .map(|(e, _)| e)
                })
                .unwrap_or(root),
        };
        if self.grammar_data().unclosed.contains(&container) {
            return refuse("The block is not closed, so nothing can be added to it.");
        }
        let children = &self.entries[container].children;
        let insert_entry = |(offset, new): (usize, String)| vec![(Operation::InsertEntry, offset, new)];
        // A block holds statements on lines of their own; a container's other children are its arguments.
        let statements = || {
            let start = self.entries[container].own.start;
            children
                .iter()
                .copied()
                .rfind(|&c| self.entries[c].line_span.is_some() && self.entries[c].own.start > start)
        };
        let splices = match insert.place.as_str() {
            "start" => insert_entry(match children.first() {
                Some(&first) => self.before_entry(first, text),
                None => self.at_end_of_body(text),
            }),
            "end-of-document" => insert_entry(self.at_end_of_body(text)),
            "last-child" => match statements() {
                Some(last) => insert_entry(self.after_entry(last, text)),
                None => {
                    let opens = parent.and_then(|(_, opens)| opens);
                    self.first_child(container, text, opens)?
                }
            },
            "end" if container != root => match statements() {
                Some(last) => insert_entry(self.after_entry(last, text)),
                None => self.first_child(container, text, None)?,
            },
            "end" => insert_entry(match children.last() {
                Some(&last) => self.after_entry(last, text),
                None => self.at_end_of_body(text),
            }),
            _ => insert_entry(match of_rule.last().or(children.last()) {
                Some(&last) => self.after_entry(last, text),
                None => self.at_end_of_body(text),
            }),
        };
        Ok(self.reads_with(&splices, check).then_some(splices))
    }

    /// Whether the body with `splices`, all insertions, reads as `check` says, given the new entry's bytes.
    fn reads_with(&self, splices: &[(Operation, usize, String)], check: &dyn Fn(&Family, Span) -> bool) -> bool {
        let body = self.text.text(0, self.text.len());
        let mut changed = String::with_capacity(body.len() + 64);
        let mut position = 0;
        let mut new = Span::new(0, 0);
        for (operation, offset, text) in splices {
            changed.push_str(&body[position..*offset]);
            position = *offset;
            if *operation == Operation::InsertEntry {
                new = Span::new(changed.len(), changed.len() + text.len());
            }
            changed.push_str(text);
        }
        changed.push_str(&body[position..]);
        self.grammar_read_again(changed).is_some_and(|after| check(&after, new))
    }

    /// A first child: inside a block with no statement, on a line before its closing token; or, with `opens`, inside
    /// a block written around it (FBL 0.11 section 4.9, *A block written around a first child*).
    fn first_child(
        &self,
        entry: EntryId,
        text: &str,
        opens: Option<&OpensText>,
    ) -> Planning<Vec<(Operation, usize, String)>> {
        let own = self.entries[entry].own;
        let cannot = "The file's binding cannot place a new entry there.";
        let tokens = &self.grammar_data().tokens;
        let last_token = tokens
            .binary_search_by_key(&own.end, |t| t.end)
            .ok()
            .map(|t| self.text.text_of(tokens[t]));
        let parent_indent = self.indent_text(entry);
        let child_indent = format!("{parent_indent}{}", self.indentation(self.step()));
        let has_block = match opens {
            Some(opens) => last_token == Some(opens.close.as_str()),
            None => true,
        };
        if has_block {
            let close =
                tokens
                    .binary_search_by_key(&own.end, |t| t.end)
                    .map_err(|_| crate::planning::plan::Refused {
                        reason: cannot.to_owned(),
                        invalid: false,
                    })?;
            let close = tokens[close];
            if !self.only_whitespace_before(close.start) {
                return refuse(cannot);
            }
            let start = self.text.lines()[self.text.line_index_at(close.start)].start;
            let newline = self.newline_at(start);
            let lines = self.indented(text, &child_indent, &newline);
            return Ok(vec![(
                Operation::InsertEntry,
                start,
                format!("{child_indent}{lines}{newline}"),
            )]);
        }
        let (Some(opens), Some(line)) = (opens, self.entries[entry].line_span) else {
            return refuse(cannot);
        };
        let newline = self.newline_at(line.end);
        let open = opens.open.replace('\n', &format!("{newline}{parent_indent}"));
        let lines = self.indented(text, &child_indent, &newline);
        let close = &opens.close;
        if line.end == self.text.len() && !self.text.text(0, line.end).ends_with('\n') {
            return Ok(vec![
                (Operation::OpenBlock, own.end, open),
                (
                    Operation::InsertEntry,
                    line.end,
                    format!("{newline}{child_indent}{lines}"),
                ),
                (
                    Operation::OpenBlock,
                    line.end,
                    format!("{newline}{parent_indent}{close}"),
                ),
            ]);
        }
        Ok(vec![
            (Operation::OpenBlock, own.end, open),
            (
                Operation::InsertEntry,
                line.end,
                format!("{child_indent}{lines}{newline}"),
            ),
            (
                Operation::OpenBlock,
                line.end,
                format!("{parent_indent}{close}{newline}"),
            ),
        ])
    }

    /// `text` as a new entry's lines with `indent`: each line feed in it starts a new line with the indentation.
    fn indented(&self, text: &str, indent: &str, newline: &str) -> String {
        text.split('\n').collect::<Vec<_>>().join(&format!("{newline}{indent}"))
    }

    fn indent_text(&self, entry: EntryId) -> String {
        let own = self.entries[entry].own;
        let line = self.text.lines()[self.text.line_index_at(own.start)];
        self.text
            .text(line.start, own.start)
            .chars()
            .take_while(|c| matches!(c, ' ' | '\t'))
            .collect()
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
        (
            start,
            format!("{indent}{}{newline}", self.indented(text, &indent, &newline)),
        )
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

    pub(super) fn grammar_remove(&self, plan: &mut Plan<'_>, element: usize) -> Planning {
        let entry = plan.reading.element(element).entry();
        plan.add_span(Operation::RemoveEntry, self.entries[entry].removal_span(), "");
        Ok(())
    }
}

/// The items `split` cuts a value into, each without the white space at its ends, an empty piece being no item (FBL
/// 0.13 section 4.9): each with its span in the body, `value` standing at `span`.
fn split_items<'a>(value: &'a str, span: Span, split: &str) -> Vec<(&'a str, Span)> {
    let mut items = Vec::new();
    let mut start = 0;
    let pieces = value
        .match_indices(split)
        .map(|(at, _)| at)
        .chain(std::iter::once(value.len()));
    for end in pieces {
        let piece = &value[start..end];
        let trimmed = piece.trim_start();
        let lead = piece.len() - trimmed.len();
        let trimmed = trimmed.trim_end();
        if !trimmed.is_empty() {
            let at = span.start + start + lead;
            items.push((trimmed, Span::new(at, at + trimmed.len())));
        }
        start = end + split.len();
    }
    items
}

/// A part's `create.emit` with its value (FBL 0.11 section 4.9).
fn create_text(emit: &str, value: &str) -> String {
    emit.replace("{value}", value)
}

#[cfg(test)]
#[path = "grammar_tests.rs"]
mod tests;
