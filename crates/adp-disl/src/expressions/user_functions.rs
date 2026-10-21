//! A specification's own functions (DISL section 3.4; the standalone host's `UserFunctions`), compiled into its CEL
//! environment in declaration order, so each may call only those declared before it.
//!
//! Order is the termination guarantee: a function body is compiled in an environment that holds the functions
//! declared before it and, when it declares `recursion`, itself, so a call into a later function, or into itself
//! without `recursion`, fails to compile, and indirect recursion cannot be written at all. A function sees its
//! parameters, and `diagram` and `env` only when it names them in `uses`, taken from the evaluation that calls it.
//! Bounded recursion counts the nested calls of one function within one evaluation, the top-level call being depth
//! 1; a call that would go deeper than `maxDepth` evaluates `atMaxDepth` over its own parameters instead of the body.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::LazyLock;

use adp_cel::{Call, Environment, Function, Program, Style, Value};
use regex::Regex;

use super::plugins::SIMPLE_IDENTIFIER;
use crate::json;
use crate::loading::diagnostic::DislDiagnostic;
use crate::loading::metamodel::DislFunctionDeclaration;

thread_local! {
    /// The depth of each recursive function within one evaluation, keyed by the evaluation's budget.
    static DEPTHS: RefCell<HashMap<(usize, String), i64>> = RefCell::new(HashMap::new());
}

static REFUSED_CALL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^The function '([^']+)' is not supported").expect("the refusal pattern compiles"));

/// One user function's programs, filled in once they compile.
#[derive(Default)]
struct Programs {
    body: Option<Program>,
    at_max_depth: Option<Program>,
}

/// Adds every function of `declarations` to `environment`, in order, and reports each one that does not compile; a
/// function that does not compile is not added.
pub(crate) fn compile(
    environment: &mut Environment,
    declarations: &[DislFunctionDeclaration],
    diagnostics: &mut Vec<DislDiagnostic>,
) {
    let declared: HashSet<&str> = declarations.iter().map(|d| d.name.as_str()).collect();
    for declaration in declarations {
        let pointer = json::pointer("/functions", &declaration.name);
        if !valid(declaration, &pointer, environment, diagnostics) {
            continue;
        }

        let programs = Rc::new(RefCell::new(Programs::default()));
        let function = function(declaration, Rc::clone(&programs));
        let mut scope = environment.clone();
        scope.declare_variables(declaration.parameters.iter().map(|p| p.name.as_str()));
        scope.declare_variables(declaration.uses.iter().map(String::as_str));
        if declaration.recursion.is_some() {
            scope.add_function(function.clone());
        }

        let compiled = scope.compile(&declaration.cel).and_then(|body| {
            let at_max_depth = match &declaration.recursion {
                Some(recursion) => Some(scope.compile(&recursion.at_max_depth)?),
                None => None,
            };
            Ok((body, at_max_depth))
        });
        match compiled {
            Ok((body, at_max_depth)) => {
                let mut p = programs.borrow_mut();
                p.body = Some(body);
                p.at_max_depth = at_max_depth;
            }
            Err(e) => {
                diagnostics.push(DislDiagnostic::error(
                    json::pointer(&pointer, "cel"),
                    explain(declaration, &e, &declared),
                ));
                continue;
            }
        }
        environment.add_function(function);
    }
}

