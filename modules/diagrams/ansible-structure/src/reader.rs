//! Reads one registered folder into an [`AnsibleProject`], recognising Ansible's own conventions and ignoring
//! everything else without complaint (etalii.adp spec 030, task T079).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/ansible-structure/backend/
//! EtAlii.Adp.Diagram.AnsibleStructure/AnsibleProjectReader.cs` and `AnsibleYaml.cs`. It reads the files it is handed
//! (the files the binding's file rules select, by their relative paths and bytes) and what [`AnsibleFolderListing`]
//! says of the folder that no file can; it opens no file itself, and nothing here could write one. Recognition is by
//! convention: a playbook is a YAML file whose top level is a non-empty list of mappings, so a data file in the same
//! folder excludes itself. Every enumeration is ordinal, in the reader's own order: the root's playbooks, then
//! `playbooks/`, then roles, then inventories (finding B6), whatever order the files were handed in.

use std::collections::HashMap;

use adp_fbl::FblBinding;
use adp_fbl::plugins::PluginFile;
use adp_fbl::routing::platform_ignores_case;

use crate::listing::AnsibleFolderListing;
use crate::model::{
    AnsibleDirective, AnsibleDirectiveKind, AnsibleFailure, AnsibleInventory, AnsibleInventoryGroup, AnsiblePlay,
    AnsiblePlaybook, AnsibleProject, AnsibleRole, AnsibleRoleContents, AnsibleTaskFile, AnsibleVariableFolder,
};
use crate::yaml::{self, YamlNode};

/// Files whose presence is recorded on the project rather than drawn as a box.
const ANNOTATION_NAMES: [&str; 3] = ["ansible.cfg", "requirements.yml", "requirements.yaml"];

/// A root inventory, for a project that keeps one file rather than an inventories folder.
const ROOT_INVENTORY_NAMES: [&str; 6] = [
    "inventory",
    "inventory.yml",
    "inventory.yaml",
    "hosts",
    "hosts.yml",
    "hosts.yaml",
];

/// The include and import directives a task may carry, by the key's last segment.
const DIRECTIVE_NAMES: [(&str, AnsibleDirectiveKind); 4] = [
    ("include_tasks", AnsibleDirectiveKind::IncludeTasks),
    ("import_tasks", AnsibleDirectiveKind::ImportTasks),
    ("include_role", AnsibleDirectiveKind::IncludeRole),
    ("import_role", AnsibleDirectiveKind::ImportRole),
];

/// What one YAML file read to: its first document's root (none for an empty file), or why it did not parse.
enum Parsed {
    Document(Option<YamlNode>),
    Unreadable(AnsibleFailure),
}

/// One tolerant parse of one handed file (`AnsibleYaml.Read`): decoded as UTF-8, a byte order mark taken off.
fn parse(file: &PluginFile, relative_path: &str) -> Parsed {
    if file.is_oversized {
        return Parsed::Unreadable(AnsibleFailure {
            relative_path: relative_path.to_owned(),
            message: "The file is larger than this host reads, so it is not read.".into(),
            line: 0,
        });
    }
    let text = text_of(file);
    match yaml::parse(&text) {
        Ok(root) => Parsed::Document(root),
        Err(error) => Parsed::Unreadable(AnsibleFailure {
            relative_path: relative_path.to_owned(),
            message: error.message,
            line: u32::try_from(error.line).unwrap_or(0),
        }),
    }
}

/// The file's text, as the shared reader decodes a file on disk.
fn text_of(file: &PluginFile) -> String {
    let text = String::from_utf8_lossy(&file.bytes);
    text.strip_prefix('\u{feff}').unwrap_or(&text).to_owned()
}

fn same_path(a: &str, b: &str) -> bool {
    if platform_ignores_case() {
        a.eq_ignore_ascii_case(b)
    } else {
        a == b
    }
}

fn key_of(path: &str) -> String {
    if platform_ignores_case() {
        path.to_ascii_lowercase()
    } else {
        path.to_owned()
    }
}

