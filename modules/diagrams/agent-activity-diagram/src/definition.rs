//! The bundled definition, loaded once: the DISL specification this module derives from, its FBL binding, and the
//! few things the module reads off the specification by name (etalii.adp spec 030, task T075).
//!
//! Ported from the standalone host's `src/diagrams/agent-activity-diagram/backend/EtAlii.Adp.Diagram.AgentActivityDiagram/
//! AadDefinition.cs` at `8b6ccfc`. The standalone host bundled `definition/agent-activity-diagram.dis` and
//! `definition/agent-activity-diagram.fbl` into its module as resources; they are the files etalii.adp publishes, byte
//! for byte the ones vendored here under `vendor/etalii.adp/definitions/diagrams/`, which this module embeds the same
//! way. **What a status is called, which statuses there are and in what order are the definition's**: the module reads
//! them here and never writes them down a second time.

use std::collections::BTreeSet;
use std::rc::Rc;

use adp_cel::Value;
use adp_context::{PropertyEditor, PropertyRow};
use adp_diagram::DislDiagramDefinition;
use adp_disl::{
    DerivedMenuEntry, DislDiagram, DislElement, DislEnv, DislMenuTarget, DislPluginFunctions, DislSpecification,
};
use adp_fbl::FblBinding;
use serde_json::Value as Json;

const SPECIFICATION: &[u8] =
    include_bytes!("../../../../vendor/etalii.adp/definitions/diagrams/agent-activity-diagram.dis");
const BINDING: &[u8] = include_bytes!("../../../../vendor/etalii.adp/definitions/diagrams/agent-activity-diagram.fbl");

/// The specification's block of wire ids.
const WIRE_BLOCK: &str = "x-aad";

/// The key of a location's one group of rows, in the file and on the wire.
pub const PULL_REQUESTS_GROUP: &str = "pullRequests";

/// What a relation to the wrong kind of element, or an element that is gone, answers (`AadEdits.Gone`).
pub const GONE: &str = "That is no longer in this diagram.";

/// One member of an enum: its name in the model, how the file writes it, and the words it is shown with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AadEnumMember {
    pub name: String,
    pub stored: String,
    pub label: String,
}

/// One enum of the definition: its members in declared order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AadEnum {
    pub members: Vec<AadEnumMember>,
}

impl AadEnum {
    /// The member whose name or stored form is `text`, or none for a value the definition does not have.
    pub fn find(&self, text: &str) -> Option<&AadEnumMember> {
        self.members.iter().find(|m| m.name == text || m.stored == text)
    }
}

/// One of the four relations: the two types it joins, and the key on one of them that states it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AadRelationType {
    pub type_name: String,
    pub source: String,
    pub target: String,
    pub attribute: String,
    pub held_by_source: bool,
}

/// A relation between two elements, its ends in the order the file has them.
#[derive(Debug, Clone)]
pub struct AadDrawnRelation {
    pub relation: AadRelationType,
    pub source: DislElement,
    pub target: DislElement,
}

/// The loaded definition.
#[derive(Debug)]
pub struct AadDefinition {
    pub definition: DislDiagramDefinition,
    pub binding: Rc<FblBinding>,
    enums: Vec<(String, AadEnum)>,
    relations: Vec<AadRelationType>,
    collapsed_by_default: BTreeSet<String>,
}

thread_local! {
    static LOADED: Rc<AadDefinition> = Rc::new(AadDefinition::load());
}

/// The definition, loaded the first time it is asked for on this thread. A definition that does not load is a broken
/// build of this module, not a state a user can be in, so it panics where it is first asked for.
pub fn get() -> Rc<AadDefinition> {
    LOADED.with(Rc::clone)
}

fn string_of<'a>(json: &'a Json, name: &str) -> Option<&'a str> {
    json.get(name).and_then(Json::as_str)
}

