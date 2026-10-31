//! The FBL section 11 persistence plugin of the binding `helm-chart.fbl#chart`, `net.etalii.adp.helm.chartFolder`
//! (etalii.adp spec 030, task T080).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/helm-chart/backend/EtAlii.Adp.Diagram.HelmChart/
//! HelmChartFolderPlugin.cs` and `HelmOpenEndRelations.cs`. It reads a chart folder with [`crate::reader`] from the
//! files the folder body hands it, and delivers one element per node in the tool's order (the chart, the values
//! layers, the schema, the templates, the CRDs, what `charts/` holds, the dependencies, the lock), each with the
//! attributes the definition declares and the file and line it is read at; then `Includes` and `Resolves`, a relation
//! without a target for an open end (gap 7 of `helm-chart.md`). The definition computes every id again and derives
//! `Declares`, `Overrides` and `Configures`. Every chart-owned YAML file that does not parse is one `std.unparseable`
//! at its file and the parser's line, in the tool's order. It reads and never plans: the binding is read-only.

use std::collections::HashSet;

use adp_cel::{Map, Value};
use adp_fbl::model::{FblElement, Finding, FindingSeverity, SourceLocation, codes};
use adp_fbl::plugins::{
    PersistencePlugin, PluginPlanRequest, PluginPlanResult, PluginReadRequest, PluginReadResult, PluginTemplateRequest,
};
use adp_fbl::text::Span;

use crate::model::{
    CrdsSummary, DependencyDeclaration, DependencyResolution, HelmChart, LockFile, TemplateFile, TemplateRole,
    ValuesFile, VendoredEntry, YamlFailure,
};
use crate::{reader, resolution};

/// The plugin's id, as the binding's `reader.plugin` names it.
pub const PLUGIN_ID: &str = "net.etalii.adp.helm.chartFolder";

/// The plugin.
#[derive(Debug, Clone, Copy, Default)]
pub struct HelmChartFolderPlugin;

impl PersistencePlugin for HelmChartFolderPlugin {
    fn id(&self) -> &str {
        PLUGIN_ID
    }

    fn read(&self, request: &PluginReadRequest) -> PluginReadResult {
        result(&reader::read(request))
    }

    /// Refused with the binding's sentence: the binding is read-only, so the host never asks.
    fn plan(&self, _request: &PluginPlanRequest<'_>) -> PluginPlanResult {
        PluginPlanResult::Refused(crate::definition::HelmDefinition::get().write_refusal())
    }

    /// A chart is a folder that is already there, so its plugin is never asked for a template.
    fn template(&self, _request: &PluginTemplateRequest) -> Vec<u8> {
        Vec::new()
    }

    /// Nothing beyond the file rules: a directory the reading counts is under a folder a file rule's glob could add a
    /// file to.
    fn watch(&self, _last: &PluginReadResult) -> Vec<String> {
        Vec::new()
    }
}

/// The id of a template's node.
pub fn template_id(template: &TemplateFile) -> String {
    format!("tpl:{}", template.relative_path)
}

/// The id of a dependency's node: its effective name, so two dependencies of one effective name share it.
pub fn dependency_id(dependency: &DependencyDeclaration) -> String {
    format!("dep:{}", dependency.effective_name())
}

/// The id of what `charts/` holds: an archive or an unpacked subchart.
pub fn vendored_id(entry: &VendoredEntry) -> String {
    format!("{}:{}", if entry.sealed { "tgz" } else { "sub" }, entry.relative_path)
}

/// What a read delivers for a chart already read: its elements and a finding per chart-owned YAML file that did not
/// parse.
pub fn result(chart: &HelmChart) -> PluginReadResult {
    PluginReadResult::new(elements(chart), findings(chart), false)
}

