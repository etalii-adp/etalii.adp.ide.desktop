//! An activity file as the diagram reads it (etalii.adp spec 030, task T075): its elements and relations, and the
//! reader's own part - what is locked where, which groups are folded, and whether archived specifications show.
//!
//! Ported from the standalone host's `src/diagrams/agent-activity-diagram/backend/EtAlii.Adp.Diagram.AgentActivityDiagram/
//! _Model/AadModel.cs` at `8b6ccfc`.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use adp_cel::Value;
use adp_disl::DislElement;
use adp_fbl::FblElement;

use crate::body::AadBody;
use crate::definition::{self, PULL_REQUESTS_GROUP, text_of};

/// The five kinds of element an agent activity diagram draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AadKind {
    Project,
    Specification,
    Agent,
    Location,
    Environment,
}

impl AadKind {
    /// The type's name in the definition.
    pub fn type_name(self) -> &'static str {
        match self {
            AadKind::Project => "Project",
            AadKind::Specification => "Specification",
            AadKind::Agent => "Agent",
            AadKind::Location => "Location",
            AadKind::Environment => "Environment",
        }
    }

    /// The wire type's suffix: the type in lower case.
    pub fn wire(self) -> &'static str {
        match self {
            AadKind::Project => "project",
            AadKind::Specification => "specification",
            AadKind::Agent => "agent",
            AadKind::Location => "location",
            AadKind::Environment => "environment",
        }
    }

    pub const ALL: [AadKind; 5] = [
        AadKind::Project,
        AadKind::Specification,
        AadKind::Agent,
        AadKind::Location,
        AadKind::Environment,
    ];
}

/// One row of a list inside an element: a task or a pull request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AadRow {
    pub id: String,
    pub title: String,
    /// A task's status as the definition's member name; empty for a pull request and for a status the definition
    /// does not have.
    pub status: String,
    /// When the row was last changed, as the file holds it; empty when it does not say.
    pub updated: String,
    pub link: String,
}

/// One element, with everything the file says about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AadElement {
    pub kind: AadKind,
    pub id: String,
    /// The element's name; for a location, its branch.
    pub name: String,
    /// A specification's status as the definition's member name; empty otherwise.
    pub status: String,
    /// That status in words, or the file's own word for a status the definition does not have.
    pub status_label: String,
    pub link: String,
    /// A location's folder: the worktree's, or `Default`.
    pub folder: String,
    pub branch_link: String,
    pub folder_link: String,
    /// An environment's kind, in words.
    pub kind_label: String,
    /// A specification's tasks, or a location's pull requests.
    pub rows: Vec<AadRow>,
}

/// One relation, read from a key on the element at its "one" end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AadRelation {
    /// The relation's derived id, such as `specification:a-dev2`.
    pub id: String,
    /// The relation type's name in the definition.
    pub type_name: String,
    pub from: String,
    pub to: String,
}

/// Where the reader put an element.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AadPlacement {
    pub x: f64,
    pub y: f64,
}

/// An activity file as the diagram reads it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AadModel {
    pub elements: Vec<AadElement>,
    pub relations: Vec<AadRelation>,
    pub placements: HashMap<String, AadPlacement>,
    /// By element, the groups that are folded, in the definition's order of names.
    pub collapsed: BTreeMap<String, BTreeSet<String>>,
    pub show_archived: bool,
}

