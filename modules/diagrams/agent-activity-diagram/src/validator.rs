//! What an activity file has to say for itself in the Errors and Warnings panel (etalii.adp spec 030, task T075): what
//! the definition's constraints find, and what the reader could not read.
//!
//! Ported from the standalone host's `src/diagrams/agent-activity-diagram/backend/EtAlii.Adp.Diagram.AgentActivityDiagram/
//! AadProviders.cs` (`AadValidator`) at `8b6ccfc`. A file that breaks a rule still opens and draws; nothing here
//! refuses anything. A reference to something the file does not have and an id used twice break the file's own rules
//! and are errors; an entry an agent wrote without an id is shown and merely mentioned; a file of a newer version is an
//! error, since nothing can be changed in it. A lock or a group state whose element is gone is removed on the next
//! write and not reported, and a key the diagram does not read is kept and mentioned once.

use std::collections::HashSet;

use adp_cel::Value;
use adp_diagram::{DiagramProblem, DiagramProblemLocation, DiagramProblemSeverity, DislDiagramDefinition};
use adp_disl::DislConstraintOptions;
use adp_disl::derivation::constraints;
use adp_fbl::{Finding, FindingSeverity};

use crate::body::{AadBody, KNOWN_VERSION};
use crate::definition;

const DANGLING_REFERENCE: &str = "fbl.dangling-reference";
const HEADER_MISMATCH: &str = "fbl.header-mismatch";

fn line(number: usize) -> Option<DiagramProblemLocation> {
    Some(DiagramProblemLocation::Line(
        u32::try_from(number.max(1)).unwrap_or(u32::MAX),
    ))
}

/// The findings of one body, the constraints' first and then the reader's, each on the line it is about.
pub fn validate(body: &AadBody) -> Vec<DiagramProblem> {
    let definition = definition::get();
    let options = DislConstraintOptions {
        env: Some(definition.env()),
        ..DislConstraintOptions::default()
    };
    let mut problems: Vec<DiagramProblem> =
        constraints::evaluate(definition.specification(), &body.typed().diagram, &options)
            .iter()
            .map(|finding| DiagramProblem {
                severity: DislDiagramDefinition::severity_of(finding),
                message: finding.message.clone(),
                rule_id: finding.code.clone(),
                location: line(finding.line.and_then(|l| usize::try_from(l).ok()).unwrap_or(1)),
            })
            .collect();
    let orphans: HashSet<String> = body.orphaned_view_entries().into_iter().collect();
    for finding in body
        .model()
        .findings
        .iter()
        .filter(|f| !is_about_an_orphan(body, &orphans, f))
    {
        let message = if finding.code == HEADER_MISMATCH && body.version() > KNOWN_VERSION {
            format!(
                "This file was written for version {} of the activity file. It is shown, and it is not changed.",
                body.version()
            )
        } else {
            finding.message.clone()
        };
        problems.push(DiagramProblem {
            severity: severity_of(body, finding),
            message,
            rule_id: finding.code.clone(),
            location: line(finding.location.as_ref().map_or(1, |l| l.line)),
        });
    }
    // Keys the diagram does not read: kept on every write, and mentioned once each.
    for element in &body.model().elements {
        let Some(Value::List(keys)) = element.attributes.get("unknownKeys") else {
            continue;
        };
        for key in keys.iter() {
            if let Value::String(key) = key {
                problems.push(DiagramProblem {
                    severity: DiagramProblemSeverity::Info,
                    message: format!("The key \"{key}\" is not one the diagram reads. It is kept as it is."),
                    rule_id: "aad.unknown-key".to_owned(),
                    location: line(element.line),
                });
            }
        }
    }
    problems
}

/// What a reader's finding weighs here.
fn severity_of(body: &AadBody, finding: &Finding) -> DiagramProblemSeverity {
    match finding.code.as_str() {
        DANGLING_REFERENCE | "std.duplicateId" => DiagramProblemSeverity::Error,
        "std.missingId" => DiagramProblemSeverity::Info,
        HEADER_MISMATCH if body.version() > KNOWN_VERSION => DiagramProblemSeverity::Error,
        _ => match finding.severity {
            FindingSeverity::Error => DiagramProblemSeverity::Error,
            FindingSeverity::Info => DiagramProblemSeverity::Info,
            FindingSeverity::Warning => DiagramProblemSeverity::Warning,
        },
    }
}

/// A lock or a group state whose element is gone is removed on the next write and not reported.
fn is_about_an_orphan(body: &AadBody, orphans: &HashSet<String>, finding: &Finding) -> bool {
    if finding.code != DANGLING_REFERENCE {
        return false;
    }
    let Some(location) = &finding.location else {
        return false;
    };
    body.model()
        .elements
        .iter()
        .any(|e| orphans.contains(&e.id) && location.line >= e.line && location.line < e.line + 4)
}
