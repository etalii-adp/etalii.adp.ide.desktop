//! A description's files (ATD §5): the root a file belongs to, the parts its imports reach, their versions and their
//! syntax, with the findings of §5 (versions, syntax, imports, cycles and parts).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use adp_atd_syntax::{LineIndex, Parse, VERSIONS, declared_version};
use serde_json::Value as Json;

use crate::{Files, Finding, Severity};

/// One file of a description.
pub struct FileUnit {
    pub path: PathBuf,
    /// The path as findings and spans name it.
    pub name: String,
    pub text: String,
    pub parse: Option<Parse>,
    /// The abstract tree, when the file parsed far enough.
    pub tree: Option<Json>,
    /// The modules of each grammar `grammar from` names in this file, by the module's name, as `Files` gave them, and
    /// each inline grammar's text under `grammar`, by the binding's name: a runtime has no files of its own (ATD §11.9).
    pub grammars: BTreeMap<String, BTreeMap<String, String>>,
}

impl FileUnit {
    /// The tool or part construct of the file's tree.
    pub fn construct(&self) -> Option<&Json> {
        self.tree.as_ref().map(|t| &t["description"])
    }

    pub fn members(&self) -> &[Json] {
        self.construct()
            .and_then(|c| c["members"].as_array())
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
}

/// A description: its root (when one was found) and its parts, and the findings of reading them.
pub struct Description {
    /// The root first, then the parts in the order the imports reach them.
    pub files: Vec<FileUnit>,
    /// Whether the first file is a root (a tool construct), rather than a part found without its root.
    pub rooted: bool,
    pub findings: Vec<Finding>,
}

impl Description {
    pub fn has_trees(&self) -> bool {
        self.files.iter().any(|f| f.tree.is_some())
    }

    /// The tool construct of the root.
    pub fn tool(&self) -> Option<&Json> {
        self.files
            .first()
            .and_then(FileUnit::construct)
            .filter(|c| c["kind"] == "tool")
    }

    /// `diagram`, `designer` or `editor`.
    pub fn kind(&self) -> Option<&str> {
        self.tool().and_then(|t| t["toolKind"].as_str())
    }

    pub fn origin(&self) -> Option<&str> {
        self.tool().and_then(|t| t["origin"].as_str())
    }

    /// Every member of every file, in order.
    pub fn members(&self) -> impl Iterator<Item = &Json> {
        self.files.iter().flat_map(|f| f.members().iter())
    }

    /// The modules of the grammar of the binding named `binding`, by the module's name, when it has one.
    pub fn grammar(&self, binding: &str) -> Option<&BTreeMap<String, String>> {
        self.files.iter().find_map(|f| f.grammars.get(binding))
    }

