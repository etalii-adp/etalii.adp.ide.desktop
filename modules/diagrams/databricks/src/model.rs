//! What a Databricks configuration file says, under each of the family's three readings, with the lines that declare
//! each part (etalii.adp spec 030, task T081).
//!
//! Ported from the standalone host's `src/diagrams/databricks/backend/EtAlii.Adp.Diagram.Databricks/_Model/` at
//! `8b6ccfc`. All three readings are carried at once because one file legitimately serves several: a `databricks.yml`
//! is a bundle that may also declare jobs and pipelines inline, and a resource file holds jobs the bundle diagram
//! counts and the job diagram opens.

use crate::lines::LineRange;

/// One `include:` entry of a bundle: a glob naming resource files to pull in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleInclude {
    pub glob: String,
    pub lines: LineRange,
}

/// One declared bundle variable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleVariable {
    pub name: String,
    /// Its default value as written; empty when it has none.
    pub default: String,
    /// Its description; empty when it has none.
    pub description: String,
    pub lines: LineRange,
}

/// One resource a bundle declares: anything keyed under a kind below `resources:`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleResource {
    /// The resource kind as written: `jobs`, `pipelines`.
    pub kind: String,
    /// The resource's key under its kind.
    pub key: String,
    pub lines: LineRange,
}

/// One deployment target of a bundle, with the resources it overrides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleTarget {
    pub name: String,
    /// Its `mode:` as written, or empty.
    pub mode: String,
    /// Whether the target says `default: true`.
    pub is_default: bool,
    /// The resources this target overrides, as kind/key pairs.
    pub overrides: Vec<BundleResource>,
    pub lines: LineRange,
}

/// A construct the parser recognised as present but does not model: an unmodelled root key, an unknown resource
/// kind. It draws as a generic box, and no writer ever touches its lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownNode {
    /// Where it sits, as a dotted breadcrumb: `sync` or `resources.experiments`.
    pub path: String,
    /// The key itself, the last segment of the path.
    pub key: String,
    pub lines: LineRange,
}

/// What a `databricks.yml` says: the structure the bundle diagram draws.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BundleModel {
    /// The bundle's name; empty when the file declares none.
    pub name: String,
    pub includes: Vec<BundleInclude>,
    pub variables: Vec<BundleVariable>,
    pub resources: Vec<BundleResource>,
    pub targets: Vec<BundleTarget>,
    pub unknown_nodes: Vec<UnknownNode>,
}

/// One `job_clusters:` entry: the compute a task can bind to by key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobCluster {
    pub key: String,
    pub spark_version: String,
    pub node_type: String,
    /// The new cluster's `num_workers`; zero when unspecified.
    pub workers: i32,
    pub lines: LineRange,
}

/// One `depends_on` entry: the upstream task, and after a condition task which of its outcomes this edge follows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobDependency {
    pub task_key: String,
    /// The `outcome:` as written, or empty for an ordinary edge.
    pub outcome: String,
    pub lines: LineRange,
}

/// One task of a job: what it runs, what it waits for, and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobTask {
    pub key: String,
    /// `notebook`, `python`, `wheel`, `sql`, `dbt`, `pipeline`, `run-job`, `condition`, `for-each`, `jar`, or `other`.
    pub task_type: String,
    /// The type's principal source in display form.
    pub source: String,
    /// The `job_cluster_key`; empty means serverless.
    pub cluster_key: String,
    /// The `run_if` as written; empty means the default `ALL_SUCCESS`.
    pub run_if: String,
    pub depends_on: Vec<JobDependency>,
    pub lines: LineRange,
}

/// One Lakeflow job as a resource file declares it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobModel {
    pub key: String,
    pub name: String,
    pub tasks: Vec<JobTask>,
    pub clusters: Vec<JobCluster>,
    /// The `schedule:`'s cron expression as written; empty when there is none.
    pub schedule: String,
    pub continuous: bool,
    pub lines: LineRange,
}

/// One `libraries:` entry: a source the pipeline's transformations come from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineLibrary {
    /// `notebook`, `file`, `glob`, or `other`.
    pub kind: String,
    pub path: String,
    pub lines: LineRange,
}

/// One `notifications:` entry: who hears about which pipeline events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineNotification {
    pub recipients: Vec<String>,
    pub alerts: Vec<String>,
    pub lines: LineRange,
}

/// One Lakeflow declarative pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineModel {
    /// The pipeline's key under `resources: pipelines:`; empty for a bare settings file.
    pub key: String,
    pub name: String,
    pub catalog: String,
    /// The target schema (`schema:` or the legacy `target:`).
    pub schema: String,
    pub serverless: bool,
    pub continuous: bool,
    pub development: bool,
    pub channel: String,
    pub libraries: Vec<PipelineLibrary>,
    pub notifications: Vec<PipelineNotification>,
    pub unknown_nodes: Vec<UnknownNode>,
    pub lines: LineRange,
}

/// What one Databricks configuration file parsed to under each of the family's readings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DatabricksModel {
    pub bundle: BundleModel,
    pub jobs: Vec<JobModel>,
    pub pipelines: Vec<PipelineModel>,
}

/// What a parser read from a body (`ParseOutcome`): the model, the problem on the line it names, and for a text that
/// is not YAML why nothing is written to it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outcome {
    pub model: DatabricksModel,
    /// The 0-based line the reader stopped on, and what it said.
    pub problems: Vec<(usize, String)>,
    pub unreadable: Option<String>,
}
