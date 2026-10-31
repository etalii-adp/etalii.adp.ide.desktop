//! A registration (`.adp`) in the line form of FBL section 8.1 (the standalone host's `Registration/
//! RegistrationDocument` and `OpenRegistration`): the origin on line 1, headers, the `layout:` and `identities:`
//! blocks and the blocks the binding declares, and whatever follows them kept as unbound content. A block the binding
//! does not declare is read past and kept as it is (FBL 0.7 section 8.1). Every part keeps its span, so the registration is
//! written by splices like any body (FBL section 8.4): placing an element writes its layout entry in the ordinal
//! order of ids, with numbers in `{decimals: 3}` form and the registration's own line ending (FBL section 8.3).

mod blocks;
#[cfg(test)]
mod blocks_tests;
pub mod document;
pub mod legacy_sidecar;

use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::LazyLock;

use regex::Regex;

use crate::documents::types::{FblBinding, RegistrationBlockDeclaration};

use crate::history::{SplicedFile, UndoResult};
use crate::model::{Finding, FindingSeverity, SourceLocation, codes};
use crate::planning::new_text;
use crate::planning::{ModelChange, PlanResult, PlannedEdit};
use crate::splice::{Operation, Splice};
use crate::text::{BodyText, Span};

/// The headers FBL itself defines (FBL section 8.1); a binding declares others.
const FBL_HEADERS: [&str; 3] = ["body", "view", "resource"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistrationHeader {
    pub key: String,
    pub value: String,
    pub line: Span,
}

/// One `key: value` entry of a block, the key being everything before the line's last `": "`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistrationEntry {
    pub key: String,
    pub value: String,
    pub key_span: Span,
    pub value_span: Span,
    pub line: Span,
    pub indent: usize,
}

/// A block: its name line, its whole span, and its entries in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistrationBlock {
    pub name: String,
    pub name_line: Span,
    pub span: Span,
    pub entries: Vec<RegistrationEntry>,
}

/// A registration as read. The line form has no syntax error: anything not understood is unbound content.
#[derive(Debug, Clone)]
pub struct RegistrationDocument {
    text: BodyText,
    /// The origin of the tool type (FBL section 8.1, line 1).
    pub origin: String,
    pub headers: Vec<RegistrationHeader>,
    pub layout: Option<RegistrationBlock>,
    pub identities: Option<RegistrationBlock>,
    /// Every block read, in the order the registration holds them, those the binding does not declare included.
    pub blocks: Vec<RegistrationBlock>,
    /// The names of the blocks neither FBL nor the binding declares (FBL 0.7 section 8.1): kept as they are written.
    pub undeclared: Vec<String>,
    /// Where a new block goes: the end of the header region's last line.
    pub after_headers: usize,
}

/// A line that opens a block (FBL section 8.1): a Name and a colon, with nothing but whitespace after it.
static BLOCK_LINE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?<name>[A-Za-z_][A-Za-z0-9_.-]*):[ \t]*$").expect("the block line pattern"));

fn block_name(content: &str) -> Option<&str> {
    BLOCK_LINE
        .captures(content)
        .and_then(|c| c.name("name"))
        .map(|m| m.as_str())
}

impl RegistrationDocument {
    /// Reads a registration whose binding declares no block.
    pub fn read(bytes: Vec<u8>) -> RegistrationDocument {
        Self::read_declared(bytes, &[])
    }

    /// Reads a registration whose binding declares the blocks `declared` (FBL 0.7 section 8.1).
    pub fn read_declared(bytes: Vec<u8>, declared: &[String]) -> RegistrationDocument {
        let text = BodyText::new(bytes);
        let lines = text.lines().to_vec();
        let origin = text.text(text.bom_length(), lines[0].content_end).trim().to_owned();
        let mut after_headers = lines[0].end;
        let mut index = 1;
        let mut headers = Vec::new();
        while index < lines.len() {
            let line = lines[index];
            let content = text.text(line.start, line.content_end);
            if content.trim().is_empty() {
                index += 1;
                continue;
            }
            if block_name(content).is_some() {
                break;
            }
            let Some(separator) = content.find(": ") else { break };
            if separator == 0 || content.starts_with(char::is_whitespace) {
                break;
            }
            headers.push(RegistrationHeader {
                key: content[..separator].to_owned(),
                value: content[separator + 2..].trim().to_owned(),
                line: Span::new(line.start, line.end),
            });
            after_headers = line.end;
            index += 1;
        }
        let mut document = RegistrationDocument {
            origin,
            headers,
            layout: None,
            identities: None,
            blocks: Vec::new(),
            undeclared: Vec::new(),
            after_headers,
            text,
        };
        while index < lines.len() {
            let line = lines[index];
            let content = document.text.text(line.start, line.content_end);
            if content.trim().is_empty() {
                index += 1;
                continue;
            }
            let Some(name) = block_name(content).map(str::to_owned) else {
                break;
            };
            // A second block of one name ends the blocks: it and what follows are unbound content.
            if document.blocks.iter().any(|b| b.name == name) {
                break;
            }
            let block = read_block(&document.text, &name, &mut index);
            if name == "layout" {
                document.layout = Some(block.clone());
            } else if name == "identities" {
                document.identities = Some(block.clone());
            } else if !declared.contains(&name) {
                document.undeclared.push(name);
            }
            document.blocks.push(block);
        }
        document
    }

