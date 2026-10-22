//! The three readings of a Databricks file: as a bundle, as the jobs it declares and as the pipelines it declares,
//! each recording which lines declare what (etalii.adp spec 030, task T081).
//!
//! Ported from the standalone host's `src/diagrams/databricks/backend/EtAlii.Adp.Diagram.Databricks/BundleParser.cs`,
//! `JobParser.cs`, `PipelineParser.cs` and `_Model/DatabricksModel.cs` at `8b6ccfc`. Forgiving on purpose: a resource
//! of a kind it does not know, a root key it does not model, a task without a key, a dependency naming nothing; none
//! of these stop a reading, and the problems are the rules' to report. Only a text that is not YAML fails, and it is
//! read as the empty model with the reader's sentence.

use crate::lines::LineDocument;
use crate::model::{
    BundleInclude, BundleModel, BundleResource, BundleTarget, BundleVariable, DatabricksModel, JobCluster,
    JobDependency, JobModel, JobTask, Outcome, PipelineLibrary, PipelineModel, PipelineNotification, UnknownNode,
};
use crate::yaml::{self, YamlNode, entry_range, range_of};

const MODELLED_ROOT_KEYS: [&str; 5] = ["bundle", "include", "variables", "resources", "targets"];
const MODELLED_RESOURCE_KINDS: [&str; 2] = ["jobs", "pipelines"];
const MODELLED_PIPELINE_KEYS: [&str; 10] = [
    "name",
    "catalog",
    "schema",
    "target",
    "serverless",
    "continuous",
    "development",
    "channel",
    "libraries",
    "notifications",
];

/// The `*_task` mappings that decide a task's type, and the field within each that is its principal source.
const TASK_KINDS: [(&str, &str, &str); 10] = [
    ("notebook_task", "notebook", "notebook_path"),
    ("spark_python_task", "python", "python_file"),
    ("python_wheel_task", "wheel", "entry_point"),
    ("sql_task", "sql", ""),
    ("dbt_task", "dbt", "project_directory"),
    ("pipeline_task", "pipeline", "pipeline_id"),
    ("run_job_task", "run-job", "job_id"),
    ("condition_task", "condition", "op"),
    ("for_each_task", "for-each", "inputs"),
    ("spark_jar_task", "jar", "main_class_name"),
];

fn text(value: Option<&str>) -> String {
    value.unwrap_or_default().to_owned()
}

/// The three readings of `document`, or the reader's line and sentence when it is not YAML (`DatabricksModel.Read`).
pub fn read(document: &LineDocument) -> Result<DatabricksModel, (usize, String)> {
    let root = yaml::parse(&document.text()).map_err(|error| (error.line.max(1) - 1, error.message))?;
    let root = root.filter(YamlNode::is_mapping);
    Ok(DatabricksModel {
        bundle: parse_bundle(root.as_ref(), document),
        jobs: parse_jobs(root.as_ref(), document),
        pipelines: parse_pipelines(root.as_ref(), document),
    })
}

/// What the store and the rules keep of `document` (`DatabricksModel.Outcome`): its readings, or for a text that is
/// not YAML the empty model, the reader's sentence on the line it names, and that sentence as why nothing is written.
pub fn outcome(document: &LineDocument) -> Outcome {
    match read(document) {
        Ok(model) => Outcome {
            model,
            problems: Vec::new(),
            unreadable: None,
        },
        Err((line, message)) => Outcome {
            model: DatabricksModel::default(),
            problems: vec![(line, message.clone())],
            unreadable: Some(message),
        },
    }
}

// ---- the bundle ----------------------------------------------------------------------------------------------------

