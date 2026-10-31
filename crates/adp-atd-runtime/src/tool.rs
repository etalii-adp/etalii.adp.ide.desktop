//! A diagram's description as a tool `adp-diagram` opens (etalii.adp spec 035, T126, research R16): the lowered DISL
//! specification, the lowered FBL binding, and the runtime's module, which gives the engines what a hand-written
//! module gives them for today's tools, from the description: the persistence plugins it declares, positions from
//! the layout capability it names, and each viewer's view state from its persistence settings. No code here knows a
//! tool type (FR-040).

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use adp_atd_check::{Description, Finding, Severity, Symbols};
use adp_atd_script::debug::Debugger;
use adp_atd_syntax::Span;
use adp_cel::{Map, Value};
use adp_diagram::disl_definition::DislDiagramDefinition;
use adp_diagram::{DiagramDocument, DiagramModule, DiagramTool, ModulePlacement};
use adp_disl::{DislElement, DislPluginFunctions, DislSpecification};
use adp_fbl::model::{RuleEvent, RuleObserver};
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
    /// One `ATD9010` for each capability the description requires that this host does not provide, or not in the
    /// range required (ATD §13.4): the description still opens, and the gestures that need it are refused.
    pub findings: Vec<Finding>,
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
    let mut document = lowering::fbl::lower(&members).ok_or("The description has no binding.")?;
    // An inline grammar is the binding's text; a grammar in a file is read beside the description, by the checker.
    for (name, binding) in document["bindings"].as_object_mut().into_iter().flatten() {
        if binding["body"]["family"] == "grammar"
            && binding["body"].get("grammar").is_none()
            && let Some(text) = description.grammar(name).and_then(|modules| modules.get("grammar"))
        {
            binding["body"]["grammar"] = json!(text);
        }
    }
    let bytes = serde_json::to_vec(&document).expect("JSON writes");
    let (problems, read) = adp_fbl::documents::load(&bytes);
    let read = read.ok_or_else(|| {
        let lines: Vec<String> = problems.iter().map(ToString::to_string).collect();
        format!("The description's binding does not lower to FBL: {}", lines.join("; "))
    })?;
    let origin = description.origin().unwrap_or_default().to_owned();
    let (name, mut binding) = read
        .bindings
        .iter()
        .find(|(_, b)| b.claims.origins.contains(&origin))
        .or_else(|| read.bindings.first())
        .map(|(name, binding)| (name.clone(), binding.clone()))
        .ok_or("The description's binding does not lower to FBL.")?;
    if let Some(grammar) = binding.body.grammar.as_mut().filter(|g| g.file.is_some())
        && let Some(modules) = description.grammar(&name)
    {
        grammar.modules = modules.clone();
    }
    let specification = loaded.specification.clone();
    let wire_block = DislDiagramDefinition::wire_block_of(&specification);
    let definition = DislDiagramDefinition::of(specification.clone(), Vec::new(), wire_block.as_deref());
    let label = loaded
        .json
        .pointer("/language/label")
        .and_then(Json::as_str)
        .unwrap_or(&origin);
    let mut missing = Vec::new();
    let mut findings = Vec::new();
    for required in description.members().filter(|m| m["kind"] == "requires") {
        let name = required["capability"]["text"].as_str().unwrap_or_default();
        let range = required["range"].as_str().unwrap_or_default();
        if let Some(reason) = host.capabilities.lacks(name, range) {
            findings.push(lacking(required, &reason));
            missing.push((name.to_owned(), reason));
        }
    }
    let mut module = RuntimeModule::of(&loaded.json, host, missing);
    if let Some(debugger) = &host.debugger {
        module.rule_observer = Some(rule_observer(description, Rc::clone(debugger)));
    }
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
        findings,
    })
}

