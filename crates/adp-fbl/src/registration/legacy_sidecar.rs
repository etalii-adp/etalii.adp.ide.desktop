//! A sidecar file hosts wrote before FBL (FBL section 8.7; the standalone host's `Registration/LegacySidecar`): a
//! legacy layout, keyed by view key and then by element id with `{"x", "y"}`, or *(FBL 0.12)* legacy identities, an
//! array of `{"id", "kind", "key"}`. It is read when the registration has no matching block and written back by json
//! splices while it exists; this crate never creates one.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_cel::Value;

use crate::documents::types::{
    BodySettings, Claims, DEFAULT_SETTLE, Family as FamilyName, FblBinding, RegistrationSettings, TextDefaults,
};
use crate::families::json::quote;
use crate::families::tree::{TreeValue, ValueId, ValueKind};
use crate::families::{EntryId, EntryKind, Family};
use crate::history::{SplicedFile, UndoResult};
use crate::model::{FblOptions, Finding, FindingSeverity, SourceLocation, codes};
use crate::planning::{PlanResult, PlannedEdit, new_text};
use crate::splice::{Operation, Splice};
use crate::text::{BodyText, Span};

/// The binding a sidecar is read with: one json body and no rules.
fn sidecar_binding() -> FblBinding {
    FblBinding {
        name: "sidecar".into(),
        title: None,
        claims: Claims::default(),
        body: BodySettings {
            is_folder: false,
            family: Some(FamilyName::Json),
            also_read: Vec::new(),
            recognise_all: Vec::new(),
            recognise_any: Vec::new(),
            recognise_none: Vec::new(),
            files: Vec::new(),
            ignore: Vec::new(),
            settle: DEFAULT_SETTLE,
            grammar: None,
        },
        plugin: None,
        read_only: None,
        text: TextDefaults::default(),
        header: None,
        comment: None,
        report_unmatched: false,
        blocks: Vec::new(),
        sections: Vec::new(),
        rules: Vec::new(),
        registration: RegistrationSettings::default(),
        template: None,
    }
}

/// An entry of a legacy identities file (FBL 0.12 section 8.7), with where it and its `key` value stand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IdentityEntry {
    pub id: String,
    pub kind: Option<String>,
    pub key: String,
    pub key_span: Span,
    pub own: Span,
}

impl IdentityEntry {
    /// The entry's key among the identities of a reading: `<kind>:<key>` with a kind.
    pub fn entry_key(&self) -> String {
        format!("{}{}", self.kind_prefix(), self.key)
    }

    fn kind_prefix(&self) -> String {
        self.kind.as_ref().map_or(String::new(), |kind| format!("{kind}:"))
    }

    /// Whether a rule declares the entry's kind; an entry without a kind is the binding's.
    pub fn is_declared(&self, declared: &[String]) -> bool {
        self.kind.as_ref().is_none_or(|kind| declared.contains(kind))
    }
}

/// A new entry of a legacy identities file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NewIdentity {
    pub id: String,
    pub kind: Option<String>,
    pub key: String,
}

/// One open legacy sidecar: its bytes, its json reading, and the one history of its edits.
pub struct LegacySidecar {
    file: SplicedFile,
    reading: Family,
}

impl std::fmt::Debug for LegacySidecar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LegacySidecar")
            .field("bytes", &self.file.bytes().len())
            .finish_non_exhaustive()
    }
}

impl LegacySidecar {
    pub fn open(bytes: Vec<u8>) -> LegacySidecar {
        let reading = read(&bytes);
        LegacySidecar {
            file: SplicedFile::new(bytes),
            reading,
        }
    }

    /// The sidecar path a registration's binding names (`{base}.layout.json` and the like), with `{base}` the
    /// body's base name, beside the body.
    pub fn path_for(pattern: &str, body_path: &Path) -> PathBuf {
        let full = std::path::absolute(body_path).unwrap_or_else(|_| body_path.to_path_buf());
        let folder = full.parent().map(Path::to_path_buf).unwrap_or_default();
        let base = body_path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        folder.join(pattern.replace("{base}", &base))
    }

    pub fn bytes(&self) -> &[u8] {
        self.file.bytes()
    }

    /// Whether the sidecar is not one JSON object; it is then neither applied nor written.
    pub fn is_unreadable(&self) -> bool {
        self.root().is_none()
    }

