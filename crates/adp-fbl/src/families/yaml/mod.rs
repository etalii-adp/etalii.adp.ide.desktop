//! The `yaml` family (FBL section 4.3; the standalone host's `Files/Yaml/YamlFamily`): the parser reads the first
//! document's structure and spans, and writing keeps each replaced scalar's style and the body's indentation (FBL
//! section 6.3).

pub(crate) mod flow;
pub(crate) mod parser;
pub(crate) mod scalars;

use adp_cel::Value;

use crate::documents::types::{AttributeBinding, InsertSettings};
use crate::families::tree::{TreeEntry, ValueId, ValueKind, ValueStyle};
use crate::families::{Entry, EntryId, EntryKind, Family};
use crate::model::{FindingSeverity, codes};
use crate::planning::new_text;
use crate::planning::plan::{Plan, Planning, refuse};
use crate::rules::{Captures, InsertRequest, Node, ReadElement, SlotChange, SlotRead};
use crate::splice::Operation;
use crate::text::Span;

impl Family {
    pub(super) fn yaml_parse(&mut self) {
        let parsed = match parser::parse(&self.text) {
            Ok(parsed) => parsed,
            Err(error) => {
                self.unreadable = Some((error.offset, error.message));
                return;
            }
        };
        self.entries = parsed.entries;
        let root_value = parsed.root;
        let root_span = parsed.values[root_value].span;
        match &mut self.kind {
            super::FamilyKind::Yaml(data) => data.values = parsed.values,
            _ => unreachable!("a yaml family"),
        }
        self.entries.push(Entry {
            own: root_span,
            line_span: None,
            parent: None,
            children: Vec::new(),
            name: None,
            indent: 0,
            kind: EntryKind::Tree(TreeEntry {
                is_root: true,
                key_span: None,
                value: root_value,
                separator: None,
            }),
        });
        let root = self.entries.len() - 1;
        if let super::FamilyKind::Yaml(data) = &mut self.kind {
            data.root = Some(root);
        }
        self.index(root);
        self.leaves = parsed.leaves;
        for (name, span) in parsed.duplicates {
            self.report(
                codes::DUPLICATE_KEY,
                FindingSeverity::Warning,
                format!("The key '{name}' appears again in this mapping; only the first is read."),
                Some(span),
            );
        }
        for i in 0..self.order.len() {
            let id = self.order[i];
            if !self.entries[id].tree().is_root {
                self.entries[id].line_span = self.yaml_line_span_of(id);
            }
        }
    }

    /// Between leaves there is only whitespace, comments, indicators, anchors, tags and document markers.
    pub(super) fn yaml_is_trivia(&self, gap: Span) -> bool {
        let bytes = self.text.bytes();
        let mut i = gap.start;
        while i < gap.end {
            match bytes[i] {
                b' ' | b'\t' | b'\r' | b'\n' | b':' | b'-' | b'?' | b',' | b'.' => {}
                b'#' | b'%' => {
                    while i < gap.end && !matches!(bytes[i], b'\r' | b'\n') {
                        i += 1;
                    }
                    continue;
                }
                b'&' | b'!' => {
                    while i < gap.end && !matches!(bytes[i], b' ' | b'\t' | b'\r' | b'\n') {
                        i += 1;
                    }
                    continue;
                }
                _ => return false,
            }
            i += 1;
        }
        true
    }

    /// An entry's line span (FBL section 4.1.1) when it starts and ends its lines: from its first line, extended
    /// over the comment lines directly above it at its indentation, to the end of its last line's ending, a
    /// trailing comment included.
    fn yaml_line_span_of(&self, entry: EntryId) -> Option<Span> {
        let e = &self.entries[entry];
        if !self.only_whitespace_before(e.own.start) || !self.only_trivia_after(e.own.end, Some(b'#')) {
            return None;
        }
        let lines = self.text.lines();
        let first = self.text.line_index_at(e.own.start);
        let last = self.text.line_index_at(e.own.start.max(e.own.end.saturating_sub(1)));
        let mut start = lines[first].start;
        for above in (0..first).rev() {
            let line = lines[above];
            let mut p = line.start;
            while p < line.content_end && self.text.bytes()[p] == b' ' {
                p += 1;
            }
            if p >= line.content_end || self.text.bytes()[p] != b'#' || p - line.start != e.indent {
                break;
            }
            start = line.start;
        }
        Some(Span::new(start, lines[last].end))
    }