/// The handed files and the listing of one reading: where the reader looks in place of the disk.
struct Reading<'a> {
    handed: Vec<PluginFile>,
    by_key: HashMap<String, usize>,
    listing: &'a AnsibleFolderListing,
}

impl Reading<'_> {
    /// The handed files directly in `folder`, in ordinal order, each named by that folder as written here and its own
    /// name.
    fn files_in(&self, folder: &str) -> Vec<(String, &PluginFile)> {
        let mut files: Vec<(String, &PluginFile)> = self
            .handed
            .iter()
            .filter(|f| same_path(parent_of(&f.relative_path), folder))
            .map(|f| (join(folder, name_of(&f.relative_path)), f))
            .collect();
        files.sort_by(|a, b| a.0.cmp(&b.0));
        files
    }

    fn find(&self, relative: &str) -> Option<&PluginFile> {
        self.by_key.get(&key_of(relative)).map(|&i| &self.handed[i])
    }
}

fn parent_of(relative: &str) -> &str {
    relative.rfind('/').map_or("", |slash| &relative[..slash])
}

fn name_of(relative: &str) -> &str {
    relative.rfind('/').map_or(relative, |slash| &relative[slash + 1..])
}

fn join(folder: &str, name: &str) -> String {
    if folder.is_empty() {
        name.to_owned()
    } else {
        format!("{folder}/{name}")
    }
}

/// `Path.GetExtension`: from the last dot of the file name on, empty when it has none or ends in one.
fn extension_of(relative: &str) -> &str {
    let name = name_of(relative);
    match name.rfind('.') {
        Some(dot) if dot + 1 < name.len() => &name[dot..],
        _ => "",
    }
}

fn is_yaml(relative: &str) -> bool {
    matches!(extension_of(relative), ".yml" | ".yaml")
}

/// Reads the folder `listing` lists from `files`, the files `binding` selects with their bytes, and from the listing.
pub fn read(files: &[PluginFile], listing: &AnsibleFolderListing, binding: &FblBinding) -> AnsibleProject {
    let mut handed: Vec<PluginFile> = Vec::new();
    let mut by_key: HashMap<String, usize> = HashMap::new();
    for file in files.iter().cloned().chain(listing.linked_files(binding)) {
        let key = key_of(&file.relative_path);
        if by_key.contains_key(&key) {
            continue;
        }
        by_key.insert(key, handed.len());
        handed.push(file);
    }
    let reading = Reading {
        handed,
        by_key,
        listing,
    };
    let mut failures = Vec::new();
    let playbooks = read_playbooks(&reading, &mut failures);
    let roles = read_roles(&reading, &mut failures);
    let inventories = read_inventories(&reading);
    let annotations = read_annotations(&reading);
    AnsibleProject {
        folder_path: listing.root().to_string_lossy().into_owned(),
        playbooks,
        roles,
        inventories,
        annotations,
        failures,
    }
}

/// The handed YAML files directly in `folder`, by their exact extension, in ordinal order.
fn yaml_files_in<'a>(reading: &'a Reading<'_>, folder: &str) -> Vec<(String, &'a PluginFile)> {
    reading
        .files_in(folder)
        .into_iter()
        .filter(|(relative, _)| is_yaml(relative))
        .collect()
}

fn record(failure: AnsibleFailure, failures: &mut Vec<AnsibleFailure>) {
    if !failures
        .iter()
        .any(|f| f.relative_path.eq_ignore_ascii_case(&failure.relative_path))
    {
        failures.push(failure);
    }
}

fn line_of(node: &YamlNode) -> u32 {
    u32::try_from(node.start().line).unwrap_or(0)
}

/// A `when:` as written; a list of conditions joined the way a reader would read it.
fn condition(mapping: Option<&YamlNode>) -> String {
    match mapping.and_then(|m| m.get("when")) {
        Some(YamlNode::Scalar { value, .. }) => value.clone(),
        Some(YamlNode::Sequence { items, .. }) => items
            .iter()
            .filter_map(YamlNode::as_scalar)
            .collect::<Vec<_>>()
            .join(" and "),
        _ => String::new(),
    }
}

