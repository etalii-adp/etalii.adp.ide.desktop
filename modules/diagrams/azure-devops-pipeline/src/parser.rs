//! Reads a pipeline's lines into a [`PipelineModel`], recording for every element the lines that declare it
//! (etalii.adp spec 030, task T078).
//!
//! Ported from the standalone host's `src/diagrams/azure-devops-pipeline/backend/EtAlii.Adp.Diagram.AzureDevOpsPipeline/
//! PipelineParser.cs` at `8b6ccfc`. The line ranges are the index the writer splices through. Expressions are never
//! evaluated: a `${{ if }}` block is unwrapped so what it holds still appears, the expression kept as its gate. A
//! file without `stages` gets the implicit stage the schema wraps a `jobs`-only or `steps`-only file in; templates are
//! followed where [`PipelineTemplates`] lets them be, and every reference is recorded whether or not it was.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::lines::{LineDocument, LineRange};
use crate::model::{
    PipelineExecution, PipelineJob, PipelineModel, PipelinePool, PipelineStage, PipelineStep, PipelineStrategy,
    PoolOrigin, StepKind, StrategyKind, TemplateReference, TemplateSlot, UnresolvedReason, UnresolvedTemplate,
};
use crate::templates::PipelineTemplates;
use crate::yaml::{self, YamlNode};

/// The id of the implicit stage.
pub const IMPLICIT_STAGE_ID: &str = "stage-0";
/// The id of the implicit job.
pub const IMPLICIT_JOB_ID: &str = "stage-0/job-0";

/// One thing that kept a text from being read, on its 0-based line (`ParseProblem`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseProblem {
    pub line: usize,
    pub message: String,
}

/// What the store and the validator keep of a document (`ParseOutcome<PipelineModel>`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParseOutcome {
    pub model: PipelineModel,
    pub problems: Vec<ParseProblem>,
    /// Why nothing may be written to it, or none when it read.
    pub unreadable: Option<String>,
}

impl ParseOutcome {
    pub fn is_usable(&self) -> bool {
        self.unreadable.is_none()
    }
}

/// Reads `document` without following any template it names.
pub fn parse(document: &LineDocument) -> Result<PipelineModel, yaml::YamlError> {
    Parser::new(document, None, PathBuf::new(), String::new(), HashSet::new(), 0).read()
}

/// Reads `document`, at `document_path`, following the templates `resolver` lets it reach.
pub fn parse_with(
    document: &LineDocument,
    resolver: &PipelineTemplates,
    document_path: &Path,
) -> Result<PipelineModel, yaml::YamlError> {
    Parser::new(
        document,
        Some(resolver),
        document_path.to_path_buf(),
        String::new(),
        HashSet::new(),
        0,
    )
    .read()
}

/// What is kept of `document`: its model, or, for a text that is not YAML this can read, the empty model, the
/// reader's sentence on the line it names, and that line and sentence as the reason nothing is written to it. Never
/// fails.
pub fn outcome(document: &LineDocument, resolver: Option<&PipelineTemplates>, document_path: &Path) -> ParseOutcome {
    let read = Parser::new(
        document,
        resolver,
        document_path.to_path_buf(),
        String::new(),
        HashSet::new(),
        0,
    )
    .read();
    match read {
        Ok(model) => ParseOutcome {
            model,
            problems: Vec::new(),
            unreadable: None,
        },
        Err(error) => ParseOutcome {
            model: PipelineModel::default(),
            problems: vec![ParseProblem {
                line: error.line.max(1) - 1,
                message: error.message.clone(),
            }],
            unreadable: Some(format!("Line {}: {}", error.line, error.message)),
        },
    }
}

struct Parser<'a> {
    document: &'a LineDocument,
    resolver: Option<&'a PipelineTemplates>,
    document_path: PathBuf,
    source: String,
    expanding: HashSet<PathBuf>,
    first_template: usize,
    templates: Vec<TemplateReference>,
    unresolved: Vec<UnresolvedTemplate>,
    pipeline_pool: PipelinePool,
}