    /// The positions of one view, its key matched ignoring case (FBL section 8.7); without a view, the first. Of two
    /// keys that differ only in case, the first in the file applies.
    pub fn positions(&self, view: Option<&str>) -> BTreeMap<String, (f64, f64)> {
        let mut positions = BTreeMap::new();
        let Some(entry) = self.view(view) else {
            return positions;
        };
        let value = self.reading.value_of(entry);
        if value.kind != ValueKind::Mapping {
            return positions;
        }
        for &element in &value.entries {
            let Some(name) = &self.reading.entries[element].name else {
                continue;
            };
            let mapping = self.reading.entries[element].tree().value;
            if self.reading.tree_value(mapping).kind != ValueKind::Mapping {
                continue;
            }
            if positions.keys().any(|k: &String| k.eq_ignore_ascii_case(name)) {
                continue;
            }
            if let (Some(x), Some(y)) = (self.number(mapping, "x"), self.number(mapping, "y")) {
                positions.insert(name.clone(), (x, y));
            }
        }
        positions
    }

    /// The entries of a legacy identities file, in order (FBL 0.12 section 8.7); none when it is not one JSON array.
    pub(crate) fn identity_entries(&self) -> Option<Vec<IdentityEntry>> {
        let root = self.array()?;
        let text = |mapping: ValueId, key: &str| {
            let value = self.reading.value_of(self.reading.member(mapping, key)?);
            match &value.typed {
                Value::String(text) => Some((text.clone(), value.span)),
                _ => None,
            }
        };
        let mut entries = Vec::new();
        for &item in &self.reading.tree_value(root).entries {
            let mapping = self.reading.entries[item].tree().value;
            if self.reading.tree_value(mapping).kind != ValueKind::Mapping {
                continue;
            }
            let (Some((id, _)), Some((key, key_span))) = (text(mapping, "id"), text(mapping, "key")) else {
                continue;
            };
            entries.push(IdentityEntry {
                id,
                kind: text(mapping, "kind").map(|(kind, _)| kind),
                key,
                key_span,
                own: self.reading.entries[item].own,
            });
        }
        Some(entries)
    }

    /// What reading legacy identities reports (FBL 0.12 sections 8.5, 8.7): each entry whose kind no rule declares, which
    /// is kept, and each whose key names no element of `keys`, which the next write of the body prunes, as
    /// `fbl.stale-view-data`.
    pub(crate) fn identity_findings(
        &self,
        declared: &[String],
        keys: &HashSet<String>,
        file_name: &str,
    ) -> Vec<Finding> {
        let entries = self.identity_entries().unwrap_or_default();
        let mut findings = Vec::new();
        for entry in &entries {
            let message = if !entry.is_declared(declared) {
                format!(
                    "The identities file keeps an entry of the kind '{}', which no rule declares; it is kept as it is.",
                    entry.kind.as_deref().unwrap_or_default()
                )
            } else if !keys.contains(&entry.entry_key()) {
                format!(
                    "The identities file keeps an id for '{}', which the file no longer has; it is removed at the next write.",
                    entry.key
                )
            } else {
                continue;
            };
            findings.push(self.finding(
                codes::STALE_VIEW_DATA,
                FindingSeverity::Info,
                message,
                entry.own,
                file_name,
            ));
        }
        findings
    }

