//! The nodes and edges of one [`AnsibleProject`]: the relationships that are the diagram, resolved against what is
//! actually in the folder (etalii.adp spec 030, task T079).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/ansible-structure/backend/
//! EtAlii.Adp.Diagram.AnsibleStructure/AnsibleGraph.cs`. Pure: a project in, a graph out. A play is a box of its own
//! only when its playbook holds more than one; a task file is drawn only when something includes it; a play's hosts
//! pattern reaches every inventory that defines a matching group. No per-variable usage edge is drawn: that would be an
//! inference, and a wrong edge is worse than a missing one.

use std::collections::HashSet;

use crate::model::{
    AnsibleDirective, AnsibleDirectiveKind, AnsibleEdge, AnsibleEdgeKind, AnsibleInventory, AnsibleNode,
    AnsibleNodeKind, AnsiblePlay, AnsiblePlaybook, AnsibleProject, AnsibleRole, AnsibleTargetResolution,
    AnsibleTaskFile, AnsibleVariableFolder,
};

/// The graph of a project.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AnsibleGraph {
    pub nodes: Vec<AnsibleNode>,
    pub edges: Vec<AnsibleEdge>,
}

impl AnsibleGraph {
    /// The node with that id.
    pub fn node(&self, id: &str) -> Option<&AnsibleNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// The graph of `project`.
    pub fn derive(project: &AnsibleProject) -> AnsibleGraph {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        // A play's colour index runs in declaration order across the whole project, so two playbooks' plays never
        // share one.
        let mut play_index = 0_i64;
        for playbook in &project.playbooks {
            nodes.push(AnsibleNode {
                id: playbook_id(playbook),
                kind: AnsibleNodeKind::Playbook,
                name: playbook.name.clone(),
                relative_path: playbook.relative_path.clone(),
                play_index: -1,
            });
            let plays_are_drawn = playbook.plays.len() > 1;
            for play in &playbook.plays {
                let index = play_index;
                play_index += 1;
                if plays_are_drawn {
                    nodes.push(AnsibleNode {
                        id: play_id(playbook, play),
                        kind: AnsibleNodeKind::Play,
                        name: if play.name.is_empty() {
                            format!("play {}", play.index + 1)
                        } else {
                            play.name.clone()
                        },
                        relative_path: playbook.relative_path.clone(),
                        play_index: index,
                    });
                }
            }
        }

        for role in &project.roles {
            nodes.push(AnsibleNode {
                id: role_id(&role.name),
                kind: AnsibleNodeKind::Role,
                name: role.name.clone(),
                relative_path: role.relative_path.clone(),
                play_index: -1,
            });
        }

        for inventory in &project.inventories {
            nodes.push(AnsibleNode {
                id: inventory_id(inventory),
                kind: AnsibleNodeKind::Inventory,
                name: inventory.name.clone(),
                relative_path: inventory.relative_path.clone(),
                play_index: -1,
            });
            for folder in &inventory.variable_folders {
                nodes.push(AnsibleNode {
                    id: variable_folder_id(folder),
                    kind: AnsibleNodeKind::VariableFolder,
                    name: folder.name.clone(),
                    relative_path: folder.relative_path.clone(),
                    play_index: -1,
                });
            }
        }

        // Task files are drawn only when something includes them, so they are added with the include edges.
        let mut drawn_task_files: HashSet<String> = HashSet::new();

        for playbook in &project.playbooks {
            let plays_are_drawn = playbook.plays.len() > 1;
            for import in &playbook.imports {
                let target = resolve_path(project, &import.target, &playbook.relative_path);
                edges.push(AnsibleEdge {
                    source_id: playbook_id(playbook),
                    target_id: target.map(playbook_id).unwrap_or_default(),
                    kind: AnsibleEdgeKind::ImportsPlaybook,
                    resolution: resolution(import, target.is_some()),
                    directive: import.clone(),
                });
            }
            for play in &playbook.plays {
                let owner = if plays_are_drawn {
                    play_id(playbook, play)
                } else {
                    playbook_id(playbook)
                };
                for directive in &play.directives {
                    // A play's tasks may include a task file, but with no role to resolve it against there is nothing
                    // to point at; only role references are drawn.
                    if matches!(
                        directive.kind,
                        AnsibleDirectiveKind::IncludeTasks | AnsibleDirectiveKind::ImportTasks
                    ) {
                        continue;
                    }
                    let role = if directive.is_expression() {
                        None
                    } else {
                        project.role(&directive.target)
                    };
                    edges.push(AnsibleEdge {
                        source_id: owner.clone(),
                        target_id: role.map(|r| role_id(&r.name)).unwrap_or_default(),
                        kind: AnsibleEdgeKind::UsesRole,
                        resolution: resolution(directive, role.is_some()),
                        directive: directive.clone(),
                    });
                }
                if !play.hosts.is_empty() {
                    edges.extend(target_edges(project, &owner, play));
                }
            }
        }

        for role in &project.roles {
            for dependency in &role.dependencies {
                let target = if dependency.is_expression() {
                    None
                } else {
                    project.role(&dependency.target)
                };
                edges.push(AnsibleEdge {
                    source_id: role_id(&role.name),
                    target_id: target.map(|r| role_id(&r.name)).unwrap_or_default(),
                    kind: AnsibleEdgeKind::DependsOn,
                    resolution: resolution(dependency, target.is_some()),
                    directive: dependency.clone(),
                });
            }
            for include in &role.task_includes {
                if matches!(
                    include.kind,
                    AnsibleDirectiveKind::IncludeRole | AnsibleDirectiveKind::ImportRole
                ) {
                    // A role reaching for another role is the relationship a play's roles: list declares.
                    let used = if include.is_expression() {
                        None
                    } else {
                        project.role(&include.target)
                    };
                    edges.push(AnsibleEdge {
                        source_id: role_id(&role.name),
                        target_id: used.map(|r| role_id(&r.name)).unwrap_or_default(),
                        kind: AnsibleEdgeKind::UsesRole,
                        resolution: resolution(include, used.is_some()),
                        directive: include.clone(),
                    });
                    continue;
                }
                let file = if include.is_expression() {
                    None
                } else {
                    resolve_task_file(role, include)
                };
                if let Some(file) = file
                    && drawn_task_files.insert(file.relative_path.clone())
                {
                    nodes.push(AnsibleNode {
                        id: task_file_id(file),
                        kind: AnsibleNodeKind::TaskFile,
                        name: file.name.clone(),
                        relative_path: file.relative_path.clone(),
                        play_index: -1,
                    });
                }
                edges.push(AnsibleEdge {
                    source_id: role_id(&role.name),
                    target_id: file.map(task_file_id).unwrap_or_default(),
                    kind: AnsibleEdgeKind::IncludesTasks,
                    resolution: resolution(include, file.is_some()),
                    directive: include.clone(),
                });
            }
        }

        AnsibleGraph { nodes, edges }
    }
}

