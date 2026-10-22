//! What a pipeline file contains, in Azure Pipelines' own vocabulary (etalii.adp spec 030, task T078).
//!
//! Ported from the standalone host's `src/diagrams/azure-devops-pipeline/backend/EtAlii.Adp.Diagram.AzureDevOpsPipeline/
//! _Model/` at `8b6ccfc`: the stages, jobs and steps with the lines that declare each, what decides whether and how
//! each runs, the pools after inheritance, the strategies and the template references. Every value that may be an
//! expression is kept as text, never evaluated.

use std::collections::BTreeSet;

use crate::lines::LineRange;

/// The properties that decide whether and how an element runs (`PipelineExecution`). `declared` is what separates a
/// key that is absent from one whose value is empty.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PipelineExecution {
    pub condition: String,
    pub continue_on_error: String,
    pub enabled: String,
    pub timeout_in_minutes: String,
    pub declared: BTreeSet<String>,
}

impl PipelineExecution {
    pub const CONDITION_KEY: &'static str = "condition";
    pub const CONTINUE_ON_ERROR_KEY: &'static str = "continueOnError";
    pub const ENABLED_KEY: &'static str = "enabled";
    pub const TIMEOUT_KEY: &'static str = "timeoutInMinutes";

    /// The keys it carries, in one place so they cannot drift.
    pub const KEYS: [&'static str; 4] = [
        Self::CONDITION_KEY,
        Self::CONTINUE_ON_ERROR_KEY,
        Self::ENABLED_KEY,
        Self::TIMEOUT_KEY,
    ];

    /// Whether the document carries `key` at all, whatever its value.
    pub fn has(&self, key: &str) -> bool {
        self.declared.contains(key)
    }

    /// Whether a `condition` was declared, which is what the canvas marks.
    pub fn has_condition(&self) -> bool {
        self.has(Self::CONDITION_KEY)
    }

    /// Switched off outright: only a literal `false` counts.
    pub fn is_disabled(&self) -> bool {
        self.enabled.eq_ignore_ascii_case("false")
    }

    /// A failure declared not to fail the run: only a literal `true` counts.
    pub fn continues_on_error(&self) -> bool {
        self.continue_on_error.eq_ignore_ascii_case("true")
    }
}

/// Which level a pool came from, after inheritance (`PipelinePoolOrigin`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PoolOrigin {
    #[default]
    None,
    Pipeline,
    Stage,
    Job,
}

/// Where an element runs, as declared (`PipelinePool`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PipelinePool {
    pub name: String,
    pub vm_image: String,
    pub demands: Vec<String>,
    pub origin: PoolOrigin,
    pub lines: LineRange,
}

impl PipelinePool {
    pub fn is_declared(&self) -> bool {
        self.origin != PoolOrigin::None
    }

    /// What to show: the image where there is one.
    pub fn label(&self) -> &str {
        if self.vm_image.is_empty() {
            &self.name
        } else {
            &self.vm_image
        }
    }

    /// Whether it reached a job by inheritance rather than being declared on it.
    pub fn is_inherited_by_a_job(&self) -> bool {
        matches!(self.origin, PoolOrigin::Pipeline | PoolOrigin::Stage)
    }
}

/// The kind of `strategy` a job declares (`PipelineStrategyKind`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum StrategyKind {
    #[default]
    None,
    RunOnce,
    Rolling,
    Canary,
    Matrix,
    Parallel,
}