    fn line_end_after(&self, entry: EntryId) -> usize {
        let e = &self.entries[entry];
        e.line_span.map(|l| l.end).unwrap_or_else(|| {
            self.text.lines()[self.text.line_index_at(e.own.start.max(e.own.end.saturating_sub(1)))].end
        })
    }

    // ---- new text (FBL section 6.3) ----

    pub(super) fn yaml_format(&self, read: &SlotRead, binding: Option<&AttributeBinding>, value: &Value) -> String {
        let member = match read.node {
            Node::TreeMember(m) => Some(m),
            _ => None,
        };
        let old = member.map(|m| self.entries[m].tree().value);
        let wire = binding.and_then(|b| new_text::wire(b, value, read.wire.as_deref()));
        let written_value = wire.clone().map(Value::from).unwrap_or_else(|| value.clone());
        let indent = member.map(|m| self.entries[m].indent).unwrap_or(0);
        let mut written = self.scalar(&written_value, old, binding, indent);
        // A number or a bool read from a quoted scalar (FBL 0.5 quoted: convert) keeps its quotes.
        if let Some(old) = old
            && binding.is_some_and(|b| b.quoted.as_deref() == Some("convert"))
            && matches!(self.tree_value(old).style, ValueStyle::Single | ValueStyle::Double)
            && matches!(written_value, Value::Bool(_) | Value::Int(_) | Value::Double(_))
        {
            written = if self.tree_value(old).style == ValueStyle::Single {
                scalars::single_quoted(&written)
            } else {
                scalars::double_quoted(&written)
            };
        }
        if old.is_some_and(|o| self.tree_value(o).style == ValueStyle::Empty) {
            format!(" {written}")
        } else {
            written
        }
    }

    fn scalar(
        &self,
        value: &Value,
        old: Option<ValueId>,
        binding: Option<&AttributeBinding>,
        key_indent: usize,
    ) -> String {
        match value {
            Value::Null => "null".into(),
            Value::Bool(b) => if *b { "true" } else { "false" }.into(),
            Value::String(s) => self.yaml_string(s, old, binding, key_indent),
            Value::List(list) => {
                let items: Vec<String> = list
                    .iter()
                    .map(|v| match v {
                        Value::String(item) => flow_item(item),
                        other => self.scalar(other, None, binding, key_indent),
                    })
                    .collect();
                format!("[{}]", items.join(", "))
            }
            Value::Int(_) | Value::Double(_) => {
                new_text::number(value.as_number().unwrap_or(0.0), binding.and_then(|b| b.decimals))
            }
            other => self.yaml_string(&new_text::plain(other, binding), old, binding, key_indent),
        }
    }

    fn yaml_string(
        &self,
        value: &str,
        old: Option<ValueId>,
        binding: Option<&AttributeBinding>,
        key_indent: usize,
    ) -> String {
        let keep_precision = binding.is_some_and(|b| b.keep_time_precision);
        let time_typed = keep_precision || self.is_time_typed(binding);
        let mut value = value.to_owned();
        if keep_precision
            && let Some(old) = old.map(|o| self.tree_value(o))
            && old.kind == ValueKind::Scalar
            && !old.text.is_empty()
            && let Some(kept) = new_text::keep_precision(&old.text, &value)
        {
            value = kept;
        }
        let multiline = value.contains('\n') || value.contains('\r');
        match old.map(|o| self.tree_value(o).style) {
            Some(ValueStyle::Plain) if scalars::is_plain_writable(&value, time_typed) => return value,
            Some(ValueStyle::Single) if !multiline => return scalars::single_quoted(&value),
            Some(ValueStyle::Double) => return scalars::double_quoted(&value),
            Some(ValueStyle::Literal) if multiline => return self.literal(&value, key_indent),
            _ => {}
        }
        match binding.and_then(|b| b.style.as_deref()) {
            Some("single") if !multiline => return scalars::single_quoted(&value),
            Some("double") => return scalars::double_quoted(&value),
            Some("literal") if multiline => return self.literal(&value, key_indent),
            Some("plain")
                if scalars::is_plain_writable(&value, time_typed)
                    || (self.is_text_read(binding) && scalars::is_plain_syntax(&value)) =>
            {
                return value;
            }
            _ => {}
        }
        if scalars::is_plain_safe(&value, time_typed) {
            return value;
        }
        if self.binding.text.quote == "single" && !multiline {
            scalars::single_quoted(&value)
        } else {
            scalars::double_quoted(&value)
        }
    }

