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
    diagram_tool_for(description, symbols, host, None)
}

/// [`diagram_tool`] with the binding that claims `body`, a body's file name, among a description's several bindings,
/// one per format family (ATD §11.1): the first of the tool type's origin whose extensions end the name, else the first
/// of its origin.
pub fn diagram_tool_for(
    description: &Description,
    symbols: &Symbols,
    host: &RuntimeHost,
    body: Option<&str>,
) -> Result<RuntimeTool, String> {
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
    let claims = |b: &adp_fbl::FblBinding| {
        body.is_some_and(|body| {
            let body = body.to_ascii_lowercase();
            b.claims
                .extensions
                .iter()
                .any(|e| body.ends_with(&e.to_ascii_lowercase()))
        })
    };
    let (name, mut binding) = read
        .bindings
        .iter()
        .find(|(_, b)| b.claims.origins.contains(&origin) && claims(b))
        .or_else(|| read.bindings.iter().find(|(_, b)| b.claims.origins.contains(&origin)))
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
/// Sizes by node id: a width and a height.
type Sizes = HashMap<String, (f64, f64)>;

const PLACED_BY_HOST: [&str; 3] = ["boundary", "edges", "sequence"];

/// The layout a description names by a capability (ATD §8.8): the capability and its arguments.
#[derive(Debug, Clone)]
struct CapabilityLayout {
    capability: String,
    arguments: serde_json::Map<String, Json>,
}

/// DISL's standard `layered` layout, left to right (DISL §10.3): its gaps between the nodes of a layer and between
/// layers.
#[derive(Debug, Clone, Copy)]
struct LayeredLayout {
    node: f64,
    layer: f64,
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
    /// Whether the point is the node's centre (`anchor: center`), rather than its top-left.
    centred: bool,
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
    /// Whether the axis holds instants (`valueType: datetime` or `date`, DISL section 5.5): a value is drawn at the
    /// seconds it is after the axis's origin, one canvas unit a second.
    instants: bool,
}

/// The seconds since 1970-01-01T00:00:00Z of an instant as a time axis reads one (DISL section 5.5), at face value: the
/// forms the core's instants read; none for any other text.
fn seconds_of(text: &str) -> Option<f64> {
    use adp_disl::expressions::instants;
    instants::parse(text.trim().trim_matches(['"', '\''])).map(|ticks| ticks as f64 / instants::TICKS_PER_SECOND as f64)
}

/// The text of the instant `seconds` after the epoch, written as finely as `written` was: a date, at the start of its
/// day, or a date and a time to the second, the second rounded half to even.
fn instant_text(seconds: f64, written: &str) -> String {
    use adp_disl::expressions::instants::{self, Precision, TICKS_PER_DAY, TICKS_PER_SECOND};
    let ticks = seconds.round_ties_even() as i64 * TICKS_PER_SECOND;
    let (year, month, day) = instants::civil_from_days(ticks.div_euclid(TICKS_PER_DAY));
    let date = format!("{year:04}-{month:02}-{day:02}");
    match Precision::of(written) {
        Precision::Date => date,
        Precision::DateTime => {
            let seconds = ticks.rem_euclid(TICKS_PER_DAY) / TICKS_PER_SECOND;
            format!(
                "{date}T{:02}:{:02}:{:02}",
                seconds / 3600,
                seconds / 60 % 60,
                seconds % 60
            )
        }
    }
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
        if self.instants {
            let value = element.value_of(&self.attribute);
            let text = match &value {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            };
            return match seconds_of(&text) {
                Some(seconds) if set => seconds + self.offset - self.origin,
                _ => 0.0,
            };
        }
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
        if self.instants {
            // An instant, written as finely as the value it replaces.
            let written = match element.value_of(&self.attribute) {
                Value::String(text) => text,
                other => other.to_string(),
            };
            return Value::String(instant_text(canvas - self.offset + self.origin, &written));
        }
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
            Json::String(text) if is_instants(axis) => seconds_of(text).unwrap_or(0.0),
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
        whole: axis["precision"].as_i64() == Some(0),
        reversed: None,
        instants: is_instants(axis),
    })
}

