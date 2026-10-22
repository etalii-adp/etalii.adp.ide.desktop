//! What waits for what, at one level: the stages of a pipeline, or the jobs of one stage (etalii.adp spec 030, task
//! T078).
//!
//! Ported from the standalone host's `src/diagrams/azure-devops-pipeline/backend/EtAlii.Adp.Diagram.AzureDevOpsPipeline/
//! PipelineGraphBuilder.cs` and `_Model/PipelineGraph.cs`, `PipelineEdge.cs` and `PipelineEdgeCondition.cs` at `8b6ccfc`.
//! `dependsOn` is matched by name, case-insensitively; a stage that declares none waits for the one before it, a job
//! for nothing; a name that matches nothing is kept as a broken edge, and a cycle is reported rather than refused.

use std::collections::{HashMap, HashSet};

use crate::model::{PipelineModel, PipelineStage};

/// Which outcome of what it waits for lets an element run (`PipelineEdgeCondition`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeCondition {
    OnSuccess,
    Always,
    OnFailure,
    OnSuccessOrFailure,
    Custom,
}

impl EdgeCondition {
    /// Its wire name, as the canvas reads it.
    pub fn name(self) -> &'static str {
        match self {
            EdgeCondition::OnSuccess => "onSuccess",
            EdgeCondition::Always => "always",
            EdgeCondition::OnFailure => "onFailure",
            EdgeCondition::OnSuccessOrFailure => "onSuccessOrFailure",
            EdgeCondition::Custom => "custom",
        }
    }
}

/// One "waits for": `to_id` does not start until `from_id` is done (`PipelineEdge`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineEdge {
    pub from_id: String,
    pub to_id: String,
    pub from_name: String,
    pub condition: EdgeCondition,
    pub condition_text: String,
    pub is_implicit: bool,
    pub is_broken: bool,
}

impl PipelineEdge {
    pub fn is_conditional(&self) -> bool {
        !self.condition_text.is_empty()
    }
}

/// The elements of one level and their dependencies (`PipelineGraph`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PipelineGraph {
    pub node_ids: Vec<String>,
    pub edges: Vec<PipelineEdge>,
    pub cycles: Vec<Vec<String>>,
}

impl PipelineGraph {
    pub fn broken_edges(&self) -> impl Iterator<Item = &PipelineEdge> {
        self.edges.iter().filter(|e| e.is_broken)
    }

    /// What `node_id` waits for, skipping edges that name nothing.
    pub fn dependencies_of<'a>(&'a self, node_id: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.edges
            .iter()
            .filter(move |e| !e.is_broken && e.to_id == node_id)
            .map(|e| e.from_id.as_str())
    }

    /// What waits for `node_id`.
    pub fn dependents_of<'a>(&'a self, node_id: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.edges
            .iter()
            .filter(move |e| !e.is_broken && e.from_id == node_id)
            .map(|e| e.to_id.as_str())
    }
}

struct GraphNode<'a> {
    id: &'a str,
    name: &'a str,
    depends_on: &'a [String],
    depends_on_declared: bool,
    condition: &'a str,
}

/// The stages of `model` and what each waits for; a stage with no `dependsOn` waits for the one before it.
pub fn of_stages(model: &PipelineModel) -> PipelineGraph {
    build(
        model
            .stages
            .iter()
            .map(|s| GraphNode {
                id: &s.id,
                name: &s.name,
                depends_on: &s.depends_on,
                depends_on_declared: s.depends_on_declared,
                condition: &s.execution.condition,
            })
            .collect(),
        true,
    )
}

/// The jobs of `stage` and what each waits for; a job with no `dependsOn` waits for nothing.
pub fn of_jobs(stage: &PipelineStage) -> PipelineGraph {
    build(
        stage
            .jobs
            .iter()
            .map(|j| GraphNode {
                id: &j.id,
                name: &j.name,
                depends_on: &j.depends_on,
                depends_on_declared: j.depends_on_declared,
                condition: &j.execution.condition,
            })
            .collect(),
        false,
    )
}

