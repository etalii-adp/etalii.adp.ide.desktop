//! The workspace tree, live watching, and the rules for names and registrations (etalii.adp spec 030, FR-013,
//! FR-023, task T026).
//!
//! Ported from the standalone host's `EtAlii.Adp.Hierarchy` at `8b6ccfc`: registrations (`.adp` files) nest under
//! the file they register and never under a folder, a folder's own registration is the bare `.adp` inside it, a
//! registration whose subject is missing stays where it is and is marked, and names are judged by one set of rules for
//! renaming and creating alike. The explorer's actions use these; the tree follows the disk through a [`Watcher`].

use std::cmp::Ordering;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher as _};

/// The extension of a registration file.
pub const REGISTRATION_EXTENSION: &str = ".adp";

/// Folders the tree never shows: version control and build output that belong to other tools.
const HIDDEN_FOLDERS: &[&str] = &[".git", ".hg", ".svn"];

/// How a registration file names its subject (the standalone host's `DiagramRegistrationName`): `subject.adp`,
/// `subject.qualifier.adp` when a subject carries several, or the bare `.adp` registering its folder. The parse is
/// shape only: `my.config.adp` reads both as the subject `my.config` and as `my` qualified by `config`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistrationName {
    /// Everything before the extension; empty for the folder's own registration.
    pub full_base: String,
    /// The base without its last dot-separated segment, when it has one.
    pub subject_base: String,
    /// The last dot-separated segment, or empty.
    pub qualifier: String,
    pub folder_scoped: bool,
}

impl RegistrationName {
    pub fn parse(file_name: &str) -> Option<Self> {
        let lower = file_name.to_ascii_lowercase();
        if !lower.ends_with(REGISTRATION_EXTENSION) {
            return None;
        }
        let full_base = &file_name[..file_name.len() - REGISTRATION_EXTENSION.len()];
        if full_base.is_empty() {
            return Some(Self {
                full_base: String::new(),
                subject_base: String::new(),
                qualifier: String::new(),
                folder_scoped: true,
            });
        }
        // A leading dot is not a separator: `.hidden.adp` is the subject `.hidden`.
        Some(match full_base.rfind('.') {
            Some(at) if at > 0 => Self {
                full_base: full_base.into(),
                subject_base: full_base[..at].into(),
                qualifier: full_base[at + 1..].into(),
                folder_scoped: false,
            },
            _ => Self {
                full_base: full_base.into(),
                subject_base: full_base.into(),
                qualifier: String::new(),
                folder_scoped: false,
            },
        })
    }

    pub fn is_qualified(&self) -> bool {
        !self.qualifier.is_empty()
    }
}

/// The place one registration takes among its siblings (the standalone host's `NestedEntry`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nested {
    pub name: String,
    /// The sibling file it nests under, or `None` when it stays where it is.
    pub subject: Option<String>,
    /// A subject was resolved but is not there: shown where it is, marked, so the broken pair can be fixed.
    pub orphan: bool,
}

/// Places every registration among one folder's `(name, is_folder)` siblings (the standalone host's
/// `HierarchyNesting.Assign`). `body_of` gives the body file name a registration resolves in this folder, or `None`.
pub fn nest(siblings: &[(String, bool)], body_of: impl Fn(&str) -> Option<String>) -> Vec<Nested> {
    let files: Vec<&str> = siblings
        .iter()
        .filter(|(_, folder)| !folder)
        .map(|(n, _)| n.as_str())
        .collect();
    let mut placed = Vec::new();
    for (name, is_folder) in siblings {
        if *is_folder {
            continue;
        }
        let Some(registration) = RegistrationName::parse(name) else {
            continue;
        };
        if registration.folder_scoped {
            placed.push(Nested {
                name: name.clone(),
                subject: None,
                orphan: false,
            });
            continue;
        }
        match body_of(name) {
            None => placed.push(Nested {
                name: name.clone(),
                subject: None,
                orphan: false,
            }),
            Some(body) => match files.iter().find(|f| f.eq_ignore_ascii_case(&body)) {
                Some(subject) => placed.push(Nested {
                    name: name.clone(),
                    subject: Some((*subject).into()),
                    orphan: false,
                }),
                None => placed.push(Nested {
                    name: name.clone(),
                    subject: None,
                    orphan: true,
                }),
            },
        }
    }
    placed
}