fn parse_bundle(root: Option<&YamlNode>, document: &LineDocument) -> BundleModel {
    let Some(root) = root else {
        return BundleModel::default();
    };
    let name = root
        .mapping("bundle")
        .map(|bundle| text(bundle.scalar("name")))
        .unwrap_or_default();
    let (resources, unknown_kinds) = read_resources(root, document);
    let mut unknown_nodes = read_unknown_root_keys(root, document);
    unknown_nodes.extend(unknown_kinds);
    BundleModel {
        name,
        includes: root
            .sequence("include")
            .iter()
            .filter_map(|node| {
                node.as_scalar().map(|glob| BundleInclude {
                    glob: glob.to_owned(),
                    lines: range_of(node, document),
                })
            })
            .collect(),
        variables: read_variables(root, document),
        resources,
        targets: read_targets(root, document),
        unknown_nodes,
    }
}

fn read_variables(root: &YamlNode, document: &LineDocument) -> Vec<BundleVariable> {
    let Some(mapping) = root.mapping("variables") else {
        return Vec::new();
    };
    mapping
        .entries()
        .iter()
        .filter_map(|(key, value)| {
            let name = key.as_scalar()?;
            // A variable may be declared bare (`name:`) or with a mapping of default and description.
            let declaration = value.is_mapping().then_some(value);
            Some(BundleVariable {
                name: name.to_owned(),
                default: text(declaration.and_then(|d| d.scalar("default"))),
                description: text(declaration.and_then(|d| d.scalar("description"))),
                lines: entry_range(key, value, document),
            })
        })
        .collect()
}

fn keyed_resources(kind: &str, entries: &YamlNode, document: &LineDocument) -> Vec<BundleResource> {
    entries
        .entries()
        .iter()
        .filter_map(|(key_node, body)| {
            key_node.as_scalar().map(|key| BundleResource {
                kind: kind.to_owned(),
                key: key.to_owned(),
                lines: entry_range(key_node, body, document),
            })
        })
        .collect()
}

fn read_resources(root: &YamlNode, document: &LineDocument) -> (Vec<BundleResource>, Vec<UnknownNode>) {
    let mut resources = Vec::new();
    let mut unknown_kinds = Vec::new();
    let Some(mapping) = root.mapping("resources") else {
        return (resources, unknown_kinds);
    };
    for (kind_node, entries) in mapping.entries() {
        let Some(kind) = kind_node.as_scalar() else { continue };
        if !MODELLED_RESOURCE_KINDS.contains(&kind) {
            // An experiment, a model, a schema: real configuration this module does not model.
            unknown_kinds.push(UnknownNode {
                path: format!("resources.{kind}"),
                key: kind.to_owned(),
                lines: entry_range(kind_node, entries, document),
            });
            continue;
        }
        if entries.is_mapping() {
            resources.extend(keyed_resources(kind, entries, document));
        }
    }
    (resources, unknown_kinds)
}

fn read_targets(root: &YamlNode, document: &LineDocument) -> Vec<BundleTarget> {
    let Some(mapping) = root.mapping("targets") else {
        return Vec::new();
    };
    mapping
        .entries()
        .iter()
        .filter_map(|(name_node, body)| {
            let name = name_node.as_scalar()?;
            if !body.is_mapping() {
                return None;
            }
            Some(BundleTarget {
                name: name.to_owned(),
                mode: text(body.scalar("mode")),
                is_default: body.flag("default"),
                overrides: read_overrides(body, document),
                lines: entry_range(name_node, body, document),
            })
        })
        .collect()
}

/// The resources a target overrides: everything keyed under its own `resources:`, as kind/key pairs.
fn read_overrides(target: &YamlNode, document: &LineDocument) -> Vec<BundleResource> {
    let Some(mapping) = target.mapping("resources") else {
        return Vec::new();
    };
    mapping
        .entries()
        .iter()
        .filter_map(|(kind_node, entries)| {
            let kind = kind_node.as_scalar()?;
            entries.is_mapping().then(|| keyed_resources(kind, entries, document))
        })
        .flatten()
        .collect()
}