impl StrategyKind {
    /// The key the file spells it with.
    pub fn key(self) -> &'static str {
        match self {
            StrategyKind::None => "",
            StrategyKind::RunOnce => "runOnce",
            StrategyKind::Rolling => "rolling",
            StrategyKind::Canary => "canary",
            StrategyKind::Matrix => "matrix",
            StrategyKind::Parallel => "parallel",
        }
    }

    /// The kind a key names, in the order the parser tries them.
    pub fn of_key(key: &str) -> Option<StrategyKind> {
        match key {
            "runOnce" => Some(StrategyKind::RunOnce),
            "rolling" => Some(StrategyKind::Rolling),
            "canary" => Some(StrategyKind::Canary),
            "matrix" => Some(StrategyKind::Matrix),
            "parallel" => Some(StrategyKind::Parallel),
            _ => None,
        }
    }

    /// Its name as .NET's `ToString` writes it.
    pub fn name(self) -> &'static str {
        match self {
            StrategyKind::None => "None",
            StrategyKind::RunOnce => "RunOnce",
            StrategyKind::Rolling => "Rolling",
            StrategyKind::Canary => "Canary",
            StrategyKind::Matrix => "Matrix",
            StrategyKind::Parallel => "Parallel",
        }
    }
}

/// A job's `strategy` block (`PipelineStrategy`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineStrategy {
    pub kind: StrategyKind,
    pub multiplicity: i32,
    pub multiplicity_expression: String,
    pub lines: LineRange,
}

impl Default for PipelineStrategy {
    fn default() -> Self {
        PipelineStrategy {
            kind: StrategyKind::None,
            multiplicity: 1,
            multiplicity_expression: String::new(),
            lines: LineRange::default(),
        }
    }
}

impl PipelineStrategy {
    pub fn is_deployment(&self) -> bool {
        matches!(
            self.kind,
            StrategyKind::RunOnce | StrategyKind::Rolling | StrategyKind::Canary
        )
    }

    pub fn is_multiplied(&self) -> bool {
        self.multiplicity > 1 || !self.multiplicity_expression.is_empty()
    }
}

/// What a step is, named by the key it leads with (`PipelineStepKind`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum StepKind {
    #[default]
    Unknown,
    Task,
    Script,
    Bash,
    Pwsh,
    PowerShell,
    Checkout,
    Download,
    Publish,
    Template,
}

impl StepKind {
    /// The kind a step's key names.
    pub fn of_key(key: &str) -> Option<StepKind> {
        match key {
            "task" => Some(StepKind::Task),
            "script" => Some(StepKind::Script),
            "bash" => Some(StepKind::Bash),
            "pwsh" => Some(StepKind::Pwsh),
            "powershell" => Some(StepKind::PowerShell),
            "checkout" => Some(StepKind::Checkout),
            "download" => Some(StepKind::Download),
            "publish" => Some(StepKind::Publish),
            "template" => Some(StepKind::Template),
            _ => None,
        }
    }

    /// Its name as .NET's `ToString` writes it.
    pub fn name(self) -> &'static str {
        match self {
            StepKind::Unknown => "Unknown",
            StepKind::Task => "Task",
            StepKind::Script => "Script",
            StepKind::Bash => "Bash",
            StepKind::Pwsh => "Pwsh",
            StepKind::PowerShell => "PowerShell",
            StepKind::Checkout => "Checkout",
            StepKind::Download => "Download",
            StepKind::Publish => "Publish",
            StepKind::Template => "Template",
        }
    }
}

/// One step (`PipelineStep`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineStep {
    pub id: String,
    pub kind: StepKind,
    pub display_name: String,
    pub identifier: String,
    pub execution: PipelineExecution,
    pub hook: String,
    pub template: String,
    pub lines: LineRange,
}

impl PipelineStep {
    pub fn is_from_template(&self) -> bool {
        !self.template.is_empty()
    }

    /// Its `displayName`, or the first line of its identifying value, or its kind.
    pub fn label(&self) -> String {
        if !self.display_name.is_empty() {
            return self.display_name.clone();
        }
        let first = self.identifier.split('\n').next().unwrap_or_default().trim();
        if first.is_empty() {
            self.kind.name().to_owned()
        } else {
            first.to_owned()
        }
    }
}

