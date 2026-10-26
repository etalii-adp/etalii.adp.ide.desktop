//! Lowering a checked description onto the engines (etalii.adp spec 035, T126, research R16): a diagram's description
//! as an in-memory DISL specification that `adp-disl` loads with the description's script as its extension, so that
//! the engines that drive today's tools drive the description too. No code here knows a tool type (FR-040).

pub mod actions;
pub mod disl;
pub mod expressions;
pub mod fbl;
pub mod script;
pub mod values;

use std::cell::RefCell;
use std::rc::Rc;

use adp_atd_check::{Description, Symbols};
use adp_atd_script::{Options, STEPS, Script};
use adp_cel::{Call, Function, Style, Value};
use adp_disl::{DislDiagnostic, DislElement, DislExtensions, DislLoader, DislPluginFunctions, DislSpecification};
use serde_json::Value as Json;

pub use self::disl::LoweredDiagram;
use self::script::{ScriptActions, ScriptCell};

/// A diagram's description, lowered and loaded.
pub struct LoadedDiagram {
    pub specification: DislSpecification,
    /// The specification as lowered, for what DISL's view of it does not show.
    pub json: Json,
    pub script: Script,
}

/// The DISL specification of a diagram's description.
pub fn lower_diagram(description: &Description) -> Result<LoweredDiagram, String> {
    let tool = description.tool().ok_or("The description has no tool construct.")?;
    if description.kind() != Some("diagram") {
        return Err(format!(
            "A `{}` is not lowered onto DISL.",
            description.kind().unwrap_or("description")
        ));
    }
    let members: Vec<&Json> = description.members().collect();
    let mut lowered = disl::lower(tool, &members);
    expressions::as_disl_writes(&mut lowered.specification);
    Ok(lowered)
}

/// Lowers a diagram's description and loads it into `adp-disl`, its functions and `extension` actions the script's;
/// `plugins` are the host's implementations of the plugins it declares.
pub fn load_diagram(
    description: &Description,
    symbols: &Symbols,
    plugins: &DislPluginFunctions,
) -> Result<LoadedDiagram, Vec<DislDiagnostic>> {
    let lowered = lower_diagram(description).map_err(|e| vec![DislDiagnostic::error("", e)])?;
    let options = Options {
        steps: steps(&lowered.specification),
    };
    let cell: ScriptCell = Rc::new(RefCell::new(None));
    let functions = symbols.functions.clone();
    let made = Rc::clone(&cell);
    let parameters = lowered
        .specification
        .pointer("/behavior/operations")
        .and_then(Json::as_object)
        .map(|operations| {
            operations
                .iter()
                .map(|(name, operation)| {
                    let params = operation["params"].as_object();
                    (
                        name.clone(),
                        params.map(|p| p.keys().cloned().collect()).unwrap_or_default(),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    let extensions = DislExtensions {
        environment: Some(Box::new(move |mut environment| {
            environment.add_function(host_attribute());
            environment.add_function(end_id());
            let script = Script::with_functions(functions, environment, options);
            let extended = script.environment().clone();
            *made.borrow_mut() = Some(script);
            extended
        })),
        actions: Some(Rc::new(ScriptActions {
            script: Rc::clone(&cell),
            scripts: lowered.scripts.clone(),
            parameters,
        })),
    };
    let bytes = serde_json::to_vec(&lowered.specification).expect("JSON writes");
    let result = DislLoader::load_extended(&bytes, plugins, extensions);
    let Some(specification) = result.specification else {
        return Err(result.diagnostics);
    };
    let script = cell.borrow().clone().expect("the environment was made");
    Ok(LoadedDiagram {
        specification,
        json: lowered.specification,
        script,
    })
}

/// What a call of `hostAttribute` that is not over an element and a name answers.
pub const HOST_ATTRIBUTE_MISUSED: &str =
    "hostAttribute() takes an element of the document and the name of a host attribute.";

/// `hostAttribute(element, name)` (ATD §12.8): the value the binding read into the host attribute `name` of
/// `element` (persistence's `hostAttributes`, DISL §11.2), as it read it; null when it read none.
fn host_attribute() -> Function {
    let body = |call: &Call<'_>| -> Result<Value, String> {
        let (Some(element), Some(name)) = (
            call.arguments.first().and_then(DislElement::from_value),
            call.arguments.get(1).and_then(Value::as_str),
        ) else {
            return Err(HOST_ATTRIBUTE_MISUSED.to_owned());
        };
        Ok(element.host_attributes().get(name).cloned().unwrap_or(Value::Null))
    };
    Function::new("hostAttribute", Style::Global, 2, 2, Some(Rc::new(body)))
}

/// What a call of `endId` that is not over a relation and one of its two ends answers.
pub const END_ID_MISUSED: &str = "endId() takes a relation of the document and 'source' or 'target'.";

/// `endId(relation, end)` (ATD §12.8): the id `relation`'s `end` (`source` or `target`) was read or created with,
/// also when it reaches no element, else the id of the element it reaches; null when it has neither.
fn end_id() -> Function {
    let body = |call: &Call<'_>| -> Result<Value, String> {
        let (Some(relation), Some(end)) = (
            call.arguments.first().and_then(DislElement::from_value),
            call.arguments.get(1).and_then(Value::as_str),
        ) else {
            return Err(END_ID_MISUSED.to_owned());
        };
        if !relation.is_relation() {
            return Err(END_ID_MISUSED.to_owned());
        }
        let (written, reached) = match end {
            "source" => (relation.source_id(), relation.source()),
            "target" => (relation.target_id(), relation.target()),
            _ => return Err(END_ID_MISUSED.to_owned()),
        };
        Ok(written
            .or_else(|| reached.map(|e| e.id()))
            .map_or(Value::Null, Value::String))
    };
    Function::new("endId", Style::Global, 2, 2, Some(Rc::new(body)))
}

/// The script's budget: the description's `limits.steps` when lower than the default (ATD §7.15).
fn steps(specification: &Json) -> u64 {
    specification
        .pointer("/language/limits/steps")
        .and_then(Json::as_u64)
        .map_or(STEPS, |steps| steps.min(STEPS))
}