impl AadDefinition {
    fn load() -> AadDefinition {
        let definition = DislDiagramDefinition::load(SPECIFICATION, Some(WIRE_BLOCK), &DislPluginFunctions::none())
            .unwrap_or_else(|e| panic!("agent-activity-diagram.dis does not load: {e}"));
        let (problems, document) = adp_fbl::documents::load(BINDING);
        let binding = document
            .and_then(|d| d.binding("aad").cloned())
            .unwrap_or_else(|| panic!("agent-activity-diagram.fbl has no binding 'aad': {problems:?}"));
        let root = definition.specification().root().clone();
        AadDefinition {
            enums: read_enums(&definition),
            relations: read_relations(&root),
            collapsed_by_default: read_collapsed_by_default(&root),
            definition,
            binding: Rc::new(binding),
        }
    }

    pub fn specification(&self) -> &DislSpecification {
        self.definition.specification()
    }

    /// The enumeration of that name, or none for any other type.
    pub fn enum_of(&self, type_name: &str) -> Option<&AadEnum> {
        self.enums.iter().find(|(n, _)| n == type_name).map(|(_, e)| e)
    }

    fn enum_named(&self, name: &str) -> &AadEnum {
        self.enum_of(name)
            .unwrap_or_else(|| panic!("agent-activity-diagram.dis has no enum {name}"))
    }

    /// A task's statuses, in the order their groups are listed.
    pub fn task_status(&self) -> &AadEnum {
        self.enum_named("TaskStatus")
    }

    pub fn specification_status(&self) -> &AadEnum {
        self.enum_named("SpecificationStatus")
    }

    pub fn environment_kind(&self) -> &AadEnum {
        self.enum_named("EnvironmentKind")
    }

    /// The groups that are folded until the reader unfolds them: the task statuses the definition marks collapsed,
    /// and a location's pull requests.
    pub fn collapsed_by_default(&self) -> &BTreeSet<String> {
        &self.collapsed_by_default
    }

    /// What an expression reads as `env` for one gesture: the moment it was made.
    pub fn env(&self) -> DislEnv {
        DislEnv {
            now: Some(crate::now()),
            ..DislEnv::default()
        }
    }

    fn type_json(&self, name: &str) -> Option<&Json> {
        self.specification().root().get("metamodel")?.get("types")?.get(name)
    }

    /// The size the definition gives an element of a type before its lists are added: the default of its notation's
    /// `size` (DISL section 6.6).
    pub fn size_of(&self, type_name: &str) -> (f64, f64) {
        let size = self
            .specification()
            .root()
            .pointer(&format!("/notation/nodes/{type_name}/size/default"));
        match size.and_then(Json::as_array).map(Vec::as_slice) {
            Some([w, h]) => (w.as_f64().unwrap_or(160.0), h.as_f64().unwrap_or(48.0)),
            _ => (160.0, 48.0),
        }
    }

    /// The element, row or relation with that id. A relation is derived from the key that states it and has the id
    /// the definition gives it, such as `project:s-knowledge`.
    pub fn element_of(diagram: &DislDiagram, id: &str) -> Option<DislElement> {
        diagram
            .nodes()
            .into_iter()
            .find(|n| n.id() == id)
            .or_else(|| diagram.relations().into_iter().find(|r| r.id() == id))
    }

    /// The attribute a rename writes: `name`, a row's `title`, a location's `branch`.
    pub fn label_attribute(&self, element: &DislElement) -> Option<String> {
        self.type_json(&element.type_name())
            .and_then(|t| string_of(t, "labelAttribute"))
            .map(str::to_owned)
    }

    /// An element's name as its type's label attribute has it, or its id when it has none.
    pub fn name_of(&self, element: &DislElement) -> String {
        match self.label_attribute(element).map(|a| element.value_of(&a)) {
            Some(Value::String(name)) if !name.is_empty() => name,
            _ => element.id(),
        }
    }

    /// A type's name in words, lower case, as a sentence uses it: "pull request".
    fn words_of(&self, type_name: &str) -> String {
        self.type_json(type_name)
            .and_then(|t| string_of(t, "label"))
            .unwrap_or(type_name)
            .to_lowercase()
    }

