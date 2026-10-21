//! The FBL section 11 persistence plugin of the binding `ansible-structure.fbl#ansible`,
//! `net.etalii.adp.ansible.folder` (etalii.adp spec 030, task T079).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/ansible-structure/backend/
//! EtAlii.Adp.Diagram.AnsibleStructure/AnsibleFolderPlugin.cs`. It reads an Ansible project folder with
//! [`crate::reader`] and [`AnsibleGraph`], so the DISL model holds exactly the nodes and relations the canvas, the rows
//! and the findings read: the graph's nodes in the graph's order, then its relations (finding B6), each with its file
//! and line and the graph's id, which the definition's `derived` ids compute again. A relation whose target is missing
//! or an expression has no target (finding B7); a file that does not parse is one `std.unparseable` at its file and the
//! parser's line. It reads and never plans: the binding is read-only.

use std::rc::Rc;

use adp_cel::{Map, Value};
use adp_fbl::FblBinding;
use adp_fbl::model::{FblElement, Finding, FindingSeverity, SourceLocation, codes};
use adp_fbl::plugins::{
    PersistencePlugin, PluginPlanRequest, PluginPlanResult, PluginReadRequest, PluginReadResult, PluginTemplateRequest,
};
use adp_fbl::text::Span;

use crate::graph::AnsibleGraph;
use crate::listing::AnsibleFolderListing;
use crate::model::{
    AnsibleDirectiveKind, AnsibleEdge, AnsibleNode, AnsibleNodeKind, AnsiblePlaybook, AnsibleProject,
    AnsibleTargetResolution,
};
use crate::reader;

/// The plugin's id, as the binding's `reader.plugin` names it.
pub const PLUGIN_ID: &str = "net.etalii.adp.ansible.folder";

/// What the reading depends on beyond the files the rules select, relative to the folder.
const WATCHED: [&str; 6] = [
    "ansible.cfg",
    "collections",
    "group_vars",
    "host_vars",
    "inventories",
    "roles",
];

/// The plugin for one folder.
#[derive(Debug, Clone)]
pub struct AnsibleFolderPlugin {
    listing: AnsibleFolderListing,
    binding: Rc<FblBinding>,
}

impl AnsibleFolderPlugin {
    /// The plugin for the folder `listing` lists, reading through `binding`.
    pub fn new(listing: AnsibleFolderListing, binding: Rc<FblBinding>) -> Self {
        AnsibleFolderPlugin { listing, binding }
    }

    /// The project `files` read to, before it becomes elements.
    pub fn project(&self, files: &[adp_fbl::plugins::PluginFile]) -> AnsibleProject {
        reader::read(files, &self.listing, &self.binding)
    }
}

impl PersistencePlugin for AnsibleFolderPlugin {
    fn id(&self) -> &str {
        PLUGIN_ID
    }

    fn read(&self, request: &PluginReadRequest) -> PluginReadResult {
        result(&self.project(&request.files))
    }

    fn plan(&self, _request: &PluginPlanRequest<'_>) -> PluginPlanResult {
        PluginPlanResult::Refused(
            "An Ansible structure diagram is read-only, so its plugin is never asked to plan a change.".into(),
        )
    }

    fn template(&self, _request: &PluginTemplateRequest) -> Vec<u8> {
        Vec::new()
    }

    fn watch(&self, _last: &PluginReadResult) -> Vec<String> {
        WATCHED.iter().map(|w| (*w).to_owned()).collect()
    }
}

/// What a read delivers for a project already read: its elements and a finding per file that did not parse.
pub fn result(project: &AnsibleProject) -> PluginReadResult {
    let findings = project
        .failures
        .iter()
        .map(|failure| Finding {
            code: codes::UNPARSEABLE.into(),
            severity: FindingSeverity::Error,
            message: failure.message.clone(),
            location: Some(SourceLocation {
                file: failure.relative_path.clone(),
                line: failure.line as usize,
                column: 1,
                length: 0,
            }),
            detail: None,
        })
        .collect();
    PluginReadResult::new(elements(project, &AnsibleGraph::derive(project)), findings, false)
}

