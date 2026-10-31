//! A diagram as DISL's CEL sees it (DISL section 12.2; the standalone host's `DislDiagram`): the root element, its
//! attributes as fields, and its nodes and relations in model order - the stored ones in persistence order, then the
//! derived ones in their computation order (section 12.5). Built element by element, parents before their children,
//! by the model builder or a test; the element lists are what the expressions iterate, so their order is the order
//! every derived id, finding and position depends on. The working state an action list changes (section 9.4) is a
//! diagram too.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::{Rc, Weak};

use adp_cel::{Map, Object, Value};
use adp_fbl::FblBodyView;

use super::budgets::{self, DislBudgetState};
use super::cycles;
use super::element::{DislElement, NewElement, list, type_text};
use super::knots;
use super::values;
use super::views::Views;
use crate::loading::metamodel::DislType;
use crate::loading::specification::DislSpecification;

/// A diagram of a specification; cloning it gives another handle to the same diagram.
#[derive(Clone)]
pub struct DislDiagram(pub(crate) Rc<DiagramData>);

type Ends = HashMap<usize, (Vec<DislElement>, Vec<DislElement>)>;
type Cycles = (Vec<Vec<DislElement>>, bool);

pub(crate) struct DiagramData {
    this: Weak<DiagramData>,
    specification: DislSpecification,
    attributes: RefCell<Map>,
    nodes: RefCell<Vec<DislElement>>,
    relations: RefCell<Vec<DislElement>>,
    ends: RefCell<Option<Ends>>,
    cycles: RefCell<HashMap<(String, i64), Cycles>>,
    knots: RefCell<HashMap<String, Vec<Vec<DislElement>>>>,
    /// The budgets' states once computed, until the diagram or one of its elements changes.
    budgets: RefCell<Option<Rc<Vec<DislBudgetState>>>>,
    computing_budgets: Cell<bool>,
    file: RefCell<Option<String>>,
    read_from: RefCell<Option<String>>,
    /// The views the reading found in the body, and `diagram.views` once computed from them.
    body_views: RefCell<Vec<FblBodyView>>,
    views: RefCell<Option<Views>>,
}

impl fmt::Debug for DiagramData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("diagram")
    }
}

impl fmt::Debug for DislDiagram {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("diagram")
    }
}

impl PartialEq for DislDiagram {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl DiagramData {
    fn handle(&self) -> DislDiagram {
        DislDiagram(
            self.this
                .upgrade()
                .expect("a diagram answers CEL through a live handle"),
        )
    }

    /// The number of nodes and relations, which the cycle search's cost is proportional to.
    pub(crate) fn size(&self) -> usize {
        self.nodes.borrow().len() + self.relations.borrow().len()
    }

    pub(crate) fn cycles(&self, relation_type: &str, max: i64) -> Cycles {
        self.handle().cycles(relation_type, max)
    }

    pub(crate) fn knots(&self, relation_type: &str) -> Vec<Vec<DislElement>> {
        self.handle().knots(relation_type)
    }

    pub(crate) fn budgets(&self) -> Result<Rc<Vec<DislBudgetState>>, String> {
        self.handle().try_budgets()
    }

    /// An element changed: the budgets are computed again when next read.
    pub(crate) fn forget_budgets(&self) {
        *self.budgets.borrow_mut() = None;
        *self.views.borrow_mut() = None;
    }
}

impl DislDiagram {
    /// A diagram of `specification` without diagram attributes.
    pub fn new(specification: &DislSpecification) -> Self {
        DislDiagram(Rc::new_cyclic(|this| DiagramData {
            this: this.clone(),
            specification: specification.clone(),
            attributes: RefCell::new(Map::default()),
            nodes: RefCell::new(Vec::new()),
            relations: RefCell::new(Vec::new()),
            ends: RefCell::new(None),
            cycles: RefCell::new(HashMap::new()),
            knots: RefCell::new(HashMap::new()),
            budgets: RefCell::new(None),
            computing_budgets: Cell::new(false),
            file: RefCell::new(None),
            read_from: RefCell::new(None),
            body_views: RefCell::new(Vec::new()),
            views: RefCell::new(None),
        }))
    }

