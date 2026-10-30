//! A body read through a binding (FBL section 5; the standalone host's `Rules/BodyReading`): the family's lossless
//! reading, which rule claimed which entry, the elements and relations with where each value lives, and the
//! findings. Reading is a pure function of the bytes, the binding and the options, and never fails on content
//! (FBL section 7.4).

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use adp_cel::{Map, Program, Value, Variables};
use serde_json::Value as Json;

use crate::documents::loader::scalar_text;
use crate::documents::types::{AttributeBinding, FblBinding, Slot};
use crate::expressions::{self, CelContext};
use crate::families::{EntryId, EntryKind, Family, FamilyKind};
use crate::model::{
    FblElement, FblModel, FblOptions, FblView, Finding, FindingSeverity, IdRequest, RuleEvent, SourceLocation, codes,
};
use crate::planning::new_text;
use crate::rules::read_mode::{self, ReadMode};
use crate::rules::{Candidate, Node, ReadElement, SlotRead, capture, scalar_conversion};
use crate::text::{BodyText, Span};

/// The value a binding's JSON gives, as an attribute value.
pub(crate) fn from_json(value: &Json) -> Value {
    match value {
        Json::String(s) => Value::from(s.as_str()),
        Json::Number(n) => n
            .as_i64()
            .map(Value::Int)
            .unwrap_or_else(|| Value::Double(n.as_f64().unwrap_or(0.0))),
        Json::Bool(b) => Value::Bool(*b),
        _ => Value::Null,
    }
}

/// Per reference, the first element of each key.
type KeyIndex = HashMap<String, usize>;

/// How a lazily made CEL variable is made.
type MakeVariable = fn(&Family, &Candidate) -> Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ParentKey {
    Element(usize),
    Entry(EntryId),
}

/// A body read through a binding.
#[derive(Debug)]
pub struct BodyReading {
    pub(crate) family: Family,
    /// Every element read, by index; `order` lists those that are in the reading, in document order.
    elements: Vec<ReadElement>,
    order: Vec<usize>,
    claims: HashMap<EntryId, String>,
    by_entry: HashMap<EntryId, usize>,
    /// The views the body's view blocks define (FBL section 4.7), in document order.
    views: Vec<FblView>,
    /// The values of the binding's resource capture (FBL section 8.2) in document order.
    resources: Vec<String>,
    resource: Option<String>,
    unreadable: Option<(usize, String)>,
    findings: Vec<Finding>,
    programs: RefCell<HashMap<String, Option<Rc<Program>>>>,
    parents: RefCell<HashMap<ParentKey, Value>>,
    referenced: RefCell<HashMap<(usize, usize), Rc<KeyIndex>>>,
}

impl BodyReading {
    pub fn read(bytes: Vec<u8>, binding: Rc<FblBinding>, options: Rc<FblOptions>) -> BodyReading {
        let text = BodyText::new(bytes);
        let family = Family::create(text, binding.clone(), options.clone());
        let mut reading = BodyReading {
            family,
            elements: Vec::new(),
            order: Vec::new(),
            claims: HashMap::new(),
            by_entry: HashMap::new(),
            views: Vec::new(),
            resources: Vec::new(),
            resource: None,
            unreadable: None,
            findings: Vec::new(),
            programs: RefCell::new(HashMap::new()),
            parents: RefCell::new(HashMap::new()),
            referenced: RefCell::new(HashMap::new()),
        };
        let length = reading.family.text.len();
        if length > options.max_body_bytes {
            return reading.make_unreadable(
                0,
                format!(
                    "The body is larger than the {} bytes this host reads, so it is not read at all.",
                    options.max_body_bytes
                ),
            );
        }
        if !reading.family.text.is_valid_utf8() {
            let offset = reading.family.text.invalid_offset();
            return reading.make_unreadable(offset, "The body is not valid UTF-8.".into());
        }
        reading.family.parse();
        if let Some((offset, message)) = reading.family.unreadable.clone() {
            return reading.make_unreadable(offset, message);
        }
        if reading.family.order.len() > options.max_entries {
            return reading.make_unreadable(
                0,
                format!(
                    "The body has more than the {} entries this host reads, so it is not read at all.",
                    options.max_entries
                ),
            );
        }
        if let Some(header) = &binding.header
            && !reading.family.header_holds(header)
        {
            let mark = match &header.key {
                Some(key) => format!(
                    "'{key}: {}'",
                    header.value.as_ref().map(scalar_text).unwrap_or_default()
                ),
                None => "its first line".to_owned(),
            };
            if header.required {
                return reading.make_unreadable(
                    0,
                    format!("The body does not start with the mark the binding requires ({mark})."),
                );
            }
            let text = &reading.family.text;
            let span = Span::new(text.bom_length(), text.lines()[0].content_end);
            reading.family.report(
                codes::HEADER_MISMATCH,
                FindingSeverity::Warning,
                format!("The body does not carry the binding's header mark ({mark}); it is read anyway."),
                Some(span),
            );
        }
        reading.claim();
        reading.resolve();
        if options.report_unbound_keys {
            reading.report_unbound_keys();
        }
        let claims = reading.claims.clone();
        reading.family.after_read(&|e| claims.get(&e).cloned());
        let mut findings = reading.family.findings.borrow().clone();
        findings.append(&mut reading.findings);
        reading.findings = findings;
        reading.compact();
        reading
    }

