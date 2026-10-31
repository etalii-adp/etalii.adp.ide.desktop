//! What the engine that reads a body through its binding works with (FBL sections 4.1 and 5; the standalone host's
//! `Rules/FamilyReader` records): a candidate, which is an entry a rule's selector or line matched with what the
//! match bound; what reading one slot of it gave; and an element or relation as the engine keeps it, with where
//! each of its values lives so that it can be written back.

pub(crate) mod read_mode;
pub mod reading;
pub(crate) mod scalar_conversion;
pub(crate) mod selector;

use std::rc::Rc;

use adp_cel::{Map, Value};

use crate::documents::types::Rule;
use crate::families::EntryId;
use crate::planning::new_text;
use crate::text::Span;

/// The captures a selector bound or the groups a line matched, by name, in the order they were bound.
pub(crate) type Captures = Rc<Vec<(String, String)>>;

pub(crate) fn capture<'a>(captures: &'a Captures, name: &str) -> Option<&'a str> {
    captures.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str())
}

/// An entry a rule's selector or line matched (by index into the binding's rules or blocks), with the captures or
/// groups the match bound.
#[derive(Debug, Clone)]
pub(crate) struct Candidate {
    pub rule: Option<usize>,
    pub block: Option<usize>,
    pub entry: EntryId,
    pub captures: Captures,
}

/// A word of a lines or blocks group (FBL section 4.6), with its span (quotes included) and its text (quotes
/// excluded).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Word {
    pub text: String,
    pub span: Span,
    pub quoted: bool,
}

/// The family's own node for a slot, for writing: a yaml or json member, the mapping a missing key goes in, an
/// xml attribute or element, an element's text, a lines group or word, or a value read from an `override` slot.
#[derive(Debug, Clone, Default)]
pub(crate) enum Node {
    #[default]
    None,
    TreeMember(EntryId),
    TreeMapping(usize),
    XmlAttribute(EntryId, usize),
    XmlElement(EntryId),
    XmlText {
        element: EntryId,
        span: Option<Span>,
        html: bool,
    },
    Group,
    Word(Word),
    /// The continuation of a `bullets` item (FBL 0.7 section 4.8).
    Continuation,
    /// A value read from an attribute's `override` slot, which writing removes (FBL section 5.2).
    Override(Box<Node>),
}

impl Node {
    /// The node without its override wrapper.
    pub fn inner(&self) -> &Node {
        match self {
            Node::Override(inner) => inner,
            other => other,
        }
    }
}

/// One item of a slot of several (a word slot of an attribute with `many`, FBL 0.7 section 4.6; *(FBL 0.11)* a `{node}`
/// slot): its value, the span the value is written in (the word's group `value`, or the whole word), and the word it is
/// in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WordItem {
    pub value: String,
    pub value_span: Span,
    pub word: Word,
    /// *(FBL 0.9)* The grammar node the item is read from; the statement's entry in any other family.
    pub node: EntryId,
    /// *(FBL 0.12)* The item attributes of a slot with `items`, each with its text and its span.
    pub parts: Vec<ItemPart>,
}

/// One item attribute of a slot with `items` (FBL 0.12 section 5.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ItemPart {
    pub name: String,
    pub text: String,
    pub span: Span,
}

/// What keys an element's stored id (FBL 0.12 section 8.6): its rule's `sidecar.kind`, the value of `sidecar.key`, and
/// that value with the `ordinal` of a later holder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IdentityKey {
    pub kind: Option<String>,
    pub key: String,
}

impl IdentityKey {
    /// The key of the element's entry: `<kind>:<key>` when its rule declares a kind.
    pub fn entry(&self) -> String {
        match &self.kind {
            Some(kind) => format!("{kind}:{}", self.key),
            None => self.key.clone(),
        }
    }
}

/// What reading one slot of one entry gave.
#[derive(Debug, Clone, Default)]
pub(crate) struct SlotRead {
    /// *(FBL 0.15)* Which of the attribute's `or` slots was read; none for its own.
    pub alternative: Option<usize>,
    pub value: Value,
    pub span: Option<Span>,
    pub present: bool,
    pub writable: bool,
    pub reason: Option<String>,
    pub node: Node,
    /// For a group read as words: every word of the group, so references inside it can be rewritten one by one.
    pub words: Option<Rc<Vec<Word>>>,
    /// For a slot of several items, of an attribute with `many` (FBL 0.7 section 4.6) or *(FBL 0.12, 0.13)* with `items`
    /// or `split`: every item it holds, in order; None for any other slot.
    pub items: Option<Rc<Vec<WordItem>>>,
    /// The quote the value is written in on the wire, when the span includes one.
    pub quote: Option<String>,
    /// The value as written, before maps or conversion, for keeping a map's wire value.
    pub wire: Option<String>,
    /// Whether the slot holds a true scalar of a yaml or json tree, so that `wire` is its written text (FBL
    /// section 4.1.5). A flow collection has the kind of a scalar and is not one.
    pub written_scalar: bool,
    /// The style name of a written scalar (FBL 0.5 section 2.4), which says whether it is quoted.
    pub style: Option<&'static str>,
}

