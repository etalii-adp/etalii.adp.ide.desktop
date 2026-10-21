//! The `json` family (FBL section 4.4; the standalone host's `Files/Json/JsonFamily`): an RFC 8259 text read byte
//! for byte, members and items with their own spans and the separator after each, comments refused, duplicate
//! names reported.

use adp_cel::Value;

use crate::documents::types::{AttributeBinding, InsertSettings};
use crate::families::tree::{TreeEntry, TreeValue, ValueId, ValueKind, ValueStyle};
use crate::families::{Entry, EntryId, EntryKind, Family, FamilyKind};
use crate::model::{FindingSeverity, codes};
use crate::planning::new_text;
use crate::planning::plan::{Plan, Planning, refuse};
use crate::rules::{InsertRequest, ReadElement, SlotChange, SlotRead};
use crate::splice::Operation;
use crate::text::Span;

struct SyntaxError {
    offset: usize,
    message: String,
}

fn fail<T>(offset: usize, message: impl Into<String>) -> Result<T, SyntaxError> {
    Err(SyntaxError {
        offset,
        message: message.into(),
    })
}

struct JsonParser<'a> {
    family: &'a mut Family,
    values: Vec<TreeValue>,
    position: usize,
}

impl JsonParser<'_> {
    fn bytes(&self) -> &[u8] {
        self.family.text.bytes()
    }

    fn current(&self) -> u8 {
        self.bytes().get(self.position).copied().unwrap_or(0)
    }

    fn column(&self, offset: usize) -> usize {
        let text = &self.family.text;
        let index = text.line_index_at(offset);
        let line = text.lines()[index];
        offset - line.start.max(if index == 0 { text.bom_length() } else { 0 })
    }

    fn skip_whitespace(&mut self) -> Result<(), SyntaxError> {
        while self.position < self.bytes().len() {
            match self.bytes()[self.position] {
                b' ' | b'\t' | b'\r' | b'\n' => self.position += 1,
                b'/' => return fail(self.position, "A JSON body cannot hold comments."),
                _ => break,
            }
        }
        Ok(())
    }

    fn expect(&mut self, expected: u8) -> Result<(), SyntaxError> {
        if self.current() != expected {
            return fail(self.position, format!("'{}' was expected here.", expected as char));
        }
        self.position += 1;
        Ok(())
    }

    fn add_value(&mut self, value: TreeValue) -> ValueId {
        self.values.push(value);
        self.values.len() - 1
    }

    fn add_entry(&mut self, own: Span, name: Option<String>, key_span: Option<Span>, value: ValueId) -> EntryId {
        let indent = self.column(key_span.unwrap_or(own).start);
        self.family.entries.push(Entry {
            own,
            line_span: None,
            parent: None,
            children: Vec::new(),
            name,
            indent,
            kind: EntryKind::Tree(TreeEntry {
                is_root: false,
                key_span,
                value,
                separator: None,
            }),
        });
        self.family.entries.len() - 1
    }

    fn set_separator(&mut self, entry: EntryId, at: usize) {
        if let EntryKind::Tree(t) = &mut self.family.entries[entry].kind {
            t.separator = Some(at);
        }
    }

    fn value(&mut self) -> Result<ValueId, SyntaxError> {
        if self.position >= self.bytes().len() {
            return fail(self.position, "The body ends where a value was expected.");
        }
        match self.current() {
            b'{' => self.object(),
            b'[' => self.array(),
            b'"' => {
                let (text, span) = self.string()?;
                Ok(self.add_value(TreeValue::scalar(
                    ValueStyle::JsonString,
                    span,
                    text.clone(),
                    Value::from(text),
                )))
            }
            _ => self.literal(),
        }
    }

    fn object(&mut self) -> Result<ValueId, SyntaxError> {
        let start = self.position;
        self.position += 1;
        let mut entries = Vec::new();
        let mut names: Vec<String> = Vec::new();
        self.skip_whitespace()?;
        if self.current() == b'}' {
            self.position += 1;
            return Ok(self.container(ValueKind::Mapping, start, entries));
        }
        loop {
            self.skip_whitespace()?;
            if self.current() != b'"' {
                return fail(self.position, "A member name in quotes was expected here.");
            }
            let (name, key_span) = self.string()?;
            self.skip_whitespace()?;
            self.expect(b':')?;
            self.skip_whitespace()?;
            let value = self.value()?;
            let own = Span::new(key_span.start, self.values[value].span.end);
            let member = self.add_entry(own, Some(name.clone()), Some(key_span), value);
            if names.contains(&name) {
                self.family.report(
                    codes::DUPLICATE_KEY,
                    FindingSeverity::Warning,
                    format!("The member \"{name}\" appears again in this object; only the first is read."),
                    Some(key_span),
                );
            } else {
                names.push(name);
                entries.push(member);
            }
            self.skip_whitespace()?;
            if self.current() == b',' {
                self.set_separator(member, self.position);
                self.position += 1;
                continue;
            }
            self.expect(b'}')?;
            break;
        }
        Ok(self.container(ValueKind::Mapping, start, entries))
    }

    fn array(&mut self) -> Result<ValueId, SyntaxError> {
        let start = self.position;
        self.position += 1;
        let mut entries = Vec::new();
        self.skip_whitespace()?;
        if self.current() == b']' {
            self.position += 1;
            return Ok(self.container(ValueKind::Sequence, start, entries));
        }
        loop {
            self.skip_whitespace()?;
            let value = self.value()?;
            let span = self.values[value].span;
            let item = self.add_entry(span, None, None, value);
            entries.push(item);
            self.skip_whitespace()?;
            if self.current() == b',' {
                self.set_separator(item, self.position);
                self.position += 1;
                continue;
            }
            self.expect(b']')?;
            break;
        }
        Ok(self.container(ValueKind::Sequence, start, entries))
    }

    fn container(&mut self, kind: ValueKind, start: usize, entries: Vec<EntryId>) -> ValueId {
        let mut value = TreeValue::new(kind, ValueStyle::Block, Span::new(start, self.position));
        value.entries = entries;
        self.add_value(value)
    }

    fn string(&mut self) -> Result<(String, Span), SyntaxError> {
        let start = self.position;
        self.position += 1;
        let mut units: Vec<u16> = Vec::new();
        let mut text = String::new();
        let mut run_start = self.position;
        let flush_units = |units: &mut Vec<u16>, text: &mut String| {
            if !units.is_empty() {
                text.push_str(&String::from_utf16_lossy(units));
                units.clear();
            }
        };
        loop {
            if self.position >= self.bytes().len() {
                return fail(start, "A string is never closed.");
            }
            let b = self.bytes()[self.position];
            if b == b'"' {
                break;
            }
            if b < 0x20 {
                return fail(self.position, "A string holds a control character that is not escaped.");
            }
            if b != b'\\' {
                self.position += 1;
                continue;
            }
            if run_start < self.position {
                flush_units(&mut units, &mut text);
                text.push_str(self.family.text.text(run_start, self.position));
            }
            self.position += 1;
            let escape = self.current();
            self.position += 1;
            let simple = match escape {
                b'"' => Some('"'),
                b'\\' => Some('\\'),
                b'/' => Some('/'),
                b'b' => Some('\u{8}'),
                b'f' => Some('\u{c}'),
                b'n' => Some('\n'),
                b'r' => Some('\r'),
                b't' => Some('\t'),
                b'u' => None,
                _ => return fail(self.position - 1, "This escape is not one JSON knows."),
            };
            match simple {
                Some(c) => {
                    flush_units(&mut units, &mut text);
                    text.push(c);
                }
                None => {
                    let digits = (self.position + 4 <= self.bytes().len())
                        .then(|| self.family.text.text(self.position, self.position + 4))
                        .filter(|d| d.len() == 4 && d.bytes().all(|b| b.is_ascii_hexdigit()));
                    let Some(code) = digits.and_then(|d| u16::from_str_radix(d, 16).ok()) else {
                        return fail(self.position, "A \\u escape needs four hexadecimal digits.");
                    };
                    units.push(code);
                    self.position += 4;
                }
            }
            run_start = self.position;
        }
        flush_units(&mut units, &mut text);
        text.push_str(self.family.text.text(run_start, self.position));
        self.position += 1;
        let span = Span::new(start, self.position);
        self.family.leaves.push(span);
        Ok((text, span))
    }

    fn literal(&mut self) -> Result<ValueId, SyntaxError> {
        let start = self.position;
        while self.position < self.bytes().len()
            && matches!(self.bytes()[self.position], b'a'..=b'z' | b'0'..=b'9' | b'-' | b'+' | b'.' | b'E')
        {
            self.position += 1;
        }
        let raw = self.family.text.text(start, self.position).to_owned();
        let typed = match raw.as_str() {
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            "null" => Value::Null,
            _ => {
                if let Ok(l) = raw.parse::<i64>() {
                    Value::Int(l)
                } else if !raw.is_empty() && !raw.starts_with('+') && is_float(&raw) {
                    Value::Double(raw.parse().unwrap_or(0.0))
                } else if raw.is_empty() {
                    return fail(start, "A value was expected here.");
                } else {
                    return fail(start, format!("'{raw}' is not a JSON value."));
                }
            }
        };
        let span = Span::new(start, self.position);
        self.family.leaves.push(span);
        Ok(self.add_value(TreeValue::scalar(ValueStyle::JsonOther, span, raw, typed)))
    }
}