    /// Gives back the room the reading's lists grew into and no longer need: a list that doubled as it grew holds up
    /// to twice what it keeps, and a reading is kept for as long as its body is open. Nothing read changes.
    fn compact(&mut self) {
        self.elements.shrink_to_fit();
        self.order.shrink_to_fit();
        self.family.compact();
    }

    fn binding(&self) -> &FblBinding {
        &self.family.binding
    }

    fn options(&self) -> &FblOptions {
        &self.family.options
    }

    pub(crate) fn unreadable(&self) -> Option<&(usize, String)> {
        self.unreadable.as_ref()
    }

    pub(crate) fn binding_rc(&self) -> &Rc<FblBinding> {
        &self.family.binding
    }

    /// The gaps of the body no part of the reading accounts for (FBL section 4.1).
    pub fn unaccounted(&self) -> Vec<Span> {
        self.family.unaccounted()
    }

    /// Whether the body could not be read, and why.
    pub fn unreadable_message(&self) -> Option<&str> {
        self.unreadable.as_ref().map(|(_, m)| m.as_str())
    }

    pub(crate) fn claimed_by(&self, entry: EntryId) -> Option<&str> {
        self.claims.get(&entry).map(String::as_str)
    }

    pub(crate) fn element_of(&self, entry: EntryId) -> Option<usize> {
        self.by_entry.get(&entry).copied()
    }

    pub(crate) fn element(&self, index: usize) -> &ReadElement {
        &self.elements[index]
    }

    /// The elements and relations in document order, by index.
    pub(crate) fn element_indices(&self) -> &[usize] {
        &self.order
    }

    pub(crate) fn elements(&self) -> impl Iterator<Item = &ReadElement> {
        self.order.iter().map(|i| &self.elements[*i])
    }

    pub(crate) fn find(&self, id: &str) -> Option<usize> {
        self.order.iter().copied().find(|i| self.elements[*i].id == id)
    }

    fn make_unreadable(mut self, offset: usize, message: String) -> BodyReading {
        self.claims.clear();
        self.by_entry.clear();
        self.elements.clear();
        self.order.clear();
        self.findings.clear();
        let (line, column) = self.family.text.position(offset.min(self.family.text.len()));
        self.findings.push(Finding {
            code: codes::UNPARSEABLE.into(),
            severity: FindingSeverity::Error,
            message: message.clone(),
            location: Some(SourceLocation {
                file: self.options().file_name.clone(),
                line,
                column,
                length: 0,
            }),
            detail: None,
        });
        self.unreadable = Some((offset, message));
        self
    }

    /// Reports each key of a read entry that nothing in the binding reads: not an attribute of a rule that reads
    /// there, not its id, not a key such a rule writes, and not the container of a rule that reads further down. In
    /// xml that is an attribute or a child element of a read element. What is reported is kept as it is.
    fn report_unbound_keys(&self) {
        let mut known: HashMap<String, (HashSet<String>, HashSet<String>, bool)> = HashMap::new();
        let mut seen: HashSet<EntryId> = HashSet::new();
        for element in self.elements() {
            let Some(at) = &self.binding().rules[element.rule].at else {
                continue;
            };
            if !seen.insert(element.entry()) {
                continue;
            }
            let names = known.entry(at.clone()).or_insert_with(|| self.known_names(at)).clone();
            let entry = &self.family.entries[element.entry()];
            match &entry.kind {
                // What no rule of a grammar body maps is kept and binds nothing (FBL 0.9 section 4.9).
                EntryKind::Grammar(_) => {}
                EntryKind::Tree(tree) => {
                    let value = self.family.tree_value(tree.value);
                    if value.kind != crate::families::tree::ValueKind::Mapping || value.via_alias {
                        continue;
                    }
                    for &member in &value.entries {
                        let m = &self.family.entries[member];
                        if let Some(key) = &m.name
                            && !names.0.contains(key)
                            && !names.1.contains(key)
                        {
                            self.unbound("key", key, m.tree().key_span.unwrap_or(m.own));
                        }
                    }
                }
                EntryKind::Xml(xml) => {
                    for attribute in &xml.attributes {
                        // A namespace declaration and an xml: attribute are the format's own.
                        if !names.0.contains(&attribute.name) && !attribute.name.starts_with("xml") {
                            self.unbound("attribute", &attribute.name, attribute.own);
                        }
                    }
                    for child in xml.elements() {
                        let c = &self.family.entries[child];
                        if !names.2
                            && let Some(name) = &c.name
                            && !names.1.contains(name)
                        {
                            self.unbound("element", name, c.xml().start_tag);
                        }
                    }
                }
                EntryKind::Statement(_) => {}
            }
        }
    }

    fn unbound(&self, kind: &str, name: &str, at: Span) {
        self.family.report(
            codes::UNBOUND_KEY,
            FindingSeverity::Info,
            format!("The {kind} '{name}' is not one this file's binding reads; it is kept as it is."),
            Some(at),
        );
    }

