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

/// The arguments of a layout that say what the host gives the capability, rather than being passed on to it.
const PLACED_BY_HOST: [&str; 3] = ["boundary", "edges", "sequence"];

/// The layout a description names by a capability (ATD §8.8): the capability and its arguments.
#[derive(Debug, Clone)]
struct CapabilityLayout {
    capability: String,
    arguments: serde_json::Map<String, Json>,
}

/// DISL's standard `tidyTree` (DISL §10.3), run by the capability `layout.tidyTree`'s code: its gaps, and the rows the
/// tool keeps around it, which the contract says are the tool's and not the capability's.
#[derive(Debug, Clone)]
struct TidyLayout {
    gaps: crate::capabilities::tidy_tree::TidyGaps,
    /// Whether a row is drawn at the first height the registration stores for any node of it (`rows.stored: y`).
    stored_rows: bool,
    /// The least space a stored row keeps between its parent's bottom and its own top (`rows.minGap`).
    min_gap: f64,
}

/// Where a node type's notation places it (ATD §8.2): the attribute and the axis's canvas units per value, on each axis.
#[derive(Debug, Clone)]
struct ScaledPlacement {
    x: ScaledAxis,
    y: ScaledAxis,
    /// The attributes of the opposite corner of a node drawn between two points (`x2`, `y2`, DISL section 5.8).
    opposite: Option<(ScaledAxis, ScaledAxis)>,
    /// The second point's axis of each of x and y where the placement states one (`x2`, `y2`).
    seconds: Vec<(bool, ScaledAxis)>,
}

#[derive(Debug, Clone)]
struct ScaledAxis {
    attribute: String,
    /// The canvas units of one value along the axis (`scale`): 1 when the axis states none.
    scale: f64,
    /// The value at canvas 0 (`origin`): a `yearMonth` axis's as its month index.
    origin: f64,
    /// What the placement adds to the value before it is placed (`offset`, DISL section 5.8).
    offset: f64,
    /// A time axis's scale `{unit, size}` (DISL section 5.5): `size` canvas units per step of the unit, the unit
    /// given, or read from the diagram's attribute of that name.
    time: Option<TimeScale>,
    /// Whether a value along the axis is whole (`precision: 0`).
    whole: bool,
    /// The values the axis runs between, where it runs from its maximum to its minimum (`reversed`): their sum.
    reversed: Option<f64>,
}

#[derive(Debug, Clone)]
enum TimeUnitOf {
    Given(String),
    Attribute(String),
}

#[derive(Debug, Clone)]
struct TimeScale {
    unit: TimeUnitOf,
    size: f64,
}

/// The months in one step of a DISL time unit (DISL section 5.5).
fn months_of(unit: &str) -> f64 {
    match unit {
        "quarter" => 3.0,
        "year" => 12.0,
        "decade" => 120.0,
        "century" => 1200.0,
        "millennium" => 12000.0,
        _ => 1.0,
    }
}

impl ScaledAxis {
    /// The canvas units of one value along this axis for `element`'s diagram: a time axis's size per step of its unit.
    fn units(&self, element: &DislElement) -> f64 {
        self.units_in(&element.diagram())
    }

    /// The canvas units of one value along this axis in `diagram`.
    fn units_in(&self, diagram: &adp_disl::DislDiagram) -> f64 {
        match &self.time {
            Some(time) => {
                let unit = match &time.unit {
                    TimeUnitOf::Given(unit) => unit.clone(),
                    TimeUnitOf::Attribute(name) => diagram.value_of(name).to_string(),
                };
                time.size / months_of(unit.trim_matches(['"', '\'']))
            }
            None => self.scale,
        }
    }

    /// The canvas coordinate of `element`'s value along this axis; 0 for one that does not read as a number.
    fn canvas(&self, element: &DislElement) -> f64 {
        // An absent value is drawn at canvas 0, as an attribute that is never set is no value of the axis.
        let set = element.attributes().get(&self.attribute).is_some_and(|v| !v.is_null());
        match element.value_of(&self.attribute).as_number() {
            Some(value) if set => self.at(element, value),
            _ => 0.0,
        }
    }

    /// The canvas coordinate of `value` along this axis, for `element`'s diagram, its placement's `offset` added.
    fn at(&self, element: &DislElement, value: f64) -> f64 {
        (self.reversed.map_or(value, |sum| sum - value) + self.offset - self.origin) * self.units(element)
    }

