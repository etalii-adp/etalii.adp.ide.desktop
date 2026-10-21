//! The Ansible structure diagram's DISL definition and its binding, each loaded once, and what is derived from them:
//! the model of a reading, the property rows of every node and relation, the findings, the sentence a reparenting move
//! is refused with and the one a property write is refused with (etalii.adp spec 030, task T079).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/ansible-structure/backend/
//! EtAlii.Adp.Diagram.AnsibleStructure/AnsibleDefinition.cs` and `AnsibleInventoryRows.cs`. The definition is the
//! vendored `definitions/diagrams/ansible-structure.dis`, byte for byte the one the standalone host bundles, and the
//! binding is the module's own `ansible-structure.fbl`, vendored from the standalone host. Everything is read-only
//! (`env.readOnly`): every row carries the reason its form item gives, which names the file the value is defined in. An
//! inventory's groups and variable folders are one row per item of a list, which a form item cannot state, so they are
//! written here after the rows its form derives (gap 7 of `ansible-structure.md`).

use std::rc::Rc;

use adp_cel::Value;
use adp_context::{PropertyEditor, PropertyRow};
use adp_disl::derivation::constraints;
use adp_disl::derivation::forms;
use adp_disl::model::builder;
use adp_disl::{
    DislConstraintOptions, DislDiagram, DislElement, DislEnv, DislFinding, DislLoader, DislModel, DislPluginFunctions,
    DislReaderFinding, DislSpecification, WireIdMap,
};
use adp_fbl::{FblBinding, FblModel, Finding};

/// The vendored definition, as the standalone host bundles it.
const DEFINITION: &[u8] = include_bytes!("../../../../vendor/etalii.adp/definitions/diagrams/ansible-structure.dis");

/// The vendored binding, the standalone host's module's own.
const BINDING: &[u8] = include_bytes!(
    "../../../../vendor/etalii.adp.ide.standalone/src/diagrams/ansible-structure/backend/EtAlii.Adp.Diagram.AnsibleStructure/ansible-structure.fbl"
);

/// The binding's name in its document.
const BINDING_NAME: &str = "ansible";

/// The definition, its binding and its wire ids.
#[derive(Debug, Clone)]
pub struct AnsibleDefinition {
    pub specification: DislSpecification,
    pub binding: Rc<FblBinding>,
    pub ids: WireIdMap,
}

thread_local! {
    static LOADED: AnsibleDefinition = AnsibleDefinition::load();
}

impl AnsibleDefinition {
    /// The definition, loaded once per thread.
    pub fn get() -> AnsibleDefinition {
        LOADED.with(Clone::clone)
    }

