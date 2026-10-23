//! The checker of ATD (etalii.adp spec 035, research R9, R19; ATD §14, §15): a description's files and versions, its
//! names and imports, the constructs its kind admits, the types of its expressions, its settings against DISL's,
//! DESL's and FBL's schemas, the checks across its model, surface and bindings, and its script; findings with codes,
//! positions, suggestions and fixes.

mod cel;
mod checks;
mod description;
pub mod embedded;
mod script;
mod settings;
mod suggest;
mod symbols;
pub mod trace;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{Value as Json, json};

pub use description::{Description, FileUnit, describe};
pub use symbols::Symbols;

/// How bad a finding is (ATD §15.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    Error,
    Warning,
    Information,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Information => "information",
        }
    }
}

/// A text edit of a fix, in byte offsets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub file: String,
    pub start: usize,
    pub end: usize,
    pub text: String,
}

/// A fix: a title and its edits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fix {
    pub title: String,
    pub edits: Vec<Edit>,
}

/// A finding (ATD §15, `$defs/Finding`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub code: &'static str,
    pub severity: Severity,
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
    /// The byte range, for language servers and fixes.
    pub start: usize,
    pub end: usize,
    pub message: String,
    pub suggestions: Vec<String>,
    pub fixes: Vec<Fix>,
}

impl Finding {
    pub fn to_json(&self) -> Json {
        let mut object = json!({
            "code": self.code, "severity": self.severity.as_str(), "file": self.file,
            "line": self.line, "column": self.column, "endLine": self.end_line, "endColumn": self.end_column,
            "message": self.message,
        });
        if !self.suggestions.is_empty() {
            object["suggestions"] = json!(self.suggestions);
        }
        if !self.fixes.is_empty() {
            object["fixes"] = Json::Array(
                self.fixes
                    .iter()
                    .map(|f| {
                        let edits: Vec<Json> = f
                            .edits
                            .iter()
                            .map(|e| json!({"file": e.file, "start": e.start, "end": e.end, "text": e.text}))
                            .collect();
                        json!({"title": f.title, "edits": edits})
                    })
                    .collect(),
            );
        }
        object
    }

    /// `file:line:column: severity CODE: message`, the text form of `atd check` (contracts/checker.md).
    pub fn to_line(&self) -> String {
        format!(
            "{}:{}:{}: {} {}: {}",
            self.file,
            self.line,
            self.column,
            self.severity.as_str(),
            self.code,
            self.message
        )
    }
}

/// The findings as `$defs/Findings`, in the stable order of ATD §15.4.
pub fn findings_json(findings: &[Finding]) -> Json {
    json!({"findings": findings.iter().map(Finding::to_json).collect::<Vec<_>>()})
}

/// The files a checker reads: the file system, or the texts a language server holds open.
pub trait Files {
    /// The text of a file, by path.
    fn read(&self, path: &Path) -> Option<String>;
    /// The `.atd` files of a folder.
    fn list(&self, folder: &Path) -> Vec<PathBuf>;
}

/// The file system.
pub struct FsFiles;

impl Files for FsFiles {
    fn read(&self, path: &Path) -> Option<String> {
        std::fs::read_to_string(path).ok()
    }

    fn list(&self, folder: &Path) -> Vec<PathBuf> {
        let mut found: Vec<PathBuf> = std::fs::read_dir(folder)
            .map(|entries| {
                entries
                    .flatten()
                    .map(|e| e.path())
                    .filter(|p| p.extension().is_some_and(|e| e == "atd"))
                    .collect()
            })
            .unwrap_or_default();
        found.sort();
        found
    }
}

/// Open texts over another set of files: a text open here wins over the file.
pub struct Overlay<'a> {
    pub open: BTreeMap<PathBuf, String>,
    pub under: &'a dyn Files,
}

impl Files for Overlay<'_> {
    fn read(&self, path: &Path) -> Option<String> {
        self.open.get(path).cloned().or_else(|| self.under.read(path))
    }

    fn list(&self, folder: &Path) -> Vec<PathBuf> {
        let mut found = self.under.list(folder);
        for path in self.open.keys() {
            if path.parent() == Some(folder) && !found.contains(path) && path.extension().is_some_and(|e| e == "atd") {
                found.push(path.clone());
            }
        }
        found.sort();
        found
    }
}

/// The description a file belongs to, checked: its files and its findings, sorted (ATD §15.4).
pub struct Checked {
    pub description: Description,
    pub symbols: Option<Symbols>,
    pub findings: Vec<Finding>,
}

/// Checks the description `path` belongs to: the root it is, or the root that imports it (ATD §5).
pub fn check(files: &dyn Files, path: &Path) -> Checked {
    let mut description = describe(files, path);
    let mut findings = std::mem::take(&mut description.findings);
    let mut symbols = None;
    // A description whose files do not all parse is checked no further than its syntax (ATD §14.1): what the parser
    // skipped would otherwise show again as names that resolve to nothing.
    let parses = description
        .files
        .iter()
        .all(|f| f.parse.as_ref().is_some_and(|p| p.errors.is_empty()));
    if description.has_trees() {
        let collected = symbols::collect(&description);
        if parses {
            checks::run(&description, &collected, &mut findings);
            settings::run(&description, &collected, &mut findings);
            script::run(&description, &collected, &mut findings);
        }
        symbols = Some(collected);
    }
    sort(&mut findings);
    findings.dedup_by(|a, b| a.code == b.code && a.file == b.file && a.start == b.start && a.end == b.end);
    add_fixes(&description, &mut findings);
    Checked {
        description,
        symbols,
        findings,
    }
}

/// A finding with closest names gets one fix per name: the name replaced by it (FR-091).
fn add_fixes(description: &Description, findings: &mut [Finding]) {
    for finding in findings
        .iter_mut()
        .filter(|f| !f.suggestions.is_empty() && f.fixes.is_empty())
    {
        let Some(file) = description.file(&finding.file) else {
            continue;
        };
        let Some(written) = file.text.get(finding.start..finding.end) else {
            continue;
        };
        let sigil = if written.starts_with(['@', '$', '#']) {
            &written[..1]
        } else {
            ""
        };
        finding.fixes = finding
            .suggestions
            .iter()
            .map(|name| Fix {
                title: format!("Change to `{name}`"),
                edits: vec![Edit {
                    file: finding.file.clone(),
                    start: finding.start,
                    end: finding.end,
                    text: format!("{sigil}{name}"),
                }],
            })
            .collect();
    }
}

pub(crate) fn sort(findings: &mut [Finding]) {
    findings.sort_by(|a, b| (&a.file, a.line, a.column, a.code).cmp(&(&b.file, b.line, b.column, b.code)));
}

/// A finding at the span of a node of the abstract tree.
pub(crate) fn finding_at(code: &'static str, node: &Json, message: String) -> Finding {
    let span = adp_atd_syntax::Span::of(node).unwrap_or_default();
    Finding {
        code,
        severity: Severity::Error,
        file: span.file,
        line: span.line,
        column: span.column,
        end_line: span.end_line,
        end_column: span.end_column,
        start: span.start,
        end: span.end,
        message,
        suggestions: Vec::new(),
        fixes: Vec::new(),
    }
}