/// .NET's `double.TryParse` with `NumberStyles.Float` over what a JSON literal can hold.
fn is_float(raw: &str) -> bool {
    !raw.chars().any(|c| c.is_ascii_alphabetic() && c != 'e' && c != 'E') && raw.parse::<f64>().is_ok()
}

impl Family {
    pub(super) fn json_parse(&mut self) {
        let start = self.text.bom_length();
        let mut parser = JsonParser {
            family: self,
            values: Vec::new(),
            position: start,
        };
        let result = (|| -> Result<ValueId, SyntaxError> {
            parser.skip_whitespace()?;
            let value = parser.value()?;
            parser.skip_whitespace()?;
            if parser.position < parser.bytes().len() {
                return fail(parser.position, "The body has more than one JSON value.");
            }
            Ok(value)
        })();
        let values = std::mem::take(&mut parser.values);
        let root_value = match result {
            Ok(value) => value,
            Err(error) => {
                self.unreadable = Some((error.offset, error.message));
                return;
            }
        };
        let span = values[root_value].span;
        let indent = {
            let index = self.text.line_index_at(span.start);
            let line = self.text.lines()[index];
            span.start - line.start.max(if index == 0 { self.text.bom_length() } else { 0 })
        };
        if let FamilyKind::Json(data) = &mut self.kind {
            data.values = values;
        }
        self.entries.push(Entry {
            own: span,
            line_span: None,
            parent: None,
            children: Vec::new(),
            name: None,
            indent,
            kind: EntryKind::Tree(TreeEntry {
                is_root: true,
                key_span: None,
                value: root_value,
                separator: None,
            }),
        });
        let root = self.entries.len() - 1;
        if let FamilyKind::Json(data) = &mut self.kind {
            data.root = Some(root);
        }
        self.index(root);
        for i in 0..self.order.len() {
            let id = self.order[i];
            if !self.entries[id].tree().is_root {
                self.entries[id].line_span = self.json_line_span_of(id);
            }
        }
    }