fn directive(
    kind: AnsibleDirectiveKind,
    target: &str,
    condition: String,
    relative: &str,
    line: u32,
) -> AnsibleDirective {
    AnsibleDirective {
        kind,
        target: target.to_owned(),
        condition,
        declared_in: relative.to_owned(),
        line,
    }
}

// ---- playbooks -------------------------------------------------------------------------------------------------------

fn read_playbooks(reading: &Reading<'_>, failures: &mut Vec<AnsibleFailure>) -> Vec<AnsiblePlaybook> {
    // Root level and playbooks/: the two places the recommended layout puts them.
    let mut candidates = yaml_files_in(reading, "");
    candidates.extend(yaml_files_in(reading, "playbooks"));

    let mut playbooks = Vec::new();
    for (relative, file) in candidates {
        let root = match parse(file, &relative) {
            Parsed::Document(root) => root,
            Parsed::Unreadable(failure) => {
                record(failure, failures);
                continue;
            }
        };
        // The shape test that identifies a playbook: a list whose entries are mappings.
        let Some(YamlNode::Sequence { items, .. }) = &root else {
            continue;
        };
        if items.is_empty() || !items.iter().all(|i| matches!(i, YamlNode::Mapping { .. })) {
            continue;
        }

        let mut plays = Vec::new();
        let mut imports = Vec::new();
        let mut index = 0;
        for entry in items {
            // import_playbook is a sibling of the plays, not something a play contains.
            if let Some(imported) = entry.scalar("import_playbook") {
                imports.push(directive(
                    AnsibleDirectiveKind::ImportPlaybook,
                    imported,
                    condition(Some(entry)),
                    &relative,
                    line_of(entry),
                ));
                continue;
            }
            plays.push(read_play(entry, index, &relative));
            index += 1;
        }
        playbooks.push(AnsiblePlaybook {
            name: name_of(&relative).to_owned(),
            relative_path: relative,
            plays,
            imports,
        });
    }
    playbooks
}

fn read_play(play: &YamlNode, index: usize, relative: &str) -> AnsiblePlay {
    let mut directives = Vec::new();
    // roles: is a list whose entries are either a name or a mapping with role: and a when:.
    if let Some(YamlNode::Sequence { items, .. }) = play.get("roles") {
        for entry in items {
            match entry {
                YamlNode::Scalar { value, .. } => directives.push(directive(
                    AnsibleDirectiveKind::Roles,
                    value,
                    String::new(),
                    relative,
                    line_of(entry),
                )),
                YamlNode::Mapping { .. } => {
                    if let Some(named) = entry.scalar("role") {
                        directives.push(directive(
                            AnsibleDirectiveKind::Roles,
                            named,
                            condition(Some(entry)),
                            relative,
                            line_of(entry),
                        ));
                    }
                }
                YamlNode::Sequence { .. } => {}
            }
        }
    }
    // A play's tasks may also reach for a role directly.
    directives.extend(task_directives(play, relative));
    AnsiblePlay {
        name: play.scalar("name").unwrap_or_default().to_owned(),
        hosts: play.scalar("hosts").unwrap_or_default().to_owned(),
        index,
        line: line_of(play),
        directives,
    }
}

// ---- roles -----------------------------------------------------------------------------------------------------------

