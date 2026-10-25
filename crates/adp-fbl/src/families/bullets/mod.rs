//! The `bullets` family (FBL 0.7 sections 4.8 and 6.3; the standalone host's `Files/Bullets/BulletsFamily` at
//! `a7fdf03`): the list items under the headings a binding names, read as a tree, and the statements outside every
//! item, read as the `lines` family reads its own. An item's text is matched and written as a statement is, so the
//! groups, the words and re-emitting are those of [`super::lines`]; what is the family's own is its kinds of line,
//! its sections, the continuation, and where a new or moved item goes.

pub(crate) mod parser;
#[cfg(test)]
mod tests;

use std::collections::HashMap;

use adp_cel::Value;

use crate::documents::types::{HeaderSettings, Rule, Slot};
use crate::families::{EntryId, Family, FamilyKind};
use crate::model::{FindingSeverity, codes};
use crate::planning::new_text;
use crate::planning::plan::{Plan, Planning, refuse};
use crate::rules::reading::BodyReading;
use crate::rules::{Candidate, InsertRequest, Node, ReadElement, SlotChange, SlotRead};
use crate::splice::Operation;
use crate::text::Span;
use parser::columns;

/// What a bullets reading keeps besides its statements: each section the binding names that the body has, as the
/// line of its heading and the line it ends before.
#[derive(Debug, Default)]
pub(crate) struct BulletsData {
    pub sections: HashMap<String, (usize, usize)>,
}

/// Where a new item goes and how it is written: its offset, the indentation and marker it takes, and whether a
/// blank line stands before it.
struct Place {
    offset: usize,
    indent: String,
    marker: char,
    blank_before: bool,
}

impl Family {
    fn bullets_data(&self) -> &BulletsData {
        match &self.kind {
            FamilyKind::Lines(l) => l.bullets.as_ref().expect("the family is bullets"),
            _ => panic!("the family is not bullets"),
        }
    }

    pub(crate) fn is_bullets(&self) -> bool {
        matches!(&self.kind, FamilyKind::Lines(l) if l.bullets.is_some())
    }

    fn is_item(&self, entry: EntryId) -> bool {
        self.entries[entry].statement().bullet.is_some()
    }

    pub(super) fn bullets_parse(&mut self) {
        let parsed = self.read_bullets_body();
        let mut sections = HashMap::new();
        for section in &self.binding.sections {
            let regex = self.regex(&section.heading, section.case_insensitive);
            let Some(first) = parsed.headings.iter().find(|h| regex.is_match(&h.text)) else {
                continue;
            };
            let end = parsed
                .headings
                .iter()
                .find(|h| h.line > first.line && h.level <= first.level)
                .map_or(self.text.lines().len(), |h| h.line);
            sections.insert(section.name.clone(), (first.line, end));
        }
        if let FamilyKind::Lines(data) = &mut self.kind {
            data.statements = parsed.statements;
            data.bullets = Some(BulletsData { sections });
        }
    }

    /// Whether `item` stands in the section `name`: its item line is one of the section's lines.
    fn stands_in(&self, item: EntryId, name: &str) -> bool {
        let line = self.entries[item].statement().first_line;
        self.bullets_data()
            .sections
            .get(name)
            .is_some_and(|&(first, end)| first < line && line < end)
    }

    /// Line 1, after a byte-order mark, must match the header (FBL 0.7 section 5.6); when it is a statement, no rule
    /// reads it.
    pub(super) fn bullets_header_holds(&self, header: &HeaderSettings) -> bool {
        let Some(expression) = &header.line else { return true };
        let line = self.text.lines()[0];
        let mut first = self.text.bom_length();
        while first < line.content_end && matches!(self.text.bytes()[first], b' ' | b'\t') {
            first += 1;
        }
        if !self
            .regex(expression, false)
            .is_match(self.text.text(first, line.content_end))
        {
            return false;
        }
        if let Some(&statement) = self.lines_statements().iter().find(|&&s| {
            let s = self.entries[s].statement();
            s.first_line == 0 && s.bullet.is_none()
        }) {
            self.entries[statement].statement().is_header.set(true);
        }
        true
    }