fn node(id: String, type_name: &str, attributes: Map, file: Option<&str>, line: u32) -> FblElement {
    FblElement {
        id,
        id_is_stored: false,
        type_name: type_name.to_owned(),
        rule: type_name.to_owned(),
        is_relation: false,
        attributes,
        parent_id: None,
        parent_slot: None,
        source: None,
        target: None,
        referenced: Vec::new(),
        own_span: Span::default(),
        line: line as usize,
        file: file.map(str::to_owned),
    }
}

/// The nodes in the tool's order, then the two relations that may have an open end.
fn elements(chart: &HelmChart) -> Vec<FblElement> {
    let resolution = resolution::matched(&chart.dependencies, &chart.vendored);
    let mut elements = vec![node(
        "chart".into(),
        "Chart",
        chart_attributes(chart),
        Some("Chart.yaml"),
        chart.metadata.as_ref().map_or(0, |m| m.line),
    )];
    for values in &chart.values {
        elements.push(node(
            format!("values:{}", values.relative_path),
            "Values",
            values_attributes(values),
            Some(&values.relative_path),
            0,
        ));
    }
    if let Some(schema) = &chart.schema {
        elements.push(node(
            format!("schema:{}", schema.relative_path),
            "Schema",
            path(&schema.relative_path),
            Some(&schema.relative_path),
            0,
        ));
    }
    for template in &chart.templates {
        let type_name = if template.role == TemplateRole::Partial {
            "Partial"
        } else {
            "Template"
        };
        elements.push(node(
            template_id(template),
            type_name,
            template_attributes(template),
            Some(&template.relative_path),
            0,
        ));
    }
    if let Some(crds) = &chart.crds {
        elements.push(node("crds".into(), "Crds", crds_attributes(crds), None, 0));
    }
    // What is actually vendored, whether or not anything declares it: the content is drawn.
    for entry in &chart.vendored {
        elements.push(if entry.sealed {
            node(
                vendored_id(entry),
                "Archive",
                vendored_attributes(entry),
                Some(&entry.relative_path),
                0,
            )
        } else {
            node(vendored_id(entry), "Subchart", vendored_attributes(entry), None, 0)
        });
    }
    // A dependency is read at its entry in the file that declares it: requirements.yaml for a legacy chart.
    let declaring = if chart.legacy {
        "requirements.yaml"
    } else {
        "Chart.yaml"
    };
    for resolved in &resolution.dependencies {
        elements.push(node(
            dependency_id(&resolved.dependency),
            "Dependency",
            dependency_attributes(&resolved.dependency),
            Some(declaring),
            resolved.dependency.line,
        ));
    }
    if let Some(lock) = &chart.lock {
        elements.push(node(
            format!("lock:{}", lock.relative_path),
            "Lock",
            lock_attributes(lock),
            Some(&lock.relative_path),
            0,
        ));
    }
    for relation in open_end_relations(chart, &resolution) {
        let mut attributes = Map::default();
        attributes.insert("label", Value::String(relation.label));
        elements.push(FblElement {
            id: relation.id,
            id_is_stored: false,
            type_name: relation.type_name.to_owned(),
            rule: relation.type_name.to_owned(),
            is_relation: true,
            attributes,
            parent_id: None,
            parent_slot: None,
            source: Some(relation.source_id),
            target: relation.target_id,
            referenced: Vec::new(),
            own_span: Span::default(),
            line: 0,
            file: None,
        });
    }
    elements
}

/// The chart-owned YAML that did not parse, one `std.unparseable` per file at the parser's line, in the tool's order:
/// `Chart.yaml`, `requirements.yaml`, the values files, the lock, the CRDs.
fn findings(chart: &HelmChart) -> Vec<Finding> {
    let failures = chart
        .metadata_failure
        .iter()
        .chain(&chart.dependencies_failure)
        .chain(chart.values.iter().filter_map(|v| v.failure.as_ref()))
        .chain(chart.lock.iter().filter_map(|l| l.failure.as_ref()))
        .chain(chart.crds.iter().flat_map(|c| &c.failures));
    failures
        .map(|failure| Finding {
            code: codes::UNPARSEABLE.into(),
            severity: FindingSeverity::Error,
            message: format!("'{}' is not readable YAML: {}", failure.relative_path, failure.message),
            location: Some(SourceLocation {
                file: failure.relative_path.clone(),
                line: failure.line as usize,
                column: usize::from(failure.line != 0),
                length: 0,
            }),
            detail: None,
        })
        .collect()
}

