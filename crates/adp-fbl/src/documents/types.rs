//! A loaded FBL document (FBL sections 2.1 and 3; the standalone host's `Documents/FblModelTypes`): its version and
//! its bindings by name, each binding with its claims, its body, its text defaults and its rules. A rule's
//! attribute bindings keep the order the binding writes them in, which is the order new keys are written in.

use std::sync::LazyLock;

use regex::Regex;
use serde_json::Value as Json;

/// A loaded FBL document (FBL section 2.1).
#[derive(Debug, Clone)]
pub struct FblDocument {
    pub version: String,
    pub bindings: Vec<(String, FblBinding)>,
}

impl FblDocument {
    pub fn binding(&self, name: &str) -> Option<&FblBinding> {
        self.bindings.iter().find(|(n, _)| n == name).map(|(_, b)| b)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Family {
    Yaml,
    Json,
    Xml,
    Lines,
    Blocks,
}

/// One binding (FBL section 3): how one format maps to a language's model.
#[derive(Debug, Clone)]
pub struct FblBinding {
    pub name: String,
    pub title: Option<String>,
    pub claims: Claims,
    pub body: BodySettings,
    /// None for a declared reader; the plugin otherwise (FBL section 11).
    pub plugin: Option<PluginReader>,
    /// None when the body may be written; the reason (possibly empty) when it is read-only.
    pub read_only: Option<String>,
    pub text: TextDefaults,
    pub header: Option<HeaderSettings>,
    pub comment: Option<String>,
    pub report_unmatched: bool,
    pub blocks: Vec<BlockRule>,
    /// Elements before relations, in binding order: the order rules are offered an entry (FBL section 5.1).
    pub rules: Vec<Rule>,
    pub registration: RegistrationSettings,
    pub template: Option<TemplateSettings>,
}

impl FblBinding {
    /// Elements before relations, in binding order.
    pub fn all_rules(&self) -> &[Rule] {
        &self.rules
    }

    pub fn elements(&self) -> impl Iterator<Item = &Rule> {
        self.rules.iter().filter(|r| !r.is_relation)
    }

    pub fn relations(&self) -> impl Iterator<Item = &Rule> {
        self.rules.iter().filter(|r| r.is_relation)
    }

    pub fn rule(&self, name: &str) -> Option<&Rule> {
        self.rules.iter().find(|r| r.name == name)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Claims {
    pub extensions: Vec<String>,
    pub names: Vec<String>,
    pub shared: bool,
    pub registration_only: bool,
    pub marker: Option<Marker>,
    pub suggest: Vec<String>,
    pub origins: Vec<String>,
    /// Per origin: whether a bare file opens as it, and what suggests it.
    pub readings: Vec<(String, ReadingClaim)>,
}

#[derive(Debug, Clone)]
pub struct ReadingClaim {
    pub bare: bool,
    pub suggest: Vec<String>,
}

/// A marker (FBL section 12.2): exactly one of a root key, a first-line prefix or a pattern.
#[derive(Debug, Clone)]
pub struct Marker {
    pub root_key: Option<String>,
    pub root_value: Option<Json>,
    pub first_line: Option<String>,
    pub pattern: Option<String>,
    pub lines: i64,
}

/// The `settle` of a folder whose binding states none (FBL section 10.3).
pub const DEFAULT_SETTLE: i64 = 400;

#[derive(Debug, Clone)]
pub struct BodySettings {
    pub is_folder: bool,
    pub family: Option<Family>,
    pub also_read: Vec<Family>,
    pub recognise_all: Vec<String>,
    pub recognise_any: Vec<String>,
    pub recognise_none: Vec<String>,
    pub files: Vec<FileRule>,
    pub ignore: Vec<String>,
    /// How long, in milliseconds, a watched folder must be quiet before it is read again (FBL section 10.3).
    pub settle: i64,
}

#[derive(Debug, Clone)]
pub struct FileRule {
    pub name: Option<String>,
    pub glob: String,
    pub family: Option<Family>,
}

#[derive(Debug, Clone)]
pub struct PluginReader {
    pub plugin: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TextDefaults {
    pub newline: String,
    /// Spaces per step, or 0 for a tab.
    pub indent: usize,
    pub sequence_flush: bool,
    pub quote: String,
}

impl Default for TextDefaults {
    fn default() -> Self {
        TextDefaults {
            newline: "\n".into(),
            indent: 2,
            sequence_flush: false,
            quote: "double".into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct HeaderSettings {
    pub key: Option<String>,
    pub value: Option<Json>,
    pub line: Option<String>,
    pub required: bool,
}

#[derive(Debug, Clone)]
pub struct BlockRule {
    pub name: String,
    pub line: String,
    pub within: Option<Vec<String>>,
    pub case_insensitive: bool,
    pub view: Option<String>,
}

/// An element rule or a relation rule (FBL sections 5.1 and 5.4).
#[derive(Debug, Clone, Default)]
pub struct Rule {
    pub name: String,
    pub type_name: String,
    pub is_relation: bool,
    pub at: Option<String>,
    pub line: Option<String>,
    pub within: Option<Vec<String>>,
    pub opens: bool,
    pub case_insensitive: bool,
    pub files: Vec<String>,
    pub when: Option<String>,
    pub id: Option<IdBinding>,
    pub parent: Option<ParentBinding>,
    /// The attribute bindings in the order the binding writes them.
    pub attributes: Vec<(String, AttributeBinding)>,
    pub source: Option<AttributeBinding>,
    pub target: Option<AttributeBinding>,
    pub insert: Option<InsertSettings>,
    pub remove: Option<RemoveSettings>,
    pub snapshot_undo: bool,
    pub read_only: Option<String>,
}

/// A placeholder of an `emit` text: an attribute's name, or `id`, in braces.
static PLACEHOLDER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{([A-Za-z_][A-Za-z0-9_]*)\}").expect("the placeholder pattern compiles"));

impl Rule {
    pub fn attribute(&self, name: &str) -> Option<&AttributeBinding> {
        self.attributes.iter().find(|(n, _)| n == name).map(|(_, a)| a)
    }

    pub fn attribute_index(&self, name: &str) -> Option<usize> {
        self.attributes.iter().position(|(n, _)| n == name)
    }

    /// The order a new entry's keys or attributes are written in, as the names they are written under:
    /// `insert.keys`, or, for a rule that writes its entry from `emit`, the order of the text's placeholders. A key
    /// or attribute added to an existing entry takes its place in this order (FBL section 6.1). Empty for a rule
    /// that cannot insert.
    pub fn wire_order(&self) -> Vec<String> {
        let Some(insert) = &self.insert else {
            return Vec::new();
        };
        if !insert.keys.is_empty() {
            return insert.keys.clone();
        }
        let Some(emit) = &insert.emit else {
            return Vec::new();
        };
        let mut order: Vec<String> = Vec::new();
        for placeholder in PLACEHOLDER.captures_iter(emit) {
            let name = &placeholder[1];
            let slot: Option<&Slot> = if name == "id" {
                self.id.as_ref().and_then(|i| i.from.as_ref())
            } else {
                self.attribute(name).map(|a| &a.slot)
            };
            let wire = slot.and_then(|s| s.key.clone().or_else(|| s.xml_attribute.clone()));
            if let Some(wire) = wire
                && !order.contains(&wire)
            {
                order.push(wire);
            }
        }
        order
    }
}

#[derive(Debug, Clone)]
pub struct IdBinding {
    pub from: Option<Slot>,
    pub sidecar_key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ParentBinding {
    pub rules: Vec<String>,
    pub slot: Option<String>,
}

/// A slot (FBL section 5.2): exactly one place a value is read from and written to. Exactly one of the slot kinds
/// is set; `child`, `word` and `flag` qualify it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Slot {
    pub key: Option<String>,
    pub xml_attribute: Option<String>,
    pub text: bool,
    pub child: Option<String>,
    pub group: Option<String>,
    pub parent: Option<String>,
    pub capture: Option<String>,
    pub value: Option<String>,
    pub word: Option<String>,
    pub flag: bool,
}

impl Slot {
    pub fn is_computed(&self) -> bool {
        self.value.is_some()
    }

    /// How a slot is named in a refusal: `key name`, `attribute name`, `text`, …
    pub fn describe(&self) -> String {
        if let Some(k) = &self.key {
            format!("key {k}")
        } else if let Some(a) = &self.xml_attribute {
            format!("attribute {a}")
        } else if self.text {
            "text".into()
        } else if let Some(g) = &self.group {
            format!("group {g}")
        } else if let Some(p) = &self.parent {
            format!("parent {p}")
        } else if let Some(c) = &self.capture {
            format!("capture {c}")
        } else if self.value.is_some() {
            "value".into()
        } else {
            "slot".into()
        }
    }
}

/// An attribute binding (FBL section 5.2): a slot with the options that decide reading and writing.
#[derive(Debug, Clone, Default)]
pub struct AttributeBinding {
    pub slot: Slot,
    /// "remove", "keep" or "refuse"; none for the default.
    pub empty: Option<String>,
    /// Per family name: "insert" or "refuse".
    pub absent: Vec<(String, String)>,
    pub default: Option<Json>,
    /// None for "shortest", else the number of decimals.
    pub decimals: Option<i32>,
    pub keep_time_precision: bool,
    pub style: Option<String>,
    pub reference: Option<ReferenceBinding>,
    /// Wire value to model value, in document order.
    pub map: Option<Vec<(String, String)>>,
    pub override_slot: Option<Slot>,
    pub html_paragraphs: bool,
    pub create: Option<CreateChild>,
    pub read_only: Option<String>,
    /// yaml, json (FBL 0.5): "text" or "typed", what the slot reads from a scalar.
    pub read: Option<String>,
    /// yaml, read as typed (FBL 0.5): "convert" to convert a quoted scalar from its written text.
    pub quoted: Option<String>,
    /// yaml, json (FBL 0.5): what the attribute is when its value does not convert.
    pub fallback: Option<FallbackBinding>,
}

impl std::ops::Deref for AttributeBinding {
    type Target = Slot;

    fn deref(&self) -> &Slot {
        &self.slot
    }
}

impl AttributeBinding {
    /// An attribute binding that is only its slot, as a slot read or an id written through one.
    pub fn of_slot(slot: Slot) -> Self {
        AttributeBinding {
            slot,
            ..AttributeBinding::default()
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReferenceBinding {
    pub to: Vec<String>,
    pub by: String,
}

/// An attribute binding's `fallback` (FBL 0.5 section 5.2): its value, and whether `fbl.unconverted-value` is
/// reported.
#[derive(Debug, Clone)]
pub struct FallbackBinding {
    pub value: Json,
    pub report: bool,
}

/// How a missing child holding the value is written. xml: the element's text and its place. yaml (FBL 0.4):
/// `at_end`, each missing level of the mapping `child` reaches created at the end of the level above it.
#[derive(Debug, Clone)]
pub struct CreateChild {
    pub emit: String,
    pub place: String,
    pub before: Option<String>,
    pub at_end: bool,
}

#[derive(Debug, Clone)]
pub struct InsertSettings {
    /// "after-last", "end", "start", "last-child", "next-sibling", "end-of-document" or "before".
    pub place: String,
    pub container: Option<String>,
    pub create: Option<CreateContainer>,
    pub keys: Vec<String>,
    pub emit: Option<String>,
    pub skeleton: Option<String>,
    pub when: Option<String>,
}

/// How a missing container is created: `at` is "end-of-document", "before", "after", "under" or (FBL 0.4) "end".
#[derive(Debug, Clone)]
pub struct CreateContainer {
    pub at: String,
    pub argument: Option<String>,
    pub text: Option<String>,
}

/// What goes with a removed entry. `remove_empty_levels` (FBL 0.4) implies `remove_container_when_empty`, and
/// removes the levels above the container that it leaves empty too.
#[derive(Debug, Clone)]
pub struct RemoveSettings {
    pub cascade: Vec<String>,
    pub remove_container_when_empty: bool,
    pub remove_empty_levels: bool,
}

#[derive(Debug, Clone, Default)]
pub struct RegistrationSettings {
    pub headers: Vec<String>,
    pub create_on_first_placement: bool,
    pub resource_capture: Option<String>,
    pub legacy_layout: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TemplateSettings {
    pub text: String,
    pub by_origin: Vec<(String, String)>,
}
