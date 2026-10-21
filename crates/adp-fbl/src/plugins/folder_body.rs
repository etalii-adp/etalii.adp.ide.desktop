//! A folder subject read by a persistence plugin (FBL sections 10 and 11.2; the standalone host's
//! `Plugins/FolderBody`): the folder qualifies by the binding's `recognise` (section 10.1), its files are selected
//! by the file rules and `ignore` in ordinal order of relative path and never through a reparse point (section
//! 10.2), and all of them are handed to the plugin in one `read`. A folder that fails `recognise` or is gone is an
//! unreadable body; a selected file that cannot be opened is DISL's `std.unparseable` at that file, and the rest is
//! read (section 10.3). Without the plugin the folder opens read-only with DISL's `std.pluginMissing` (section
//! 15.1). This host does not yet write a folder subject's files (section 10.4), so every plan is refused.
//!
//! Watching is the host's, through plain functions: it watches what [`FolderBody::dependencies`] names and passes
//! every changed path to [`FolderBody::notify`] with the time it saw it; this crate opens no watcher and runs no
//! timer. Once no change has arrived for the binding's `settle` milliseconds ([`FolderBody::deadline`]), the
//! host's [`FolderBody::poll`] reads the folder again, once however many files changed, and returns what changed. A
//! burst that touched only the registration is returned without reading the folder (section 10.3).

use std::io::Read as _;
use std::path::{MAIN_SEPARATOR, MAIN_SEPARATOR_STR, Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use crate::documents::types::FblBinding;
use crate::model::{FblModel, FblOptions, Finding, FindingSeverity, SourceLocation, codes};
use crate::planning::{ModelChange, PlanResult};
use crate::routing::{self, FolderFile, glob_matches, platform_ignores_case};

use super::{PersistencePlugin, PluginFile, PluginPlanRequest, PluginPlanResult, PluginReadRequest, PluginReadResult};

/// The key of a finding's `detail` naming the element it is about, DISL's `subject` (FBL section 11.4).
pub const SUBJECT_DETAIL: &str = "subject";

/// What a settled burst of changes changed: the folder, which was read again, the registration, or both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FolderBodyChange {
    pub folder_read: bool,
    pub registration: bool,
}

/// One open folder subject whose binding names a plugin.
pub struct FolderBody {
    /// The folder's full path.
    folder: PathBuf,
    binding: Rc<FblBinding>,
    plugin: Option<Rc<dyn PersistencePlugin>>,
    options: Rc<FblOptions>,
    registration: Option<PathBuf>,
    last: PluginReadResult,
    files: Vec<PluginFile>,
    unopened: Vec<String>,
    watched: Vec<PathBuf>,
    due: Option<Instant>,
    folder_changed: bool,
    registration_changed: bool,
    closed: bool,
}

impl std::fmt::Debug for FolderBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FolderBody")
            .field("folder", &self.folder)
            .field("binding", &self.binding.name)
            .finish_non_exhaustive()
    }
}

impl FolderBody {
    /// Opens a folder subject whose binding names a plugin. `plugin` is the one the caller has installed, or none;
    /// a plugin with another id counts as missing. `registration` is the path of the folder's registration, whether
    /// or not it exists yet. A binding that reads a file, or is read by its declared rules, is the caller's error.
    pub fn open(
        folder: &Path,
        binding: Rc<FblBinding>,
        plugin: Option<Rc<dyn PersistencePlugin>>,
        options: Rc<FblOptions>,
        registration: Option<&Path>,
    ) -> Result<FolderBody, String> {
        if folder.as_os_str().is_empty() {
            return Err("The folder is not named.".into());
        }
        if !binding.body.is_folder {
            return Err(format!("The binding '{}' reads a file, not a folder.", binding.name));
        }
        let Some(reader) = &binding.plugin else {
            return Err(format!(
                "The binding '{}' is read by its declared rules, not by a plugin.",
                binding.name
            ));
        };
        let plugin = plugin.filter(|p| p.id() == reader.plugin);
        let mut body = FolderBody {
            folder: full(folder),
            binding,
            plugin,
            options,
            registration: registration.map(full),
            last: PluginReadResult::default(),
            files: Vec::new(),
            unopened: Vec::new(),
            watched: Vec::new(),
            due: None,
            folder_changed: false,
            registration_changed: false,
            closed: false,
        };
        body.last = body.read_now();
        Ok(body)
    }