    /// The block `name`, when the registration has it and it is FBL's or declared.
    pub fn block(&self, name: &str) -> Option<&RegistrationBlock> {
        self.blocks
            .iter()
            .find(|b| b.name == name && !self.undeclared.contains(&b.name))
    }

    pub(crate) fn text(&self) -> &BodyText {
        &self.text
    }

    fn header(&self, key: &str) -> Option<&str> {
        self.headers.iter().find(|h| h.key == key).map(|h| h.value.as_str())
    }

    pub fn body(&self) -> Option<&str> {
        self.header("body")
    }

    pub fn view(&self) -> Option<&str> {
        self.header("view")
    }

    pub fn resource(&self) -> Option<&str> {
        self.header("resource")
    }

    /// The identities block as natural key to id (FBL section 8.6).
    pub fn identity_map(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        for entry in self.identities.iter().flat_map(|b| &b.entries) {
            map.entry(entry.key.clone()).or_insert_with(|| entry.value.clone());
        }
        map
    }

    /// The headers neither FBL nor the binding declares, reported as `fbl.unknown-header` (FBL section 8.1).
    pub fn unknown_headers(&self, declared: &[String], file_name: &str) -> Vec<Finding> {
        self.headers
            .iter()
            .filter(|h| !FBL_HEADERS.contains(&h.key.as_str()) && !declared.contains(&h.key))
            .map(|h| {
                let (line, column) = self.text.position(h.line.start);
                Finding {
                    code: codes::UNKNOWN_HEADER.into(),
                    severity: FindingSeverity::Info,
                    message: format!(
                        "The header '{}' is neither FBL's nor the binding's; it is kept as it is.",
                        h.key
                    ),
                    location: Some(SourceLocation {
                        file: file_name.to_owned(),
                        line,
                        column,
                        length: self.text.code_points(h.line.start, h.line.end),
                    }),
                    detail: None,
                }
            })
            .collect()
    }

    /// *(FBL 0.10)* The finding of a `resource` header that names a resource the body does not hold, at the header's
    /// line (FBL sections 7.4, 8.2).
    pub fn missing_resource(&self, file_name: &str) -> Option<Finding> {
        let header = self.headers.iter().find(|h| h.key == "resource")?;
        let (line, column) = self.text.position(header.line.start);
        Some(Finding {
            code: codes::MISSING_RESOURCE.into(),
            severity: FindingSeverity::Warning,
            message: format!(
                "The body holds no resource '{}', so this reading is empty.",
                header.value
            ),
            location: Some(SourceLocation {
                file: file_name.to_owned(),
                line,
                column,
                length: self.text.code_points(header.line.start, header.line.end),
            }),
            detail: None,
        })
    }

    /// The blocks neither FBL nor the binding declares, reported once each as `fbl.unknown-block` (FBL 0.7 section
    /// 8.1).
    pub fn unknown_blocks(&self, file_name: &str) -> Vec<Finding> {
        self.blocks
            .iter()
            .filter(|b| self.undeclared.contains(&b.name))
            .map(|b| {
                self.located(
                    codes::UNKNOWN_BLOCK,
                    format!(
                        "The block '{}' is neither FBL's nor the binding's; it is kept as it is.",
                        b.name
                    ),
                    Span::new(b.name_line.start, b.name_line.start + b.name.len() + 1),
                    file_name,
                )
            })
            .collect()
    }