    /// The value a canvas coordinate means along this axis for `element`'s diagram: the nearest whole one on a whole
    /// axis, and a month on a time axis.
    fn value(&self, element: &DislElement, canvas: f64) -> Value {
        if let Some(time) = &self.time {
            // A whole step of the unit from the origin, halves away from zero (the axis's calendar snapping).
            let months = time.size / self.units(element);
            let steps = (canvas / time.size).round();
            return Value::Int((self.origin + steps * months) as i64);
        }
        let value = canvas / self.units(element) + self.origin - self.offset;
        let value = self.reversed.map_or(value, |sum| sum - value);
        // A whole axis, or an attribute the model holds as an int, takes the nearest whole value.
        let int = element
            .element_type()
            .attributes
            .get(&self.attribute)
            .is_some_and(|a| a.type_name == "int");
        if self.whole || int {
            Value::Int(value.round() as i64)
        } else {
            Value::Double(value)
        }
    }
}

/// The axis `name` of the coordinates as a value-to-canvas mapping, with no attribute.
fn axis_of(specification: &Json, name: &str) -> Option<ScaledAxis> {
    let axis = specification["coordinates"]["axes"].get(name)?;
    Some(ScaledAxis {
        attribute: String::new(),
        origin: match &axis["origin"] {
            Json::String(text) => adp_disl::expressions::year_month::parse(text).map_or(0.0, |m| m as f64),
            other => other.as_f64().unwrap_or(0.0),
        },
        offset: 0.0,
        time: axis["scale"].as_object().map(|scale| TimeScale {
            unit: match &scale["unit"] {
                Json::Object(bound) if bound.get("attribute").is_some() => {
                    TimeUnitOf::Attribute(lowering::values::text(&bound["attribute"]))
                }
                other => TimeUnitOf::Given(other.as_str().unwrap_or("month").to_owned()),
            },
            size: scale["size"].as_f64().unwrap_or(1.0),
        }),
        scale: axis["scale"].as_f64().filter(|s| *s != 0.0).unwrap_or(1.0),
        whole: false,
        reversed: None,
    })
}

