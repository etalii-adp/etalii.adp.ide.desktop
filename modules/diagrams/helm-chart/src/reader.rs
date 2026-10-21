//! Reads one chart folder into a [`HelmChart`], tolerantly, from what the folder body hands the plugin, without
//! opening a file or writing a byte (etalii.adp spec 030, task T080).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/helm-chart/backend/EtAlii.Adp.Diagram.HelmChart/
//! HelmChartReader.cs` and `HelmYaml.cs`. Chart-owned YAML goes through [`crate::yaml`]; `templates/` content goes
//! through [`crate::template_scan`] and is never parsed as YAML. Every lookup the standalone reader made on the disk is
//! a lookup in what it is handed: the selected files, by relative path and bytes, and the directories
//! [`crate::entries`] adds, each a path ending in `/`. A name is looked up as the file system would have found it,
//! ignoring case where the platform does; the matches the reader always made ignoring case (`values.yaml`,
//! `NOTES.txt`, the YAML extensions) still do on every platform. A file handed without bytes costs that file's detail
//! and nothing else.

use adp_fbl::plugins::{PluginFile, PluginReadRequest};
use adp_fbl::routing::platform_ignores_case;

use crate::model::{
    ChartMetadata, ConditionState, CrdsSummary, DependencyDeclaration, HelmChart, LockEntry, LockFile, SchemaFile,
    TemplateFacts, TemplateFile, TemplateRole, ValuesFile, VendoredEntry, YamlFailure,
};
use crate::template_scan;
use crate::yaml::{self, YamlNode};

/// What one YAML file read to: its first document's root (none for an empty file), or why it did not parse.
enum Parsed {
    Document(Option<YamlNode>),
    Unreadable(YamlFailure),
}

/// One tolerant parse of one handed file (`HelmYaml.Read`), naming it `relative_path` in anything it reports.
fn parse(file: &PluginFile, relative_path: &str) -> Parsed {
    if file.is_oversized {
        return Parsed::Unreadable(YamlFailure {
            relative_path: relative_path.to_owned(),
            message: "The file is larger than this host reads, so it is not read.".into(),
            line: 0,
        });
    }
    match yaml::parse(&text_of(file)) {
        Ok(root) => Parsed::Document(root),
        Err(error) => Parsed::Unreadable(YamlFailure {
            relative_path: relative_path.to_owned(),
            message: error.message,
            line: u32::try_from(error.line).unwrap_or(0),
        }),
    }
}

/// The file's text, decoded as the shared reader decodes a file on disk: UTF-8, a byte order mark taken off.
fn text_of(file: &PluginFile) -> String {
    let text = String::from_utf8_lossy(&file.bytes);
    text.strip_prefix('\u{feff}').unwrap_or(&text).to_owned()
}

/// Whether two names are one, as the platform's file system compares them.
fn same_name(a: &str, b: &str) -> bool {
    if platform_ignores_case() {
        a.eq_ignore_ascii_case(b)
    } else {
        a == b
    }
}

fn starts_with_name(text: &str, prefix: &str) -> bool {
    text.len() >= prefix.len() && text.is_char_boundary(prefix.len()) && same_name(&text[..prefix.len()], prefix)
}

fn ends_with_name(text: &str, suffix: &str) -> bool {
    text.len() >= suffix.len()
        && text.is_char_boundary(text.len() - suffix.len())
        && same_name(&text[text.len() - suffix.len()..], suffix)
}

fn file_name(relative: &str) -> &str {
    relative.rsplit_once('/').map_or(relative, |(_, name)| name)
}

fn parent_of(relative: &str) -> &str {
    relative.rsplit_once('/').map_or("", |(parent, _)| parent)
}

/// The extension of `name` with its dot, as `Path.GetExtension` gives it; empty for none.
fn extension_of(name: &str) -> &str {
    let name = file_name(name);
    name.rfind('.').map_or("", |dot| &name[dot..])
}

