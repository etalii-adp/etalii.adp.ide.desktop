//! A node or a relation as DISL's CEL sees it (DISL section 12.2; the standalone host's `DislElement`): `id`,
//! `type`, `kind`, every attribute as a field, the containment members of a node, the ends of a relation, and the
//! methods `isA`, `descendants`, `ancestors`, `childrenOfType`, `incomingOf`, `outgoingOf`, `positionIn`, `label`,
//! `other` and `location`, and the node-specific graph functions of section 12.4 `successors`, `predecessors`,
//! `reachable` and `inCycle`.
//!
//! An unset attribute reads as its default, else its type's zero value, while `has(e.attr)` holds only for a stored
//! value (section 4.3); a derived attribute is computed from its expression whenever it is read. A reference
//! attribute reads as the element its stored id names, or null. Elements compare by identity, never by id: CEL's
//! `==`, `in` and `positionIn` all see two elements with one id as two, which is what lets ids be computed from
//! positions (section 11.5.2) and duplicate ids be found.
//!
//! An element is a handle into its diagram, which owns it: the element reaches the diagram through a weak
//! reference, so a diagram that is dropped takes its elements' model with it.

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt;
use std::rc::{Rc, Weak};

use adp_cel::{Map, Object, Value};

use super::diagram::{DiagramData, DislDiagram};
use super::values;
use super::written::WrittenElement;
use crate::derivation::env::DislEnv;
use crate::derivation::evaluation::vars;
use crate::json;
use crate::loading::metamodel::{DislAttribute, DislType};

/// A node or relation of a diagram; cloning it gives another handle to the same element.
#[derive(Clone)]
pub struct DislElement(pub(crate) Rc<ElementData>);

pub(crate) struct ElementData {
    this: Weak<ElementData>,
    pub(crate) diagram: Weak<DiagramData>,
    pub(crate) state: RefCell<ElementState>,
}

#[derive(Clone)]
pub(crate) struct ElementState {
    pub(crate) element_type: Rc<DislType>,
    pub(crate) id: String,
    pub(crate) attributes: Map,
    pub(crate) parent: Option<Weak<ElementData>>,
    pub(crate) slot: Option<String>,
    pub(crate) children: Vec<DislElement>,
    pub(crate) source: Option<DislElement>,
    pub(crate) target: Option<DislElement>,
    pub(crate) is_derived: bool,
    pub(crate) sources: Vec<DislElement>,
    pub(crate) id_is_stored: bool,
    pub(crate) source_id: Option<String>,
    pub(crate) target_id: Option<String>,
    pub(crate) host_attributes: Map,
    pub(crate) line: Option<i32>,
    pub(crate) file: Option<String>,
    pub(crate) view: Option<Map>,
}

/// What a new element is made of, for [`DislDiagram::add`]: a node beneath its parent, or a relation at its ends,
/// with what a reader recorded about it.
#[derive(Clone)]
pub struct NewElement {
    pub type_name: String,
    pub id: String,
    pub attributes: Map,
    pub parent: Option<DislElement>,
    pub slot: Option<String>,
    pub source: Option<DislElement>,
    pub target: Option<DislElement>,
    /// A relation's source as stored, which names nothing when `source` is none.
    pub source_id: Option<String>,
    /// A relation's target as stored, which names nothing when `target` is none.
    pub target_id: Option<String>,
    pub is_derived: bool,
    pub sources: Vec<DislElement>,
    /// Whether the id was read from storage; false for an id the reader made up or the runtime computed.
    pub id_is_stored: bool,
    /// What the reader read beside the metamodel's attributes; CEL does not see these.
    pub host_attributes: Map,
    /// The 1-based line the reader found the element on.
    pub line: Option<i32>,
    /// The file the reader found the element in, relative to the diagram's subject (section 8.6); none for the
    /// primary file.
    pub file: Option<String>,
}