    /// One edit of legacy identities (FBL 0.12 section 8.7): `moved`, entries by their key and each's new one, a
    /// `replace-value` of `key`; `pruned`, a `remove-entry` with one separator; `new`, each an `insert-entry` after the
    /// array's last item, laid out as that item is.
    pub(crate) fn plan_identities(
        &self,
        moved: &[(String, String)],
        pruned: &[String],
        new: &[NewIdentity],
    ) -> PlanResult {
        let Some(entries) = self.identity_entries() else {
            return PlanResult::Refused("The identities file could not be read, so it is not written.".into());
        };
        let mut splices = Vec::new();
        for entry in &entries {
            let key = entry.entry_key();
            if let Some((_, to)) = moved.iter().find(|(from, _)| *from == key) {
                let natural = to.strip_prefix(&entry.kind_prefix()).unwrap_or(to);
                splices.push(Splice {
                    operation: Operation::ReplaceValue,
                    start: entry.key_span.start,
                    end: entry.key_span.end,
                    text: quote(natural),
                });
            }
        }
        let gone: Vec<usize> = entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| pruned.contains(&entry.entry_key()))
            .map(|(place, _)| place)
            .collect();
        let mut at = 0;
        while at < gone.len() {
            let first = gone[at];
            let mut last = first;
            while gone.get(at + 1) == Some(&(last + 1)) {
                at += 1;
                last = gone[at];
            }
            at += 1;
            let (start, end) = match (first.checked_sub(1), entries.get(last + 1)) {
                (Some(before), _) => (entries[before].own.end, entries[last].own.end),
                (None, Some(after)) => (entries[first].own.start, after.own.start),
                (None, None) => (entries[first].own.start, entries[last].own.end),
            };
            splices.push(Splice {
                operation: Operation::RemoveEntry,
                start,
                end,
                text: String::new(),
            });
        }
        splices.extend(self.new_identities(new));
        splices.sort_by_key(|s| (s.start, usize::from(s.start != s.end)));
        planned(splices)
    }

    /// The new entries after the array's last item, one line each when it is written on one line.
    fn new_identities(&self, new: &[NewIdentity]) -> Vec<Splice> {
        let Some(root) = self.array().filter(|_| !new.is_empty()) else {
            return Vec::new();
        };
        let newline = self.newline();
        let container = self.reading.tree_value(root);
        let empty = container.entries.is_empty();
        let (at, indent, one_line) = match container.entries.last() {
            Some(&last) => {
                let entry = &self.reading.entries[last];
                (entry.own.end, entry.indent, self.is_one_line(last))
            }
            None => (container.span.start + 1, self.column(container.span.start) + 2, false),
        };
        let step = container
            .entries
            .last()
            .and_then(|&last| {
                let mapping = self.reading.tree_value(self.reading.entries[last].tree().value);
                let member = &self.reading.entries[*mapping.entries.first()?];
                member.indent.checked_sub(indent)
            })
            .filter(|&step| step > 0)
            .unwrap_or(2);
        let mut splices = Vec::new();
        for (place, identity) in new.iter().enumerate() {
            let mut members = vec![format!("\"id\": {}", quote(&identity.id))];
            if let Some(kind) = &identity.kind {
                members.push(format!("\"kind\": {}", quote(kind)));
            }
            members.push(format!("\"key\": {}", quote(&identity.key)));
            let item = if one_line {
                format!("{{ {} }}", members.join(", "))
            } else {
                let inner = " ".repeat(indent + step);
                let members: Vec<String> = members.iter().map(|m| format!("{inner}{m}")).collect();
                format!(
                    "{{{newline}{}{newline}{}}}",
                    members.join(&format!(",{newline}")),
                    " ".repeat(indent)
                )
            };
            let before = if place > 0 || !empty { "," } else { "" };
            let mut text = if one_line {
                format!("{before} {item}")
            } else {
                format!("{before}{newline}{}{item}", " ".repeat(indent))
            };
            if empty && place + 1 == new.len() {
                text.push_str(newline);
                text.push_str(&" ".repeat(indent - 2));
            }
            splices.push(Splice {
                operation: Operation::InsertEntry,
                start: at,
                end: at,
                text,
            });
        }
        splices
    }

    /// What reading the view reports (FBL 0.11 section 8.7): each key after the first that matches one id, as
    /// `fbl.duplicate-key`, and each key that names no element of `ids`, as `fbl.stale-view-data`.
    pub fn findings(&self, view: Option<&str>, ids: &HashSet<String>, file_name: &str) -> Vec<Finding> {
        let mut findings = Vec::new();
        let Some(members) = self.view_members(view) else {
            return findings;
        };
        let mut seen: Vec<&str> = Vec::new();
        for &member in &members {
            let entry = &self.reading.entries[member];
            let Some(key) = entry.name.as_deref() else { continue };
            let at = entry.tree().key_span.unwrap_or(entry.own);
            if !names_element(ids, key) {
                findings.push(self.finding(
                    codes::STALE_VIEW_DATA,
                    FindingSeverity::Info,
                    format!(
                        "The layout file keeps a position for '{key}', which the file no longer has; it is removed at the next write."
                    ),
                    at,
                    file_name,
                ));
            } else if seen.iter().any(|s| s.eq_ignore_ascii_case(key)) {
                findings.push(self.finding(
                    codes::DUPLICATE_KEY,
                    FindingSeverity::Warning,
                    format!("Another key of this view already names '{key}'; this one is neither applied nor written."),
                    at,
                    file_name,
                ));
            } else {
                seen.push(key);
            }
        }
        findings
    }

    fn finding(&self, code: &str, severity: FindingSeverity, message: String, at: Span, file_name: &str) -> Finding {
        let (line, column) = self.reading.text.position(at.start);
        Finding {
            code: code.into(),
            severity,
            message,
            location: Some(SourceLocation {
                file: file_name.to_owned(),
                line,
                column,
                length: self.reading.text.code_points(at.start, at.end),
            }),
            detail: None,
        }
    }

    /// Places an element in a view (FBL 0.11 section 8.7): its numbers replaced in place, written shortest, or a new
    /// member after the view's last, laid out as that one is, the view's object made first when the file has none. An
    /// entry of the view whose key names no element of `known`, when known, goes in the same edit.
    pub fn plan_place(
        &self,
        view: Option<&str>,
        id: &str,
        x: f64,
        y: f64,
        known: Option<&HashSet<String>>,
    ) -> PlanResult {
        let Some(root) = self.root() else {
            return PlanResult::Refused("The layout file could not be read, so it is not written.".into());
        };
        let x_text = new_text::number(x, None);
        let y_text = new_text::number(y, None);
        let Some(view_entry) = self.view(view) else {
            let Some(view) = view else {
                return PlanResult::Refused("The layout file has no view to place the element in.".into());
            };
            return planned(self.new_view(root, view, id, &x_text, &y_text));
        };
        let view_value = self.reading.entries[view_entry].tree().value;
        if self.reading.tree_value(view_value).kind != ValueKind::Mapping {
            return PlanResult::Refused("The layout file's view is not an object.".into());
        }
        let members = self.reading.tree_value(view_value).entries.clone();
        let stale: Vec<EntryId> = known.map_or(Vec::new(), |known| {
            members
                .iter()
                .copied()
                .filter(|&m| {
                    self.reading.entries[m]
                        .name
                        .as_deref()
                        .is_some_and(|key| !key.eq_ignore_ascii_case(id) && !names_element(known, key))
                })
                .collect()
        });
        let mut splices = self.without(&members, &stale);
        let existing = members.iter().copied().find(|&m| {
            self.reading.entries[m]
                .name
                .as_deref()
                .is_some_and(|key| key.eq_ignore_ascii_case(id))
        });
        if let Some(existing) = existing {
            let existing = self.reading.entries[existing].tree().value;
            if self.reading.tree_value(existing).kind == ValueKind::Mapping
                && let (Some(x_member), Some(y_member)) =
                    (self.scalar_member(existing, "x"), self.scalar_member(existing, "y"))
            {
                for (member, text) in [(x_member, &x_text), (y_member, &y_text)] {
                    if self.reading.text.text_of(member.span) != text.as_str() {
                        splices.push(Splice {
                            operation: Operation::ReplaceValue,
                            start: member.span.start,
                            end: member.span.end,
                            text: text.clone(),
                        });
                    }
                }
                splices.sort_by_key(|s| s.start);
                return planned(splices);
            }
        }
        let kept: Vec<EntryId> = members.iter().copied().filter(|m| !stale.contains(m)).collect();
        // What the new member follows: the last that stays, else it stands where the first that goes stood.
        let (after, one_line) = match kept.last().or(members.last()) {
            Some(&last) => (
                Some(last),
                self.reading.entries[last].line_span.is_none() || self.is_one_line(last),
            ),
            None => (None, false),
        };
        let quoted = quote(id);
        let new = |indent: usize| {
            if one_line {
                format!("{quoted}: {{ \"x\": {x_text}, \"y\": {y_text} }}")
            } else {
                format!("{quoted}: {}", self.position(indent, &x_text, &y_text))
            }
        };
        match after {
            Some(after) if !kept.is_empty() => {
                let last = &self.reading.entries[after];
                let text = if last.line_span.is_some() {
                    format!(",{}{}{}", self.newline(), " ".repeat(last.indent), new(last.indent))
                } else {
                    format!(", {}", new(last.indent))
                };
                splices.push(Splice {
                    operation: Operation::InsertEntry,
                    start: last.own.end,
                    end: last.own.end,
                    text,
                });
            }
            Some(after) => {
                let first = self.reading.entries[members[0]].own.start;
                let indent = self.reading.entries[after].indent;
                splices.insert(
                    0,
                    Splice {
                        operation: Operation::InsertEntry,
                        start: first,
                        end: first,
                        text: new(indent),
                    },
                );
            }
            None => {
                return self.insert(view_value, &quoted, &|indent| self.position(indent, &x_text, &y_text));
            }
        }
        splices.sort_by_key(|s| (s.start, s.end));
        planned(splices)
    }

    /// The splices that take `gone` out of `members`, each with one separator: the one after it, or, for those that
    /// end the object, the one before them with the whitespace up to them; all of them when none stays.
    fn without(&self, members: &[EntryId], gone: &[EntryId]) -> Vec<Splice> {
        let own = |m: EntryId| self.reading.entries[m].own;
        let remove = |start: usize, end: usize| Splice {
            operation: Operation::RemoveEntry,
            start,
            end,
            text: String::new(),
        };
        let mut splices = Vec::new();
        let last_kept = members.iter().rposition(|m| !gone.contains(m));
        let Some(last_kept) = last_kept else {
            if let (Some(&first), Some(&last)) = (members.first(), members.last()) {
                splices.push(remove(own(first).start, own(last).end));
            }
            return splices;
        };
        for (index, &member) in members.iter().enumerate().take(last_kept) {
            if gone.contains(&member) {
                splices.push(remove(own(member).start, own(members[index + 1]).start));
            }
        }
        if last_kept + 1 < members.len() {
            let last = members[members.len() - 1];
            splices.push(remove(own(members[last_kept]).end, own(last).end));
        }
        splices
    }

    /// The object of a new view at the end of the root (FBL 0.11 section 8.7): its opening and closing each an
    /// `ensure-container`, around the member's `insert-entry`, all at one offset.
    fn new_view(&self, root: ValueId, view: &str, id: &str, x: &str, y: &str) -> Vec<Splice> {
        let container = self.reading.tree_value(root);
        let newline = self.newline();
        let step = self.step();
        let last = container.entries.last().map(|&l| &self.reading.entries[l]);
        let (at, lead, indent) = match last {
            Some(last) => (last.own.end, ",", last.indent),
            None => (container.span.start + 1, "", self.column(container.span.start) + step),
        };
        let member = indent + step;
        let splice = |operation: Operation, text: String| Splice {
            operation,
            start: at,
            end: at,
            text,
        };
        vec![
            splice(
                Operation::EnsureContainer,
                format!("{lead}{newline}{}{}: {{", " ".repeat(indent), quote(view)),
            ),
            splice(
                Operation::InsertEntry,
                format!(
                    "{newline}{}{}: {}",
                    " ".repeat(member),
                    quote(id),
                    self.position(member, x, y)
                ),
            ),
            splice(
                Operation::EnsureContainer,
                match last {
                    Some(_) => format!("{newline}{}}}", " ".repeat(indent)),
                    None => format!(
                        "{newline}{}}}{newline}{}",
                        " ".repeat(indent),
                        " ".repeat(indent - step)
                    ),
                },
            ),
        ]
    }

    /// Whether a member is written on one line.
    fn is_one_line(&self, member: EntryId) -> bool {
        !self
            .reading
            .text
            .text_of(self.reading.entries[member].own)
            .contains('\n')
    }

    /// The members of one view, its key matched ignoring case; none when the file has no such view.
    fn view_members(&self, view: Option<&str>) -> Option<Vec<EntryId>> {
        let value = self.reading.value_of(self.view(view)?);
        (value.kind == ValueKind::Mapping).then(|| value.entries.clone())
    }

    /// Plans a change with `plan` and applies it: the planned edit, or the refusal with nothing written.
    pub fn change(&mut self, plan: impl FnOnce(&LegacySidecar) -> PlanResult) -> PlanResult {
        let result = plan(self);
        if let PlanResult::Planned(edit) = &result {
            match self.file.apply(edit) {
                Ok(true) => self.reread(),
                Ok(false) => {}
                Err(problem) => return PlanResult::Invalid(problem),
            }
        }
        result
    }

    pub fn undo(&mut self, current: Option<&[u8]>) -> UndoResult {
        let result = self.file.undo(current);
        if let UndoResult::Done(_) = result {
            self.reread();
        }
        result
    }

    pub fn redo(&mut self, current: Option<&[u8]>) -> UndoResult {
        let result = self.file.redo(current);
        if let UndoResult::Done(_) = result {
            self.reread();
        }
        result
    }

    fn reread(&mut self) {
        self.reading = read(self.file.bytes());
    }

    /// Applies an edit that is no step of the history (FBL 0.12 section 8.6, an id first stored).
    pub(crate) fn apply_outside_history(&mut self, edit: &PlannedEdit) {
        if matches!(self.file.apply_outside_history(edit), Ok(true)) {
            self.reread();
        }
    }

    /// The root array's value; none when the sidecar is not one JSON array.
    fn array(&self) -> Option<ValueId> {
        if self.reading.unreadable.is_some() {
            return None;
        }
        let root = self.reading.entries[self.reading.tree_root()?].tree().value;
        (self.reading.tree_value(root).kind == ValueKind::Sequence).then_some(root)
    }

    /// The root object's value; none when the sidecar is not one JSON object.
    fn root(&self) -> Option<ValueId> {
        if self.reading.unreadable.is_some() {
            return None;
        }
        let root = self.reading.entries[self.reading.tree_root()?].tree().value;
        (self.reading.tree_value(root).kind == ValueKind::Mapping).then_some(root)
    }

    fn view(&self, view: Option<&str>) -> Option<EntryId> {
        let root = self.root()?;
        let mut members = self.reading.tree_value(root).entries.iter().copied();
        match view {
            None => members.next(),
            Some(view) => members.find(|&m| {
                self.reading.entries[m]
                    .name
                    .as_deref()
                    .is_some_and(|name| name.eq_ignore_ascii_case(view))
            }),
        }
    }

    fn number(&self, mapping: ValueId, key: &str) -> Option<f64> {
        match &self.reading.value_of(self.reading.member(mapping, key)?).typed {
            Value::Int(i) => Some(*i as f64),
            Value::Double(d) => Some(*d),
            _ => None,
        }
    }

    fn scalar_member(&self, mapping: ValueId, key: &str) -> Option<&TreeValue> {
        let value = self.reading.value_of(self.reading.member(mapping, key)?);
        (value.kind == ValueKind::Scalar).then_some(value)
    }

    /// The indentation step of the file: a member's indentation minus its object's, else two.
    fn step(&self) -> usize {
        for &id in &self.reading.order {
            let entry = &self.reading.entries[id];
            let (EntryKind::Tree(_), Some(_), Some(parent)) = (&entry.kind, entry.line_span, entry.parent) else {
                continue;
            };
            let parent = &self.reading.entries[parent];
            if let EntryKind::Tree(tree) = &parent.kind
                && !tree.is_root
                && parent.line_span.is_some()
                && entry.indent > parent.indent
            {
                return entry.indent - parent.indent;
            }
        }
        2
    }

    fn newline(&self) -> &'static str {
        self.reading.text.dominant_ending().unwrap_or("\n")
    }

    fn position(&self, indent: usize, x: &str, y: &str) -> String {
        let newline = self.newline();
        let inner = " ".repeat(indent + self.step());
        format!(
            "{{{newline}{inner}\"x\": {x},{newline}{inner}\"y\": {y}{newline}{}}}",
            " ".repeat(indent)
        )
    }

    /// A new member after the object's last, written as the file writes its members (FBL section 6.3, json values).
    fn insert(&self, container: ValueId, key: &str, value: &dyn Fn(usize) -> String) -> PlanResult {
        let container = self.reading.tree_value(container);
        let newline = self.newline();
        let Some(&last) = container.entries.last() else {
            let column = self.column(container.span.start);
            let indent = column + self.step();
            let text = format!(
                "{newline}{}{key}: {}{newline}{}",
                " ".repeat(indent),
                value(indent),
                " ".repeat(column)
            );
            let at = container.span.start + 1;
            return planned(vec![Splice {
                operation: Operation::InsertEntry,
                start: at,
                end: at,
                text,
            }]);
        };
        let last = &self.reading.entries[last];
        let inserted = if last.line_span.is_some() {
            format!(",{newline}{}{key}: {}", " ".repeat(last.indent), value(last.indent))
        } else {
            format!(", {key}: {}", value(last.indent))
        };
        planned(vec![Splice {
            operation: Operation::InsertEntry,
            start: last.own.end,
            end: last.own.end,
            text: inserted,
        }])
    }

    fn column(&self, offset: usize) -> usize {
        offset - self.reading.text.lines()[self.reading.text.line_index_at(offset)].start
    }
}