fn read_roles(reading: &Reading<'_>, failures: &mut Vec<AnsibleFailure>) -> Vec<AnsibleRole> {
    let listing = reading.listing;
    if !listing.folder_exists("roles") {
        return Vec::new();
    }
    let mut roles = Vec::new();
    for name in listing.folders("roles") {
        let folder = format!("roles/{name}");
        let task_files: Vec<AnsibleTaskFile> = yaml_files_in(reading, &format!("{folder}/tasks"))
            .into_iter()
            .map(|(relative, _)| AnsibleTaskFile {
                name: name_of(&relative).to_owned(),
                relative_path: relative,
            })
            .collect();
        let contents = AnsibleRoleContents {
            task_files: task_files.len(),
            handlers: listing.file_count(&format!("{folder}/handlers")),
            templates: listing.file_count(&format!("{folder}/templates")),
            files: listing.file_count(&format!("{folder}/files")),
            defaults: listing.file_count(&format!("{folder}/defaults")),
            vars: listing.file_count(&format!("{folder}/vars")),
            has_meta: listing.folder_exists(&format!("{folder}/meta")),
            has_library: listing.folder_exists(&format!("{folder}/library")),
        };
        let dependencies = read_dependencies(reading, &folder, failures);
        let task_includes = read_task_includes(reading, &folder, failures);
        roles.push(AnsibleRole {
            name,
            relative_path: folder,
            contents,
            task_files,
            dependencies,
            task_includes,
        });
    }
    roles
}

fn read_dependencies(
    reading: &Reading<'_>,
    role_folder: &str,
    failures: &mut Vec<AnsibleFailure>,
) -> Vec<AnsibleDirective> {
    let Some((relative, file)) = ["main.yml", "main.yaml"]
        .iter()
        .map(|name| format!("{role_folder}/meta/{name}"))
        .find_map(|relative| reading.find(&relative).map(|file| (relative, file)))
    else {
        return Vec::new();
    };
    let root = match parse(file, &relative) {
        Parsed::Document(Some(root @ YamlNode::Mapping { .. })) => root,
        Parsed::Document(_) => return Vec::new(),
        Parsed::Unreadable(failure) => {
            record(failure, failures);
            return Vec::new();
        }
    };
    let Some(YamlNode::Sequence { items, .. }) = root.get("dependencies") else {
        return Vec::new();
    };
    let mut directives = Vec::new();
    for entry in items {
        // Both shapes Ansible accepts: a bare name, or a mapping naming role: (or name:).
        let (target, mapping) = match entry {
            YamlNode::Scalar { value, .. } => (Some(value.as_str()), None),
            YamlNode::Mapping { .. } => (entry.scalar("role").or_else(|| entry.scalar("name")), Some(entry)),
            YamlNode::Sequence { .. } => (None, None),
        };
        if let Some(target) = target {
            directives.push(directive(
                AnsibleDirectiveKind::Dependency,
                target,
                condition(mapping),
                &relative,
                line_of(entry),
            ));
        }
    }
    directives
}

fn read_task_includes(
    reading: &Reading<'_>,
    role_folder: &str,
    failures: &mut Vec<AnsibleFailure>,
) -> Vec<AnsibleDirective> {
    let mut directives = Vec::new();
    for (relative, file) in yaml_files_in(reading, &format!("{role_folder}/tasks")) {
        let items = match parse(file, &relative) {
            Parsed::Document(Some(YamlNode::Sequence { items, .. })) => items,
            Parsed::Document(_) => continue,
            Parsed::Unreadable(failure) => {
                record(failure, failures);
                continue;
            }
        };
        for task in items.iter().filter(|t| matches!(t, YamlNode::Mapping { .. })) {
            directives.extend(task_directives(task, &relative));
        }
    }
    directives
}

/// Matches `include_tasks` and `ansible.builtin.include_tasks` alike, and nothing else.
fn matches_directive(key: &str, suffix: &str) -> bool {
    key == suffix
        || (key.len() > suffix.len() && key.ends_with(suffix) && key.as_bytes()[key.len() - suffix.len() - 1] == b'.')
}