fn text(s: &str) -> Value {
    Value::String(s.to_owned())
}

fn int(n: usize) -> Value {
    Value::Int(i64::try_from(n).unwrap_or(i64::MAX))
}

fn strings(values: &[String]) -> Value {
    Value::list(values.iter().map(|v| text(v)).collect())
}

fn path(relative: &str) -> Map {
    let mut map = Map::default();
    map.insert("path", text(relative));
    map
}

/// Stores what was read and leaves unset what was not, so the attribute reads as its default.
fn set(map: &mut Map, name: &str, value: Option<Value>) {
    if let Some(value) = value {
        map.insert(name, value);
    }
}

/// A file that did not parse, as the definition's `YamlFailure`.
fn failure(failure: Option<&YamlFailure>) -> Option<Value> {
    failure.map(|f| {
        let mut map = Map::default();
        map.insert("path", text(&f.relative_path));
        map.insert("message", text(&f.message));
        map.insert("line", Value::Int(i64::from(f.line)));
        Value::map(map)
    })
}

fn chart_attributes(chart: &HelmChart) -> Map {
    let mut attributes = path("Chart.yaml");
    if let Some(metadata) = &chart.metadata {
        set(&mut attributes, "name", metadata.name.as_deref().map(text));
        set(&mut attributes, "version", metadata.version.as_deref().map(text));
        attributes.insert("versionLine", Value::Int(i64::from(metadata.version_line)));
        set(&mut attributes, "appVersion", metadata.app_version.as_deref().map(text));
        set(&mut attributes, "apiVersion", metadata.api_version.as_deref().map(text));
        attributes.insert("chartType", text(&metadata.chart_type));
        attributes.insert("description", text(&metadata.description));
        attributes.insert("deprecated", Value::Bool(metadata.deprecated));
    }
    set(&mut attributes, "failure", failure(chart.metadata_failure.as_ref()));
    attributes.insert("templateCount", int(chart.templates.len()));
    attributes.insert("dependencyCount", int(chart.dependencies.len()));
    attributes.insert("valuesLayerCount", int(chart.values.len()));
    attributes
}

fn values_attributes(values: &ValuesFile) -> Map {
    let mut attributes = path(&values.relative_path);
    attributes.insert("isDefault", Value::Bool(values.is_default));
    attributes.insert("keys", strings(&values.top_level_keys));
    set(&mut attributes, "failure", failure(values.failure.as_ref()));
    attributes
}

fn template_attributes(template: &TemplateFile) -> Map {
    let mut attributes = path(&template.relative_path);
    attributes.insert("kinds", strings(&template.facts.kinds));
    attributes.insert(
        "undetermined",
        Value::Bool(template.role == TemplateRole::Manifest && template.facts.kinds.is_empty()),
    );
    attributes.insert("defines", strings(&template.facts.defines));
    attributes.insert("references", strings(&template.facts.references));
    attributes
}

fn crds_attributes(crds: &CrdsSummary) -> Map {
    let mut attributes = path("crds");
    attributes.insert("files", int(crds.file_count));
    attributes.insert(
        "failures",
        Value::list(
            crds.failures
                .iter()
                .map(|f| failure(Some(f)).unwrap_or(Value::Null))
                .collect(),
        ),
    );
    attributes
}

fn dependency_attributes(dependency: &DependencyDeclaration) -> Map {
    let mut attributes = Map::default();
    attributes.insert("name", text(&dependency.name));
    set(&mut attributes, "alias", dependency.alias.as_deref().map(text));
    set(
        &mut attributes,
        "constraint",
        dependency.version_constraint.as_deref().map(text),
    );
    set(
        &mut attributes,
        "repository",
        dependency.repository.as_deref().map(text),
    );
    set(&mut attributes, "condition", dependency.condition.as_deref().map(text));
    attributes.insert("conditionState", text(dependency.state.key()));
    attributes
}