    /// What the binding knows of an entry read at a path: the keys the rules there read and write, and the next
    /// step of every rule that reads further down. A step that is not a plain name admits any child.
    fn known_names(&self, at: &str) -> (HashSet<String>, HashSet<String>, bool) {
        let mut keys = HashSet::new();
        let mut children = HashSet::new();
        let mut any_child = false;
        let prefix = if at.ends_with('/') {
            at.to_owned()
        } else {
            format!("{at}/")
        };
        let mut step = |path: &str, children: &mut HashSet<String>| {
            let step = path.split('/').next().unwrap_or_default();
            let name = step.split('[').next().unwrap_or_default();
            if name.is_empty() || name == "*" || name.contains('{') {
                any_child = true;
            } else {
                children.insert(name.to_owned());
            }
        };
        for rule in self.binding().all_rules() {
            if rule.at.as_deref() == Some(at) {
                let slots = rule
                    .attributes
                    .iter()
                    .map(|(_, a)| &a.slot)
                    .chain(rule.id.as_ref().and_then(|i| i.from.as_ref()));
                for slot in slots {
                    if let Some(child) = &slot.child {
                        step(child, &mut children);
                    } else if let Some(key) = slot.key.as_ref().or(slot.xml_attribute.as_ref()) {
                        keys.insert(key.clone());
                    }
                }
                if let Some(insert) = &rule.insert {
                    keys.extend(insert.keys.iter().cloned());
                }
            } else if let Some(below) = &rule.at
                && let Some(rest) = below.strip_prefix(&prefix)
            {
                step(rest, &mut children);
            }
        }
        if at == "/" {
            // The root also holds the marks a file is known by, and may name its schema.
            if let Some(header) = self.binding().header.as_ref().and_then(|h| h.key.clone()) {
                keys.insert(header);
            }
            if let Some(marker) = self.binding().claims.marker.as_ref().and_then(|m| m.root_key.clone()) {
                keys.insert(marker);
            }
            keys.insert("$schema".into());
        }
        (keys, children, any_child)
    }

    // ---- claiming entries (FBL section 5.1) ----

    fn claim(&mut self) {
        let mut offered: HashMap<EntryId, Vec<Candidate>> = HashMap::new();
        for block in 0..self.binding().blocks.len() {
            for candidate in self.family.block_candidates(block) {
                offered.entry(candidate.entry).or_default().push(candidate);
            }
        }
        for rule in 0..self.binding().rules.len() {
            for candidate in self.family.candidates(rule) {
                offered.entry(candidate.entry).or_default().push(candidate);
            }
        }
        let order = self.family.order.clone();
        for entry in order {
            let Some(candidates) = offered.get(&entry) else {
                continue;
            };
            let mut error: Option<String> = None;
            for candidate in candidates {
                let binding = self.family.binding.clone();
                let within = match (candidate.rule, candidate.block) {
                    (Some(r), _) => binding.rules[r].within.as_deref(),
                    (None, Some(b)) => binding.blocks[b].within.as_deref(),
                    _ => None,
                };
                let claims = &self.claims;
                if !self.family.admits(within, entry, &|e| claims.get(&e).cloned()) {
                    continue;
                }
                if let Some(rule_index) = candidate.rule {
                    let rule = &binding.rules[rule_index];
                    if !self.selected_resource(candidate) {
                        continue;
                    }
                    if let Some(when) = &rule.when {
                        let (holds, problem) = self.evaluate(when, candidate);
                        if let Some(problem) = problem {
                            error.get_or_insert(problem);
                        }
                        if !matches!(holds, Some(Value::Bool(true))) {
                            continue;
                        }
                    }
                    if self.options().unreadable_types.contains(&rule.type_name) {
                        // The specification maps the rule's type as unreadable (FBL 0.7 section 4.8, DESL section
                        // 5.2): the entry is the rule's, an unreadable entry, and what it holds is not looked into.
                        error = Some("it is in no form this file's entries are written in.".into());
                        break;
                    }
                    let line = self.family.text.position(self.family.entries[entry].own.start).0;
                    let mut element = ReadElement {
                        rule: rule_index,
                        candidate: candidate.clone(),
                        id: String::new(),
                        id_stored: false,
                        id_read: None,
                        key: String::new(),
                        key_attribute: None,
                        slots: Vec::new(),
                        attributes: Map::default(),
                        parent: None,
                        position_in_slot: 0,
                        source_read: None,
                        target_read: None,
                        source_element: None,
                        target_element: None,
                        line,
                        mistyped: None,
                        unconverted: Vec::new(),
                        is_relation: rule.is_relation,
                    };
                    self.read_slots(&mut element);
                    if let Some(mistyped) = element.mistyped.clone() {
                        // The entry is this rule's and no other's: it is not offered to a later candidate.
                        error = Some(mistyped);
                        break;
                    }
                    for (message, span, attribute, text) in &element.unconverted {
                        self.family.report_with(
                            codes::UNCONVERTED_VALUE,
                            FindingSeverity::Warning,
                            message.clone(),
                            *span,
                            Some(vec![
                                ("attribute".into(), attribute.clone()),
                                ("text".into(), text.clone()),
                            ]),
                        );
                    }
                    self.claims.insert(entry, rule.name.clone());
                    self.elements.push(element);
                    let index = self.elements.len() - 1;
                    self.by_entry.insert(entry, index);
                    self.order.push(index);
                } else if let Some(block_index) = candidate.block {
                    let block = &binding.blocks[block_index];
                    self.claims.insert(entry, block.name.clone());
                    if let Some(group) = &block.view
                        && let Some(view) = capture(&candidate.captures, group)
                    {
                        let own = self.family.entries[entry].own;
                        self.views.push(FblView {
                            name: view.to_owned(),
                            block: block.name.clone(),
                            span: own,
                            line: self.family.text.position(own.start).0,
                        });
                    }
                }
                error = None;
                break;
            }
            if let Some(error) = error {
                let own = self.family.entries[entry].own;
                self.family.report(
                    codes::UNREADABLE_ENTRY,
                    FindingSeverity::Warning,
                    format!("This entry cannot be read: {error}"),
                    Some(own),
                );
            }
        }
    }

