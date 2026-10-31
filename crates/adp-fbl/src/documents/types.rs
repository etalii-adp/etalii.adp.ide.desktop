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
    Bullets,
    /// *(FBL 0.9)* A body read by the grammar its binding carries (FBL section 4.9).
    Grammar,
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
    /// The sections of a `bullets` body (FBL 0.7 section 4.8), each found by its first matching heading.
    pub sections: Vec<SectionRule>,
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
    /// *(FBL 0.9)* The grammar of a `grammar` body (FBL section 4.9).
    pub grammar: Option<GrammarSettings>,
    /// *(FBL 0.15)* grammar: the directives a slot with `expand` is read through (FBL section 4.9).
    pub prefixes: Option<PrefixSettings>,
    /// *(FBL 0.15)* grammar: the numberings a slot with `ordinal` reads, by name (FBL section 4.9).
    pub ordinals: Vec<(String, OrdinalSettings)>,
    /// *(FBL 0.15)* grammar: `closed` reads a body's elements in the order their entries end (FBL section 4.9).
    pub order: Option<String>,
}

/// `body.prefixes` (FBL 0.15 section 4.9): the directives that declare a prefix or a base, and how a term is written.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PrefixSettings {
    pub declarations: Vec<PrefixDirective>,
    pub bases: Vec<PrefixDirective>,
    /// A term written whole: the group `value` is the IRI, with its numeric escapes.
    pub absolute: String,
    /// A term written under a prefix: the groups `prefix` and `local`.
    pub prefixed: String,
    /// What a local name may hold for an IRI to be written under a prefix.
    pub local: String,
    /// The terminals whose tokens are terms under a prefix, each of which a declaration must be in force for.
    pub terminals: Vec<String>,
}

/// A directive of `body.prefixes` (FBL 0.15 section 4.9): the entries it is, and the slots of their name and IRI.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PrefixDirective {
    pub at: String,
    pub name: Option<Slot>,
    pub iri: Slot,
}

/// One of `body.ordinals` (FBL 0.15 section 4.9): the nodes numbered, and the terminal whose equal texts are one node.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OrdinalSettings {
    pub at: Vec<String>,
    pub same: Option<String>,
}

/// The grammar of a `grammar` body (FBL 0.9 section 4.9): its modules in ATD grammar notation, the start rule and
/// the reading in part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrammarSettings {
    /// The module the body is read by: `grammar` for one given as text, the file's name without `.lark` for one in
    /// a file beside the document.
    pub root: String,
    /// The file the grammar is in, when the binding names one.
    pub file: Option<String>,
    /// The text of each module, by name: the root and the modules it imports. A grammar in a file has none until
    /// the document is loaded from a path, which reads them beside it.
    pub modules: std::collections::BTreeMap<String, String>,
    pub start: String,
    pub recover: Option<Recover>,
}

/// `recover` (FBL 0.9 section 4.9): the rule a body is read by unit by unit, and the terminals reading skips to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recover {
    pub unit: String,
    /// Terminal names, or the text of anonymous string terminals.
    pub sync: Vec<String>,
    /// *(FBL 0.11)* The two terminals that open and close a block, for reading in part inside blocks.
    pub nested: Option<NestedRecover>,
    /// *(FBL 0.11)* Each ignored terminal and the text a token of it must end with, else `fbl.unclosed-comment`.
    pub unclosed: Vec<(String, String)>,
}

/// `recover.nested` (FBL 0.11 section 4.9): each a terminal's name, or the text of an anonymous string terminal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedRecover {
    pub open: String,
    pub close: String,
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
    /// *(FBL 0.11)* A `grammar` body: the selector of the rule's entries (FBL section 4.9).
    pub at: Option<String>,
    /// *(FBL 0.11)* A `grammar` body: the `{node}` or `{value}` slot that names a view.
    pub view_slot: Option<Slot>,
    /// *(FBL 0.11)* The readings the rule's entries are views for; empty for every reading.
    pub origins: Vec<String>,
    /// *(FBL 0.11)* A `grammar` body: how a new entry of the rule is written.
    pub insert: Option<InsertSettings>,
}

/// A section of a `bullets` body (FBL 0.7 section 4.8): its name, and the expression its heading's text matches.
#[derive(Debug, Clone)]
pub struct SectionRule {
    pub name: String,
    pub heading: String,
    pub case_insensitive: bool,
}

/// An element rule or a relation rule (FBL sections 5.1 and 5.4).
#[derive(Debug, Clone, Default)]
pub struct Rule {
    pub name: String,
    pub type_name: String,
    pub is_relation: bool,
    /// *(FBL 0.10)* The readings the rule is a rule of (FBL section 5.1); empty for every reading.
    pub origins: Vec<String>,
    pub at: Option<String>,
    pub line: Option<String>,
    pub within: Option<Vec<String>>,
    pub opens: bool,
    /// *(FBL 0.11)* A `grammar` body: the text of a block written around a first child (FBL section 4.9).
    pub opens_text: Option<OpensText>,
    /// *(FBL 0.11)* A `grammar` body: the rule matches only inside the view the reading shows (FBL section 9.3).
    pub selected_view: bool,
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

