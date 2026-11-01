//! The `grammar` family (FBL 0.9 section 4.9): a body read by the grammar its binding carries, whose nodes and
//! tokens are the entries rules select by their names; a value is the text of a node, or the group `value` of its
//! `match`, with its escapes resolved (FBL 0.11), or several items, each a node, a structured value (FBL 0.12) or a piece
//! of a node's value (FBL 0.13), of which a set writes the difference; new entries are written from `emit` and removed
//! with their lines, an absent part from its binding's `create`, a block around a first child from its rule's `opens`,
//! and every write is read again before it is made. *(FBL 0.15)* A slot may reach up and across the tree, and read a term
//! through the body's prefix directives, a node's number or its name.

use std::collections::HashMap;
use std::sync::Arc;

use adp_cel::{Map, Value};

use super::lines::common_subsequence;
use super::{Entry, EntryId, EntryKind, Family, FamilyKind};
use crate::documents::types::{
    AttributeBinding, ContinueSettings, HeaderSettings, InsertSettings, OpensText, PrefixSettings, Slot, SlotEscape,
    SlotExpand,
};
use crate::grammar::{self, Grammar, Reading};
use crate::iri;
use crate::model::{FindingSeverity, codes};
use crate::planning::new_text;
use crate::planning::plan::{Plan, Planning, refuse};
use crate::rules::selector;
use crate::rules::{
    Candidate, InsertRequest, ItemPart, ReadElement, SlotChange, SlotRead, Word, WordItem, scalar_conversion,
};
use crate::splice::Operation;
use crate::text::Span;