/// The placements of the description's node types that have an axis with a `scale` of its own; a placement on axes
/// without one draws its attributes as they are, which the engines do by themselves.
fn scaled_placements(specification: &Json) -> HashMap<String, ScaledPlacement> {
    let coordinates = &specification["coordinates"];
    let axis = |placement: &Json, system: &str, name: &str| -> Option<ScaledAxis> {
        let attribute = placement[name]["attribute"].as_str()?.to_owned();
        let along = name.trim_end_matches('2');
        let axis = &coordinates["axes"][coordinates["systems"][system][along].as_str()?];
        let time = axis["scale"].as_object().map(|scale| TimeScale {
            unit: match &scale["unit"] {
                Json::Object(bound) if bound.get("attribute").is_some() => {
                    TimeUnitOf::Attribute(lowering::values::text(&bound["attribute"]))
                }
                other => TimeUnitOf::Given(other.as_str().unwrap_or("month").to_owned()),
            },
            size: scale["size"].as_f64().unwrap_or(1.0),
        });
        let origin = match &axis["origin"] {
            Json::String(text) => adp_disl::expressions::year_month::parse(text).map_or(0.0, |m| m as f64),
            other => other.as_f64().unwrap_or(0.0),
        };
        Some(ScaledAxis {
            attribute,
            origin,
            offset: placement[name]["offset"].as_f64().unwrap_or(0.0),
            time,
            scale: axis["scale"].as_f64().filter(|s| *s != 0.0).unwrap_or(1.0),
            whole: axis["precision"].as_i64() == Some(0),
            reversed: (axis["reversed"] == true)
                .then(|| axis["min"].as_f64().unwrap_or(0.0) + axis["max"].as_f64().unwrap_or(0.0)),
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
                opposite: axis(placement, system, "x2").zip(axis(placement, system, "y2")),
                seconds: [(true, "x2"), (false, "y2")]
                    .into_iter()
                    .filter_map(|(on_x, name)| Some((on_x, axis(placement, system, name)?)))
                    .collect(),
            };
            let scales = |axis: &ScaledAxis| axis.scale != 1.0 || axis.time.is_some() || axis.origin != 0.0;
            (scales(&scaled.x) || scales(&scaled.y)).then(|| (name.clone(), scaled))
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
    tidy: Option<TidyLayout>,
    /// The default coordinate system's x and y axes, which a point given on the canvas is a domain point of.
    system: Option<(ScaledAxis, ScaledAxis)>,
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
        let tidy = specification.pointer("/layout").and_then(|layout| {
            let algorithm = &layout["algorithms"][layout["default"].as_str()?];
            (algorithm["algorithm"] == "tidyTree").then(|| {
                let defaults = crate::capabilities::tidy_tree::TidyGaps::default();
                let spacing = &algorithm["spacing"];
                let rows = &algorithm["tidyTree"]["rows"];
                TidyLayout {
                    gaps: crate::capabilities::tidy_tree::TidyGaps {
                        node: spacing["node"].as_f64().unwrap_or(defaults.node),
                        layer: spacing["layer"].as_f64().unwrap_or(defaults.layer),
                        component: spacing["component"].as_f64().unwrap_or(defaults.component),
                    },
                    stored_rows: rows["stored"] == "y",
                    min_gap: rows["minGap"].as_f64().unwrap_or(0.0),
                }
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
            tidy,
            system: specification["coordinates"]["default"]
                .as_str()
                .and_then(|default| {
                    let system = &specification["coordinates"]["systems"][default];
                    axis_of(specification, system["x"].as_str()?).zip(axis_of(specification, system["y"].as_str()?))
                })
                .filter(|(x, y)| x.time.is_some() || y.time.is_some() || x.scale != 1.0 || y.scale != 1.0),
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

    /// The fixed size a node's notation states, else the size it is measured at.
    fn stated_size(element: &DislElement) -> (f64, f64) {
        let specification = element.diagram().specification();
        let fixed = &specification.root()["notation"]["nodes"][element.type_name()]["size"]["fixed"];
        match (fixed[0].as_f64(), fixed[1].as_f64()) {
            (Some(width), Some(height)) => (width, height),
            _ => Self::size(&specification, element),
        }
    }

    /// The size of a node that fits its content (DISL section 6.6, `autoSize: fitContent`), measured in its own style's
    /// font and padding: as wide as its widest unwrapped label, or its wrapped ones up to what the maximum leaves them,
    /// within its minimum and maximum, and as tall as its lines: one for an unwrapped label, however short, and for a
    /// wrapped one as many as it takes at that width.
    fn fitted(specification: &DislSpecification, element: &DislElement) -> Option<(f64, f64)> {
        let root = specification.root();
        let node = &root["notation"]["nodes"][element.type_name()];
        let size = &node["size"];
        // `fitWidth` fits the width alone, its height the stated one (DISL section 6.6).
        let fit_width = size["autoSize"] == "fitWidth";
        if size["autoSize"] != "fitContent" && !fit_width {
            return None;
        }
        let styles = &root["notation"]["styles"];
        let setting = |pick: &dyn Fn(&Json) -> Option<f64>| {
            let mut name = node["style"].as_str();
            while let Some(style) = name.map(|n| &styles[n]) {
                if let Some(found) = pick(style) {
                    return Some(found);
                }
                name = style["extends"].as_str().or_else(|| style["extends"][0].as_str());
            }
            None
        };
        let font = setting(&|s: &Json| s["font"]["size"].as_f64()).unwrap_or(14.0);
        let line_height = setting(&|s: &Json| s["font"]["lineHeight"].as_f64()).unwrap_or(1.4);
        let down = setting(&|s: &Json| s["padding"][0].as_f64()).unwrap_or(0.0);
        let across = setting(&|s: &Json| s["padding"][1].as_f64()).unwrap_or(0.0);
        let limit = |key: &str, i: usize| size[key][i].as_f64();
        let (min_width, max_width) = (limit("min", 0).unwrap_or(0.0), limit("max", 0).unwrap_or(f64::MAX));
        let room = max_width - 2.0 * across;
        let mut widest: f64 = 0.0;
        let mut wrapped = Vec::new();
        let mut lines = 0.0;
        for label in node["labels"].as_array().into_iter().flatten() {
            let text = Self::evaluate(specification, element, &label["text"]).map_or(String::new(), |v| match v {
                Value::String(text) => text,
                other => other.to_string(),
            });
            let width = adp_layout::text_metric::width_of(&text, font);
            if label["wrap"] == "word" {
                widest = widest.max(width.min(room));
                wrapped.push(width);
            } else {
                widest = widest.max(width);
                lines += 1.0;
            }
        }
        let width = (widest + 2.0 * across).clamp(min_width, max_width);
        if fit_width {
            let height = size["default"][1].as_f64().or_else(|| limit("min", 1)).unwrap_or(0.0);
            let round = crate::capabilities::two_sided_tree::round2;
            return Some((round(width), round(height)));
        }
        for text_width in wrapped {
            lines += (text_width / (width - 2.0 * across)).ceil();
        }
        let height = (lines * font * line_height + 2.0 * down).max(limit("min", 1).unwrap_or(0.0));
        let round = crate::capabilities::two_sided_tree::round2;
        Some((round(width), round(height)))
    }

    /// A node's size, as it fits its content, else as its notation's `size` states it, else its `min`.
    fn size(specification: &DislSpecification, element: &DislElement) -> (f64, f64) {
        if let Some(fitted) = Self::fitted(specification, element) {
            return fitted;
        }
        let size = &specification.root()["notation"]["nodes"][element.type_name()]["size"];
        // The size a node is drawn at unless it says otherwise (`default`, DISL section 6.6), else its minimum.
        let min = |i: usize| {
            size["default"][i]
                .as_f64()
                .or_else(|| size["min"][i].as_f64())
                .unwrap_or(0.0)
        };
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

impl RuntimeModule {
    /// DISL's standard `rows` (DISL §10.2), run by the shared row packing the hype cycle graph's module arranged with
    /// (`adp-layout`'s `row_packing`, etalii.adp spec 076, T045): each element of a type `extent` names, across from its
    /// `from` to its `to` and as many rows tall as `rows` says, evaluated with `x`, `x2`, `width` and `height` of where
    /// it is drawn, `clearance` apart on a row, and the relations of the type `affinity` names as the links. Each
    /// element whose row changes has the attribute `writes` names set, the last written first, as one step.
    fn rows(
        &self,
        specification: &DislSpecification,
        diagram: &adp_disl::DislDiagram,
    ) -> Option<Result<Vec<adp_disl::DislChange>, String>> {
        use adp_layout::row_packing::{RowItem, pack};
        let algorithm = specification.root()["layout"]["algorithms"]
            .as_object()?
            .values()
            .find(|a| a["algorithm"] == "rows")?
            .clone();
        let settings = &algorithm["rows"];
        let attribute = settings["writes"][0].as_str().unwrap_or("row").to_owned();
        let clearance = settings["clearance"].as_f64().unwrap_or(0.0);
        let mut items = Vec::new();
        let mut placed: Vec<DislElement> = Vec::new();
        for (type_name, extent) in settings["extent"].as_object().into_iter().flatten() {
            let mut nodes: Vec<(usize, DislElement)> =
                diagram.nodes_of_type(type_name).into_iter().enumerate().collect();
            nodes.sort_by_key(|(index, node)| (node.line().unwrap_or(i32::MAX), *index));
            for (_, node) in nodes {
                let Some(placement) = self.scaled.get(&node.type_name()) else {
                    continue;
                };
                if !node
                    .attributes()
                    .get(&placement.x.attribute)
                    .is_some_and(|v| !v.is_null())
                {
                    continue;
                }
                let x = placement.x.canvas(&node);
                let (width, height) = Self::size(specification, &node);
                let x2 = placement
                    .seconds
                    .iter()
                    .find(|(on_x, _)| *on_x)
                    .map_or(x + width, |(_, second)| second.canvas(&node));
                let width = x2 - x;
                let evaluate = |value: &Json| -> Option<f64> {
                    let source = value.get("cel").and_then(Json::as_str).or_else(|| value.as_str())?;
                    let program = specification
                        .environment("element", &["x", "x2", "width", "height"])
                        .compile(source)
                        .ok()?;
                    let variables: HashMap<String, Value> = HashMap::from([
                        ("self".to_owned(), node.to_value()),
                        ("diagram".to_owned(), diagram.to_value()),
                        ("x".to_owned(), Value::Double(x)),
                        ("x2".to_owned(), Value::Double(x2)),
                        ("width".to_owned(), Value::Double(width)),
                        ("height".to_owned(), Value::Double(height)),
                    ]);
                    program.evaluate(&variables).ok()?.as_number()
                };
                let (Some(from), Some(to)) = (evaluate(&extent["from"]), evaluate(&extent["to"])) else {
                    continue;
                };
                let rows = extent.get("rows").and_then(evaluate).map_or(1, |r| r.max(1.0) as usize);
                items.push(RowItem::tall(node.id(), from, to, rows));
                placed.push(node);
            }
        }
        if items.is_empty() {
            return Some(Err("There is nothing to arrange.".to_owned()));
        }
        let affinity = settings["affinity"].as_str().unwrap_or_default();
        let links: Vec<(String, String)> = diagram
            .relations_of_type(affinity)
            .iter()
            .map(|r| (r.source_id().unwrap_or_default(), r.target_id().unwrap_or_default()))
            .collect();
        let linked: Vec<(&str, &str)> = links.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
        let rows = pack(&items, &linked, clearance, &[]);
        let mut changes: Vec<(i32, adp_disl::DislChange)> = placed
            .iter()
            .filter_map(|node| {
                let row = *rows.get(&node.id())? as i64;
                if node.value_of(&attribute).as_number() == Some(row as f64) {
                    return None;
                }
                let mut attributes = Map::default();
                attributes.insert(attribute.clone(), Value::Int(row));
                Some((
                    node.line().unwrap_or(i32::MAX),
                    adp_disl::DislChange::Set {
                        element_id: node.id(),
                        type_name: node.type_name(),
                        attributes,
                    },
                ))
            })
            .collect();
        if changes.is_empty() {
            return Some(Err("This diagram is already arranged.".to_owned()));
        }
        changes.sort_by_key(|(line, _)| std::cmp::Reverse(*line));
        Some(Ok(changes.into_iter().map(|(_, change)| change).collect()))
    }

    /// The centre of every node of a `tidyTree` (DISL §10.3): x as the tree alone places it, and every child of one
    /// parent in one row, at the first height the registration stores for any of them, else `layer` below its
    /// parent's bottom, never closer than `minGap`.
    fn tidy_positions(tidy: &TidyLayout, document: &DiagramDocument) -> HashMap<String, (f64, f64)> {
        use crate::capabilities::tidy_tree::{TidyTree, compute};
        let diagram = &document.model().diagram;
        let nodes = diagram.nodes();
        let (width, height) = nodes.first().map_or((0.0, 0.0), Self::stated_size);
        let roots: Vec<DislElement> = nodes.iter().filter(|n| n.parent().is_none()).cloned().collect();
        let tree = TidyTree {
            roots: roots.iter().map(DislElement::id).collect(),
            children: nodes
                .iter()
                .map(|n| (n.id(), n.children().iter().map(DislElement::id).collect()))
                .collect(),
            width,
            height,
        };
        let computed = compute(&tree, &tidy.gaps);
        let stored = if tidy.stored_rows {
            document.authored_positions()
        } else {
            HashMap::new()
        };
        let mut positions = HashMap::new();
        let mut rows = vec![(roots, 0.0, f64::NEG_INFINITY)];
        while let Some((row, hanging, floor)) = rows.pop() {
            let dragged = row.iter().find_map(|n| stored.get(&n.id()).map(|p| p.1));
            let y = dragged.unwrap_or(hanging).max(floor);
            for node in row.iter().rev() {
                let x = computed.get(&node.id()).map_or(0.0, |p| p.0);
                positions.insert(node.id(), (x + width / 2.0, y + height / 2.0));
                rows.push((node.children(), y + height + tidy.gaps.layer, y + height + tidy.min_gap));
            }
        }
        positions
    }
}

impl DiagramModule for RuntimeModule {
    /// A node that fits its content, measured as the layout measures it.
    fn size_of(&self, document: &DiagramDocument, element: &DislElement) -> Option<(f64, f64)> {
        if let Some(placement) = self.scaled.get(&element.type_name())
            && let Some((x2, y2)) = &placement.opposite
        {
            return Some((
                (x2.canvas(element) - placement.x.canvas(element)).abs(),
                (y2.canvas(element) - placement.y.canvas(element)).abs(),
            ));
        }
        let specification = document.tool().definition.specification();
        // A size bound to the element's values, or fixed (DISL section 6.6).
        let bound = &specification.root()["notation"]["nodes"][element.type_name()]["size"];
        let sized = self
            .layout
            .as_ref()
            .and_then(|layout| self.capabilities.layout(&layout.capability))
            .is_some_and(|capability| capability.draws_its_sizes());
        // A width fitted to the text (`fitWidth`) is what a layout seats the node by; it is drawn at its stated size.
        let fits_content = bound["autoSize"] == "fitContent";
        Self::fitted(specification, element)
            .filter(|_| fits_content)
            .or_else(|| {
                let stated =
                    bound.get("width").is_some() || bound.get("height").is_some() || bound.get("fixed").is_some();
                (sized && stated).then(|| Self::stated_size(element))
            })
    }

    /// A node on an axis with a `scale`: its values in the axis's canvas units (ATD §8.1).
    fn position(&self, element: &DislElement) -> Option<(f64, f64)> {
        let placement = self.scaled.get(&element.type_name())?;
        if let Some((x2, y2)) = &placement.opposite {
            // The middle of the two points: a node of a size of its own is drawn around where it is placed.
            return Some((
                (placement.x.canvas(element) + x2.canvas(element)) / 2.0,
                (placement.y.canvas(element) + y2.canvas(element)) / 2.0,
            ));
        }
        // The anchored point itself, as the cases record it (research R16).
        Some((placement.x.canvas(element), placement.y.canvas(element)))
    }

    /// What a gesture to a canvas point writes of a node on an axis with a `scale`: the values the point means.
    fn placement(&self, element: &DislElement, x: f64, y: f64) -> Option<Result<ModulePlacement, String>> {
        let placement = self.scaled.get(&element.type_name())?;
        let specification = element.diagram().specification();
        let canvas = |axis: &ScaledAxis, value: &Value| axis.at(element, value.as_number().unwrap_or_default());
        let bounds =
            |x: f64, y: f64| -> Map { [("x", Value::Double(x)), ("y", Value::Double(y))].into_iter().collect() };
        let (new_x, new_y) = (placement.x.value(element, x), placement.y.value(element, y));
        let mut values = vec![
            (placement.x.attribute.clone(), new_x.clone()),
            (placement.y.attribute.clone(), new_y.clone()),
        ];
        // A node drawn between two points keeps its extent: its second point moves as far as its first.
        {
            for (on_x, second) in &placement.seconds {
                let (first, moved) = if *on_x {
                    (&placement.x, &new_x)
                } else {
                    (&placement.y, &new_y)
                };
                if second.attribute == first.attribute {
                    continue;
                }
                let old = element.value_of(&first.attribute).as_number().unwrap_or_default();
                let delta = moved.as_number().unwrap_or_default() - old;
                if let Some(end) = element.value_of(&second.attribute).as_number() {
                    let value = if second.time.is_some() || second.whole {
                        Value::Int((end + delta).round() as i64)
                    } else {
                        Value::Double(end + delta)
                    };
                    values.push((second.attribute.clone(), value));
                }
            }
        }
        let new_bounds = bounds(canvas(&placement.x, &new_x), canvas(&placement.y, &new_y));
        let type_name = element.type_name();
        let attributes: Map = values
            .into_iter()
            .map(|(name, value)| {
                let stored = adp_disl::behavior::write::stored_form(&specification, &type_name, &name, &value);
                (name, stored)
            })
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

    /// The domain point a canvas point means on the default coordinate system's scaled axes (DISL section 5.5): what
    /// an operation run there, or a tool dropped there, reads as `position`.
    fn position_in(&self, diagram: &adp_disl::DislDiagram, x: f64, y: f64) -> Option<Map> {
        let (along_x, along_y) = self.system.as_ref()?;
        let value = |axis: &ScaledAxis, canvas: f64| canvas / axis.units_in(diagram) + axis.origin;
        Some(
            [
                ("x", Value::Double(value(along_x, x))),
                ("y", Value::Double(value(along_y, y))),
            ]
            .into_iter()
            .collect(),
        )
    }

    /// A description's change rules are the runtime's to run: no code of the tool type does their work.
    fn runs_change_rules(&self) -> bool {
        true
    }

    /// A `plugin` action naming a capability at the `arrange` extension point (ATD §13.3): the nodes the notation lets
    /// a person place, with their sizes, and the stored relations between them, given to it with the action's
    /// arguments; each `move` it gives stored as the node's position, its centre, and a finding of severity `error`
    /// the refusal. A capability the host lacks is refused with the reason (ATD §13.4).
    fn plugin(
        &self,
        document: &DiagramDocument,
        name: &str,
        arguments: &Map,
    ) -> Option<Result<Vec<adp_diagram::DiagramChange>, String>> {
        if let Some((_, reason)) = self.missing.iter().find(|(missing, _)| missing == name) {
            return Some(Err(reason.clone()));
        }
        let arranger = self.capabilities.arranger(name)?;
        let specification = document.tool().definition.specification();
        let diagram = &document.model().diagram;
        let placed_by_notation = |element: &DislElement| {
            let placement = &specification.root()["notation"]["nodes"][element.type_name()]["placement"];
            placement["x"].get("cel").is_some() || placement["movable"] == false
        };
        let nodes: Vec<DislElement> = diagram.nodes().into_iter().filter(|n| !placed_by_notation(n)).collect();
        let elements: Vec<Json> = nodes
            .iter()
            .map(|node| {
                let (width, height) = Self::size(specification, node);
                json!({ "id": node.id(), "width": width, "height": height })
            })
            .collect();
        let relations: Vec<Json> = diagram
            .relations()
            .into_iter()
            .filter(|relation| !relation.is_derived())
            .filter_map(|relation| {
                Some(json!({ "source": relation.source()?.id(), "target": relation.target()?.id() }))
            })
            .collect();
        let args: serde_json::Map<String, Json> = arguments
            .iter()
            .map(|(key, value)| {
                let value = match value {
                    Value::Int(n) => json!(n),
                    Value::Double(n) => json!(n),
                    other => json!(other.to_string()),
                };
                (key.clone(), value)
            })
            .collect();
        let moves = match arranger.moves(
            &json!({ "elements": elements, "relations": relations }),
            &Json::Object(args),
        ) {
            Ok(moves) => moves,
            Err(reason) => return Some(Err(reason)),
        };
        Some(moves.map(|moves| {
            moves
                .into_iter()
                .map(|(id, x, y)| adp_diagram::DiagramChange::Place { id, x, y })
                .collect()
        }))
    }

    /// Every entry a rule the specification maps as unreadable read (DISL section 8.7, `persistence.typeMap`), as
    /// `std.unreadableEntry` at its line, with the entry's text and line as `detail.entry`, which the built-in's
    /// `message` words: no sentence is the runtime's.
    fn reader_findings(&self, document: &DiagramDocument) -> Vec<adp_disl::DislReaderFinding> {
        let body = document.body_bytes();
        document
            .model()
            .unreadable
            .iter()
            .map(|element| {
                let span = element.own_span;
                let text = String::from_utf8_lossy(&body[span.start.min(body.len())..span.end.min(body.len())]);
                let text = text.trim_end_matches(['\r', '\n']).to_owned();
                let mut finding = adp_disl::DislReaderFinding::unreadable(&text, Some(element.line as i32), None);
                let entry: Map = [("text", Value::String(text)), ("line", Value::Int(element.line as i64))]
                    .into_iter()
                    .collect();
                finding.detail.insert("entry", Value::map(entry));
                finding
            })
            .collect()
    }

    /// A `layout` action whose plugin is a capability the host lacks is refused with the reason (ATD §13.4); any other
    /// is left to the engines.
    fn layout(
        &self,
        specification: &DislSpecification,
        plugin: &str,
        diagram: &adp_disl::DislDiagram,
    ) -> Option<Result<Vec<adp_disl::DislChange>, String>> {
        if plugin == "rows" {
            return self.rows(specification, diagram);
        }
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

    /// The registration without the positions stored for `ids`: their entries of its `layout:` block gone, and the
    /// block's line with its last entry (FBL section 8.4).
    fn unpinned(&self, document: &DiagramDocument, ids: &[String]) -> Option<Vec<u8>> {
        let text = String::from_utf8_lossy(document.registration_bytes()?).into_owned();
        let lines: Vec<&str> = text.split_inclusive('\n').collect();
        let header = lines.iter().position(|l| l.trim_end() == "layout:")?;
        let entries = lines[header + 1..]
            .iter()
            .take_while(|l| l.starts_with(' ') || l.starts_with('\t'))
            .count();
        let block = header + 1..header + 1 + entries;
        let gone = |line: &str| {
            line.split_once(':')
                .is_some_and(|(key, _)| ids.iter().any(|id| id == key.trim()))
        };
        let kept: Vec<&str> = lines[block.clone()].iter().copied().filter(|l| !gone(l)).collect();
        if kept.len() == entries {
            return None;
        }
        let mut written = String::new();
        written.extend(lines[..header].iter().copied());
        if !kept.is_empty() {
            written.push_str(lines[header]);
            written.extend(kept);
        }
        written.extend(lines[block.end..].iter().copied());
        Some(written.into_bytes())
    }

    /// Each viewer's view of `element`: the description's `persistence.view.initial`, with what `view` actions set;
    /// and, of a node a `tidyTree` lays out, whether the registration pins it and its `bounds` (DISL section 11.6),
    /// which come from the registration and the layout rather than from what was set.
    fn view_of(&self, document: &DiagramDocument, element: &DislElement) -> Option<Map> {
        let set = self
            .views
            .borrow()
            .get(&(document.body_path().to_path_buf(), element.id()))
            .cloned();
        let authored = document.authored_positions();
        let placed = self.tidy.is_some();
        if self.initial_view.is_empty() && set.is_none() && !placed {
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
        if placed && !element.is_relation() {
            let (width, height) = Self::stated_size(element);
            let centre = self
                .tidy
                .as_ref()
                .and_then(|tidy| Self::tidy_positions(tidy, document).get(&element.id()).copied());
            view.insert("pinned".to_owned(), Value::Bool(authored.contains_key(&element.id())));
            if let Some((x, y)) = centre {
                let bounds: Map = [
                    ("x", Value::Double(x - width / 2.0)),
                    ("y", Value::Double(y - height / 2.0)),
                    ("width", Value::Double(width)),
                    ("height", Value::Double(height)),
                ]
                .into_iter()
                .collect();
                view.insert("bounds".to_owned(), Value::map(bounds));
            }
        }
        Some(view)
    }

    /// The centre of every node the layout capability places: each node with the size its notation gives it, the
    /// capability's arguments evaluated for each node, and a collapsed node's descendants left out.
    fn positions(&self, document: &DiagramDocument) -> Option<HashMap<String, (f64, f64)>> {
        if let Some(tidy) = &self.tidy {
            return Some(Self::tidy_positions(tidy, document));
        }
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
        // What the view shows is what is laid out (DISL section 3.5), with each boundary its layout draws around it.
        let is_boundary = |element: &DislElement| {
            layout
                .arguments
                .get("boundary")
                .and_then(|value| Self::evaluate(specification, element, value))
                .is_some_and(|value| value == Value::Bool(true))
        };
        let shown = diagram.shown_in(&specification.default_viewpoint());
        let shown_ids: std::collections::HashSet<String> = shown.iter().map(DislElement::id).collect();
        // A node its notation places by an expression (DISL section 6.4, a loop's badge at its variables' centroid) is
        // not the layout's to place.
        let computed = |element: &DislElement| {
            specification.root()["notation"]["nodes"][element.type_name()]["placement"]["x"]
                .get("cel")
                .is_some()
        };
        let mut placed: Vec<DislElement> = diagram
            .nodes()
            .into_iter()
            .filter(|element| (shown_ids.contains(&element.id()) || is_boundary(element)) && !computed(element))
            .collect();
        // The order the description gives them, else the model's.
        if let Some(sequence) = layout.arguments.get("sequence") {
            placed.sort_by_cached_key(|element| {
                Self::evaluate(specification, element, sequence).and_then(|value| value.as_number().map(|n| n as i64))
            });
        }
        let boundaries: std::collections::HashSet<String> = placed
            .iter()
            .filter(|element| is_boundary(element))
            .map(DislElement::id)
            .collect();
        let placed_ids: std::collections::HashSet<String> = placed.iter().map(DislElement::id).collect();
        let placed_id = |id: String| {
            if boundaries.contains(&id) {
                format!("boundary:{id}")
            } else {
                id
            }
        };
        for element in placed {
            let boundary = boundaries.contains(&element.id());
            let (width, height) = if boundary {
                (0.0, 0.0)
            } else if capability.draws_its_sizes() {
                Self::stated_size(&element)
            } else {
                Self::size(specification, &element)
            };
            sizes.insert(element.id(), (width, height));
            let mut attributes = serde_json::Map::new();
            for (key, value) in layout
                .arguments
                .iter()
                .filter(|(key, _)| !queries.contains(&key.as_str()) && !PLACED_BY_HOST.contains(&key.as_str()))
            {
                if value.get("cel").is_some() {
                    if let Some(evaluated) = Self::evaluate(specification, &element, value)
                        && !evaluated.is_null()
                    {
                        // A list stays one, as a capability's query reads it (ATD §13.3).
                        attributes.insert(
                            key.clone(),
                            match &evaluated {
                                Value::List(items) => {
                                    Json::Array(items.iter().map(|item| json!(item.to_string())).collect())
                                }
                                other => json!(other.to_string()),
                            },
                        );
                    }
                } else {
                    args.insert(key.clone(), value.clone());
                }
            }
            if boundary {
                attributes.insert("boundary".into(), json!(true));
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
            // The nearest ancestor that is laid out too.
            let holder = std::iter::successors(parent, DislElement::parent).find(|a| placed_ids.contains(&a.id()));
            let mut node = json!({
                "id": placed_id(element.id()), "width": width, "height": height,
                "parent": holder.map(|p| placed_id(p.id())), "order": order, "attributes": attributes,
            });
            if !boundary && let Some((x, y)) = authored.get(&element.id()) {
                node["pinned"] = json!({ "x": x, "y": y });
            }
            nodes.push(node);
        }
        // The relations the description says the layout follows, else the stored ones.
        let followed = layout.arguments.get("edges");
        let edges: Vec<Json> = diagram
            .relations()
            .into_iter()
            .filter(|relation| match followed {
                Some(value) => Self::evaluate(specification, relation, value) == Some(Value::Bool(true)),
                None => !relation.is_derived(),
            })
            .map(|relation| {
                // An end as the model holds it, not as it was read: under a prefix the two differ.
                let end = |element: Option<DislElement>, read: Option<String>| element.map(|e| e.id()).or(read);
                json!({
                    "id": relation.id(),
                    "source": end(relation.source(), relation.source_id()),
                    "target": end(relation.target(), relation.target_id()),
                })
            })
            .collect();
        // The corners as the algorithm computes them, not rounded, so that the centres are where its module put them.
        let placed = capability
            .place(&json!({ "nodes": nodes, "edges": edges }), &Json::Object(args))
            .ok()?;
        // A capability that does not consult pins leaves a stored position to the host: the node is drawn there.
        let stored = if capability.consults_pins() {
            HashMap::new()
        } else {
            authored
        };
        Some(
            placed
                .into_iter()
                .filter_map(|(id, x, y)| {
                    let (width, height) = *sizes.get(&id)?;
                    // A stored position is where the node is drawn, as the registration keeps it.
                    if let Some(stored) = stored.get(&id) {
                        return Some((id, *stored));
                    }
                    Some((id, (x + width / 2.0, y + height / 2.0)))
                })
                .collect(),
        )
    }
}