    /// Reads a folder once, for a caller that keeps nothing open: nothing is watched or pending afterwards.
    pub fn read_once(
        folder: &Path,
        binding: Rc<FblBinding>,
        plugin: Option<Rc<dyn PersistencePlugin>>,
        options: Rc<FblOptions>,
    ) -> Result<FblModel, String> {
        Ok(FolderBody::open(folder, binding, plugin, options, None)?.model())
    }

    pub fn model(&self) -> FblModel {
        FblModel {
            elements: self.last.elements.clone(),
            findings: self.last.findings.clone(),
            unreadable: self.last.unreadable,
            ..FblModel::default()
        }
    }

    /// Read-only without the plugin, when the folder is unreadable, or when the binding is read-only.
    pub fn is_read_only(&self) -> bool {
        self.plugin.is_none() || self.last.unreadable || self.binding.read_only.is_some()
    }

    /// Reads the folder again, now (FBL section 10.3).
    pub fn reread(&mut self) {
        self.last = self.read_now();
    }

    /// The full paths the reading depends on, for the host to watch (FBL sections 10.3 and 11.2): the files read
    /// and those selected that could not be opened, the folders the file rules' globs could add files to, what the
    /// plugin's `watch` adds, and the registration. In ordinal order, each once.
    pub fn dependencies(&self) -> Vec<PathBuf> {
        let mut paths: Vec<PathBuf> = Vec::new();
        paths.extend(self.files.iter().map(|f| self.full_of(&f.relative_path)));
        paths.extend(self.unopened.iter().map(|f| self.full_of(f)));
        paths.extend(self.glob_folders());
        paths.extend(self.watched.iter().cloned());
        paths.extend(self.registration.iter().cloned());
        let mut distinct: Vec<PathBuf> = Vec::new();
        for path in paths {
            if !distinct.iter().any(|p| same_path(p, &path)) {
                distinct.push(path);
            }
        }
        distinct.sort_by(|a, b| a.to_string_lossy().cmp(&b.to_string_lossy()));
        distinct
    }

    /// Takes one path the host saw change at `now`: created, changed, deleted or renamed, file or folder, full or
    /// relative to the folder. A path the reading does not depend on is ignored; any other starts, or starts again,
    /// the settle delay.
    pub fn notify(&mut self, path: &Path, now: Instant) {
        if path.as_os_str().is_empty() || self.closed {
            return;
        }
        let changed = if path.is_absolute() {
            full(path)
        } else {
            full(&self.folder.join(path))
        };
        let registration = self.registration.as_ref().is_some_and(|r| same_path(&changed, r));
        if !registration && !self.concerns(&changed) {
            return;
        }
        if registration {
            self.registration_changed = true;
        } else {
            self.folder_changed = true;
        }
        let settle = u64::try_from(self.binding.body.settle).unwrap_or(0);
        self.due = Some(now + Duration::from_millis(settle));
    }

    /// When the pending burst of changes settles, for the host's timer; none when nothing is pending.
    pub fn deadline(&self) -> Option<Instant> {
        self.due
    }

    /// Called by the host's timer at `now`: once the burst has settled, reads the folder again when it changed, and
    /// returns what changed; none before the deadline, when nothing is pending, or once the body is closed.
    pub fn poll(&mut self, now: Instant) -> Option<FolderBodyChange> {
        let due = self.due?;
        if self.closed || now < due {
            return None;
        }
        self.due = None;
        let change = FolderBodyChange {
            folder_read: self.folder_changed,
            registration: self.registration_changed,
        };
        self.folder_changed = false;
        self.registration_changed = false;
        if change.folder_read {
            self.reread();
        }
        (change.folder_read || change.registration).then_some(change)
    }