/// One job, or the implicit one around a `steps`-only file (`PipelineJob`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineJob {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub is_deployment: bool,
    pub environment: String,
    pub strategy: PipelineStrategy,
    pub pool: PipelinePool,
    pub execution: PipelineExecution,
    pub depends_on: Vec<String>,
    pub depends_on_declared: bool,
    pub gate: String,
    pub is_implicit: bool,
    pub steps: Vec<PipelineStep>,
    pub template: String,
    pub lines: LineRange,
}

impl PipelineJob {
    pub fn is_from_template(&self) -> bool {
        !self.template.is_empty()
    }

    pub fn label(&self) -> &str {
        if !self.display_name.is_empty() {
            &self.display_name
        } else if !self.name.is_empty() {
            &self.name
        } else {
            "job"
        }
    }
}

/// One stage, or the implicit one around a file without `stages` (`PipelineStage`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineStage {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub pool: PipelinePool,
    pub execution: PipelineExecution,
    pub trigger_is_manual: bool,
    pub is_skippable: String,
    pub depends_on: Vec<String>,
    pub depends_on_declared: bool,
    pub gate: String,
    pub is_implicit: bool,
    pub jobs: Vec<PipelineJob>,
    pub template: String,
    pub lines: LineRange,
}

impl PipelineStage {
    pub fn is_from_template(&self) -> bool {
        !self.template.is_empty()
    }

    pub fn label(&self) -> &str {
        if !self.display_name.is_empty() {
            &self.display_name
        } else if !self.name.is_empty() {
            &self.name
        } else {
            "stage"
        }
    }
}

/// Where a `template` reference appeared (`PipelineTemplateSlot`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateSlot {
    Extends,
    Stages,
    Jobs,
    Steps,
}

/// A `template:` or `extends:` reference, exactly as the file wrote it (`PipelineTemplateReference`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateReference {
    pub id: String,
    pub owner_id: String,
    pub slot: TemplateSlot,
    pub path: String,
    pub resource: String,
    pub parameter_names: Vec<String>,
    pub lines: LineRange,
}

impl TemplateReference {
    /// The reference as written: its path, and `@resource` where it names one.
    pub fn reference(&self) -> String {
        if self.resource.is_empty() {
            self.path.clone()
        } else {
            format!("{}@{}", self.path, self.resource)
        }
    }
}

/// Why a template reference was not followed (`PipelineTemplateUnresolvedReason`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnresolvedReason {
    None,
    OtherRepository,
    OutsideWorkspace,
    ParameterDependent,
    NotFound,
    Unreadable,
    Cyclic,
}

/// A template that was not followed, and why (`PipelineTemplateUnresolved`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedTemplate {
    pub reference: TemplateReference,
    pub reason: UnresolvedReason,
}

impl UnresolvedTemplate {
    /// What to tell the reader, in the terms they wrote the file in.
    pub fn explanation(&self) -> String {
        match self.reason {
            UnresolvedReason::OtherRepository => format!(
                "It comes from the '{}' repository resource, which is not checked out here.",
                self.reference.resource
            ),
            UnresolvedReason::OutsideWorkspace => {
                "Its path leads outside this workspace, so it was not read.".to_owned()
            }
            UnresolvedReason::ParameterDependent => {
                "Its path depends on a parameter, so which file it means is decided at compile time.".to_owned()
            }
            UnresolvedReason::NotFound => format!("There is no file at '{}'.", self.reference.path),
            UnresolvedReason::Unreadable => format!("'{}' could not be read as a pipeline.", self.reference.path),
            UnresolvedReason::Cyclic => format!(
                "'{}' includes itself, directly or through another template.",
                self.reference.path
            ),
            UnresolvedReason::None => String::new(),
        }
    }
}

/// A whole pipeline (`PipelineModel`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PipelineModel {
    pub stages: Vec<PipelineStage>,
    pub templates: Vec<TemplateReference>,
    pub extends: Option<TemplateReference>,
    pub unresolved: Vec<UnresolvedTemplate>,
}