    /// A multi-line string written `|-`, its lines one step deeper than its key (FBL section 6.3).
    fn literal(&self, value: &str, key_indent: usize) -> String {
        let newline = self
            .text
            .dominant_ending()
            .unwrap_or(&self.binding.text.newline)
            .to_owned();
        let indent = self.indentation(key_indent + self.step());
        let normalised = value.replace("\r\n", "\n");
        let mut out = "|-".to_owned();
        for l in normalised.split('\n') {
            out.push_str(&newline);
            if !l.is_empty() {
                out.push_str(&indent);
                out.push_str(l);
            }
        }
        out
    }

    // ---- writing ----

    pub(super) fn yaml_write(
        &self,
        plan: &mut Plan<'_>,
        element: &ReadElement,
        changes: &[SlotChange<'_>],
    ) -> Planning {
        for change in changes {
            let read = &change.read;
            let member = match read.node {
                Node::TreeMember(m) => Some(m),
                _ => None,
            };
            if change.is_empty && change.binding.empty.as_deref() == Some("remove") {
                if let Some(member) = member {
                    // FBL 0.4, section 5.2: the levels the key leaves empty go with it, up to the entry itself.
                    if change.binding.create.as_ref().is_some_and(|c| c.at_end)
                        && let Some(reached) = &change.binding.child
                    {
                        let most = reached.trim_matches('/').split('/').count();
                        self.remove_emptied_levels(plan, member, most, &|e| e == member);
                    }
                    let m = &self.entries[member];
                    plan.add_span(Operation::RemoveKey, m.line_span.unwrap_or(m.own), "");
                }
                continue;
            }
            if read.present
                && let Some(member) = member
            {
                let written = self.yaml_format(read, Some(change.binding), &change.value);
                let span = self.value_of(member).span;
                if written != self.text.text_of(span) {
                    plan.add_span(Operation::ReplaceValue, span, written);
                }
                continue;
            }
            let mapping = match read.node {
                Node::TreeMapping(v) => Some(v),
                _ => self.mapping(element.entry(), change.binding.child.as_deref()),
            };
            if mapping.is_none()
                && change.binding.create.as_ref().is_some_and(|c| c.at_end)
                && let (Some(child), Some(created)) = (&change.binding.child, &change.binding.key)
            {
                // FBL 0.4, section 5.2: every missing level of the child mapping, then the key, at one offset.
                let segments: Vec<&str> = child.trim_matches('/').split('/').collect();
                let Some((at, levels, inside)) = self.missing_levels(element.entry(), &segments) else {
                    return refuse(format!(
                        "The {} file has no place to create {child} in.",
                        self.binding.name
                    ));
                };
                for level in levels {
                    plan.add(Operation::EnsureContainer, at, at, self.yaml_new_line(at, &[level]));
                }
                let line = format!(
                    "{}{created}: {}",
                    self.indentation(inside),
                    self.scalar(&change.value, None, Some(change.binding), inside)
                );
                plan.add(Operation::InsertKey, at, at, self.yaml_new_line(at, &[line]));
                continue;
            }
            let (Some(mapping), Some(key)) = (mapping, &change.binding.key) else {
                return refuse(format!(
                    "The {} file has no mapping to write '{}' in.",
                    self.binding.name,
                    change.binding.key.as_deref().unwrap_or_default()
                ));
            };
            if self.tree_value(mapping).kind != ValueKind::Mapping {
                return refuse(format!(
                    "The {} file has no mapping to write '{key}' in.",
                    self.binding.name
                ));
            }
            self.insert_key(plan, mapping, key, change);
        }
        Ok(())
    }

    /// A key the entry lacks goes at its place in the rule's key order: after the nearest present key before it,
    /// else before the nearest after it.
    fn insert_key(&self, plan: &mut Plan<'_>, mapping: ValueId, key: &str, change: &SlotChange<'_>) {
        let order = key_order(self.rule(change.rule), change.binding.child.as_deref());
        let index = order.iter().position(|k| k == key);
        let entries = &self.tree_value(mapping).entries;
        let mut before = None;
        if let Some(index) = index {
            for k in order[..index].iter().rev() {
                before = self.member(mapping, k);
                if before.is_some() {
                    break;
                }
            }
        }
        let offset = if let Some(before) = before {
            self.line_end_after(before)
        } else if change.binding.create.as_ref().is_some_and(|c| c.at_end) {
            // FBL 0.4, section 5.2: with no key of the rule before it, the mapping's first key.
            let first = &self.entries[entries[0]];
            first.line_span.map(|l| l.start).unwrap_or(first.own.start)
        } else {
            let mut after = None;
            let from = index.map(|i| i + 1).unwrap_or(0);
            for k in order.iter().skip(from) {
                after = self.member(mapping, k);
                if after.is_some() {
                    break;
                }
            }
            match after.and_then(|a| self.entries[a].line_span) {
                Some(line) => line.start,
                None => self.line_end_after(*entries.last().expect("a mapping with members")),
            }
        };
        let indent = self.entries[entries[0]].indent;
        let line = format!(
            "{}{key}: {}",
            self.indentation(indent),
            self.scalar(&change.value, None, Some(change.binding), indent)
        );
        plan.add(
            Operation::InsertKey,
            offset,
            offset,
            self.yaml_new_line(offset, &[line]),
        );
    }

    /// The text of new lines inserted at `offset`, a line start: each line with the ending of the line the
    /// insertion point is on. At the end of a body without a final newline the break goes before the text and none
    /// after it (FBL section 6.3).
    fn yaml_new_line(&self, offset: usize, lines: &[String]) -> String {
        let newline = if offset > 0 {
            self.newline_at(offset - 1)
        } else {
            self.newline_at(0)
        };
        let text_lines = self.text.lines();
        if offset == self.text.len() && !self.text.is_empty() && text_lines.last().is_some_and(|l| l.ending.is_empty())
        {
            return lines.iter().map(|l| format!("{newline}{l}")).collect();
        }
        lines.iter().map(|l| format!("{l}{newline}")).collect()
    }

    /// The columns between a key and the `-` of its sequence's items, as the body shows it, else
    /// `text.sequenceIndent`.
    fn sequence_offset(&self) -> usize {
        for &id in &self.order {
            let entry = &self.entries[id];
            let tree = entry.tree();
            let value = self.tree_value(tree.value);
            if tree.key_span.is_some()
                && entry.line_span.is_some()
                && value.kind == ValueKind::Sequence
                && !value.entries.is_empty()
            {
                return self.entries[value.entries[0]].indent.saturating_sub(entry.indent);
            }
        }
        if self.binding.text.sequence_flush {
            0
        } else {
            self.step()
        }
    }

    pub(super) fn yaml_insert(&self, plan: &mut Plan<'_>, request: &InsertRequest) -> Planning {
        let reading = plan.reading;
        let rule = self.rule(request.rule);
        let insert = rule.insert.as_ref().expect("an insertable rule");
        // FBL section 5's place: {before: key} is not implemented here yet.
        if insert.place == "before" {
            return refuse(format!(
                "A {} body cannot place a new entry '{}'.",
                self.family_name(),
                insert.place
            ));
        }
        let parent = match request.parent {
            Some(p) => Some(reading.element(p).entry()),
            None => self.end_parent(plan, request),
        };
        let captures: Captures = match request.parent {
            Some(p) => reading.element(p).candidate.captures.clone(),
            None => parent
                .and_then(|p| reading.element_of(p))
                .map(|e| reading.element(e).candidate.captures.clone())
                .unwrap_or_else(|| Family::captures_of(plan)),
        };
        let selector = insert
            .container
            .clone()
            .unwrap_or_else(|| container_of(rule.at.as_deref()));
        let container = self.container(&selector, parent, &captures);
        let keys = self.item_lines(plan, request);
        if keys.is_empty() {
            return refuse(format!("A new {} would have no keys to write.", rule.type_name));
        }
        let Some(container) = container else {
            return self.ensure_container(plan, insert, &selector, parent, &keys);
        };
        let value = self.value_of(container);
        if value.kind == ValueKind::Sequence {
            let siblings = &value.entries;
            let previous = match insert.place.as_str() {
                "start" => None,
                "after-last" => siblings
                    .iter()
                    .copied()
                    .rfind(|s| reading.claimed_by(*s) == Some(rule.name.as_str()))
                    .or_else(|| siblings.last().copied()),
                _ => siblings.last().copied(),
            };
            let model = previous.unwrap_or(siblings[0]);
            let dash = self.entries[model].indent;
            let model_value = self.value_of(model);
            let key_indent = if model_value.kind == ValueKind::Mapping && !model_value.entries.is_empty() {
                self.entries[model_value.entries[0]].indent
            } else {
                dash + 2
            };
            let offset = match previous {
                None => {
                    let first = &self.entries[siblings[0]];
                    first.line_span.map(|l| l.start).unwrap_or(first.own.start)
                }
                Some(previous) => self.line_end_after(previous),
            };
            plan.add(
                Operation::InsertEntry,
                offset,
                offset,
                self.yaml_new_line(offset, &self.item(&keys, dash, key_indent)),
            );
            return Ok(());
        }
        if value.style == ValueStyle::Empty {
            let offset = self.line_end_after(container);
            let dash = self.entries[container].indent + self.sequence_offset();
            plan.add(
                Operation::InsertEntry,
                offset,
                offset,
                self.yaml_new_line(offset, &self.item(&keys, dash, dash + 2)),
            );
            return Ok(());
        }
        if value.style == ValueStyle::FlowSequence
            && matches!(&value.flow, Some(Value::List(l)) if l.is_empty())
            && let Some(key_span) = self.entries[container].tree().key_span
        {
            // An empty flow sequence ("elements: []", the template's) becomes a block sequence: the flow value goes
            // and the item follows on its own line.
            let colon = self.text.text(key_span.end, value.span.start).find(':').unwrap_or(0);
            plan.add(Operation::ReplaceValue, key_span.end + colon + 1, value.span.end, "");
            let offset = self.line_end_after(container);
            let dash = self.entries[container].indent + self.sequence_offset();
            plan.add(
                Operation::InsertEntry,
                offset,
                offset,
                self.yaml_new_line(offset, &self.item(&keys, dash, dash + 2)),
            );
            return Ok(());
        }
        refuse(format!(
            "The {selector} of this file is not a list a {} can be added to.",
            rule.type_name
        ))
    }

    /// The entry a relation is written inside when one of its ends is its enclosing entry (a Databricks dependency
    /// inside its task).
    fn end_parent(&self, plan: &Plan<'_>, request: &InsertRequest) -> Option<EntryId> {
        let rule = self.rule(request.rule);
        if rule.target.as_ref().is_some_and(|t| t.parent.is_some()) {
            return request.target.map(|t| plan.reading.element(t).entry());
        }
        if rule.source.as_ref().is_some_and(|s| s.parent.is_some()) {
            return request.source.map(|s| plan.reading.element(s).entry());
        }
        None
    }

    fn ensure_container(
        &self,
        plan: &mut Plan<'_>,
        insert: &InsertSettings,
        selector: &str,
        parent: Option<EntryId>,
        keys: &[String],
    ) -> Planning {
        let Some(create) = &insert.create else {
            return refuse(format!(
                "The file has no {} to add to.",
                selector.trim_start_matches('/')
            ));
        };
        let trimmed = selector.trim_end_matches('/');
        let name = &trimmed[trimmed.rfind('/').map(|i| i + 1).unwrap_or(0)..];
        let absolute = selector.starts_with('/');
        let mut owner = if absolute { self.tree_root() } else { parent };
        if absolute && selector.trim_matches('/').contains('/') {
            owner = self.container(&container_of(Some(selector)), parent, &Captures::default());
        }
        let mapping = owner.map(|o| self.entries[o].tree().value);
        let mapping_with_members = mapping.filter(|m| {
            let v = self.tree_value(*m);
            v.kind == ValueKind::Mapping && !v.entries.is_empty()
        });
        let member_of = |key: &Option<String>| -> Option<EntryId> {
            let key = key.as_ref()?;
            mapping.and_then(|m| self.member(m, key))
        };
        let (offset, indent) = match create.at.as_str() {
            "end" => {
                // FBL 0.4, section 6.2, Missing levels: outermost first, then the entry, all at one offset.
                let from = if absolute { self.tree_root() } else { parent };
                let segments: Vec<&str> = selector.trim_matches('/').split('/').collect();
                let Some(from) = from else {
                    return refuse(format!("The file has no place to create {name} in."));
                };
                let Some((at, mut levels, inside)) = self.missing_levels(from, &segments) else {
                    return refuse(format!("The file has no place to create {name} in."));
                };
                if let Some(own) = &create.text
                    && let Some(last) = levels.last_mut()
                {
                    *last = own.clone();
                }
                for level in levels {
                    plan.add(Operation::EnsureContainer, at, at, self.yaml_new_line(at, &[level]));
                }
                let first = (inside as i64 - self.step() as i64 + self.sequence_offset() as i64).max(0) as usize;
                plan.add(
                    Operation::InsertEntry,
                    at,
                    at,
                    self.yaml_new_line(at, &self.item(keys, first, first + 2)),
                );
                return Ok(());
            }
            "end-of-document" => (
                self.text.len(),
                mapping_with_members
                    .map(|m| self.entries[self.tree_value(m).entries[0]].indent)
                    .unwrap_or(0),
            ),
            "after" if member_of(&create.argument).is_some() => {
                let after = member_of(&create.argument).expect("a member");
                (self.line_end_after(after), self.entries[after].indent)
            }
            "before" if member_of(&create.argument).is_some() => {
                let before = &self.entries[member_of(&create.argument).expect("a member")];
                (
                    before.line_span.map(|l| l.start).unwrap_or(before.own.start),
                    before.indent,
                )
            }
            // Before a key the file does not have: at its end, as after-last falls back to end.
            "before" if mapping_with_members.is_some() => (
                self.text.len(),
                self.entries[self.tree_value(mapping_with_members.expect("a mapping")).entries[0]].indent,
            ),
            "under" => {
                let under = create
                    .argument
                    .as_ref()
                    .and_then(|a| self.container(a, parent, &Captures::default()))
                    .map(|u| self.value_of(u))
                    .filter(|v| v.kind == ValueKind::Mapping && !v.entries.is_empty());
                let Some(under) = under else {
                    return refuse(format!("The file has no place to create {name} in."));
                };
                (
                    self.line_end_after(*under.entries.last().expect("members")),
                    self.entries[under.entries[0]].indent,
                )
            }
            _ => return refuse(format!("The file has no place to create {name} in.")),
        };
        let container_text = create
            .text
            .clone()
            .unwrap_or_else(|| format!("{}{name}:", self.indentation(indent)));
        plan.add(
            Operation::EnsureContainer,
            offset,
            offset,
            self.yaml_new_line(offset, &[container_text]),
        );
        let dash = indent + self.sequence_offset();
        plan.add(
            Operation::InsertEntry,
            offset,
            offset,
            self.yaml_new_line(offset, &self.item(keys, dash, dash + 2)),
        );
        Ok(())
    }

    /// The levels of `segments` that `from` does not reach, as the key lines that create them, outermost first;
    /// where they go, which is the end of the innermost level the body has; and the indentation of what the
    /// innermost level holds (FBL 0.4, section 6.2). None when a missing level is named by `*` or a capture, which
    /// gives no key to write, or when the level to write in is not a block mapping.
    fn missing_levels(&self, from: EntryId, segments: &[&str]) -> Option<(usize, Vec<String>, usize)> {
        let mut level = from;
        let mut reached = 0;
        while reached < segments.len() {
            let value = self.entries[level].tree().value;
            if self.tree_value(value).kind != ValueKind::Mapping {
                break;
            }
            let Some(member) = self.member(value, segments[reached]) else {
                break;
            };
            level = member;
            reached += 1;
        }
        let is_root = self.entries[level].tree().is_root;
        let value = self.value_of(level);
        let (offset, mut indent) = if value.kind == ValueKind::Mapping && !value.entries.is_empty() {
            (
                if is_root {
                    self.text.len()
                } else {
                    self.line_end_after(*value.entries.last().expect("members"))
                },
                self.entries[value.entries[0]].indent,
            )
        } else if value.style == ValueStyle::Empty {
            (
                if is_root {
                    self.text.len()
                } else {
                    self.line_end_after(level)
                },
                if is_root {
                    0
                } else {
                    self.entries[level].indent + self.step()
                },
            )
        } else {
            return None;
        };
        let mut lines = Vec::new();
        for segment in &segments[reached..] {
            if *segment == "*" || segment.starts_with('{') {
                return None;
            }
            lines.push(format!("{}{segment}:", self.indentation(indent)));
            indent += self.step();
        }
        Some((offset, lines, indent))
    }

    /// Removes the heads of the levels above `start` that its removal leaves empty, at most `most` of them and
    /// never the root, outermost first (FBL 0.4, section 6.2). A level is left empty when everything in it is
    /// `going` or is such a level itself.
    fn remove_emptied_levels(&self, plan: &mut Plan<'_>, start: EntryId, most: usize, going: &dyn Fn(EntryId) -> bool) {
        let mut heads: Vec<Span> = Vec::new();
        let mut emptied: Option<EntryId> = None;
        let mut level = self.entries[start].parent;
        while heads.len() < most
            && let Some(l) = level
            && !self.entries[l].tree().is_root
            && let Some(line) = self.entries[l].line_span
            && self
                .value_of(l)
                .entries
                .iter()
                .all(|e| Some(*e) == emptied || going(*e))
        {
            let first = &self.entries[self.value_of(l).entries[0]];
            heads.insert(0, line.with_end(first.line_span.unwrap_or(first.own).start));
            emptied = Some(l);
            level = self.entries[l].parent;
        }
        for head in heads {
            if !plan.touches(head) {
                plan.add_span(Operation::RemoveContainer, head, "");
            }
        }
    }

    fn item(&self, lines: &[String], dash: usize, key_indent: usize) -> Vec<String> {
        lines
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let line = self.nested(line, key_indent);
                if i == 0 {
                    format!(
                        "{}-{}{line}",
                        self.indentation(dash),
                        " ".repeat((key_indent as i64 - dash as i64 - 1).max(1) as usize)
                    )
                } else {
                    format!("{}{line}", self.indentation(key_indent))
                }
            })
            .collect()
    }

    /// A key line whose value runs over several lines (a literal block, written relative to column 0) with its
    /// continuation lines moved under the key's indentation; blank ones stay empty.
    fn nested(&self, line: &str, key_indent: usize) -> String {
        if !line.contains('\n') {
            return line.to_owned();
        }
        let mut parts: Vec<String> = line.split('\n').map(str::to_owned).collect();
        for part in parts.iter_mut().skip(1) {
            if !part.trim_end_matches('\r').is_empty() {
                *part = format!("{}{part}", self.indentation(key_indent));
            }
        }
        parts.join("\n")
    }

    /// The key lines of a new item in `insert.keys` order, then the skeleton's lines.
    fn item_lines(&self, plan: &Plan<'_>, request: &InsertRequest) -> Vec<String> {
        let rule = self.rule(request.rule);
        let insert = rule.insert.as_ref().expect("an insertable rule");
        let mut lines = Vec::new();
        for key in key_order(rule, None) {
            if !insert.keys.is_empty() && !insert.keys.contains(&key) {
                continue;
            }
            if let Some(value) = self.yaml_wire_value(plan, request, &key) {
                lines.push(format!("{key}: {value}"));
            }
        }
        if let Some(skeleton) = &insert.skeleton {
            let rendered = new_text::render(skeleton, &|name| match request.values.get(name) {
                Some(v) if !v.is_null() => Some(new_text::plain(v, rule.attribute(name))),
                _ if name == "id" => request.id.clone(),
                _ => None,
            });
            let rendered = rendered.replace("\r\n", "\n");
            lines.extend(rendered.trim_end_matches('\n').split('\n').map(str::to_owned));
        }
        lines
    }

    fn yaml_wire_value(&self, plan: &Plan<'_>, request: &InsertRequest, key: &str) -> Option<String> {
        let rule = self.rule(request.rule);
        let reading = plan.reading;
        if rule
            .id
            .as_ref()
            .and_then(|i| i.from.as_ref())
            .and_then(|f| f.key.as_deref())
            == Some(key)
            && let Some(id) = &request.id
        {
            return Some(self.scalar(&Value::from(id.as_str()), None, None, 0));
        }
        if rule.source.as_ref().and_then(|s| s.key.as_deref()) == Some(key)
            && let Some(source) = request.source
        {
            return Some(self.scalar(&Value::from(reading.element(source).key.as_str()), None, None, 0));
        }
        if rule.target.as_ref().and_then(|t| t.key.as_deref()) == Some(key)
            && let Some(target) = request.target
        {
            return Some(self.scalar(&Value::from(reading.element(target).key.as_str()), None, None, 0));
        }
        for (name, binding) in &rule.attributes {
            if binding.key.as_deref() != Some(key) || binding.child.is_some() || binding.is_computed() {
                continue;
            }
            let Some(value) = request.values.get(name) else {
                continue;
            };
            if new_text::is_empty(value) && binding.empty.as_deref() != Some("keep") {
                continue;
            }
            let wire = new_text::wire(binding, value, None).map(Value::from);
            return Some(self.scalar(wire.as_ref().unwrap_or(value), None, Some(binding), 0));
        }
        None
    }

    pub(super) fn yaml_remove(&self, plan: &mut Plan<'_>, element: usize, removed: &[usize]) -> Planning {
        let reading = plan.reading;
        let read_element = reading.element(element);
        let entry = read_element.entry();
        if self.entries[entry].tree().is_root {
            return refuse(format!("The whole {} file cannot be removed.", self.binding.name));
        }
        let rule = self.rule(read_element.rule);
        let span = self.entries[entry].line_span.unwrap_or(self.entries[entry].own);
        if let Some(remove) = rule.remove.as_ref().filter(|r| r.remove_container_when_empty) {
            // The container alone, or with remove-empty-levels every level of its selector that a key names,
            // counted from the container up to the first level a * or a capture names.
            let mut levels = 1;
            if remove.remove_empty_levels {
                let selector = rule
                    .insert
                    .as_ref()
                    .and_then(|i| i.container.clone())
                    .unwrap_or_else(|| container_of(rule.at.as_deref()));
                levels = selector
                    .trim_matches('/')
                    .split('/')
                    .rev()
                    .take_while(|s| !s.is_empty() && *s != "*" && !s.starts_with('{'))
                    .count();
            }
            let removed_entries: Vec<EntryId> = removed.iter().map(|r| reading.element(*r).entry()).collect();
            let gone: Vec<EntryId> = removed
                .iter()
                .filter(|r| {
                    self.rule(reading.element(**r).rule)
                        .remove
                        .as_ref()
                        .is_some_and(|x| x.remove_container_when_empty)
                })
                .map(|r| reading.element(*r).entry())
                .collect();
            // A sibling container goes too when this edit removes everything in it.
            self.remove_emptied_levels(plan, entry, levels, &|other| {
                removed_entries.contains(&other) || {
                    let v = self.value_of(other);
                    !v.entries.is_empty() && v.entries.iter().all(|e| gone.contains(e))
                }
            });
        }
        let span = self.without_final_newline(plan, span, removed);
        plan.add_span(Operation::RemoveEntry, span, "");
        Ok(())
    }

    /// The last entry of a body without a final newline, removed with the line ending before it, so the body still
    /// has no final newline - as an insertion after such a line keeps it so (FBL section 6.3). Not when the entry
    /// above is removed too, whose span ends where this one starts.
    fn without_final_newline(&self, plan: &Plan<'_>, span: Span, removed: &[usize]) -> Span {
        let bytes = self.text.bytes();
        if span.end != self.text.len()
            || span.start == 0
            || bytes.last() == Some(&b'\n')
            || bytes[span.start - 1] != b'\n'
        {
            return span;
        }
        if removed.iter().any(|r| {
            let e = &self.entries[plan.reading.element(*r).entry()];
            matches!(e.kind, EntryKind::Tree(_)) && e.line_span.unwrap_or(e.own).end == span.start
        }) {
            return span;
        }
        let mut start = span.start - 1;
        if start > 0 && bytes[start - 1] == b'\r' {
            start -= 1;
        }
        span.with_start(start)
    }
}