    /// Layout entries for ids the model does not have (FBL section 8.5), and the entries of the declared blocks
    /// keyed by ids (FBL 0.7 section 8.1): reported as `fbl.stale-view-data`, applied to nothing, and removed at the
    /// registration's next write.
    /// The entries of the `identities` block whose key names no element (FBL 0.12 section 8.6, as section 8.5 has it of
    /// a layout entry), reported as `fbl.stale-view-data`.
    pub(crate) fn stale_identities(&self, keys: &HashSet<String>, file_name: &str) -> Vec<Finding> {
        self.identities
            .iter()
            .flat_map(|block| &block.entries)
            .filter(|entry| !keys.contains(&entry.key))
            .map(|entry| {
                self.located(
                    codes::STALE_VIEW_DATA,
                    format!(
                        "The identities keep an id for '{}', which the file no longer has; it is removed at the next write of the body.",
                        entry.key
                    ),
                    entry.key_span,
                    file_name,
                )
            })
            .collect()
    }

    pub fn stale_entries(
        &self,
        ids: &HashSet<String>,
        file_name: &str,
        declarations: &[RegistrationBlockDeclaration],
    ) -> Vec<Finding> {
        let mut findings: Vec<Finding> = self
            .layout
            .iter()
            .flat_map(|b| &b.entries)
            .filter(|e| !ids.contains(&e.key))
            .map(|e| {
                self.located(
                    codes::STALE_VIEW_DATA,
                    format!(
                        "The layout keeps a position for '{}', which the file no longer has; it is removed at the next save of the registration.",
                        e.key
                    ),
                    e.key_span,
                    file_name,
                )
            })
            .collect();
        for declaration in declarations.iter().filter(|d| d.keyed_by_ids) {
            let Some(block) = self.block(&declaration.name) else {
                continue;
            };
            for entry in block.entries.iter().filter(|e| !ids.contains(&e.key)) {
                findings.push(self.located(
                    codes::STALE_VIEW_DATA,
                    format!(
                        "The block '{}' keeps an entry for '{}', which the file no longer has; it is removed at the next save of the registration.",
                        block.name, entry.key
                    ),
                    entry.key_span,
                    file_name,
                ));
            }
        }
        findings
    }

    fn located(&self, code: &str, message: String, span: Span, file_name: &str) -> Finding {
        let (line, column) = self.text.position(span.start);
        Finding {
            code: code.into(),
            severity: FindingSeverity::Info,
            message,
            location: Some(SourceLocation {
                file: file_name.to_owned(),
                line,
                column,
                length: self.text.code_points(span.start, span.end),
            }),
            detail: None,
        }
    }
}

fn read_block(text: &BodyText, name: &str, index: &mut usize) -> RegistrationBlock {
    let lines = text.lines();
    let header = lines[*index];
    let mut entries = Vec::new();
    let mut end = header.end;
    *index += 1;
    while *index < lines.len() {
        let line = lines[*index];
        let content = text.text(line.start, line.content_end);
        if content.is_empty() || !content.starts_with(char::is_whitespace) || content.trim().is_empty() {
            break;
        }
        let Some(separator) = content.rfind(": ") else { break };
        let key_start = content.len() - content.trim_start().len();
        let key = &content[key_start..separator.max(key_start)];
        let value_text = &content[separator + 2..];
        let key_offset = line.start + key_start;
        let value_offset = line.start + separator + 2;
        entries.push(RegistrationEntry {
            key: key.to_owned(),
            value: value_text.trim().to_owned(),
            key_span: Span::new(key_offset, key_offset + key.len()),
            value_span: Span::new(value_offset, value_offset + value_text.trim_end().len()),
            line: Span::new(line.start, line.end),
            indent: key_start,
        });
        end = line.end;
        *index += 1;
    }
    RegistrationBlock {
        name: name.to_owned(),
        name_line: Span::new(header.start, header.end),
        span: Span::new(header.start, end),
        entries,
    }
}

/// The members a declared block stands for while it is absent, by the block's name, in their order: the initial
/// and the implicit members of its list, which the specification gives and the body implies (DESL 0.2 sections 5.5,
/// 7.7.2). None, or no members for a block, writes the changed entry alone.
pub type AbsentMembers = Rc<dyn Fn(&str) -> Option<Vec<(String, String)>>>;

