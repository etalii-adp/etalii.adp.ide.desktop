//! What the yaml and json families share (FBL sections 4.1.1, 4.1.4, 4.2 and 5.2; the standalone host's
//! `Rules/TreeFamily`): a value is a scalar, a mapping or a sequence with its span as written, an entry is a mapping
//! member, a sequence item or the document root, and selectors, CEL values and reading keys work the same over both.

use adp_cel::{Map, Value};

use crate::documents::loader::scalar_text;
use crate::documents::types::{AttributeBinding, FblBinding, HeaderSettings, Slot};
use crate::families::{Entry, EntryId, EntryKind, Family};
use crate::model::FblOptions;
use crate::planning::plan::Plan;
use crate::rules::read_mode::{self, ReadMode};
use crate::rules::{Candidate, Captures, Node, SlotRead, capture, selector};
use crate::text::Span;

pub(crate) type ValueId = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ValueKind {
    Scalar,
    Mapping,
    Sequence,
}

/// How a value is written: FBL section 6.3 keeps a replaced value's style when the new value fits it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ValueStyle {
    Plain,
    Single,
    Double,
    Literal,
    Folded,
    FlowSequence,
    FlowMapping,
    Block,
    Empty,
    JsonString,
    JsonOther,
}

/// A value of a yaml or json tree (FBL section 4.1.1): a scalar, a mapping or a sequence, with its span as written.
#[derive(Debug, Clone)]
pub(crate) struct TreeValue {
    pub kind: ValueKind,
    pub style: ValueStyle,
    /// The value as written: quotes included; a block collection from its first to its last entry's last byte.
    pub span: Span,
    /// A scalar's text, decoded.
    pub text: String,
    /// A scalar's value by the YAML 1.2 core schema or JSON's types: string, int, double, bool or null.
    pub typed: Value,
    /// The members of a mapping or the items of a sequence, in order.
    pub entries: Vec<EntryId>,
    /// A flow collection's members as CEL sees them: readable, but writable only as a whole (FBL section 4.3).
    pub flow: Option<Value>,
    /// A flow collection's members as written (FBL section 4.1.5), in the shape of `flow`.
    pub flow_text: Option<Value>,
    /// The names of the styles a flow collection's members are written in (FBL section 2.4).
    pub flow_style: Option<Value>,
    /// Reached through an alias or a merge key: readable, never writable (FBL section 4.3).
    pub via_alias: bool,
    /// yaml: the mappings a `<<` key merges into this one, for reading.
    pub merged: Vec<ValueId>,
}

impl TreeValue {
    pub fn new(kind: ValueKind, style: ValueStyle, span: Span) -> Self {
        TreeValue {
            kind,
            style,
            span,
            text: String::new(),
            typed: Value::Null,
            entries: Vec::new(),
            flow: None,
            flow_text: None,
            flow_style: None,
            via_alias: false,
            merged: Vec::new(),
        }
    }

    pub fn scalar(style: ValueStyle, span: Span, text: String, typed: Value) -> Self {
        TreeValue {
            text,
            typed,
            ..TreeValue::new(ValueKind::Scalar, style, span)
        }
    }
}

/// A mapping member, a sequence item, or the document root (FBL section 4.1.1).
#[derive(Debug, Clone)]
pub(crate) struct TreeEntry {
    pub is_root: bool,
    /// A member's key as written, quotes included.
    pub key_span: Option<Span>,
    pub value: ValueId,
    /// json: the offset of the comma after this entry.
    pub separator: Option<usize>,
}

/// What a yaml or json reading keeps besides its entries.
#[derive(Debug, Default)]
pub(crate) struct TreeData {
    pub values: Vec<TreeValue>,
    pub root: Option<EntryId>,
    /// The attribute bindings the host says hold a date or a date-time, by rule and attribute index.
    pub time_typed: Vec<(usize, usize)>,
    /// The attributes read as text (FBL 0.5 section 5.2): a value written for one reads back as the same text.
    pub text_read: Vec<(usize, usize)>,
}

impl TreeData {
    pub fn new(binding: &FblBinding, options: &FblOptions) -> Self {
        let mut data = TreeData::default();
        for (r, rule) in binding.rules.iter().enumerate() {
            for (a, (name, attribute)) in rule.attributes.iter().enumerate() {
                if options.time_attributes.contains(&format!("{}.{name}", rule.name)) {
                    data.time_typed.push((r, a));
                }
                if read_mode::of(rule, name, attribute, options) == ReadMode::Text {
                    data.text_read.push((r, a));
                }
            }
        }
        data
    }
}

/// Which reading of a tree value CEL is given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reading {
    /// `entry`: every scalar by its typed value.
    Typed,
    /// `text`: every scalar by its written text.
    Text,
    /// `style`: every scalar by the name of the style it is written in.
    Style,
}