    /// Whether the named relation joins an element of the one type to an element of the other, in that order.
    pub fn joins(&self, relation_type: &str, source_type: &str, target_type: &str) -> bool {
        self.relations
            .iter()
            .any(|r| r.type_name == relation_type && r.source == source_type && r.target == target_type)
    }

    /// The keys that name an element of another type than their relation joins, each as the relation it was written to
    /// be: its id as a derived relation has it, its type, and its two ends in the relation's order.
    pub fn misjoined(&self, diagram: &DislDiagram) -> Vec<(String, String, String, String)> {
        let mut found = Vec::new();
        for relation in &self.relations {
            let (holder_type, other_type) = if relation.held_by_source {
                (&relation.source, &relation.target)
            } else {
                (&relation.target, &relation.source)
            };
            for holder in diagram.nodes_of_type(holder_type) {
                let Some(other) = DislElement::from_value(&holder.value_of(&relation.attribute)) else {
                    continue;
                };
                if other.is_relation() || other.is_a(other_type) {
                    continue;
                }
                let id = format!("{}:{}", relation.attribute, holder.id());
                found.push(if relation.held_by_source {
                    (id, relation.type_name.clone(), holder.id(), other.id())
                } else {
                    (id, relation.type_name.clone(), other.id(), holder.id())
                });
            }
        }
        found
    }

    /// The one relation the two elements' types allow, with its ends in the order the file has them.
    pub fn relation_between(&self, from: &DislElement, to: &DislElement) -> Option<AadDrawnRelation> {
        for relation in &self.relations {
            if from.is_a(&relation.source) && to.is_a(&relation.target) {
                return Some(AadDrawnRelation {
                    relation: relation.clone(),
                    source: from.clone(),
                    target: to.clone(),
                });
            }
            if to.is_a(&relation.source) && from.is_a(&relation.target) {
                return Some(AadDrawnRelation {
                    relation: relation.clone(),
                    source: to.clone(),
                    target: from.clone(),
                });
            }
        }
        None
    }

    /// Why two elements cannot be joined: what the first of them can be joined to instead.
    pub fn no_relation(&self, from: &DislElement, to: &DislElement) -> String {
        if from.id() == to.id() {
            return "An element cannot be joined to itself.".to_owned();
        }
        let own = from.type_name();
        let partners: Vec<String> = self
            .relations
            .iter()
            .filter_map(|r| {
                if r.source == own {
                    Some(&r.target)
                } else if r.target == own {
                    Some(&r.source)
                } else {
                    None
                }
            })
            .map(|t| self.words_of(t))
            .collect();
        let own_words = self.words_of(&own);
        let instead = if partners.is_empty() {
            format!("{} has no relations.", a(&own_words, true))
        } else {
            format!(
                "{} is joined to {}.",
                a(&own_words, true),
                partners.iter().map(|p| a(p, false)).collect::<Vec<_>>().join(" or ")
            )
        };
        format!(
            "{} cannot be joined to {}. {instead}",
            a(&own_words, true),
            a(&self.words_of(&to.type_name()), false)
        )
    }

    /// Why a relation cannot be drawn although its ends are of the right types: the element that would hold it
    /// already names another, and each holds one.
    pub fn occupied(&self, drawn: &AadDrawnRelation) -> Option<String> {
        let (holder, other) = if drawn.relation.held_by_source {
            (&drawn.source, &drawn.target)
        } else {
            (&drawn.target, &drawn.source)
        };
        let current = DislElement::from_value(&holder.value_of(&drawn.relation.attribute))?;
        (current.id() != other.id()).then(|| {
            format!(
                "\"{}\" is already joined to \"{}\". Remove that line first: each {} has one {}.",
                self.name_of(holder),
                self.name_of(&current),
                self.words_of(&holder.type_name()),
                self.words_of(&current.type_name())
            )
        })
    }