/// The entries of a mapping in written order, with any merge key (`<<`) expanded after the mapping's own.
fn entries(mapping: &YamlNode) -> Vec<(String, &YamlNode)> {
    let YamlNode::Mapping { entries: own, .. } = mapping else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut merged: Option<Vec<(String, &YamlNode)>> = None;
    for (key, value) in own {
        let Some(name) = key.as_scalar() else {
            continue;
        };
        if name != "<<" {
            out.push((name.to_owned(), value));
            continue;
        }
        let merged = merged.get_or_insert_with(Vec::new);
        let sources: Vec<&YamlNode> = match value {
            YamlNode::Sequence { items, .. } => items.iter().collect(),
            other => vec![other],
        };
        for source in sources {
            if matches!(source, YamlNode::Mapping { .. }) {
                merged.extend(entries(source));
            }
        }
    }
    if let Some(merged) = merged {
        let names: HashSet<&str> = own.iter().filter_map(|(k, _)| k.as_scalar()).collect();
        out.extend(merged.into_iter().filter(|(k, _)| !names.contains(k.as_str())));
    }
    out
}

fn entry<'n>(mapping: &'n YamlNode, key: &str) -> Option<&'n YamlNode> {
    entries(mapping).into_iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// The scalar under `key`, or empty.
fn scalar(mapping: &YamlNode, key: &str) -> String {
    match entry(mapping, key) {
        Some(YamlNode::Scalar { value, .. }) => value.clone(),
        _ => String::new(),
    }
}

fn is_mapping(node: &YamlNode) -> bool {
    matches!(node, YamlNode::Mapping { .. })
}

fn sequence_of(node: Option<&YamlNode>) -> Option<&YamlNode> {
    node.filter(|n| matches!(n, YamlNode::Sequence { .. }))
}

fn mapping_of(node: Option<&YamlNode>) -> Option<&YamlNode> {
    node.filter(|n| is_mapping(n))
}

fn items(sequence: &YamlNode) -> &[YamlNode] {
    match sequence {
        YamlNode::Sequence { items, .. } => items,
        _ => &[],
    }
}

/// A sequence item that is nothing but a `${{ ... }}` key over a list: its expression and the list.
fn gate(mapping: &YamlNode) -> Option<(String, &YamlNode)> {
    let all = entries(mapping);
    match all.as_slice() {
        [(key, value @ YamlNode::Sequence { .. })] if key.starts_with("${{") => Some((key.clone(), value)),
        _ => None,
    }
}

fn combine(outer: &str, inner: &str) -> String {
    if outer.is_empty() {
        inner.to_owned()
    } else {
        format!("{outer} and {inner}")
    }
}

/// The properties deciding whether and how an element runs, read as text.
fn read_execution(element: &YamlNode) -> PipelineExecution {
    let declared: std::collections::BTreeSet<String> = PipelineExecution::KEYS
        .iter()
        .filter(|key| entry(element, key).is_some())
        .map(|key| (*key).to_owned())
        .collect();
    if declared.is_empty() {
        return PipelineExecution::default();
    }
    PipelineExecution {
        condition: scalar(element, PipelineExecution::CONDITION_KEY),
        continue_on_error: scalar(element, PipelineExecution::CONTINUE_ON_ERROR_KEY),
        enabled: scalar(element, PipelineExecution::ENABLED_KEY),
        timeout_in_minutes: scalar(element, PipelineExecution::TIMEOUT_KEY),
        declared,
    }
}

/// `dependsOn` as one name or a list, and whether the key was there at all.
fn read_depends_on(element: &YamlNode) -> (Vec<String>, bool) {
    match entry(element, "dependsOn") {
        None => (Vec::new(), false),
        Some(YamlNode::Scalar { value, .. }) if !value.is_empty() => (vec![value.clone()], true),
        Some(YamlNode::Sequence { items, .. }) => (
            items
                .iter()
                .filter_map(YamlNode::as_scalar)
                .filter(|v| !v.is_empty())
                .map(str::to_owned)
                .collect(),
            true,
        ),
        Some(_) => (Vec::new(), true),
    }
}