fn is_yaml(relative: &str) -> bool {
    let extension = extension_of(relative);
    extension.eq_ignore_ascii_case(".yaml") || extension.eq_ignore_ascii_case(".yml")
}

/// What the plugin is handed, looked up as the file system would have answered the reader (`Handed`).
struct Handed<'a> {
    files: &'a [PluginFile],
    directories: Vec<String>,
}

impl<'a> Handed<'a> {
    fn new(request: &'a PluginReadRequest) -> Self {
        Handed {
            files: &request.files,
            directories: request
                .supplementary_entries
                .iter()
                .filter(|e| e.ends_with('/'))
                .map(|e| e.trim_end_matches('/').to_owned())
                .collect(),
        }
    }

    /// The handed file at `relative`.
    fn file(&self, relative: &str) -> Option<&'a PluginFile> {
        self.files.iter().find(|f| same_name(&f.relative_path, relative))
    }

    /// The handed files directly in `folder` (the chart folder for empty) whose name `accept` takes.
    fn files_in(&self, folder: &str, accept: impl Fn(&str) -> bool) -> Vec<&'a PluginFile> {
        self.files
            .iter()
            .filter(|f| same_name(parent_of(&f.relative_path), folder) && accept(file_name(&f.relative_path)))
            .collect()
    }

    /// The handed files anywhere under `folder`.
    fn files_under(&self, folder: &str) -> Vec<&'a PluginFile> {
        let prefix = format!("{folder}/");
        self.files
            .iter()
            .filter(|f| starts_with_name(&f.relative_path, &prefix))
            .collect()
    }

    /// The directories directly in `folder`, as relative paths, in ordinal order: those a directory entry names and
    /// those a handed file lies in.
    fn directories(&self, folder: &str) -> Vec<String> {
        let prefix = format!("{folder}/");
        let holding = self.files.iter().filter_map(|f| {
            let path = &f.relative_path;
            if !starts_with_name(path, &prefix) {
                return None;
            }
            let slash = path[prefix.len()..].find('/')? + prefix.len();
            (slash > prefix.len()).then(|| path[..slash].to_owned())
        });
        let mut distinct: Vec<String> = Vec::new();
        for directory in self
            .directories
            .iter()
            .filter(|d| same_name(parent_of(d), folder))
            .cloned()
            .chain(holding)
        {
            if !distinct.iter().any(|d| same_name(d, &directory)) {
                distinct.push(directory);
            }
        }
        distinct.sort();
        distinct
    }

    /// Whether `folder` exists: a directory entry names it, or a file under it was handed.
    fn folder_exists(&self, folder: &str) -> bool {
        self.directories.iter().any(|d| same_name(d, folder)) || !self.files_under(folder).is_empty()
    }
}

/// The 1-based line a node starts on.
fn line(node: &YamlNode) -> u32 {
    u32::try_from(node.start().line).unwrap_or(0)
}

/// The scalar under `key` of a mapping; none where there is none or it is not a scalar.
fn scalar<'n>(mapping: &'n YamlNode, key: &str) -> Option<&'n YamlNode> {
    mapping.get(key).filter(|node| matches!(node, YamlNode::Scalar { .. }))
}

fn scalar_value(mapping: &YamlNode, key: &str) -> Option<String> {
    scalar(mapping, key).and_then(YamlNode::as_scalar).map(str::to_owned)
}

/// `bool.TryParse`: `true` or `false` in any case, with whitespace around it.
fn parse_bool(text: &str) -> Option<bool> {
    let trimmed = text.trim();
    if trimmed.eq_ignore_ascii_case("true") {
        Some(true)
    } else if trimmed.eq_ignore_ascii_case("false") {
        Some(false)
    } else {
        None
    }
}