    /// The element whose key states a drawn relation, and the key.
    pub fn holder(&self, relation: &DislElement) -> Option<(DislElement, String)> {
        let declared = self.relations.iter().find(|r| r.type_name == relation.type_name())?;
        let holder = if declared.held_by_source {
            relation.source()
        } else {
            relation.target()
        }?;
        Some((holder, declared.attribute.clone()))
    }

    /// The groups an element folds: a specification's task statuses, a location's pull requests.
    pub fn groups_of(&self, element: &DislElement) -> Vec<String> {
        if element.is_a("Specification") {
            self.task_status().members.iter().map(|m| m.name.clone()).collect()
        } else if element.is_a("Location") {
            vec![PULL_REQUESTS_GROUP.to_owned()]
        } else {
            Vec::new()
        }
    }

    /// A group's key as the file writes it: a task status in its stored word.
    pub fn stored_group(&self, group: &str) -> String {
        self.task_status()
            .find(group)
            .map_or_else(|| group.to_owned(), |m| m.stored.clone())
    }

    /// The menu the definition derives for an element, a row or a relation.
    /// The menu entry a double-click on `element` runs: the one of the kind its node names in its `doubleClick`
    /// (DISL section 6.9), `editLabel` when it names none; none for `none`, and for a type the notation draws as a row.
    pub fn double_click(&self, element: &DislElement) -> Option<String> {
        let node = self
            .definition
            .specification()
            .root()
            .get("notation")?
            .get("nodes")?
            .get(element.type_name().as_str())?;
        let named = node.get("doubleClick").and_then(|d| d.as_str()).unwrap_or("editLabel");
        self.menus(element)
            .into_iter()
            .flatten()
            .find(|entry| entry.kind == named)
            .map(|entry| entry.id)
    }

    pub fn menus(&self, element: &DislElement) -> Vec<Vec<DerivedMenuEntry>> {
        self.definition
            .menu_entries(&DislMenuTarget::Element(element.clone()), &self.env())
    }

    /// The property grid's rows for an element or a row: its type's attributes in the definition's groups and order. A
    /// reference shows the name of what it names and is changed by drawing.
    pub fn rows(&self, element: &DislElement, read_only: bool) -> Vec<PropertyRow> {
        let Some(attributes) = self
            .type_json(&element.type_name())
            .and_then(|t| t.get("attributes"))
            .and_then(Json::as_object)
        else {
            return Vec::new();
        };
        let because = if read_only {
            "The file could not be read, so nothing can be edited until it can."
        } else {
            ""
        };
        let mut rows: Vec<(String, i64, PropertyRow)> = Vec::new();
        for (name, declared) in attributes {
            let type_name = string_of(declared, "type").unwrap_or("string");
            let group = string_of(declared, "group").unwrap_or_default().to_owned();
            let order = declared.get("order").and_then(Json::as_i64).unwrap_or(0);
            let label = string_of(declared, "label").map_or_else(|| words(name), str::to_owned);
            let value = element.value_of(name);
            let mut row = if let Some(choices) = self.enum_of(type_name) {
                let text = text_of(&value);
                let shown = choices.find(&text).map_or(text, |m| m.label.clone());
                let mut row = PropertyRow::new(name, &label, PropertyEditor::Choice, shown);
                row.candidates = choices.members.iter().map(|m| m.label.clone()).collect();
                row.read_only_reason = because.to_owned();
                row
            } else if self.specification().metamodel().type_of(type_name).is_some() {
                let shown = DislElement::from_value(&value).map_or_else(|| "None".to_owned(), |e| self.name_of(&e));
                let mut row = PropertyRow::new(name, &label, PropertyEditor::Line, shown);
                row.read_only_reason = "A relation is drawn on the canvas, and removed there.".to_owned();
                row
            } else {
                let mut row = PropertyRow::new(name, &label, PropertyEditor::Line, text_of(&value));
                row.read_only_reason = because.to_owned();
                row
            };
            row.group = group.clone();
            rows.push((group, order, row));
        }
        const GROUPS: [&str; 3] = ["Identity", "Relations", "Link"];
        let rank = |group: &str| GROUPS.iter().position(|g| *g == group).unwrap_or(GROUPS.len());
        rows.sort_by(|a, b| rank(&a.0).cmp(&rank(&b.0)).then(a.1.cmp(&b.1)));
        rows.into_iter().map(|(_, _, row)| row).collect()
    }
}

