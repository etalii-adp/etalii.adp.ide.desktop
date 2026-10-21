//! The findings of a chart folder as the host is handed them: in the tool's order and in the tool's words (etalii.adp
//! spec 030, task T080).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/helm-chart/backend/EtAlii.Adp.Diagram.HelmChart/
//! HelmDefinition.cs` (`Findings`), `HelmFindingOrder.cs`, `HelmNotAChart.cs` and `HelmValidator.cs`. A registered
//! folder that is not a chart has exactly one warning, `helm.not-a-chart`, at the folder itself (gap 8 of
//! `helm-chart.md`); otherwise the definition's findings, each run of findings of one code about dependencies put in
//! the order of the dependencies, which is by effective name (gap 9). A finding without a line is at line 0. Every
//! location is relative to the chart folder, and rebased onto the project when the folder is not its root.

use std::collections::HashMap;
use std::path::Path;

use adp_diagram::{DiagramProblem, DiagramProblemLocation, DiagramProblemSeverity};
use adp_disl::{DislDiagram, DislFinding};

/// The tool's code for a registered folder that is not a chart.
pub const NOT_A_CHART: &str = "helm.not-a-chart";

/// What the tool says of it.
pub const NOT_A_CHART_SENTENCE: &str = "This folder is not a Helm chart: it has no Chart.yaml.";

/// The one finding of a registered folder that is not a chart, at the folder (`.`).
pub fn not_a_chart() -> Vec<DiagramProblem> {
    vec![DiagramProblem {
        severity: DiagramProblemSeverity::Warning,
        message: NOT_A_CHART_SENTENCE.to_owned(),
        rule_id: NOT_A_CHART.to_owned(),
        location: Some(DiagramProblemLocation::File {
            relative_path: ".".to_owned(),
            line: 0,
        }),
    }]
}

/// `findings` of `diagram` in the tool's order (`HelmFindingOrder.Of`): within a run of findings of one code that are
/// each about a dependency, in the order of the dependencies; a stable sort, so every other finding keeps its place.
pub fn ordered<'a>(diagram: &DislDiagram, findings: &'a [DislFinding]) -> Vec<&'a DislFinding> {
    let mut places: HashMap<String, usize> = HashMap::new();
    for (index, dependency) in diagram.nodes_of_type("Dependency").iter().enumerate() {
        places.entry(dependency.id()).or_insert(index);
    }
    let place_of = |finding: &DislFinding| finding.element_ids.first().and_then(|id| places.get(id).copied());
    let mut ordered = Vec::with_capacity(findings.len());
    let mut at = 0;
    while at < findings.len() {
        if place_of(&findings[at]).is_none() {
            ordered.push(&findings[at]);
            at += 1;
            continue;
        }
        let code = &findings[at].code;
        let mut run = Vec::new();
        while at < findings.len() && &findings[at].code == code && place_of(&findings[at]).is_some() {
            run.push(&findings[at]);
            at += 1;
        }
        run.sort_by_key(|f| place_of(f));
        ordered.extend(run);
    }
    ordered
}

/// `findings` of `diagram` as the host's problems, located relative to the chart folder.
pub fn problems(diagram: &DislDiagram, findings: &[DislFinding]) -> Vec<DiagramProblem> {
    ordered(diagram, findings).into_iter().map(problem).collect()
}

fn problem(finding: &DislFinding) -> DiagramProblem {
    DiagramProblem {
        severity: match finding.severity.as_str() {
            "error" => DiagramProblemSeverity::Error,
            "warning" => DiagramProblemSeverity::Warning,
            _ => DiagramProblemSeverity::Info,
        },
        message: finding.message.clone(),
        rule_id: finding.code.clone(),
        location: finding.file.as_ref().map(|file| DiagramProblemLocation::File {
            relative_path: file.clone(),
            line: finding.line.and_then(|l| u32::try_from(l).ok()).unwrap_or(0),
        }),
    }
}

/// `problems` with each location made relative to the project at `root` rather than to the chart folder at `folder`
/// (`HelmValidator.Rebased`); the folder's own `.` becomes the folder's path.
pub fn rebased(problems: Vec<DiagramProblem>, root: &Path, folder: &Path) -> Vec<DiagramProblem> {
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
                *relative_path = if relative_path.is_empty() || relative_path == "." {
                    prefix.clone()
                } else {
                    format!("{prefix}/{relative_path}")
                };
            }
            problem
        })
        .collect()
}