/// An open registration (FBL section 8): its bytes, its line form, and its own history.
pub struct OpenRegistration {
    file: SplicedFile,
    document: RegistrationDocument,
    /// The blocks the binding declares (FBL 0.7 section 8.1), in the order it declares them.
    declarations: Vec<RegistrationBlockDeclaration>,
    /// The headers the binding declares (FBL section 8.1).
    headers: Vec<String>,
    /// The first change to an absent block whose list has members writes them all, with the change applied (FBL 0.7
    /// section 8.4).
    pub absent_members: Option<AbsentMembers>,
    /// The ids the reading has, when known. A layout entry for any other id is stale and is removed as part of the
    /// next placement (FBL section 8.5).
    pub known_ids: Option<HashSet<String>>,
    /// Ids DISL marks ephemeral: such an element's position is never stored (FBL section 8.5).
    pub ephemeral: Option<HashSet<String>>,
    /// The natural keys the reading has, when known: an `identities` entry for any other key is stale (FBL 8.6).
    pub known_keys: Option<HashSet<String>>,
    /// The line ending a block created anew is written with, in place of the registration's own (FBL section 8.3):
    /// the standalone host's `RegistrationLayout` wrote a new `layout:` block with `AdpFileWriter.NewLine`, CRLF on
    /// every platform, and a tool type whose parity is that host's bytes asks for it. None for the registration's own.
    pub created_block_ending: Option<String>,
}

impl std::fmt::Debug for OpenRegistration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenRegistration")
            .field("document", &self.document)
            .field("declarations", &self.declarations)
            .field("known_ids", &self.known_ids)
            .finish_non_exhaustive()
    }
}

impl OpenRegistration {
    pub fn open(bytes: Vec<u8>) -> OpenRegistration {
        Self::open_declared(bytes, Vec::new(), Vec::new())
    }

    /// Opens a registration through its binding, which declares its headers and blocks (FBL 0.7 section 8.1).
    pub fn open_with(bytes: Vec<u8>, binding: &FblBinding) -> OpenRegistration {
        Self::open_declared(
            bytes,
            binding.registration.blocks.clone(),
            binding.registration.headers.clone(),
        )
    }

    fn open_declared(
        bytes: Vec<u8>,
        declarations: Vec<RegistrationBlockDeclaration>,
        headers: Vec<String>,
    ) -> OpenRegistration {
        let declared: Vec<String> = declarations.iter().map(|d| d.name.clone()).collect();
        OpenRegistration {
            document: RegistrationDocument::read_declared(bytes.clone(), &declared),
            file: SplicedFile::new(bytes),
            declarations,
            headers,
            absent_members: None,
            known_ids: None,
            ephemeral: None,
            known_keys: None,
            created_block_ending: None,
        }
    }

    pub fn bytes(&self) -> &[u8] {
        self.file.bytes()
    }

    pub fn can_undo(&self) -> bool {
        self.file.can_undo()
    }

    pub fn document(&self) -> &RegistrationDocument {
        &self.document
    }

    /// The entries of the declared block `block` (FBL 0.7 section 8.1), by key and value in their order, or None
    /// when the registration does not hold it.
    pub fn block_entries(&self, block: &str) -> Option<Vec<(String, String)>> {
        self.document
            .block(block)
            .map(|b| b.entries.iter().map(|e| (e.key.clone(), e.value.clone())).collect())
    }

    /// What reading the registration reports (FBL sections 8.1, 8.5, 0.7 section 8.1): the headers and blocks
    /// neither FBL nor the binding declares, and, when the known ids are set, the entries for ids the body does not
    /// have.
    pub fn findings(&self, file_name: &str) -> Vec<Finding> {
        let mut findings = self.document.unknown_headers(&self.headers, file_name);
        findings.extend(self.document.unknown_blocks(file_name));
        if let Some(ids) = &self.known_ids {
            findings.extend(self.document.stale_entries(ids, file_name, &self.declarations));
        }
        findings
    }

    /// Whether writes to the block `name` are steps of the history (FBL 0.7 section 8.1).
    pub(crate) fn keeps_in_history(&self, name: &str) -> bool {
        self.declaration(name).is_none_or(|d| d.in_history)
    }

    fn declaration(&self, name: &str) -> Option<&RegistrationBlockDeclaration> {
        self.declarations.iter().find(|d| d.name == name)
    }

    fn plan(&self, change: &ModelChange) -> PlanResult {
        match change {
            ModelChange::Save => PlanResult::Planned(PlannedEdit::default()),
            ModelChange::Place { id, x, y } => {
                if self.ephemeral.as_ref().is_some_and(|e| e.contains(id)) {
                    return PlanResult::Refused(format!(
                        "The position of '{id}' is kept for this session only, because its id changes whenever the file is edited."
                    ));
                }
                PlanResult::Planned(PlannedEdit {
                    splices: self.plan_place(id, *x, *y),
                    snapshot: false,
                })
            }
            ModelChange::Identify { key, id } => PlanResult::Planned(PlannedEdit {
                splices: self.plan_identify(key, id),
                snapshot: false,
            }),
            ModelChange::Setting(setting) => {
                let Some(declaration) = self.declaration(&setting.block) else {
                    return PlanResult::Refused(format!("The binding declares no block '{}'.", setting.block));
                };
                let members = self.absent_members.as_ref().and_then(|members| members(&setting.block));
                match blocks::plan(
                    &self.document,
                    &self.declarations,
                    declaration,
                    setting,
                    self.known_ids.as_ref(),
                    members,
                ) {
                    Ok(splices) => PlanResult::Planned(PlannedEdit {
                        splices,
                        snapshot: false,
                    }),
                    Err(refused) => PlanResult::Refused(refused.reason),
                }
            }
            _ => PlanResult::Invalid(
                "A registration is changed only by placing elements, storing ids and writing settings.".into(),
            ),
        }
    }