fn valid(
    declaration: &DislFunctionDeclaration,
    pointer: &str,
    environment: &Environment,
    diagnostics: &mut Vec<DislDiagnostic>,
) -> bool {
    let mut valid = true;
    if !SIMPLE_IDENTIFIER.is_match(&declaration.name) {
        diagnostics.push(DislDiagnostic::error(
            pointer,
            format!(
                "'{}' is not a simple identifier, so CEL cannot call it.",
                declaration.name
            ),
        ));
        valid = false;
    }
    if environment.function(&declaration.name, Style::Global).is_some() {
        diagnostics.push(DislDiagnostic::error(
            pointer,
            format!(
                "'{}' is already a function of the DISL library or of this specification.",
                declaration.name
            ),
        ));
        valid = false;
    }
    for parameter in declaration
        .parameters
        .iter()
        .filter(|p| !SIMPLE_IDENTIFIER.is_match(&p.name))
    {
        diagnostics.push(DislDiagnostic::error(
            json::pointer(pointer, "params"),
            format!("The parameter '{}' is not a simple identifier.", parameter.name),
        ));
        valid = false;
    }
    for used in declaration
        .uses
        .iter()
        .filter(|u| !matches!(u.as_str(), "diagram" | "env"))
    {
        diagnostics.push(DislDiagnostic::error(
            json::pointer(pointer, "uses"),
            format!("A function may use 'diagram' and 'env', not '{used}'."),
        ));
        valid = false;
    }
    if declaration.recursion.as_ref().is_some_and(|r| r.max_depth < 1) {
        diagnostics.push(DislDiagnostic::error(
            json::pointer(pointer, "recursion"),
            "recursion.maxDepth must be at least 1.",
        ));
        valid = false;
    }
    valid
}

/// The compile error, with why a call to a function of this specification is refused when that is the cause.
fn explain(declaration: &DislFunctionDeclaration, message: &str, declared: &HashSet<&str>) -> String {
    let Some(refused) = REFUSED_CALL.captures(message) else {
        return message.to_owned();
    };
    let name = &refused[1];
    if name == declaration.name {
        return format!("'{name}' calls itself, which a function may do only when it declares recursion (DISL §3.4).");
    }
    if declared.contains(name) {
        format!(
            "'{}' calls '{name}', which is declared after it or does not compile; a function may call only the functions declared before it (DISL §3.4).",
            declaration.name
        )
    } else {
        message.to_owned()
    }
}

/// The CEL function the environment offers for one user function.
fn function(declaration: &DislFunctionDeclaration, programs: Rc<RefCell<Programs>>) -> Function {
    let declaration = declaration.clone();
    let arity = declaration.parameters.len();
    let name = declaration.name.clone();
    Function::new(
        &name,
        Style::Global,
        arity,
        arity,
        Some(Rc::new(move |call: &Call<'_>| {
            let mut variables: HashMap<String, Value> = HashMap::new();
            for (index, parameter) in declaration.parameters.iter().enumerate() {
                variables.insert(parameter.name.clone(), call[index].clone());
            }
            for used in &declaration.uses {
                let value = call.variables.get(used).ok_or_else(|| {
                    format!(
                        "'{}' uses '{used}', which the expression calling it has no value for.",
                        declaration.name
                    )
                })?;
                variables.insert(used.clone(), value);
            }
            let (body, at_max_depth) = {
                let p = programs.borrow();
                (p.body.clone(), p.at_max_depth.clone())
            };
            let Some(body) = body else {
                return Err(format!("'{}' did not compile.", declaration.name));
            };
            let Some(recursion) = &declaration.recursion else {
                return body.evaluate_within(&variables, call.budget);
            };

            let key = (std::ptr::from_ref(call.budget) as usize, declaration.name.clone());
            let depth = DEPTHS.with(|d| d.borrow().get(&key).copied().unwrap_or(0)) + 1;
            if depth > recursion.max_depth {
                return match at_max_depth {
                    Some(program) => program.evaluate_within(&variables, call.budget),
                    None => Ok(Value::Null),
                };
            }
            DEPTHS.with(|d| d.borrow_mut().insert(key.clone(), depth));
            let result = body.evaluate_within(&variables, call.budget);
            DEPTHS.with(|d| {
                let mut depths = d.borrow_mut();
                if depth - 1 == 0 {
                    depths.remove(&key);
                } else {
                    depths.insert(key, depth - 1);
                }
            });
            result
        })),
    )
}
