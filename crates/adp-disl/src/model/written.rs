//! An element as a finding about it names it (DISL section 11.5.4; the standalone host's `DislWrittenElement`):
//! every member the element's own, but `id` the id it is written with, which for a later holder of a written id
//! differs from its ephemeral model id. It compares equal to the element it shows, both ways round.

use std::any::Any;
use std::rc::Rc;

use adp_cel::{Object, Value};

use super::element::{DislElement, ElementData};

#[derive(Debug)]
pub(crate) struct WrittenElement {
    element: DislElement,
    written_id: String,
}

impl WrittenElement {
    pub(crate) fn new(element: DislElement, written_id: String) -> Self {
        WrittenElement { element, written_id }
    }

    pub(crate) fn element(&self) -> &DislElement {
        &self.element
    }
}

impl Object for WrittenElement {
    fn into_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn member(&self, name: &str) -> Option<Value> {
        if name == "id" && !self.element.element_type().attributes.contains_key("id") {
            return Some(Value::String(self.written_id.clone()));
        }
        self.element.member(name)
    }

    fn has_member(&self, name: &str) -> bool {
        self.element.has_member(name)
    }

    fn invoke(&self, name: &str, arguments: &[Value]) -> Option<Result<Value, String>> {
        self.element.invoke(name, arguments)
    }

    fn equals(&self, other: &dyn Object) -> bool {
        if let Some(element) = other.as_any().downcast_ref::<ElementData>() {
            return std::ptr::eq(Rc::as_ptr(&self.element.0), element);
        }
        other
            .as_any()
            .downcast_ref::<WrittenElement>()
            .is_some_and(|w| w.element == self.element)
    }

    fn text(&self) -> String {
        format!("{} {}", self.element.type_name(), self.written_id)
    }
}
