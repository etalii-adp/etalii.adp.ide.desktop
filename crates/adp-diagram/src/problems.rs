//! What is wrong with a diagram, as its tool type's rules say (etalii.adp spec 030, FR-019; data-model.md,
//! Finding); ported from the standalone host's `EtAlii.Adp.Diagram` at `8b6ccfc`: `IDiagramValidator.cs`,
//! `DiagramValidators.cs` and `_Model/DiagramProblem*.cs` and `DiagramValidationRequest.cs`.
//!
//! A tool type has rules by registering a validator for its origin; one without is a tool type without rules, which
//! is legitimate. Two validators for one origin are a deployment error, refused rather than one of them picked.

use std::collections::HashMap;
use std::path::PathBuf;

/// How bad a problem is. The order is load-bearing: "no findings above info" is `severity > Info`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiagramProblemSeverity {
    /// Worth saying, but nothing is wrong. Not counted as a problem.
    Info,
    /// Worth attention, but the document still means something.
    Warning,
    /// The document is wrong: it cannot be trusted until this is fixed.
    Error,
}

/// Where a problem sits, when its rule can say; none means the diagram's own file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagramProblemLocation {
    /// One element, by its id.
    Element(String),
    /// One line of the document text, 1-based.
    Line(u32),
    /// Another file, relative to the project root, at a 1-based line or `0` for the whole file: the case of a
    /// diagram whose subject is a folder of files.
    File { relative_path: String, line: u32 },
}

/// One thing wrong with one diagram.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagramProblem {
    pub severity: DiagramProblemSeverity,
    /// What is wrong, in the person's terms.
    pub message: String,
    /// The rule's stable id, prefixed with its tool type's short name (`mindmap.root-missing`).
    pub rule_id: String,
    pub location: Option<DiagramProblemLocation>,
}

/// Everything a validator is given about the one diagram it judges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagramValidationRequest {
    /// The body as read; for a tool type without a body of its own, the registration's text.
    pub document: String,
    /// The diagram's file base name, for messages that name it.
    pub base_name: String,
    pub root_path: PathBuf,
    pub body_path: PathBuf,
    /// The registration it was routed through, or none for a body opened without one.
    pub registration_path: Option<PathBuf>,
    /// The folder a folder-subject tool type reads; none for a document subject.
    pub subject_folder: Option<PathBuf>,
}

/// Judges the documents of one tool type.
pub trait DiagramValidator {
    /// The origin of the tool type whose documents this judges.
    fn origin(&self) -> &str;

    /// What the validator is called, for the refusal of two validators for one origin.
    fn name(&self) -> &str;

    /// The version of the rules, so a release of a tool type's rules invalidates only its own cached verdicts.
    fn rules_version(&self) -> String;

    /// The problems in the diagram the request describes, or none.
    fn validate(&self, request: &DiagramValidationRequest) -> Vec<DiagramProblem>;
}

/// The registered validators, by origin.
#[derive(Default)]
pub struct DiagramValidators {
    by_origin: HashMap<String, Box<dyn DiagramValidator>>,
}

impl DiagramValidators {
    /// The validators, or the refusal of two that claim one origin, naming both.
    pub fn new(validators: Vec<Box<dyn DiagramValidator>>) -> Result<Self, String> {
        let mut by_origin: HashMap<String, Box<dyn DiagramValidator>> = HashMap::new();
        for validator in validators {
            if let Some(existing) = by_origin.get(validator.origin()) {
                return Err(format!(
                    "Two validators are registered for '{}': {} and {}.",
                    validator.origin(),
                    existing.name(),
                    validator.name()
                ));
            }
            by_origin.insert(validator.origin().to_owned(), validator);
        }
        Ok(DiagramValidators { by_origin })
    }

    /// The validator for `origin`, when its tool type has rules at all.
    pub fn get(&self, origin: &str) -> Option<&dyn DiagramValidator> {
        self.by_origin.get(origin).map(|v| v.as_ref())
    }

    /// The version of the rules that judge `origin`'s documents; empty when it has none.
    pub fn rules_version(&self, origin: &str) -> String {
        self.get(origin).map(|v| v.rules_version()).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Stub(&'static str, &'static str);

    impl DiagramValidator for Stub {
        fn origin(&self) -> &str {
            self.0
        }
        fn name(&self) -> &str {
            self.1
        }
        fn rules_version(&self) -> String {
            "1.2.3".into()
        }
        fn validate(&self, _: &DiagramValidationRequest) -> Vec<DiagramProblem> {
            Vec::new()
        }
    }

    #[test]
    fn the_validator_registered_for_an_origin_is_found() {
        let validators = DiagramValidators::new(vec![Box::new(Stub("freeplane/mindmap", "Stub"))]).unwrap();
        assert_eq!(validators.get("freeplane/mindmap").map(|v| v.name()), Some("Stub"));
    }

    #[test]
    fn an_origin_without_rules_is_silent() {
        let validators = DiagramValidators::new(vec![Box::new(Stub("freeplane/mindmap", "Stub"))]).unwrap();
        assert!(validators.get("uml/class").is_none());
    }

    #[test]
    fn two_validators_claiming_one_origin_are_refused_naming_both() {
        let refusal = DiagramValidators::new(vec![
            Box::new(Stub("freeplane/mindmap", "StubValidator")),
            Box::new(Stub("freeplane/mindmap", "OtherStubValidator")),
        ])
        .err()
        .expect("refused");
        assert!(refusal.contains("freeplane/mindmap"), "{refusal}");
        assert!(
            refusal.contains("StubValidator") && refusal.contains("OtherStubValidator"),
            "{refusal}"
        );
    }

    #[test]
    fn the_rules_version_is_the_validators_own_and_empty_without_one() {
        let validators = DiagramValidators::new(vec![Box::new(Stub("freeplane/mindmap", "Stub"))]).unwrap();
        assert_eq!(validators.rules_version("freeplane/mindmap"), "1.2.3");
        assert_eq!(DiagramValidators::default().rules_version("freeplane/mindmap"), "");
    }

    #[test]
    fn info_orders_below_warning_and_error() {
        assert!(DiagramProblemSeverity::Info < DiagramProblemSeverity::Warning);
        assert!(DiagramProblemSeverity::Warning < DiagramProblemSeverity::Error);
    }
}