/// A play's `hosts:` pattern against every inventory that defines a matching group: one edge per inventory. A pattern
/// nothing defines yields no edge; the rules report it instead.
fn target_edges(project: &AnsibleProject, owner: &str, play: &AnsiblePlay) -> Vec<AnsibleEdge> {
    let directive = AnsibleDirective {
        kind: AnsibleDirectiveKind::Roles,
        target: play.hosts.clone(),
        condition: String::new(),
        declared_in: String::new(),
        line: play.line,
    };
    project
        .inventories
        .iter()
        .filter(|inventory| matches(Some(&play.hosts), inventory))
        .map(|inventory| AnsibleEdge {
            source_id: owner.to_owned(),
            target_id: inventory_id(inventory),
            kind: AnsibleEdgeKind::Targets,
            resolution: AnsibleTargetResolution::Resolved,
            directive: directive.clone(),
        })
        .collect()
}

/// Whether a host pattern names anything `inventory` defines: a deliberate subset of Ansible's pattern language (union
/// separators, `all` and `*`, a trailing glob). An exclusion or an intersection creates no edge.
pub fn matches(pattern: Option<&str>, inventory: &AnsibleInventory) -> bool {
    for token in pattern
        .unwrap_or_default()
        .split([':', ','])
        .map(str::trim)
        .filter(|t| !t.is_empty())
    {
        if token.starts_with('!') || token.starts_with('&') {
            continue;
        }
        if token == "all" || token == "*" {
            // Every inventory defines "all" implicitly, but only if it has anything in it.
            if !inventory.groups.is_empty() {
                return true;
            }
            continue;
        }
        if inventory.groups.iter().any(|g| name_matches(token, &g.name)) {
            return true;
        }
    }
    false
}

fn name_matches(token: &str, group: &str) -> bool {
    match token.strip_suffix('*') {
        Some(prefix) => group.starts_with(prefix),
        None => token == group,
    }
}

fn resolution(directive: &AnsibleDirective, found: bool) -> AnsibleTargetResolution {
    if directive.is_expression() {
        AnsibleTargetResolution::Unresolvable
    } else if found {
        AnsibleTargetResolution::Resolved
    } else {
        AnsibleTargetResolution::Missing
    }
}

/// The playbook a path names, resolved relative to the file that named it.
fn resolve_path<'a>(project: &'a AnsibleProject, target: &str, declaring_file: &str) -> Option<&'a AnsiblePlaybook> {
    let folder = folder_of(declaring_file);
    let combined = normalise(&if folder.is_empty() {
        target.to_owned()
    } else {
        format!("{folder}/{target}")
    });
    project
        .playbooks
        .iter()
        .find(|p| p.relative_path.eq_ignore_ascii_case(&combined))
}

/// The task file an include names, resolved relative to the file that included it.
fn resolve_task_file<'a>(role: &'a AnsibleRole, include: &AnsibleDirective) -> Option<&'a AnsibleTaskFile> {
    let folder = folder_of(&include.declared_in);
    let combined = normalise(&if folder.is_empty() {
        include.target.clone()
    } else {
        format!("{folder}/{}", include.target)
    });
    role.task_files
        .iter()
        .find(|f| f.relative_path.eq_ignore_ascii_case(&combined))
}

fn folder_of(relative: &str) -> &str {
    relative.rfind('/').map_or("", |slash| &relative[..slash])
}

/// Collapses `.` and `..` segments without touching the disk.
fn normalise(path: &str) -> String {
    let mut segments: Vec<&str> = Vec::new();
    for segment in path.split('/').filter(|s| !s.is_empty()) {
        if segment == "." {
            continue;
        }
        if segment == ".." && segments.last().is_some_and(|last| *last != "..") {
            segments.pop();
            continue;
        }
        segments.push(segment);
    }
    segments.join("/")
}

fn playbook_id(playbook: &AnsiblePlaybook) -> String {
    format!("playbook:{}", playbook.relative_path)
}

fn play_id(playbook: &AnsiblePlaybook, play: &AnsiblePlay) -> String {
    format!("play:{}#{}", playbook.relative_path, play.index)
}

fn role_id(name: &str) -> String {
    format!("role:{name}")
}

fn task_file_id(file: &AnsibleTaskFile) -> String {
    format!("taskfile:{}", file.relative_path)
}

fn inventory_id(inventory: &AnsibleInventory) -> String {
    format!("inventory:{}", inventory.relative_path)
}

fn variable_folder_id(folder: &AnsibleVariableFolder) -> String {
    format!("vars:{}", folder.relative_path)
}