impl NewElement {
    pub fn new(type_name: &str, id: &str) -> Self {
        NewElement {
            type_name: type_name.to_owned(),
            id: id.to_owned(),
            attributes: Map::default(),
            parent: None,
            slot: None,
            source: None,
            target: None,
            source_id: None,
            target_id: None,
            is_derived: false,
            sources: Vec::new(),
            id_is_stored: true,
            host_attributes: Map::default(),
            line: None,
            file: None,
        }
    }
}

impl fmt::Debug for DislElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_string())
    }
}

impl fmt::Debug for ElementData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = self.state.borrow();
        write!(f, "{} {}", state.element_type.name, state.id)
    }
}

impl fmt::Display for DislElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = self.0.state.borrow();
        write!(f, "{} {}", state.element_type.name, state.id)
    }
}

impl PartialEq for DislElement {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for DislElement {}

impl std::hash::Hash for DislElement {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.key().hash(state);
    }
}

thread_local! {
    /// The derived attributes being computed on this thread, so one that reads itself stops.
    static COMPUTING: RefCell<HashSet<(usize, String)>> = RefCell::new(HashSet::new());
}

/// A type name argument of an element method.
pub(crate) fn type_text(value: &Value) -> Result<&str, String> {
    value.as_str().ok_or_else(|| "A type name was expected.".to_owned())
}

pub(crate) fn list(elements: impl IntoIterator<Item = DislElement>) -> Value {
    Value::list(elements.into_iter().map(|e| e.to_value()).collect())
}

impl DislElement {
    pub(crate) fn make(diagram: &DislDiagram, new: &NewElement, element_type: Rc<DislType>) -> Result<Self, String> {
        for (name, _) in new.attributes.iter() {
            if !element_type.attributes.contains_key(name) {
                return Err(format!("'{name}' is not an attribute of {}.", element_type.name));
            }
        }
        Ok(DislElement(Rc::new_cyclic(|this| ElementData {
            this: this.clone(),
            diagram: Rc::downgrade(&diagram.0),
            state: RefCell::new(ElementState {
                element_type,
                id: new.id.clone(),
                attributes: new.attributes.clone(),
                parent: new.parent.as_ref().map(|p| Rc::downgrade(&p.0)),
                slot: if new.parent.is_some() { new.slot.clone() } else { None },
                children: Vec::new(),
                source: new.source.clone(),
                target: new.target.clone(),
                is_derived: new.is_derived,
                sources: new.sources.clone(),
                id_is_stored: new.id_is_stored,
                source_id: new.source_id.clone(),
                target_id: new.target_id.clone(),
                host_attributes: new.host_attributes.clone(),
                line: new.line,
                file: new.file.clone(),
                view: None,
            }),
        })))
    }

    /// The element's identity, for maps keyed by element.
    pub(crate) fn key(&self) -> usize {
        Rc::as_ptr(&self.0) as usize
    }

    /// The element as a CEL value.
    pub fn to_value(&self) -> Value {
        Value::Object(Rc::clone(&self.0) as Rc<dyn Object>)
    }

    /// The element a CEL value holds, if it holds one (an element shown under its written id is not one).
    pub fn from_value(value: &Value) -> Option<DislElement> {
        value.downcast::<ElementData>().map(DislElement)
    }

    /// The diagram the element is in.
    ///
    /// # Panics
    /// When the diagram has been dropped: an element is a handle into its diagram.
    pub fn diagram(&self) -> DislDiagram {
        self.try_diagram().expect("The element's diagram has been dropped.")
    }

    pub(crate) fn try_diagram(&self) -> Option<DislDiagram> {
        self.0.diagram.upgrade().map(DislDiagram)
    }

    pub fn element_type(&self) -> Rc<DislType> {
        Rc::clone(&self.0.state.borrow().element_type)
    }

    pub fn type_name(&self) -> String {
        self.0.state.borrow().element_type.name.clone()
    }