    /// An entry starts its line when only whitespace precedes it, and ends it when only its separator and
    /// whitespace follow; its line span then takes the separator with it.
    fn json_line_span_of(&self, entry: EntryId) -> Option<Span> {
        let own = self.entries[entry].own;
        if !self.only_whitespace_before(own.start) {
            return None;
        }
        let bytes = self.text.bytes();
        let mut i = own.end;
        let line = self.text.lines()[self.text.line_index_at(own.start.max(i.saturating_sub(1)))];
        while i < line.content_end && matches!(bytes[i], b' ' | b'\t') {
            i += 1;
        }
        if i < line.content_end && bytes[i] == b',' {
            i += 1;
        }
        while i < line.content_end && matches!(bytes[i], b' ' | b'\t') {
            i += 1;
        }
        if i != line.content_end {
            return None;
        }
        Some(Span::new(
            self.text.lines()[self.text.line_index_at(own.start)].start,
            line.end,
        ))
    }

    // ---- writing ----

    pub(super) fn json_format(&self, read: &SlotRead, binding: Option<&AttributeBinding>, value: &Value) -> String {
        match binding.and_then(|b| new_text::wire(b, value, read.wire.as_deref())) {
            Some(wire) => quote(&wire),
            None => write(value, binding),
        }
    }