fn read_unknown_root_keys(root: &YamlNode, document: &LineDocument) -> Vec<UnknownNode> {
    root.entries()
        .iter()
        .filter_map(|(key_node, value)| {
            let key = key_node.as_scalar()?;
            (!MODELLED_ROOT_KEYS.contains(&key)).then(|| UnknownNode {
                path: key.to_owned(),
                key: key.to_owned(),
                lines: entry_range(key_node, value, document),
            })
        })
        .collect()
}

// ---- the jobs ------------------------------------------------------------------------------------------------------

fn parse_jobs(root: Option<&YamlNode>, document: &LineDocument) -> Vec<JobModel> {
    let Some(keyed) = root
        .and_then(|r| r.mapping("resources"))
        .and_then(|r| r.mapping("jobs"))
    else {
        return Vec::new();
    };
    keyed
        .entries()
        .iter()
        .filter_map(|(key_node, job)| {
            let key = key_node.as_scalar()?;
            if !job.is_mapping() {
                return None;
            }
            Some(JobModel {
                key: key.to_owned(),
                name: text(job.scalar("name")),
                tasks: read_tasks(job, document),
                clusters: read_clusters(job, document),
                schedule: job
                    .mapping("schedule")
                    .map(|s| text(s.scalar("quartz_cron_expression")))
                    .unwrap_or_default(),
                continuous: job.contains_key("continuous"),
                lines: entry_range(key_node, job, document),
            })
        })
        .collect()
}

fn read_tasks(job: &YamlNode, document: &LineDocument) -> Vec<JobTask> {
    job.sequence("tasks")
        .iter()
        .filter(|node| node.is_mapping())
        .map(|task| {
            let (task_type, source) = kind_of(task);
            JobTask {
                key: text(task.scalar("task_key")),
                task_type: task_type.to_owned(),
                source,
                cluster_key: text(task.scalar("job_cluster_key")),
                run_if: text(task.scalar("run_if")),
                depends_on: read_dependencies(task, document),
                lines: range_of(task, document),
            }
        })
        .collect()
}

/// A task's type is whichever `*_task` mapping it carries; one carrying none is `other`, which still draws.
fn kind_of(task: &YamlNode) -> (&'static str, String) {
    for (key, task_type, source_field) in TASK_KINDS {
        let Some(declaration) = task.mapping(key) else { continue };
        let source = if task_type == "sql" {
            // The SQL task's source sits one level deeper, and which key is there depends on what it runs.
            sql_source(declaration)
        } else if source_field.is_empty() {
            String::new()
        } else {
            text(declaration.scalar(source_field))
        };
        return (task_type, source);
    }
    ("other", String::new())
}

fn sql_source(declaration: &YamlNode) -> String {
    for kind in ["query", "dashboard", "alert", "file"] {
        if let Some(inner) = declaration.mapping(kind) {
            return text(inner.scalar(&format!("{kind}_id")).or_else(|| inner.scalar("path")));
        }
    }
    String::new()
}

fn read_dependencies(task: &YamlNode, document: &LineDocument) -> Vec<JobDependency> {
    task.sequence("depends_on")
        .iter()
        .filter(|node| node.is_mapping())
        .filter_map(|dependency| {
            let upstream = dependency.scalar("task_key").filter(|k| !k.is_empty())?;
            Some(JobDependency {
                task_key: upstream.to_owned(),
                outcome: text(dependency.scalar("outcome")),
                lines: range_of(dependency, document),
            })
        })
        .collect()
}

/// `int.TryParse` in the invariant culture: surrounding white space and a sign allowed.
fn int_of(value: Option<&str>) -> i32 {
    value.and_then(|v| v.trim().parse().ok()).unwrap_or(0)
}

fn read_clusters(job: &YamlNode, document: &LineDocument) -> Vec<JobCluster> {
    job.sequence("job_clusters")
        .iter()
        .filter(|node| node.is_mapping())
        .filter_map(|cluster| {
            let key = cluster.scalar("job_cluster_key").filter(|k| !k.is_empty())?;
            let spec = cluster.mapping("new_cluster");
            Some(JobCluster {
                key: key.to_owned(),
                spark_version: text(spec.and_then(|s| s.scalar("spark_version"))),
                node_type: text(spec.and_then(|s| s.scalar("node_type_id"))),
                workers: int_of(spec.and_then(|s| s.scalar("num_workers"))),
                lines: range_of(cluster, document),
            })
        })
        .collect()
}

