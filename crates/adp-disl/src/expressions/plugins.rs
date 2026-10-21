//! The CEL functions a specification's plugins declare (DISL section 13.1.1; the standalone host's
//! `PluginFunctions` and `DislPluginFunctions`), registered in the environment every expression compiles against
//! before the user functions, so both they and the specification's expressions may call them by their bare names.
//!
//! A host implements the functions of a plugin it has, and a function it does not implement is a function of a
//! plugin that is absent: its `fallback` is evaluated in its place, and without one a call is an evaluation error,
//! so a constraint that calls it reports that it could not be evaluated rather than passing. An implementation
//! receives the call's arguments as CEL values - an element is the model's [`crate::DislElement`], also where a
//! finding's view shows it with the id it is written with - and must not change the model or anything CEL can see.

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::LazyLock;

use adp_cel::{Call, Environment, Function, Style, Value};
use regex::Regex;

use crate::json::{self, Json};
use crate::loading::diagnostic::DislDiagnostic;
use crate::model::written::WrittenElement;

/// A host's implementation of a plugin's CEL function.
pub type PluginFunction = Rc<dyn Fn(&[Value]) -> Result<Value, String>>;

/// The host's implementations of the CEL functions a specification's plugins declare, keyed by function name.
#[derive(Clone, Default)]
pub struct DislPluginFunctions {
    implementations: HashMap<String, PluginFunction>,
}

impl DislPluginFunctions {
    /// No implementations: every plugin is absent.
    pub fn none() -> Self {
        DislPluginFunctions::default()
    }

    /// Implements the plugin function `name`.
    pub fn add(mut self, name: &str, implementation: impl Fn(&[Value]) -> Result<Value, String> + 'static) -> Self {
        assert!(!name.is_empty(), "A plugin function has a name.");
        self.implementations.insert(name.to_owned(), Rc::new(implementation));
        self
    }

    fn get(&self, name: &str) -> Option<&PluginFunction> {
        self.implementations.get(name)
    }
}

pub(crate) static SIMPLE_IDENTIFIER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^[A-Za-z_][A-Za-z0-9_]*$").expect("the identifier pattern compiles"));

pub(crate) fn register(
    environment: &mut Environment,
    root: &Json,
    implementations: &DislPluginFunctions,
    diagnostics: &mut Vec<DislDiagnostic>,
) {
    for (plugin, value) in json::members(root, "plugins") {
        let Some(functions) = json::items(value, "celFunctions") else {
            continue;
        };
        for (index, declaration) in functions.iter().enumerate() {
            let pointer = json::pointer_at(
                &json::pointer(&json::pointer("/plugins", plugin), "celFunctions"),
                index,
            );
            if let Some(function) = function(plugin, declaration, &pointer, environment, implementations, diagnostics) {
                environment.add_function(function);
            }
        }
    }
}

fn function(
    plugin: &str,
    declaration: &Json,
    pointer: &str,
    environment: &Environment,
    implementations: &DislPluginFunctions,
    diagnostics: &mut Vec<DislDiagnostic>,
) -> Option<Function> {
    let name = json::string_of(declaration, "name").unwrap_or("").to_owned();
    if !SIMPLE_IDENTIFIER.is_match(&name) {
        diagnostics.push(DislDiagnostic::error(
            json::pointer(pointer, "name"),
            format!("'{name}' is not a simple identifier, so CEL cannot call it (DISL §13.1.1)."),
        ));
        return None;
    }
    if environment.function(&name, Style::Global).is_some() {
        diagnostics.push(DislDiagnostic::error(
            json::pointer(pointer, "name"),
            format!("'{name}' is already a function of the DISL library or of another plugin (DISL §13.1.1)."),
        ));
        return None;
    }

    // A parameter is a CEL type (the 0.1 form) or {name, type}; only a named one can be read by the fallback.
    let parameters: Vec<Option<String>> = json::items(declaration, "params")
        .map(|params| {
            params
                .iter()
                .map(|p| {
                    if p.is_object() {
                        json::string_of(p, "name").map(str::to_owned)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    let arity = parameters.len();
    let cost = json::int_of(declaration, "cost").filter(|c| *c > 0).unwrap_or(1);
    let uses = json::strings_of(declaration, "uses");

    if let Some(implementation) = implementations.get(&name) {
        let implementation = Rc::clone(implementation);
        return Some(
            Function::new(
                &name,
                Style::Global,
                arity,
                arity,
                Some(Rc::new(move |call: &Call<'_>| {
                    let arguments: Vec<Value> = call.arguments.iter().map(unwrapped).collect();
                    implementation(&arguments)
                })),
            )
            .with_cost(move |_| cost),
        );
    }

    let Some(fallback) = json::string_of(declaration, "fallback") else {
        let absent = format!(
            "'{name}' is a function of the plugin '{plugin}', which this host does not have, and it declares no fallback (DISL §13.1.1)."
        );
        return Some(
            Function::new(
                &name,
                Style::Global,
                arity,
                arity,
                Some(Rc::new(move |_: &Call<'_>| Err(absent.clone()))),
            )
            .with_cost(move |_| cost),
        );
    };

    if parameters.iter().any(Option::is_none) {
        diagnostics.push(DislDiagnostic::error(
            json::pointer(pointer, "params"),
            format!(
                "The fallback of '{name}' reads its parameters by name, so each is declared as {{name, type}} (DISL §13.1.1)."
            ),
        ));
        return None;
    }
    let parameters: Vec<String> = parameters.into_iter().flatten().collect();

    let mut scope = environment.clone();
    scope.declare_variables(parameters.iter().map(String::as_str));
    scope.declare_variables(uses.iter().map(String::as_str));
    let program = match scope.compile(fallback) {
        Ok(program) => program,
        Err(e) => {
            diagnostics.push(DislDiagnostic::error(json::pointer(pointer, "fallback"), e));
            return None;
        }
    };

    let function_name = name.clone();
    Some(
        Function::new(
            &name,
            Style::Global,
            arity,
            arity,
            Some(Rc::new(move |call: &Call<'_>| {
                let mut variables: HashMap<String, Value> = HashMap::new();
                for (index, parameter) in parameters.iter().enumerate() {
                    variables.insert(parameter.clone(), call[index].clone());
                }
                for used in &uses {
                    let value = call.variables.get(used).ok_or_else(|| {
                        format!("'{function_name}' uses '{used}', which the expression calling it has no value for.")
                    })?;
                    variables.insert(used.clone(), value);
                }
                program.evaluate_within(&variables, call.budget)
            })),
        )
        .with_cost(move |_| cost),
    )
}

/// An element as the model holds it, also when a finding shows it under its written id.
fn unwrapped(argument: &Value) -> Value {
    match argument.downcast::<WrittenElement>() {
        Some(written) => written.element().to_value(),
        None => argument.clone(),
    }
}