    /// The registration's `resource` header selects one value of the binding's resource capture; without it, the
    /// first value in document order (FBL section 8.2).
    fn selected_resource(&mut self, candidate: &Candidate) -> bool {
        let Some(name) = self.binding().registration.resource_capture.clone() else {
            return true;
        };
        let Some(value) = capture(&candidate.captures, &name).map(str::to_owned) else {
            return true;
        };
        if !self.resources.contains(&value) {
            self.resources.push(value.clone());
        }
        if self.resource.is_none() {
            self.resource = Some(self.options().resource.clone().unwrap_or_else(|| value.clone()));
        }
        self.resource.as_deref() == Some(value.as_str())
    }

    // ---- CEL ----

    fn program(&self, expression: &str, context: CelContext) -> Result<Rc<Program>, String> {
        let key = format!("{context:?}:{expression}");
        if let Some(cached) = self.programs.borrow().get(&key) {
            return cached.clone().ok_or_else(String::new);
        }
        let compiled = expressions::compile(expression, context);
        let cached = compiled.as_ref().ok().cloned().map(Rc::new);
        self.programs.borrow_mut().insert(key, cached.clone());
        match compiled {
            Ok(_) => Ok(cached.expect("a compiled program")),
            Err(e) => Err(e),
        }
    }

    fn rule_context(&self) -> CelContext {
        match &self.family.kind {
            FamilyKind::Lines(_) => CelContext::Lines,
            FamilyKind::Yaml(_) | FamilyKind::Json(_) => CelContext::Scalars,
            FamilyKind::Xml(_) | FamilyKind::Grammar(_) => CelContext::Tree,
        }
    }

    /// The value of `expression` for `candidate`, or the problem that kept it from one.
    fn evaluate(&self, expression: &str, candidate: &Candidate) -> (Option<Value>, Option<String>) {
        let program = match self.program(expression, self.rule_context()) {
            Ok(program) => program,
            Err(problem) => return (None, Some(problem)),
        };
        let variables = self.variables(candidate);
        match program.evaluate(&variables) {
            Ok(value) => (Some(value), None),
            Err(problem) => (None, Some(problem)),
        }
    }

    /// Whether `insert.when` admits a new entry with `attributes`.
    pub(crate) fn insert_allowed(&self, expression: &str, attributes: &Map) -> bool {
        let Ok(program) = self.program(expression, CelContext::Insert) else {
            return false;
        };
        let mut variables = HashMap::new();
        variables.insert("attributes".to_owned(), Value::map(attributes.clone()));
        program.is_true(&variables)
    }

    fn variables<'a>(&'a self, candidate: &'a Candidate) -> CelVariables<'a> {
        let mut values: Vec<(&'static str, Value)> = vec![
            ("entry", self.family.cel_value(candidate)),
            (
                "line",
                Value::Int(
                    self.family
                        .text
                        .position(self.family.entries[candidate.entry].own.start)
                        .0 as i64,
                ),
            ),
        ];
        let registration: Map = self
            .options()
            .registration_headers
            .iter()
            .map(|(k, v)| (k.clone(), Value::from(v.as_str())))
            .collect();
        values.push(("registration", Value::map(registration)));
        let (name, extra) = self.family.cel_extra(candidate);
        values.push((name, extra));
        let mut parent = Value::Null;
        if let Some(&enclosing) = self.family.enclosing(candidate.entry).first() {
            parent = match self.by_entry.get(&enclosing) {
                Some(&element) => self.parent_value(ParentKey::Element(element), || {
                    self.family.cel_value(&self.elements[element].candidate)
                }),
                None => self.parent_value(ParentKey::Entry(enclosing), || {
                    self.family.cel_value(&Candidate {
                        rule: None,
                        block: None,
                        entry: enclosing,
                        captures: Rc::default(),
                    })
                }),
            };
        }
        values.push(("parent", parent));
        CelVariables {
            values,
            reading: self,
            candidate,
            lazy: self.rule_context() == CelContext::Scalars,
            text: RefCell::new(None),
            style: RefCell::new(None),
        }
    }

    /// The CEL value of a parent, by the element or the entry it is a pure function of, converted on first use.
    fn parent_value(&self, key: ParentKey, convert: impl FnOnce() -> Value) -> Value {
        if let Some(value) = self.parents.borrow().get(&key) {
            return value.clone();
        }
        let value = convert();
        self.parents.borrow_mut().insert(key, value.clone());
        value
    }