    /// A diagram of `specification` with the diagram attributes `attributes`, as CEL values.
    pub fn with_attributes(specification: &DislSpecification, attributes: Map) -> Result<Self, String> {
        for (name, _) in attributes.iter() {
            if !specification.metamodel().diagram_attributes.contains_key(name) {
                return Err(format!("'{name}' is not an attribute of the diagram."));
            }
        }
        let diagram = DislDiagram::new(specification);
        *diagram.0.attributes.borrow_mut() = attributes;
        Ok(diagram)
    }

    pub fn specification(&self) -> DislSpecification {
        self.0.specification.clone()
    }

    /// What `diagram.file` reads (section 8.2): the file a finding attributes the diagram to, relative to its
    /// subject, which a host binds when a message names it; none, and no `file` member, when the host gives none.
    pub fn file(&self) -> Option<String> {
        self.0.file.borrow().clone()
    }

    pub fn set_file(&self, file: Option<&str>) {
        *self.0.file.borrow_mut() = file.map(str::to_owned);
    }

    /// The file the reader read the elements from, relative to the subject: where an element's location is when nothing
    /// names another (section 8.6), known from the first read, before a host binds `diagram.file`.
    pub(crate) fn read_from(&self) -> Option<String> {
        self.0.read_from.borrow().clone()
    }

    /// The value a diagram attribute holds, none when it has none stored.
    pub(crate) fn stored_attribute(&self, name: &str) -> Option<Value> {
        self.0.attributes.borrow().get(name).cloned()
    }

    /// Keeps the views the reading found in the body (DISL section 12.2, `views`).
    pub(crate) fn set_body_views(&self, views: Vec<FblBodyView>) {
        *self.0.body_views.borrow_mut() = views;
        *self.0.views.borrow_mut() = None;
    }

    /// `diagram.views`: computed when first read, until the diagram or one of its elements changes.
    fn views(&self) -> Views {
        if let Some(views) = self.0.views.borrow().as_ref() {
            return Rc::clone(views);
        }
        let body_views = self.0.body_views.borrow().clone();
        let views = Rc::new(self.views_of(&body_views));
        *self.0.views.borrow_mut() = Some(Rc::clone(&views));
        views
    }

    pub(crate) fn set_read_from(&self, file: Option<&str>) {
        *self.0.read_from.borrow_mut() = file.map(str::to_owned);
    }

    /// The diagram attributes that have a stored value.
    pub fn attributes(&self) -> Map {
        self.0.attributes.borrow().clone()
    }

    /// The model's nodes, in model order.
    pub fn nodes(&self) -> Vec<DislElement> {
        self.0.nodes.borrow().clone()
    }

    /// The model's relations, in model order.
    pub fn relations(&self) -> Vec<DislElement> {
        self.0.relations.borrow().clone()
    }

    /// The nodes, then the relations.
    pub fn elements(&self) -> Vec<DislElement> {
        let mut all = self.nodes();
        all.extend(self.relations());
        all
    }

    /// The diagram as a CEL value.
    pub fn to_value(&self) -> Value {
        Value::Object(Rc::clone(&self.0) as Rc<dyn Object>)
    }

    /// The diagram a CEL value holds, if it holds one.
    pub fn from_value(value: &Value) -> Option<DislDiagram> {
        value.downcast::<DiagramData>().map(DislDiagram)
    }