impl PipelineModel {
    /// Every job, stage order then declared order.
    pub fn jobs(&self) -> impl Iterator<Item = &PipelineJob> {
        self.stages.iter().flat_map(|s| s.jobs.iter())
    }

    /// Every step, job order then declared order.
    pub fn steps(&self) -> impl Iterator<Item = &PipelineStep> {
        self.jobs().flat_map(|j| j.steps.iter())
    }
}

/// Which of the three levels an element id named (`PipelineElementLocationKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocationKind {
    Stage,
    Job,
    Step,
}

/// An element as the writer sees it: which lines to edit, and whether editing them is allowed (`PipelineEditTarget`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditTarget {
    pub id: String,
    pub lines: LineRange,
    pub is_implicit: bool,
    pub template: String,
}

impl EditTarget {
    pub fn of_stage(stage: &PipelineStage) -> Self {
        EditTarget {
            id: stage.id.clone(),
            lines: stage.lines,
            is_implicit: stage.is_implicit,
            template: stage.template.clone(),
        }
    }

    pub fn of_job(job: &PipelineJob) -> Self {
        EditTarget {
            id: job.id.clone(),
            lines: job.lines,
            is_implicit: job.is_implicit,
            template: job.template.clone(),
        }
    }

    pub fn of_step(step: &PipelineStep) -> Self {
        EditTarget {
            id: step.id.clone(),
            lines: step.lines,
            is_implicit: false,
            template: step.template.clone(),
        }
    }

    /// Whether this element's own text is in this file, and so may be rewritten here.
    pub fn is_editable(&self) -> bool {
        !self.is_implicit && self.template.is_empty()
    }

    /// Why it may not be edited; empty when it may.
    pub fn read_only_reason(&self) -> String {
        if self.is_implicit {
            "This is implied by the pipeline's shape rather than written down, so there is nothing here to edit."
                .to_owned()
        } else if !self.template.is_empty() {
            format!("This comes from {}, so it has to be edited there.", self.template)
        } else {
            String::new()
        }
    }
}

/// An element found in a model, with the stage and job around it (`PipelineElementLocation`).
#[derive(Debug, Clone, Copy)]
pub struct Location<'a> {
    pub kind: LocationKind,
    pub stage: &'a PipelineStage,
    pub job: Option<&'a PipelineJob>,
    pub step: Option<&'a PipelineStep>,
}

impl<'a> Location<'a> {
    pub fn target(&self) -> EditTarget {
        match self.kind {
            LocationKind::Stage => EditTarget::of_stage(self.stage),
            LocationKind::Job => EditTarget::of_job(self.job.expect("a job's location has its job")),
            LocationKind::Step => EditTarget::of_step(self.step.expect("a step's location has its step")),
        }
    }

    pub fn id(&self) -> &'a str {
        match self.kind {
            LocationKind::Stage => &self.stage.id,
            LocationKind::Job => &self.job.expect("a job's location has its job").id,
            LocationKind::Step => &self.step.expect("a step's location has its step").id,
        }
    }

    pub fn label(&self) -> String {
        match self.kind {
            LocationKind::Stage => self.stage.label().to_owned(),
            LocationKind::Job => self.job.expect("a job's location has its job").label().to_owned(),
            LocationKind::Step => self.step.expect("a step's location has its step").label(),
        }
    }

    pub fn execution(&self) -> &'a PipelineExecution {
        match self.kind {
            LocationKind::Stage => &self.stage.execution,
            LocationKind::Job => &self.job.expect("a job's location has its job").execution,
            LocationKind::Step => &self.step.expect("a step's location has its step").execution,
        }
    }

    pub fn display_name(&self) -> &'a str {
        match self.kind {
            LocationKind::Stage => &self.stage.display_name,
            LocationKind::Job => &self.job.expect("a job's location has its job").display_name,
            LocationKind::Step => &self.step.expect("a step's location has its step").display_name,
        }
    }
}