    // ---- slots (FBL section 5.2) ----

    fn read_slots(&self, element: &mut ReadElement) {
        let binding = self.family.binding.clone();
        let rule = &binding.rules[element.rule];
        // Told before each attribute is read, with those read so far, and once all are.
        let observe = |element: &ReadElement, first: bool| {
            if let Some(observer) = &self.options().rule_observer {
                observer(&RuleEvent {
                    rule: &rule.name,
                    type_name: &rule.type_name,
                    line: element.line,
                    attributes: &element.attributes,
                    first,
                });
            }
        };
        let mut first = true;
        for (name, attribute) in &rule.attributes {
            observe(element, first);
            first = false;
            let many = attribute.word.is_some() && !attribute.flag && self.options().many(&rule.type_name, name);
            let read = if many {
                self.read_many(&element.candidate, attribute, element.rule)
            } else {
                self.read_slot(&element.candidate, &attribute.slot, Some(attribute), element.rule)
            };
            element.slots.push((name.clone(), read.clone()));
            let mut value = if read.present { read.value.clone() } else { Value::Null };
            let mode = read_mode::of(rule, name, attribute, self.options());
            let mut scalar = read.present && read.written_scalar && !attribute.flag && self.family.is_tree();
            // A slot read as text takes what the author wrote (FBL 0.5 section 5.2), which a map then translates.
            if scalar && mode == ReadMode::Text {
                value = Value::from(read.written());
            }
            if read.present
                && let Some(map) = &attribute.map
                && let Some(wire) = value.as_str()
                && let Some((_, mapped)) = map
                    .iter()
                    .find(|(k, _)| k == wire || (rule.case_insensitive && k.to_uppercase() == wire.to_uppercase()))
            {
                value = Value::from(mapped.as_str());
                scalar = false;
            }
            if attribute.flag {
                value = Value::Bool(read.present);
            } else if read.present
                && let Some(reference) = &attribute.reference
                && !reference.to.contains(&rule.name)
                && let Some(words) = &read.words
                && !words.is_empty()
            {
                value = Value::list(words.iter().map(|w| Value::from(w.text.as_str())).collect());
            }
            if read.present
                && !attribute.flag
                && read.items.is_none()
                && mode == ReadMode::Typed
                && let Some(type_name) = self.options().attribute_type(&rule.type_name, name)
            {
                if scalar && read.value.is_null() && read.style == Some("plain") && read.written().is_empty() {
                    // Nothing written (key:) is absent, so the default applies (FBL 0.5 section 5.2).
                    if let Some(empty) = &attribute.default {
                        element.attributes.insert(name.clone(), from_json(empty));
                    }
                    continue;
                }
                value = self.convert(element, name, attribute, &read, &type_name, value, scalar);
            }
            if !read.present
                && !attribute.flag
                && let Some(fallback) = &attribute.default
            {
                value = from_json(fallback);
            }
            if read.present || attribute.flag || attribute.default.is_some() {
                element.attributes.insert(name.clone(), value);
            }
        }
        observe(element, first);
        if let Some(source) = &rule.source {
            element.source_read = Some(Box::new(self.read_slot(
                &element.candidate,
                &source.slot,
                Some(source),
                element.rule,
            )));
        }
        if let Some(target) = &rule.target {
            element.target_read = Some(Box::new(self.read_slot(
                &element.candidate,
                &target.slot,
                Some(target),
                element.rule,
            )));
        }
        if let Some(from) = rule.id.as_ref().and_then(|i| i.from.as_ref()) {
            element.id_read = Some(Box::new(self.read_slot(&element.candidate, from, None, element.rule)));
        }
    }

    /// A typed slot's value converted to its attribute's type (FBL 0.5 section 5.2): the converted value, else the
    /// fallback, reported once the entry is claimed, else the value with the element marked mistyped.
    #[allow(clippy::too_many_arguments)]
    fn convert(
        &self,
        element: &mut ReadElement,
        name: &str,
        attribute: &AttributeBinding,
        read: &SlotRead,
        type_name: &str,
        value: Value,
        scalar: bool,
    ) -> Value {
        let quoted = attribute.quoted.as_deref() == Some("convert") && self.family.is_yaml();
        let style = if scalar { read.style } else { None };
        if let Some(converted) = scalar_conversion::convert(type_name, &value, &read.written(), style, quoted) {
            return converted;
        }
        let Some(fallback) = &attribute.fallback else {
            element
                .mistyped
                .get_or_insert_with(|| format!("'{name}' is not a {type_name}."));
            return value;
        };
        let replacement = from_json(&fallback.value);
        if fallback.report {
            let kind = scalar_conversion::describe(type_name);
            let mut capitalised = name.chars();
            let capitalised = match capitalised.next() {
                Some(first) => first.to_uppercase().collect::<String>() + capitalised.as_str(),
                None => String::new(),
            };
            let message = format!(
                "'{}' is not {kind}, so {capitalised} is read as {}. Write {kind} to change it.",
                read.written(),
                new_text::plain(&replacement, None)
            );
            element
                .unconverted
                .push((message, read.span, name.to_owned(), read.written()));
        }
        replacement
    }