    fn type_for(&self, name: &str, relation: bool) -> Result<Rc<DislType>, String> {
        let t = self
            .0
            .specification
            .metamodel()
            .type_of(name)
            .cloned()
            .ok_or_else(|| format!("'{name}' is not a type of this specification."))?;
        if t.is_relation != relation {
            return Err(format!(
                "'{name}' is {}.",
                if t.is_relation {
                    "a relation type"
                } else {
                    "a node type"
                }
            ));
        }
        if t.is_abstract() {
            return Err(format!("'{name}' is abstract."));
        }
        Ok(t)
    }

    /// Adds a top-level node of `type_name`.
    pub fn add_node(&self, type_name: &str, id: &str, attributes: Map) -> Result<DislElement, String> {
        let t = self.type_for(type_name, false)?;
        let mut new = NewElement::new(type_name, id);
        new.attributes = attributes;
        let element = DislElement::make(self, &new, t)?;
        self.add_made(element)
    }

    /// Adds a node of `type_name` beneath `parent`, in `slot`.
    pub fn add_child(
        &self,
        type_name: &str,
        id: &str,
        attributes: Map,
        parent: &DislElement,
        slot: Option<&str>,
    ) -> Result<DislElement, String> {
        let t = self.type_for(type_name, false)?;
        let mut new = NewElement::new(type_name, id);
        new.attributes = attributes;
        new.parent = Some(parent.clone());
        new.slot = slot.map(str::to_owned);
        let element = DislElement::make(self, &new, t)?;
        self.add_made(element)
    }

    /// Adds a relation of `type_name` from `source` to `target`; either may be missing, as a model being read can
    /// have it.
    pub fn add_relation(
        &self,
        type_name: &str,
        id: &str,
        source: Option<&DislElement>,
        target: Option<&DislElement>,
        attributes: Map,
    ) -> Result<DislElement, String> {
        let t = self.type_for(type_name, true)?;
        let mut new = NewElement::new(type_name, id);
        new.attributes = attributes;
        new.source = source.cloned();
        new.target = target.cloned();
        new.source_id = source.map(DislElement::id);
        new.target_id = target.map(DislElement::id);
        let element = DislElement::make(self, &new, t)?;
        self.add_made(element)
    }

    /// Adds an element made for this diagram: a node beneath its parent, a relation at its ends.
    pub fn add(&self, new: NewElement) -> Result<DislElement, String> {
        let t = self
            .0
            .specification
            .metamodel()
            .type_of(&new.type_name)
            .cloned()
            .ok_or_else(|| format!("'{}' is not a type of this specification.", new.type_name))?;
        let element = DislElement::make(self, &new, t)?;
        self.add_made(element)
    }

    fn owns(&self, element: &DislElement) -> bool {
        element.0.diagram.upgrade().is_some_and(|d| Rc::ptr_eq(&d, &self.0))
    }

    pub(crate) fn add_made(&self, element: DislElement) -> Result<DislElement, String> {
        if element.is_relation() {
            let foreign = |end: Option<DislElement>| end.is_some_and(|e| !self.owns(&e));
            if foreign(element.source()) || foreign(element.target()) {
                return Err("A relation's ends belong to its own diagram.".into());
            }
            self.0.relations.borrow_mut().push(element.clone());
            self.changed();
            return Ok(element);
        }
        if let Some(parent) = element.parent() {
            if !self.owns(&parent) {
                return Err("The parent belongs to another diagram.".into());
            }
            parent.add_child(&element);
        }
        self.0.nodes.borrow_mut().push(element.clone());
        self.changed();
        Ok(element)
    }

    /// Takes `element` out of the diagram, with its relations' ends left as they are: an action's effect on the
    /// working state (section 9.4).
    pub(crate) fn remove(&self, element: &DislElement) {
        if element.is_relation() {
            self.0.relations.borrow_mut().retain(|r| r != element);
            self.changed();
            return;
        }
        if let Some(parent) = element.parent() {
            parent.remove_child(element);
        }
        let mut nodes = self.0.nodes.borrow_mut();
        if let Some(index) = nodes.iter().position(|n| n == element) {
            nodes.remove(index);
        }
        drop(nodes);
        self.changed();
    }

