//! The FBL section 11 persistence plugin of the Azure DevOps pipeline's binding, the capability
//! `reader.azurePipelinesYaml` (etalii.adp spec 035, T132): it reads a pipeline with the module's [`parser`] into the
//! binding types of the draft declared binding (`specifications/fbl/azure-devops-pipeline.fbl.draft`), every value the
//! text it was written with, and plans each model change with the module's own edits, so the bytes are the module's.
//! A stage, a job and a step carry the ids the module gives them, which the description's derived ids compute again.

use adp_cel::{Map, Value};
use adp_fbl::model::{FblElement, Finding, FindingSeverity, SourceLocation, codes};
use adp_fbl::planning::ModelChange;
use adp_fbl::plugins::{
    PersistencePlugin, PluginFile, PluginPlanRequest, PluginPlanResult, PluginReadRequest, PluginReadResult,
    PluginSplice, PluginTemplateRequest,
};
use adp_fbl::splice::{Operation, Splice};
use adp_fbl::text::Span;

use crate::commands::{self, AddKind, EditOutcome, PipelineEdit};
use crate::edits;
use crate::lines::LineDocument;
use crate::model::{
    PipelineExecution, PipelineJob, PipelineModel, PipelinePool, PipelineStage, PipelineStep, StrategyKind,
};
use crate::parser;

/// The capability's name, which the binding's reader names.
pub const CAPABILITY: &str = "reader.azurePipelinesYaml";

const BOM: char = '\u{feff}';

/// The plugin, which holds nothing of its own: every read and plan is of the bytes it is handed.
#[derive(Debug, Clone, Copy, Default)]
pub struct PipelineYamlPlugin;

fn body_text(files: &[PluginFile]) -> String {
    let bytes = files
        .iter()
        .find(|f| f.relative_path.is_empty())
        .or_else(|| files.first())
        .map(|f| f.bytes.clone())
        .unwrap_or_default();
    let text = String::from_utf8_lossy(&bytes).into_owned();
    text.strip_prefix(BOM).map(str::to_owned).unwrap_or(text)
}

impl PersistencePlugin for PipelineYamlPlugin {
    fn id(&self) -> &str {
        CAPABILITY
    }

    fn read(&self, request: &PluginReadRequest) -> PluginReadResult {
        let text = body_text(&request.files);
        let outcome = parser::outcome(&LineDocument::parse(&text), None, std::path::Path::new(""));
        if let Some(reason) = outcome.unreadable {
            let finding = Finding {
                code: codes::UNPARSEABLE.into(),
                severity: FindingSeverity::Error,
                message: reason,
                location: Some(SourceLocation {
                    file: String::new(),
                    line: 1,
                    column: 1,
                    length: 0,
                }),
                detail: None,
            };
            return PluginReadResult::new(Vec::new(), vec![finding], true);
        }
        PluginReadResult::new(elements(&outcome.model), Vec::new(), false)
    }

    fn plan(&self, request: &PluginPlanRequest<'_>) -> PluginPlanResult {
        let text = body_text(request.files);
        let mut body = LineDocument::parse(&text);
        let model = match parser::parse(&body) {
            Ok(model) => model,
            Err(_) => return PluginPlanResult::Refused("The pipeline could not be read, so it is not written.".into()),
        };
        let edits = match edits_of(&model, request.change) {
            Ok(edits) => edits,
            Err(reason) => return PluginPlanResult::Refused(reason),
        };
        for edit in edits {
            let model = match parser::parse(&body) {
                Ok(model) => model,
                Err(_) => return PluginPlanResult::Refused("The pipeline could not be read again.".into()),
            };
            if let EditOutcome::Refused(reason) = commands::apply_to(&mut body, &model, &edit) {
                return PluginPlanResult::Refused(reason);
            }
        }
        PluginPlanResult::Planned(spliced(&text, &body.text()))
    }

    fn template(&self, _request: &PluginTemplateRequest) -> Vec<u8> {
        Vec::new()
    }
}

/// The one splice that turns `before` into `after`: what lies between their common start and their common end.
fn spliced(before: &str, after: &str) -> Vec<PluginSplice> {
    if before == after {
        return Vec::new();
    }
    let (a, b) = (before.as_bytes(), after.as_bytes());
    let mut start = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    while !before.is_char_boundary(start) || !after.is_char_boundary(start) {
        start -= 1;
    }
    let mut end = 0;
    while end < a.len() - start && end < b.len() - start && a[a.len() - 1 - end] == b[b.len() - 1 - end] {
        end += 1;
    }
    while !before.is_char_boundary(a.len() - end) || !after.is_char_boundary(b.len() - end) {
        end -= 1;
    }
    vec![PluginSplice {
        file: String::new(),
        splice: Splice {
            operation: Operation::ReplaceValue,
            start,
            end: a.len() - end,
            text: after[start..b.len() - end].to_owned(),
        },
    }]
}