fn build(nodes: Vec<GraphNode<'_>>, sequential_by_default: bool) -> PipelineGraph {
    if nodes.is_empty() {
        return PipelineGraph::default();
    }
    let mut by_name: HashMap<String, &str> = HashMap::new();
    for node in nodes.iter().filter(|n| !n.name.is_empty()) {
        by_name.entry(node.name.to_lowercase()).or_insert(node.id);
    }
    let mut edges = Vec::new();
    for (index, node) in nodes.iter().enumerate() {
        let condition = classify(node.condition);
        if !node.depends_on.is_empty() {
            for name in node.depends_on {
                let from = by_name.get(&name.to_lowercase());
                edges.push(PipelineEdge {
                    from_id: from.map(|f| (*f).to_owned()).unwrap_or_default(),
                    to_id: node.id.to_owned(),
                    from_name: name.clone(),
                    condition,
                    condition_text: node.condition.to_owned(),
                    is_implicit: false,
                    is_broken: from.is_none(),
                });
            }
            continue;
        }
        if node.depends_on_declared || !sequential_by_default || index == 0 {
            continue;
        }
        let previous = &nodes[index - 1];
        edges.push(PipelineEdge {
            from_id: previous.id.to_owned(),
            to_id: node.id.to_owned(),
            from_name: previous.name.to_owned(),
            condition,
            condition_text: node.condition.to_owned(),
            is_implicit: true,
            is_broken: false,
        });
    }
    let node_ids: Vec<String> = nodes.iter().map(|n| n.id.to_owned()).collect();
    let cycles = find_cycles(&node_ids, &edges);
    PipelineGraph {
        node_ids,
        edges,
        cycles,
    }
}

/// Which outcome a `condition` lets its element run on: the four Azure documents, anything else custom.
pub fn classify(condition: &str) -> EdgeCondition {
    let trimmed = condition.trim();
    if trimmed.is_empty() {
        return EdgeCondition::OnSuccess;
    }
    let Some(open) = trimmed.find('(') else {
        return EdgeCondition::Custom;
    };
    let name = &trimmed[..open];
    let Some(arguments) = trimmed[open + 1..].strip_suffix(')') else {
        return EdgeCondition::Custom;
    };
    if !quoted_list(arguments) {
        return EdgeCondition::Custom;
    }
    match name.to_lowercase().as_str() {
        "succeeded" => EdgeCondition::OnSuccess,
        "failed" => EdgeCondition::OnFailure,
        "always" => EdgeCondition::Always,
        "succeededorfailed" => EdgeCondition::OnSuccessOrFailure,
        _ => EdgeCondition::Custom,
    }
}

/// Whether `text` is `\s*('...'(\s*,\s*'...')*)?\s*`: nothing, or single-quoted names separated by commas.
fn quoted_list(text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() {
        return true;
    }
    let mut rest = text;
    loop {
        let Some(after_quote) = rest.strip_prefix('\'') else {
            return false;
        };
        let Some(close) = after_quote.find('\'') else {
            return false;
        };
        rest = after_quote[close + 1..].trim_start();
        if rest.is_empty() {
            return true;
        }
        let Some(after_comma) = rest.strip_prefix(',') else {
            return false;
        };
        rest = after_comma.trim_start();
    }
}

/// Every cycle, as the ids going round it, each loop once, found depth first in declared order.
fn find_cycles(node_ids: &[String], edges: &[PipelineEdge]) -> Vec<Vec<String>> {
    let mut outgoing: HashMap<&str, Vec<&str>> = node_ids.iter().map(|id| (id.as_str(), Vec::new())).collect();
    for edge in edges.iter().filter(|e| !e.is_broken) {
        if let Some(targets) = outgoing.get_mut(edge.from_id.as_str()) {
            targets.push(edge.to_id.as_str());
        }
    }
    struct Walk<'a> {
        outgoing: HashMap<&'a str, Vec<&'a str>>,
        seen: HashSet<&'a str>,
        on_path: HashSet<&'a str>,
        path: Vec<&'a str>,
        reported: HashSet<String>,
        cycles: Vec<Vec<String>>,
    }
    impl<'a> Walk<'a> {
        fn walk(&mut self, id: &'a str) {
            self.seen.insert(id);
            self.on_path.insert(id);
            self.path.push(id);
            let nexts = self.outgoing.get(id).cloned().unwrap_or_default();
            for next in nexts {
                if self.on_path.contains(next) {
                    let at = self.path.iter().position(|p| *p == next).unwrap_or(0);
                    let cycle: Vec<String> = self.path[at..].iter().map(|s| (*s).to_owned()).collect();
                    let mut key = cycle.clone();
                    key.sort();
                    if self.reported.insert(key.join(" ")) {
                        self.cycles.push(cycle);
                    }
                } else if !self.seen.contains(next) {
                    self.walk(next);
                }
            }
            self.path.pop();
            self.on_path.remove(id);
        }
    }
    let mut walk = Walk {
        outgoing,
        seen: HashSet::new(),
        on_path: HashSet::new(),
        path: Vec::new(),
        reported: HashSet::new(),
        cycles: Vec::new(),
    };
    for id in node_ids {
        if !walk.seen.contains(id.as_str()) {
            walk.walk(id);
        }
    }
    walk.cycles
}
