//! The findings the definition derives, as the host is handed them: in the tool's order and in the tool's words
//! (etalii.adp spec 030, task T079).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/ansible-structure/backend/
//! EtAlii.Adp.Diagram.AnsibleStructure/AnsibleFindings.cs` and `AnsibleValidator.cs`, a stand-in for gaps 6 and 8 of
//! `ansible-structure.md`. The runtime lists the reader's findings, then each rule's in declaration order; the tool
//! lists the findings about relations in relation order, so each run of findings about relations is put in the
//! model's relation order. A file that does not parse opens its sentence with the file. A `std.unparseable` naming no
//! file is not passed on, so a folder that is not there opens empty and says nothing. Every location is relative to
//! the folder, and rebased onto the project when the folder is not the project's root.

use std::collections::HashMap;

use adp_diagram::{DiagramProblem, DiagramProblemLocation, DiagramProblemSeverity};
use adp_disl::{DislDiagram, DislFinding};

const UNPARSEABLE: &str = "std.unparseable";

/// `findings` of `diagram` as the host's problems, located relative to the folder.
pub fn problems(diagram: &DislDiagram, findings: &[DislFinding]) -> Vec<DiagramProblem> {
    let mut relations: HashMap<String, usize> = HashMap::new();
    for (index, relation) in diagram.relations().iter().enumerate() {
        relations.entry(relation.id()).or_insert(index);
    }
    let relation_of = |finding: &DislFinding| finding.element_ids.first().and_then(|id| relations.get(id).copied());
    let passed: Vec<&DislFinding> = findings
        .iter()
        .filter(|f| f.constraint_id != UNPARSEABLE || f.file.as_deref().is_some_and(|f| !f.is_empty()))
        .collect();

    let mut ordered: Vec<&DislFinding> = Vec::with_capacity(passed.len());
    let mut at = 0;
    while at < passed.len() {
        if relation_of(passed[at]).is_none() {
            ordered.push(passed[at]);
            at += 1;
            continue;
        }
        let mut run = Vec::new();
        while at < passed.len() && relation_of(passed[at]).is_some() {
            run.push(passed[at]);
            at += 1;
        }
        // A stable sort, as LINQ's OrderBy is.
        run.sort_by_key(|f| relation_of(f));
        ordered.extend(run);
    }
    ordered.into_iter().map(problem).collect()
}

fn problem(finding: &DislFinding) -> DiagramProblem {
    let message = if finding.constraint_id == UNPARSEABLE {
        format!(
            "'{}' is not readable YAML: {}",
            finding.file.as_deref().unwrap_or_default(),
            finding.message
        )
    } else {
        finding.message.clone()
    };
    DiagramProblem {
        severity: match finding.severity.as_str() {
            "error" => DiagramProblemSeverity::Error,
            "warning" => DiagramProblemSeverity::Warning,
            _ => DiagramProblemSeverity::Info,
        },
        message,
        rule_id: finding.code.clone(),
        location: finding.file.as_ref().map(|file| DiagramProblemLocation::File {
            relative_path: file.clone(),
            line: finding.line.and_then(|l| u32::try_from(l).ok()).unwrap_or(0),
        }),
    }
}

/// `problems` with each location made relative to the project at `root` rather than to the folder at `folder`
/// (`AnsibleValidator.Rebased`).
pub fn rebased(problems: Vec<DiagramProblem>, root: &std::path::Path, folder: &std::path::Path) -> Vec<DiagramProblem> {
    let Ok(relative) = folder.strip_prefix(root) else {
        return problems;
    };
    let prefix = relative.to_string_lossy().replace('\\', "/");
    if prefix.is_empty() || prefix == "." {
        return problems;
    }
    problems
        .into_iter()
        .map(|mut problem| {
            if let Some(DiagramProblemLocation::File { relative_path, .. }) = &mut problem.location {
                *relative_path = format!("{prefix}/{relative_path}");
            }
            problem
        })
        .collect()
}
