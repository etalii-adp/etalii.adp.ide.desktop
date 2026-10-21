//! Which C4 rule mirrors one of Structurizr's inspector rules, which is ADP's own, and which of Structurizr's ADP does
//! not implement, each with its reason (etalii.adp spec 030, task T061).
//!
//! Ported from the standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4/C4StructurizrMirror.cs` at
//! `8b6ccfc`. The reconciliation against the recorded Structurizr verdicts compares only the mirrored rules, so this
//! table accounts for every rule id: adding a rule forces the decision rather than deferring it.

use crate::rules::codes;

/// Each ADP rule that mirrors Structurizr rules, with the rules it mirrors (one per element kind on Structurizr's side
/// where ADP asks one question).
pub const MIRRORED_RULES: [(&str, &[&str]); 9] = [
    (
        codes::MISSING_DESCRIPTION,
        &[
            "model.person.description",
            "model.softwaresystem.description",
            "model.container.description",
            "model.component.description",
        ],
    ),
    (codes::MISSING_TECHNOLOGY, &["model.container.technology"]),
    (codes::MISSING_PROTOCOL, &["model.relationship.technology"]),
    (
        codes::MISSING_DEPLOYMENT_DESCRIPTION,
        &["model.deploymentnode.description"],
    ),
    (
        codes::MISSING_DEPLOYMENT_TECHNOLOGY,
        &["model.deploymentnode.technology"],
    ),
    (
        codes::MISSING_INFRASTRUCTURE_DESCRIPTION,
        &["model.infrastructurenode.description"],
    ),
    (
        codes::MISSING_INFRASTRUCTURE_TECHNOLOGY,
        &["model.infrastructurenode.technology"],
    ),
    (codes::DISCONNECTED_ELEMENT, &["model.element.disconnected"]),
    (codes::ELEMENT_NOT_ON_ANY_VIEW, &["model.element.noview"]),
];

/// Each rule that is ADP's own, with why Structurizr has nothing to compare it with.
pub const ADP_ONLY_RULES: [(&str, &str); 7] = [
    (
        codes::UNLABELLED_RELATIONSHIP,
        "C4 requires every relationship to be labelled. Structurizr inspects for a technology but not for a description.",
    ),
    (
        codes::DANGLING_RELATIONSHIP,
        "An identifier that resolves to nothing. Structurizr's parser refuses the document outright, so its inspector never sees one.",
    ),
    (
        codes::KIND_NOT_PERMITTED,
        "What a view of each kind may draw. An editing-surface rule: it stops a user putting a component on a context diagram.",
    ),
    (
        codes::EMPTY_VIEW,
        "A view that would draw nothing. Structurizr has the inverse - an element on no view - which ADP now mirrors separately.",
    ),
    (
        codes::UNKNOWN_VIEW_SCOPE,
        "A view scoped to something the model does not declare. Refused by Structurizr's parser rather than reported by its inspector.",
    ),
    (
        codes::MISPLACED_ELEMENT,
        "A component outside a container, or a container outside a software system. Refused by Structurizr's parser rather than inspected.",
    ),
    (
        codes::INCLUDE_NOT_FOLLOWED,
        "ADP does not follow `!include`, and says so rather than silently reading half a model. Structurizr follows it, so it has nothing to report.",
    ),
];

/// Each Structurizr rule ADP does not implement, with why.
pub const NOT_IMPLEMENTED_RULES: [(&str, &str); 3] = [
    (
        "workspace.scope",
        "A Structurizr workspace-metadata concept. ADP writes one workspace per document and has nothing to set a scope from, so mirroring it would produce a warning a user could not act on.",
    ),
    (
        "model.softwaresystem.documentation",
        "Depends on `!docs`, which ADP round-trips but does not read. A documentation model would have to exist first.",
    ),
    (
        "model.softwaresystem.decisions",
        "Depends on `!adrs`, likewise. ADP carries the directive and the folder untouched and has no notion of a decision record.",
    ),
];

/// The Structurizr rules `rule` mirrors, or none when it mirrors none.
pub fn mirrored(rule: &str) -> Option<&'static [&'static str]> {
    MIRRORED_RULES
        .iter()
        .find(|(id, _)| *id == rule)
        .map(|(_, rules)| *rules)
}

/// Whether `rule` is recorded as ADP's own.
pub fn is_adp_only(rule: &str) -> bool {
    ADP_ONLY_RULES.iter().any(|(id, _)| *id == rule)
}

/// The ADP rule that mirrors Structurizr rule `structurizr_rule`, or none.
pub fn adp_rule_mirroring(structurizr_rule: &str) -> Option<&'static str> {
    MIRRORED_RULES
        .iter()
        .find(|(_, rules)| rules.contains(&structurizr_rule))
        .map(|(id, _)| *id)
}

/// Every Structurizr rule some ADP rule mirrors, each once, sorted.
pub fn mirrored_structurizr_rules() -> Vec<&'static str> {
    let mut rules: Vec<&str> = MIRRORED_RULES
        .iter()
        .flat_map(|(_, rules)| rules.iter().copied())
        .collect();
    rules.sort_unstable();
    rules.dedup();
    rules
}