    /// *(FBL 0.14)* The capture a tree rule's `at` ends in: an insert of the rule adds a member of that mapping, named by
    /// its key (FBL section 6.2).
    pub fn member_capture(&self) -> Option<&str> {
        let last = self.at.as_deref()?.rsplit('/').next()?;
        last.strip_prefix('{')?.strip_suffix('}')
    }

    /// *(FBL 0.14)* The name of the attribute bound to the capture a new member's key is the value of.
    pub fn member_key_attribute(&self) -> Option<&str> {
        let capture = self.member_capture()?;
        self.attributes
            .iter()
            .find(|(_, a)| a.capture.as_deref() == Some(capture))
            .map(|(name, _)| name.as_str())
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

/// `opens` in the `grammar` family (FBL 0.11 section 4.9): the text written after an entry's last token, and the
/// closing line's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpensText {
    pub open: String,
    pub close: String,
}

#[derive(Debug, Clone)]
pub struct IdBinding {
    pub from: Option<Slot>,
    pub sidecar_key: Option<String>,
    /// *(FBL 0.12)* The key space of the rule's identities (FBL section 8.6).
    pub sidecar_kind: Option<String>,
    /// *(FBL 0.12)* What follows the natural key of each later holder of one key, `{n}` its ordinal (FBL section 8.6).
    pub sidecar_ordinal: Option<String>,
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
    /// The continuation of a `bullets` item (FBL 0.7 section 4.8): its lines before its first nested item.
    pub continuation: bool,
    pub word: Option<String>,
    pub flag: bool,
    /// *(FBL 0.9)* A `grammar` slot: the relative selector of the node whose text is the value (`.` the entry).
    pub node: Option<String>,
    /// *(FBL 0.9)* With `node`: the regular expression whose group `value` is the value.
    pub node_match: Option<String>,
    /// *(FBL 0.11)* With `node`: how a value written inside quotes escapes them (FBL section 4.9).
    pub escape: Option<SlotEscape>,
    /// *(FBL 0.13)* With `node`: the text a node's value is cut into items at (FBL section 4.9).
    pub split: Option<String>,
    /// *(FBL 0.12)* With `node`: the item attributes of each node reached, each a slot relative to it (FBL section 5.2).
    pub items: Vec<(String, Slot)>,
    /// *(FBL 0.12)* With `items` or `split`: the text written between two items, `", "` when none is given.
    pub separator: Option<String>,
    /// *(FBL 0.15)* With `node`: the node's text is a term, read through `body.prefixes` (FBL section 4.9).
    pub expand: Option<SlotExpand>,
    /// *(FBL 0.15)* With `node`: the value is the node's number in this numbering of `body.ordinals` (FBL section 4.9).
    pub ordinal: Option<String>,
    /// *(FBL 0.15)* With `node`: the value is the node's name: its rule, alias or terminal (FBL section 4.9).
    pub node_name: bool,
}

/// A slot's `expand` (FBL 0.15 section 4.9).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SlotExpand {
    /// A text that stands for an IRI whatever the directives say, in the slot's order.
    pub keywords: Vec<(String, String)>,
}

/// A `{node}` slot's `escape` (FBL 0.11 section 4.9): the escape character and the quote it escapes; *(FBL 0.15)* the
/// other sequences it reads, the characters it writes by one, and the quote a set writes the whole node in.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SlotEscape {
    pub with: char,
    pub quote: char,
    /// The character after `with`, and the character it stands for.
    pub sequences: Vec<(char, char)>,
    /// The letter after `with`, and how many hexadecimal digits of a code point follow it.
    pub unicode: Vec<(char, usize)>,
    /// The characters a value writes by their sequence, beside `quote` and `with`.
    pub writes: Vec<char>,
    /// A set replaces the whole node, with the value between two of these.
    pub requote: Option<char>,
}