    /// Plans and applies `change`. A setting of a block declared `history: false` is applied outside the history
    /// (FBL 0.7 section 7.1): no undo step, and the history carried over it.
    pub fn change(&mut self, change: &ModelChange) -> PlanResult {
        let result = self.plan(change);
        if let PlanResult::Planned(edit) = &result {
            let outside = matches!(change, ModelChange::Setting(setting)
                if self.declaration(&setting.block).is_some_and(|d| !d.in_history));
            let applied = if outside {
                self.file.apply_outside_history(edit)
            } else {
                self.file.apply(edit)
            };
            match applied {
                Ok(true) => self.reread(),
                Ok(false) => {}
                Err(problem) => return PlanResult::Invalid(problem),
            }
        }
        result
    }

    /// Moves and prunes the entries of the `identities` block and writes a new one, as one edit (FBL 0.12 section 8.6): a
    /// moved entry by a `replace-value` of its key, a pruned one by a `remove-entry` of its line.
    pub(crate) fn change_identities(
        &mut self,
        moves: &[(String, String)],
        pruned: &[String],
        new: Option<(&str, &str)>,
    ) -> PlanResult {
        let mut splices = Vec::new();
        for entry in self.document.identities.iter().flat_map(|block| &block.entries) {
            if let Some((_, to)) = moves.iter().find(|(from, _)| *from == entry.key) {
                splices.push(splice(
                    Operation::ReplaceValue,
                    entry.key_span.start,
                    entry.key_span.end,
                    to.clone(),
                ));
            } else if pruned.contains(&entry.key) {
                splices.push(splice(
                    Operation::RemoveEntry,
                    entry.line.start,
                    entry.line.end,
                    String::new(),
                ));
            }
        }
        if let Some((key, id)) = new {
            splices.extend(self.plan_identify(key, id));
        }
        let edit = PlannedEdit {
            splices: ordered(splices),
            snapshot: false,
        };
        match self.file.apply(&edit) {
            Ok(true) => self.reread(),
            Ok(false) => {}
            Err(problem) => return PlanResult::Invalid(problem),
        }
        PlanResult::Planned(edit)
    }

    /// Stores an id read without an entry (FBL 0.12 section 8.6): written with the next write of the body, and no step
    /// of the history.
    pub(crate) fn store_identity(&mut self, key: &str, id: &str) -> Vec<Splice> {
        let edit = PlannedEdit {
            splices: self.plan_identify(key, id),
            snapshot: false,
        };
        if matches!(self.file.apply_outside_history(&edit), Ok(true)) {
            self.reread();
        }
        edit.splices
    }

    pub fn undo(&mut self) -> UndoResult {
        let result = self.file.undo(None);
        if let UndoResult::Done(_) = result {
            self.reread();
        }
        result
    }

    pub fn redo(&mut self) -> UndoResult {
        let result = self.file.redo(None);
        if let UndoResult::Done(_) = result {
            self.reread();
        }
        result
    }

    fn reread(&mut self) {
        let declared: Vec<String> = self.declarations.iter().map(|d| d.name.clone()).collect();
        self.document = RegistrationDocument::read_declared(self.file.bytes().to_vec(), &declared);
    }

    fn plan_place(&self, id: &str, x: f64, y: f64) -> Vec<Splice> {
        let x = new_text::number(x, Some(3));
        let y = new_text::number(y, Some(3));
        let text = self.document.text();
        self.plan_entry(
            self.document.layout.as_ref(),
            "layout",
            self.document.after_headers,
            self.known_ids.as_ref(),
            id,
            &format!("{x} {y}"),
            &|existing| {
                let parts = numbers(text, existing.value_span);
                if parts.len() != 2 {
                    return vec![splice(
                        Operation::ReplaceValue,
                        existing.value_span.start,
                        existing.value_span.end,
                        format!("{x} {y}"),
                    )];
                }
                let mut splices = Vec::new();
                if text.text_of(parts[0]) != x {
                    splices.push(splice(Operation::ReplaceValue, parts[0].start, parts[0].end, x.clone()));
                }
                if text.text_of(parts[1]) != y {
                    splices.push(splice(Operation::ReplaceValue, parts[1].start, parts[1].end, y.clone()));
                }
                splices
            },
        )
    }

