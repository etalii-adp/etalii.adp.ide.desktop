//! A diagram's description as a tool `adp-diagram` opens (etalii.adp spec 035, T126, research R16): the lowered DISL
//! specification, the lowered FBL binding, and the runtime's module, which gives the engines what a hand-written
//! module gives them for today's tools, from the description: the persistence plugins it declares, positions from
//! the layout capability it names, and each viewer's view state from its persistence settings. No code here knows a
//! tool type (FR-040).

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use adp_atd_check::{Description, Symbols};
use adp_atd_script::debug::Debugger;
use adp_cel::{Map, Value};
use adp_diagram::disl_definition::DislDiagramDefinition;
use adp_diagram::{DiagramDocument, DiagramModule, DiagramTool};
use adp_disl::{DislElement, DislPluginFunctions, DislSpecification};
use adp_fbl::plugins::PersistencePlugin;
use serde_json::{Value as Json, json};

use crate::capabilities::Capabilities;
use crate::lowering::{self, LoadedDiagram};

/// The persistence plugins of a host, by id.
pub type Plugins = Rc<dyn Fn(&str) -> Option<Rc<dyn PersistencePlugin>>>;

/// What the host gives a description's tool: its plugins' persistence and CEL functions, its capabilities, and the
/// debugger its script runs under, if any (research R21).
#[derive(Clone)]
pub struct RuntimeHost {
    pub plugins: Plugins,
    pub functions: DislPluginFunctions,
    pub capabilities: Capabilities,
    pub debugger: Option<Rc<Debugger>>,
}

impl Default for RuntimeHost {
    fn default() -> Self {
        RuntimeHost {
            plugins: Rc::new(|_| None),
            functions: DislPluginFunctions::none(),
            capabilities: Capabilities::desktop(),
            debugger: None,
        }
    }
}

/// A diagram's description, as the tool `adp-diagram` opens its documents with.
pub struct RuntimeTool {
    pub tool: Rc<DiagramTool>,
    pub loaded: LoadedDiagram,
}

/// The tool of a checked diagram description.
pub fn diagram_tool(description: &Description, symbols: &Symbols, host: &RuntimeHost) -> Result<RuntimeTool, String> {
    let loaded = lowering::load_diagram(description, symbols, &host.functions).map_err(|diagnostics| {
        let lines: Vec<String> = diagnostics.iter().map(ToString::to_string).collect();
        format!("The description does not lower to DISL: {}", lines.join("; "))
    })?;
    if let Some(debugger) = &host.debugger {
        debugger.attach_script(&loaded.script, ());
    }
    let members: Vec<&Json> = description.members().collect();
    let document = lowering::fbl::lower(&members).ok_or("The description has no binding.")?;
    let bytes = serde_json::to_vec(&document).expect("JSON writes");
    let (problems, read) = adp_fbl::documents::load(&bytes);
    let read = read.ok_or_else(|| {
        let lines: Vec<String> = problems.iter().map(ToString::to_string).collect();
        format!("The description's binding does not lower to FBL: {}", lines.join("; "))
    })?;
    let origin = description.origin().unwrap_or_default().to_owned();
    let binding = read
        .bindings
        .iter()
        .find(|(_, b)| b.claims.origins.contains(&origin))
        .or_else(|| read.bindings.first())
        .map(|(_, binding)| binding.clone())
        .ok_or("The description's binding does not lower to FBL.")?;
    let specification = loaded.specification.clone();
    let wire_block = DislDiagramDefinition::wire_block_of(&specification);
    let definition = DislDiagramDefinition::of(specification.clone(), Vec::new(), wire_block.as_deref());
    let label = loaded
        .json
        .pointer("/language/label")
        .and_then(Json::as_str)
        .unwrap_or(&origin);
    let module = RuntimeModule::of(&loaded.json, host);
    let tool = DiagramTool {
        origin: origin.clone(),
        display_name: label.to_owned(),
        definition,
        extensions: binding.claims.extensions.clone(),
        binding: Rc::new(binding),
        module: Some(Rc::new(module)),
    };
    Ok(RuntimeTool {
        tool: Rc::new(tool),
        loaded,
    })
}

/// The layout a description names by a capability (ATD §8.8): the capability and its arguments.
#[derive(Debug, Clone)]
struct CapabilityLayout {
    capability: String,
    arguments: serde_json::Map<String, Json>,
}

/// What a hand-written module gives the engines, taken from a description.
pub struct RuntimeModule {
    plugins: Plugins,
    capabilities: Capabilities,
    layout: Option<CapabilityLayout>,
    /// Each viewer's initial view state, by key: `collapsed` from `persistence.view.initial`.
    initial_view: serde_json::Map<String, Json>,
    /// The view state `view` actions set, by document and element, over the initial one.
    views: RefCell<HashMap<(PathBuf, String), Map>>,
}

impl std::fmt::Debug for RuntimeModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RuntimeModule({:?})", self.layout)
    }
}

impl RuntimeModule {
    fn of(specification: &Json, host: &RuntimeHost) -> Self {
        let layout = specification.pointer("/layout").and_then(|layout| {
            let default = layout["default"].as_str()?;
            let algorithm = &layout["algorithms"][default];
            let capability = algorithm["algorithm"].as_str()?.strip_prefix("capability:")?;
            Some(CapabilityLayout {
                capability: capability.to_owned(),
                arguments: algorithm["x-arguments"].as_object().cloned().unwrap_or_default(),
            })
        });
        let initial_view = specification
            .pointer("/persistence/view/initial")
            .and_then(Json::as_object)
            .cloned()
            .unwrap_or_default();
        RuntimeModule {
            plugins: Rc::clone(&host.plugins),
            capabilities: host.capabilities.clone(),
            layout,
            initial_view,
            views: RefCell::default(),
        }
    }