    pub(super) fn json_write(
        &self,
        plan: &mut Plan<'_>,
        element: &ReadElement,
        changes: &[SlotChange<'_>],
    ) -> Planning {
        for change in changes {
            let read = &change.read;
            if change.is_empty && change.binding.empty.as_deref() == Some("remove") {
                if let crate::rules::Node::TreeMember(member) = read.node {
                    plan.add_span(Operation::RemoveKey, self.json_removal_span(member), "");
                }
                continue;
            }
            if read.present
                && let Some(span) = read.span
            {
                let written = self.json_format(read, Some(change.binding), &change.value);
                if written != self.text.text_of(span) {
                    plan.add_span(Operation::ReplaceValue, span, written);
                }
                continue;
            }
            let mapping = self.mapping(element.entry(), change.binding.child.as_deref());
            let (Some(mapping), Some(key)) = (mapping, &change.binding.key) else {
                return refuse(format!(
                    "The {} file has no object to write \"{}\" in.",
                    self.binding.name,
                    change.binding.key.as_deref().unwrap_or_default()
                ));
            };
            let added = format!(
                "{}: {}",
                quote(key),
                self.json_format(read, Some(change.binding), &change.value)
            );
            let order = self.rule(change.rule).wire_order();
            let (offset, text) = match self.member_before(mapping, key, &order) {
                Some(before) => {
                    let b = &self.entries[before];
                    let separator = if b.line_span.is_some() {
                        format!(",{}{}", self.newline_at(b.own.end), self.indentation(b.indent))
                    } else {
                        ", ".into()
                    };
                    (b.own.end, separator + &added)
                }
                None => self.new_member(mapping, &added),
            };
            plan.add(Operation::InsertKey, offset, offset, text);
        }
        Ok(())
    }

    /// The member a new `key` is written right after: the nearest one before it in the rule's key order (FBL
    /// section 6.1). None when the order does not place the key, or places it after every member, and it goes last.
    fn member_before(&self, mapping: ValueId, key: &str, order: &[String]) -> Option<EntryId> {
        let position = order.iter().position(|k| k == key)?;
        let entries = &self.tree_value(mapping).entries;
        let before = entries.iter().copied().rfind(|e| {
            self.entries[*e]
                .name
                .as_ref()
                .and_then(|n| order.iter().position(|k| k == n))
                .is_some_and(|at| at < position)
        })?;
        (Some(&before) != entries.last()).then_some(before)
    }

    /// A new member or item after a container's last entry, with the previous entry's separator (FBL 6.3).
    fn new_member(&self, container: ValueId, text: &str) -> (usize, String) {
        let value = self.tree_value(container);
        let Some(&last) = value.entries.last() else {
            return (value.span.start + 1, text.to_owned());
        };
        let last = &self.entries[last];
        if last.line_span.is_some() {
            (
                last.own.end,
                format!(
                    ",{}{}{text}",
                    self.newline_at(last.own.end),
                    self.indentation(last.indent)
                ),
            )
        } else {
            (last.own.end, format!(", {text}"))
        }
    }

    pub(super) fn json_insert(&self, plan: &mut Plan<'_>, request: &InsertRequest) -> Planning {
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
        let parent_entry = request.parent.map(|p| reading.element(p).entry());
        let container = insert
            .container
            .as_ref()
            .and_then(|c| self.container(c, parent_entry, &Family::captures_of(plan)));
        let text = match &insert.emit {
            Some(emit) => new_text::render(emit, &|name| match name {
                "id" => request.id.as_ref().map(|id| escape(id)),
                "source" => request.source.map(|s| escape(&reading.element(s).key)),
                "target" => request.target.map(|t| escape(&reading.element(t).key)),
                _ => request
                    .values
                    .get(name)
                    .filter(|v| !v.is_null())
                    .map(|v| escape(&new_text::plain(v, rule.attribute(name)))),
            }),
            None => {
                let members: Vec<String> = insert
                    .keys
                    .iter()
                    .filter_map(|key| {
                        self.json_wire_value(plan, request, key)
                            .map(|value| format!("{}: {value}", quote(key)))
                    })
                    .collect();
                format!("{{ {} }}", members.join(", "))
            }
        };
        let Some(container) = container else {
            let Some((offset, opening)) = self.new_container(insert, parent_entry) else {
                return refuse(format!(
                    "The {} file has no {} to add the {} to.",
                    self.binding.name,
                    insert.container.as_deref().unwrap_or("container"),
                    rule.type_name
                ));
            };
            // A container that encloses its entries on one line cannot be written before an entry that goes inside
            // it, so it is created as its opening and its closing with the entry between them, all three at one
            // offset (FBL section 6.2).
            plan.add(Operation::EnsureContainer, offset, offset, opening);
            plan.add(Operation::InsertEntry, offset, offset, text);
            plan.add(Operation::EnsureContainer, offset, offset, "]");
            return Ok(());
        };
        let value = self.value_of(container);
        if value.kind == ValueKind::Scalar {
            return refuse(format!(
                "The {} file has no {} to add the {} to.",
                self.binding.name,
                insert.container.as_deref().unwrap_or("container"),
                rule.type_name
            ));
        }
        if insert.place == "start"
            && let Some(&first) = value.entries.first()
        {
            let first = &self.entries[first];
            let suffix = if first.line_span.is_some() {
                format!(
                    ",{}{}",
                    self.newline_at(first.own.start),
                    self.indentation(first.indent)
                )
            } else {
                ", ".into()
            };
            plan.add(Operation::InsertEntry, first.own.start, first.own.start, text + &suffix);
            return Ok(());
        }
        let (offset, inserted) = self.new_member(self.entries[container].tree().value, &text);
        plan.add(Operation::InsertEntry, offset, offset, inserted);
        Ok(())
    }

    /// Where a missing container is created and how it opens (FBL section 6.2 `create`): a list under the
    /// selector's last name, right after the member `create.at.after` names in the parent entry's object, or after
    /// that object's last member for `create.at: "end"`. None when the rule creates no container, or creates it in
    /// a way this family does not write yet.
    fn new_container(&self, insert: &InsertSettings, parent: Option<EntryId>) -> Option<(usize, String)> {
        let selector = insert.container.as_ref()?;
        let create = insert.create.as_ref()?;
        if !matches!(create.at.as_str(), "after" | "end") || create.text.is_some() || selector.starts_with('/') {
            return None;
        }
        let mapping = self.entries[parent?].tree().value;
        let value = self.tree_value(mapping);
        if value.kind != ValueKind::Mapping || value.entries.is_empty() {
            return None;
        }
        // "end" (FBL 0.4) is after the last member of the container's parent; "after" names the member.
        let member = if create.at == "end" {
            value.entries.last().copied()
        } else {
            create.argument.as_ref().and_then(|a| self.member(mapping, a))
        }?;
        if self.entries[member].line_span.is_some() {
            return None;
        }
        let trimmed = selector.trim_end_matches('/');
        let name = &trimmed[trimmed.rfind('/').map(|i| i + 1).unwrap_or(0)..];
        Some((self.entries[member].own.end, format!(", {}: [", quote(name))))
    }

    fn json_wire_value(&self, plan: &Plan<'_>, request: &InsertRequest, key: &str) -> Option<String> {
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
            return Some(quote(id));
        }
        if rule.source.as_ref().and_then(|s| s.key.as_deref()) == Some(key)
            && let Some(source) = request.source
        {
            return Some(quote(&reading.element(source).key));
        }
        if rule.target.as_ref().and_then(|t| t.key.as_deref()) == Some(key)
            && let Some(target) = request.target
        {
            return Some(quote(&reading.element(target).key));
        }
        for (name, binding) in &rule.attributes {
            if binding.key.as_deref() == Some(key)
                && binding.child.is_none()
                && let Some(value) = request.values.get(name)
                && !value.is_null()
            {
                return Some(write(value, Some(binding)));
            }
        }
        None
    }