    fn load() -> AnsibleDefinition {
        let loaded = DislLoader::load_bytes(DEFINITION, &DislPluginFunctions::none());
        let specification = loaded
            .specification
            .unwrap_or_else(|| panic!("the vendored ansible-structure.dis loads: {:?}", loaded.diagnostics));
        let (problems, document) = adp_fbl::documents::loader::load(BINDING);
        let binding = document
            .and_then(|d| d.binding(BINDING_NAME).cloned())
            .unwrap_or_else(|| panic!("the vendored ansible-structure.fbl loads: {problems:?}"));
        let ids = WireIdMap::of(&specification, "x-ansible");
        AnsibleDefinition {
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

    /// The model of a reading through the binding.
    pub fn model(&self, reading: &FblModel) -> DislModel {
        builder::from(reading, &self.specification)
    }

    /// Why no property can be written: the binding's `readOnly`.
    pub fn write_refusal(&self) -> String {
        self.binding.read_only.clone().unwrap_or_default()
    }

    /// Why nothing on the diagram can be moved under another element: the message of the definition's one
    /// containment rule.
    pub fn move_refusal(&self) -> String {
        self.specification
            .root()
            .get("constraints")
            .and_then(|c| c.get("rules"))
            .and_then(|r| r.as_array())
            .into_iter()
            .flatten()
            .find(|rule| rule.get("kind").and_then(|k| k.as_str()) == Some("containment"))
            .and_then(|rule| rule.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or_default()
            .to_owned()
    }

    /// The label the definition gives the value of the enumerated `attribute` of `element`, which are the words a
    /// file spells a directive with; the value itself when the enumeration words none.
    pub fn label(&self, element: &DislElement, attribute: &str) -> String {
        let value = text(&element.value_of(attribute));
        let metamodel = self.specification.metamodel();
        element
            .element_type()
            .attributes
            .get(attribute)
            .and_then(|a| metamodel.enums.get(&a.type_name))
            .and_then(|e| e.value_of(&value))
            .map(|v| v.label.clone().unwrap_or_else(|| v.key.clone()))
            .unwrap_or(value)
    }

    /// The findings of `model`: every finding of its reading as the reader's, then the definition's rules.
    pub fn findings(&self, model: &DislModel) -> Vec<DislFinding> {
        let options = DislConstraintOptions {
            env: Some(AnsibleDefinition::env()),
            reader_findings: model.findings.iter().map(reader_finding).collect(),
            written_id: None,
        };
        constraints::evaluate(&self.specification, &model.diagram, &options)
    }

    /// The property rows of the element `element_id` names in `diagram`: its form's, and an inventory's groups and
    /// variable folders; none for an id the model does not hold.
    pub fn rows(&self, diagram: &DislDiagram, element_id: &str) -> Vec<PropertyRow> {
        let Some(element) = diagram.element_by_id(element_id) else {
            return Vec::new();
        };
        let mut rows: Vec<PropertyRow> = forms::derive(
            &self.specification,
            &element,
            &AnsibleDefinition::env(),
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
        .collect();
        rows.extend(inventory_rows(&element));
        rows
    }
}

/// A finding of the reading as the reader's finding of its code: its message is the reason, and a line of 0 is no
/// line.
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
        reader.detail.insert("reason", Value::String(finding.message.clone()));
    }
    reader
}

pub(crate) fn text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        _ => String::new(),
    }
}

fn items(element: &DislElement, attribute: &str) -> Vec<Value> {
    match element.value_of(attribute) {
        Value::List(items) => items.as_ref().clone(),
        _ => Vec::new(),
    }
}

fn field(item: &Value, name: &str) -> Value {
    match item {
        Value::Map(map) => map.get(name).cloned().unwrap_or_default(),
        _ => Value::Null,
    }
}

fn count(item: &Value, name: &str) -> i64 {
    match field(item, name) {
        Value::Int(n) => n,
        _ => 0,
    }
}

/// The section the attribute declares its rows in.
fn group_of(element: &DislElement, attribute: &str) -> String {
    element
        .element_type()
        .attributes
        .get(attribute)
        .and_then(|a| a.json.get("group"))
        .and_then(|g| g.as_str())
        .unwrap_or_default()
        .to_owned()
}

fn read_only_row(id: String, label: String, value: String, source: &str, group: &str) -> PropertyRow {
    PropertyRow {
        id,
        label,
        value,
        editor: PropertyEditor::Line,
        read_only_reason: format!("Defined in {source}; edit it in a text editor."),
        group: group.to_owned(),
        candidates: Vec::new(),
    }
}

/// The rows of an inventory's groups, then of its variable folders (`AnsibleInventoryRows.Of`); none for an element
/// that is not an inventory.
fn inventory_rows(inventory: &DislElement) -> Vec<PropertyRow> {
    let attributes = &inventory.element_type().attributes;
    if !attributes.contains_key("groups") || !attributes.contains_key("variableFolders") {
        return Vec::new();
    }
    let path = text(&inventory.value_of("path"));
    let mut rows = Vec::new();
    let groups = group_of(inventory, "groups");
    for group in items(inventory, "groups") {
        let name = text(&field(&group, "name"));
        let hosts = count(&group, "hostCount");
        let value = if hosts == 1 {
            "1 host".to_owned()
        } else {
            format!("{hosts} hosts")
        };
        rows.push(read_only_row(
            format!("ansible.group.{name}"),
            name,
            value,
            &path,
            &groups,
        ));
    }
    let variables = group_of(inventory, "variableFolders");
    for folder in items(inventory, "variableFolders") {
        let name = text(&field(&folder, "name"));
        let files = count(&folder, "fileCount");
        let value = if files == 1 {
            "1 file".to_owned()
        } else {
            format!("{files} files")
        };
        let source = text(&field(&folder, "path"));
        rows.push(read_only_row(
            format!("ansible.vars.{name}"),
            name,
            value,
            &source,
            &variables,
        ));
    }
    rows
}
