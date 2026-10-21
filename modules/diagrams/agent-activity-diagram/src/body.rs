//! One activity file, open: its text, read through the bundled FBL binding and changed only by splices that binding
//! plans (etalii.adp spec 030, task T075).
//!
//! Ported from the standalone host's `src/diagrams/agent-activity-diagram/backend/EtAlii.Adp.Diagram.AgentActivityDiagram/
//! AadBody.cs` at `8b6ccfc`. **The module has no parser and no writer of its own**: reading is `adp_fbl`'s
//! [`OpenBody`], an edit is a [`ModelChange`] the library plans as splices, and the text between the splices is never
//! touched, which is why an agent's comments and key order survive an edit made here. FBL leaves three things to the
//! host, supplied here once: the ids of the entries that store none (the root is the diagram; a locked position and a
//! group state are view data, keyed by the element they name, DISL 0.4 `persistence.view.bind`), and which attributes
//! hold a moment in time, so that one is written plain as YAML reads it.

use std::cell::OnceCell;
use std::collections::HashSet;
use std::rc::Rc;

use adp_cel::{Map, Value};
use adp_disl::model::builder;
use adp_disl::{DislModel, DislTransaction};
use adp_fbl::{FblModel, FblOptions, IdRequest, ModelChange, OpenBody, PlanResult};

use crate::definition;

/// The schema version this host knows; a file of a newer one is shown and never written.
pub const KNOWN_VERSION: i64 = 1;

/// What a new activity file holds, for the Add dialog: the header line alone. No list is written empty: the binding
/// creates each with its first entry and removes it with its last.
pub fn empty_document(line_ending: &str) -> String {
    format!("agent-activity-diagram: {KNOWN_VERSION}{line_ending}")
}

/// One activity file, read.
#[derive(Debug)]
pub struct AadBody {
    open: OpenBody,
    model: OnceCell<FblModel>,
    typed: OnceCell<DislModel>,
}

fn text_attribute(attributes: &Map, name: &str) -> String {
    match attributes.get(name) {
        Some(value) => definition::text_of(value),
        None => String::new(),
    }
}

/// The id of an entry whose rule stores none. The names are the definition's (its research note R2): the fixtures
/// list them, and the definition's operations address elements by them.
fn derive_id(request: &IdRequest<'_>) -> Option<String> {
    match request.rule {
        "diagram" => Some("diagram".to_owned()),
        "placement" => Some(format!("pinned:{}", text_attribute(request.attributes, "element"))),
        "group" => Some(format!(
            "groups:{}#{}",
            text_attribute(request.attributes, "element"),
            text_attribute(request.attributes, "group")
        )),
        _ => None,
    }
}

fn options() -> Rc<FblOptions> {
    // The attributes the definition types as a date-time, as `rule.attribute`.
    let time_attributes: HashSet<String> = ["task.updated", "pullRequest.updated"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    Rc::new(FblOptions {
        file_name: "body.aad".to_owned(),
        derive_id: Some(Rc::new(derive_id)),
        time_attributes,
        ..FblOptions::default()
    })
}

impl AadBody {
    /// Reads `text` as an activity file. Never fails for what the file holds: what cannot be read is a finding.
    pub fn parse(text: &str) -> AadBody {
        let binding = definition::get().binding.clone();
        AadBody {
            open: OpenBody::open(text.as_bytes().to_vec(), binding, options()),
            model: OnceCell::new(),
            typed: OnceCell::new(),
        }
    }

    /// The file's text, exactly as it would be written.
    pub fn text(&self) -> String {
        String::from_utf8_lossy(self.open.bytes()).into_owned()
    }

    /// The entries the binding read, with the reader's own findings.
    pub fn model(&self) -> &FblModel {
        self.model.get_or_init(|| self.open.model())
    }

    /// The same entries as the definition's model: typed elements, their children and the relations derived from
    /// their keys.
    pub fn typed(&self) -> &DislModel {
        self.typed
            .get_or_init(|| builder::from(self.model(), definition::get().specification()))
    }

    /// Empty for a file the binding could read; otherwise the reader's own sentence for why it could not.
    pub fn unreadable(&self) -> String {
        if !self.typed().is_unreadable {
            return String::new();
        }
        self.model().findings.first().map_or_else(
            || "the file could not be read as an activity file".to_owned(),
            |f| f.message.clone(),
        )
    }

    /// The version the file declares, or the known one when it declares none that can be read.
    pub fn version(&self) -> i64 {
        self.model()
            .elements
            .iter()
            .find(|e| e.type_name == "Diagram")
            .and_then(|e| match e.attributes.get("version") {
                Some(Value::Int(version)) => Some(*version),
                _ => None,
            })
            .unwrap_or(KNOWN_VERSION)
    }

    /// The entries of the reader's part that name an element the file no longer has: a lock or a group state left
    /// behind when an agent removed its element.
    pub fn orphaned_view_entries(&self) -> Vec<String> {
        let model = self.model();
        let elements: HashSet<&str> = model
            .elements
            .iter()
            .filter(|e| !matches!(e.type_name.as_str(), "Placement" | "GroupState" | "Diagram"))
            .map(|e| e.id.as_str())
            .collect();
        model
            .elements
            .iter()
            .filter(|e| matches!(e.type_name.as_str(), "Placement" | "GroupState"))
            .filter(|e| match e.attributes.get("element") {
                Some(Value::String(named)) => !elements.contains(named.as_str()),
                _ => true,
            })
            .map(|e| e.id.clone())
            .collect()
    }

    /// Plans `change` as splices and applies them, or says why not.
    pub fn change(&mut self, change: &ModelChange) -> Result<(), String> {
        let result = self.open.change(change);
        self.model = OnceCell::new();
        self.typed = OnceCell::new();
        match result {
            PlanResult::Planned(_) => Ok(()),
            PlanResult::Refused(reason) | PlanResult::Invalid(reason) => Err(reason),
        }
    }

    /// Writes a transaction's changes into the body, stopping at the first the binding refuses.
    pub fn apply(&mut self, transaction: &DislTransaction) -> Result<(), String> {
        if let Some(refusal) = &transaction.refusal {
            return Err(refusal.clone());
        }
        let definition = definition::get();
        for change in &transaction.changes {
            let written = adp_disl::behavior::write::to_fbl(definition.specification(), change);
            self.change(&written)?;
        }
        Ok(())
    }
}