/// A value's text, or none for an empty one.
fn text(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::Null => None,
        Value::String(s) => Some(s.clone()),
        other => Some(other.to_string()),
    }
}

/// The module's edits that make `change`.
fn edits_of(model: &PipelineModel, change: &ModelChange) -> Result<Vec<PipelineEdit>, String> {
    match change {
        ModelChange::Save => Ok(Vec::new()),
        ModelChange::Set { id, attributes } => {
            let mut edits = Vec::new();
            for (name, value) in attributes.iter() {
                let element_id = id.clone();
                edits.push(match name.as_str() {
                    "displayName" => PipelineEdit::Rename {
                        element_id,
                        // The name as the module keeps it, without the spaces around it.
                        display_name: text(Some(value)).unwrap_or_default().trim().to_owned(),
                    },
                    "enabled" => PipelineEdit::SetEnabled {
                        element_id,
                        enabled: !text(Some(value)).is_some_and(|v| v.trim().eq_ignore_ascii_case("false")),
                    },
                    "dependsOn" => match value {
                        Value::Null => PipelineEdit::SetDependencies {
                            element_id,
                            depends_on: Vec::new(),
                            declared: false,
                        },
                        Value::List(items) => PipelineEdit::SetDependencies {
                            element_id,
                            depends_on: items.iter().filter_map(|i| text(Some(i))).collect(),
                            declared: true,
                        },
                        other => PipelineEdit::SetDependencies {
                            element_id,
                            depends_on: text(Some(other)).into_iter().collect(),
                            declared: true,
                        },
                    },
                    _ => return Err("This is edited in the pipeline file.".to_owned()),
                });
            }
            Ok(edits)
        }
        ModelChange::Add {
            type_name,
            attributes,
            parent_id,
            ..
        } => {
            let name = text(attributes.get("name")).unwrap_or_default();
            let kind = match type_name.as_str() {
                "Stage" => AddKind::Stage,
                "Job" => AddKind::Job,
                "DeploymentJob" => AddKind::DeploymentJob,
                "Step" => AddKind::Step,
                "DependsOn" => {
                    // A relation drawn: the name of its source among what its target waits for.
                    let target = text(attributes.get("target")).unwrap_or_default();
                    let source = text(attributes.get("source")).unwrap_or_default();
                    let Some(location) = edits::locate(model, &target) else {
                        return Err(commands::GONE.to_owned());
                    };
                    let mut depends_on = location.depends_on().to_vec();
                    let source_name = edits::locate(model, &source).map_or(source, |l| l.name().to_owned());
                    depends_on.push(source_name);
                    return Ok(vec![PipelineEdit::SetDependencies {
                        element_id: target,
                        depends_on,
                        declared: true,
                    }]);
                }
                other => return Err(format!("A {other} is not something this pipeline can add.")),
            };
            let parent_id = parent_id.clone().unwrap_or_default();
            if made_with_its_parent(model, kind, &parent_id, &name) {
                return Ok(Vec::new());
            }
            Ok(vec![PipelineEdit::Add { kind, parent_id, name }])
        }
        ModelChange::Remove { id } => {
            if let Some(edge) = id.strip_prefix("edge:") {
                // A relation removed: its source's name off what its target waits for.
                let (source, target) = edge.split_once("->").unwrap_or((edge, ""));
                let Some(location) = edits::locate(model, target) else {
                    return Ok(Vec::new());
                };
                let name = edits::locate(model, source).map_or(source.trim_start_matches('?'), |l| l.name());
                let depends_on: Vec<String> = location
                    .depends_on()
                    .iter()
                    .filter(|n| !n.eq_ignore_ascii_case(name))
                    .cloned()
                    .collect();
                return Ok(vec![PipelineEdit::SetDependencies {
                    element_id: target.to_owned(),
                    declared: true,
                    depends_on,
                }]);
            }
            // A child of an element removed in the same edit is gone with it.
            if edits::locate(model, id).is_none() {
                return Ok(Vec::new());
            }
            Ok(vec![PipelineEdit::Remove { element_id: id.clone() }])
        }
        ModelChange::Move { id, index, .. } => {
            // FBL's index is the sibling it goes before, counted before the move; the module's is where it ends up.
            let from = edits::locate(model, id)
                .and_then(|l| l.job.and_then(|j| j.steps.iter().position(|s| s.id == *id)))
                .map_or(0, |p| p as i64);
            let to_index = if *index > from { *index - 1 } else { *index };
            Ok(vec![PipelineEdit::MoveStep {
                step_id: id.clone(),
                to_index,
            }])
        }
        _ => Err("This is edited in the pipeline file.".to_owned()),
    }
}