    pub fn file(&self, name: &str) -> Option<&FileUnit> {
        self.files.iter().find(|f| f.name == name)
    }
}

fn display(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Reads one file: its version, its parse and its tree, with their findings.
fn read(files: &dyn Files, path: &Path, findings: &mut Vec<Finding>) -> Option<FileUnit> {
    let text = files.read(path)?;
    let name = display(path);
    let mut unit = FileUnit {
        path: path.to_path_buf(),
        name: name.clone(),
        text,
        parse: None,
        tree: None,
        grammars: BTreeMap::new(),
    };
    let lines = LineIndex::new(&unit.text);
    let at = |start: usize, end: usize| {
        let (line, column) = lines.position(&unit.text, start);
        let (end_line, end_column) = lines.position(&unit.text, end);
        (line, column, end_line, end_column)
    };
    match declared_version(&unit.text) {
        None => {
            let (line, column, end_line, end_column) = at(0, 0);
            findings.push(Finding {
                code: "ATD0002",
                severity: Severity::Error,
                file: name,
                line,
                column,
                end_line,
                end_column,
                start: 0,
                end: 0,
                message: "The file does not start with `atd <major>.<minor>`, the version of ATD it is written in."
                    .into(),
                suggestions: Vec::new(),
                fixes: Vec::new(),
            });
            return Some(unit);
        }
        Some((version, start, end)) if !VERSIONS.contains(&version.as_str()) => {
            let (line, column, end_line, end_column) = at(start, end);
            findings.push(Finding {
                code: "ATD0001",
                severity: Severity::Error,
                file: name,
                line,
                column,
                end_line,
                end_column,
                start,
                end,
                message: format!(
                    "This description is written in ATD {version}, which this reader does not know; it reads ATD {}.",
                    VERSIONS.join(", ")
                ),
                suggestions: Vec::new(),
                fixes: Vec::new(),
            });
            return Some(unit);
        }
        Some(_) => {}
    }
    let parse = adp_atd_syntax::parse(&unit.text);
    for error in &parse.errors {
        let (line, column, end_line, end_column) = at(error.start, error.end);
        findings.push(Finding {
            code: "ATD1001",
            severity: Severity::Error,
            file: name.clone(),
            line,
            column,
            end_line,
            end_column,
            start: error.start,
            end: error.end,
            message: error.message.clone(),
            suggestions: Vec::new(),
            fixes: Vec::new(),
        });
    }
    unit.tree = adp_atd_syntax::lower(&parse, &name).ok();
    unit.parse = Some(parse);
    unit.grammars = crate::grammars::read(files, &unit);
    Some(unit)
}

/// The imports of a file's tree: each import's path and node, a part's only (no `as`).
fn part_imports(unit: &FileUnit) -> Vec<(String, Json)> {
    unit.members()
        .iter()
        .filter(|m| m["kind"] == "import" && m.get("as").is_none())
        .map(|m| (m["path"].as_str().unwrap_or("").to_owned(), m.clone()))
        .collect()
}

fn resolve(from: &Path, relative: &str) -> PathBuf {
    let folder = from.parent().unwrap_or(Path::new(""));
    let mut path = folder.to_path_buf();
    for part in relative.split('/') {
        match part {
            "." | "" => {}
            ".." => {
                path.pop();
            }
            other => path.push(other),
        }
    }
    path
}

/// The description `path` belongs to: itself when it is a root; else the root in its folder whose imports reach it.
pub fn describe(files: &dyn Files, path: &Path) -> Description {
    let mut findings = Vec::new();
    let Some(first) = read(files, path, &mut findings) else {
        return Description {
            files: Vec::new(),
            rooted: false,
            findings,
        };
    };
    let is_root = first.construct().is_some_and(|c| c["kind"] == "tool");
    if !is_root && first.tree.is_some() {
        // Find the root that reaches this part.
        let folder = path.parent().unwrap_or(Path::new(""));
        for candidate in files.list(folder) {
            if candidate == path {
                continue;
            }
            let mut scratch = Vec::new();
            let Some(unit) = read(files, &candidate, &mut scratch) else {
                continue;
            };
            if !unit.construct().is_some_and(|c| c["kind"] == "tool") {
                continue;
            }
            let reached = assemble(files, unit, &mut Vec::new());
            if reached.files.iter().any(|f| f.path == path) {
                return reached;
            }
        }
    }
    let mut description = assemble(files, first, &mut findings);
    description.rooted = is_root;
    description.findings.splice(0..0, findings);
    description
}

/// The root and the parts its imports reach, depth first in the order of the imports, with the findings of imports:
/// a missing file, a cycle reported once at the import that closes it, a part of another origin (ATD §5.2, §5.3).
fn assemble(files: &dyn Files, root: FileUnit, findings: &mut Vec<Finding>) -> Description {
    let origin = root.construct().and_then(|c| c["origin"].as_str()).map(str::to_owned);
    let rooted = root.construct().is_some_and(|c| c["kind"] == "tool");
    let mut units = vec![root];
    let mut found = Vec::new();
    let mut on_stack: Vec<PathBuf> = vec![units[0].path.clone()];
    let mut seen: BTreeSet<PathBuf> = BTreeSet::from([units[0].path.clone()]);
    visit(files, 0, &mut units, &mut on_stack, &mut seen, &mut found);
    if let Some(origin) = origin.filter(|_| rooted) {
        for unit in units.iter().skip(1) {
            if let Some(construct) = unit.construct()
                && construct["kind"] == "part"
                && construct["origin"].as_str() != Some(origin.as_str())
            {
                let part_origin = construct["origin"].as_str().unwrap_or("");
                let mut finding = crate::finding_at(
                    "ATD2007",
                    construct,
                    format!(
                        "This part says it is part of `{part_origin}`, but `{}` imports it into `{origin}`.",
                        units[0]
                            .path
                            .file_name()
                            .map(|f| f.to_string_lossy().into_owned())
                            .unwrap_or_default()
                    ),
                );
                place_on_origin(&mut finding, unit, part_origin);
                found.push(finding);
            }
        }
    }
    findings.append(&mut found);
    Description {
        files: units,
        rooted,
        findings: std::mem::take(findings),
    }
}

fn visit(
    files: &dyn Files,
    index: usize,
    units: &mut Vec<FileUnit>,
    on_stack: &mut Vec<PathBuf>,
    seen: &mut BTreeSet<PathBuf>,
    findings: &mut Vec<Finding>,
) {
    let imports = part_imports(&units[index]);
    for (relative, node) in imports {
        let path = resolve(&units[index].path, &relative);
        if on_stack.contains(&path) {
            let mut cycle: Vec<String> = on_stack
                .iter()
                .skip_while(|p| **p != path)
                .map(|p| {
                    p.file_name()
                        .map(|f| f.to_string_lossy().into_owned())
                        .unwrap_or_default()
                })
                .collect();
            cycle.push(
                path.file_name()
                    .map(|f| f.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            );
            let quoted: Vec<String> = cycle.iter().map(|f| format!("`{f}`")).collect();
            findings.push(crate::finding_at(
                "ATD2003",
                &node,
                format!("The imports {} form a cycle.", quoted.join(" → ")),
            ));
            continue;
        }
        if seen.contains(&path) {
            continue;
        }
        let outside = relative.starts_with('/') || relative.split('/').any(|p| p == "..");
        let unit = if outside { None } else { read(files, &path, findings) };
        let Some(unit) = unit else {
            findings.push(crate::finding_at(
                "ATD2009",
                &node,
                format!("The imported file `{relative}` is not there, or not in the description's folder."),
            ));
            continue;
        };
        seen.insert(path.clone());
        units.push(unit);
        let child = units.len() - 1;
        on_stack.push(path);
        visit(files, child, units, on_stack, seen, findings);
        on_stack.pop();
    }
}

/// Puts a finding on the origin a part names.
fn place_on_origin(finding: &mut Finding, unit: &FileUnit, origin: &str) {
    if let Some(offset) = unit.text.find(&format!("of {origin}")).map(|i| i + 3) {
        let lines = LineIndex::new(&unit.text);
        let (line, column) = lines.position(&unit.text, offset);
        let (end_line, end_column) = lines.position(&unit.text, offset + origin.len());
        finding.line = line;
        finding.column = column;
        finding.end_line = end_line;
        finding.end_column = end_column;
        finding.start = offset;
        finding.end = offset + origin.len();
        finding.file = unit.name.clone();
    }
}