impl SlotEscape {
    /// `with` before `quote` or `with` read as that character, *(FBL 0.15)* before a key of `sequences` as what it stands
    /// for, before a key of `unicode` and that many hexadecimal digits as that code point; `with` before any other
    /// character kept as it is.
    pub fn read(&self, text: &str) -> String {
        let mut read = String::with_capacity(text.len());
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            if c != self.with {
                read.push(c);
                continue;
            }
            let Some(&next) = chars.peek() else {
                read.push(c);
                continue;
            };
            if next == self.quote || next == self.with {
                read.push(next);
                chars.next();
            } else if let Some((_, stands)) = self.sequences.iter().find(|(key, _)| *key == next) {
                read.push(*stands);
                chars.next();
            } else if let Some((_, digits)) = self.unicode.iter().find(|(key, _)| *key == next) {
                let hex: String = chars.clone().skip(1).take(*digits).collect();
                match u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                    Some(point) if hex.len() == *digits => {
                        read.push(point);
                        chars.nth(*digits);
                    }
                    _ => read.push(c),
                }
            } else {
                read.push(c);
            }
        }
        read
    }

    /// Every `quote` and `with` character written after `with`, *(FBL 0.15)* and every character of `writes` by its
    /// sequence.
    pub fn write(&self, text: &str) -> String {
        let mut written = String::with_capacity(text.len());
        for c in text.chars() {
            if c == self.quote || c == self.with {
                written.push(self.with);
                written.push(c);
            } else if self.writes.contains(&c)
                && let Some((key, _)) = self.sequences.iter().find(|(_, stands)| *stands == c)
            {
                written.push(self.with);
                written.push(*key);
            } else {
                written.push(c);
            }
        }
        written
    }
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
        } else if self.continuation {
            "continuation".into()
        } else if let Some(n) = &self.node {
            format!("node {n}")
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
    /// *(FBL 0.11)* grammar: further `{node}` slots of the entry, whose values follow the first slot's.
    pub gather: Vec<Slot>,
    /// *(FBL 0.15)* grammar: further slots of the entry, the first present of which is read when the attribute's own is
    /// absent, and written as itself.
    pub or: Vec<Slot>,
    /// *(FBL 0.10)* The slot read only when the attribute's own is absent, and removed by a write of the attribute
    /// (FBL section 5.2): written as a binding of that slot alone that an empty value removes.
    pub secondary: Option<Box<AttributeBinding>>,
    /// *(FBL 0.10)* The sentence of a refusal by `empty: "refuse"` (FBL section 6.4).
    pub refusal_empty: Option<String>,
    /// *(FBL 0.10)* The sentence of a refusal by `absent` (FBL section 6.4).
    pub refusal_absent: Option<String>,
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
/// `at_end`, each missing level of the mapping `child` reaches created at the end of the level above it. grammar
/// (FBL 0.11): `after`, the attributes whose parts precede this one, nearest first, and whether the part is `pad`ded.
#[derive(Debug, Clone)]
pub struct CreateChild {
    pub emit: String,
    pub place: String,
    pub before: Option<String>,
    pub at_end: bool,
    pub after: Vec<String>,
    pub pad: bool,
}

#[derive(Debug, Clone)]
pub struct InsertSettings {
    /// "after-last", "end", "start", "last-child", "next-sibling", "end-of-document" or "before".
    pub place: String,
    /// With `place` "before": the member of the container that the new entry goes before.
    pub before: Option<String>,
    pub container: Option<String>,
    pub create: Option<CreateContainer>,
    pub keys: Vec<String>,
    pub emit: Option<String>,
    pub skeleton: Option<String>,
    pub when: Option<String>,
    /// *(FBL 0.10)* The sentence of a refusal because `when` does not hold (FBL section 6.4).
    pub refusal_when: Option<String>,
    /// *(FBL 0.10)* The sentence of a refusal because the container is missing and there is no `create`.
    pub refusal_container: Option<String>,
    /// *(FBL 0.14)* The sentence of a refusal because a new member's key is missing or empty.
    pub refusal_key: Option<String>,
    /// *(FBL 0.14)* The sentence of a refusal because the mapping already holds a new member's key.
    pub refusal_duplicate: Option<String>,
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
    /// The blocks the binding declares (FBL 0.7 section 8.1), in the order it declares them, which is the order they
    /// are written in (section 8.4).
    pub blocks: Vec<RegistrationBlockDeclaration>,
    pub resource_capture: Option<String>,
    /// *(FBL 0.10)* `resource.byOrigin`: the capture each origin selects by, none for an origin that selects no
    /// resource (FBL section 8.2). Empty when the capture is the same for every reading.
    pub resource_by_origin: Vec<(String, Option<String>)>,
    pub legacy_layout: Option<String>,
    /// *(FBL 0.12)* The legacy identities file, as `{base}.identities.json` (FBL section 8.7).
    pub legacy_identities: Option<String>,
}

impl RegistrationSettings {
    /// The capture a registration of `origin` selects a resource by (FBL section 8.2): `byOrigin`'s for its origin,
    /// none for an origin it maps to null or does not list, else `capture`.
    pub fn resource_capture_of(&self, origin: Option<&str>) -> Option<&str> {
        if self.resource_by_origin.is_empty() {
            return self.resource_capture.as_deref();
        }
        let origin = origin?;
        self.resource_by_origin
            .iter()
            .find(|(o, _)| o == origin)
            .and_then(|(_, capture)| capture.as_deref())
    }
}

/// A block of the registration that a binding declares (FBL 0.7 section 8.1, `registration.blocks`): its name,
/// whether its entries stand where the edit put them (`kept`) or in the ordinal order of their keys, whether its keys
/// are element ids, which are pruned when stale (FBL section 8.5), and whether its writes are steps of the history
/// (FBL section 7.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistrationBlockDeclaration {
    pub name: String,
    pub kept: bool,
    pub keyed_by_ids: bool,
    pub in_history: bool,
}

#[derive(Debug, Clone)]
pub struct TemplateSettings {
    pub text: String,
    pub by_origin: Vec<(String, String)>,
}
