//! The findings of a dependency graph as the tool reports them (the standalone host's `DependencyGraphFindings.cs`,
//! `Judge` and `Problem`, and `DependencyGraphRules.cs` at `8b6ccfc`): the specification's constraints, evaluated by
//! the DISL runtime, with three things the specification cannot state.
//!
//! - **The id an element is written with.** A node or a dependency is named by the binding's `storedId`, which need
//!   be neither present nor unique, so `self.id` reads it, the missing-id rule sees an empty id and a later holder of
//!   an id is reported as a duplicate (DISL section 11.5.4).
//! - **The order.** First the ids, a missing and a duplicate id interleaved in declaration order, then per dependency
//!   its dangling source, its dangling target and its self-dependency: `constraints.order` sorts by code and cannot
//!   interleave two.
//! - **Where.** A missing or duplicate id, and a body that is not YAML, at its line, since such an element cannot be
//!   named by its id; everything else on its dependency. A repeated key, the one finding of the binding's own that is
//!   passed on, is a warning at its line before the tool's own findings.

use std::rc::Rc;

use adp_cel::Value;
use adp_diagram::{
    DiagramDocument, DiagramProblem, DiagramProblemLocation, DiagramProblemSeverity, DislDiagramDefinition,
};
use adp_disl::derivation::constraints;
use adp_disl::{DislConstraintOptions, DislElement};
use adp_fbl::model::codes;

/// A relation naming an element that does not exist.
pub const DANGLING_RELATION: &str = "dependencies.dangling-relation";

/// Two declarations sharing one id, which makes every reference to it ambiguous.
pub const DUPLICATE_ID: &str = "dependencies.duplicate-id";

/// An element with no id at all, which nothing can select, connect or edit.
pub const MISSING_ID: &str = "dependencies.missing-id";

/// A node declared to depend on itself.
pub const SELF_DEPENDENCY: &str = "dependencies.self-dependency";

/// A body that is not YAML.
pub const UNPARSEABLE: &str = "dependencies.unparseable";

/// The id `element` is written with: the binding's `storedId`, empty for an entry without one.
pub fn written_id(element: &DislElement) -> String {
    element
        .host_attributes()
        .get("storedId")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// The group a finding of `code` is reported in: the ids, then the dependencies' ends.
fn group(code: &str) -> usize {
    match code {
        MISSING_ID | DUPLICATE_ID => 0,
        DANGLING_RELATION | SELF_DEPENDENCY => 1,
        _ => usize::MAX,
    }
}

/// The problems of `document`, evaluated from the session's `options`, grouped and located as the tool reports them.
pub fn judge(document: &DiagramDocument, mut options: DislConstraintOptions) -> Vec<DiagramProblem> {
    options.written_id = Some(Rc::new(written_id));
    let mut findings: Vec<_> = constraints::evaluate(
        document.tool().definition.specification(),
        &document.model().diagram,
        &options,
    )
    .into_iter()
    .enumerate()
    .collect();
    findings.sort_by_key(|(arising, finding)| (group(&finding.code), finding.line.unwrap_or(i32::MAX), *arising));
    let repeated_keys = document
        .fbl()
        .findings
        .iter()
        .filter(|f| f.code == codes::DUPLICATE_KEY)
        .map(|f| DiagramProblem {
            severity: DiagramProblemSeverity::Warning,
            message: f.message.clone(),
            rule_id: f.code.clone(),
            location: Some(DiagramProblemLocation::Line(
                f.location.as_ref().map(|l| l.line).unwrap_or(1).max(1) as u32,
            )),
        });
    repeated_keys
        .chain(findings.into_iter().map(|(_, finding)| {
            let location = if matches!(finding.code.as_str(), MISSING_ID | DUPLICATE_ID | UNPARSEABLE) {
                DiagramProblemLocation::Line(finding.line.unwrap_or(1).max(1) as u32)
            } else {
                DiagramProblemLocation::Element(finding.element_ids.first().cloned().unwrap_or_default())
            };
            DiagramProblem {
                severity: DislDiagramDefinition::severity_of(&finding),
                message: finding.message,
                rule_id: finding.code,
                location: Some(location),
            }
        }))
        .collect()
}