    /// Stops the settle delay: a change notified before is never read, and one notified after is ignored.
    pub fn close(&mut self) {
        self.closed = true;
        self.due = None;
    }

    /// The plan for one model change. A read-only binding's plugin is never asked to plan (FBL section 11.4); a
    /// plan the plugin does make is refused too, because this host does not yet write a folder subject's files
    /// (FBL section 10.4).
    pub fn plan(&self, change: &ModelChange) -> PlanResult {
        let Some(plugin) = &self.plugin else {
            return PlanResult::Refused(self.missing_reason());
        };
        if self.last.unreadable {
            return PlanResult::Refused("The folder could not be read, so it is never written.".into());
        }
        if let Some(reason) = &self.binding.read_only {
            return PlanResult::Refused(if reason.is_empty() {
                "This folder is read-only.".into()
            } else {
                reason.clone()
            });
        }
        match plugin.plan(&PluginPlanRequest {
            files: &self.files,
            last: &self.last,
            change,
        }) {
            PluginPlanResult::Refused(reason) => PlanResult::Refused(reason),
            PluginPlanResult::Planned(_) => PlanResult::Refused(
                "The files of a folder are not written by this host, so the change is not applied.".into(),
            ),
        }
    }

    fn missing_reason(&self) -> String {
        format!(
            "The plugin '{}' that reads this folder is not installed, so it is opened read-only.",
            self.binding
                .plugin
                .as_ref()
                .map(|p| p.plugin.as_str())
                .unwrap_or_default()
        )
    }

    /// Whether a change at `changed` can change the reading: a file read, a folder holding one, a path a file rule
    /// or `recognise` selects or a folder that could hold one, or a path the plugin's `watch` named or one under it.
    fn concerns(&self, changed: &Path) -> bool {
        if self
            .watched
            .iter()
            .any(|w| same_path(changed, w) || relative_to(w, changed).is_some_and(|r| !r.is_empty()))
        {
            return true;
        }
        let Some(relative) = relative_to(&self.folder, changed) else {
            return false;
        };
        if relative.is_empty() {
            return true;
        }
        let caseless = platform_ignores_case();
        let under = format!("{relative}/");
        if self.files.iter().any(|f| {
            same_text(&f.relative_path, &relative, caseless) || starts_with(&f.relative_path, &under, caseless)
        }) {
            return true;
        }
        if self
            .binding
            .body
            .ignore
            .iter()
            .any(|g| glob_matches(g, &relative, caseless))
        {
            return false;
        }
        self.globs()
            .any(|g| glob_matches(g, &relative, caseless) || could_hold(g, &relative, caseless))
    }

    /// The globs whose matches change the reading: the file rules' and `recognise`'s.
    fn globs(&self) -> impl Iterator<Item = &String> {
        let body = &self.binding.body;
        body.files
            .iter()
            .map(|r| &r.glob)
            .chain(&body.recognise_all)
            .chain(&body.recognise_any)
            .chain(&body.recognise_none)
    }

    /// The existing folders a file rule's glob could add a file to: the folder itself, and every folder below it
    /// that could hold a match, never through a reparse point and never an ignored one.
    fn glob_folders(&self) -> Vec<PathBuf> {
        let mut found = vec![self.folder.clone()];
        if !self.folder.is_dir() {
            return found;
        }
        let caseless = platform_ignores_case();
        let globs: Vec<&String> = self.binding.body.files.iter().map(|r| &r.glob).collect();
        let mut pending = vec![(self.folder.clone(), String::new())];
        while let Some((directory, prefix)) = pending.pop() {
            // A folder removed while walked: the change that removed it is notified, and read after it settles.
            let Ok(entries) = std::fs::read_dir(&directory) else {
                continue;
            };
            for entry in entries.flatten() {
                let Ok(metadata) = std::fs::symlink_metadata(entry.path()) else {
                    continue;
                };
                if metadata.file_type().is_symlink() || !metadata.is_dir() {
                    continue;
                }
                let relative = format!("{prefix}{}", entry.file_name().to_string_lossy());
                if self
                    .binding
                    .body
                    .ignore
                    .iter()
                    .any(|g| glob_matches(g, &relative, caseless))
                {
                    continue;
                }
                if !globs.iter().any(|g| could_hold(g, &relative, caseless)) {
                    continue;
                }
                found.push(entry.path());
                pending.push((entry.path(), format!("{relative}/")));
            }
        }
        found
    }