    fn plan_identify(&self, key: &str, id: &str) -> Vec<Splice> {
        let after = self
            .document
            .layout
            .as_ref()
            .map(|l| l.span.end)
            .unwrap_or(self.document.after_headers);
        let text = self.document.text();
        self.plan_entry(
            self.document.identities.as_ref(),
            "identities",
            after,
            self.known_keys.as_ref(),
            key,
            id,
            &|existing| {
                if text.text_of(existing.value_span) == id {
                    Vec::new()
                } else {
                    vec![splice(
                        Operation::ReplaceValue,
                        existing.value_span.start,
                        existing.value_span.end,
                        id.to_owned(),
                    )]
                }
            },
        )
    }

    /// Writes one entry of a block (FBL sections 8.3 and 8.6): its value replaced in place when it exists, else
    /// inserted in the ordinal order of keys, creating the block at `create_at` first; stale entries (keys not in
    /// `known`) are removed in the same edit (FBL section 8.5).
    #[allow(clippy::too_many_arguments)]
    fn plan_entry(
        &self,
        block: Option<&RegistrationBlock>,
        name: &str,
        create_at: usize,
        known: Option<&HashSet<String>>,
        key: &str,
        value: &str,
        replace: &dyn Fn(&RegistrationEntry) -> Vec<Splice>,
    ) -> Vec<Splice> {
        let text = self.document.text();
        let mut splices = Vec::new();
        let stale: Vec<&RegistrationEntry> = match (block, known) {
            (Some(block), Some(known)) => block
                .entries
                .iter()
                .filter(|e| e.key != key && !known.contains(&e.key))
                .collect(),
            _ => Vec::new(),
        };
        for entry in &stale {
            splices.push(splice(
                Operation::RemoveEntry,
                entry.line.start,
                entry.line.end,
                String::new(),
            ));
        }
        if let Some(existing) = block.and_then(|b| b.entries.iter().find(|e| e.key == key)) {
            splices.extend(replace(existing));
            return ordered(splices);
        }
        let ends_without_newline = text.lines().last().is_some_and(|l| l.ending.is_empty());
        let Some(block) = block else {
            let offset = create_at;
            let newline = match &self.created_block_ending {
                Some(ending) => ending.clone(),
                None => text.newline_at(offset.saturating_sub(1), "\n"),
            };
            let lead = offset == text.len() && ends_without_newline && offset > 0;
            if lead {
                splices.push(splice(
                    Operation::EnsureContainer,
                    offset,
                    offset,
                    format!("{newline}{name}:"),
                ));
                splices.push(splice(
                    Operation::InsertEntry,
                    offset,
                    offset,
                    format!("{newline}  {key}: {value}"),
                ));
            } else {
                splices.push(splice(
                    Operation::EnsureContainer,
                    offset,
                    offset,
                    format!("{name}:{newline}"),
                ));
                splices.push(splice(
                    Operation::InsertEntry,
                    offset,
                    offset,
                    format!("  {key}: {value}{newline}"),
                ));
            }
            return splices;
        };
        let kept: Vec<&RegistrationEntry> = block.entries.iter().filter(|e| !stale.contains(e)).collect();
        let next = kept.iter().find(|e| e.key.as_bytes() > key.as_bytes());
        let indent = " ".repeat(kept.first().map(|e| e.indent).unwrap_or(2));
        let at = match next {
            Some(next) => next.line.start,
            None => kept.last().map(|e| e.line.end).unwrap_or(block.name_line.end),
        };
        let ending = text.newline_at(at.saturating_sub(1), "\n");
        let entry_text = if at == text.len() && ends_without_newline {
            format!("{ending}{indent}{key}: {value}")
        } else {
            format!("{indent}{key}: {value}{ending}")
        };
        splices.push(splice(Operation::InsertEntry, at, at, entry_text));
        ordered(splices)
    }
}