/// A step's kind is the first key it leads with that names one.
fn read_step_kind(step: &YamlNode) -> (StepKind, String) {
    for (key, value) in entries(step) {
        if let Some(kind) = StepKind::of_key(&key) {
            return (kind, value.as_scalar().unwrap_or_default().to_owned());
        }
    }
    (StepKind::Unknown, String::new())
}

/// How many copies of a job a strategy produces, or the expression that decides it.
fn read_multiplicity(kind: StrategyKind, value: &YamlNode) -> (i32, String) {
    match (kind, value) {
        (StrategyKind::Matrix, YamlNode::Mapping { entries, .. }) => (entries.len().max(1) as i32, String::new()),
        (StrategyKind::Matrix, YamlNode::Scalar { value, .. }) if !value.is_empty() => (1, value.clone()),
        (StrategyKind::Parallel, YamlNode::Scalar { value, .. }) if !value.is_empty() => match value.parse::<i32>() {
            Ok(parsed) => (parsed.max(1), String::new()),
            Err(_) => (1, value.clone()),
        },
        _ => (1, String::new()),
    }
}

fn inherit(own: PipelinePool, inherited: &PipelinePool) -> PipelinePool {
    if own.is_declared() { own } else { inherited.clone() }
}

impl<'a> Parser<'a> {
    fn new(
        document: &'a LineDocument,
        resolver: Option<&'a PipelineTemplates>,
        document_path: PathBuf,
        source: String,
        expanding: HashSet<PathBuf>,
        first_template: usize,
    ) -> Self {
        Parser {
            document,
            resolver,
            document_path,
            source,
            expanding,
            first_template,
            templates: Vec::new(),
            unresolved: Vec::new(),
            pipeline_pool: PipelinePool::default(),
        }
    }

    fn range(&self, node: &YamlNode) -> LineRange {
        yaml::range_of(node, self.document)
    }

    fn read(mut self) -> Result<PipelineModel, yaml::YamlError> {
        let Some(root) = yaml::parse(&self.document.text())? else {
            return Ok(PipelineModel::default());
        };
        if !is_mapping(&root) {
            return Ok(PipelineModel::default());
        }
        let extends = self.read_extends(&root);
        let stages = match &extends {
            None => self.read_pipeline_stages(&root),
            Some(extends) => self.follow(extends).map(|m| m.stages).unwrap_or_default(),
        };
        Ok(PipelineModel {
            stages,
            templates: self.templates,
            extends,
            unresolved: self.unresolved,
        })
    }

    fn read_extends(&self, root: &YamlNode) -> Option<TemplateReference> {
        let extends = mapping_of(entry(root, "extends"))?;
        let path = scalar(extends, "template");
        Some(self.template_reference(
            "extends".to_owned(),
            String::new(),
            TemplateSlot::Extends,
            &path,
            extends,
        ))
    }

    fn read_pipeline_stages(&mut self, root: &YamlNode) -> Vec<PipelineStage> {
        self.pipeline_pool = self.read_pool(root, PoolOrigin::Pipeline);
        if let Some(stages) = sequence_of(entry(root, "stages")) {
            let mut read = Vec::new();
            self.read_stages(stages, "", &mut read);
            return read;
        }
        if let Some(jobs) = sequence_of(entry(root, "jobs")) {
            let pool = self.pipeline_pool.clone();
            let read = self.read_jobs_into(jobs, IMPLICIT_STAGE_ID, &pool);
            return vec![self.implicit_stage(read, self.range(jobs), pool)];
        }
        if let Some(steps) = sequence_of(entry(root, "steps")) {
            let pool = self.pipeline_pool.clone();
            let read = self.read_steps(steps, IMPLICIT_JOB_ID, "", 0);
            let lines = self.range(steps);
            let job = PipelineJob {
                id: IMPLICIT_JOB_ID.to_owned(),
                name: String::new(),
                display_name: String::new(),
                is_deployment: false,
                environment: String::new(),
                strategy: PipelineStrategy::default(),
                pool: pool.clone(),
                execution: PipelineExecution::default(),
                depends_on: Vec::new(),
                depends_on_declared: false,
                gate: String::new(),
                is_implicit: true,
                steps: read,
                template: self.source.clone(),
                lines,
            };
            return vec![self.implicit_stage(vec![job], lines, pool)];
        }
        Vec::new()
    }