/// Reads the chart the folder body hands in `request`.
pub fn read(request: &PluginReadRequest) -> HelmChart {
    let handed = Handed::new(request);

    let mut metadata = None;
    let mut metadata_failure = None;
    let mut chart_root: Option<YamlNode> = None;
    if let Some(chart_yaml) = handed.file("Chart.yaml") {
        match parse(chart_yaml, "Chart.yaml") {
            Parsed::Document(Some(root @ YamlNode::Mapping { .. })) => {
                metadata = Some(read_metadata(&root));
                chart_root = Some(root);
            }
            // Parsed but empty, or not a mapping: no metadata to show, nothing to blame a line on.
            Parsed::Document(_) => {}
            Parsed::Unreadable(failure) => metadata_failure = Some(failure),
        }
    }

    let legacy = metadata
        .as_ref()
        .and_then(|m: &ChartMetadata| m.api_version.as_deref())
        .is_some_and(|v| v.eq_ignore_ascii_case("v1"));

    let (values, default_values) = read_values(&handed);
    let (dependencies, dependencies_failure) =
        read_dependencies(&handed, chart_root.as_ref(), legacy, default_values.as_ref());

    HelmChart {
        metadata,
        metadata_failure,
        legacy,
        values,
        schema: handed.file("values.schema.json").map(|_| SchemaFile {
            relative_path: "values.schema.json".into(),
        }),
        templates: read_templates(&handed),
        crds: read_crds(&handed),
        dependencies,
        vendored: read_vendored(&handed),
        lock: read_lock(&handed, legacy),
        dependencies_failure,
    }
}

fn read_metadata(root: &YamlNode) -> ChartMetadata {
    let version = scalar(root, "version");
    ChartMetadata {
        name: scalar_value(root, "name"),
        version: version.and_then(YamlNode::as_scalar).map(str::to_owned),
        app_version: scalar_value(root, "appVersion"),
        api_version: scalar_value(root, "apiVersion"),
        chart_type: scalar_value(root, "type")
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| "application".into()),
        description: scalar_value(root, "description").unwrap_or_default(),
        deprecated: scalar_value(root, "deprecated")
            .and_then(|d| parse_bool(&d))
            .unwrap_or(false),
        line: line(root),
        version_line: version.map_or(0, line),
    }
}

fn read_values(handed: &Handed<'_>) -> (Vec<ValuesFile>, Option<YamlNode>) {
    let mut result = Vec::new();
    let mut default_root = None;
    for file in handed.files_in("", |name| starts_with_name(name, "values")) {
        let name = file.relative_path.as_str();
        // values.schema.json matches the prefix but is not a values layer.
        if !is_yaml(name) {
            continue;
        }
        let is_default = name.eq_ignore_ascii_case("values.yaml") || name.eq_ignore_ascii_case("values.yml");
        match parse(file, name) {
            Parsed::Document(Some(root @ YamlNode::Mapping { .. })) => {
                let YamlNode::Mapping { entries, .. } = &root else {
                    unreachable!("matched as a mapping");
                };
                let mut keys: Vec<String> = entries
                    .iter()
                    .filter_map(|(key, _)| key.as_scalar())
                    .filter(|key| !key.is_empty())
                    .map(str::to_owned)
                    .collect();
                keys.sort();
                result.push(ValuesFile {
                    relative_path: name.to_owned(),
                    is_default,
                    has_global: keys.iter().any(|k| k == "global"),
                    top_level_keys: keys,
                    failure: None,
                });
                if is_default {
                    default_root = Some(root);
                }
            }
            // Empty, or not a mapping: a values layer with nothing in it yet.
            Parsed::Document(_) => result.push(ValuesFile {
                relative_path: name.to_owned(),
                is_default,
                has_global: false,
                top_level_keys: Vec::new(),
                failure: None,
            }),
            Parsed::Unreadable(failure) => result.push(ValuesFile {
                relative_path: name.to_owned(),
                is_default,
                has_global: false,
                top_level_keys: Vec::new(),
                failure: Some(failure),
            }),
        }
    }
    // The default layer first, then the overrides ordinally: the stack as it is drawn.
    result.sort_by(|a, b| {
        (!a.is_default)
            .cmp(&!b.is_default)
            .then_with(|| a.relative_path.cmp(&b.relative_path))
    });
    (result, default_root)
}

