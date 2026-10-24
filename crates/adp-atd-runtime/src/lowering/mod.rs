//! Lowering a checked description onto the engines (etalii.adp spec 035, T126, research R16): a diagram's description
//! as an in-memory DISL specification that `adp-disl` loads with the description's script as its extension, so that
//! the engines that drive today's tools drive the description too. No code here knows a tool type (FR-040).

pub mod actions;
pub mod disl;
pub mod fbl;
pub mod script;
pub mod values;

use std::cell::RefCell;
use std::rc::Rc;

use adp_atd_check::{Description, Symbols};
use adp_atd_script::{Options, STEPS, Script};
use adp_disl::{DislDiagnostic, DislExtensions, DislLoader, DislPluginFunctions, DislSpecification};
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
    Ok(disl::lower(tool, &members))
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
        environment: Some(Box::new(move |environment| {
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

/// The script's budget: the description's `limits.steps` when lower than the default (ATD §7.15).
fn steps(specification: &Json) -> u64 {
    specification
        .pointer("/language/limits/steps")
        .and_then(Json::as_u64)
        .map_or(STEPS, |steps| steps.min(STEPS))
}