/// What tells the debugger of each binding rule as it reads an entry: it pauses at the rule's line in the
/// description, with the entry's line and the attributes read so far as its locals.
fn rule_observer(description: &Description, debugger: Rc<Debugger>) -> RuleObserver {
    let mut rules: HashMap<String, Json> = HashMap::new();
    for binding in description.members().filter(|m| m["kind"] == "binding") {
        for rule in binding["members"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|m| m["kind"] == "rule")
        {
            let name = rule
                .get("as")
                .map_or_else(|| lowering::values::text(&rule["type"]["name"]), lowering::values::text);
            rules.entry(name).or_insert_with(|| rule.clone());
        }
    }
    Rc::new(move |event: &RuleEvent<'_>| {
        let Some(rule) = rules.get(event.rule) else {
            return;
        };
        let mut locals = vec![
            ("type".to_owned(), Value::from(event.type_name)),
            ("line".to_owned(), Value::Int(event.line as i64)),
        ];
        locals.extend(
            event
                .attributes
                .iter()
                .map(|(name, value)| (name.clone(), value.clone())),
        );
        // A rule reading an entry cannot be stopped part way; a stop takes effect at the next statement of the script.
        let _ = debugger.observe(rule, &format!("{} (binding rule)", event.rule), &locals, event.first);
    })
}

/// The finding of a capability the host lacks, at the `requires` that names it.
fn lacking(required: &Json, reason: &str) -> Finding {
    let span = Span::of(required).unwrap_or_default();
    Finding {
        code: "ATD9010",
        severity: Severity::Warning,
        file: span.file,
        line: span.line,
        column: span.column,
        end_line: span.end_line,
        end_column: span.end_column,
        start: span.start,
        end: span.end,
        message: format!("{reason} The description opens; the gestures that need it are refused."),
        suggestions: Vec::new(),
        fixes: Vec::new(),
    }
}

/// The layout a description names by a capability (ATD §8.8): the capability and its arguments.
#[derive(Debug, Clone)]
struct CapabilityLayout {
    capability: String,
    arguments: serde_json::Map<String, Json>,
}

/// Where a node type's notation places it (ATD §8.2): the attribute and the axis's canvas units per value, on each axis.
#[derive(Debug, Clone)]
struct ScaledPlacement {
    x: ScaledAxis,
    y: ScaledAxis,
}

#[derive(Debug, Clone)]
struct ScaledAxis {
    attribute: String,
    /// The canvas units of one value along the axis (`scale`): 1 when the axis states none.
    scale: f64,
    /// Whether a value along the axis is whole (`precision: 0`).
    whole: bool,
}

impl ScaledAxis {
    /// The canvas coordinate of `element`'s value along this axis; 0 for one that does not read as a number.
    fn canvas(&self, element: &DislElement) -> f64 {
        element.value_of(&self.attribute).as_number().unwrap_or_default() * self.scale
    }

    /// The value a canvas coordinate means along this axis: the nearest whole one on a whole axis.
    fn value(&self, canvas: f64) -> Value {
        let value = canvas / self.scale;
        if self.whole {
            Value::Int(value.round() as i64)
        } else {
            Value::Double(value)
        }
    }
}

/// The placements of the description's node types that have an axis with a `scale` of its own; a placement on axes
/// without one draws its attributes as they are, which the engines do by themselves.
fn scaled_placements(specification: &Json) -> HashMap<String, ScaledPlacement> {
    let coordinates = &specification["coordinates"];
    let axis = |placement: &Json, system: &str, name: &str| -> Option<ScaledAxis> {
        let attribute = placement[name]["attribute"].as_str()?.to_owned();
        let axis = &coordinates["axes"][coordinates["systems"][system][name].as_str()?];
        Some(ScaledAxis {
            attribute,
            scale: axis["scale"].as_f64().filter(|s| *s != 0.0).unwrap_or(1.0),
            whole: axis["precision"].as_i64() == Some(0),
        })
    };
    let default = coordinates["default"].as_str().unwrap_or_default();
    specification["notation"]["nodes"]
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(name, node)| {
            let placement = &node["placement"];
            let system = placement["system"].as_str().unwrap_or(default);
            let scaled = ScaledPlacement {
                x: axis(placement, system, "x")?,
                y: axis(placement, system, "y")?,
            };
            (scaled.x.scale != 1.0 || scaled.y.scale != 1.0).then(|| (name.clone(), scaled))
        })
        .collect()
}