/// The registration to write beside a body the first time the user places an element or changes a setting a
/// declared block keeps, when the binding's `registration.createOnFirstPlacement` is true (FBL section 8.4, 0.7
/// section 8.4; the standalone host's `Registration/RegistrationCreation`). It is written before the edit is planned,
/// outside the history: the origin line, then `body: <name>` when the base names differ, in the body's dominant line
/// ending. `body_name` is the body's path relative to the registration's folder, `/`-separated. Refused with a reason
/// when the binding does not create one, or when a file of that name exists (`existing`) that is not a registration
/// of this origin and body; an existing one that is gives its own bytes.
pub fn create_registration(
    binding: &FblBinding,
    origin: &str,
    registration_name: &str,
    body_name: &str,
    body: &[u8],
    existing: Option<&[u8]>,
) -> Result<Vec<u8>, String> {
    if !binding.registration.create_on_first_placement {
        return Err(format!(
            "'{body_name}' has no registration, and its tool type keeps its view data only in one, so the change cannot be kept."
        ));
    }
    let named = base_name(registration_name) != base_name(body_name);
    if let Some(existing) = existing {
        let document = RegistrationDocument::read(existing.to_vec());
        let body_of = document
            .body()
            .map(str::to_owned)
            .or_else(|| (!named).then(|| body_name.to_owned()));
        return if document.origin == origin && body_of.as_deref() == Some(body_name) {
            Ok(existing.to_vec())
        } else {
            Err(format!(
                "'{registration_name}' exists and is no registration of '{body_name}', so none is created beside it."
            ))
        };
    }
    let newline = BodyText::new(body.to_vec())
        .dominant_ending()
        .map(str::to_owned)
        .unwrap_or_else(|| binding.text.newline.clone());
    let mut text = format!("{origin}{newline}");
    if named {
        text.push_str(&format!("body: {body_name}{newline}"));
    }
    Ok(text.into_bytes())
}

/// A path's file name without its last extension, as .NET's `Path.GetFileNameWithoutExtension` gives it.
fn base_name(path: &str) -> &str {
    let file = path.rsplit(['/', '\\']).next().unwrap_or(path);
    match file.rfind('.') {
        Some(dot) => &file[..dot],
        None => file,
    }
}

fn splice(operation: Operation, start: usize, end: usize, text: String) -> Splice {
    Splice {
        operation,
        start,
        end,
        text,
    }
}

/// The splices in body order. At one offset an insertion goes before a removal that starts there (FBL 8.5, 8.6).
fn ordered(splices: Vec<Splice>) -> Vec<Splice> {
    let mut indexed: Vec<(usize, Splice)> = splices.into_iter().enumerate().collect();
    indexed.sort_by_key(|(i, s)| (s.start, s.end, *i));
    indexed.into_iter().map(|(_, s)| s).collect()
}

fn numbers(text: &BodyText, value: Span) -> Vec<Span> {
    let bytes = text.bytes();
    let mut spans = Vec::new();
    let mut i = value.start;
    while i < value.end {
        while i < value.end && bytes[i] == b' ' {
            i += 1;
        }
        let start = i;
        while i < value.end && bytes[i] != b' ' {
            i += 1;
        }
        if i > start {
            spans.push(Span::new(start, i));
        }
    }
    spans
}

/// Where a registration's body is (FBL section 8.2), or why it is not opened.
#[derive(Debug, Clone, PartialEq)]
pub struct BodyLocation {
    pub path: Option<std::path::PathBuf>,
    pub exists: bool,
    pub refusal: Option<String>,
    /// What locating found: `fbl.missing-body` when the body is missing (FBL sections 7.4 and 8.2), else nothing.
    pub findings: Vec<Finding>,
}

impl BodyLocation {
    /// A missing body opens empty with `fbl.missing-body`, and nothing is written until the registration names a
    /// file.
    pub fn is_missing(&self) -> bool {
        self.refusal.is_none() && !self.exists
    }

    fn refused(reason: &str) -> Self {
        BodyLocation {
            path: None,
            exists: false,
            refusal: Some(reason.to_owned()),
            findings: Vec::new(),
        }
    }
}

