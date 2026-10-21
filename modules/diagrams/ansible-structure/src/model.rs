//! What the reader makes of an Ansible project folder, before it becomes a model (etalii.adp spec 030, task T079).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/ansible-structure/backend/
//! EtAlii.Adp.Diagram.AnsibleStructure/_Model/` (the project, its playbooks and plays, roles and their contents, task
//! files, inventories with their groups and variable folders, the directives that relate them, and the nodes and edges
//! of the graph). Plain data: no file access, nothing that changes once read.

/// Which Ansible directive declared a relation (`AnsibleDirectiveKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnsibleDirectiveKind {
    /// A play's `roles:` list.
    Roles,
    ImportRole,
    IncludeRole,
    ImportPlaybook,
    ImportTasks,
    IncludeTasks,
    /// A `meta/main.yml` `dependencies:` entry.
    Dependency,
}

/// One directive as written: what it names, its `when:`, and where (`AnsibleDirective`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnsibleDirective {
    pub kind: AnsibleDirectiveKind,
    pub target: String,
    pub condition: String,
    /// The file it is declared in, relative to the folder.
    pub declared_in: String,
    /// The 1-based line it is declared on.
    pub line: u32,
}

impl AnsibleDirective {
    /// Resolved during the run rather than before it.
    pub fn is_dynamic(&self) -> bool {
        matches!(
            self.kind,
            AnsibleDirectiveKind::IncludeRole | AnsibleDirectiveKind::IncludeTasks
        )
    }

    /// The target holds `{{`, so what it names is not knowable without running Ansible.
    pub fn is_expression(&self) -> bool {
        self.target.contains("{{")
    }
}

/// One play of a playbook (`AnsiblePlay`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnsiblePlay {
    /// As written; empty when it has none.
    pub name: String,
    pub hosts: String,
    /// Its 0-based position among the playbook's plays.
    pub index: usize,
    pub line: u32,
    pub directives: Vec<AnsibleDirective>,
}

/// A YAML file whose top level is a list of mappings (`AnsiblePlaybook`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnsiblePlaybook {
    pub relative_path: String,
    pub name: String,
    pub plays: Vec<AnsiblePlay>,
    pub imports: Vec<AnsibleDirective>,
}

/// Which of a role's canonical folders exist and how many files each holds directly (`AnsibleRoleContents`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AnsibleRoleContents {
    pub task_files: usize,
    pub handlers: usize,
    pub templates: usize,
    pub files: usize,
    pub defaults: usize,
    pub vars: usize,
    pub has_meta: bool,
    pub has_library: bool,
}

impl AnsibleRoleContents {
    /// Nothing Ansible would run.
    pub fn is_hollow(&self) -> bool {
        self.task_files == 0
            && self.handlers == 0
            && self.templates == 0
            && self.files == 0
            && self.defaults == 0
            && self.vars == 0
            && !self.has_meta
            && !self.has_library
    }
}

/// A YAML file in a role's `tasks/` (`AnsibleTaskFile`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnsibleTaskFile {
    pub name: String,
    pub relative_path: String,
}

/// A folder under `roles/` (`AnsibleRole`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnsibleRole {
    pub name: String,
    pub relative_path: String,
    pub contents: AnsibleRoleContents,
    pub task_files: Vec<AnsibleTaskFile>,
    pub dependencies: Vec<AnsibleDirective>,
    pub task_includes: Vec<AnsibleDirective>,
}

/// One group an inventory defines (`AnsibleInventoryGroup`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnsibleInventoryGroup {
    pub name: String,
    pub host_count: usize,
}

/// A `group_vars` or `host_vars` folder (`AnsibleVariableFolder`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnsibleVariableFolder {
    pub name: String,
    pub relative_path: String,
    pub file_count: usize,
}

/// An environment under `inventories/`, or one inventory file at the root (`AnsibleInventory`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnsibleInventory {
    pub name: String,
    pub relative_path: String,
    pub groups: Vec<AnsibleInventoryGroup>,
    pub variable_folders: Vec<AnsibleVariableFolder>,
}