/// The include and import directives one task mapping carries, in the short form and the fully qualified one.
fn task_directives(task: &YamlNode, relative: &str) -> Vec<AnsibleDirective> {
    let YamlNode::Mapping { entries, .. } = task else {
        return Vec::new();
    };
    let mut directives = Vec::new();
    for (suffix, kind) in DIRECTIVE_NAMES {
        for (key, value) in entries {
            let Some(name) = key.as_scalar() else {
                continue;
            };
            if !matches_directive(name, suffix) {
                continue;
            }
            // include_tasks: x.yml, or include_tasks: { file: x.yml } / { name: x } for a role.
            let target = match value {
                YamlNode::Scalar { value, .. } => Some(value.as_str()),
                YamlNode::Mapping { .. } => value.scalar("file").or_else(|| value.scalar("name")),
                YamlNode::Sequence { .. } => None,
            };
            if let Some(target) = target {
                directives.push(directive(kind, target, condition(Some(task)), relative, line_of(key)));
            }
        }
    }
    directives
}

// ---- inventories -----------------------------------------------------------------------------------------------------

fn read_inventories(reading: &Reading<'_>) -> Vec<AnsibleInventory> {
    let mut inventories = Vec::new();
    if reading.listing.folder_exists("inventories") {
        for name in reading.listing.folders("inventories") {
            let environment = format!("inventories/{name}");
            inventories.push(AnsibleInventory {
                groups: groups_in(reading, &environment),
                variable_folders: variable_folders_in(reading, &environment),
                name,
                relative_path: environment,
            });
        }
    }
    // A project that keeps one inventory file at its root; its group_vars and host_vars sit beside it.
    for name in ROOT_INVENTORY_NAMES {
        let Some(file) = reading.find(name) else {
            continue;
        };
        inventories.push(AnsibleInventory {
            name: name.to_owned(),
            relative_path: name.to_owned(),
            groups: groups_of(name, file),
            variable_folders: variable_folders_in(reading, ""),
        });
    }
    inventories
}

fn is_inventory_file(relative: &str) -> bool {
    matches!(extension_of(relative), ".yml" | ".yaml" | "")
        || ROOT_INVENTORY_NAMES
            .iter()
            .any(|n| n.eq_ignore_ascii_case(name_of(relative)))
}

fn groups_in(reading: &Reading<'_>, folder: &str) -> Vec<AnsibleInventoryGroup> {
    reading
        .files_in(folder)
        .into_iter()
        .filter(|(relative, _)| is_inventory_file(relative))
        .flat_map(|(relative, file)| groups_of(&relative, file))
        .collect()
}

/// The groups one inventory file defines: as YAML (`all -> children -> <group> -> hosts`), else as the INI it is. An
/// INI inventory is not the user's mistake, so nothing is recorded as a failure.
pub fn groups_of(relative: &str, file: &PluginFile) -> Vec<AnsibleInventoryGroup> {
    let Parsed::Document(Some(YamlNode::Mapping { entries, .. })) = parse(file, relative) else {
        return ini_groups_of(file);
    };
    let mut groups = Vec::new();
    for (top_key, top) in &entries {
        if !matches!(top, YamlNode::Mapping { .. }) {
            continue;
        }
        if let Some(YamlNode::Mapping { entries: nested, .. }) = top.get("children") {
            for (group_key, group_value) in nested {
                if let Some(group) = group_key.as_scalar() {
                    groups.push(AnsibleInventoryGroup {
                        name: group.to_owned(),
                        host_count: host_count(group_value),
                    });
                }
            }
        }
        // A group declared at the top level rather than under all -> children.
        if top.get("hosts").is_some()
            && let Some(name) = top_key.as_scalar()
            && name != "all"
        {
            groups.push(AnsibleInventoryGroup {
                name: name.to_owned(),
                host_count: host_count(top),
            });
        }
    }
    groups
}

fn host_count(group: &YamlNode) -> usize {
    match group.get("hosts") {
        Some(YamlNode::Mapping { entries, .. }) => entries.len(),
        _ => 0,
    }
}