/// The order of one subject's registrations: the unqualified one first, then by qualifier, ordinal.
pub fn compare_registrations(a: &str, b: &str) -> Ordering {
    match (RegistrationName::parse(a), RegistrationName::parse(b)) {
        (Some(a), Some(b)) => a
            .is_qualified()
            .cmp(&b.is_qualified())
            .then_with(|| a.qualifier.cmp(&b.qualifier)),
        _ => a.cmp(b),
    }
}

/// Characters no portable file name may hold, whatever the platform allows.
const INVALID: &[char] = &['\\', '/', ':', '*', '?', '"', '<', '>', '|'];

/// Judges a name for an entry directly inside `parent` (the standalone host's `EntryNameRules`): for a new entry, or
/// for renaming the entry now called `current`.
pub fn validate_name(name: &str, parent: &Path, current: Option<&str>) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Enter a name.".into());
    }
    if name == "." || name == ".." || name.contains(INVALID) || name.chars().any(char::is_control) {
        return Err("A name cannot contain a path or any of \\ / : * ? \" < > |".into());
    }
    if current == Some(name) {
        return Err("Enter a name that differs from the current one.".into());
    }
    let same_entry = current.is_some_and(|c| c.eq_ignore_ascii_case(name));
    if !same_entry && parent.join(name).symlink_metadata().is_ok() {
        return Err(format!("An item named '{name}' already exists in this folder."));
    }
    Ok(())
}

/// What a node of the tree is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Folder,
    File,
    Registration,
}

/// One entry of the tree; a file's children are the registrations nested under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub name: String,
    /// Relative to the workspace's root, with `/` between segments.
    pub path: String,
    pub kind: Kind,
    pub orphan: bool,
    pub children: Vec<Node>,
}

/// The workspace's tree, read from disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hierarchy {
    pub root: PathBuf,
    pub children: Vec<Node>,
}

impl Hierarchy {
    /// Reads the whole tree under `root`.
    pub fn scan(root: &Path) -> io::Result<Self> {
        Ok(Self {
            root: root.to_path_buf(),
            children: scan_folder(root, "")?,
        })
    }

    /// Reads one folder again after a change inside it, replacing its children in the tree.
    pub fn rescan(&mut self, folder: &str) -> io::Result<()> {
        let children = scan_folder(&self.root.join(folder), folder)?;
        if folder.is_empty() {
            self.children = children;
            return Ok(());
        }
        if let Some(node) = find_mut(&mut self.children, folder) {
            node.children = children;
        }
        Ok(())
    }

    pub fn find(&self, path: &str) -> Option<&Node> {
        fn walk<'a>(nodes: &'a [Node], path: &str) -> Option<&'a Node> {
            nodes.iter().find_map(|n| {
                if n.path == path {
                    Some(n)
                } else {
                    walk(&n.children, path)
                }
            })
        }
        walk(&self.children, path)
    }
}

fn find_mut<'a>(nodes: &'a mut [Node], path: &str) -> Option<&'a mut Node> {
    for node in nodes {
        if node.path == path {
            return Some(node);
        }
        if path.starts_with(&format!("{}/", node.path))
            && let Some(found) = find_mut(&mut node.children, path)
        {
            return Some(found);
        }
    }
    None
}