fn element(
    id: String,
    type_name: &str,
    is_relation: bool,
    attributes: Map,
    ends: (Option<String>, Option<String>),
    line: u32,
    file: &str,
) -> FblElement {
    FblElement {
        id,
        id_is_stored: false,
        type_name: type_name.to_owned(),
        rule: type_name.to_owned(),
        is_relation,
        attributes,
        parent_id: None,
        parent_slot: None,
        source: ends.0,
        target: ends.1,
        own_span: Span::default(),
        line: line as usize,
        file: Some(file.to_owned()),
    }
}

/// One element per node in the graph's order, then one per relation in the graph's order.
fn elements(project: &AnsibleProject, graph: &AnsibleGraph) -> Vec<FblElement> {
    let mut elements = Vec::with_capacity(graph.nodes.len() + graph.edges.len());
    for node in &graph.nodes {
        let (attributes, line) = match node.kind {
            AnsibleNodeKind::Playbook => (playbook(project, node), 0),
            AnsibleNodeKind::Play => play(project, node),
            AnsibleNodeKind::Role => (role(project, node), 0),
            AnsibleNodeKind::TaskFile => (identity(node), 0),
            AnsibleNodeKind::Inventory => (inventory(project, node), 0),
            AnsibleNodeKind::VariableFolder => (variable_folder(project, node), 0),
        };
        let type_name = node.kind.type_name();
        elements.push(element(
            node.id.clone(),
            type_name,
            false,
            attributes,
            (None, None),
            line,
            &node.relative_path,
        ));
    }
    for edge in &graph.edges {
        let target = (edge.resolution == AnsibleTargetResolution::Resolved && !edge.target_id.is_empty())
            .then(|| edge.target_id.clone());
        elements.push(element(
            edge.id(),
            edge.kind.type_name(),
            true,
            relation(edge),
            (Some(edge.source_id.clone()), target),
            edge.directive.line,
            &edge.directive.declared_in,
        ));
    }
    elements
}

fn int(n: usize) -> Value {
    Value::Int(i64::try_from(n).unwrap_or(i64::MAX))
}

fn text(s: &str) -> Value {
    Value::String(s.to_owned())
}

fn identity(node: &AnsibleNode) -> Map {
    let mut map = Map::default();
    map.insert("name", text(&node.name));
    map.insert("path", text(&node.relative_path));
    map
}

fn annotations(project: &AnsibleProject) -> Value {
    Value::list(project.annotations.iter().map(|a| text(a)).collect())
}

fn playbook_of<'a>(project: &'a AnsibleProject, node: &AnsibleNode) -> Option<&'a AnsiblePlaybook> {
    project.playbooks.iter().find(|p| p.relative_path == node.relative_path)
}

fn playbook(project: &AnsibleProject, node: &AnsibleNode) -> Map {
    let mut attributes = identity(node);
    attributes.insert("annotations", annotations(project));
    if let Some(playbook) = playbook_of(project, node) {
        let plays = playbook
            .plays
            .iter()
            .map(|play| {
                let mut map = Map::default();
                map.insert("ordinal", int(play.index));
                map.insert("name", text(&play.name));
                map.insert("hosts", text(&play.hosts));
                map.insert("line", Value::Int(i64::from(play.line)));
                Value::map(map)
            })
            .collect();
        attributes.insert("plays", Value::list(plays));
        // The playbook stands for its play when it holds exactly one, and carries its hosts.
        let hosts = match playbook.plays.as_slice() {
            [only] => only.hosts.clone(),
            _ => String::new(),
        };
        attributes.insert("hosts", Value::String(hosts));
    }
    attributes
}

fn play(project: &AnsibleProject, node: &AnsibleNode) -> (Map, u32) {
    let mut attributes = identity(node);
    attributes.insert("annotations", annotations(project));
    attributes.insert("playIndex", Value::Int(node.play_index));
    // A play's id is its playbook's path, '#' and its index among the playbook's plays.
    let mut line = 0;
    if let Some((_, index)) = node.id.rsplit_once('#')
        && let Ok(index) = index.parse::<usize>()
        && let Some(play) = playbook_of(project, node).and_then(|p| p.plays.iter().find(|play| play.index == index))
    {
        attributes.insert("ordinal", int(play.index));
        attributes.insert("hosts", text(&play.hosts));
        line = play.line;
    }
    (attributes, line)
}