/// Finds a registration's body (FBL section 8.2): the `body` header relative to the registration's folder, else
/// the sibling with the registration's base name and the first existing claimed extension, else the registration's
/// folder for a folder binding. A body outside the workspace root, or reached through a symbolic link, is refused
/// (FBL section 16).
pub fn locate_body(
    registration_path: &std::path::Path,
    registration: &RegistrationDocument,
    binding: &crate::documents::FblBinding,
    workspace_root: &std::path::Path,
) -> BodyLocation {
    use std::path::{Component, Path, PathBuf};

    /// The path with `.` and `..` resolved without touching the file system, as .NET's `GetFullPath` does.
    fn normalise(path: &Path) -> PathBuf {
        let mut out = PathBuf::new();
        for component in path.components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    out.pop();
                }
                other => out.push(other.as_os_str()),
            }
        }
        out
    }

    let registration_full =
        normalise(&std::path::absolute(registration_path).unwrap_or_else(|_| registration_path.to_path_buf()));
    let folder = registration_full.parent().map(Path::to_path_buf).unwrap_or_default();
    let root = normalise(&std::path::absolute(workspace_root).unwrap_or_else(|_| workspace_root.to_path_buf()));
    let candidate = if let Some(body) = registration.body() {
        if Path::new(body).is_absolute() || body.starts_with(['/', '\\']) {
            return BodyLocation::refused("The registration's body is an absolute path, which is never followed.");
        }
        normalise(&folder.join(body))
    } else if binding.body.is_folder {
        folder.clone()
    } else {
        let base = registration_full
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        binding
            .claims
            .extensions
            .iter()
            .map(|e| folder.join(format!("{base}{e}")))
            .find(|p| p.is_file())
            .unwrap_or_else(|| {
                folder.join(format!(
                    "{base}{}",
                    binding
                        .claims
                        .extensions
                        .first()
                        .map(String::as_str)
                        .unwrap_or_default()
                ))
            })
    };
    if !candidate.starts_with(&root) {
        return BodyLocation::refused(
            "The registration's body is outside the workspace, so it is not opened without the user's consent.",
        );
    }
    let mut current = candidate.clone();
    while current.starts_with(&root) && current != root {
        if std::fs::symlink_metadata(&current).is_ok_and(|m| m.file_type().is_symlink()) {
            return BodyLocation::refused(
                "The registration's body is reached through a link, which is not followed without the user's consent.",
            );
        }
        if !current.pop() {
            break;
        }
    }
    let exists = if binding.body.is_folder {
        candidate.is_dir()
    } else {
        candidate.is_file()
    };
    let mut findings = Vec::new();
    if !exists {
        // On the registration's `body` header, or on its origin line when the body is the derived sibling.
        let text = registration.text();
        let span = registration
            .headers
            .iter()
            .find(|h| h.key == "body")
            .map(|h| h.line)
            .unwrap_or(Span::new(text.bom_length(), text.lines()[0].end));
        let (line, column) = text.position(span.start);
        let name = |p: &Path| {
            p.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        };
        findings.push(Finding {
            code: codes::MISSING_BODY.into(),
            severity: FindingSeverity::Error,
            message: format!(
                "The body '{}' does not exist, so the document opens empty and nothing is written until the registration names a file.",
                name(&candidate)
            ),
            location: Some(SourceLocation {
                file: name(&registration_full),
                line,
                column,
                length: text.code_points(span.start, span.end),
            }),
            detail: None,
        });
    }
    BodyLocation {
        path: Some(candidate),
        exists,
        refusal: None,
        findings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A registration with a byte order mark, CRLF endings, a header no one declares and a block after the layout
    /// that FBL does not know (FBL section 8.1): placing an element writes its one entry in the layout's order, and
    /// every other byte, the unknown header and block included, stays as it was; undo gives the bytes back.
    #[test]
    fn placing_an_element_keeps_the_unknown_header_and_block_and_every_other_byte() {
        let original = "\u{feff}freeplane/mindmap\r\nbody: ideas.mm\r\nowner: Peter\r\nlayout:\r\n  ID_2: 10 20\r\n\r\nnotes:\r\n  anything: kept as it is\r\n";
        let mut registration = OpenRegistration::open(original.as_bytes().to_vec());
        let document = registration.document();
        assert_eq!(document.origin, "freeplane/mindmap");
        assert_eq!(document.body(), Some("ideas.mm"));
        let unknown = document.unknown_headers(&[], "ideas.adp");
        assert_eq!(unknown.len(), 1);
        assert_eq!(unknown[0].code, codes::UNKNOWN_HEADER);
        assert!(unknown[0].message.contains("'owner'"), "{}", unknown[0].message);

        let placed = registration.change(&ModelChange::Place {
            id: "ID_1".into(),
            x: 1.5,
            y: 2.0,
        });
        assert!(matches!(placed, PlanResult::Planned(_)), "{placed:?}");
        assert_eq!(
            String::from_utf8(registration.bytes().to_vec()).unwrap(),
            original.replace("layout:\r\n", "layout:\r\n  ID_1: 1.5 2\r\n")
        );

        assert!(matches!(registration.undo(), UndoResult::Done(_)));
        assert_eq!(registration.bytes(), original.as_bytes());
    }
}