/// FBL 0.5 section 2.4's names: a yaml scalar's style, a json string `double` and every other json scalar `plain`.
pub(crate) fn style_name(style: ValueStyle) -> &'static str {
    match style {
        ValueStyle::Single => "single",
        ValueStyle::Double | ValueStyle::JsonString => "double",
        ValueStyle::Literal => "literal",
        ValueStyle::Folded => "folded",
        _ => "plain",
    }
}

/// A value as CEL sees it, walked over `values`.
pub(crate) fn cel_of(values: &[TreeValue], entries: &[Entry], id: ValueId, reading: Reading) -> Value {
    let value = &values[id];
    match value.kind {
        ValueKind::Mapping => {
            let mut map = Map::default();
            for &merged in &value.merged {
                if let Value::Map(inherited) = cel_of(values, entries, merged, reading) {
                    for (k, v) in inherited.iter() {
                        map.insert(k.clone(), v.clone());
                    }
                }
            }
            for &member in &value.entries {
                let entry = &entries[member];
                if let Some(name) = &entry.name
                    && name != "<<"
                {
                    map.insert(name.clone(), cel_of(values, entries, entry.tree().value, reading));
                }
            }
            Value::map(map)
        }
        ValueKind::Sequence => Value::list(
            value
                .entries
                .iter()
                .map(|e| cel_of(values, entries, entries[*e].tree().value, reading))
                .collect(),
        ),
        ValueKind::Scalar => match reading {
            Reading::Typed => value.flow.clone().unwrap_or_else(|| value.typed.clone()),
            Reading::Text => value
                .flow_text
                .clone()
                .unwrap_or_else(|| Value::from(value.text.as_str())),
            Reading::Style => value
                .flow_style
                .clone()
                .unwrap_or_else(|| Value::from(style_name(value.style))),
        },
    }
}

/// A scalar whose text is what was written: not a flow collection, nor an alias of a collection.
fn is_written_scalar(value: &TreeValue) -> bool {
    value.kind == ValueKind::Scalar
        && value.flow.is_none()
        && !matches!(value.style, ValueStyle::FlowSequence | ValueStyle::FlowMapping)
}

impl Family {
    pub fn tree_value(&self, id: ValueId) -> &TreeValue {
        &self.tree_data().values[id]
    }

    /// The value of a tree entry.
    pub fn value_of(&self, entry: EntryId) -> &TreeValue {
        self.tree_value(self.entries[entry].tree().value)
    }

    /// The first member of a mapping named `key`.
    pub fn member(&self, mapping: ValueId, key: &str) -> Option<EntryId> {
        self.tree_value(mapping)
            .entries
            .iter()
            .copied()
            .find(|e| self.entries[*e].name.as_deref() == Some(key))
    }

    pub fn tree_root(&self) -> Option<EntryId> {
        self.tree_data().root
    }

    pub fn tree_cel(&self, value: ValueId, reading: Reading) -> Value {
        cel_of(&self.tree_data().values, &self.entries, value, reading)
    }

    pub(super) fn tree_candidates(&self, rule: usize) -> Vec<Candidate> {
        let (Some(at), Some(root)) = (self.rule(rule).at.as_deref(), self.tree_root()) else {
            return Vec::new();
        };
        selector::matches(&self.entries, root, at, None)
            .into_iter()
            .map(|(entry, captures)| Candidate {
                rule: Some(rule),
                block: None,
                entry,
                captures,
            })
            .collect()
    }

    pub(super) fn tree_header_holds(&self, header: &HeaderSettings) -> bool {
        let Some(key) = &header.key else { return true };
        let Some(root) = self.tree_root() else { return false };
        let Some(member) = self.member(self.entries[root].tree().value, key) else {
            return false;
        };
        let Some(expected) = &header.value else { return true };
        scalar_text(expected) == self.value_of(member).text
    }

    /// The mapping a slot is read in: the entry's own, or the one `child` reaches from it.
    pub fn mapping(&self, entry: EntryId, child: Option<&str>) -> Option<ValueId> {
        let value = match child {
            None => self.entries[entry].tree().value,
            Some(child) => {
                let (reached, _) = selector::matches(&self.entries, entry, child, None)
                    .into_iter()
                    .next()?;
                self.entries[reached].tree().value
            }
        };
        (self.tree_value(value).kind == ValueKind::Mapping).then_some(value)
    }