fn vendored_attributes(entry: &VendoredEntry) -> Map {
    let mut attributes = path(&entry.relative_path);
    attributes.insert("entryName", text(&entry.entry_name));
    set(&mut attributes, "chartName", entry.chart_name.as_deref().map(text));
    set(
        &mut attributes,
        "chartVersion",
        entry.chart_version.as_deref().map(text),
    );
    attributes.insert("chartType", text(&entry.chart_type));
    set(&mut attributes, "failure", failure(entry.failure.as_ref()));
    if !entry.sealed {
        attributes.insert("templates", int(entry.template_count));
        attributes.insert("deeper", int(entry.deeper_count));
    }
    attributes
}

fn lock_attributes(lock: &LockFile) -> Map {
    let mut attributes = path(&lock.relative_path);
    let entries = lock
        .entries
        .iter()
        .map(|entry| {
            let mut map = Map::default();
            map.insert("name", text(&entry.name));
            map.insert("version", text(entry.version.as_deref().unwrap_or_default()));
            map.insert("line", Value::Int(i64::from(entry.line)));
            Value::map(map)
        })
        .collect();
    attributes.insert("entries", Value::list(entries));
    set(&mut attributes, "failure", failure(lock.failure.as_ref()));
    attributes
}

/// One `Resolves` or `Includes` relation as the plugin delivers it (`HelmOpenEndRelation`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenEndRelation {
    pub id: String,
    pub type_name: &'static str,
    pub source_id: String,
    /// None for an open end.
    pub target_id: Option<String>,
    pub label: String,
}

fn relation(type_name: &'static str, source_id: String, target_id: Option<String>, label: String) -> OpenEndRelation {
    OpenEndRelation {
        id: format!(
            "edge:{source_id}|{type_name}|{}|{label}",
            target_id.as_deref().unwrap_or_default()
        ),
        type_name,
        source_id,
        target_id,
        label,
    }
}

/// The two relations of a chart that may have an open end (`HelmOpenEndRelations.Of`): every include first, in
/// template order, to the first partial that defines its name, then every dependency's `Resolves`, in dependency order.
/// One relation per id: a repeat is not delivered.
pub fn open_end_relations(chart: &HelmChart, resolution: &DependencyResolution) -> Vec<OpenEndRelation> {
    let partials: Vec<&TemplateFile> = chart
        .templates
        .iter()
        .filter(|t| t.role == TemplateRole::Partial)
        .collect();
    let mut relations = Vec::new();
    for template in &chart.templates {
        for reference in &template.facts.references {
            let defining = partials.iter().find(|p| p.facts.defines.contains(reference));
            relations.push(relation(
                "Includes",
                template_id(template),
                defining.map(|p| template_id(p)),
                reference.clone(),
            ));
        }
    }
    for resolved in &resolution.dependencies {
        let dependency = &resolved.dependency;
        relations.push(match &resolved.vendored {
            Some(vendored) => relation(
                "Resolves",
                dependency_id(dependency),
                Some(vendored_id(vendored)),
                String::new(),
            ),
            None => relation(
                "Resolves",
                dependency_id(dependency),
                None,
                dependency.effective_name().to_owned(),
            ),
        });
    }
    let mut delivered = HashSet::new();
    relations.retain(|r| delivered.insert(r.id.clone()));
    relations
}

/// The definition's model of a chart already read, as a reading through the binding builds it.
pub fn model_of(chart: &HelmChart) -> adp_disl::DislModel {
    let result = result(chart);
    crate::definition::HelmDefinition::get().model(&adp_fbl::FblModel {
        elements: result.elements,
        findings: result.findings,
        unreadable: result.unreadable,
        ..adp_fbl::FblModel::default()
    })
}