/// A splice of a set of several items, before it is read again.
type Pending = (Operation, Span, String);

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
    /// *(FBL 0.15)* Each prefix declared, in document order: where it takes force, its name and its IRI.
    pub declarations: Vec<(usize, String, String)>,
    /// *(FBL 0.15)* Each base declared, in document order: where it takes force and its IRI.
    pub bases: Vec<(usize, String)>,
    /// *(FBL 0.15)* The number of each node in each numbering of `body.ordinals`.
    pub ordinals: HashMap<(String, EntryId), usize>,
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
        // A byte-order mark belongs to no node, and offsets count it (FBL section 2.6).
        let mut text = self.text.text(0, self.text.len()).to_owned();
        text.replace_range(..self.text.bom_length(), &" ".repeat(self.text.bom_length()));
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
            // A token the grammar names is an entry of its own: the block is the entry that holds it (FBL 0.11 section 4.9).
            let opening = self.owner_of(open.start);
            let named = self.binding.body.grammar.as_ref().and_then(|g| g.recover.as_ref());
            let named = named.and_then(|r| r.nested.as_ref());
            unclosed.push(match self.entries[opening].parent {
                Some(block) if named.is_some_and(|n| self.entries[opening].name.as_ref() == Some(&n.open)) => block,
                _ => opening,
            });
        }
        if let FamilyKind::Grammar(data) = &mut self.kind {
            data.grammar = Some(built);
            data.root = Some(0);
            data.unclosed = unclosed;
        }
        self.grammar_directives();
        self.grammar_ordinals();
    }

    /// The prefixes and bases the directives of `body.prefixes` declare, each in force from the end of its directive,
    /// and the first term under a prefix none is in force for, which makes the body unreadable (FBL 0.15 section 4.9).
    fn grammar_directives(&mut self) {
        let Some(settings) = self.binding.body.prefixes.clone() else {
            return;
        };
        let root = 0;
        let mut bases: Vec<(usize, String)> = Vec::new();
        let mut directives: Vec<(usize, Option<&Slot>, &Slot)> = Vec::new();
        for directive in settings.declarations.iter().chain(&settings.bases) {
            for (entry, _) in selector::matches(&self.entries, root, &directive.at, None) {
                directives.push((entry, directive.name.as_ref(), &directive.iri));
            }
        }
        directives.sort_by_key(|(entry, ..)| self.entries[*entry].own.start);
        let mut declarations: Vec<(usize, String, String)> = Vec::new();
        for (entry, name, iri) in directives {
            let own = self.entries[entry].own;
            let written = self.directive_value(entry, iri).unwrap_or_default();
            let value = self.resolved(&settings, &written, bases.last().map(|(_, base)| base.as_str()));
            match name.and_then(|slot| self.directive_value(entry, slot)) {
                Some(name) => declarations.push((own.end, name, value)),
                None => bases.push((own.end, value)),
            }
        }
        // A name inside a directive declares; it is no use of a prefix.
        let declaring: Vec<Span> = settings
            .declarations
            .iter()
            .flat_map(|directive| selector::matches(&self.entries, root, &directive.at, None))
            .map(|(entry, _)| self.entries[entry].own)
            .collect();
        let prefixed = self.regex(&format!(r"\A(?:{})\z", settings.prefixed), false);
        let mut undeclared = None;
        for index in 1..self.entries.len() {
            let entry = &self.entries[index];
            if !entry
                .name
                .as_ref()
                .is_some_and(|name| settings.terminals.contains(name))
            {
                continue;
            }
            let text = self.text.text_of(entry.own);
            let Some(prefix) = prefixed
                .regex
                .captures(text)
                .and_then(|c| c.name("prefix").map(|p| p.as_str().to_owned()))
            else {
                continue;
            };
            let start = entry.own.start;
            if !declarations.iter().any(|(at, name, _)| *at <= start && *name == prefix)
                && undeclared.as_ref().is_none_or(|(at, _)| start < *at)
                && !declaring.iter().any(|own| own.start <= start && start < own.end)
            {
                undeclared = Some((start, prefix));
            }
        }
        if let Some((offset, prefix)) = undeclared {
            self.unreadable = Some((offset, format!("The prefix '{prefix}:' is not declared.")));
        }
        if let FamilyKind::Grammar(data) = &mut self.kind {
            data.declarations = declarations;
            data.bases = bases;
        }
    }

    /// How many nodes hold this one: of two with one span, the one that holds the other has the smaller depth.
    pub(crate) fn depth_of(&self, entry: EntryId) -> usize {
        std::iter::successors(self.entries[entry].parent, |&parent| self.entries[parent].parent).count()
    }

    /// A directive's name or IRI, as its slot reads it from the directive.
    fn directive_value(&self, entry: EntryId, slot: &Slot) -> Option<String> {
        let target = self.grammar_reach(entry, slot.node.as_deref()?)?;
        Some(self.node_value(target, slot)?.0)
    }

    /// A term written whole: without what `absolute` leaves out, its numeric escapes read, resolved against `base`.
    fn resolved(&self, settings: &PrefixSettings, written: &str, base: Option<&str>) -> String {
        let regex = self.regex(&format!(r"\A(?:{})\z", settings.absolute), false);
        let inner = regex
            .regex
            .captures(written)
            .and_then(|c| c.name("value"))
            .map_or(written, |value| value.as_str());
        let value = SlotEscape {
            with: '\\',
            quote: '\\',
            unicode: vec![('u', 4), ('U', 8)],
            ..SlotEscape::default()
        }
        .read(inner);
        match base {
            None => value,
            Some(base) if value.is_empty() => base.to_owned(),
            Some(_) if iri::is_absolute(&value) => value,
            Some(base) => iri::resolve(base, &value).unwrap_or(value),
        }
    }

    /// A term, as a slot with `expand` reads it (FBL 0.15 section 4.9): a keyword's IRI, the IRI of a prefixed name under
    /// the last declaration in force where it stands, or the IRI written whole, against the base in force there.
    fn expanded(&self, target: EntryId, expand: &SlotExpand) -> String {
        let text = self.node_text(target);
        if let Some((_, iri)) = expand.keywords.iter().find(|(word, _)| word == text) {
            return iri.clone();
        }
        let Some(settings) = &self.binding.body.prefixes else {
            return text.to_owned();
        };
        let start = self.entries[target].own.start;
        let data = self.grammar_data();
        let prefixed = self.regex(&format!(r"\A(?:{})\z", settings.prefixed), false);
        if let Some(captures) = prefixed.regex.captures(text) {
            let prefix = captures.name("prefix").map_or("", |p| p.as_str());
            let local = captures.name("local").map_or("", |l| l.as_str());
            if let Some((_, _, iri)) = data
                .declarations
                .iter()
                .rev()
                .find(|(at, name, _)| *at <= start && name == prefix)
            {
                let mut read = iri.clone();
                let mut chars = local.chars();
                while let Some(c) = chars.next() {
                    read.push(if c == '\\' { chars.next().unwrap_or(c) } else { c });
                }
                return read;
            }
            return text.to_owned();
        }
        let base = data.bases.iter().rev().find(|(at, _)| *at <= start);
        self.resolved(settings, text, base.map(|(_, base)| base.as_str()))
    }

    /// A term as a set or an add writes it (FBL 0.15 section 6.3): its keyword, else under the longest declared prefix
    /// that leaves a local name `local` admits, a later declaration winning a tie, else written whole.
    fn compressed(&self, iri: &str, expand: &SlotExpand) -> String {
        if let Some((word, _)) = expand.keywords.iter().find(|(_, stands)| stands == iri) {
            return word.clone();
        }
        let whole = format!("<{iri}>");
        let Some(settings) = &self.binding.body.prefixes else {
            return whole;
        };
        let mut best: Option<&(usize, String, String)> = None;
        for declaration in &self.grammar_data().declarations {
            if iri.starts_with(declaration.2.as_str())
                && iri.len() != declaration.2.len()
                && best.is_none_or(|b| declaration.2.len() >= b.2.len())
            {
                best = Some(declaration);
            }
        }
        let Some((_, prefix, namespace)) = best else {
            return whole;
        };
        let local = &iri[namespace.len()..];
        if self
            .regex(&format!(r"\A(?:{})\z", settings.local), false)
            .regex
            .is_match(local)
        {
            format!("{prefix}:{local}")
        } else {
            whole
        }
    }

    /// The numberings of `body.ordinals` (FBL 0.15 section 4.9): the nodes each selects numbered from 0, in document
    /// order with a node before the nodes it holds, and a node of the terminal `same` taking the number of an earlier one
    /// with its text.
    fn grammar_ordinals(&mut self) {
        let mut numbered = HashMap::new();
        for (name, numbering) in &self.binding.body.ordinals {
            let mut nodes: Vec<EntryId> = numbering
                .at
                .iter()
                .flat_map(|at| selector::matches(&self.entries, 0, at, None))
                .map(|(entry, _)| entry)
                .collect();
            nodes.sort_by_key(|&entry| {
                let own = self.entries[entry].own;
                (own.start, std::cmp::Reverse(own.end), self.depth_of(entry))
            });
            nodes.dedup();
            let mut next = 0;
            let mut seen: HashMap<&str, usize> = HashMap::new();
            for entry in nodes {
                let same = numbering.same.as_deref() == self.entries[entry].name.as_deref();
                let number = if same {
                    *seen.entry(self.node_text(entry)).or_insert_with(|| {
                        next += 1;
                        next - 1
                    })
                } else {
                    next += 1;
                    next - 1
                };
                numbered.insert((name.clone(), entry), number);
            }
        }
        if let FamilyKind::Grammar(data) = &mut self.kind {
            data.ordinals = numbered;
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
        let own = self.entries[target].own;
        // A node's number or its name is no text of the body, and is not written (FBL 0.15 section 4.9).
        if let Some(numbering) = &slot.ordinal {
            let number = self.grammar_data().ordinals.get(&(numbering.clone(), target));
            return match number {
                Some(number) => SlotRead::new(Value::from(number.to_string().as_str()), Some(own), true, false)
                    .with_reason("A number the reading gives a node is not written in the file."),
                None => SlotRead::absent(),
            };
        }
        if slot.node_name {
            let name = self.entries[target].name.clone().unwrap_or_default();
            return SlotRead::new(Value::from(name.as_str()), Some(own), true, false)
                .with_reason("A node's name is not written in the file.");
        }
        if let Some(expand) = &slot.expand {
            let value = self.expanded(target, expand);
            let mut read = SlotRead::new(Value::from(value.as_str()), Some(own), true, true);
            read.wire = Some(value);
            return read;
        }
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
    /// 4.9): each node a slot reaches, or the pieces of it `split` cuts, with the escapes resolved per item, or *(FBL
    /// 0.12)* the structured value `items` reads from each node. The attribute is present with no item too: it holds
    /// the empty list.
    pub(super) fn grammar_read_many(&self, candidate: &Candidate, attribute: &AttributeBinding) -> SlotRead {
        let equals = |e: EntryId, p: &str, v: &str| self.path_equals(e, p, v);
        let mut items = Vec::new();
        let mut values = Vec::new();
        for slot in std::iter::once(&attribute.slot).chain(&attribute.gather) {
            let Some(node) = &slot.node else { continue };
            for (target, _) in selector::matches(&self.entries, candidate.entry, node, Some(&equals)) {
                if !slot.items.is_empty() {
                    let (item, value) = self.structured_item(candidate, target, slot, node);
                    items.push(item);
                    values.push(value);
                    continue;
                }
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
                    values.push(Value::from(read.as_str()));
                    items.push(WordItem {
                        value_span: piece_span,
                        word: Word {
                            text: read.clone(),
                            span: piece_span,
                            quoted: false,
                        },
                        value: read,
                        node: target,
                        parts: Vec::new(),
                    });
                }
            }
        }
        let mut read = SlotRead::new(Value::list(values), None, true, true);
        read.items = Some(std::rc::Rc::new(items));
        read
    }

    /// One item of a slot with `items` (FBL 0.12 section 5.2): a map of its item attributes, each the text of the node
    /// its slot reaches from `node`, typed as the specification types `attribute.item`, and each with its span.
    fn structured_item(&self, candidate: &Candidate, node: EntryId, slot: &Slot, path: &str) -> (WordItem, Value) {
        let own = self.entries[node].own;
        let type_of = |name: &str| {
            let rule = self.rule(candidate.rule?);
            let attribute = rule
                .attributes
                .iter()
                .find(|(_, a)| a.slot.node.as_deref() == Some(path))?;
            let item = format!("{}.{name}", attribute.0);
            self.options.attribute_type(&rule.type_name, &item)
        };
        let mut parts = Vec::new();
        let mut value = Map::default();
        for (name, item) in &slot.items {
            let Some((text, span)) = item
                .node
                .as_deref()
                .and_then(|n| self.grammar_reach(node, n))
                .and_then(|target| self.node_value(target, item))
            else {
                continue;
            };
            let read = match type_of(name) {
                Some(type_name) => scalar_conversion::from_text(&type_name, Value::from(text.as_str())),
                None => Value::from(text.as_str()),
            };
            value.insert(name.clone(), read);
            parts.push(ItemPart {
                name: name.clone(),
                text,
                span,
            });
        }
        let item = WordItem {
            value: self.node_text(node).to_owned(),
            value_span: own,
            word: Word {
                text: self.node_text(node).to_owned(),
                span: own,
                quoted: false,
            },
            node,
            parts,
        };
        (item, Value::map(value))
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
        if let (Some(binding), Value::List(items)) = (binding, value) {
            let items: Vec<String> = items.iter().map(|item| self.new_item(binding, item)).collect();
            return items.join(separator(&binding.slot));
        }
        self.slot_format(binding.map(|b| &b.slot), &grammar_value(binding, value))
    }

    /// A value's text as a slot writes it: a term by its `expand` (FBL 0.15 section 6.3), else by its `escape`, between
    /// its `requote` where it has one (FBL 0.11, 0.15 section 6.3).
    fn slot_format(&self, slot: Option<&Slot>, text: &str) -> String {
        if let Some(expand) = slot.and_then(|s| s.expand.as_ref()) {
            return self.compressed(text, expand);
        }
        match slot.and_then(|s| s.escape.as_ref()) {
            Some(escape) => {
                let written = escape.write(text);
                match escape.requote {
                    Some(quote) => format!("{quote}{written}{quote}"),
                    None => written,
                }
            }
            None => text.to_owned(),
        }
    }

    /// An item as a slot writes it (FBL 0.12 and 0.13 section 6.3): by the `create.emit` of a slot with `items`, each
    /// placeholder an item attribute's value; else as a value, escaped by the slot's `escape`.
    fn new_item(&self, binding: &AttributeBinding, item: &Value) -> String {
        if !binding.slot.items.is_empty()
            && let (Some(create), Value::Map(values)) = (&binding.create, item)
        {
            return values.iter().fold(create.emit.clone(), |text, (name, value)| {
                text.replace(&format!("{{{name}}}"), &new_text::plain(value, None))
            });
        }
        let text = new_text::plain(item, None);
        match &binding.slot.escape {
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
            if let Some(items) = &change.read.items {
                self.grammar_write_items(plan, element, change, items)?;
                continue;
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
            // What was read is written: the attribute's own slot, or the one of its `or` (FBL 0.15 section 4.9).
            let slot = match change.read.alternative {
                Some(at) => &change.binding.or[at],
                None => &change.binding.slot,
            };
            if change.is_empty && change.binding.empty.as_deref() == Some("remove") {
                let part = self.part_span(element.entry(), slot, span);
                if !self.reads_back(element, slot, (part, ""), None) {
                    return refuse(format!(
                        "The {} would not read back from the text without its {}.",
                        self.rule(change.rule).type_name,
                        change.attribute
                    ));
                }
                plan.add_span(Operation::RemoveKey, part, "");
                continue;
            }
            let value = grammar_value(Some(change.binding), &change.value);
            let written = self.slot_format(Some(slot), &value);
            // A string written in another quote replaces the whole node (FBL 0.15 section 6.3).
            let span = match slot.escape.as_ref().and_then(|e| e.requote) {
                Some(_) => slot
                    .node
                    .as_deref()
                    .and_then(|node| self.grammar_reach(element.entry(), node))
                    .map_or(span, |node| self.entries[node].own),
                None => span,
            };
            if !self.reads_back(element, slot, (span, &written), Some(&value)) {
                return refuse(format!(
                    "The new {} would not read back from the text.",
                    change.attribute
                ));
            }
            plan.add_span(Operation::ReplaceValue, span, written);
        }
        Ok(())
    }

    /// A set of a slot of several items (FBL 0.11 section 5.2, FBL 0.12 and 0.13 section 6.3): the difference between the
    /// items held and the new list, read again before it is made. The items of their longest common subsequence stay;
    /// between two of them, an item removed and one added are one change, and the rest are removals and additions.
    fn grammar_write_items(
        &self,
        plan: &mut Plan<'_>,
        element: &ReadElement,
        change: &SlotChange<'_>,
        items: &[WordItem],
    ) -> Planning {
        let slot = &change.binding.slot;
        let one;
        let values: &[Value] = match &change.value {
            Value::List(values) => values,
            Value::Null => &[],
            value => {
                one = [value.clone()];
                &one
            }
        };
        let wanted: Vec<String> = values.iter().map(|value| wanted_item(slot, value)).collect();
        let held_items: Vec<String> = items.iter().map(held_item).collect();
        let held: Vec<&str> = held_items.iter().map(String::as_str).collect();
        let kept = common_subsequence(&held, &wanted);
        let mut pending = Vec::new();
        let mut gone = Vec::new();
        let mut added = Vec::new();
        let (mut i, mut j) = (0, 0);
        for (anchor_held, anchor_wanted) in kept.into_iter().chain(std::iter::once((held.len(), wanted.len()))) {
            let paired = (anchor_held - i).min(anchor_wanted - j);
            for k in 0..paired {
                self.changed_item(&mut pending, &items[i + k], &values[j + k], slot);
            }
            gone.extend(i + paired..anchor_held);
            added.extend(&values[j + paired..anchor_wanted]);
            i = anchor_held + 1;
            j = anchor_wanted + 1;
        }
        self.removed_items(&mut pending, slot, items, &gone);
        self.added_items(&mut pending, element, change, items, &added)?;
        if !self.items_read_back(element, change, &pending, &wanted) {
            return refuse(format!(
                "The new {} would not read back from the text.",
                change.attribute
            ));
        }
        for (operation, span, text) in pending {
            plan.add_span(operation, span, text);
        }
        Ok(())
    }

    /// A changed item: a `replace-value` of its span, or of each value of a slot with `items` that changes.
    fn changed_item(&self, pending: &mut Vec<Pending>, item: &WordItem, value: &Value, slot: &Slot) {
        if slot.items.is_empty() {
            let text = new_text::plain(value, None);
            let written = match &slot.escape {
                Some(escape) => escape.write(&text),
                None => text,
            };
            pending.push((Operation::ReplaceValue, item.value_span, written));
            return;
        }
        let Value::Map(values) = value else { return };
        for part in &item.parts {
            let Some(new) = values.get(&part.name).map(|v| new_text::plain(v, None)) else {
                continue;
            };
            if comparable(&new) != comparable(&part.text) {
                pending.push((Operation::ReplaceValue, part.span, new));
            }
        }
    }

    /// The removals of a set. A node that is one item goes with the whitespace before it. Among the items of one node
    /// of a slot with `split`, or those of a slot with `items`, a run after an item that stays goes from that item's
    /// end to the run's end, and a run before one, from the run's start to that item's start; a node of a slot with
    /// `split` left with no item goes as a node does.
    fn removed_items(&self, pending: &mut Vec<Pending>, slot: &Slot, items: &[WordItem], gone: &[usize]) {
        if slot.split.is_none() && slot.items.is_empty() {
            for &item in gone {
                let own = self.entries[items[item].node].own;
                pending.push((Operation::RemoveKey, self.with_space_before(own), String::new()));
            }
            return;
        }
        let same_group = |a: usize, b: usize| slot.split.is_none() || items[a].node == items[b].node;
        let mut at = 0;
        while at < gone.len() {
            let first = gone[at];
            let mut last = first;
            while gone
                .get(at + 1)
                .is_some_and(|&next| next == last + 1 && same_group(next, first))
            {
                at += 1;
                last = gone[at];
            }
            at += 1;
            let before = first.checked_sub(1).filter(|&b| same_group(b, first));
            let after = Some(last + 1).filter(|&a| a < items.len() && same_group(a, first));
            let span = match (before, after) {
                (Some(before), _) => Span::new(items[before].value_span.end, items[last].value_span.end),
                (None, Some(after)) => Span::new(items[first].value_span.start, items[after].value_span.start),
                (None, None) if slot.split.is_some() => self.with_space_before(self.entries[items[first].node].own),
                (None, None) => Span::new(items[first].value_span.start, items[last].value_span.end),
            };
            pending.push((Operation::RemoveKey, span, String::new()));
        }
    }

    /// The additions of a set: after the slot's last item, by `separator` and the item where a node holds several, by
    /// the slot's `create` where each is a node. Where no node holds an item, a slot with `split` writes all of them
    /// into the value of the last node it reaches, or, reaching none, into the one node its `create` writes.
    fn added_items(
        &self,
        pending: &mut Vec<Pending>,
        element: &ReadElement,
        change: &SlotChange<'_>,
        items: &[WordItem],
        added: &[&Value],
    ) -> Planning {
        if added.is_empty() {
            return Ok(());
        }
        let binding = change.binding;
        let slot = &binding.slot;
        let cut = slot.split.is_some();
        if let Some(last) = items.last()
            && (cut || !slot.items.is_empty())
        {
            let at = Span::new(last.value_span.end, last.value_span.end);
            for value in added {
                let text = format!("{}{}", separator(slot), self.new_item(binding, value));
                pending.push((Operation::InsertKey, at, text));
            }
            return Ok(());
        }
        let joined = || {
            let written: Vec<String> = added.iter().map(|value| self.new_item(binding, value)).collect();
            written.join(separator(slot))
        };
        if cut {
            let equals = |e: EntryId, p: &str, v: &str| self.path_equals(e, p, v);
            let reached = slot.node.as_deref().map_or(Vec::new(), |node| {
                selector::matches(&self.entries, element.entry(), node, Some(&equals))
            });
            if let Some((_, span)) = reached.last().and_then(|(node, _)| self.node_value(*node, slot)) {
                pending.push((Operation::ReplaceValue, span, joined()));
                return Ok(());
            }
        }
        let cannot = || {
            refuse(format!(
                "The {} has no {} in this file to set.",
                self.rule(change.rule).type_name,
                change.attribute
            ))
        };
        let Some(create) = binding.create.as_ref() else {
            return cannot();
        };
        let offset = match items.last() {
            Some(last) => Some(self.entries[last.node].own.end),
            None => create.after.iter().find_map(|name| self.end_of_part(element, name)),
        };
        let Some(offset) = offset else {
            return cannot();
        };
        let at = Span::new(offset, offset);
        if cut {
            pending.push((Operation::InsertKey, at, create_text(&create.emit, &joined())));
            return Ok(());
        }
        for value in added {
            let written = self.new_item(binding, value);
            pending.push((Operation::InsertKey, at, create_text(&create.emit, &written)));
        }
        Ok(())
    }

    /// Whether the body with `pending` reads as it did, the changed entry holding the items `wanted` (FBL 0.9 section
    /// 4.9).
    fn items_read_back(
        &self,
        element: &ReadElement,
        change: &SlotChange<'_>,
        pending: &[Pending],
        wanted: &[String],
    ) -> bool {
        let candidates = self.candidates(element.rule);
        let Some(place) = candidates.iter().position(|c| c.entry == element.entry()) else {
            return false;
        };
        let mut ordered: Vec<&Pending> = pending.iter().collect();
        ordered.sort_by_key(|(_, span, _)| (span.start, usize::from(span.start != span.end)));
        let body = self.text.text(0, self.text.len());
        let mut changed = String::with_capacity(body.len() + 64);
        let mut position = 0;
        for (_, span, text) in ordered {
            if span.start < position {
                return false;
            }
            changed.push_str(&body[position..span.start]);
            changed.push_str(text);
            position = span.end;
        }
        changed.push_str(&body[position..]);
        let Some(after) = self.grammar_read_again(changed) else {
            return false;
        };
        let again = after.candidates(element.rule);
        if again.len() != candidates.len() {
            return false;
        }
        let read = after.grammar_read_many(&again[place], change.binding);
        read.items
            .is_some_and(|items| items.iter().map(held_item).eq(wanted.iter().cloned()))
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
        let value = grammar_value(Some(change.binding), &change.value);
        let slot = &change.binding.slot;
        if !self.reads_back(element, slot, (Span::new(offset, offset), &text), Some(&value)) {
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
        self.with_space_before(own)
    }

    /// Whether the body reads as it did with `start` to `end` replaced by `written`, and the changed entry holds
    /// `value` in the slot, or no longer has the slot without a value (FBL 0.9 section 4.9): nothing more fails to
    /// parse, its rule holds as many entries, and the entry at its place reads back.
    fn reads_back(
        &self,
        element: &ReadElement,
        slot: &Slot,
        (span, written): (Span, &str),
        value: Option<&str>,
    ) -> bool {
        let candidates = self.candidates(element.rule);
        let Some(place) = candidates.iter().position(|c| c.entry == element.entry()) else {
            return false;
        };
        let body = self.text.text(0, self.text.len());
        let changed = format!("{}{written}{}", &body[..span.start], &body[span.end..]);
        let Some(after) = self.grammar_read_again(changed) else {
            return false;
        };
        let again = after.candidates(element.rule);
        if again.len() != candidates.len() {
            return false;
        }
        let read = after.grammar_read(&again[place], slot);
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
        let continued = insert.continues.as_ref().and_then(|continues| {
            let statement = self.continued_statement(continues, request)?;
            Some((continues, statement))
        });
        let emit = continued
            .as_ref()
            .map(|(continues, _)| &continues.emit)
            .or(insert.emit.as_ref());
        let Some(emit) = emit else {
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
                            || matches!(value, Value::List(_))
                            || after.grammar_read(c, &binding.slot).value.as_str()
                                == Some(grammar_value(Some(binding), value).as_str())
                    })
                })
        };
        let check =
            |after: &Family, new: Span| after.candidates(request.rule).len() == count + 1 && holds_values(after, new);
        if let Some((continues, statement)) = continued {
            let Some(splices) = self.continue_statement(continues, statement, &text, &check) else {
                return refuse(format!("The new {} would not read back from the text.", rule.type_name));
            };
            for (operation, span, new) in splices {
                plan.add_span(operation, span, new);
            }
            return Ok(());
        }
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
        if !self.options.admits(&rule.origins) {
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
                    .and_then(|slot| slot.escape.as_ref())
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
        // A block holds statements on lines of their own; a container's other children are its arguments, and the
        // tokens that open and close it, which a grammar may name (FBL 0.11 section 4.9, `recover.nested`).
        let nested = self
            .binding
            .body
            .grammar
            .as_ref()
            .and_then(|g| g.recover.as_ref())
            .and_then(|r| r.nested.as_ref());
        let delimits = |name: Option<&String>| {
            nested.is_some_and(|nested| name.is_some_and(|name| *name == nested.open || *name == nested.close))
        };
        let statements = || {
            let start = self.entries[container].own.start;
            children.iter().copied().rfind(|&c| {
                self.entries[c].line_span.is_some()
                    && self.entries[c].own.start > start
                    && !delimits(self.entries[c].name.as_ref())
            })
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
        // FBL 0.16, section 6.3: an `emit`'s opening line feed is a blank line, left out where one would be a second.
        let last_blank = body.is_empty()
            || body.ends_with('\n') && body.split('\n').rev().nth(1).is_some_and(|last| last.trim().is_empty());
        let text = if last_blank {
            text.strip_prefix('\n').unwrap_or(text)
        } else {
            text
        };
        let line = self.indented(text, "", &newline);
        if body.is_empty() || body.ends_with('\n') {
            (end, format!("{line}{newline}"))
        } else {
            (end, format!("{newline}{line}"))
        }
    }

    /// The last statement of `continues.at`, in document order, whose slots of `same` read what the add gives their
    /// attributes (FBL 0.16 section 6.3).
    fn continued_statement(&self, continues: &ContinueSettings, request: &InsertRequest) -> Option<EntryId> {
        let rule = self.rule(request.rule);
        let wanted: Vec<(&Slot, String)> = continues
            .same
            .iter()
            .map(|(name, slot)| {
                let value = request.values.get(name).filter(|v| !v.is_null())?;
                Some((slot, grammar_value(rule.attribute(name), value)))
            })
            .collect::<Option<_>>()?;
        let root = self.grammar_data().root?;
        let equals = |e: EntryId, p: &str, v: &str| self.path_equals(e, p, v);
        selector::matches(&self.entries, root, &continues.at, Some(&equals))
            .into_iter()
            .map(|(entry, _)| entry)
            .filter(|&entry| {
                let candidate = Candidate {
                    rule: None,
                    block: None,
                    entry,
                    captures: Default::default(),
                };
                wanted.iter().all(|(slot, value)| {
                    let read = self.grammar_read(&candidate, slot);
                    read.present && read.value.as_str() == Some(value.as_str())
                })
            })
            .max_by_key(|&entry| self.entries[entry].own.end)
    }

    /// The splices that write `text` into `statement` (FBL 0.16 section 6.3): its last token replaced by the separator,
    /// and a line of its own after that token's line, none when the body would not read as `check` says.
    fn continue_statement(
        &self,
        continues: &ContinueSettings,
        statement: EntryId,
        text: &str,
        check: &dyn Fn(&Family, Span) -> bool,
    ) -> Option<Vec<(Operation, Span, String)>> {
        let own = self.entries[statement].own;
        let tokens = &self.grammar_data().tokens;
        let last = tokens[tokens.binary_search_by_key(&own.end, |t| t.end).ok()?];
        let lines = self.text.lines();
        let first_line = self.text.line_index_at(own.start);
        let last_line = self.text.line_index_at(last.start);
        let whitespace = |line: usize| -> String {
            self.text
                .text(lines[line].start, lines[line].content_end)
                .chars()
                .take_while(|c| c.is_whitespace())
                .collect()
        };
        let indent = if last_line > first_line {
            whitespace(first_line + 1)
        } else {
            format!("{}{}", whitespace(first_line), self.indentation(self.default_step()))
        };
        let line = lines[last_line];
        let newline = self.newline_at(line.end);
        let written = self.indented(text, &indent, &newline);
        let (at, new) = if line.end == self.text.len() && !self.text.text(0, line.end).ends_with('\n') {
            (line.end, format!("{newline}{indent}{written}"))
        } else {
            (line.end, format!("{indent}{written}{newline}"))
        };
        let body = self.text.text(0, self.text.len());
        let changed = format!(
            "{}{}{}{new}{}",
            &body[..last.start],
            continues.separator,
            &body[last.end..at],
            &body[at..]
        );
        let shift = continues.separator.len() as isize - (last.end - last.start) as isize;
        let start = (at as isize + shift) as usize;
        let after = self.grammar_read_again(changed)?;
        check(&after, Span::new(start, start + new.len())).then(|| {
            vec![
                (Operation::ReplaceSeparator, last, continues.separator.clone()),
                (Operation::InsertEntry, Span::new(at, at), new),
            ]
        })
    }

    pub(super) fn grammar_remove(&self, plan: &mut Plan<'_>, element: usize) -> Planning {
        let reading = plan.reading;
        let target = reading.element(element);
        let rule = self.rule(target.rule);
        let entry = target.entry();
        let settings = rule.remove.as_ref().filter(|remove| !remove.parts.is_empty());
        let Some(settings) = settings else {
            plan.add_span(Operation::RemoveEntry, self.entries[entry].removal_span(), "");
            return Ok(());
        };
        let shared = settings.parts.iter().find_map(|part| {
            let item = self.grammar_reach(entry, &part.item)?;
            let name = &self.entries[item].name;
            let list = self.entries[item].parent?;
            self.entries[list]
                .children
                .iter()
                .any(|&other| other != item && self.entries[other].name == *name)
                .then_some((item, part.separator.as_str()))
        });
        let span = match shared {
            Some((item, separator)) => self.part_removal(self.entries[item].own, separator),
            None => {
                let whole = settings
                    .whole
                    .as_deref()
                    .and_then(|whole| self.grammar_reach(entry, whole));
                self.whole_removal(self.entries[whole.unwrap_or(entry)].own)
            }
        };
        let body = self.text.text(0, self.text.len());
        let changed = format!("{}{}", &body[..span.start], &body[span.end..]);
        if self.grammar_read_again(changed).is_none() {
            return refuse(format!("The {} would not read back from the text.", rule.type_name));
        }
        plan.add_span(Operation::RemoveEntry, span, "");
        Ok(())
    }

    /// What a part of a list takes with it (FBL 0.16 section 6.3): one separator, the white space that binds it, and
    /// the lines nothing but white space would be left of.
    fn part_removal(&self, part: Span, separator: &str) -> Span {
        let body = self.text.text(0, self.text.len());
        let tokens = &self.grammar_data().tokens;
        let before = tokens
            .partition_point(|t| t.end <= part.start)
            .checked_sub(1)
            .map(|t| tokens[t]);
        let horizontal = |c: char| c == ' ' || c == '\t';
        let mut span = part;
        match before
            .filter(|t| self.text.text_of(*t) == separator && body[t.end..part.start].chars().all(char::is_whitespace))
        {
            Some(token) => span.start = token.start,
            None => {
                let rest = &body[part.end..];
                let at = part.end + (rest.len() - rest.trim_start_matches(horizontal).len());
                if body[at..].starts_with(separator) {
                    let past = at + separator.len();
                    let rest = &body[past..];
                    span.end = past + (rest.len() - rest.trim_start_matches(horizontal).len());
                }
            }
        }
        let lines = self.text.lines();
        let first = self.text.line_index_at(span.start);
        let last = self.text.line_index_at(span.start.max(span.end.saturating_sub(1)));
        let prefix = &body[lines[first].start..span.start];
        let suffix = &body[span.end..lines[last].content_end];
        if !prefix.trim().is_empty() && prefix.ends_with(char::is_whitespace) && suffix.starts_with(char::is_whitespace)
        {
            span.start -= prefix.len() - prefix.trim_end().len();
        }
        if prefix.trim().is_empty() && suffix.trim().is_empty() {
            return self.lines_removal(first, last);
        }
        span
    }

    /// A statement that goes whole (FBL 0.16 section 6.3): the lines it stands on, with the comment that follows it there
    /// and no comment above it, or its own span when something else stands on those lines.
    fn whole_removal(&self, own: Span) -> Span {
        if !self.grammar_only_trivia_before(own.start) {
            return own;
        }
        let Some(last) = self.grammar_last_trivia_line(own.end) else {
            return own;
        };
        self.lines_removal(self.text.line_index_at(own.start), last)
    }

    /// Whole lines with their endings: a last line without one takes the ending before it, so that the body still ends
    /// without one.
    fn lines_removal(&self, first: usize, last: usize) -> Span {
        let lines = self.text.lines();
        let end = lines[last].end;
        let unterminated = end == self.text.len() && !self.text.text(0, end).ends_with('\n');
        if unterminated && first > 0 {
            return Span::new(lines[first - 1].content_end, end);
        }
        Span::new(lines[first].start, end)
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

/// A value as a slot holds it on the wire: through the binding's `map`, else as text (FBL section 5.2).
fn grammar_value(binding: Option<&AttributeBinding>, value: &Value) -> String {
    binding
        .and_then(|b| new_text::wire(b, value, None))
        .unwrap_or_else(|| new_text::plain(value, binding))
}

/// The text written between two items (FBL 0.12 section 5.2).
fn separator(slot: &Slot) -> &str {
    slot.separator.as_deref().unwrap_or(", ")
}

/// A number as a set compares it, however it is spelled; any other text as it is.
fn comparable(text: &str) -> String {
    match text.parse::<f64>() {
        Ok(number) if number.is_finite() => new_text::number(number, None),
        _ => text.to_owned(),
    }
}

/// An item as a set compares it: its value, or those of its item attributes in the slot's order.
fn held_item(item: &WordItem) -> String {
    if item.parts.is_empty() {
        return item.value.clone();
    }
    let values: Vec<String> = item.parts.iter().map(|part| comparable(&part.text)).collect();
    values.join("\u{1f}")
}

/// A new item as a set compares it with [`held_item`].
fn wanted_item(slot: &Slot, value: &Value) -> String {
    let Value::Map(values) = value else {
        return new_text::plain(value, None);
    };
    let values: Vec<String> = slot
        .items
        .iter()
        .filter_map(|(name, _)| values.get(name))
        .map(|value| comparable(&new_text::plain(value, None)))
        .collect();
    values.join("\u{1f}")
}

/// A part's `create.emit` with its value (FBL 0.11 section 4.9).
fn create_text(emit: &str, value: &str) -> String {
    emit.replace("{value}", value)
}

#[cfg(test)]
#[path = "grammar_tests.rs"]
mod tests;