fn scan_folder(folder: &Path, relative: &str) -> io::Result<Vec<Node>> {
    let mut siblings = Vec::new();
    for entry in fs::read_dir(folder)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let is_folder = entry.file_type()?.is_dir();
        if is_folder && HIDDEN_FOLDERS.contains(&name.as_str()) {
            continue;
        }
        siblings.push((name, is_folder));
    }
    let names: Vec<String> = siblings.iter().map(|(n, _)| n.clone()).collect();
    let placed = nest(&siblings, |registration| body_of(folder, registration, &names));
    let join = |name: &str| {
        if relative.is_empty() {
            name.to_owned()
        } else {
            format!("{relative}/{name}")
        }
    };

    let mut nodes = Vec::new();
    for (name, is_folder) in &siblings {
        let path = join(name);
        if *is_folder {
            let children = scan_folder(&folder.join(name), &path)?;
            nodes.push(Node {
                name: name.clone(),
                path,
                kind: Kind::Folder,
                orphan: false,
                children,
            });
            continue;
        }
        if let Some(place) = placed.iter().find(|p| &p.name == name) {
            if place.subject.is_some() {
                continue; // shown under its subject only
            }
            nodes.push(Node {
                name: name.clone(),
                path,
                kind: Kind::Registration,
                orphan: place.orphan,
                children: Vec::new(),
            });
            continue;
        }
        let mut registrations: Vec<&Nested> = placed
            .iter()
            .filter(|p| p.subject.as_deref().is_some_and(|s| s.eq_ignore_ascii_case(name)))
            .collect();
        registrations.sort_by(|a, b| compare_registrations(&a.name, &b.name));
        let children = registrations
            .into_iter()
            .map(|r| Node {
                name: r.name.clone(),
                path: join(&r.name),
                kind: Kind::Registration,
                orphan: false,
                children: Vec::new(),
            })
            .collect();
        nodes.push(Node {
            name: name.clone(),
            path,
            kind: Kind::File,
            orphan: false,
            children,
        });
    }
    nodes.sort_by(|a, b| {
        let rank = |n: &Node| if n.kind == Kind::Folder { 0 } else { 1 };
        rank(a)
            .cmp(&rank(b))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(nodes)
}

/// The body file a registration resolves in its own folder: the `body:` header when it names a file beside it, and
/// otherwise the sibling its name derives (`ideas.mm.adp` → `ideas.mm`, `ideas.second.adp` → `ideas`). A body in
/// another folder or a name that derives no existing file resolves nothing here. Resolving by the tool type's
/// extension, as the standalone host's `DiagramFilePair` does, joins with the catalogue (T048).
fn body_of(folder: &Path, registration: &str, siblings: &[String]) -> Option<String> {
    if let Some(body) = body_header(&folder.join(registration)) {
        return (!body.contains(['/', '\\'])).then_some(body);
    }
    let name = RegistrationName::parse(registration)?;
    let exists = |candidate: &str| siblings.iter().any(|s| s.eq_ignore_ascii_case(candidate));
    [name.full_base, name.subject_base]
        .into_iter()
        .find(|candidate| !candidate.is_empty() && exists(candidate))
}

/// The `body:` header among the first lines after the origin, as FBL section 8 places it.
fn body_header(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    text.lines()
        .skip(1)
        .take(8)
        .find_map(|line| line.strip_prefix("body:").map(|b| b.trim().to_owned()))
        .filter(|b| !b.is_empty())
}

/// One change on disk, by its absolute path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    Added(PathBuf),
    Removed(PathBuf),
    Renamed { from: PathBuf, to: PathBuf },
    Modified(PathBuf),
}

/// Watches a workspace for changes on disk and hands them over in the order they happened.
pub struct Watcher {
    _watcher: RecommendedWatcher,
    changes: Receiver<Change>,
}

impl Watcher {
    pub fn start(root: &Path) -> notify::Result<Self> {
        Self::start_waking(root, || {})
    }

    /// Starts watching `root`, calling `wake` on the watcher's thread whenever it has handed over changes, so a
    /// window waiting for input can draw them without being asked otherwise.
    pub fn start_waking(root: &Path, wake: impl Fn() + Send + 'static) -> notify::Result<Self> {
        let (send, changes) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
            let Ok(event) = event else { return };
            let mut sent = false;
            for change in changes_of(event) {
                sent |= send.send(change).is_ok();
            }
            if sent {
                wake();
            }
        })?;
        watcher.watch(root, RecursiveMode::Recursive)?;
        Ok(Self {
            _watcher: watcher,
            changes,
        })
    }

    pub fn changes(&self) -> &Receiver<Change> {
        &self.changes
    }
}

fn changes_of(event: notify::Event) -> Vec<Change> {
    use notify::event::{ModifyKind, RenameMode};
    let mut paths = event.paths.into_iter();
    match event.kind {
        EventKind::Create(_) => paths.map(Change::Added).collect(),
        EventKind::Remove(_) => paths.map(Change::Removed).collect(),
        EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => match (paths.next(), paths.next()) {
            (Some(from), Some(to)) => vec![Change::Renamed { from, to }],
            (Some(path), None) => vec![Change::Modified(path)],
            _ => Vec::new(),
        },
        EventKind::Modify(ModifyKind::Name(RenameMode::From)) => paths.map(Change::Removed).collect(),
        EventKind::Modify(ModifyKind::Name(RenameMode::To)) => paths.map(Change::Added).collect(),
        EventKind::Modify(_) | EventKind::Any | EventKind::Other => paths.map(Change::Modified).collect(),
        EventKind::Access(_) => Vec::new(),
    }
}