    fn implicit_stage(&self, jobs: Vec<PipelineJob>, lines: LineRange, pool: PipelinePool) -> PipelineStage {
        PipelineStage {
            id: IMPLICIT_STAGE_ID.to_owned(),
            name: String::new(),
            display_name: String::new(),
            pool,
            execution: PipelineExecution::default(),
            trigger_is_manual: false,
            is_skippable: String::new(),
            depends_on: Vec::new(),
            depends_on_declared: false,
            gate: String::new(),
            is_implicit: true,
            jobs,
            template: self.source.clone(),
            lines,
        }
    }

    fn read_stages(&mut self, sequence: &YamlNode, gate_expression: &str, into: &mut Vec<PipelineStage>) {
        for item in items(sequence) {
            if !is_mapping(item) {
                continue;
            }
            if let Some((expression, block)) = gate(item) {
                self.read_stages(block, &combine(gate_expression, &expression), into);
                continue;
            }
            let template = scalar(item, "template");
            if !template.is_empty() {
                let id = self.next_template_id();
                let reference = self.template_reference(id, String::new(), TemplateSlot::Stages, &template, item);
                self.templates.push(reference.clone());
                if let Some(expanded) = self.follow(&reference) {
                    into.extend(expanded.stages);
                }
                continue;
            }
            let name = scalar(item, "stage");
            let id = if name.is_empty() {
                format!("stage-{}", into.len())
            } else {
                name.clone()
            };
            let (depends_on, declared) = read_depends_on(item);
            let own = self.read_pool(item, PoolOrigin::Stage);
            let pool = inherit(own, &self.pipeline_pool);
            let jobs = match sequence_of(entry(item, "jobs")) {
                Some(jobs) => self.read_jobs_into(jobs, &id, &pool),
                None => Vec::new(),
            };
            into.push(PipelineStage {
                id,
                name,
                display_name: scalar(item, "displayName"),
                pool,
                execution: read_execution(item),
                trigger_is_manual: scalar(item, "trigger").eq_ignore_ascii_case("manual"),
                is_skippable: scalar(item, "isSkippable"),
                depends_on,
                depends_on_declared: declared,
                gate: gate_expression.to_owned(),
                is_implicit: false,
                jobs,
                template: self.source.clone(),
                lines: self.range(item),
            });
        }
    }

    fn read_jobs_into(&mut self, sequence: &YamlNode, stage_id: &str, inherited: &PipelinePool) -> Vec<PipelineJob> {
        let mut jobs = Vec::new();
        self.read_jobs(sequence, stage_id, "", inherited, &mut jobs);
        jobs
    }