impl SlotRead {
    pub fn new(value: Value, span: Option<Span>, present: bool, writable: bool) -> Self {
        SlotRead {
            value,
            span,
            present,
            writable,
            ..SlotRead::default()
        }
    }

    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    pub fn absent() -> Self {
        SlotRead::new(Value::Null, None, false, true)
    }

    pub fn read_only_absent(reason: impl Into<String>) -> Self {
        SlotRead::new(Value::Null, None, false, false).with_reason(reason)
    }

    /// The written text of the slot (FBL section 4.1.5): what the author wrote for a true scalar, otherwise the
    /// value as text, which is already the written text for the xml, lines and blocks families.
    pub fn written(&self) -> String {
        match (&self.wire, self.written_scalar) {
            (Some(wire), true) => wire.clone(),
            _ => new_text::plain(&self.value, None),
        }
    }
}

/// An element or relation as the engine keeps it: the public element plus where each value lives.
#[derive(Debug, Clone)]
pub(crate) struct ReadElement {
    pub rule: usize,
    pub candidate: Candidate,
    pub id: String,
    pub id_stored: bool,
    /// Boxed, as are the source and the target: most elements have none, and an element is kept for each entry read.
    pub id_read: Option<Box<SlotRead>>,
    /// *(FBL 0.12)* Whether the id is the one the specification's strategy made, which a sidecar rule stores.
    pub id_derived: bool,
    /// *(FBL 0.12)* What keys the id of a rule with `id.sidecar`.
    pub identity: Option<IdentityKey>,
    /// The value references name this element by: its stored id, or the attribute its own rules reference it by.
    pub key: String,
    /// The attribute whose value is `key`, when it is an attribute.
    pub key_attribute: Option<String>,
    pub slots: Vec<(String, SlotRead)>,
    pub attributes: Map,
    pub parent: Option<usize>,
    /// The element's zero-based place among those of the same parent and slot, in document order.
    pub position_in_slot: usize,
    pub source_read: Option<Box<SlotRead>>,
    pub target_read: Option<Box<SlotRead>>,
    pub source_element: Option<usize>,
    pub target_element: Option<usize>,
    pub line: usize,
    /// Why the entry cannot be read as this element: the first value that does not convert and has no fallback.
    pub mistyped: Option<String>,
    /// The values read as their fallback, reported once the entry is claimed (FBL 0.5 section 7.4).
    pub unconverted: Vec<(String, Option<Span>, String, String)>,
    pub is_relation: bool,
}

impl ReadElement {
    pub fn entry(&self) -> EntryId {
        self.candidate.entry
    }

    pub fn slot(&self, name: &str) -> Option<&SlotRead> {
        self.slots.iter().find(|(n, _)| n == name).map(|(_, s)| s)
    }

    /// An attribute as a key or a reference compares it (FBL 0.5 section 5.7): the written text of its slot, or
    /// the value as text where the slot is absent, holds a list or has a map to translate it.
    pub fn text_of(&self, rule: &Rule, attribute: &str) -> String {
        let value = self.attributes.get(attribute).cloned().unwrap_or_default();
        if let Some(read) = self.slot(attribute)
            && read.present
            && !matches!(value, Value::List(_))
            && rule.attribute(attribute).is_some_and(|b| b.map.is_none())
        {
            return read.written();
        }
        new_text::plain(&value, None)
    }
}

/// One attribute to write, after the engine's checks: its binding, how it is read now, and its new value.
#[derive(Debug, Clone)]
pub(crate) struct SlotChange<'a> {
    pub attribute: String,
    pub binding: &'a crate::documents::types::AttributeBinding,
    /// The rule whose binding writes the attribute, by index.
    pub rule: usize,
    pub read: SlotRead,
    pub value: Value,
    pub is_empty: bool,
}

/// What adding an element or relation needs to know (FBL section 6.2).
#[derive(Debug, Clone)]
pub(crate) struct InsertRequest {
    pub rule: usize,
    pub id: Option<String>,
    pub values: Map,
    pub parent: Option<usize>,
    pub source: Option<usize>,
    pub target: Option<usize>,
    /// The position among its siblings, for a family that places by position (FBL 0.7 section 6.3).
    pub index: Option<i64>,
}