/// A file that did not parse: which, why, and the parser's line (`TolerantYamlFailure`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnsibleFailure {
    pub relative_path: String,
    pub message: String,
    pub line: u32,
}

/// One registered folder as read (`AnsibleProject`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AnsibleProject {
    pub folder_path: String,
    pub playbooks: Vec<AnsiblePlaybook>,
    pub roles: Vec<AnsibleRole>,
    pub inventories: Vec<AnsibleInventory>,
    /// The files whose presence is recorded rather than drawn: `ansible.cfg`, `requirements.yml`.
    pub annotations: Vec<String>,
    pub failures: Vec<AnsibleFailure>,
}

impl AnsibleProject {
    pub fn is_empty(&self) -> bool {
        self.playbooks.is_empty() && self.roles.is_empty() && self.inventories.is_empty()
    }

    /// The role named `name`, compared ordinally.
    pub fn role(&self, name: &str) -> Option<&AnsibleRole> {
        self.roles.iter().find(|r| r.name == name)
    }
}

/// What a node of the graph stands for (`AnsibleNodeKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnsibleNodeKind {
    Playbook,
    Play,
    Role,
    TaskFile,
    Inventory,
    VariableFolder,
}

impl AnsibleNodeKind {
    /// The definition's type name.
    pub fn type_name(self) -> &'static str {
        match self {
            AnsibleNodeKind::Playbook => "Playbook",
            AnsibleNodeKind::Play => "Play",
            AnsibleNodeKind::Role => "Role",
            AnsibleNodeKind::TaskFile => "TaskFile",
            AnsibleNodeKind::Inventory => "Inventory",
            AnsibleNodeKind::VariableFolder => "VariableFolder",
        }
    }
}

/// One box of the graph (`AnsibleNode`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnsibleNode {
    pub id: String,
    pub kind: AnsibleNodeKind,
    pub name: String,
    pub relative_path: String,
    /// A play's index across the project; -1 for anything else.
    pub play_index: i64,
}

/// What a relation of the graph says (`AnsibleEdgeKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnsibleEdgeKind {
    UsesRole,
    ImportsPlaybook,
    IncludesTasks,
    DependsOn,
    Targets,
}

impl AnsibleEdgeKind {
    /// The definition's relation type name, which is also how an edge's id spells it.
    pub fn type_name(self) -> &'static str {
        match self {
            AnsibleEdgeKind::UsesRole => "UsesRole",
            AnsibleEdgeKind::ImportsPlaybook => "ImportsPlaybook",
            AnsibleEdgeKind::IncludesTasks => "IncludesTasks",
            AnsibleEdgeKind::DependsOn => "DependsOn",
            AnsibleEdgeKind::Targets => "Targets",
        }
    }
}

/// What became of a relation's target (`AnsibleTargetResolution`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnsibleTargetResolution {
    Resolved,
    Missing,
    Unresolvable,
}

/// One relation of the graph (`AnsibleEdge`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnsibleEdge {
    pub source_id: String,
    /// Empty for a target that is missing or an expression.
    pub target_id: String,
    pub kind: AnsibleEdgeKind,
    pub resolution: AnsibleTargetResolution,
    pub directive: AnsibleDirective,
}

impl AnsibleEdge {
    /// `edge:`, the source's id, the kind and the target as written; a `Targets` edge adds the inventory's id.
    pub fn id(&self) -> String {
        if self.kind == AnsibleEdgeKind::Targets {
            format!(
                "edge:{}|{}|{}|{}",
                self.source_id,
                self.kind.type_name(),
                self.directive.target,
                self.target_id
            )
        } else {
            format!(
                "edge:{}|{}|{}",
                self.source_id,
                self.kind.type_name(),
                self.directive.target
            )
        }
    }

    pub fn is_dynamic(&self) -> bool {
        self.directive.is_dynamic()
    }
}