fn read_templates(handed: &Handed<'_>) -> Vec<TemplateFile> {
    let mut result: Vec<TemplateFile> = handed
        .files_under("templates")
        .into_iter()
        .map(|file| {
            let relative = file.relative_path.as_str();
            let name = file_name(relative);
            let role = if name.starts_with('_') {
                TemplateRole::Partial
            } else if name.eq_ignore_ascii_case("NOTES.txt") {
                TemplateRole::Notes
            } else if relative.len() >= "templates/tests/".len()
                && relative.is_char_boundary("templates/tests/".len())
                && relative[.."templates/tests/".len()].eq_ignore_ascii_case("templates/tests/")
            {
                TemplateRole::Test
            } else {
                TemplateRole::Manifest
            };
            TemplateFile {
                relative_path: relative.to_owned(),
                role,
                // A template that cannot be read yields no facts, and no error.
                facts: if file.is_oversized {
                    TemplateFacts::default()
                } else {
                    template_scan::scan(&text_of(file))
                },
            }
        })
        .collect();
    result.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    result
}

fn read_crds(handed: &Handed<'_>) -> Option<CrdsSummary> {
    if !handed.folder_exists("crds") {
        return None;
    }
    let mut count = 0;
    let mut failures = Vec::new();
    for file in handed.files_under("crds") {
        if !is_yaml(&file.relative_path) {
            continue;
        }
        count += 1;
        if let Parsed::Unreadable(failure) = parse(file, &file.relative_path) {
            failures.push(failure);
        }
    }
    failures.sort_by(|a: &YamlFailure, b| a.relative_path.cmp(&b.relative_path));
    Some(CrdsSummary {
        file_count: count,
        failures,
    })
}

fn read_dependencies(
    handed: &Handed<'_>,
    chart_root: Option<&YamlNode>,
    legacy: bool,
    default_values: Option<&YamlNode>,
) -> (Vec<DependencyDeclaration>, Option<YamlFailure>) {
    let requirements;
    let list = if legacy {
        // Helm 2: the dependencies live in their own file beside Chart.yaml, whose own parse failure is reported.
        match handed.file("requirements.yaml").map(|f| parse(f, "requirements.yaml")) {
            Some(Parsed::Document(Some(root @ YamlNode::Mapping { .. }))) => {
                requirements = root;
                requirements.get("dependencies")
            }
            Some(Parsed::Unreadable(failure)) => return (Vec::new(), Some(failure)),
            _ => None,
        }
    } else {
        chart_root.and_then(|root| root.get("dependencies"))
    };
    let Some(YamlNode::Sequence { items, .. }) = list else {
        return (Vec::new(), None);
    };
    let mut result = Vec::new();
    for entry in items.iter().filter(|i| matches!(i, YamlNode::Mapping { .. })) {
        // A nameless entry declares nothing recognisable.
        let Some(name) = scalar_value(entry, "name").filter(|n| !n.is_empty()) else {
            continue;
        };
        let condition = scalar_value(entry, "condition");
        result.push(DependencyDeclaration {
            name,
            alias: scalar_value(entry, "alias"),
            version_constraint: scalar_value(entry, "version"),
            repository: scalar_value(entry, "repository"),
            state: resolve(condition.as_deref(), default_values),
            condition,
            line: line(entry),
        });
    }
    // A stable sort, as LINQ's OrderBy is.
    result.sort_by(|a, b| a.effective_name().cmp(b.effective_name()));
    (result, None)
}

/// Walks a dotted condition path into the default values.
fn resolve(condition: Option<&str>, default_values: Option<&YamlNode>) -> ConditionState {
    let Some(condition) = condition.filter(|c| !c.is_empty()) else {
        return ConditionState::None;
    };
    // No parsed default values to resolve against: the truth is unknowable, which is not the path being absent.
    let Some(mut current) = default_values else {
        return ConditionState::Unknown;
    };
    for segment in condition.split('.') {
        match current {
            YamlNode::Mapping { .. } => match current.get(segment) {
                Some(next) => current = next,
                None => return ConditionState::Missing,
            },
            _ => return ConditionState::Missing,
        }
    }
    match current.as_scalar().and_then(parse_bool) {
        Some(true) => ConditionState::On,
        Some(false) => ConditionState::Off,
        None => ConditionState::Unknown,
    }
}