    /// Moves node `element`, with everything beneath it, under `parent` in `slot`, or to the top level: an action's
    /// effect on the working state (section 9.4). `index` is where among the new parent's children it goes, counted
    /// before the move with the node itself included; negative or past the end puts it last. The node list keeps
    /// document order: the moved subtree follows the sibling it now comes after.
    pub(crate) fn reparent(
        &self,
        element: &DislElement,
        parent: Option<&DislElement>,
        slot: Option<&str>,
        index: i64,
    ) -> Result<(), String> {
        if !self.owns(element) || element.is_relation() {
            return Err("Only a node of this diagram can move.".into());
        }
        if let Some(parent) = parent {
            if !self.owns(parent) || parent.is_relation() {
                return Err("A node moves beneath a node of its own diagram.".into());
            }
            if parent == element || parent.ancestors().contains(element) {
                return Err("A node cannot move beneath itself.".into());
            }
        }

        let siblings = self.siblings_under(parent);
        let mut at = if index < 0 || index as usize > siblings.len() {
            siblings.len()
        } else {
            index as usize
        };
        if element.parent().as_ref() == parent
            && let Some(current) = siblings.iter().position(|s| s == element)
            && current < at
        {
            at -= 1;
        }

        let mut subtree: HashSet<usize> = element.descendants().iter().map(DislElement::key).collect();
        subtree.insert(element.key());
        let block: Vec<DislElement> = {
            let mut nodes = self.0.nodes.borrow_mut();
            let block = nodes.iter().filter(|n| subtree.contains(&n.key())).cloned().collect();
            nodes.retain(|n| !subtree.contains(&n.key()));
            block
        };
        if let Some(old) = element.parent() {
            old.remove_child(element);
        }
        element.set_parent(parent, slot.map(str::to_owned));
        if let Some(parent) = parent {
            parent.insert_child(at, element);
        }

        let after = if at == 0 {
            None
        } else {
            self.siblings_under(parent)
                .into_iter()
                .filter(|s| s != element)
                .nth(at - 1)
        };
        let position = {
            let nodes = self.0.nodes.borrow();
            let index_of = |e: &DislElement| nodes.iter().position(|n| n == e);
            if let Some(after) = after {
                let mut members = after.descendants();
                members.insert(0, after);
                members.iter().filter_map(index_of).max().map(|m| m + 1).unwrap_or(0)
            } else if let Some(parent) = parent {
                index_of(parent).map(|p| p + 1).unwrap_or(0)
            } else {
                nodes.iter().position(|n| n.parent().is_none()).unwrap_or(nodes.len())
            }
        };
        {
            let mut nodes = self.0.nodes.borrow_mut();
            let position = position.min(nodes.len());
            for (offset, node) in block.into_iter().enumerate() {
                nodes.insert(position + offset, node);
            }
        }
        self.changed();
        Ok(())
    }

    /// What is computed from the elements and their order is computed again.
    fn changed(&self) {
        *self.0.ends.borrow_mut() = None;
        self.0.cycles.borrow_mut().clear();
        self.0.knots.borrow_mut().clear();
        self.0.forget_budgets();
    }

    /// The children of `parent`, or the top-level nodes, in model order.
    pub(crate) fn siblings_under(&self, parent: Option<&DislElement>) -> Vec<DislElement> {
        match parent {
            Some(parent) => parent.children(),
            None => self.nodes().into_iter().filter(|n| n.parent().is_none()).collect(),
        }
    }

    /// Sets diagram attribute `name`, as a reader does.
    pub(crate) fn set_attribute(&self, name: &str, value: Value) {
        self.0.attributes.borrow_mut().insert(name, value);
        self.0.forget_budgets();
    }

