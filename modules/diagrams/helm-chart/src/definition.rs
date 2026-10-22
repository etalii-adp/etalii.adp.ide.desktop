//! The Helm chart diagram's DISL definition and its binding, each loaded once, and what is derived from them: the model
//! of a reading, the property rows of every node and relation, the findings and the sentence a write is refused with
//! (etalii.adp spec 030, task T080).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/helm-chart/backend/EtAlii.Adp.Diagram.HelmChart/
//! HelmDefinition.cs`. The definition is the vendored `definitions/diagrams/helm-chart.dis`, byte for byte the one the
//! standalone host bundles, and the binding is the module's own `helm-chart.fbl`, vendored from the standalone host.
//! Everything is read-only (`env.readOnly`): every row carries the reason its form item gives, which names the file the
//! value is defined in, and a write is refused with the definition's `std.readOnly`.

use std::rc::Rc;

use adp_cel::Value;
use adp_context::{PropertyEditor, PropertyRow};
use adp_disl::derivation::constraints;
use adp_disl::derivation::forms;
use adp_disl::model::builder;
use adp_disl::{
    DislConstraintOptions, DislDiagram, DislEnv, DislFinding, DislLoader, DislModel, DislPluginFunctions,
    DislReaderFinding, DislSpecification, WireIdMap,
};
use adp_fbl::{FblBinding, FblModel, Finding};

/// The vendored definition, as the standalone host bundles it.
const DEFINITION: &[u8] = include_bytes!("../../../../vendor/etalii.adp/definitions/diagrams/helm-chart.dis");

/// The vendored binding, the standalone host's module's own.
const BINDING: &[u8] = include_bytes!(
    "../../../../vendor/etalii.adp.ide.standalone/src/diagrams/helm-chart/backend/EtAlii.Adp.Diagram.HelmChart/helm-chart.fbl"
);

/// The binding's name in its document.
const BINDING_NAME: &str = "chart";

/// The definition, its binding and its wire ids.
#[derive(Debug, Clone)]
pub struct HelmDefinition {
    pub specification: DislSpecification,
    pub binding: Rc<FblBinding>,
    pub ids: WireIdMap,
}

thread_local! {
    static LOADED: HelmDefinition = HelmDefinition::load();
}

impl HelmDefinition {
    /// The definition, loaded once per thread.
    pub fn get() -> HelmDefinition {
        LOADED.with(Clone::clone)
    }

    fn load() -> HelmDefinition {
        let loaded = DislLoader::load_bytes(DEFINITION, &DislPluginFunctions::none());
        let specification = loaded
            .specification
            .unwrap_or_else(|| panic!("the vendored helm-chart.dis loads: {:?}", loaded.diagnostics));
        let (problems, document) = adp_fbl::documents::loader::load(BINDING);
        let binding = document
            .and_then(|d| d.binding(BINDING_NAME).cloned())
            .unwrap_or_else(|| panic!("the vendored helm-chart.fbl loads: {problems:?}"));
        let ids = WireIdMap::of(&specification, "x-helm");
        HelmDefinition {
            specification,
            binding: Rc::new(binding),
            ids,
        }
    }

    /// What `env` reads: nothing in this diagram can be edited.
    pub fn env() -> DislEnv {
        DislEnv {
            read_only: true,
            ..DislEnv::default()
        }
    }

    /// The model of a reading through the binding, with the three relations that always have both ends derived.
    pub fn model(&self, reading: &FblModel) -> DislModel {
        builder::from(reading, &self.specification)
    }

    /// Why nothing here can be written: the definition's `std.readOnly`.
    pub fn write_refusal(&self) -> String {
        self.specification
            .root()
            .get("behavior")
            .and_then(|b| b.get("messages"))
            .and_then(|m| m.get("std.readOnly"))
            .and_then(|m| m.as_str())
            .unwrap_or_default()
            .to_owned()
    }

    /// The prefix of every wire type (`x-helm.typePrefix`).
    pub fn type_prefix(&self) -> String {
        self.specification
            .root()
            .get("x-helm")
            .and_then(|x| x.get("typePrefix"))
            .and_then(|p| p.as_str())
            .unwrap_or_default()
            .to_owned()
    }

    /// The definition's findings of `model`, with the reading's own findings `reading` as the reader's (the runtime's
    /// order; [`crate::findings`] puts them in the tool's). Only the reading's findings are handed, not those the model
    /// builder adds, so no finding appears that the tool does not give.
    pub fn findings(&self, model: &DislModel, reading: &[Finding]) -> Vec<DislFinding> {
        let options = DislConstraintOptions {
            env: Some(HelmDefinition::env()),
            reader_findings: reading.iter().map(reader_finding).collect(),
            written_id: None,
        };
        constraints::evaluate(&self.specification, &model.diagram, &options)
    }

    /// The property rows of the element `element_id` names in `diagram`: its form's; none for the empty id and for an
    /// id the model does not hold.
    pub fn rows(&self, diagram: &DislDiagram, element_id: &str) -> Vec<PropertyRow> {
        if element_id.is_empty() {
            return Vec::new();
        }
        let Some(element) = diagram.element_by_id(element_id) else {
            return Vec::new();
        };
        forms::derive(
            &self.specification,
            &element,
            &HelmDefinition::env(),
            &self.ids,
            "inspector",
        )
        .into_iter()
        .map(|row| PropertyRow {
            id: row.id,
            label: row.label,
            value: row.value,
            editor: PropertyEditor::Line,
            read_only_reason: row.read_only_reason,
            group: row.group,
            candidates: Vec::new(),
        })
        .collect()
    }
}

/// A finding of the reading as the reader's finding of its code: its message is the reason, which the built-in gives
/// as it is, and a line of 0, which says the parser gave none, is no line.
fn reader_finding(finding: &Finding) -> DislReaderFinding {
    let file = finding
        .location
        .as_ref()
        .map(|l| l.file.as_str())
        .filter(|f| !f.is_empty());
    let line = finding
        .location
        .as_ref()
        .filter(|l| l.line > 0)
        .and_then(|l| i32::try_from(l.line).ok());
    let mut reader = DislReaderFinding::not_parsed(&finding.message, line, file);
    reader.built_in = finding.code.clone();
    if let Some(detail) = &finding.detail {
        for (key, value) in detail {
            reader.detail.insert(key.clone(), Value::String(value.clone()));
        }
    }
    reader.detail.insert("reason", Value::String(finding.message.clone()));
    reader
}

/// A string attribute as the model reads it: stored, else its default; empty for any other value.
pub(crate) fn text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        _ => String::new(),
    }
}