fn read_vendored(handed: &Handed<'_>) -> Vec<VendoredEntry> {
    let mut result: Vec<VendoredEntry> = handed
        .directories("charts")
        .iter()
        .map(|directory| read_unpacked(handed, directory))
        .collect();
    for file in handed.files_in("charts", |name| ends_with_name(name, ".tgz")) {
        result.push(VendoredEntry {
            entry_name: strip_version_tail(file_name(&file.relative_path)),
            relative_path: file.relative_path.clone(),
            sealed: true,
            chart_name: None,
            chart_version: None,
            chart_type: "application".into(),
            template_count: 0,
            deeper_count: 0,
            failure: None,
        });
    }
    result.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    result
}

/// One level deep: the nested chart summarised, never recursed further.
fn read_unpacked(handed: &Handed<'_>, relative: &str) -> VendoredEntry {
    let entry = |chart_name, chart_version, chart_type: String, failure| VendoredEntry {
        entry_name: file_name(relative).to_owned(),
        relative_path: relative.to_owned(),
        sealed: false,
        chart_name,
        chart_version,
        chart_type,
        template_count: handed.files_under(&format!("{relative}/templates")).len(),
        deeper_count: handed.files_in(&format!("{relative}/charts"), |_| true).len()
            + handed.directories(&format!("{relative}/charts")).len(),
        failure,
    };
    let Some(nested) = handed.file(&format!("{relative}/Chart.yaml")) else {
        return entry(None, None, "application".into(), None);
    };
    match parse(nested, &nested.relative_path) {
        Parsed::Document(Some(root @ YamlNode::Mapping { .. })) => {
            let metadata = read_metadata(&root);
            entry(metadata.name, metadata.version, metadata.chart_type, None)
        }
        Parsed::Unreadable(failure) => entry(None, None, "application".into(), Some(failure)),
        Parsed::Document(_) => entry(None, None, "application".into(), None),
    }
}

/// The archive's name without its version-and-extension tail: `common-2.31.4.tgz` matches `common`.
pub fn strip_version_tail(file_name: &str) -> String {
    let stem = &file_name[..file_name.len().saturating_sub(".tgz".len())];
    match stem.rfind('-') {
        Some(dash) if dash > 0 && stem.as_bytes().get(dash + 1).is_some_and(u8::is_ascii_digit) => {
            stem[..dash].to_owned()
        }
        _ => stem.to_owned(),
    }
}

fn read_lock(handed: &Handed<'_>, legacy: bool) -> Option<LockFile> {
    let name = if legacy { "requirements.lock" } else { "Chart.lock" };
    let file = handed.file(name)?;
    let lock = |entries, failure| LockFile {
        relative_path: name.to_owned(),
        entries,
        failure,
    };
    Some(match parse(file, name) {
        Parsed::Unreadable(failure) => lock(Vec::new(), Some(failure)),
        Parsed::Document(Some(root @ YamlNode::Mapping { .. })) => match root.get("dependencies") {
            Some(YamlNode::Sequence { items, .. }) => {
                let mut entries: Vec<LockEntry> = items
                    .iter()
                    .filter(|i| matches!(i, YamlNode::Mapping { .. }))
                    .filter_map(|entry| {
                        let name = scalar_value(entry, "name").filter(|n| !n.is_empty())?;
                        Some(LockEntry {
                            name,
                            version: scalar_value(entry, "version"),
                            line: line(entry),
                        })
                    })
                    .collect();
                entries.sort_by(|a, b| a.name.cmp(&b.name));
                lock(entries, None)
            }
            _ => lock(Vec::new(), None),
        },
        Parsed::Document(_) => lock(Vec::new(), None),
    })
}