    fn read_jobs(
        &mut self,
        sequence: &YamlNode,
        stage_id: &str,
        gate_expression: &str,
        inherited: &PipelinePool,
        into: &mut Vec<PipelineJob>,
    ) {
        for item in items(sequence) {
            if !is_mapping(item) {
                continue;
            }
            if let Some((expression, block)) = gate(item) {
                self.read_jobs(block, stage_id, &combine(gate_expression, &expression), inherited, into);
                continue;
            }
            let template = scalar(item, "template");
            if !template.is_empty() {
                let id = self.next_template_id();
                let reference = self.template_reference(id, stage_id.to_owned(), TemplateSlot::Jobs, &template, item);
                self.templates.push(reference.clone());
                if let Some(expanded) = self.follow(&reference) {
                    // A jobs template's jobs arrive in its own implicit stage; here they belong to this stage.
                    for job in expanded.stages.into_iter().flat_map(|s| s.jobs) {
                        let index = into.len();
                        into.push(reparent(job, stage_id, index));
                    }
                }
                continue;
            }
            let deployment = scalar(item, "deployment");
            let is_deployment = !deployment.is_empty();
            let name = if is_deployment { deployment } else { scalar(item, "job") };
            let id = if name.is_empty() {
                format!("{stage_id}/job-{}", into.len())
            } else {
                format!("{stage_id}/{name}")
            };
            let strategy = self.read_strategy(item);
            let (depends_on, declared) = read_depends_on(item);
            let own = self.read_pool(item, PoolOrigin::Job);
            let steps = self.read_job_steps(item, &id, &strategy);
            into.push(PipelineJob {
                id,
                name,
                display_name: scalar(item, "displayName"),
                is_deployment,
                environment: scalar(item, "environment"),
                strategy,
                pool: inherit(own, inherited),
                execution: read_execution(item),
                depends_on,
                depends_on_declared: declared,
                gate: gate_expression.to_owned(),
                is_implicit: false,
                steps,
                template: self.source.clone(),
                lines: self.range(item),
            });
        }
    }

    /// A plain job's steps, or a deployment job's, gathered per lifecycle hook of its strategy.
    fn read_job_steps(&mut self, job: &YamlNode, job_id: &str, strategy: &PipelineStrategy) -> Vec<PipelineStep> {
        if let Some(direct) = sequence_of(entry(job, "steps")) {
            return self.read_steps(direct, job_id, "", 0);
        }
        if strategy.kind == StrategyKind::None {
            return Vec::new();
        }
        let Some(strategy_node) = mapping_of(entry(job, "strategy")) else {
            return Vec::new();
        };
        let Some(hooks) = mapping_of(entry(strategy_node, strategy.kind.key())) else {
            return Vec::new();
        };
        let mut steps = Vec::new();
        for (key, value) in entries(hooks) {
            if is_mapping(value)
                && let Some(hook_steps) = sequence_of(entry(value, "steps"))
            {
                let offset = steps.len();
                let read = self.read_steps(hook_steps, job_id, &key, offset);
                steps.extend(read);
            }
        }
        steps
    }

    fn read_steps(&mut self, sequence: &YamlNode, job_id: &str, hook: &str, offset: usize) -> Vec<PipelineStep> {
        let mut steps: Vec<PipelineStep> = Vec::new();
        for item in items(sequence) {
            if !is_mapping(item) {
                continue;
            }
            let (kind, identifier) = read_step_kind(item);
            steps.push(PipelineStep {
                id: format!("{job_id}/step-{}", offset + steps.len()),
                kind,
                display_name: scalar(item, "displayName"),
                identifier: identifier.clone(),
                execution: read_execution(item),
                hook: hook.to_owned(),
                template: self.source.clone(),
                lines: self.range(item),
            });
            if kind != StepKind::Template {
                continue;
            }
            let id = self.next_template_id();
            let reference = self.template_reference(id, job_id.to_owned(), TemplateSlot::Steps, &identifier, item);
            self.templates.push(reference.clone());
            let Some(expanded) = self.follow(&reference) else {
                continue;
            };
            for contributed in expanded.stages.into_iter().flat_map(|s| s.jobs).flat_map(|j| j.steps) {
                let id = format!("{job_id}/step-{}", offset + steps.len());
                steps.push(PipelineStep {
                    id,
                    hook: hook.to_owned(),
                    ..contributed
                });
            }
        }
        steps
    }

    fn read_strategy(&self, job: &YamlNode) -> PipelineStrategy {
        let Some(strategy) = mapping_of(entry(job, "strategy")) else {
            return PipelineStrategy::default();
        };
        for (key, value) in entries(strategy) {
            let Some(kind) = StrategyKind::of_key(&key) else {
                continue;
            };
            let (multiplicity, expression) = read_multiplicity(kind, value);
            return PipelineStrategy {
                kind,
                multiplicity,
                multiplicity_expression: expression,
                lines: self.range(strategy),
            };
        }
        PipelineStrategy::default()
    }