/// Whether a key names an element of `ids`, ignoring the case of ASCII letters (FBL 0.11 section 8.7).
fn names_element(ids: &HashSet<String>, key: &str) -> bool {
    ids.contains(key) || ids.iter().any(|id| id.eq_ignore_ascii_case(key))
}

fn planned(splices: Vec<Splice>) -> PlanResult {
    PlanResult::Planned(PlannedEdit {
        splices,
        snapshot: false,
    })
}

fn read(bytes: &[u8]) -> Family {
    let mut family = Family::create(
        BodyText::new(bytes.to_vec()),
        Rc::new(sidecar_binding()),
        Rc::new(FblOptions::default()),
    );
    family.parse();
    family
}

/// The standalone host's sidecar tests at 8b6ccfc, ported from `Registration/Registration.Tests.cs`.
#[cfg(test)]
mod tests {
    use super::*;

    fn planned_ok(result: &PlanResult) -> bool {
        matches!(result, PlanResult::Planned(_))
    }

    #[test]
    fn a_legacy_layout_is_read_for_its_view_ignoring_case() {
        let sidecar =
            LegacySidecar::open(b"{\n  \"SystemContext\": {\n    \"a\": { \"x\": 40, \"y\": 60.5 }\n  }\n}\n".to_vec());

        let positions = sidecar.positions(Some("systemcontext"));

        assert_eq!(positions["a"], (40.0, 60.5));
        assert!(sidecar.positions(Some("Containers")).is_empty());
    }