/// What a hand-written module gives the engines, taken from a description.
pub struct RuntimeModule {
    scaled: HashMap<String, ScaledPlacement>,
    plugins: Plugins,
    /// Told of each binding rule as it reads an entry, when the script runs under the debugger.
    rule_observer: Option<RuleObserver>,
    capabilities: Capabilities,
    /// The capabilities the description requires that the host lacks, each with the reason.
    missing: Vec<(String, String)>,
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
    fn of(specification: &Json, host: &RuntimeHost, missing: Vec<(String, String)>) -> Self {
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
            scaled: scaled_placements(specification),
            plugins: Rc::clone(&host.plugins),
            rule_observer: None,
            capabilities: host.capabilities.clone(),
            missing,
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

    /// A query argument of a layout capability (ATD §13.3), evaluated over the diagram, as the text of a CEL constant, as
    /// the capability's fixtures state it; a literal as it is.
    fn query(specification: &DislSpecification, diagram: &adp_disl::DislDiagram, value: &Json) -> Option<Json> {
        let Some(source) = value.get("cel").and_then(Json::as_str) else {
            return Some(value.clone());
        };
        let program = specification.environment("element", &[]).compile(source).ok()?;
        let variables = HashMap::from([("diagram".to_owned(), diagram.to_value())]);
        Some(json!(match program.evaluate(&variables).ok()? {
            Value::String(text) => format!("'{text}'"),
            Value::Double(number) => adp_fbl::planning::dotnet_double(number),
            other => other.to_string(),
        }))
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
    /// A node on an axis with a `scale`: its values in the axis's canvas units (ATD §8.1).
    fn position(&self, element: &DislElement) -> Option<(f64, f64)> {
        let placement = self.scaled.get(&element.type_name())?;
        Some((placement.x.canvas(element), placement.y.canvas(element)))
    }

    /// What a gesture to a canvas point writes of a node on an axis with a `scale`: the values the point means.
    fn placement(&self, element: &DislElement, x: f64, y: f64) -> Option<Result<ModulePlacement, String>> {
        let placement = self.scaled.get(&element.type_name())?;
        let (new_x, new_y) = (placement.x.value(x), placement.y.value(y));
        let canvas = |axis: &ScaledAxis, value: &Value| value.as_number().unwrap_or_default() * axis.scale;
        let bounds =
            |x: f64, y: f64| -> Map { [("x", Value::Double(x)), ("y", Value::Double(y))].into_iter().collect() };
        let new_bounds = bounds(canvas(&placement.x, &new_x), canvas(&placement.y, &new_y));
        let attributes: Map = [
            (placement.x.attribute.as_str(), new_x),
            (placement.y.attribute.as_str(), new_y),
        ]
        .into_iter()
        .collect();
        Some(Ok(ModulePlacement {
            attributes,
            old_bounds: bounds(placement.x.canvas(element), placement.y.canvas(element)),
            new_bounds,
        }))
    }

    fn rule_observer(&self) -> Option<RuleObserver> {
        self.rule_observer.clone()
    }

    /// A `layout` action whose plugin is a capability the host lacks is refused with the reason (ATD §13.4); any other
    /// is left to the engines.
    fn layout(
        &self,
        _specification: &DislSpecification,
        plugin: &str,
        _diagram: &adp_disl::DislDiagram,
    ) -> Option<Result<Vec<adp_disl::DislChange>, String>> {
        let (_, reason) = self.missing.iter().find(|(name, _)| name == plugin)?;
        Some(Err(reason.clone()))
    }

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
        let queries = capability.query_arguments();
        for (key, value) in layout
            .arguments
            .iter()
            .filter(|(key, _)| queries.contains(&key.as_str()))
        {
            if let Some(constant) = Self::query(specification, diagram, value) {
                args.insert(key.clone(), constant);
            }
        }
        let authored = document.authored_positions();
        for element in diagram.nodes() {
            let (width, height) = Self::size(specification, &element);
            sizes.insert(element.id(), (width, height));
            let mut attributes = serde_json::Map::new();
            for (key, value) in layout
                .arguments
                .iter()
                .filter(|(key, _)| !queries.contains(&key.as_str()))
            {
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
            let mut node = json!({
                "id": element.id(), "width": width, "height": height,
                "parent": parent.map(|p| p.id()), "order": order, "attributes": attributes,
            });
            if let Some((x, y)) = authored.get(&element.id()) {
                node["pinned"] = json!({ "x": x, "y": y });
            }
            nodes.push(node);
        }
        let edges: Vec<Json> = diagram
            .relations()
            .into_iter()
            .filter(|relation| !relation.is_derived())
            .map(|relation| json!({ "id": relation.id(), "source": relation.source_id(), "target": relation.target_id() }))
            .collect();
        // The corners as the algorithm computes them, not rounded, so that the centres are where its module put them.
        let placed = capability
            .place(&json!({ "nodes": nodes, "edges": edges }), &Json::Object(args))
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