    /// What removing an entry takes (FBL section 6.2): its line span when it has one and is not the last of
    /// several, else its own span with one separator: the one after it, or, for the last entry, the one before it
    /// and the whitespace in between.
    fn json_removal_span(&self, entry: EntryId) -> Span {
        let e = &self.entries[entry];
        let parent = e.parent.expect("a member has a parent");
        let siblings = &self.value_of(parent).entries;
        let index = siblings.iter().position(|s| *s == entry).unwrap_or(0);
        let is_last = index + 1 == siblings.len();
        if is_last && index > 0 {
            let previous = &self.entries[siblings[index - 1]];
            if let Some(separator) = previous.tree().separator {
                return e.own.with_start(separator);
            }
        }
        if let Some(line) = e.line_span {
            return line;
        }
        if !is_last {
            return e.own.with_end(self.entries[siblings[index + 1]].own.start);
        }
        e.own
    }

    /// A move among siblings in json (FBL section 5.5): the entry is removed with the one separator a removal
    /// takes, and its own bytes are inserted with the separator its new place needs.
    pub(super) fn json_move(
        &self,
        plan: &mut Plan<'_>,
        element: usize,
        before: Option<usize>,
        last: usize,
    ) -> Planning {
        let reading = plan.reading;
        let entry = reading.element(element).entry();
        let own = self.text.text_of(self.entries[entry].own).to_owned();
        plan.add_span(Operation::RemoveEntry, self.json_removal_span(entry), "");
        if let Some(before) = before {
            let next = &self.entries[reading.element(before).entry()];
            if let Some(line) = next.line_span {
                plan.add(
                    Operation::InsertEntry,
                    line.start,
                    line.start,
                    format!("{}{own},{}", self.indentation(next.indent), self.newline_at(line.start)),
                );
                return Ok(());
            }
            plan.add(
                Operation::InsertEntry,
                next.own.start,
                next.own.start,
                format!("{own}, "),
            );
            return Ok(());
        }
        let tail = &self.entries[reading.element(last).entry()];
        let inserted = if tail.line_span.is_some() {
            format!(
                ",{}{}{own}",
                self.newline_at(tail.own.end),
                self.indentation(tail.indent)
            )
        } else {
            format!(", {own}")
        };
        plan.add(Operation::InsertEntry, tail.own.end, tail.own.end, inserted);
        Ok(())
    }