    fn full_of(&self, relative: &str) -> PathBuf {
        self.folder.join(relative.replace('/', MAIN_SEPARATOR_STR))
    }

    fn read_now(&mut self) -> PluginReadResult {
        self.files = Vec::new();
        self.unopened = Vec::new();
        self.watched = Vec::new();
        if !routing::recognise(&self.binding, &self.folder, None) {
            return unreadable(if self.folder.is_dir() {
                "The folder is not one this binding recognises, so it is not read."
            } else {
                "The folder no longer exists, so it is not read."
            });
        }
        let Some(plugin) = self.plugin.clone() else {
            return PluginReadResult::new(
                Vec::new(),
                vec![finding(
                    codes::PLUGIN_MISSING,
                    FindingSeverity::Warning,
                    self.missing_reason(),
                    None,
                )],
                false,
            );
        };
        let mut files = Vec::new();
        let mut unopened = Vec::new();
        for file in routing::folder_files(&self.binding, &self.folder, None) {
            match self.load(&file) {
                Some(loaded) => files.push(loaded),
                None => unopened.push(file.relative_path),
            }
        }
        if !self.folder.is_dir() {
            return unreadable("The folder no longer exists, so it is not read.");
        }
        let mut entries = self
            .options
            .supplementary_entries
            .as_ref()
            .map(|of| of(&self.folder))
            .unwrap_or_default();
        entries.sort();
        let request = PluginReadRequest {
            files,
            supplementary_entries: entries,
        };
        let result = plugin.read(&request);
        self.files = request.files;
        self.watched = plugin
            .watch(&result)
            .into_iter()
            .map(|p| {
                let path = Path::new(&p);
                if path.is_absolute() {
                    full(path)
                } else {
                    full(&self.folder.join(path))
                }
            })
            .collect();
        let mut findings: Vec<Finding> = unopened.iter().map(|f| unopened_finding(f)).collect();
        findings.extend(located(&result));
        self.unopened = unopened;
        PluginReadResult { findings, ..result }
    }

    /// Reads at the sharing the host's own reader uses, so a concurrent save is never refused; a file over the
    /// limit is not read at all. None when the file cannot be opened or read: removed since it was selected, held
    /// by a process that shares nothing, or not permitted (FBL section 10.3).
    fn load(&self, file: &FolderFile) -> Option<PluginFile> {
        let mut opened = std::fs::File::open(&file.full_path).ok()?;
        let length = opened.metadata().ok()?.len();
        if length > u64::try_from(self.options.max_body_bytes).unwrap_or(u64::MAX) {
            return Some(PluginFile {
                is_oversized: true,
                ..PluginFile::new(file.relative_path.clone(), Vec::new())
            });
        }
        let mut bytes = Vec::new();
        opened.read_to_end(&mut bytes).ok()?;
        Some(PluginFile::new(file.relative_path.clone(), bytes))
    }
}

fn finding(code: &str, severity: FindingSeverity, message: String, location: Option<SourceLocation>) -> Finding {
    Finding {
        code: code.into(),
        severity,
        message,
        location,
        detail: None,
    }
}

/// A selected file that could not be opened (FBL section 10.3): DISL's `std.unparseable` at that file, at no line,
/// since none of it was read. The sentence names no path of the machine, as an operating system error's would.
fn unopened_finding(relative_path: &str) -> Finding {
    finding(
        codes::UNPARSEABLE,
        FindingSeverity::Error,
        "The file could not be opened, so it is not read.".into(),
        Some(SourceLocation {
            file: relative_path.to_owned(),
            line: 0,
            column: 0,
            length: 0,
        }),
    )
}