    /// A value of the description evaluated for `element` in the `element` context: a literal as it is, `{cel}` and
    /// a bare CEL string as their value.
    fn evaluate(specification: &DislSpecification, element: &DislElement, value: &Json) -> Option<Value> {
        let source = match value {
            Json::Object(object) => object.get("cel")?.as_str()?,
            Json::String(text) => text.as_str(),
            Json::Number(n) => return n.as_f64().map(Value::Double),
            Json::Bool(b) => return Some(Value::Bool(*b)),
            _ => return None,
        };
        let program = specification.environment("element", &[]).compile(source).ok()?;
        let variables: HashMap<String, Value> = HashMap::from([
            ("self".to_owned(), element.to_value()),
            ("diagram".to_owned(), element.diagram().to_value()),
        ]);
        program.evaluate(&variables).ok()
    }

    /// A node's size, as its notation's `size` states it, else its `min`.
    fn size(specification: &DislSpecification, element: &DislElement) -> (f64, f64) {
        let size = &specification.root()["notation"]["nodes"][element.type_name()]["size"];
        let min = |i: usize| size["min"][i].as_f64().unwrap_or(0.0);
        let measure = |key: &str, fallback: f64| {
            size.get(key)
                .and_then(|v| Self::evaluate(specification, element, v))
                .and_then(|v| v.as_number())
                .unwrap_or(fallback)
        };
        // In the canvas's units, rounded to 0.01, as a capability's input is (ATD §2.4, its fixtures).
        let round = crate::capabilities::two_sided_tree::round2;
        (round(measure("width", min(0))), round(measure("height", min(1))))
    }
}

impl DiagramModule for RuntimeModule {
    /// The host's plugin `plugin_id`, else its reader capability of that name (ATD §11.7).
    fn persistence(&self, plugin_id: &str) -> Option<Rc<dyn PersistencePlugin>> {
        (self.plugins)(plugin_id).or_else(|| self.capabilities.reader(plugin_id))
    }

    /// Keeps this viewer's view of `element`, as a `view` action sets it.
    fn set_view(&self, document: &DiagramDocument, element: &DislElement, values: &Map) -> bool {
        let key = (document.body_path().to_path_buf(), element.id());
        let mut views = self.views.borrow_mut();
        let view = views.entry(key).or_default();
        for (name, value) in values.iter() {
            view.insert(name.clone(), value.clone());
        }
        true
    }

    /// Each viewer's view of `element`: the description's `persistence.view.initial`, with what `view` actions set.
    fn view_of(&self, document: &DiagramDocument, element: &DislElement) -> Option<Map> {
        let set = self
            .views
            .borrow()
            .get(&(document.body_path().to_path_buf(), element.id()))
            .cloned();
        if self.initial_view.is_empty() && set.is_none() {
            return None;
        }
        let specification = document.tool().definition.specification();
        let mut view = Map::default();
        for (key, value) in &self.initial_view {
            if let Some(value) = Self::evaluate(specification, element, value) {
                view.insert(key.clone(), value);
            }
        }
        for (key, value) in set.iter().flat_map(Map::iter) {
            view.insert(key.clone(), value.clone());
        }
        Some(view)
    }

    /// The centre of every node the layout capability places: each node with the size its notation gives it, the
    /// capability's arguments evaluated for each node, and a collapsed node's descendants left out.
    fn positions(&self, document: &DiagramDocument) -> Option<HashMap<String, (f64, f64)>> {
        let layout = self.layout.as_ref()?;
        let capability = self.capabilities.layout(&layout.capability)?;
        let specification = document.tool().definition.specification();
        let diagram = &document.model().diagram;
        let mut nodes = Vec::new();
        let mut sizes = HashMap::new();
        let mut args = serde_json::Map::new();
        for element in diagram.nodes() {
            let (width, height) = Self::size(specification, &element);
            sizes.insert(element.id(), (width, height));
            let mut attributes = serde_json::Map::new();
            for (key, value) in &layout.arguments {
                if value.get("cel").is_some() {
                    if let Some(evaluated) = Self::evaluate(specification, &element, value)
                        && !evaluated.is_null()
                    {
                        attributes.insert(key.clone(), json!(evaluated.to_string()));
                    }
                } else {
                    args.insert(key.clone(), value.clone());
                }
            }
            let collapsed = self
                .view_of(document, &element)
                .and_then(|view| view.get("collapsed").and_then(Value::as_bool))
                .unwrap_or(false);
            if collapsed {
                attributes.insert("collapsed".into(), json!(true));
            }
            let parent = element.parent();
            let order = parent
                .as_ref()
                .map(|p| p.children().iter().position(|c| *c == element).unwrap_or(0))
                .unwrap_or(0);
            nodes.push(json!({
                "id": element.id(), "width": width, "height": height,
                "parent": parent.map(|p| p.id()), "order": order, "attributes": attributes,
            }));
        }
        // The corners as the algorithm computes them, not rounded, so that the centres are where its module put them.
        let placed = capability
            .place(&json!({ "nodes": nodes, "edges": [] }), &Json::Object(args))
            .ok()?;
        Some(
            placed
                .into_iter()
                .filter_map(|(id, x, y)| {
                    let (width, height) = *sizes.get(&id)?;
                    Some((id, (x + width / 2.0, y + height / 2.0)))
                })
                .collect(),
        )
    }
}