/// Whether an axis holds instants (`valueType: datetime` or `date`).
fn is_instants(axis: &Json) -> bool {
    matches!(axis["valueType"].as_str(), Some("datetime" | "date"))
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
            Json::String(text) if is_instants(axis) => seconds_of(text).unwrap_or(0.0),
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
            instants: is_instants(axis),
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
                centred: placement["anchor"] == "center",
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
    /// Where pinned positions are kept in the body (DISL section 11.6, `view.bind.pinned`).
    pinned: Option<crate::view_bind::PinnedBind>,
    /// Where group states are kept in the body (DISL 0.11, `view.bind.groups`).
    groups: Option<crate::view_bind::GroupsBind>,
    /// Where kept filters' values are kept in the body (DISL 0.4, `view.bind.filters`).
    filters: Option<crate::view_bind::FiltersBind>,
    /// DISL's `force` layout with `tiers` (DISL section 10.4): the rings from the centre outwards.
    force: Option<Vec<Vec<String>>>,
    /// DISL's standard `layered` layout, left to right (DISL section 10.3).
    layered: Option<LayeredLayout>,
    /// The force layout's last placements, by document, with the reading they were made for: a live change starts
    /// from them (DISL section 10.4, "little movement").
    forced: RefCell<HashMap<PathBuf, (u64, adp_layout::Placements)>>,
    /// The sizes the layout capability last gave the nodes it resizes, by document and then by node id.
    resized: RefCell<HashMap<PathBuf, Sizes>>,
    /// Whether a drop from the toolbox pins the new node (DISL section 10.4, "a drag pins"): `respect: pinned` with a
    /// `trigger` other than `manual`.
    drop_pins: bool,
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
            pinned: crate::view_bind::PinnedBind::of(specification),
            groups: crate::view_bind::GroupsBind::of(specification),
            filters: crate::view_bind::FiltersBind::of(specification),
            force: specification.pointer("/layout").and_then(|layout| {
                let algorithm = &layout["algorithms"][layout["default"].as_str()?];
                (algorithm["algorithm"] == "force").then(|| {
                    algorithm["force"]["tiers"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(|tier| match tier {
                            Json::Array(types) => types.iter().filter_map(Json::as_str).map(str::to_owned).collect(),
                            other => other.as_str().map(|t| vec![t.to_owned()]).unwrap_or_default(),
                        })
                        .collect()
                })
            }),
            forced: RefCell::default(),
            layered: specification.pointer("/layout").and_then(|layout| {
                let algorithm = &layout["algorithms"][layout["default"].as_str()?];
                (algorithm["algorithm"] == "layered" && algorithm["direction"] == "right").then(|| LayeredLayout {
                    node: algorithm["spacing"]["node"].as_f64().unwrap_or(32.0),
                    layer: algorithm["spacing"]["layer"].as_f64().unwrap_or(80.0),
                })
            }),
            resized: RefCell::default(),
            drop_pins: specification.pointer("/layout/respect").and_then(Json::as_str) == Some("pinned")
                && specification.pointer("/layout/trigger").and_then(Json::as_str) != Some("manual"),
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
        let notation = adp_scene::notation::compile::node_notation(&specification, &element.type_name());
        let fixed = &notation["size"]["fixed"];
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
        let node = &adp_scene::notation::compile::node_notation(specification, &element.type_name());
        let size = &node["size"];
        // `fitWidth` fits the width alone, its height the stated one (DISL section 6.6).
        let fit_width = size["autoSize"] == "fitWidth";
        if size["autoSize"] != "fitContent" && !fit_width {
            return None;
        }
        let styles = &root["notation"]["styles"];
        // A style named, or stated in place over the one it extends.
        let setting = |pick: &dyn Fn(&Json) -> Option<f64>| {
            let mut style = match &node["style"] {
                Json::String(name) => Some(&styles[name.as_str()]),
                Json::Object(_) => Some(&node["style"]),
                _ => None,
            };
            while let Some(found_style) = style.filter(|s| !s.is_null()) {
                if let Some(found) = pick(found_style) {
                    return Some(found);
                }
                style = found_style["extends"]
                    .as_str()
                    .or_else(|| found_style["extends"][0].as_str())
                    .map(|n| &styles[n]);
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
        // A label drawn outside the node (`outside-top`, a frame's title) takes no room in it.
        let inside = |label: &&Json| !label["position"].as_str().is_some_and(|p| p.starts_with("outside"));
        for label in node["labels"].as_array().into_iter().flatten().filter(inside) {
            // A label bound to an attribute (`label @name`) reads its value.
            let value = match label["text"]["attribute"].as_str() {
                Some(attribute) => Some(element.value_of(attribute)),
                None => Self::evaluate(specification, element, &label["text"]),
            };
            let text = value.map_or(String::new(), |v| match v {
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
        let (width, height) = Self::exact_size(specification, element);
        let round = crate::capabilities::two_sided_tree::round2;
        (round(width), round(height))
    }

    /// [`Self::size`] as its expressions give it, unrounded: what a capability that measures by the drawn sizes
    /// themselves is given (a Sankey bar's height, which its stacked bands must fill exactly).
    fn exact_size(specification: &DislSpecification, element: &DislElement) -> (f64, f64) {
        if let Some(fitted) = Self::fitted(specification, element) {
            return fitted;
        }
        let notation = adp_scene::notation::compile::node_notation(specification, &element.type_name());
        let size = &notation["size"];
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
        (measure("width", min(0)), measure("height", min(1)))
    }
}

impl RuntimeModule {
    /// DISL's `force` layout with `tiers` (DISL section 10.4), by the core's tiered force layout: the centre of every
    /// node the view shows, a pinned one at its stored position and a fixed point for the others; a change of the
    /// reading starts from the last placements.
    fn force_positions(&self, tiers: &[Vec<String>], document: &DiagramDocument) -> HashMap<String, (f64, f64)> {
        use adp_layout::{LayoutAlgorithm, LayoutConnection, LayoutDefinition, LayoutElement, LayoutInput, LayoutMode};
        use std::hash::{Hash, Hasher};
        let specification = document.tool().definition.specification();
        let diagram = &document.model().diagram;
        let pinned = self
            .pinned
            .as_ref()
            .map(|bind| bind.positions(document))
            .unwrap_or_else(|| document.authored_positions());
        let shown: Vec<DislElement> = diagram
            .shown_in(&specification.default_viewpoint())
            .into_iter()
            .filter(|element| !element.is_relation() && element.parent().is_none())
            .collect();
        let ids: std::collections::HashSet<String> = shown.iter().map(DislElement::id).collect();
        let input = LayoutInput {
            elements: shown
                .iter()
                .map(|element| {
                    let (width, height) = Self::size(specification, element);
                    let at = pinned.get(&element.id());
                    LayoutElement {
                        element_type: Some(element.type_name()),
                        pinned: at.is_some(),
                        ..LayoutElement::new(
                            element.id(),
                            at.map_or(0.0, |p| p.0),
                            at.map_or(0.0, |p| p.1),
                            width,
                            height,
                        )
                    }
                })
                .collect(),
            connections: diagram
                .relations()
                .into_iter()
                .filter_map(|relation| Some((relation.source()?.id(), relation.target()?.id())))
                .filter(|(from, to)| ids.contains(from) && ids.contains(to))
                .map(|(from, to)| LayoutConnection::new(from, to))
                .collect(),
        };
        // Each reading of the body is a live change the layout starts again from, whatever it changed.
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        document.body_bytes().hash(&mut hasher);
        let reading = hasher.finish();
        let key = document.body_path().to_path_buf();
        let mut forced = self.forced.borrow_mut();
        let placements = match forced.get(&key) {
            Some((made_for, placements)) if *made_for == reading => placements.clone(),
            previous => {
                let definition = LayoutDefinition {
                    tiers: tiers.to_vec(),
                    ..LayoutDefinition::with_modes(&[LayoutMode::TieredForce])
                };
                let previous = previous.map(|(_, placements)| placements.clone());
                let placements = adp_layout::TieredForceLayout
                    .place(&input, &definition, previous.as_ref())
                    .unwrap_or_default();
                forced.insert(key, (reading, placements.clone()));
                placements
            }
        };
        let mut positions: HashMap<String, (f64, f64)> =
            placements.into_iter().map(|(id, p)| (id, (p.x, p.y))).collect();
        for (id, at) in pinned {
            if ids.contains(&id) {
                positions.insert(id, at);
            }
        }
        positions
    }

    /// Where every node stands by DISL's `layered` layout left to right, its centre: the nodes at the top of
    /// the diagram, and inside an open container the nodes it holds by the algorithm `layout.containers` names for its
    /// type, the container grown to them by its padding and header (DISL sections 6.7 and 10.3). A closed container
    /// holds nothing drawn.
    fn layered_positions(&self, layered: &LayeredLayout, document: &DiagramDocument) -> HashMap<String, (f64, f64)> {
        let specification = document.tool().definition.specification();
        let diagram = &document.model().diagram;
        let roots: Vec<DislElement> = diagram.nodes().into_iter().filter(|n| n.parent().is_none()).collect();
        let mut positions = HashMap::new();
        let mut sizes = HashMap::new();
        self.layer(
            specification,
            document,
            &roots,
            *layered,
            (0.0, 0.0),
            &mut positions,
            &mut sizes,
        );
        self.resized
            .borrow_mut()
            .insert(document.body_path().to_path_buf(), sizes);
        positions
    }

    /// `nodes` laid out in layers from `origin`, each at its size, an open container first laid out inside; returns the
    /// extent they take.
    #[allow(clippy::too_many_arguments)]
    fn layer(
        &self,
        specification: &DislSpecification,
        document: &DiagramDocument,
        nodes: &[DislElement],
        layered: LayeredLayout,
        origin: (f64, f64),
        positions: &mut HashMap<String, (f64, f64)>,
        sizes: &mut Sizes,
    ) -> (f64, f64) {
        let index: HashMap<String, usize> = nodes.iter().enumerate().map(|(i, n)| (n.id(), i)).collect();
        let mut edges: Vec<(usize, usize)> = Vec::new();
        for relation in document.model().diagram.relations() {
            let (Some(source), Some(target)) = (relation.source(), relation.target()) else {
                continue;
            };
            if let (Some(&s), Some(&t)) = (index.get(&source.id()), index.get(&target.id()))
                && s != t
                && !edges.contains(&(s, t))
            {
                edges.push((s, t));
            }
        }
        // The edges that close a cycle, found by a walk in the model's order, are left out.
        fn visit(n: usize, edges: &[(usize, usize)], state: &mut [u8], kept: &mut Vec<(usize, usize)>) {
            state[n] = 1;
            for &(s, t) in edges.iter().filter(|(s, _)| *s == n) {
                match state[t] {
                    0 => {
                        kept.push((s, t));
                        visit(t, edges, state, kept);
                    }
                    2 => kept.push((s, t)),
                    _ => {}
                }
            }
            state[n] = 2;
        }
        let mut state = vec![0u8; nodes.len()];
        let mut kept = Vec::new();
        for n in 0..nodes.len() {
            if state[n] == 0 {
                visit(n, &edges, &mut state, &mut kept);
            }
        }
        let mut layer = vec![0usize; nodes.len()];
        for _ in 0..nodes.len() {
            let mut changed = false;
            for &(s, t) in &kept {
                if layer[t] < layer[s] + 1 {
                    layer[t] = layer[s] + 1;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        // An open container's size: what it holds, laid out at its padding below its header, else its own size.
        let size_of =
            |node: &DislElement, positions: &mut HashMap<String, (f64, f64)>, sizes: &mut Sizes, at: (f64, f64)| {
                let own = Self::size(specification, node);
                let notation = adp_scene::notation::compile::node_notation(specification, &node.type_name());
                let container = &notation["container"];
                let open = !self
                    .view_of(document, node)
                    .and_then(|v| v.get("collapsed").and_then(Value::as_bool))
                    .unwrap_or(false);
                let children = node.children();
                if container.is_null() || !open || children.is_empty() {
                    sizes.insert(node.id(), own);
                    return own;
                }
                let padding = container["padding"].as_f64().unwrap_or(0.0);
                let header = container["header"]["size"].as_f64().unwrap_or(0.0);
                let inner = specification.root()["layout"]["containers"][node.type_name().as_str()]
                    .as_str()
                    .and_then(|name| specification.root()["layout"]["algorithms"].get(name))
                    .map_or(layered, |a| LayeredLayout {
                        node: a["spacing"]["node"].as_f64().unwrap_or(layered.node),
                        layer: a["spacing"]["layer"].as_f64().unwrap_or(layered.layer),
                    });
                let (w, h) = self.layer(
                    specification,
                    document,
                    &children,
                    inner,
                    (at.0 + padding, at.1 + header + padding),
                    positions,
                    sizes,
                );
                let grown = ((w + 2.0 * padding).max(own.0), (h + header + 2.0 * padding).max(own.1));
                sizes.insert(node.id(), grown);
                grown
            };
        let layers = layer.iter().max().map_or(0, |l| l + 1);
        let (mut x, mut bottom) = (origin.0, origin.1);
        for l in 0..layers {
            let members: Vec<usize> = (0..nodes.len()).filter(|i| layer[*i] == l).collect();
            let mut widest: f64 = 0.0;
            let mut y = origin.1;
            for i in members {
                let (width, height) = size_of(&nodes[i], positions, sizes, (x, y));
                // Its centre, as every layout of this host gives it, drawn at the size laid out.
                positions.insert(nodes[i].id(), (x + width / 2.0, y + height / 2.0));
                widest = widest.max(width);
                bottom = bottom.max(y + height);
                y += height + layered.node;
            }
            x += widest + layered.layer;
        }
        ((x - layered.layer - origin.0).max(0.0), bottom - origin.1)
    }

    /// A `layout` action whose capability chooses each node's row (`layout.rowPacking`): the row written as the
    /// attribute the node's placement keeps along y, for every node the capability places whose row changes. A node
    /// the description gives no `left` is not one of them. None where no node is placed on an axis with an attribute.
    fn rowed(
        &self,
        specification: &DislSpecification,
        layout: &dyn crate::capabilities::LayoutCapability,
        input: &Json,
        args: &Json,
        diagram: &adp_disl::DislDiagram,
    ) -> Option<Result<Vec<adp_disl::DislChange>, String>> {
        let mut input = input.clone();
        // Each item is one row tall and in the order the model holds it.
        if let Some(nodes) = input["nodes"].as_array_mut() {
            nodes.retain(|n| n["attributes"].get("left").is_some());
            for (order, node) in nodes.iter_mut().enumerate() {
                node["height"] = json!(1);
                node["order"] = json!(order);
            }
        }
        let placed = match layout.place(&input, args) {
            Ok(placed) => placed,
            Err(reason) => return Some(Err(reason)),
        };
        let mut changes = Vec::new();
        for (id, _, y) in placed {
            let element = diagram.element_by_id(&id)?;
            let along_y = &self.scaled.get(&element.type_name())?.y;
            let row = Value::Int(y.round() as i64);
            if element.value_of(&along_y.attribute).as_number() == row.as_number() {
                continue;
            }
            let type_name = element.type_name();
            let stored = adp_disl::behavior::write::stored_form(specification, &type_name, &along_y.attribute, &row);
            changes.push(adp_disl::DislChange::Set {
                element_id: id,
                type_name,
                attributes: [(along_y.attribute.clone(), stored)].into_iter().collect(),
            });
        }
        Some(Ok(changes))
    }

    /// A `layout` action whose capability orders the nodes rather than placing them (`layout.crossingOrder`, whose
    /// values give each node its `rank` in its column): the order written as moves of the entries, each column's nodes
    /// taking the places that column's nodes held, in their new order, so that a document listing its nodes column by
    /// column, or interleaved, keeps its shape. None for a capability that gives no rank.
    fn ordered(
        &self,
        specification: &DislSpecification,
        capability: &str,
        diagram: &adp_disl::DislDiagram,
    ) -> Option<Result<Vec<adp_disl::DislChange>, String>> {
        let layout = self.capabilities.layout(capability)?;
        let wanted = format!("capability:{capability}");
        let arguments = specification.root()["layout"]["algorithms"]
            .as_object()?
            .values()
            .find(|a| a["algorithm"] == wanted.as_str())?["x-arguments"]
            .as_object()
            .cloned()
            .unwrap_or_default();
        let evaluated = |element: &DislElement| {
            let mut attributes = serde_json::Map::new();
            for (key, value) in arguments.iter().filter(|(_, value)| value.get("cel").is_some()) {
                if let Some(evaluated) = Self::evaluate(specification, element, value)
                    && !evaluated.is_null()
                {
                    attributes.insert(key.clone(), json!(evaluated.to_string()));
                }
            }
            attributes
        };
        let args: serde_json::Map<String, Json> = arguments
            .iter()
            .filter(|(_, value)| value.get("cel").is_none())
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        let roots: Vec<DislElement> = diagram.nodes().into_iter().filter(|n| n.parent().is_none()).collect();
        let nodes: Vec<Json> = roots
            .iter()
            .map(|n| {
                let (width, height) = Self::size(specification, n);
                json!({ "id": n.id(), "width": width, "height": height, "attributes": evaluated(n) })
            })
            .collect();
        let edges: Vec<Json> = diagram
            .relations()
            .into_iter()
            .filter(|r| !r.is_derived())
            .filter_map(|r| {
                Some(json!({
                    "id": r.id(), "source": r.source()?.id(), "target": r.target()?.id(), "attributes": evaluated(&r),
                }))
            })
            .collect();
        let input = json!({ "nodes": nodes, "edges": edges });
        let values = match layout.values(&input, &Json::Object(args.clone())) {
            Ok(values) => values,
            Err(reason) => return Some(Err(reason)),
        };
        let rank: HashMap<String, f64> = values
            .into_iter()
            .filter_map(|(id, named)| Some((id, named.into_iter().find(|(name, _)| name == "rank")?.1)))
            .collect();
        if rank.is_empty() {
            return self.rowed(
                specification,
                layout.as_ref(),
                &input,
                &Json::Object(args.clone()),
                diagram,
            );
        }
        let column: HashMap<String, String> = nodes
            .iter()
            .filter_map(|n| {
                Some((
                    n["id"].as_str()?.to_owned(),
                    n["attributes"]["column"].as_str()?.to_owned(),
                ))
            })
            .collect();
        // The ranked nodes in the document's order, and the order they are to take: each one's place goes to the next
        // node of its column by rank.
        let mut current: Vec<String> = roots
            .iter()
            .map(DislElement::id)
            .filter(|id| rank.contains_key(id))
            .collect();
        let mut queues: HashMap<&str, Vec<&String>> = HashMap::new();
        for id in &current {
            queues.entry(column[id].as_str()).or_default().push(id);
        }
        for queue in queues.values_mut() {
            queue.sort_by(|a, b| rank[*a].total_cmp(&rank[*b]));
            queue.reverse();
        }
        let target: Vec<String> = current
            .iter()
            .map(|id| {
                queues
                    .get_mut(column[id].as_str())
                    .and_then(Vec::pop)
                    .cloned()
                    .unwrap_or_default()
            })
            .collect();
        // Each move puts a node directly before the one that holds its place now, among every node at the top of the
        // document, as the moves before it left them.
        let mut siblings: Vec<String> = roots.iter().map(DislElement::id).collect();
        let mut changes = Vec::new();
        for (index, id) in target.iter().enumerate() {
            if current[index] == *id {
                continue;
            }
            let anchor = current[index].clone();
            let at = siblings.iter().position(|s| *s == anchor)?;
            changes.push(adp_disl::DislChange::Reparent {
                element_id: id.clone(),
                parent_id: None,
                index: at as i64,
            });
            let from = siblings.iter().position(|s| s == id)?;
            let moved = siblings.remove(from);
            let at = siblings.iter().position(|s| *s == anchor)?;
            siblings.insert(at, moved);
            let from = current.iter().position(|s| s == id)?;
            let moved = current.remove(from);
            current.insert(index, moved);
        }
        Some(Ok(changes))
    }

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
        // A point that is the node's centre places the node around it, at its stated size (a Wardley map's component).
        if self
            .scaled
            .get(&element.type_name())
            .is_some_and(|placement| placement.centred)
        {
            return Some(Self::stated_size(element));
        }
        // A node its layout resized is drawn at the size the layout gave it.
        if let Some(size) = self
            .resized
            .borrow()
            .get(document.body_path())
            .and_then(|sizes| sizes.get(&element.id()))
        {
            return Some(*size);
        }
        let specification = document.tool().definition.specification();
        // A node the force layout places is placed by its centre, at the size its notation gives it.
        if self.force.is_some() && !element.is_relation() {
            return Some(Self::size(specification, element));
        }
        // A size bound to the element's values, or fixed (DISL section 6.6).
        let notation = adp_scene::notation::compile::node_notation(specification, &element.type_name());
        let bound = &notation["size"];
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
                // A default size is stated too: a supply chain's empty group is drawn at it, where the layout placed it.
                let stated = ["width", "height", "fixed", "default"]
                    .iter()
                    .any(|key| bound.get(*key).is_some());
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
        // A gesture places a node by its top-left; a node whose point is its centre is placed around it.
        let (x, y) = if placement.centred {
            let (width, height) = Self::stated_size(element);
            (x + width / 2.0, y + height / 2.0)
        } else {
            (x, y)
        };
        let specification = element.diagram().specification();
        // Where a value stands on the canvas: an instant at its seconds, any other value by the axis's scale.
        let canvas = |axis: &ScaledAxis, value: &Value| match value {
            Value::String(text) if axis.instants => seconds_of(text).map_or(0.0, |s| s + axis.offset - axis.origin),
            other => axis.at(element, other.as_number().unwrap_or_default()),
        };
        let (new_x, new_y) = (placement.x.value(element, x), placement.y.value(element, y));
        let mut values = vec![
            (placement.x.attribute.clone(), new_x.clone()),
            (placement.y.attribute.clone(), new_y.clone()),
        ];
        // A node drawn between two points keeps its extent: its second point moves as far as its first. Where its
        // second point along x ends up is its bounds' `x2` (a period's end, which a constraint holds after its begin).
        let mut x2 = (None, None);
        for (on_x, second) in &placement.seconds {
            let (first, moved) = if *on_x {
                (&placement.x, &new_x)
            } else {
                (&placement.y, &new_y)
            };
            if second.attribute == first.attribute {
                continue;
            }
            let was = second.canvas(element);
            let mut now = was;
            if second.instants {
                let delta = canvas(first, moved) - first.canvas(element);
                let written = match element.value_of(&second.attribute) {
                    Value::String(text) => text,
                    _ => String::new(),
                };
                if let Some(end) = seconds_of(&written) {
                    let value = Value::String(instant_text(end + delta, &written));
                    now = canvas(second, &value);
                    values.push((second.attribute.clone(), value));
                }
            } else {
                let old = element.value_of(&first.attribute).as_number().unwrap_or_default();
                let delta = moved.as_number().unwrap_or_default() - old;
                if let Some(end) = element.value_of(&second.attribute).as_number() {
                    let value = if second.time.is_some() || second.whole {
                        Value::Int((end + delta).round() as i64)
                    } else {
                        Value::Double(end + delta)
                    };
                    now = canvas(second, &value);
                    values.push((second.attribute.clone(), value));
                }
            }
            if *on_x {
                x2 = (Some(was), Some(now));
            }
        }
        let bounds = |x: f64, y: f64, x2: Option<f64>| -> Map {
            let mut bounds: Map = [("x", Value::Double(x)), ("y", Value::Double(y))].into_iter().collect();
            if let Some(x2) = x2 {
                bounds.insert("x2", Value::Double(x2));
            }
            bounds
        };
        let new_bounds = bounds(canvas(&placement.x, &new_x), canvas(&placement.y, &new_y), x2.1);
        let type_name = element.type_name();
        // A value the move leaves as it was written is not written again.
        let attributes: Map = values
            .into_iter()
            .filter(|(name, value)| !matches!(value, Value::String(text) if element.value_of(name) == Value::String(text.clone())))
            .map(|(name, value)| {
                let stored = adp_disl::behavior::write::stored_form(&specification, &type_name, &name, &value);
                (name, stored)
            })
            .collect();
        Some(Ok(ModulePlacement {
            attributes,
            old_bounds: bounds(placement.x.canvas(element), placement.y.canvas(element), x2.0),
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
        // An instant on a time axis of instants, as a date; the nearest whole value on a whole axis that has no time
        // unit of its own (a timeline's rows).
        let value = |axis: &ScaledAxis, canvas: f64| {
            if axis.instants {
                return Value::String(instant_text(canvas + axis.origin, ""));
            }
            let value = canvas / axis.units_in(diagram) + axis.origin;
            if axis.whole && axis.time.is_none() && along_x.instants {
                Value::Int(value.round() as i64)
            } else {
                Value::Double(value)
            }
        };
        Some(
            [("x", value(along_x, x)), ("y", value(along_y, y))]
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
            let notation = adp_scene::notation::compile::node_notation(specification, &element.type_name());
            let placement = &notation["placement"];
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
        if let Some(capability) = plugin.strip_prefix("capability:")
            && let Some(ordered) = self.ordered(specification, capability, diagram)
        {
            return Some(ordered);
        }
        let (_, reason) = self.missing.iter().find(|(name, _)| name == plugin)?;
        Some(Err(reason.clone()))
    }

    /// The host's plugin `plugin_id`, else its reader capability of that name (ATD §11.7).
    fn persistence(&self, plugin_id: &str) -> Option<Rc<dyn PersistencePlugin>> {
        (self.plugins)(plugin_id).or_else(|| self.capabilities.reader(plugin_id))
    }

    fn draws_initial_views(&self) -> bool {
        true
    }

    /// A place kept in the body (DISL section 11.6): a move pins where it is dropped, and a node created at a point
    /// is pinned there when a drop pins (DISL section 10.4).
    fn kept_place(
        &self,
        document: &DiagramDocument,
        id: &str,
        x: f64,
        y: f64,
        created: bool,
    ) -> Option<Option<adp_fbl::planning::ModelChange>> {
        let pinned = self.pinned.as_ref()?;
        Some((!created || self.drop_pins).then(|| pinned.pin(document, id, x, y)))
    }

    fn kept_unpin(
        &self,
        document: &DiagramDocument,
        elements: Option<&[String]>,
    ) -> Option<Vec<adp_fbl::planning::ModelChange>> {
        Some(self.pinned.as_ref()?.unpin(document, elements))
    }

    /// A group's state kept in the body (DISL 0.11, section 11.6): its entry, while the state differs from the default.
    fn kept_fold(
        &self,
        document: &DiagramDocument,
        element: &DislElement,
        key: &str,
        folded: bool,
    ) -> Option<Vec<adp_fbl::planning::ModelChange>> {
        let groups = self.groups.as_ref()?;
        let default = element.group_defaults().get(key) == Some(&Value::Bool(true));
        Some(groups.fold(document, &element.id(), key, folded, default))
    }

    /// A kept filter's value in the body (DISL 0.4, section 11.6).
    fn kept_filter(
        &self,
        document: &DiagramDocument,
        filter: &str,
        value: &Value,
    ) -> Option<adp_fbl::planning::ModelChange> {
        self.filters.as_ref()?.set(document, filter, value.clone())
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
        let bound = self.pinned.as_ref().map(|pinned| pinned.positions(document));
        let folds = self.groups.as_ref().map(|groups| groups.states(document));
        if self.initial_view.is_empty() && set.is_none() && !placed && bound.is_none() && folds.is_none() {
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
        if let Some(bound) = &bound
            && !element.is_relation()
        {
            view.insert("pinned".to_owned(), Value::Bool(bound.contains_key(&element.id())));
        }
        // The groups' states the body keeps, which `self.view.groups` reads over their defaults (DISL 0.11).
        if let Some(folds) = &folds
            && !element.is_relation()
        {
            let stored = folds.get(&element.id()).cloned().unwrap_or_default();
            view.insert("groups".to_owned(), Value::map(stored));
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
        if let Some(tiers) = &self.force {
            return Some(self.force_positions(tiers, document));
        }
        if let Some(layered) = &self.layered {
            return Some(self.layered_positions(layered, document));
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
        // A node placed from the layout's own numbers (`layout.value`, ATD §8.8) is the layout's to place too.
        let computed = |element: &DislElement| {
            adp_scene::notation::compile::node_notation(specification, &element.type_name())["placement"]["x"]
                .get("cel")
                .and_then(Json::as_str)
                .is_some_and(|cel| cel.contains("view.bounds"))
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
            } else if capability.exact_sizes() {
                Self::exact_size(specification, &element)
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
            // A node is pinned where the registration places it, else where its own attributes do (DISL section 6.4,
            // `placement.x` and `placement.y` bound to attributes): a supply chain card the body places.
            let stored = element.attributes();
            let placed_by_attributes = document.placement_attributes(&element).and_then(|(x, y)| {
                Some((
                    stored.get(x.as_str())?.as_number()?,
                    stored.get(y.as_str())?.as_number()?,
                ))
            });
            if !boundary && let Some((x, y)) = placed_by_attributes.or_else(|| authored.get(&element.id()).copied()) {
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
                // What the layout's arguments read of the edge, each evaluated for it (a Sankey flow's `weight`).
                let mut attributes = serde_json::Map::new();
                for (key, value) in layout
                    .arguments
                    .iter()
                    .filter(|(key, value)| !queries.contains(&key.as_str()) && value.get("cel").is_some())
                    .filter(|(key, _)| !PLACED_BY_HOST.contains(&key.as_str()))
                {
                    if let Some(evaluated) = Self::evaluate(specification, &relation, value)
                        && !evaluated.is_null()
                    {
                        attributes.insert(key.clone(), json!(evaluated.to_string()));
                    }
                }
                json!({
                    "id": relation.id(),
                    "source": end(relation.source(), relation.source_id()),
                    "target": end(relation.target(), relation.target_id()),
                    "attributes": attributes,
                })
            })
            .collect();
        // The corners as the algorithm computes them, not rounded, so that the centres are where its module put them.
        let input = json!({ "nodes": nodes, "edges": edges });
        let args = Json::Object(args);
        let placed = capability.place(&input, &args).ok()?;
        // A node the layout resizes (a container fitted to its contents) is drawn at that size, its centre in the middle
        // of it.
        let resized: HashMap<String, (f64, f64)> = capability
            .sizes(&input, &args)
            .unwrap_or_default()
            .into_iter()
            .map(|(id, width, height)| (id, (width, height)))
            .collect();
        for (id, size) in &resized {
            sizes.insert(id.clone(), *size);
        }
        // The numbers the layout gives its nodes and edges, which the notation reads as `layout.value(e, name)`.
        let values: HashMap<String, HashMap<String, f64>> = capability
            .values(&input, &args)
            .unwrap_or_default()
            .into_iter()
            .map(|(id, named)| (id, named.into_iter().collect()))
            .collect();
        diagram.set_layout_values(values);
        self.resized
            .borrow_mut()
            .insert(document.body_path().to_path_buf(), resized);
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