    /// The value CEL reads for diagram attribute `name`: stored, else its default, else its type's zero value
    /// (section 4.3); null for a name the diagram does not declare.
    pub fn value_of(&self, name: &str) -> Value {
        let metamodel = self.0.specification.metamodel();
        let Some(attribute) = metamodel.diagram_attributes.get(name) else {
            return Value::Null;
        };
        self.0
            .attributes
            .borrow()
            .get(name)
            .cloned()
            .unwrap_or_else(|| values::unset(attribute, metamodel))
    }

    /// The first element with `id` in model order.
    pub fn element_by_id(&self, id: &str) -> Option<DislElement> {
        let found = self.0.nodes.borrow().iter().find(|n| n.id() == id).cloned();
        found.or_else(|| self.0.relations.borrow().iter().find(|r| r.id() == id).cloned())
    }

    /// The nodes that are `type_name` or one of its subtypes, in model order.
    pub fn nodes_of_type(&self, type_name: &str) -> Vec<DislElement> {
        self.nodes().into_iter().filter(|n| n.is_a(type_name)).collect()
    }

    /// The relations that are `type_name` or one of its subtypes, in model order.
    pub fn relations_of_type(&self, type_name: &str) -> Vec<DislElement> {
        self.relations().into_iter().filter(|r| r.is_a(type_name)).collect()
    }

    /// The elementary cycles over the relations that are `relation_type` or one of its subtypes, at most `max` of
    /// them, and whether the bound cut them (section 12.4, `diagram.cycles` and `diagram.cyclesTruncated`): each
    /// cycle in loop order from its member that comes first in model order, the cycles ordered by that member and
    /// then lexicographically by member order, a self-loop a cycle of one. Memoised until the diagram changes.
    pub fn cycles(&self, relation_type: &str, max: i64) -> (Vec<Vec<DislElement>>, bool) {
        let key = (relation_type.to_owned(), max);
        if let Some(known) = self.0.cycles.borrow().get(&key) {
            return known.clone();
        }
        let relations = self.relations_of_type(relation_type);
        let found = cycles::find(&self.nodes(), &relations, max.max(0));
        self.0.cycles.borrow_mut().insert(key, found.clone());
        found
    }

    /// The strongly connected components over the relations that are `relation_type` or one of its subtypes that have
    /// more than one member or a self-loop (section 12.4, `diagram.knots`): members in model order, components ordered
    /// by their first member. Memoised until the diagram changes.
    pub fn knots(&self, relation_type: &str) -> Vec<Vec<DislElement>> {
        if let Some(known) = self.0.knots.borrow().get(relation_type) {
            return known.clone();
        }
        let found = knots::find(&self.nodes(), &self.relations_of_type(relation_type));
        self.0
            .knots
            .borrow_mut()
            .insert(relation_type.to_owned(), found.clone());
        found
    }

    /// The state of every budget (section 3.2.1), in declaration order: computed when first asked for, and again
    /// after the diagram or one of its elements changes. A budget whose measure, order or unit reads the budgets has
    /// none, so its state is empty.
    pub fn budgets(&self) -> Rc<Vec<DislBudgetState>> {
        self.try_budgets().unwrap_or_default()
    }

    fn try_budgets(&self) -> Result<Rc<Vec<DislBudgetState>>, String> {
        if let Some(known) = self.0.budgets.borrow().as_ref() {
            return Ok(Rc::clone(known));
        }
        if self.0.computing_budgets.replace(true) {
            return Err("A budget is read while the budgets are computed.".to_owned());
        }
        let states = Rc::new(budgets::compute(self));
        self.0.computing_budgets.set(false);
        *self.0.budgets.borrow_mut() = Some(Rc::clone(&states));
        Ok(states)
    }

    /// The state of the budget `id` (section 3.2.1), or none when the specification declares none by that id.
    pub fn budget(&self, id: &str) -> Option<DislBudgetState> {
        self.budgets().iter().find(|budget| budget.id == id).cloned()
    }