    /// Reads a slot of an element of rule `rule`, read-only where the attribute, the rule or the binding says so.
    pub(crate) fn read_slot(
        &self,
        candidate: &Candidate,
        slot: &Slot,
        attribute: Option<&AttributeBinding>,
        rule: usize,
    ) -> SlotRead {
        let mut read = self.read_slot_unchecked(candidate, slot, attribute, rule);
        let reason = attribute
            .and_then(|a| a.read_only.clone())
            .or_else(|| self.binding().rules[rule].read_only.clone())
            .or_else(|| self.binding().read_only.clone());
        if let Some(reason) = reason
            && read.writable
        {
            read.writable = false;
            read.reason = Some(reason);
        }
        read
    }

    /// Reads a word slot of an attribute the tool type's specification gives `many` (FBL 0.7 section 4.6): every
    /// word of the group that matches, each an item, read-only as [`Self::read_slot`] says.
    fn read_many(&self, candidate: &Candidate, attribute: &AttributeBinding, rule: usize) -> SlotRead {
        let mut read = self.family.read_many(candidate, &attribute.slot);
        if read.items.is_none() {
            return self.read_slot(candidate, &attribute.slot, Some(attribute), rule);
        }
        let reason = attribute
            .read_only
            .clone()
            .or_else(|| self.binding().rules[rule].read_only.clone())
            .or_else(|| self.binding().read_only.clone());
        if let Some(reason) = reason {
            read.writable = false;
            read.reason = Some(reason);
        }
        read
    }

    fn read_slot_unchecked(
        &self,
        candidate: &Candidate,
        slot: &Slot,
        attribute: Option<&AttributeBinding>,
        rule: usize,
    ) -> SlotRead {
        if let Some(expression) = &slot.value {
            let (value, problem) = self.evaluate(expression, candidate);
            return match problem {
                None => SlotRead::new(value.unwrap_or_default(), None, true, false)
                    .with_reason("The value is computed from the file."),
                Some(problem) => SlotRead::read_only_absent(format!("The value cannot be computed: {problem}")),
            };
        }
        if let Some(name) = &slot.parent {
            for enclosing in self.family.enclosing(candidate.entry) {
                let Some(&parent) = self.by_entry.get(&enclosing) else {
                    continue;
                };
                let parent = &self.elements[parent];
                if let Some(containment) = &self.binding().rules[rule].parent
                    && !containment.rules.contains(&self.binding().rules[parent.rule].name)
                {
                    continue;
                }
                return match parent.slot(name) {
                    Some(read) => read.clone(),
                    None => self.family.read_raw(enclosing, name),
                };
            }
            return SlotRead::read_only_absent("No enclosing entry holds this value.");
        }
        if let Some(attribute) = attribute
            && let Some(over) = &attribute.override_slot
        {
            let overriding_slot = Slot {
                key: over.key.clone(),
                xml_attribute: over.xml_attribute.clone(),
                text: over.text,
                child: over.child.clone(),
                group: over.group.clone(),
                capture: over.capture.clone(),
                word: over.word.clone(),
                ..Slot::default()
            };
            let mut overriding = self.family.read(candidate, &overriding_slot, attribute.html_paragraphs);
            if overriding.present {
                overriding.node = Node::Override(Box::new(std::mem::take(&mut overriding.node)));
                return overriding;
            }
        }
        self.family
            .read(candidate, slot, attribute.is_some_and(|a| a.html_paragraphs))
    }

    // ---- ids, references, containment (FBL sections 5.3 to 5.5) ----

    fn resolve(&mut self) {
        // Containment first: it needs no id, and an id derived by the caller may be built from the id of the
        // element that contains this one. Elements are in document order, so a parent has its id before any of its
        // children is asked for theirs.
        let binding = self.family.binding.clone();
        for &index in &self.order {
            let Some(containment) = &binding.rules[self.elements[index].rule].parent else {
                continue;
            };
            for enclosing in self.family.enclosing(self.elements[index].entry()) {
                if let Some(&parent) = self.by_entry.get(&enclosing)
                    && containment
                        .rules
                        .contains(&binding.rules[self.elements[parent].rule].name)
                {
                    self.elements[index].parent = Some(parent);
                    break;
                }
            }
        }
        let mut positions: HashMap<(Option<usize>, Option<String>), usize> = HashMap::new();
        for &index in &self.order {
            let element = &self.elements[index];
            let slot = (
                element.parent,
                element
                    .parent
                    .and(binding.rules[element.rule].parent.as_ref().and_then(|p| p.slot.clone())),
            );
            let position = positions.get(&slot).copied().unwrap_or(0);
            positions.insert(slot, position + 1);
            self.elements[index].position_in_slot = position;
        }
        let (relations, elements): (Vec<usize>, Vec<usize>) =
            self.order.iter().partition(|i| self.elements[**i].is_relation);
        for &index in &elements {
            self.assign_id(index);
        }
        for &index in &elements {
            let key = self.key_of(index);
            self.elements[index].key = key;
        }
        let mut dangling = Vec::new();
        for &index in &relations {
            let source = self.end(index, true);
            let target = self.end(index, false);
            self.elements[index].source_element = source;
            self.elements[index].target_element = target;
            if source.is_none() || target.is_none() {
                dangling.push(index);
            }
        }
        for index in dangling {
            self.order.retain(|i| *i != index);
            let entry = self.elements[index].entry();
            self.by_entry.remove(&entry);
        }
        let order = self.order.clone();
        let relations: Vec<usize> = order
            .iter()
            .copied()
            .filter(|i| self.elements[*i].is_relation)
            .collect();
        for &index in &relations {
            self.assign_id(index);
            let key = self.key_of(index);
            self.elements[index].key = key;
        }
        let mut seen: HashSet<String> = HashSet::new();
        for &index in &order {
            if seen.insert(self.elements[index].id.clone()) {
                continue;
            }
            let element = &self.elements[index];
            self.family.report(
                codes::DUPLICATE_ID,
                FindingSeverity::Warning,
                format!(
                    "Another entry already has the id '{}'; this one is addressed by its place.",
                    element.id
                ),
                Some(self.family.entries[element.entry()].own),
            );
            let place = self.place_id(index);
            self.elements[index].id = place.clone();
            self.elements[index].id_stored = false;
            seen.insert(place);
        }
        for &index in &order {
            self.check_references(index);
        }
    }