// ---- the pipelines -------------------------------------------------------------------------------------------------

fn parse_pipelines(root: Option<&YamlNode>, document: &LineDocument) -> Vec<PipelineModel> {
    let Some(root) = root else {
        return Vec::new();
    };
    if let Some(keyed) = root.mapping("resources").and_then(|r| r.mapping("pipelines")) {
        return keyed
            .entries()
            .iter()
            .filter_map(|(key_node, body)| {
                let key = key_node.as_scalar()?;
                body.is_mapping()
                    .then(|| read_pipeline(key, body, entry_range(key_node, body, document), document))
            })
            .collect();
    }
    if root.scalar("name").is_some_and(|n| !n.is_empty()) || root.contains_key("libraries") {
        // A bare settings file: the root mapping is the pipeline.
        return vec![read_pipeline("", root, range_of(root, document), document)];
    }
    Vec::new()
}

fn read_pipeline(
    key: &str,
    pipeline: &YamlNode,
    lines: crate::lines::LineRange,
    document: &LineDocument,
) -> PipelineModel {
    PipelineModel {
        key: key.to_owned(),
        name: text(pipeline.scalar("name")),
        catalog: text(pipeline.scalar("catalog")),
        // `schema` succeeded `target` for the same idea; a file says one or the other.
        schema: text(pipeline.scalar("schema").or_else(|| pipeline.scalar("target"))),
        serverless: pipeline.flag("serverless"),
        continuous: pipeline.flag("continuous"),
        development: pipeline.flag("development"),
        channel: text(pipeline.scalar("channel")),
        libraries: read_libraries(pipeline, document),
        notifications: read_notifications(pipeline, document),
        unknown_nodes: pipeline
            .entries()
            .iter()
            .filter_map(|(key_node, value)| {
                let key = key_node.as_scalar()?;
                (!MODELLED_PIPELINE_KEYS.contains(&key)).then(|| UnknownNode {
                    path: key.to_owned(),
                    key: key.to_owned(),
                    lines: entry_range(key_node, value, document),
                })
            })
            .collect(),
        lines,
    }
}

fn read_libraries(pipeline: &YamlNode, document: &LineDocument) -> Vec<PipelineLibrary> {
    pipeline
        .sequence("libraries")
        .iter()
        .filter(|node| node.is_mapping())
        .map(|entry| {
            // A library entry is a single-key mapping naming its kind; the path field inside depends on the kind.
            let (kind, path) = match entry.entries() {
                [(kind_node, inner)] if kind_node.as_scalar().is_some() && inner.is_mapping() => (
                    kind_node.as_scalar().unwrap_or_default(),
                    text(inner.scalar("path").or_else(|| inner.scalar("include"))),
                ),
                _ => ("other", String::new()),
            };
            let modelled = matches!(kind, "notebook" | "file" | "glob");
            PipelineLibrary {
                kind: if modelled { kind } else { "other" }.to_owned(),
                path,
                lines: range_of(entry, document),
            }
        })
        .collect()
}

fn strings(mapping: &YamlNode, key: &str) -> Vec<String> {
    mapping
        .sequence(key)
        .iter()
        .filter_map(|node| node.as_scalar().map(str::to_owned))
        .collect()
}

fn read_notifications(pipeline: &YamlNode, document: &LineDocument) -> Vec<PipelineNotification> {
    pipeline
        .sequence("notifications")
        .iter()
        .filter(|node| node.is_mapping())
        .map(|entry| PipelineNotification {
            recipients: strings(entry, "email_recipients"),
            alerts: strings(entry, "alerts"),
            lines: range_of(entry, document),
        })
        .collect()
}