/// Whether an add of `kind` named `name` under `parent_id` is one the module already wrote with its parent: its block
/// for a new stage holds a job and a script step, and for a new job a script step (`blocks`), where the description
/// creates each of them. A job is so when its stage has a job of that name holding the one placeholder step; a step
/// when its job holds the one placeholder step and nothing else.
fn made_with_its_parent(model: &PipelineModel, kind: AddKind, parent_id: &str, name: &str) -> bool {
    let fresh = |job: &PipelineJob| {
        job.steps.len() == 1 && job.steps[0].identifier == crate::blocks::PLACEHOLDER_SCRIPT && job.template.is_empty()
    };
    let Some(parent) = edits::locate(model, parent_id) else {
        return false;
    };
    match kind {
        AddKind::Job | AddKind::DeploymentJob => {
            parent.job.is_none() && parent.stage.jobs.iter().any(|j| j.name == name && fresh(j))
        }
        AddKind::Step => parent.step.is_none() && parent.job.is_some_and(fresh),
        AddKind::Stage => false,
    }
}

fn pool(pool: &PipelinePool) -> Option<Value> {
    if !pool.is_declared() {
        return None;
    }
    let mut map = Map::default();
    if !pool.name.is_empty() {
        map.insert("name", Value::from(pool.name.as_str()));
    }
    if !pool.vm_image.is_empty() {
        map.insert("vmImage", Value::from(pool.vm_image.as_str()));
    }
    if !pool.demands.is_empty() {
        map.insert(
            "demands",
            Value::list(pool.demands.iter().map(|d| Value::from(d.as_str())).collect()),
        );
    }
    Some(Value::map(map))
}

fn put(attributes: &mut Map, name: &str, value: &str) {
    if !value.is_empty() {
        attributes.insert(name, Value::from(value));
    }
}

fn execution(attributes: &mut Map, execution: &PipelineExecution) {
    for (key, value) in [
        (PipelineExecution::CONDITION_KEY, &execution.condition),
        (PipelineExecution::CONTINUE_ON_ERROR_KEY, &execution.continue_on_error),
        (PipelineExecution::ENABLED_KEY, &execution.enabled),
        (PipelineExecution::TIMEOUT_KEY, &execution.timeout_in_minutes),
    ] {
        if execution.has(key) {
            attributes.insert(key, Value::from(value.as_str()));
        }
    }
}

fn depends_on(attributes: &mut Map, declared: bool, names: &[String]) {
    if declared {
        attributes.insert(
            "dependsOn",
            Value::list(names.iter().map(|n| Value::from(n.as_str())).collect()),
        );
    }
}

fn element(id: &str, type_name: &str, attributes: Map, parent: Option<&str>, line: usize) -> FblElement {
    FblElement {
        id: id.to_owned(),
        id_is_stored: false,
        type_name: type_name.to_owned(),
        rule: type_name.to_owned(),
        is_relation: false,
        attributes,
        parent_id: parent.map(str::to_owned),
        parent_slot: None,
        source: None,
        target: None,
        referenced: Vec::new(),
        own_span: Span::default(),
        line,
        file: None,
    }
}

fn stage_attributes(stage: &PipelineStage) -> Map {
    let mut attributes = Map::default();
    put(&mut attributes, "name", &stage.name);
    put(&mut attributes, "displayName", &stage.display_name);
    execution(&mut attributes, &stage.execution);
    put(&mut attributes, "template", &stage.template);
    put(&mut attributes, "gate", &stage.gate);
    put(&mut attributes, "isSkippable", &stage.is_skippable);
    attributes.insert("isImplicit", Value::Bool(stage.is_implicit));
    attributes.insert("manualTrigger", Value::Bool(stage.trigger_is_manual));
    if let Some(pool) = pool(&stage.pool) {
        attributes.insert("ownPool", pool);
    }
    depends_on(&mut attributes, stage.depends_on_declared, &stage.depends_on);
    attributes
}