/// The classic INI inventory: `[group]` sections holding one host per line, `[group:children]` gathering other
/// groups, `[group:vars]` holding configuration rather than membership. A `:children` group holds what its members
/// hold, one level deep.
fn ini_groups_of(file: &PluginFile) -> Vec<AnsibleInventoryGroup> {
    let mut host_counts: HashMap<String, usize> = HashMap::new();
    let mut children_of: Vec<(String, Vec<String>)> = Vec::new();
    let mut order: Vec<String> = Vec::new();
    let mut section: Option<String> = None;
    let mut section_is_children = false;

    let text = text_of(file);
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.len() >= 2 && line.starts_with('[') && line.ends_with(']') {
            let inner = &line[1..line.len() - 1];
            let (name, suffix) = inner.split_once(':').unwrap_or((inner, ""));
            if suffix == "vars" {
                section = None;
                continue;
            }
            section_is_children = suffix == "children";
            section = Some(name.to_owned());
            if !host_counts.contains_key(name) {
                host_counts.insert(name.to_owned(), 0);
                order.push(name.to_owned());
            }
            continue;
        }
        let Some(current) = &section else {
            // An ungrouped host above the first section belongs to no group.
            continue;
        };
        let entry = line.split_whitespace().next().unwrap_or_default().to_owned();
        if section_is_children {
            match children_of.iter_mut().find(|(g, _)| g == current) {
                Some((_, children)) => children.push(entry),
                None => children_of.push((current.clone(), vec![entry])),
            }
        } else if let Some(count) = host_counts.get_mut(current) {
            *count += 1;
        }
    }
    let direct = host_counts.clone();
    for (group, children) in &children_of {
        let gathered: usize = children.iter().map(|c| direct.get(c).copied().unwrap_or(0)).sum();
        if let Some(count) = host_counts.get_mut(group) {
            *count += gathered;
        }
    }
    order
        .into_iter()
        .map(|name| AnsibleInventoryGroup {
            host_count: host_counts[&name],
            name,
        })
        .collect()
}

fn variable_folders_in(reading: &Reading<'_>, owner: &str) -> Vec<AnsibleVariableFolder> {
    ["group_vars", "host_vars"]
        .iter()
        .filter_map(|name| {
            let path = join(owner, name);
            reading.listing.folder_exists(&path).then(|| AnsibleVariableFolder {
                name: (*name).to_owned(),
                file_count: reading.listing.file_count(&path),
                relative_path: path,
            })
        })
        .collect()
}

// ---- annotations -----------------------------------------------------------------------------------------------------

/// The files whose presence is all that is read of them; the listing says whether each is there.
fn read_annotations(reading: &Reading<'_>) -> Vec<String> {
    let mut annotations: Vec<String> = ANNOTATION_NAMES
        .iter()
        .filter(|name| reading.listing.file_exists(name))
        .map(|name| (*name).to_owned())
        .collect();
    // collections/requirements.yml, the Galaxy convention.
    for name in ["requirements.yml", "requirements.yaml"] {
        let path = format!("collections/{name}");
        if reading.listing.file_exists(&path) {
            annotations.push(path);
        }
    }
    annotations
}

/// Reads `folder` (`AnsibleProjectReader.Read(string)`): selects its files through `binding` as the folder body does,
/// reads them, and reads what it selected. A folder that is not there reads as an empty project, not an error.
pub fn read_folder(folder: &std::path::Path, binding: &FblBinding) -> AnsibleProject {
    let listing = AnsibleFolderListing::new(folder);
    if !listing.folder_exists("") {
        return AnsibleProject {
            folder_path: listing.root().to_string_lossy().into_owned(),
            ..AnsibleProject::default()
        };
    }
    let max = adp_fbl::FblOptions::default().max_body_bytes;
    let files: Vec<PluginFile> = adp_fbl::routing::folder_files(binding, listing.root(), None)
        .into_iter()
        .filter_map(|file| {
            let length = std::fs::metadata(&file.full_path).ok()?.len();
            if length > u64::try_from(max).unwrap_or(u64::MAX) {
                return Some(PluginFile {
                    is_oversized: true,
                    ..PluginFile::new(file.relative_path, Vec::new())
                });
            }
            std::fs::read(&file.full_path)
                .ok()
                .map(|bytes| PluginFile::new(file.relative_path, bytes))
        })
        .collect();
    read(&files, &listing, binding)
}
