//! What a viewpoint shows and the views of a model (DISL sections 3.5 and 12.2; etalii.adp spec 067, FR-013): an element
//! belongs to a view when its type passes the viewpoint's `include` and `exclude` and its `members` holds for it, and a
//! relation only when both its ends do. `diagram.views` lists every view of the body, whichever one this reading shows:
//! each view's statements are data for it, read as the reading that shows it reads them, and its members are what the
//! viewpoint selects there, as the elements of this diagram. A view also exposes the scope it is written with and where
//! it stands (spec 067, FR-016), so that a rule about the model can say something about the view itself.

use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use adp_cel::{Map, Object, Value};
use adp_fbl::FblBodyView;
use serde_json::Value as Json;

use super::builder;
use super::diagram::DislDiagram;
use super::element::{DislElement, list};
use crate::derivation::env::DislEnv;
use crate::expressions::contexts;

/// Whether `element`'s type is among `types`.
fn among(element: &DislElement, types: &Json) -> Option<bool> {
    let types = types.as_array()?;
    Some(types.iter().filter_map(Json::as_str).any(|t| element.is_a(t)))
}

impl DislDiagram {
    /// The elements viewpoint `viewpoint` shows, in model order, nodes first (DISL section 3.5): every element for a
    /// viewpoint the specification does not declare, which shows everything.
    pub fn shown_in(&self, viewpoint: &str) -> Vec<DislElement> {
        let specification = self.specification();
        let declared = specification
            .root()
            .get("viewpoints")
            .and_then(|all| all.get(viewpoint))
            .cloned()
            .unwrap_or(Json::Null);
        let source = match declared.get("members") {
            Some(Json::Object(members)) => members.get("cel").and_then(Json::as_str),
            Some(Json::String(text)) => Some(text.as_str()),
            _ => None,
        };
        let program = source.map(|s| specification.environment(contexts::ELEMENT, &[]).compile(s));
        let env = Value::map(
            DislEnv {
                viewpoint: viewpoint.to_owned(),
                ..DislEnv::default()
            }
            .to_cel(),
        );
        let passes = |element: &DislElement| {
            among(element, declared.get("include").unwrap_or(&Json::Null)).unwrap_or(true)
                && !among(element, declared.get("exclude").unwrap_or(&Json::Null)).unwrap_or(false)
                && match &program {
                    None => true,
                    Some(Err(_)) => false,
                    Some(Ok(program)) => {
                        let variables = HashMap::from([
                            ("self".to_owned(), element.to_value()),
                            ("diagram".to_owned(), self.to_value()),
                            ("env".to_owned(), env.clone()),
                        ]);
                        matches!(program.evaluate(&variables), Ok(Value::Bool(true)))
                    }
                }
        };
        let nodes: Vec<DislElement> = self.nodes().into_iter().filter(&passes).collect();
        let shown: HashSet<String> = nodes.iter().map(DislElement::id).collect();
        let on_view = |end: Option<DislElement>| end.is_some_and(|end| shown.contains(&end.id()));
        let relations = self
            .relations()
            .into_iter()
            .filter(|relation| on_view(relation.source()) && on_view(relation.target()))
            .filter(&passes);
        nodes.iter().cloned().chain(relations).collect()
    }

    /// `diagram.views` (DISL section 12.2): one `View` for each of `views`, in order.
    pub(crate) fn views_of(&self, views: &[FblBodyView]) -> Vec<Value> {
        views.iter().map(|view| self.view_value(view)).collect()
    }

    fn view_value(&self, view: &FblBodyView) -> Value {
        let specification = self.specification();
        // A view's own reading, its statements in place of the ones this reading shows.
        let reading = view
            .reading
            .as_ref()
            .map(|reading| builder::from(reading, &specification).diagram);
        let shows = reading.as_ref().unwrap_or(self);
        // The viewpoint of a view is what its statements say, where the description reads one, and else the one a view
        // of this reading opens in.
        let viewpoint = match shows.stored_attribute("viewpoint") {
            Some(Value::String(viewpoint)) if !viewpoint.is_empty() => viewpoint,
            _ => specification.default_viewpoint(),
        };
        let shown = shows.shown_in(&specification.default_viewpoint());
        // The elements of this diagram: a rule compares its elements with them.
        let members: Vec<DislElement> = shown.iter().filter_map(|e| self.element_by_id(&e.id())).collect();
        // The diagram attribute `scope` of the view's reading, as written: a reference that names nothing stays as read.
        let scope = match shows.stored_attribute("scope") {
            Some(Value::String(scope)) => scope,
            _ => String::new(),
        };
        // A SourceLocation always names a file (section 8.6).
        let location = self.file().or_else(|| self.read_from()).map(|file| {
            let mut location = Map::default();
            location.insert("file", Value::String(file));
            location.insert("line", Value::Int(view.line as i64));
            Value::map(location)
        });
        Value::Object(Rc::new(View {
            id: view.name.clone(),
            viewpoint,
            scope,
            members: list(members),
            location,
        }))
    }
}

/// A `View` of section 12.2, as a rule reads it.
#[derive(Debug)]
struct View {
    id: String,
    viewpoint: String,
    scope: String,
    members: Value,
    location: Option<Value>,
}

impl Object for View {
    fn into_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn member(&self, name: &str) -> Option<Value> {
        match name {
            "id" | "name" => Some(Value::String(self.id.clone())),
            "viewpoint" => Some(Value::String(self.viewpoint.clone())),
            "scope" => Some(Value::String(self.scope.clone())),
            "members" => Some(self.members.clone()),
            _ => None,
        }
    }

    fn has_member(&self, name: &str) -> bool {
        matches!(name, "id" | "name" | "viewpoint" | "scope" | "members")
    }

    fn invoke(&self, name: &str, arguments: &[Value]) -> Option<Result<Value, String>> {
        match (name, arguments.len()) {
            ("location", 0) => Some(Ok(match &self.location {
                Some(location) => Value::some(location.clone()),
                None => Value::none(),
            })),
            _ => None,
        }
    }

    fn text(&self) -> String {
        self.id.clone()
    }
}

/// The views of a reading, as `diagram.views` lists them, kept once computed.
pub(crate) type Views = Rc<Vec<Value>>;