fn job_attributes(job: &PipelineJob) -> Map {
    let mut attributes = Map::default();
    put(&mut attributes, "name", &job.name);
    put(&mut attributes, "displayName", &job.display_name);
    execution(&mut attributes, &job.execution);
    put(&mut attributes, "template", &job.template);
    put(&mut attributes, "gate", &job.gate);
    put(&mut attributes, "environment", &job.environment);
    attributes.insert("isImplicit", Value::Bool(job.is_implicit));
    let strategy = job.strategy.kind;
    attributes.insert(
        "strategy",
        Value::from(if strategy == StrategyKind::None {
            "none"
        } else {
            strategy.key()
        }),
    );
    if job.strategy.multiplicity > 0 {
        attributes.insert("multiplicity", Value::Int(i64::from(job.strategy.multiplicity)));
    }
    put(
        &mut attributes,
        "multiplicityExpression",
        &job.strategy.multiplicity_expression,
    );
    if let Some(pool) = pool(&job.pool) {
        attributes.insert("ownPool", pool);
    }
    depends_on(&mut attributes, job.depends_on_declared, &job.depends_on);
    attributes
}

fn step_attributes(step: &PipelineStep) -> Map {
    let mut attributes = Map::default();
    attributes.insert("stepKind", Value::from(step.kind.name().to_lowercase().as_str()));
    put(&mut attributes, "displayName", &step.display_name);
    put(&mut attributes, "identifier", &step.identifier);
    put(&mut attributes, "hook", &step.hook);
    put(&mut attributes, "template", &step.template);
    execution(&mut attributes, &step.execution);
    attributes
}

/// The pipeline, its stages with their jobs and steps in written order, then a relation per name a `dependsOn` writes.
fn elements(model: &PipelineModel) -> Vec<FblElement> {
    let mut elements = Vec::new();
    let mut pipeline = Map::default();
    if let Some(pool) = model
        .stages
        .first()
        .and_then(|s| pool(&s.pool).filter(|_| s.is_implicit))
    {
        pipeline.insert("pool", pool);
    }
    if let Some(extends) = &model.extends {
        pipeline.insert("extends", Value::from(extends.path.as_str()));
    }
    pipeline.insert(
        "hasStagesBlock",
        Value::Bool(model.stages.iter().any(|s| !s.is_implicit)),
    );
    elements.push(element("pipeline", "Pipeline", pipeline, None, 1));
    let mut relations = Vec::new();
    for stage in &model.stages {
        let type_name = if stage.is_implicit { "ImplicitStage" } else { "Stage" };
        elements.push(element(
            &stage.id,
            type_name,
            stage_attributes(stage),
            None,
            stage.lines.start + 1,
        ));
        relations.extend(dependencies(
            model,
            &stage.id,
            stage.depends_on_declared,
            &stage.depends_on,
        ));
        for job in &stage.jobs {
            let type_name = match (job.is_implicit, job.is_deployment) {
                (true, _) => "ImplicitJob",
                (false, true) => "DeploymentJob",
                (false, false) => "Job",
            };
            elements.push(element(
                &job.id,
                type_name,
                job_attributes(job),
                Some(&stage.id),
                job.lines.start + 1,
            ));
            relations.extend(dependencies(model, &job.id, job.depends_on_declared, &job.depends_on));
            for step in &job.steps {
                elements.push(element(
                    &step.id,
                    "Step",
                    step_attributes(step),
                    Some(&job.id),
                    step.lines.start + 1,
                ));
            }
        }
    }
    elements.extend(relations);
    elements
}

/// One `DependsOn` per name `id` waits for: from what the name resolves to, else from nothing.
fn dependencies(model: &PipelineModel, id: &str, declared: bool, names: &[String]) -> Vec<FblElement> {
    if !declared {
        return Vec::new();
    }
    let Some(location) = edits::locate(model, id) else {
        return Vec::new();
    };
    let resolved = edits::names_to_ids(model, &location);
    names
        .iter()
        .map(|name| {
            let source = resolved.get(&name.to_lowercase()).cloned();
            let mut attributes = Map::default();
            attributes.insert("name", Value::from(name.as_str()));
            let from = source.clone().unwrap_or_else(|| format!("?{name}"));
            FblElement {
                is_relation: true,
                source,
                target: Some(id.to_owned()),
                ..element(&format!("edge:{from}->{id}"), "DependsOn", attributes, None, 0)
            }
        })
        .collect()
}