/// A string inside a flow collection: plain only when it is plain-safe and holds none of the flow indicators
/// `, [ ] { }` nor a `:`, which a flow collection would read as a separator or a mapping; double-quoted otherwise.
fn flow_item(value: &str) -> String {
    if scalars::is_plain_safe(value, false) && !value.contains([',', '[', ']', '{', '}', ':']) {
        value.to_owned()
    } else {
        scalars::double_quoted(value)
    }
}

/// The order keys are written in: the rule's `insert.keys`, then its bindings' keys in binding order.
fn key_order(rule: &crate::documents::types::Rule, child: Option<&str>) -> Vec<String> {
    let mut order: Vec<String> = Vec::new();
    if child.is_none()
        && let Some(insert) = &rule.insert
    {
        order.extend(insert.keys.iter().cloned());
    }
    let mut add = |slot: Option<&crate::documents::types::Slot>| {
        if let Some(slot) = slot
            && let Some(key) = &slot.key
            && slot.child.as_deref() == child
            && !order.contains(key)
        {
            order.push(key.clone());
        }
    };
    add(rule.id.as_ref().and_then(|i| i.from.as_ref()));
    add(rule.source.as_ref().map(|s| &s.slot));
    add(rule.target.as_ref().map(|t| &t.slot));
    for (_, binding) in &rule.attributes {
        add(Some(&binding.slot));
    }
    order
}

/// The selector of the container of the entries `at` selects.
pub(crate) fn container_of(at: Option<&str>) -> String {
    let Some(at) = at else { return "/".into() };
    match at.rfind('/') {
        Some(cut) if cut > 0 => at[..cut].to_owned(),
        _ => "/".into(),
    }
}