    #[test]
    fn a_legacy_layout_is_written_by_splices_and_undone() {
        let original = "{\n  \"SystemContext\": {\n    \"a\": {\n      \"x\": 40,\n      \"y\": 60\n    }\n  }\n}\n";
        let mut sidecar = LegacySidecar::open(original.as_bytes().to_vec());

        let moved = sidecar.change(|s| s.plan_place(Some("systemContext"), "a", 41.25, 60.0, None));
        let added = sidecar.change(|s| s.plan_place(Some("SystemContext"), "b", 10.0, 20.5, None));

        // Numbers replaced in place, a new member written as the file writes them.
        assert!(planned_ok(&moved), "{moved:?}");
        assert!(planned_ok(&added), "{added:?}");
        assert_eq!(
            String::from_utf8_lossy(sidecar.bytes()),
            "{\n  \"SystemContext\": {\n    \"a\": {\n      \"x\": 41.25,\n      \"y\": 60\n    },\n    \"b\": {\n      \"x\": 10,\n      \"y\": 20.5\n    }\n  }\n}\n"
        );
        assert_eq!(sidecar.positions(Some("SystemContext"))["b"], (10.0, 20.5));
        assert!(matches!(sidecar.undo(None), UndoResult::Done(_)));
        assert!(matches!(sidecar.undo(None), UndoResult::Done(_)));
        assert_eq!(String::from_utf8_lossy(sidecar.bytes()), original);
    }