/// "a project", "an agent".
fn a(words: &str, capital: bool) -> String {
    let article = if capital { "A" } else { "a" };
    let n = if words.starts_with(['a', 'e', 'i', 'o', 'u']) {
        "n "
    } else {
        " "
    };
    format!("{article}{n}{words}")
}

/// "branchLink" reads "Branch link".
fn words(name: &str) -> String {
    let mut out = String::new();
    for (index, letter) in name.chars().enumerate() {
        if index == 0 {
            out.extend(letter.to_uppercase());
        } else if letter.is_uppercase() {
            out.push(' ');
            out.extend(letter.to_lowercase());
        } else {
            out.push(letter);
        }
    }
    out
}

/// A value as the file's text holds it: a string as it is, an element by its id, a number or a truth as written.
pub fn text_of(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Bool(b) => if *b { "True" } else { "False" }.to_owned(),
        Value::Int(i) => i.to_string(),
        Value::Double(d) => d.to_string(),
        other => DislElement::from_value(other).map_or_else(|| other.to_string(), |e| e.id()),
    }
}

fn read_enums(definition: &DislDiagramDefinition) -> Vec<(String, AadEnum)> {
    definition
        .specification()
        .metamodel()
        .enums
        .iter()
        .map(|(name, declared)| {
            let members = declared
                .values
                .iter()
                .map(|v| AadEnumMember {
                    name: v.key.clone(),
                    // A member is stored under its own name unless it says otherwise: `inputRequired` is written
                    // `input-required`.
                    stored: v.stored.clone(),
                    label: v.label.clone().unwrap_or_else(|| v.key.clone()),
                })
                .collect();
            (name.clone(), AadEnum { members })
        })
        .collect()
}

fn read_relations(root: &Json) -> Vec<AadRelationType> {
    let operations = root.pointer("/behavior/operations");
    let Some(relations) = root.pointer("/metamodel/relations").and_then(Json::as_object) else {
        return Vec::new();
    };
    relations
        .iter()
        .filter_map(|(name, relation)| {
            // The relation's connect operation says which end's key states it: it sets one key on one end.
            let connect = relation.pointer("/derived/edits/connect")?.as_str()?;
            let action = operations?.get(connect)?.get("actions")?.get(0)?;
            Some(AadRelationType {
                type_name: name.clone(),
                source: string_of(relation, "source")?.to_owned(),
                target: string_of(relation, "target")?.to_owned(),
                attribute: action.get("set")?.as_object()?.keys().next()?.clone(),
                held_by_source: string_of(action, "target") == Some("self.source"),
            })
        })
        .collect()
}

fn read_collapsed_by_default(root: &Json) -> BTreeSet<String> {
    let mut collapsed = BTreeSet::new();
    let Some(nodes) = root.pointer("/notation/nodes").and_then(Json::as_object) else {
        return collapsed;
    };
    for node in nodes.values() {
        for compartment in node.get("compartments").and_then(Json::as_array).into_iter().flatten() {
            if let Some(groups) = compartment.pointer("/groupBy/collapsed").and_then(Json::as_object) {
                collapsed.extend(
                    groups
                        .iter()
                        .filter(|(_, v)| v.as_bool() == Some(true))
                        .map(|(k, _)| k.clone()),
                );
            } else if compartment.get("collapsed").and_then(Json::as_bool) == Some(true)
                && let Some(id) = string_of(compartment, "id")
            {
                collapsed.insert(id.to_owned());
            }
        }
    }
    collapsed
}