    pub(super) fn tree_read(&self, candidate: &Candidate, slot: &Slot) -> SlotRead {
        let entry = candidate.entry;
        if let Some(capture_name) = &slot.capture {
            let Some(key) = capture(&candidate.captures, capture_name) else {
                return SlotRead::absent();
            };
            let mut e = Some(entry);
            while let Some(id) = e {
                let EntryKind::Tree(tree) = &self.entries[id].kind else {
                    break;
                };
                if self.entries[id].name.as_deref() == Some(key)
                    && let Some(span) = tree.key_span
                {
                    let mut read = SlotRead::new(Value::from(key), Some(span), true, true);
                    read.node = Node::TreeMember(id);
                    return read;
                }
                e = self.entries[id].parent;
            }
            return SlotRead::new(Value::from(key), None, true, false)
                .with_reason("The captured key cannot be found to rewrite.");
        }
        let Some(name) = &slot.key else {
            return SlotRead::read_only_absent(format!("A {} entry has no {}.", self.family_name(), slot.describe()));
        };
        match self.mapping(entry, slot.child.as_deref()) {
            None => SlotRead::absent(),
            Some(mapping) => self.read_member(mapping, name),
        }
    }

    fn member_read(&self, member: EntryId, writable: bool, reason: Option<&str>) -> SlotRead {
        let value = self.value_of(member);
        let mut read = SlotRead::new(
            self.tree_cel(self.entries[member].tree().value, Reading::Typed),
            Some(value.span),
            true,
            writable,
        );
        read.reason = reason.map(str::to_owned);
        read.node = Node::TreeMember(member);
        read.wire = (value.kind == ValueKind::Scalar).then(|| value.text.clone());
        read.written_scalar = is_written_scalar(value);
        read.style = Some(style_name(value.style));
        read
    }

    fn read_member(&self, mapping: ValueId, name: &str) -> SlotRead {
        if let Some(member) = self.member(mapping, name) {
            let value = self.value_of(member);
            let writable = !value.via_alias && value.kind == ValueKind::Scalar;
            let reason = if value.via_alias {
                Some("The value is reached through an alias, so it is read-only.")
            } else if writable {
                None
            } else {
                Some("The value is a block collection, which a slot does not write.")
            };
            return self.member_read(member, writable, reason);
        }
        for &merged in &self.tree_value(mapping).merged {
            if let Some(inherited) = self.member(merged, name) {
                return self.member_read(
                    inherited,
                    false,
                    Some("The value is reached through a merge key, so it is read-only."),
                );
            }
        }
        let mut read = SlotRead::absent();
        read.node = Node::TreeMapping(mapping);
        read
    }

    pub(super) fn tree_read_raw(&self, entry: EntryId, name: &str) -> SlotRead {
        match &self.entries[entry].kind {
            EntryKind::Tree(tree) if self.tree_value(tree.value).kind == ValueKind::Mapping => {
                self.read_member(tree.value, name)
            }
            _ => SlotRead::absent(),
        }
    }

    /// The container entry an insert's `container` selector names, from the root or the parent entry.
    pub fn container(&self, selector_text: &str, parent: Option<EntryId>, captures: &Captures) -> Option<EntryId> {
        let root = self.tree_root()?;
        let resolved: Vec<String> = selector_text
            .split('/')
            .map(|s| {
                if s.len() >= 2 && s.starts_with('{') && s.ends_with('}') {
                    capture(captures, &s[1..s.len() - 1]).unwrap_or(s).to_owned()
                } else {
                    s.to_owned()
                }
            })
            .collect();
        let resolved = resolved.join("/");
        if resolved.starts_with('/') && resolved.trim_matches('/').is_empty() {
            return Some(root);
        }
        let start = selector::start(&resolved, root, parent);
        selector::matches(&self.entries, start, &resolved, None)
            .into_iter()
            .next()
            .map(|(e, _)| e)
    }

    /// The captures the binding's existing entries bound, so `{capture}` segments of a container resolve (FBL 6.2).
    pub fn captures_of(plan: &Plan<'_>) -> Captures {
        plan.reading
            .elements()
            .map(|e| e.candidate.captures.clone())
            .find(|c| !c.is_empty())
            .unwrap_or_default()
    }

    /// Indexes the tree under `entry` in document order, parenting every member and item.
    pub(super) fn index(&mut self, entry: EntryId) {
        self.order.push(entry);
        let children = self.value_of(entry).entries.clone();
        for child in children {
            self.entries[child].parent = Some(entry);
            self.entries[entry].children.push(child);
            self.index(child);
        }
    }

    /// Whether `binding` is one the host types as a date or a date-time.
    pub fn is_time_typed(&self, binding: Option<&AttributeBinding>) -> bool {
        binding
            .and_then(|b| self.binding_index(b))
            .is_some_and(|i| self.tree_data().time_typed.contains(&i))
    }

    /// Whether `binding` is read as text (FBL 0.5 section 5.2).
    pub fn is_text_read(&self, binding: Option<&AttributeBinding>) -> bool {
        binding
            .and_then(|b| self.binding_index(b))
            .is_some_and(|i| self.tree_data().text_read.contains(&i))
    }
}