    #[test]
    fn a_view_the_layout_does_not_have_is_added_and_an_empty_view_is_filled() {
        let mut sidecar = LegacySidecar::open(b"{\n  \"Context\": {}\n}\n".to_vec());

        let filled = sidecar.change(|s| s.plan_place(Some("Context"), "a", 1.0, 2.0, None));
        let added = sidecar.change(|s| s.plan_place(Some("Containers"), "b", 3.0, 4.0, None));

        assert!(planned_ok(&filled), "{filled:?}");
        assert!(planned_ok(&added), "{added:?}");
        assert_eq!(sidecar.positions(Some("Context"))["a"], (1.0, 2.0));
        assert_eq!(sidecar.positions(Some("containers"))["b"], (3.0, 4.0));
        assert_eq!(
            LegacySidecar::open(b"{}".to_vec()).plan_place(None, "a", 1.0, 2.0, None),
            PlanResult::Refused("The layout file has no view to place the element in.".into())
        );
    }

    /// FBL 0.12 section 8.7: the array the tools write, a key moved by a `replace-value` with RFC 8785's escapes, a pruned
    /// entry with one separator, a new one laid out as the last, and a kind no rule declares reported and kept.
    #[test]
    fn legacy_identities_are_read_and_written() {
        let original = "[\r\n  {\r\n    \"id\": \"c1\",\r\n    \"kind\": \"component\",\r\n    \"key\": \"Tea\"\r\n  },\r\n  {\r\n    \"id\": \"c2\",\r\n    \"kind\": \"other\",\r\n    \"key\": \"Cup\"\r\n  }\r\n]\r\n";
        let mut sidecar = LegacySidecar::open(original.as_bytes().to_vec());

        let entries = sidecar.identity_entries().unwrap();
        assert_eq!(entries[0].entry_key(), "component:Tea");
        let findings = sidecar.identity_findings(
            &["component".to_owned()],
            &HashSet::from(["component:Tea".to_owned()]),
            "x.json",
        );
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].location.as_ref().unwrap().line, 7);

        let moved = [("component:Tea".to_owned(), "component:Tea\u{1f}2\t".to_owned())];
        let new = [NewIdentity {
            id: "c3".into(),
            kind: Some("component".into()),
            key: "Pot".into(),
        }];
        let result = sidecar.change(|s| s.plan_identities(&moved, &[], &new));

        assert!(planned_ok(&result), "{result:?}");
        assert_eq!(
            String::from_utf8_lossy(sidecar.bytes()),
            "[\r\n  {\r\n    \"id\": \"c1\",\r\n    \"kind\": \"component\",\r\n    \"key\": \"Tea\\u001f2\\t\"\r\n  },\r\n  {\r\n    \"id\": \"c2\",\r\n    \"kind\": \"other\",\r\n    \"key\": \"Cup\"\r\n  },\r\n  {\r\n    \"id\": \"c3\",\r\n    \"kind\": \"component\",\r\n    \"key\": \"Pot\"\r\n  }\r\n]\r\n"
        );
        assert!(matches!(sidecar.undo(None), UndoResult::Done(_)));
        assert_eq!(String::from_utf8_lossy(sidecar.bytes()), original);

        let pruned = sidecar.change(|s| s.plan_identities(&[], &["component:Tea".to_owned()], &[]));
        assert!(planned_ok(&pruned), "{pruned:?}");
        assert_eq!(
            String::from_utf8_lossy(sidecar.bytes()),
            "[\r\n  {\r\n    \"id\": \"c2\",\r\n    \"kind\": \"other\",\r\n    \"key\": \"Cup\"\r\n  }\r\n]\r\n"
        );
    }

    /// FBL section 8.7: the object FBL 0.9 described is not read.
    #[test]
    fn an_object_of_identities_is_not_read() {
        assert_eq!(
            LegacySidecar::open(b"{ \"Tea\": \"c1\" }".to_vec()).identity_entries(),
            None
        );
    }

    /// An empty array takes its first entry between its brackets, a one-line one takes a new entry on the line.
    #[test]
    fn a_new_identity_is_laid_out_as_the_file_is() {
        let new = [NewIdentity {
            id: "c".into(),
            kind: None,
            key: "K".into(),
        }];
        let mut empty = LegacySidecar::open(b"[]\n".to_vec());
        assert!(planned_ok(&empty.change(|s| s.plan_identities(&[], &[], &new))));
        assert_eq!(
            String::from_utf8_lossy(empty.bytes()),
            "[\n  {\n    \"id\": \"c\",\n    \"key\": \"K\"\n  }\n]\n"
        );
        let mut line = LegacySidecar::open(b"[{ \"id\": \"a\", \"key\": \"A\" }]\n".to_vec());
        assert!(planned_ok(&line.change(|s| s.plan_identities(&[], &[], &new))));
        assert_eq!(
            String::from_utf8_lossy(line.bytes()),
            "[{ \"id\": \"a\", \"key\": \"A\" }, { \"id\": \"c\", \"key\": \"K\" }]\n"
        );
    }

    #[test]
    fn an_unreadable_sidecar_refuses_a_change_and_is_not_written() {
        let original = "[ \"not an object\" ]\n";
        let mut sidecar = LegacySidecar::open(original.as_bytes().to_vec());

        let result = sidecar.change(|s| s.plan_place(Some("SystemContext"), "a", 1.0, 2.0, None));

        // The refusal is the sentence a host shows (FBL section 8.7).
        assert!(sidecar.is_unreadable());
        assert_eq!(
            result,
            PlanResult::Refused("The layout file could not be read, so it is not written.".into())
        );
        assert_eq!(String::from_utf8_lossy(sidecar.bytes()), original);
    }

    #[test]
    fn the_sidecar_path_is_beside_the_body_with_its_base_name() {
        let path = LegacySidecar::path_for("{base}.layout.json", &Path::new("x").join("bottling-mes.dsl"));

        assert_eq!(path, std::path::absolute("x").unwrap().join("bottling-mes.layout.json"));
    }

    /// FBL 0.11 section 8.7: a number written shortest, not rounded; a key a later key repeats in another case; a
    /// member laid out on one line as the one before it is; and a stale member going in the same edit.
    #[test]
    fn a_view_is_written_shortest_and_its_stale_entries_go() {
        let known = HashSet::from(["a".to_owned(), "b".to_owned()]);
        let original = "{\n  \"v\": {\n    \"gone\": { \"x\": 1, \"y\": 2 },\n    \"A\": { \"x\": 1, \"y\": 2 },\n    \"a\": { \"x\": 3, \"y\": 4 }\n  }\n}\n";
        let mut sidecar = LegacySidecar::open(original.as_bytes().to_vec());

        let findings = sidecar.findings(Some("V"), &known, "x.json");
        assert_eq!(
            findings.iter().map(|f| f.code.as_str()).collect::<Vec<_>>(),
            [codes::STALE_VIEW_DATA, codes::DUPLICATE_KEY]
        );
        assert_eq!(sidecar.positions(Some("v"))["A"], (1.0, 2.0));

        let placed = sidecar.change(|s| s.plan_place(Some("v"), "b", 1.25, 0.0004, Some(&known)));

        assert!(planned_ok(&placed), "{placed:?}");
        assert_eq!(
            String::from_utf8_lossy(sidecar.bytes()),
            "{\n  \"v\": {\n    \"A\": { \"x\": 1, \"y\": 2 },\n    \"a\": { \"x\": 3, \"y\": 4 },\n    \"b\": { \"x\": 1.25, \"y\": 0.0004 }\n  }\n}\n"
        );
        assert!(matches!(sidecar.undo(None), UndoResult::Done(_)));
        assert_eq!(String::from_utf8_lossy(sidecar.bytes()), original);
    }

    #[test]
    fn a_view_the_file_does_not_have_is_made_around_its_first_member() {
        let original = "{\n  \"v\": {}\n}\n";
        let mut sidecar = LegacySidecar::open(original.as_bytes().to_vec());

        let placed = sidecar.change(|s| s.plan_place(Some("w"), "a", 1.0, 2.0, None));

        let PlanResult::Planned(edit) = &placed else {
            panic!("{placed:?}");
        };
        assert_eq!(
            edit.splices.iter().map(|s| s.operation).collect::<Vec<_>>(),
            [
                Operation::EnsureContainer,
                Operation::InsertEntry,
                Operation::EnsureContainer
            ]
        );
        assert_eq!(sidecar.positions(Some("w"))["a"], (1.0, 2.0));
    }
}