    /// A rule with `within` matches the items of the sections it names; one without matches statements.
    pub(super) fn bullets_candidates(&self, rule: usize) -> Vec<Candidate> {
        let r = self.rule(rule);
        let Some(line) = &r.line else { return Vec::new() };
        let mut candidates = Vec::new();
        for &entry in self.lines_statements() {
            if self.entries[entry].statement().is_header.get() {
                continue;
            }
            let item = self.is_item(entry);
            let skip = match &r.within {
                Some(within) => !item || !within.iter().any(|name| self.stands_in(entry, name)),
                None => item,
            };
            if skip {
                continue;
            }
            if let Some(groups) = self.match_groups(line, r.case_insensitive, entry) {
                candidates.push(Family::to_candidate(Some(rule), None, entry, &groups));
            }
        }
        candidates
    }

    /// An item nested in an item no rule read, or one that could not be read, is not looked into (FBL 0.7 section
    /// 4.8).
    pub(super) fn bullets_admits(&self, entry: EntryId, claimed_by: &dyn Fn(EntryId) -> Option<String>) -> bool {
        self.entries[entry]
            .parent
            .is_none_or(|parent| claimed_by(parent).is_some())
    }

    pub(super) fn bullets_read(&self, candidate: &Candidate, slot: &Slot) -> SlotRead {
        if !slot.continuation {
            return self.lines_read(candidate, slot);
        }
        let entry = &self.entries[candidate.entry];
        let Some(item) = entry.statement().bullet.as_deref() else {
            return SlotRead::read_only_absent("A statement has no continuation.");
        };
        let mut read = SlotRead::new(
            Value::from(item.continuation_text(&self.text)),
            item.continuation_span(&self.text, entry.statement().first_line),
            true,
            true,
        );
        read.node = Node::Continuation;
        read
    }

