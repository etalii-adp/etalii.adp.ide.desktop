//! A sidecar file hosts wrote before FBL (FBL section 8.7; the standalone host's `Registration/LegacySidecar`): a
//! legacy layout, keyed by view key and then by element id with `{"x", "y"}`, or legacy identities, a natural key
//! mapped to an id. It is read when the registration has no matching block and written back by json splices while
//! it exists; this crate never creates one.

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

    /// Legacy identities: natural key to id.
    pub fn identities(&self) -> BTreeMap<String, String> {
        let mut identities = BTreeMap::new();
        let Some(root) = self.root() else {
            return identities;
        };
        for &entry in &self.reading.tree_value(root).entries {
            if let (Some(name), Value::String(id)) =
                (&self.reading.entries[entry].name, &self.reading.value_of(entry).typed)
            {
                identities.insert(name.clone(), id.clone());
            }
        }
        identities
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

    /// Stores an id for a natural key: replaced in place, or a new member at the end of the object.
    pub fn plan_identify(&self, key: &str, id: &str) -> PlanResult {
        let Some(root) = self.root() else {
            return PlanResult::Refused("The identities file could not be read, so it is not written.".into());
        };
        let text = quote(id);
        if let Some(existing) = self.reading.member(root, key) {
            let span = self.reading.value_of(existing).span;
            if self.reading.text.text_of(span) == text {
                return planned(Vec::new());
            }
            return planned(vec![Splice {
                operation: Operation::ReplaceValue,
                start: span.start,
                end: span.end,
                text,
            }]);
        }
        self.insert(root, &quote(key), &|_| text.clone())
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

    #[test]
    fn legacy_identities_are_read_and_written() {
        let mut sidecar = LegacySidecar::open(b"{\r\n  \"Tea\": \"c1\"\r\n}\r\n".to_vec());

        let result = sidecar.change(|s| s.plan_identify("Cup", "c2"));

        assert!(planned_ok(&result), "{result:?}");
        assert_eq!(
            String::from_utf8_lossy(sidecar.bytes()),
            "{\r\n  \"Tea\": \"c1\",\r\n  \"Cup\": \"c2\"\r\n}\r\n"
        );
        assert_eq!(sidecar.identities()["Cup"], "c2");
        let same = sidecar.change(|s| s.plan_identify("Cup", "c2"));
        assert_eq!(same, PlanResult::Planned(PlannedEdit::default()));
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