    /// The element's id: stored, or computed from the id rule once the model is complete (section 11.5).
    pub fn id(&self) -> String {
        self.0.state.borrow().id.clone()
    }

    pub(crate) fn set_id(&self, id: &str) {
        self.0.state.borrow_mut().id = id.to_owned();
    }

    pub fn is_relation(&self) -> bool {
        self.0.state.borrow().element_type.is_relation
    }

    /// `node` or `relation`.
    pub fn kind(&self) -> &'static str {
        if self.is_relation() { "relation" } else { "node" }
    }

    /// The attributes that have a stored value, as CEL values.
    pub fn attributes(&self) -> Map {
        self.0.state.borrow().attributes.clone()
    }

    /// A node's parent; none for a top-level node and for a relation.
    pub fn parent(&self) -> Option<DislElement> {
        self.0
            .state
            .borrow()
            .parent
            .as_ref()
            .and_then(Weak::upgrade)
            .map(DislElement)
    }

    /// The children slot a node is in beneath its parent (section 4.8); none at the top level.
    pub fn slot(&self) -> Option<String> {
        self.0.state.borrow().slot.clone()
    }

    /// A node's children, in model order.
    pub fn children(&self) -> Vec<DislElement> {
        self.0.state.borrow().children.clone()
    }

    /// A relation's source; none for a node, and for a relation whose stored source names nothing.
    pub fn source(&self) -> Option<DislElement> {
        self.0.state.borrow().source.clone()
    }

    /// A relation's target; none for a node, and for a relation without one (section 4.9).
    pub fn target(&self) -> Option<DislElement> {
        self.0.state.borrow().target.clone()
    }

    /// Whether the element is derived (section 4.11) rather than stored.
    pub fn is_derived(&self) -> bool {
        self.0.state.borrow().is_derived
    }

    /// A derived element's sources (section 4.11.4); empty for a stored one.
    pub fn sources(&self) -> Vec<DislElement> {
        self.0.state.borrow().sources.clone()
    }

    /// Whether the id was read from storage; false for an id the reader made up or the runtime computed.
    pub fn id_is_stored(&self) -> bool {
        self.0.state.borrow().id_is_stored
    }

    pub(crate) fn set_id_is_stored(&self, stored: bool) {
        self.0.state.borrow_mut().id_is_stored = stored;
    }

    /// A relation's source as stored.
    pub fn source_id(&self) -> Option<String> {
        self.0.state.borrow().source_id.clone()
    }

    /// A relation's target as stored.
    pub fn target_id(&self) -> Option<String> {
        self.0.state.borrow().target_id.clone()
    }

    /// What the reader read beside the metamodel's attributes, as it read it; CEL does not see these.
    pub fn host_attributes(&self) -> Map {
        self.0.state.borrow().host_attributes.clone()
    }

    /// The 1-based line the reader found the element on, when it was read.
    pub fn line(&self) -> Option<i32> {
        self.0.state.borrow().line
    }

    /// The file the reader found the element in, relative to the diagram's subject (section 8.6); none for an
    /// element of the primary file, whose file is the diagram's.
    pub fn file(&self) -> Option<String> {
        self.0.state.borrow().file.clone()
    }

    /// Where the element is in the files (section 8.6): its own file, else the diagram's, with its line.
    pub(crate) fn location(&self) -> (Option<String>, Option<i32>) {
        let (file, line) = {
            let state = self.0.state.borrow();
            (state.file.clone(), state.line)
        };
        (file.or_else(|| self.try_diagram().and_then(|d| d.file())), line)
    }

    /// What `self.view` reads (section 12.2): the viewer's state of this element, which a host that holds viewer
    /// state binds before deriving a menu or a label; none, and no `view` member, for a headless model.
    pub fn view(&self) -> Option<Map> {
        self.0.state.borrow().view.clone()
    }

    pub fn set_view(&self, view: Option<Map>) {
        self.0.state.borrow_mut().view = view;
    }

    /// Whether the element's type is `type_name` or one of its subtypes (section 2.7).
    pub fn is_a(&self, type_name: &str) -> bool {
        self.0.state.borrow().element_type.is_a(type_name)
    }

    /// The value CEL reads for `name`: a derived attribute's expression (section 4.3), else stored, else its
    /// default, else its type's zero value; a reference resolved to its element. Null for a name the type does not
    /// declare.
    pub fn value_of(&self, name: &str) -> Value {
        let (attribute, stored) = {
            let state = self.0.state.borrow();
            let Some(attribute) = state.element_type.attributes.get(name) else {
                return Value::Null;
            };
            (attribute.clone(), state.attributes.get(name).cloned())
        };
        let Some(diagram) = self.try_diagram() else {
            return stored.unwrap_or(Value::Null);
        };
        let specification = diagram.specification();
        let metamodel = specification.metamodel();
        if attribute.is_derived() {
            return self.derived(&attribute, &diagram);
        }
        let Some(value) = stored else {
            return values::unset(&attribute, metamodel);
        };
        if metamodel.type_of(&attribute.type_name).is_none() {
            return value;
        }
        match &value {
            Value::String(id) => diagram.element_by_id(id).map(|e| e.to_value()).unwrap_or(Value::Null),
            Value::List(ids) => Value::list(
                ids.iter()
                    .map(|id| match id {
                        Value::String(text) => diagram.element_by_id(text).map(|e| e.to_value()).unwrap_or(Value::Null),
                        _ => Value::Null,
                    })
                    .collect(),
            ),
            _ => value,
        }
    }

    /// A derived attribute's value (section 4.3): its expression, compiled by the loader in the `element` context,
    /// with `self` this element and `env` a headless one. An expression that fails, or that reads its own value,
    /// gives the attribute's unset value, as a visual property falls back to its default (section 2.5).
    fn derived(&self, attribute: &DislAttribute, diagram: &DislDiagram) -> Value {
        let specification = diagram.specification();
        let metamodel = specification.metamodel();
        let declared_in = if metamodel.types.contains_key(&attribute.declared_by) {
            "/metamodel/types"
        } else {
            "/metamodel/relations"
        };
        let pointer = json::pointer(
            &json::pointer(
                &json::pointer(&json::pointer(declared_in, &attribute.declared_by), "attributes"),
                &attribute.name,
            ),
            "derived",
        );
        let Some(expression) = specification.expression_at(&pointer) else {
            return values::unset(attribute, metamodel);
        };
        let key = (self.key(), attribute.name.clone());
        if !COMPUTING.with(|c| c.borrow_mut().insert(key.clone())) {
            return values::unset(attribute, metamodel);
        }
        let variables = vars([
            ("self", self.to_value()),
            ("diagram", diagram.to_value()),
            ("env", Value::map(DislEnv::default().to_cel())),
        ]);
        let result = expression.program.evaluate(&variables);
        COMPUTING.with(|c| c.borrow_mut().remove(&key));
        result.unwrap_or_else(|_| values::unset(attribute, metamodel))
    }

    /// Every node beneath this one, depth first in model order.
    pub fn descendants(&self) -> Vec<DislElement> {
        let mut all = Vec::new();
        for child in self.children() {
            all.push(child.clone());
            all.extend(child.descendants());
        }
        all
    }

    /// The parent first, the top-level ancestor last.
    pub fn ancestors(&self) -> Vec<DislElement> {
        let mut all = Vec::new();
        let mut current = self.parent();
        while let Some(ancestor) = current {
            current = ancestor.parent();
            all.push(ancestor);
        }
        all
    }

    /// The relations that end at this node, in model order.
    pub fn incoming(&self) -> Vec<DislElement> {
        self.try_diagram().map(|d| d.incoming_of(self)).unwrap_or_default()
    }

    /// The relations that start at this node, in model order.
    pub fn outgoing(&self) -> Vec<DislElement> {
        self.try_diagram().map(|d| d.outgoing_of(self)).unwrap_or_default()
    }

    /// `n.successors(t)` (section 12.4): the nodes the relations of type `t` (its subtypes included) leaving this
    /// node lead to, each once, in the order of those relations.
    pub fn successors(&self, relation_type: &str) -> Vec<DislElement> {
        let mut seen = HashSet::new();
        self.outgoing()
            .into_iter()
            .filter(|r| r.is_a(relation_type))
            .filter_map(|r| r.target())
            .filter(|n| seen.insert(n.key()))
            .collect()
    }

    /// `n.predecessors(t)` (section 12.4): the nodes the relations of type `t` arriving at this node come from, each
    /// once, in the order of those relations.
    pub fn predecessors(&self, relation_type: &str) -> Vec<DislElement> {
        let mut seen = HashSet::new();
        self.incoming()
            .into_iter()
            .filter(|r| r.is_a(relation_type))
            .filter_map(|r| r.source())
            .filter(|n| seen.insert(n.key()))
            .collect()
    }

    /// `n.reachable(t)` (section 12.4): the transitive successors along relations of type `t`, breadth first in the
    /// order the relations are read, each once; this node itself only when it lies on a cycle of them.
    pub fn reachable(&self, relation_type: &str) -> Vec<DislElement> {
        let mut seen = HashSet::new();
        let mut found = Vec::new();
        let mut pending = std::collections::VecDeque::from([self.clone()]);
        while let Some(node) = pending.pop_front() {
            for next in node.successors(relation_type) {
                if seen.insert(next.key()) {
                    found.push(next.clone());
                    pending.push_back(next);
                }
            }
        }
        found
    }

    /// `n.inCycle(t)` (section 12.4): whether this node lies on a cycle of relations of type `t`, that is, whether
    /// it can reach itself along them.
    pub fn in_cycle(&self, relation_type: &str) -> bool {
        self.reachable(relation_type).iter().any(|n| n == self)
    }

    /// The value of the type's label attribute (section 4.6), as text; empty when it has none.
    fn label(&self) -> String {
        let label = self.0.state.borrow().element_type.label_attribute.clone();
        match label.map(|name| self.value_of(&name)) {
            Some(Value::Null) | None => String::new(),
            Some(Value::String(s)) => s,
            Some(other) => other.to_string(),
        }
    }

    /// Makes this node a `element_type` in place (a `retype` action, section 9.4): the attributes both types declare
    /// are kept, the others forgotten, and `mapped` stored over them.
    pub(crate) fn retype(&self, element_type: Rc<DislType>, mapped: &Map) {
        let mut state = self.0.state.borrow_mut();
        let dropped: Vec<String> = state
            .attributes
            .keys()
            .filter(|n| !element_type.attributes.contains_key(*n))
            .cloned()
            .collect();
        for name in dropped {
            state.attributes.remove(&name);
        }
        state.element_type = element_type;
        for (name, value) in mapped.iter() {
            if value.is_null() {
                state.attributes.remove(name);
            } else {
                state.attributes.insert(name.clone(), value.clone());
            }
        }
    }

    pub(crate) fn add_child(&self, child: &DislElement) {
        self.0.state.borrow_mut().children.push(child.clone());
    }

    pub(crate) fn remove_child(&self, child: &DislElement) {
        let mut state = self.0.state.borrow_mut();
        if let Some(index) = state.children.iter().position(|c| c == child) {
            state.children.remove(index);
        }
    }

    pub(crate) fn insert_child(&self, index: usize, child: &DislElement) {
        self.0.state.borrow_mut().children.insert(index, child.clone());
    }

    pub(crate) fn set_parent(&self, parent: Option<&DislElement>, slot: Option<String>) {
        let mut state = self.0.state.borrow_mut();
        state.parent = parent.map(|p| Rc::downgrade(&p.0));
        state.slot = if parent.is_some() { slot } else { None };
    }

    /// Stores `value` for `name`, or forgets the stored value for null: an action's effect on the working state
    /// (section 9.4).
    pub fn store(&self, name: &str, value: Value) -> Result<(), String> {
        let mut state = self.0.state.borrow_mut();
        if !state.element_type.attributes.contains_key(name) {
            return Err(format!("'{name}' is not an attribute of {}.", state.element_type.name));
        }
        if value.is_null() {
            state.attributes.remove(name);
        } else {
            state.attributes.insert(name, value);
        }
        Ok(())
    }

    /// The element as it is now, apart from the diagram: what a hook reads as `old` (section 9.2). Its attributes
    /// are a copy, its relations are not followed, and it is equal only to itself.
    pub fn snapshot(&self) -> DislElement {
        let mut state = self.0.state.borrow().clone();
        state.children = Vec::new();
        state.view = None;
        DislElement(Rc::new_cyclic(|this| ElementData {
            this: this.clone(),
            diagram: self.0.diagram.clone(),
            state: RefCell::new(state),
        }))
    }

    /// `e.positionIn(l)`: the zero-based position of this element in `list`, by identity, or −1 (section 12.4).
    fn position_in(&self, list: &Value) -> Result<Value, String> {
        let items = list.as_list().ok_or("positionIn() takes a list.")?;
        Ok(Value::Int(
            items
                .iter()
                .position(|item| DislElement::from_value(item).is_some_and(|e| e == *self))
                .map(|p| p as i64)
                .unwrap_or(-1),
        ))
    }

    /// `e.location()` (section 12.2): the SourceLocation the reader recorded, a map with `file` and, when known,
    /// `line`; `optional.none()` when no file is known, since a SourceLocation always names one (section 8.6).
    fn location_map(&self) -> Value {
        let (file, line) = self.location();
        let Some(file) = file else {
            return Value::none();
        };
        let mut location = Map::default();
        location.insert("file", Value::String(file));
        if let Some(line) = line {
            location.insert("line", Value::Int(i64::from(line)));
        }
        Value::some(Value::map(location))
    }

    fn other(&self, end: &Value) -> Result<Value, String> {
        let end = DislElement::from_value(end);
        let (source, target) = (self.source(), self.target());
        if end.is_some() && end == source {
            Ok(target.map(|t| t.to_value()).unwrap_or(Value::Null))
        } else if end.is_some() && end == target {
            Ok(source.map(|s| s.to_value()).unwrap_or(Value::Null))
        } else {
            Err(format!("other() was given an element that is not an end of {self}."))
        }
    }

    pub(crate) fn member(&self, name: &str) -> Option<Value> {
        // An attribute named like a member shadows it for its type (section 12.2); the reserved names (section 2.2)
        // keep the rest apart.
        let (is_relation, has_attribute) = {
            let state = self.0.state.borrow();
            (
                state.element_type.is_relation,
                state.element_type.attributes.contains_key(name),
            )
        };
        if has_attribute {
            return Some(self.value_of(name));
        }
        let element_or_null = |e: Option<DislElement>| e.map(|e| e.to_value()).unwrap_or(Value::Null);
        Some(match name {
            "id" => Value::String(self.id()),
            "type" => Value::String(self.type_name()),
            "kind" => Value::from(self.kind()),
            "derived" => Value::Bool(self.is_derived()),
            "sources" => list(self.sources()),
            "parent" if !is_relation => element_or_null(self.parent()),
            "owner" => match self.parent() {
                Some(parent) => parent.to_value(),
                None => self.try_diagram().map(|d| d.to_value()).unwrap_or(Value::Null),
            },
            "slot" if !is_relation => Value::String(self.slot().unwrap_or_default()),
            "children" if !is_relation => list(self.children()),
            "incoming" if !is_relation => list(self.incoming()),
            "outgoing" if !is_relation => list(self.outgoing()),
            "ports" if !is_relation => Value::list(Vec::new()),
            "source" if is_relation => element_or_null(self.source()),
            "target" if is_relation => element_or_null(self.target()),
            "view" => Value::map(self.view()?),
            _ => return None,
        })
    }

    pub(crate) fn has_member(&self, name: &str) -> bool {
        let attribute = self.0.state.borrow().element_type.attributes.get(name).cloned();
        match attribute {
            Some(attribute) => self.0.state.borrow().attributes.contains_key(name) || attribute.is_derived(),
            None => self.member(name).is_some_and(|v| !v.is_null()),
        }
    }

    pub(crate) fn invoke(&self, name: &str, arguments: &[Value]) -> Option<Result<Value, String>> {
        let is_relation = self.is_relation();
        let of_type = |filtered: Vec<DislElement>| -> Result<Value, String> {
            let type_name = type_text(&arguments[0])?;
            Ok(list(filtered.into_iter().filter(|e| e.is_a(type_name))))
        };
        Some(match (name, arguments.len()) {
            ("isA", 1) => type_text(&arguments[0]).map(|t| Value::Bool(self.is_a(t))),
            ("label", 0) => Ok(Value::String(self.label())),
            ("positionIn", 1) => self.position_in(&arguments[0]),
            ("descendants", 0) if !is_relation => Ok(list(self.descendants())),
            ("ancestors", 0) if !is_relation => Ok(list(self.ancestors())),
            ("childrenOfType", 1) if !is_relation => of_type(self.children()),
            ("incomingOf", 1) if !is_relation => of_type(self.incoming()),
            ("outgoingOf", 1) if !is_relation => of_type(self.outgoing()),
            ("successors", 1) if !is_relation => type_text(&arguments[0]).map(|t| list(self.successors(t))),
            ("predecessors", 1) if !is_relation => type_text(&arguments[0]).map(|t| list(self.predecessors(t))),
            ("reachable", 1) if !is_relation => type_text(&arguments[0]).map(|t| list(self.reachable(t))),
            ("inCycle", 1) if !is_relation => type_text(&arguments[0]).map(|t| Value::Bool(self.in_cycle(t))),
            ("other", 1) if is_relation => self.other(&arguments[0]),
            ("location", 0) => Ok(self.location_map()),
            _ => return None,
        })
    }
}