    /// A `pool` as a bare name or as a mapping of `name`, `vmImage` and `demands`.
    fn read_pool(&self, element: &YamlNode, origin: PoolOrigin) -> PipelinePool {
        match entry(element, "pool") {
            Some(node @ YamlNode::Scalar { value, .. }) if !value.is_empty() => PipelinePool {
                name: value.clone(),
                vm_image: String::new(),
                demands: Vec::new(),
                origin,
                lines: self.range(node),
            },
            Some(pool @ YamlNode::Mapping { .. }) => PipelinePool {
                name: scalar(pool, "name"),
                vm_image: scalar(pool, "vmImage"),
                demands: match sequence_of(entry(pool, "demands")) {
                    Some(demands) => items(demands)
                        .iter()
                        .filter_map(YamlNode::as_scalar)
                        .map(str::to_owned)
                        .collect(),
                    None => Vec::new(),
                },
                origin,
                lines: self.range(pool),
            },
            _ => PipelinePool::default(),
        }
    }

    /// Reads the file a reference names, or records why it was not read.
    fn follow(&mut self, reference: &TemplateReference) -> Option<PipelineModel> {
        let resolver = self.resolver?;
        let path = match resolver.locate(reference, &self.document_path) {
            Ok(path) => path,
            Err(reason) => {
                self.unresolved.push(UnresolvedTemplate {
                    reference: reference.clone(),
                    reason,
                });
                return None;
            }
        };
        if !self.expanding.insert(path.clone()) {
            self.unresolved.push(UnresolvedTemplate {
                reference: reference.clone(),
                reason: UnresolvedReason::Cyclic,
            });
            return None;
        }
        let result = match resolver.read(&path) {
            None => {
                self.unresolved.push(UnresolvedTemplate {
                    reference: reference.clone(),
                    reason: UnresolvedReason::Unreadable,
                });
                None
            }
            Some(document) => {
                let inner = Parser::new(
                    &document,
                    Some(resolver),
                    path.clone(),
                    resolver.display(&path),
                    self.expanding.clone(),
                    self.first_template + self.templates.len(),
                )
                .read();
                match inner {
                    Ok(model) => {
                        self.templates.extend(model.templates.iter().cloned());
                        self.unresolved.extend(model.unresolved.iter().cloned());
                        Some(model)
                    }
                    Err(_) => {
                        self.unresolved.push(UnresolvedTemplate {
                            reference: reference.clone(),
                            reason: UnresolvedReason::Unreadable,
                        });
                        None
                    }
                }
            }
        };
        self.expanding.remove(&path);
        result
    }

    fn next_template_id(&self) -> String {
        format!("template-{}", self.first_template + self.templates.len())
    }

    fn template_reference(
        &self,
        id: String,
        owner_id: String,
        slot: TemplateSlot,
        reference: &str,
        declaration: &YamlNode,
    ) -> TemplateReference {
        let (path, resource) = match reference.rfind('@') {
            Some(at) => (reference[..at].to_owned(), reference[at + 1..].to_owned()),
            None => (reference.to_owned(), String::new()),
        };
        let parameter_names = match mapping_of(entry(declaration, "parameters")) {
            Some(supplied) => entries(supplied).into_iter().map(|(k, _)| k).collect(),
            None => Vec::new(),
        };
        TemplateReference {
            id,
            owner_id,
            slot,
            path,
            resource,
            parameter_names,
            lines: self.range(declaration),
        }
    }
}

/// A job read from a jobs template, as a job of the stage that referenced it.
fn reparent(job: PipelineJob, stage_id: &str, index: usize) -> PipelineJob {
    let id = if job.name.is_empty() {
        format!("{stage_id}/job-{index}")
    } else {
        format!("{stage_id}/{}", job.name)
    };
    let steps = job
        .steps
        .into_iter()
        .enumerate()
        .map(|(position, step)| PipelineStep {
            id: format!("{id}/step-{position}"),
            ..step
        })
        .collect();
    PipelineJob { id, steps, ..job }
}