fn role(project: &AnsibleProject, node: &AnsibleNode) -> Map {
    let mut attributes = identity(node);
    if let Some(role) = project.role(&node.name) {
        let contents = role.contents;
        let mut map = Map::default();
        map.insert("taskFiles", int(contents.task_files));
        map.insert("handlers", int(contents.handlers));
        map.insert("templates", int(contents.templates));
        map.insert("files", int(contents.files));
        map.insert("defaults", int(contents.defaults));
        map.insert("vars", int(contents.vars));
        map.insert("hasMeta", Value::Bool(contents.has_meta));
        map.insert("hasLibrary", Value::Bool(contents.has_library));
        attributes.insert("contents", Value::map(map));
    }
    attributes
}

fn inventory(project: &AnsibleProject, node: &AnsibleNode) -> Map {
    let mut attributes = identity(node);
    if let Some(inventory) = project
        .inventories
        .iter()
        .find(|i| i.relative_path == node.relative_path)
    {
        let groups = inventory
            .groups
            .iter()
            .map(|g| {
                let mut map = Map::default();
                map.insert("name", text(&g.name));
                map.insert("hostCount", int(g.host_count));
                Value::map(map)
            })
            .collect();
        attributes.insert("groups", Value::list(groups));
        let folders = inventory
            .variable_folders
            .iter()
            .map(|f| {
                let mut map = Map::default();
                map.insert("name", text(&f.name));
                map.insert("path", text(&f.relative_path));
                map.insert("fileCount", int(f.file_count));
                Value::map(map)
            })
            .collect();
        attributes.insert("variableFolders", Value::list(folders));
    }
    attributes
}

fn variable_folder(project: &AnsibleProject, node: &AnsibleNode) -> Map {
    let mut attributes = identity(node);
    if let Some(folder) = project
        .inventories
        .iter()
        .flat_map(|i| &i.variable_folders)
        .find(|f| f.relative_path == node.relative_path)
    {
        attributes.insert("fileCount", int(folder.file_count));
    }
    attributes
}

fn relation(edge: &AnsibleEdge) -> Map {
    let mut map = Map::default();
    map.insert("directive", text(directive(edge.directive.kind)));
    map.insert("targetAsWritten", text(&edge.directive.target));
    map.insert("condition", text(&edge.directive.condition));
    map.insert("declaredIn", text(&edge.directive.declared_in));
    map.insert("declaredAtLine", Value::Int(i64::from(edge.directive.line)));
    map.insert(
        "resolution",
        text(match edge.resolution {
            AnsibleTargetResolution::Resolved => "resolved",
            AnsibleTargetResolution::Missing => "missing",
            AnsibleTargetResolution::Unresolvable => "expression",
        }),
    );
    map
}

/// The definition's `Directive` value of `kind`.
fn directive(kind: AnsibleDirectiveKind) -> &'static str {
    match kind {
        AnsibleDirectiveKind::Roles => "roles",
        AnsibleDirectiveKind::ImportRole => "importRole",
        AnsibleDirectiveKind::IncludeRole => "includeRole",
        AnsibleDirectiveKind::ImportPlaybook => "importPlaybook",
        AnsibleDirectiveKind::ImportTasks => "importTasks",
        AnsibleDirectiveKind::IncludeTasks => "includeTasks",
        AnsibleDirectiveKind::Dependency => "dependencies",
    }
}

/// The definition's model of a project already read, as a reading through the binding builds it.
pub fn model_of(project: &AnsibleProject) -> adp_disl::DislModel {
    let result = result(project);
    crate::definition::AnsibleDefinition::get().model(&adp_fbl::FblModel {
        elements: result.elements,
        findings: result.findings,
        unreadable: result.unreadable,
        ..adp_fbl::FblModel::default()
    })
}