    fn place_id(&self, index: usize) -> String {
        let element = &self.elements[index];
        format!("{}@{}", self.binding().rules[element.rule].name, element.line)
    }

    fn assign_id(&mut self, index: usize) {
        let binding = self.family.binding.clone();
        let element = &self.elements[index];
        let rule = &binding.rules[element.rule];
        if rule.id.as_ref().is_some_and(|i| i.from.is_some()) {
            // The id is the slot's written text (FBL 0.5 section 5.3): `id: 007` is 007, `id: ~` is ~, and `id:`
            // has none.
            if let Some(read) = &element.id_read
                && read.present
            {
                let id = read.written();
                if !id.is_empty() {
                    self.elements[index].id = id;
                    self.elements[index].id_stored = true;
                    return;
                }
            }
            self.family.report(
                codes::MISSING_ID,
                FindingSeverity::Warning,
                format!("This {} has no id; it is addressed by its place.", rule.type_name),
                Some(self.family.entries[element.entry()].own),
            );
            self.elements[index].id = self.place_id(index);
            return;
        }
        if let Some(key_expression) = rule.id.as_ref().and_then(|i| i.sidecar_key.as_deref()) {
            let (key, _) = self.evaluate(key_expression, &element.candidate);
            if let Some(key) = key
                && !key.is_null()
                && let Some(stored) = self.options().identities.get(&new_text::plain(&key, None))
            {
                let stored = stored.clone();
                self.elements[index].id = stored;
                self.elements[index].id_stored = true;
                return;
            }
        }
        let parent_id = element.parent.map(|p| self.elements[p].id.clone());
        let source_id = element.source_element.map(|p| self.elements[p].id.clone());
        let target_id = element.target_element.map(|p| self.elements[p].id.clone());
        let parent_slot = element.parent.and(rule.parent.as_ref().and_then(|p| p.slot.clone()));
        let derived = self.options().derive_id.as_ref().and_then(|derive| {
            derive(&IdRequest {
                rule: &rule.name,
                type_name: &rule.type_name,
                attributes: &element.attributes,
                source: source_id.as_deref(),
                target: target_id.as_deref(),
                line: element.line,
                parent_id: parent_id.as_deref(),
                parent_slot: parent_slot.as_deref(),
                position_in_slot: element.position_in_slot,
            })
        });
        let id = derived.unwrap_or_else(|| self.place_id(index));
        self.elements[index].id = id;
    }

    /// The value references name an element by: the attribute its own rule, else any rule of the binding,
    /// references it by, else its stored id, else its id (FBL section 5.7).
    fn key_of(&mut self, index: usize) -> String {
        let binding = self.family.binding.clone();
        let rule = &binding.rules[self.elements[index].rule];
        for (name, attribute) in &rule.attributes {
            if let Some(reference) = &attribute.reference
                && reference.to.contains(&rule.name)
                && reference.by == *name
            {
                self.elements[index].key_attribute = Some(name.clone());
                return self.elements[index].text_of(rule, name);
            }
        }
        for other in binding.all_rules() {
            for (_, attribute) in &other.attributes {
                let Some(reference) = &attribute.reference else {
                    continue;
                };
                if reference.by == "id" || !reference.to.contains(&rule.name) {
                    continue;
                }
                if rule.attribute(&reference.by).is_none() {
                    continue;
                }
                self.elements[index].key_attribute = Some(reference.by.clone());
                return self.elements[index].text_of(rule, &reference.by);
            }
        }
        let element = &self.elements[index];
        if rule.id.as_ref().is_some_and(|i| i.from.is_some())
            && let Some(read) = &element.id_read
            && read.present
        {
            return read.written();
        }
        element.id.clone()
    }