    /// Whether `element` is drawn under the budgets (section 3.2.1): no truncating budget cuts it, and for a
    /// relation, both its ends are drawn.
    pub fn is_drawn(&self, element: &DislElement) -> bool {
        if self.budgets().iter().any(|budget| budget.cuts(element)) {
            return false;
        }
        !element.is_relation()
            || element.source().is_none_or(|source| self.is_drawn(&source))
                && element.target().is_none_or(|target| self.is_drawn(&target))
    }

    fn ends(&self, node: &DislElement, incoming: bool) -> Vec<DislElement> {
        if self.0.ends.borrow().is_none() {
            let mut ends: Ends = HashMap::new();
            for relation in self.relations() {
                if let Some(source) = relation.source() {
                    ends.entry(source.key()).or_default().1.push(relation.clone());
                }
                if let Some(target) = relation.target() {
                    ends.entry(target.key()).or_default().0.push(relation.clone());
                }
            }
            *self.0.ends.borrow_mut() = Some(ends);
        }
        let ends = self.0.ends.borrow();
        ends.as_ref()
            .and_then(|e| e.get(&node.key()))
            .map(|(i, o)| if incoming { i.clone() } else { o.clone() })
            .unwrap_or_default()
    }

    pub(crate) fn incoming_of(&self, node: &DislElement) -> Vec<DislElement> {
        self.ends(node, true)
    }

    pub(crate) fn outgoing_of(&self, node: &DislElement) -> Vec<DislElement> {
        self.ends(node, false)
    }

    fn member(&self, name: &str) -> Option<Value> {
        Some(match name {
            "nodes" => list(self.nodes()),
            "relations" => list(self.relations()),
            "elements" => list(self.elements()),
            "views" if !self.0.specification.metamodel().diagram_attributes.contains_key(name) => {
                Value::list(self.views().as_ref().clone())
            }
            "id" => Value::from(""),
            "type" | "kind" => Value::from("diagram"),
            _ if self.0.specification.metamodel().diagram_attributes.contains_key(name) => {
                let value = self.value_of(name);
                let metamodel = self.0.specification.metamodel();
                // An attribute that refers to an element reads as the element, as an element's own does.
                let refers = metamodel
                    .diagram_attributes
                    .get(name)
                    .is_some_and(|attribute| metamodel.type_of(&attribute.type_name).is_some());
                match &value {
                    Value::String(id) if refers => self.element_by_id(id).map_or(Value::Null, |e| e.to_value()),
                    _ => value,
                }
            }
            "file" => Value::String(self.file()?),
            _ => return None,
        })
    }

    fn invoke(&self, name: &str, arguments: &[Value]) -> Option<Result<Value, String>> {
        Some(match (name, arguments.len()) {
            // View-only elements are a view's; a headless model has none, so the flag changes nothing here.
            ("nodesOfType", 1 | 2) => type_text(&arguments[0]).map(|t| list(self.nodes_of_type(t))),
            ("relationsOfType", 1) => type_text(&arguments[0]).map(|t| list(self.relations_of_type(t))),
            ("elementById", 1) => {
                type_text(&arguments[0]).map(|id| self.element_by_id(id).map(|e| e.to_value()).unwrap_or(Value::Null))
            }
            _ => return None,
        })
    }
}

impl Object for DiagramData {
    fn into_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn member(&self, name: &str) -> Option<Value> {
        self.handle().member(name)
    }

    fn has_member(&self, name: &str) -> bool {
        if self.specification.metamodel().diagram_attributes.contains_key(name) {
            self.attributes.borrow().contains_key(name)
        } else {
            self.handle().member(name).is_some()
        }
    }

    fn invoke(&self, name: &str, arguments: &[Value]) -> Option<Result<Value, String>> {
        self.handle().invoke(name, arguments)
    }

    fn text(&self) -> String {
        "diagram".into()
    }
}