    pub(super) fn json_remove(&self, plan: &mut Plan<'_>, element: usize) -> Planning {
        let reading = plan.reading;
        let read_element = reading.element(element);
        let entry = read_element.entry();
        if self.entries[entry].tree().is_root {
            return refuse(format!("The whole {} file cannot be removed.", self.binding.name));
        }
        // The only entry of a container that goes when it is left empty (FBL section 6.2): the container goes with
        // it, as what is before the entry, the entry itself, and what is after it.
        let rule = self.rule(read_element.rule);
        if rule.remove.as_ref().is_some_and(|r| r.remove_container_when_empty)
            && let Some(container) = self.entries[entry].parent
            && !self.entries[container].tree().is_root
            && let Some(owner) = self.entries[container].parent
            && self.value_of(container).entries.len() == 1
        {
            // The container goes as it was created (FBL section 6.2): with the separator before it, where a member
            // stands before it, so that removing what an insert created leaves the bytes the insert started from.
            // Only a first member takes the separator after it instead.
            let members = &self.value_of(owner).entries;
            let at = members.iter().position(|m| *m == container).unwrap_or(0);
            let start = if at > 0 {
                self.entries[members[at - 1]].own.end
            } else {
                self.entries[container].own.start
            };
            let end = if at == 0 && members.len() > 1 {
                self.entries[members[1]].own.start
            } else {
                self.entries[container].own.end
            };
            let own = self.entries[entry].own;
            plan.add(Operation::RemoveContainer, start, own.start, "");
            plan.add_span(Operation::RemoveEntry, own, "");
            plan.add(Operation::RemoveContainer, own.end, end, "");
            return Ok(());
        }
        plan.add_span(Operation::RemoveEntry, self.json_removal_span(entry), "");
        Ok(())
    }
}

pub(crate) fn write(value: &Value, binding: Option<&AttributeBinding>) -> String {
    match value {
        Value::Null => "null".into(),
        Value::String(s) => quote(s),
        Value::Bool(b) => if *b { "true" } else { "false" }.into(),
        Value::Int(_) | Value::Double(_) => {
            new_text::number(value.as_number().unwrap_or(0.0), binding.and_then(|b| b.decimals))
        }
        Value::List(list) => format!(
            "[{}]",
            list.iter().map(|v| write(v, binding)).collect::<Vec<_>>().join(", ")
        ),
        other => quote(&new_text::dotnet_string(other)),
    }
}

/// A JSON string with the escapes RFC 8785 uses (FBL section 6.3).
pub(crate) fn quote(value: &str) -> String {
    format!("\"{}\"", escape(value))
}

pub(crate) fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}