impl ElementData {
    fn handle(&self) -> DislElement {
        DislElement(
            self.this
                .upgrade()
                .expect("an element answers CEL through a live handle"),
        )
    }
}

impl Object for ElementData {
    fn into_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn member(&self, name: &str) -> Option<Value> {
        with_handle(self, |e| e.member(name))
    }

    fn has_member(&self, name: &str) -> bool {
        with_handle(self, |e| e.has_member(name))
    }

    fn invoke(&self, name: &str, arguments: &[Value]) -> Option<Result<Value, String>> {
        with_handle(self, |e| e.invoke(name, arguments))
    }

    /// Identity, as everywhere; the element a finding names by its written id is this one too.
    fn equals(&self, other: &dyn Object) -> bool {
        if let Some(element) = other.as_any().downcast_ref::<ElementData>() {
            return std::ptr::eq(self, element);
        }
        if let Some(written) = other.as_any().downcast_ref::<WrittenElement>() {
            return std::ptr::eq(self, Rc::as_ptr(&written.element().0));
        }
        false
    }

    fn text(&self) -> String {
        format!("{self:?}")
    }
}

/// Runs `f` with the handle of the element `data` is; CEL hands an object by reference, the model works on handles.
fn with_handle<T>(data: &ElementData, f: impl FnOnce(&DislElement) -> T) -> T {
    f(&data.handle())
}