    pub(super) fn bullets_after_read(&self, claimed_by: &dyn Fn(EntryId) -> Option<String>) {
        if !self.binding.report_unmatched {
            return;
        }
        for &statement in self.lines_statements() {
            let s = self.entries[statement].statement();
            if s.bullet.is_some() || s.is_header.get() || claimed_by(statement).is_some() {
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

    // ---- writing the continuation (FBL 0.7 section 6.3) ----

    /// The splice that writes a change of an item's continuation: none when the value is what the item has.
    pub(super) fn continuation_splice(
        &self,
        element: &ReadElement,
        change: &SlotChange<'_>,
    ) -> Option<(Operation, Span, String)> {
        let value = new_text::plain(&change.value, None);
        let item = self.bullet(element.entry());
        if change.is_empty && change.binding.empty.as_deref() == Some("remove") {
            return change.read.span.map(|span| (Operation::RemoveKey, span, String::new()));
        }
        if let Some(span) = change.read.span {
            if change.read.value.as_str() == Some(value.as_str()) {
                return None;
            }
            if value.is_empty() {
                return Some((Operation::RemoveKey, span, String::new()));
            }
            let text = columns::indented(&value, item.content_column, &self.newline_at(span.start));
            return Some((Operation::ReplaceValue, span, text));
        }
        if value.is_empty() {
            return None;
        }
        let offset = self.text.lines()[self.entries[element.entry()].statement().first_line].content_end;
        let text = columns::indented(&value, item.content_column, &self.newline_at(offset));
        Some((Operation::InsertKey, Span::new(offset, offset), text))
    }

    // ---- new and moved items (FBL 0.7 sections 6.2 and 6.3) ----

    pub(super) fn bullets_insert(&self, plan: &mut Plan<'_>, request: &InsertRequest) -> Planning {
        let reading: &BodyReading = plan.reading;
        let rule = self.rule(request.rule);
        let Some(insert) = &rule.insert else {
            return refuse(format!(
                "The binding gives no form to write a new {} in.",
                rule.type_name
            ));
        };
        let Some(emit) = &insert.emit else {
            return refuse(format!(
                "The binding gives no form to write a new {} in.",
                rule.type_name
            ));
        };
        let source = request.source.map(|s| reading.element(s).key.clone());
        let target = request.target.map(|t| reading.element(t).key.clone());
        if insert.place == "after-header" && rule.within.is_none() {
            // A statement written right after the line the header matched, as a line of its own (FBL 0.7 section
            // 6.2).
            let after_header = self.text.lines()[0].end;
            let line = self.render_new(emit, request, None, None);
            let text = self.as_new_lines(&line, after_header, false, &self.newline_at(after_header));
            plan.add(Operation::InsertEntry, after_header, after_header, text);
            return Ok(());
        }
        if !matches!(insert.place.as_str(), "after-last" | "end" | "last-child" | "start") {
            return refuse(format!("A bullets body cannot place a new entry '{}'.", insert.place));
        }
        let line = self.render_new(emit, request, source.as_deref(), target.as_deref());
        let parent = request.parent.map(|p| reading.element(p).entry());
        let siblings = self.bullet_siblings(reading, rule, request.parent);
        let count = siblings.len();
        let index = request
            .index
            .map_or(if insert.place == "start" { 0 } else { count }, |i| {
                usize::try_from(i).ok().filter(|&i| i <= count).unwrap_or(count)
            });
        let place = self.place(plan, rule, parent, &siblings, index)?;
        let column = columns::of(&place.indent) + 2;
        let mut lines = format!("{}{} {line}", place.indent, place.marker);
        if let Some(name) = continuation_attribute(rule)
            && let Some(notes) = request.values.get(name)
        {
            let continuation = new_text::plain(notes, None);
            if !continuation.is_empty() {
                lines.push_str(&columns::indented(&continuation, column, "\n"));
            }
        }
        let newline = self.newline_at(place.offset);
        let text = self.as_new_lines(&lines, place.offset, place.blank_before, &newline);
        plan.add(Operation::InsertEntry, place.offset, place.offset, text);
        Ok(())
    }

    /// The line `emit` writes for a new entry of the request's rule with its values (FBL section 6.3).
    fn render_new(&self, emit: &str, request: &InsertRequest, source: Option<&str>, target: Option<&str>) -> String {
        self.render_line(
            emit,
            request.rule,
            &request.values,
            request.id.as_deref(),
            source,
            target,
        )
    }

    /// The text of new lines at `offset`, written with `newline`: each line followed by a line ending, or, at the end
    /// of a body whose last line has none, each preceded by one and the body still without a final newline.
    /// `blank_before` writes one more ending first, so a blank line stands before the text.
    fn as_new_lines(&self, text: &str, offset: usize, blank_before: bool, newline: &str) -> String {
        let endless = offset == self.text.len()
            && !self.text.is_empty()
            && self.text.lines().last().is_some_and(|l| l.ending.is_empty());
        let mut out = String::new();
        if endless {
            out.push_str(newline);
        }
        if blank_before {
            out.push_str(newline);
        }
        for (i, line) in text.split('\n').enumerate() {
            if endless && i > 0 {
                out.push_str(newline);
            }
            out.push_str(line);
            if !endless {
                out.push_str(newline);
            }
        }
        out
    }

    /// Where a new item goes among `siblings` at `index`, and the indentation and marker it takes (FBL 0.7 section
    /// 6.3): after its previous sibling's line span with that sibling's; else at the start of its next sibling's with
    /// that one's; else as its parent's first nested item, with the parent's marker at its content column; else as
    /// the first item of its section, creating the section when it is missing.
    fn place(
        &self,
        plan: &mut Plan<'_>,
        rule: &Rule,
        parent: Option<EntryId>,
        siblings: &[usize],
        index: usize,
    ) -> Planning<Place> {
        let reading = plan.reading;
        let entry_of = |element: usize| reading.element(element).entry();
        if index > 0 {
            let previous = entry_of(siblings[index - 1]);
            let item = self.bullet(previous);
            return Ok(Place {
                offset: self.entries[previous].line_span.expect("an item's line span").end,
                indent: item.indent_text.clone(),
                marker: item.marker,
                blank_before: false,
            });
        }
        if let Some(&first) = siblings.first() {
            let next = entry_of(first);
            let item = self.bullet(next);
            return Ok(Place {
                offset: self.entries[next].line_span.expect("an item's line span").start,
                indent: item.indent_text.clone(),
                marker: item.marker,
                blank_before: false,
            });
        }
        if let Some(parent) = parent {
            let item = self.bullet(parent);
            let last = item
                .continuation
                .last()
                .copied()
                .unwrap_or(self.entries[parent].statement().first_line);
            return Ok(Place {
                offset: self.text.lines()[last].end,
                indent: " ".repeat(item.content_column),
                marker: item.marker,
                blank_before: false,
            });
        }
        let insert = rule.insert.as_ref().expect("an insertable rule");
        let name = insert
            .container
            .as_deref()
            .or_else(|| rule.within.as_ref().and_then(|w| w.first()).map(String::as_str));
        if let Some(name) = name
            && let Some(&(first, end)) = self.bullets_data().sections.get(name)
        {
            let mut last_line = first;
            for line in first + 1..end {
                if !self.is_blank(line) {
                    last_line = line;
                }
            }
            return Ok(Place {
                offset: self.text.lines()[last_line].end,
                indent: String::new(),
                marker: '-',
                blank_before: true,
            });
        }
        let heading = insert
            .create
            .as_ref()
            .filter(|c| c.at == "end-of-document")
            .and_then(|c| c.text.as_deref());
        let Some(heading) = heading else {
            return refuse(format!("The file has no section to add the {} to.", rule.type_name));
        };
        let offset = self.text.len();
        let newline = self.newline_at(offset);
        let blank_or_empty = self.text.len() == self.text.bom_length() || self.is_blank(self.text.lines().len() - 1);
        let text = self.as_new_lines(heading, offset, !blank_or_empty, &newline);
        plan.add(Operation::EnsureContainer, offset, offset, text);
        Ok(Place {
            offset,
            indent: String::new(),
            marker: '-',
            blank_before: true,
        })
    }

    fn is_blank(&self, line: usize) -> bool {
        let span = self.text.lines()[line];
        (span.start..span.content_end).all(|i| matches!(self.text.bytes()[i], b' ' | b'\t'))
    }

    /// The elements a new or moved item of `rule` stands among (FBL 0.7 section 6.2): those read from items nested in
    /// `parent`, or, at the top level, the top-level items of the rule's section.
    fn bullet_siblings(&self, reading: &BodyReading, rule: &Rule, parent: Option<usize>) -> Vec<usize> {
        let name = rule
            .insert
            .as_ref()
            .and_then(|i| i.container.as_deref())
            .or_else(|| rule.within.as_ref().and_then(|w| w.first()).map(String::as_str));
        reading
            .element_indices()
            .iter()
            .copied()
            .filter(|&e| {
                let element = reading.element(e);
                let entry = element.entry();
                self.is_item(entry)
                    && element.parent == parent
                    && (parent.is_some()
                        || (self.entries[entry].parent.is_none() && name.is_none_or(|n| self.stands_in(entry, n))))
            })
            .collect()
    }

    /// A move (FBL 0.7 sections 5.5 and 6.3): the item's line span is removed, and inserted where a new item would go
    /// at `index` among `parent`'s children, every line that is not blank shifted by the difference between the
    /// column a new item's marker would have there and the column its marker had.
    pub(crate) fn bullets_move_to(
        &self,
        plan: &mut Plan<'_>,
        element: usize,
        parent: Option<usize>,
        index: usize,
    ) -> Planning {
        let reading = plan.reading;
        let read = reading.element(element);
        let entry = read.entry();
        let rule = self.rule(read.rule);
        let span = self.entries[entry].line_span.expect("an item's line span");
        let siblings: Vec<usize> = self
            .bullet_siblings(reading, rule, parent)
            .into_iter()
            .filter(|&s| s != element)
            .collect();
        let index = if index > siblings.len() { siblings.len() } else { index };
        let place = self.place(plan, rule, parent.map(|p| reading.element(p).entry()), &siblings, index)?;
        let mut offset = place.offset;
        // The place is taken in the body before the edit: just after the item's old parent is where the item starts.
        if index > 0 {
            let previous = self.entries[reading.element(siblings[index - 1]).entry()]
                .line_span
                .expect("an item's line span");
            if previous.start <= span.start && span.end <= previous.end {
                offset = span.start;
            }
        }
        let shift = columns::of(&place.indent) as isize - self.bullet(entry).marker_column as isize;
        let mut moved = String::new();
        for (line, ending) in columns::lines(self.text.text_of(span)) {
            if line.trim_matches([' ', '\t']).is_empty() {
                moved.push_str(line);
            } else {
                moved.push_str(&columns::shift(line, shift));
            }
            moved.push_str(ending);
        }
        plan.add_span(Operation::RemoveEntry, span, "");
        plan.add(Operation::InsertEntry, offset, offset, moved);
        Ok(())
    }
}

/// The name of the rule's attribute bound to the continuation, if any.
fn continuation_attribute(rule: &Rule) -> Option<&str> {
    rule.attributes
        .iter()
        .find(|(_, a)| a.continuation)
        .map(|(name, _)| name.as_str())
}