    fn end(&self, index: usize, source: bool) -> Option<usize> {
        let relation = &self.elements[index];
        let rule = &self.binding().rules[relation.rule];
        let end = if source { "source" } else { "target" };
        let read = if source {
            &relation.source_read
        } else {
            &relation.target_read
        };
        let own = self.family.entries[relation.entry()].own;
        let Some(read) = read.as_ref().filter(|r| r.present && !r.value.is_null()) else {
            self.family.report(
                codes::DANGLING_REFERENCE,
                FindingSeverity::Warning,
                format!("This {} has no {end}, so it is not read as a relation.", rule.type_name),
                Some(own),
            );
            return None;
        };
        let key = read.written();
        let found = self
            .order
            .iter()
            .copied()
            .find(|i| !self.elements[*i].is_relation && self.elements[*i].key == key);
        if found.is_none() {
            self.family.report(
                codes::DANGLING_REFERENCE,
                FindingSeverity::Warning,
                format!(
                    "The {end} '{key}' of this {} names no element, so it is not read as a relation.",
                    rule.type_name
                ),
                Some(own),
            );
        }
        found
    }

    fn check_references(&self, index: usize) {
        let element = &self.elements[index];
        let rule = &self.binding().rules[element.rule];
        for (a, (name, attribute)) in rule.attributes.iter().enumerate() {
            let Some(reference) = &attribute.reference else {
                continue;
            };
            if reference.to.contains(&rule.name) {
                continue;
            }
            let Some(read) = element.slot(name).filter(|r| r.present) else {
                continue;
            };
            let names: Vec<String> = match element.attributes.get(name) {
                Some(Value::List(list)) => list.iter().map(|v| new_text::plain(v, None)).collect(),
                _ => vec![element.text_of(rule, name)],
            };
            for key in names {
                if self.referenced_by((element.rule, a), &key).is_none() {
                    self.family.report(
                        codes::DANGLING_REFERENCE,
                        FindingSeverity::Warning,
                        format!("'{key}' in {name} names no {}.", reference.to.join(" or ")),
                        Some(read.span.unwrap_or(self.family.entries[element.entry()].own)),
                    );
                }
            }
        }
    }

    /// The first element, in document order, that the reference of attribute `reference` (rule and attribute
    /// index) can name and whose key is `key`.
    pub(crate) fn referenced_by(&self, reference: (usize, usize), key: &str) -> Option<usize> {
        let cached = self.referenced.borrow().get(&reference).cloned();
        let index = match cached {
            Some(index) => index,
            None => {
                let binding = self.binding();
                let r = binding.rules[reference.0].attributes[reference.1]
                    .1
                    .reference
                    .as_ref()
                    .expect("a reference binding");
                let mut index: HashMap<String, usize> = HashMap::new();
                for &i in &self.order {
                    let element = &self.elements[i];
                    let rule = &binding.rules[element.rule];
                    if !r.to.contains(&rule.name) {
                        continue;
                    }
                    // `by: "id"` names an entry by the id its rule stores with `id.from` (FBL 5.2).
                    let named = if r.by == "id" && !element.attributes.contains_key("id") {
                        element.id_stored.then(|| element.id.clone())
                    } else {
                        Some(element.text_of(rule, &r.by))
                    };
                    if let Some(named) = named {
                        index.entry(named).or_insert(i);
                    }
                }
                let index = Rc::new(index);
                self.referenced.borrow_mut().insert(reference, index.clone());
                index
            }
        };
        index.get(key).copied()
    }

    // ---- the public model ----

    pub fn to_model(&self) -> FblModel {
        let binding = self.binding();
        FblModel {
            elements: self
                .elements()
                .map(|e| {
                    let rule = &binding.rules[e.rule];
                    FblElement {
                        id: e.id.clone(),
                        id_is_stored: e.id_stored,
                        type_name: rule.type_name.clone(),
                        rule: rule.name.clone(),
                        is_relation: e.is_relation,
                        attributes: e.attributes.clone(),
                        parent_id: e.parent.map(|p| self.elements[p].id.clone()),
                        parent_slot: e.parent.and(rule.parent.as_ref().and_then(|p| p.slot.clone())),
                        source: e.source_element.map(|p| self.elements[p].id.clone()),
                        target: e.target_element.map(|p| self.elements[p].id.clone()),
                        own_span: self.family.entries[e.entry()].own,
                        line: e.line,
                        file: None,
                    }
                })
                .collect(),
            findings: self.findings.clone(),
            unreadable: self.unreadable.is_some(),
            views: self.views.clone(),
            resources: self.resources.clone(),
        }
    }
}

/// The variables of one CEL evaluation, of which `text` and `style` are made only when the expression reads them.
struct CelVariables<'a> {
    values: Vec<(&'static str, Value)>,
    reading: &'a BodyReading,
    candidate: &'a Candidate,
    lazy: bool,
    text: RefCell<Option<Value>>,
    style: RefCell<Option<Value>>,
}

impl Variables for CelVariables<'_> {
    fn get(&self, name: &str) -> Option<Value> {
        if let Some((_, value)) = self.values.iter().find(|(n, _)| *n == name) {
            return Some(value.clone());
        }
        if !self.lazy {
            return None;
        }
        let (cell, make): (&RefCell<Option<Value>>, MakeVariable) = match name {
            "text" => (&self.text, Family::cel_text),
            "style" => (&self.style, Family::cel_style),
            _ => return None,
        };
        if cell.borrow().is_none() {
            *cell.borrow_mut() = Some(make(&self.reading.family, self.candidate));
        }
        cell.borrow().clone()
    }
}
