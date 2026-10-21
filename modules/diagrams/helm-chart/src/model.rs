//! One chart folder as read: everything the diagram draws comes from here (etalii.adp spec 030, task T080).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/helm-chart/backend/EtAlii.Adp.Diagram.HelmChart/
//! _Model`. Every path is relative to the chart folder; the findings are rebased onto the project when they are
//! reported.

/// Why a chart-owned YAML file did not parse (`TolerantYamlFailure`): the file, the parser's words and its 1-based line,
/// 0 when it gave none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlFailure {
    pub relative_path: String,
    pub message: String,
    pub line: u32,
}

/// What `Chart.yaml` says about the chart itself (`ChartMetadata`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChartMetadata {
    pub name: Option<String>,
    pub version: Option<String>,
    pub app_version: Option<String>,
    pub api_version: Option<String>,
    /// `application` (the default) or `library`.
    pub chart_type: String,
    /// Empty when it has none.
    pub description: String,
    pub deprecated: bool,
    /// The 1-based line the root mapping starts on.
    pub line: u32,
    /// The 1-based line of the version value, 0 when there is none.
    pub version_line: u32,
}

/// One file of the values stack (`ValuesFile`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValuesFile {
    pub relative_path: String,
    /// True only for `values.yaml` (or `values.yml`) itself.
    pub is_default: bool,
    pub has_global: bool,
    /// The top-level keys, in ordinal order.
    pub top_level_keys: Vec<String>,
    pub failure: Option<YamlFailure>,
}

/// The `values.schema.json` beside the values stack (`SchemaFile`): represented, never evaluated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaFile {
    pub relative_path: String,
}

/// What a file under `templates/` is to Helm (`TemplateRole`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateRole {
    Manifest,
    Partial,
    Notes,
    Test,
}

/// What [`crate::template_scan`] found literally written in one template (`TemplateFacts`): every list ordinal and
/// distinct.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TemplateFacts {
    pub kinds: Vec<String>,
    pub api_versions: Vec<String>,
    pub defines: Vec<String>,
    pub references: Vec<String>,
}

/// One file under `templates/`, scanned but never parsed as YAML (`TemplateFile`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateFile {
    pub relative_path: String,
    pub role: TemplateRole,
    pub facts: TemplateFacts,
}

/// The `crds/` folder, summarised (`CrdsSummary`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrdsSummary {
    /// The YAML files in it.
    pub file_count: usize,
    /// The ones that did not parse, in ordinal order of path.
    pub failures: Vec<YamlFailure>,
}

/// A dependency's condition, resolved against the default values at read time (`ConditionState`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionState {
    /// The dependency declares no condition.
    None,
    /// The path exists and reads true.
    On,
    /// The path exists and reads false.
    Off,
    /// The path is not in the default values.
    Missing,
    /// The path exists but is not a boolean, or the values did not parse.
    Unknown,
}

impl ConditionState {
    /// The definition's `ConditionState` value.
    pub fn key(self) -> &'static str {
        match self {
            ConditionState::None => "none",
            ConditionState::On => "on",
            ConditionState::Off => "off",
            ConditionState::Missing => "missing",
            ConditionState::Unknown => "unknown",
        }
    }
}

/// One declared dependency, from `Chart.yaml`, or `requirements.yaml` for a legacy chart (`DependencyDeclaration`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyDeclaration {
    pub name: String,
    pub alias: Option<String>,
    /// The declared version or range, verbatim; never evaluated.
    pub version_constraint: Option<String>,
    pub repository: Option<String>,
    pub condition: Option<String>,
    pub state: ConditionState,
    /// The 1-based line the entry starts on.
    pub line: u32,
}

impl DependencyDeclaration {
    /// The alias when it has one, else the name: what `charts/` content and values keys are matched against.
    pub fn effective_name(&self) -> &str {
        match self.alias.as_deref() {
            Some(alias) if !alias.is_empty() => alias,
            _ => &self.name,
        }
    }
}

/// One thing in `charts/`: an unpacked chart directory, read one level deep, or a sealed `.tgz` (`VendoredEntry`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VendoredEntry {
    /// The directory's name, or the archive's without its version-and-extension tail.
    pub entry_name: String,
    pub relative_path: String,
    pub sealed: bool,
    pub chart_name: Option<String>,
    pub chart_version: Option<String>,
    pub chart_type: String,
    pub template_count: usize,
    pub deeper_count: usize,
    pub failure: Option<YamlFailure>,
}

/// One pinned dependency of the lock (`LockEntry`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockEntry {
    pub name: String,
    pub version: Option<String>,
    pub line: u32,
}

/// `Chart.lock`, or `requirements.lock` for a legacy chart (`LockFile`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockFile {
    pub relative_path: String,
    /// In ordinal order of name.
    pub entries: Vec<LockEntry>,
    pub failure: Option<YamlFailure>,
}

/// One chart folder, read tolerantly (`HelmChart`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelmChart {
    pub metadata: Option<ChartMetadata>,
    pub metadata_failure: Option<YamlFailure>,
    /// `apiVersion: v1`: the dependencies are in `requirements.yaml`.
    pub legacy: bool,
    /// The default layer first, then the overrides in ordinal order.
    pub values: Vec<ValuesFile>,
    pub schema: Option<SchemaFile>,
    pub templates: Vec<TemplateFile>,
    pub crds: Option<CrdsSummary>,
    /// In ordinal order of effective name.
    pub dependencies: Vec<DependencyDeclaration>,
    pub vendored: Vec<VendoredEntry>,
    pub lock: Option<LockFile>,
    /// Why a legacy chart's `requirements.yaml` did not parse.
    pub dependencies_failure: Option<YamlFailure>,
}

/// One declared dependency and the `charts/` entry that answers it, if any (`ResolvedDependency`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedDependency {
    pub dependency: DependencyDeclaration,
    pub vendored: Option<VendoredEntry>,
}

impl ResolvedDependency {
    pub fn is_resolved(&self) -> bool {
        self.vendored.is_some()
    }
}

/// What [`crate::resolution::matched`] made of a chart's dependencies (`DependencyResolutionResult`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyResolution {
    /// Every declaration with its resolution, in declaration-list order.
    pub dependencies: Vec<ResolvedDependency>,
    /// Vendored entries no declaration claims.
    pub undeclared: Vec<VendoredEntry>,
}