fn unreadable(reason: &str) -> PluginReadResult {
    PluginReadResult::new(
        Vec::new(),
        vec![finding(codes::UNPARSEABLE, FindingSeverity::Error, reason.into(), None)],
        true,
    )
}

/// A finding the plugin left without a file, about an element it read in one (its [`SUBJECT_DETAIL`]), is located
/// in that element's file, at the finding's line or else the element's.
fn located(result: &PluginReadResult) -> Vec<Finding> {
    result
        .findings
        .iter()
        .map(|f| {
            if f.location.as_ref().is_some_and(|l| !l.file.is_empty()) {
                return f.clone();
            }
            let Some(subject) = f
                .detail
                .as_ref()
                .and_then(|d| d.iter().find(|(k, _)| k == SUBJECT_DETAIL))
                .map(|(_, v)| v)
            else {
                return f.clone();
            };
            let Some((element, file)) = result
                .elements
                .iter()
                .find(|e| &e.id == subject)
                .and_then(|e| e.file.as_ref().filter(|file| !file.is_empty()).map(|file| (e, file)))
            else {
                return f.clone();
            };
            let location = match &f.location {
                Some(given) => SourceLocation {
                    file: file.clone(),
                    ..given.clone()
                },
                None => SourceLocation {
                    file: file.clone(),
                    line: element.line,
                    column: 1,
                    length: 0,
                },
            };
            Finding {
                location: Some(location),
                ..f.clone()
            }
        })
        .collect()
}

/// Whether a folder at `relative` could hold a path `glob` matches.
fn could_hold(glob: &str, relative: &str, caseless: bool) -> bool {
    let pattern: Vec<&str> = glob.split('/').collect();
    let path: Vec<&str> = relative.split('/').collect();
    for (i, segment) in path.iter().enumerate() {
        if i >= pattern.len() {
            return false;
        }
        if pattern[i] == "**" {
            return true;
        }
        if i == pattern.len() - 1 || !glob_matches(pattern[i], segment, caseless) {
            return false;
        }
    }
    true
}

/// A path made full against the current directory, as the standalone host's `Path.GetFullPath`, without a trailing
/// separator.
fn full(path: &Path) -> PathBuf {
    let made = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let text = made.to_string_lossy();
    let trimmed = text.trim_end_matches(['/', MAIN_SEPARATOR]);
    if trimmed.len() < text.len() && !trimmed.is_empty() && !trimmed.ends_with(':') {
        PathBuf::from(trimmed)
    } else {
        made
    }
}

fn same_text(a: &str, b: &str, caseless: bool) -> bool {
    if caseless { a.eq_ignore_ascii_case(b) } else { a == b }
}

fn starts_with(text: &str, prefix: &str, caseless: bool) -> bool {
    text.len() >= prefix.len()
        && text.is_char_boundary(prefix.len())
        && same_text(&text[..prefix.len()], prefix, caseless)
}

/// Whether two full paths name one entry, ignoring case where the platform does.
fn same_path(a: &Path, b: &Path) -> bool {
    same_text(&a.to_string_lossy(), &b.to_string_lossy(), platform_ignores_case())
}

/// `path` relative to `folder`, `/`-separated: empty for the folder itself, none for a path outside it.
fn relative_to(folder: &Path, path: &Path) -> Option<String> {
    let folder = folder.to_string_lossy();
    let path = path.to_string_lossy();
    let caseless = platform_ignores_case();
    if same_text(&folder, &path, caseless) {
        return Some(String::new());
    }
    let under = format!("{folder}{MAIN_SEPARATOR}");
    if !starts_with(&path, &under, caseless) {
        return None;
    }
    Some(path[under.len()..].replace(MAIN_SEPARATOR, "/"))
}