impl AadModel {
    /// Reads the model off an open body. What the body cannot say - an unreadable file - reads as empty; the findings
    /// say why.
    pub fn read(body: &AadBody) -> AadModel {
        let disl = body.typed();
        if disl.is_unreadable {
            return AadModel::default();
        }
        let definition = definition::get();
        let diagram = &disl.diagram;
        let mut elements = Vec::new();
        for kind in AadKind::ALL {
            for node in diagram.nodes_of_type(kind.type_name()) {
                elements.push(element(kind, &node));
            }
        }
        let mut relations: Vec<AadRelation> = diagram
            .relations()
            .iter()
            .filter_map(|r| {
                let (from, to) = (r.source_id()?, r.target_id()?);
                (!from.is_empty() && !to.is_empty()).then(|| AadRelation {
                    id: r.id(),
                    type_name: r.type_name(),
                    from,
                    to,
                })
            })
            .collect();
        // A key naming an element of the wrong type is still what the file says: kept, and drawn as a breach.
        relations.extend(
            definition
                .misjoined(diagram)
                .into_iter()
                .map(|(id, type_name, from, to)| AadRelation {
                    id,
                    type_name,
                    from,
                    to,
                }),
        );

        let reading = body.model();
        let mut placements = HashMap::new();
        for placement in reading.elements.iter().filter(|e| e.type_name == "Placement") {
            // The first entry for an element applies; a second is a duplicate the findings report.
            if let (Some(id), Some(x), Some(y)) = (
                text(placement, "element").filter(|t| !t.is_empty()),
                number(placement, "x"),
                number(placement, "y"),
            ) {
                placements.entry(id).or_insert(AadPlacement { x, y });
            }
        }
        let mut stored: HashMap<(String, String), bool> = HashMap::new();
        for state in reading.elements.iter().filter(|e| e.type_name == "GroupState") {
            if let (Some(id), Some(group)) = (
                text(state, "element").filter(|t| !t.is_empty()),
                text(state, "group").filter(|t| !t.is_empty()),
            ) {
                let group = definition
                    .task_status()
                    .find(&group)
                    .map_or(group.clone(), |m| m.name.clone());
                stored
                    .entry((id, group))
                    .or_insert(matches!(state.attributes.get("collapsed"), Some(Value::Bool(true))));
            }
        }
        let mut collapsed = BTreeMap::new();
        for element in elements
            .iter()
            .filter(|e| matches!(e.kind, AadKind::Specification | AadKind::Location))
        {
            let groups: Vec<String> = if element.kind == AadKind::Specification {
                definition
                    .task_status()
                    .members
                    .iter()
                    .map(|m| m.name.clone())
                    .collect()
            } else {
                vec![PULL_REQUESTS_GROUP.to_owned()]
            };
            let folded = groups
                .into_iter()
                .filter(|g| {
                    stored
                        .get(&(element.id.clone(), g.clone()))
                        .copied()
                        .unwrap_or_else(|| definition.collapsed_by_default().contains(g))
                })
                .collect();
            collapsed.insert(element.id.clone(), folded);
        }
        let show_archived = reading
            .elements
            .iter()
            .find(|e| e.type_name == "Diagram")
            .is_some_and(|e| matches!(e.attributes.get("showArchived"), Some(Value::Bool(true))));
        AadModel {
            elements,
            relations,
            placements,
            collapsed,
            show_archived,
        }
    }

    pub fn element(&self, id: &str) -> Option<&AadElement> {
        self.elements.iter().find(|e| e.id == id)
    }
}

fn element(kind: AadKind, node: &DislElement) -> AadElement {
    let definition = definition::get();
    let status = if kind == AadKind::Specification {
        attribute(node, "status")
    } else {
        String::new()
    };
    let known = definition.specification_status().find(&status);
    let environment_kind = attribute(node, "environmentKind");
    let kind_label = if kind == AadKind::Environment {
        definition
            .environment_kind()
            .find(&environment_kind)
            .map_or(environment_kind, |m| m.label.clone())
    } else {
        String::new()
    };
    let folder = attribute(node, "folder");
    AadElement {
        kind,
        id: node.id(),
        name: attribute(node, if kind == AadKind::Location { "branch" } else { "name" }),
        status: known.map(|m| m.name.clone()).unwrap_or_default(),
        status_label: known.map_or(status.clone(), |m| m.label.clone()),
        link: attribute(node, "link"),
        folder: match kind {
            AadKind::Location if folder.is_empty() => "Default".to_owned(),
            AadKind::Location => folder,
            _ => String::new(),
        },
        branch_link: attribute(node, "branchLink"),
        folder_link: attribute(node, "folderLink"),
        kind_label,
        rows: node
            .children()
            .iter()
            .filter(|c| c.is_a("Task") || c.is_a("PullRequest"))
            .map(row)
            .collect(),
    }
}

fn row(child: &DislElement) -> AadRow {
    let definition = definition::get();
    AadRow {
        id: child.id(),
        title: attribute(child, "title"),
        status: if child.is_a("Task") {
            definition
                .task_status()
                .find(&attribute(child, "status"))
                .map(|m| m.name.clone())
                .unwrap_or_default()
        } else {
            String::new()
        },
        updated: attribute(child, "updated"),
        link: attribute(child, "link"),
    }
}

/// An attribute as the file stores it.
fn attribute(element: &DislElement, name: &str) -> String {
    element.attributes().get(name).map(text_of).unwrap_or_default()
}

fn text(entry: &FblElement, name: &str) -> Option<String> {
    entry.attributes.get(name).map(text_of)
}

fn number(entry: &FblElement, name: &str) -> Option<f64> {
    match entry.attributes.get(name)? {
        Value::Double(d) => Some(*d),
        Value::Int(i) => Some(*i as f64),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}
