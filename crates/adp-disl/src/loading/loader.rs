//! Loads a DISL specification as DISL section 14.1 says, in its order (the standalone host's `DislLoader`): **1**
//! parse, rejecting a byte-order mark and duplicate keys; **2** check the version; **5** resolve names; **6** flatten
//! inheritance; **8** compile every expression in its context, the plugins' functions first and the user functions in
//! declaration order; and the semantic checks of step 9 those steps rest on (reserved attribute names, user function
//! rules).
//!
//! Not done here, and not claimed: imports (step 3; a specification that declares any is refused), JSON Schema
//! validation (step 4), generated built-ins (step 7), static type-checking and cost estimation (step 8; CEL here is
//! checked for names and arity, then evaluated dynamically under the cost limit), and the notation-, coordinate- and
//! form-specific rules of step 9. Name resolution covers the references every derivation reads; styles, shapes,
//! markers and icons are not resolved.

use super::diagnostic::{DislDiagnostic, DislSeverity};
use super::metamodel::{DislExpression, DislFunctionDeclaration, DislParameter, DislRecursion};
use super::specification::DislSpecification;
use super::{inheritance, names, walker};
use crate::behavior::extension::DislExtensions;
use crate::expressions::plugins::{self, DislPluginFunctions};
use crate::expressions::{contexts, library, text_metric, user_functions};
use crate::json::{self, Json};

/// The DISL major version this runtime reads, and the highest minor it knows.
const MAJOR: i64 = 0;
const MINOR: i64 = 3;

/// The default per-evaluation cost limit of section 2.5.
const DEFAULT_COST_LIMIT: i64 = 1_000_000;

/// Attribute names that are built-in members of elements in CEL (section 2.2).
const RESERVED: &[&str] = &[
    "id",
    "type",
    "kind",
    "parent",
    "children",
    "descendants",
    "ancestors",
    "incoming",
    "outgoing",
    "source",
    "target",
    "sourcePort",
    "targetPort",
    "ports",
    "owner",
    "view",
    "diagram",
    "self",
    "value",
    "item",
    "index",
    "env",
    "old",
    "event",
    "detail",
    "in",
    "as",
    "break",
    "const",
    "continue",
    "else",
    "for",
    "function",
    "if",
    "import",
    "let",
    "loop",
    "package",
    "namespace",
    "return",
    "var",
    "void",
    "while",
    "true",
    "false",
    "null",
];

/// Whether `name` may not be an attribute name (section 2.2).
pub(crate) fn is_reserved(name: &str) -> bool {
    RESERVED.contains(&name) || name.starts_with('_') || name.starts_with('$')
}

/// What loading a specification gives: the specification, when it has no errors, and every diagnostic.
#[derive(Debug, Clone)]
pub struct DislLoadResult {
    pub specification: Option<DislSpecification>,
    pub diagnostics: Vec<DislDiagnostic>,
}

impl DislLoadResult {
    pub fn errors(&self) -> impl Iterator<Item = &DislDiagnostic> {
        self.diagnostics.iter().filter(|d| d.severity == DislSeverity::Error)
    }
}

/// The loader of DISL specifications.
pub struct DislLoader;

fn has_errors(diagnostics: &[DislDiagnostic]) -> bool {
    diagnostics.iter().any(|d| d.severity == DislSeverity::Error)
}

impl DislLoader {
    /// Loads the specification `json`.
    pub fn load(json: &str) -> DislLoadResult {
        DislLoader::load_bytes(json.as_bytes(), &DislPluginFunctions::none())
    }

    /// Loads a specification whose plugins' CEL functions (section 13.1.1) the host implements in `plugins`; a
    /// declared function it does not implement is evaluated through its `fallback`, or is an evaluation error
    /// without one.
    pub fn load_bytes(utf8: &[u8], plugins: &DislPluginFunctions) -> DislLoadResult {
        DislLoader::load_extended(utf8, plugins, DislExtensions::default())
    }

    /// Loads a specification as [`DislLoader::load_bytes`] does, with what a runtime that lowers another language
    /// onto DISL adds: functions in its environment and the `extension` actions it runs.
    pub fn load_extended(utf8: &[u8], plugins: &DislPluginFunctions, extensions: DislExtensions) -> DislLoadResult {
        let mut diagnostics = Vec::new();
        let refused = |diagnostics| DislLoadResult {
            specification: None,
            diagnostics,
        };

        // Step 1: parse, rejecting a byte-order mark and duplicate keys (section 2.1).
        if utf8.starts_with(&[0xEF, 0xBB, 0xBF]) {
            diagnostics.push(DislDiagnostic::error(
                "",
                "A specification is UTF-8 without a byte-order mark (DISL §2.1).",
            ));
            return refused(diagnostics);
        }
        let (root, duplicates) = match json::parse(utf8) {
            Ok(parsed) => parsed,
            Err(e) => {
                diagnostics.push(DislDiagnostic::error("", format!("The specification is not JSON: {e}")));
                return refused(diagnostics);
            }
        };
        if !root.is_object() {
            diagnostics.push(DislDiagnostic::error(
                "",
                "A specification is a JSON object (DISL §2.1).",
            ));
            return refused(diagnostics);
        }
        for (pointer, key) in duplicates {
            diagnostics.push(DislDiagnostic::error(
                pointer,
                format!("The key '{key}' appears more than once in one object (DISL §2.1)."),
            ));
        }

        // Step 2: the version (section 2.9).
        version(&root, &mut diagnostics);
        match json::get(&root, "language").filter(|l| l.is_object()) {
            None => diagnostics.push(DislDiagnostic::error(
                "/language",
                "A specification declares its language (DISL §3.1).",
            )),
            Some(language) => {
                if json::string_of(language, "id").is_none() {
                    diagnostics.push(DislDiagnostic::error(
                        "/language/id",
                        "The language has no id (DISL §3.2).",
                    ));
                }
                if json::string_of(language, "version").is_none() {
                    diagnostics.push(DislDiagnostic::error(
                        "/language/version",
                        "The language has no version (DISL §3.2).",
                    ));
                }
            }
        }
        let metamodel_json = json::get(&root, "metamodel").filter(|m| m.is_object()).cloned();
        if metamodel_json.is_none() {
            diagnostics.push(DislDiagnostic::error(
                "/metamodel",
                "A specification declares its metamodel (DISL §3.1).",
            ));
        }
        if json::items(&root, "imports").is_some_and(|i| !i.is_empty()) {
            diagnostics.push(DislDiagnostic::error(
                "/imports",
                "This runtime does not resolve imports (DISL §3.3, step 3 of §14.1).",
            ));
        }
        if has_errors(&diagnostics) {
            return refused(diagnostics);
        }
        let metamodel_json = metamodel_json.unwrap_or_default();

        // Steps 5 and 6: the metamodel, names resolved and inheritance flattened (section 4.7), and the other references.
        let metamodel = inheritance::build(&metamodel_json, &mut diagnostics);
        names::resolve(&root, &metamodel, &mut diagnostics);

        // Step 8: the plugins' functions, the user functions in declaration order, then every expression in its context.
        let functions = functions(&root, &mut diagnostics);
        let mut environment = adp_cel::Environment::standard_library();
        environment.budget = cost_limit(&root);
        library::register(&mut environment, metamodel.enums.clone());
        text_metric::register(&mut environment, &root);
        plugins::register(&mut environment, &root, plugins, &mut diagnostics);
        if let Some(extend) = extensions.environment {
            environment = extend(environment);
        }
        user_functions::compile(&mut environment, &functions, &mut diagnostics);
        let expressions = compile(&root, &environment, &mut diagnostics);

        if has_errors(&diagnostics) {
            return refused(diagnostics);
        }
        DislLoadResult {
            specification: Some(DislSpecification::new(
                root,
                metamodel,
                functions,
                environment,
                expressions,
                extensions.actions,
            )),
            diagnostics,
        }
    }
}

/// The `disl` version, or its deprecated alias `dedl` (sections 2.9 and 18): a higher major is refused, a higher
/// minor warned about.
fn version(root: &Json, diagnostics: &mut Vec<DislDiagnostic>) {
    let disl = json::string_of(root, "disl");
    let dedl = json::string_of(root, "dedl");
    if disl.is_some() && dedl.is_some() {
        diagnostics.push(DislDiagnostic::error(
            "/dedl",
            "A specification carries exactly one of 'disl' and its deprecated alias 'dedl' (DISL §18).",
        ));
        return;
    }
    let pointer = if disl.is_none() { "/dedl" } else { "/disl" };
    let Some(version) = disl.or(dedl) else {
        diagnostics.push(DislDiagnostic::error(
            "/disl",
            "A specification declares the DISL version it targets (DISL §2.9).",
        ));
        return;
    };
    let parts: Vec<&str> = version.split('.').collect();
    let number = |p: &str| -> Option<i64> {
        if p.is_empty() || !p.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        p.parse().ok()
    };
    let (Some(major), Some(minor)) = (
        parts.first().and_then(|p| number(p)),
        parts.get(1).and_then(|p| number(p)),
    ) else {
        diagnostics.push(DislDiagnostic::error(
            pointer,
            format!("'{version}' is not a DISL version written as major.minor (DISL §2.9)."),
        ));
        return;
    };
    if parts.len() != 2 {
        diagnostics.push(DislDiagnostic::error(
            pointer,
            format!("'{version}' is not a DISL version written as major.minor (DISL §2.9)."),
        ));
        return;
    }
    if major > MAJOR {
        diagnostics.push(DislDiagnostic::error(
            pointer,
            format!("This runtime reads DISL {MAJOR}.x; DISL {version} is refused (DISL §2.9)."),
        ));
    } else if major == MAJOR && minor > MINOR {
        diagnostics.push(DislDiagnostic::warning(
            pointer,
            format!("This runtime knows DISL up to {MAJOR}.{MINOR}; what DISL {version} adds is not read (DISL §2.9)."),
        ));
    }
    if dedl.is_some() {
        diagnostics.push(DislDiagnostic::warning(
            pointer,
            "'dedl' is the deprecated alias of 'disl' (DISL §18).",
        ));
    }
}

fn cost_limit(root: &Json) -> i64 {
    let limits = json::member(json::member(root, "language"), "limits");
    json::int_of(limits, "celCost")
        .filter(|v| *v > 0)
        .unwrap_or(DEFAULT_COST_LIMIT)
}

fn functions(root: &Json, diagnostics: &mut Vec<DislDiagnostic>) -> Vec<DislFunctionDeclaration> {
    let mut functions = Vec::new();
    for (name, value) in json::members(root, "functions") {
        let pointer = json::pointer("/functions", name);
        let Some(cel) = json::string_of(value, "cel") else {
            diagnostics.push(DislDiagnostic::error(
                pointer,
                format!("The function '{name}' has no cel body (DISL §3.4)."),
            ));
            continue;
        };
        let parameters = json::items(value, "params")
            .map(|params| {
                params
                    .iter()
                    .map(|p| DislParameter {
                        name: json::string_of(p, "name").unwrap_or("").to_owned(),
                        type_name: json::string_of(p, "type").unwrap_or("dyn").to_owned(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let recursion = json::get(value, "recursion")
            .filter(|r| r.is_object())
            .map(|r| DislRecursion {
                max_depth: json::get(r, "maxDepth")
                    .and_then(Json::as_i64)
                    .filter(|d| i32::try_from(*d).is_ok())
                    .unwrap_or(0),
                at_max_depth: json::string_of(r, "atMaxDepth").unwrap_or("null").to_owned(),
            });
        functions.push(DislFunctionDeclaration {
            name: name.clone(),
            parameters,
            returns: json::string_of(value, "returns").unwrap_or("dyn").to_owned(),
            cel: cel.to_owned(),
            uses: json::strings_of(value, "uses"),
            recursion,
        });
    }
    functions
}

/// Compiles every expression the walker finds, each in its context with the names bound around it.
fn compile(
    root: &Json,
    functions: &adp_cel::Environment,
    diagnostics: &mut Vec<DislDiagnostic>,
) -> Vec<DislExpression> {
    let mut everything: Vec<&str> = Vec::new();
    for context in contexts::all() {
        for variable in contexts::variables_of(context) {
            if !everything.contains(variable) {
                everything.push(variable);
            }
        }
    }
    let mut expressions = Vec::new();
    for site in walker::sites(root) {
        if site.context.is_empty() {
            diagnostics.push(DislDiagnostic::warning(
                site.pointer.clone(),
                "This runtime does not know the CEL context of this position; it was compiled with every variable of any context.",
            ));
        }
        let mut environment = functions.clone();
        if site.context.is_empty() {
            environment.declare_variables(everything.iter().copied());
        } else {
            environment.declare_variables(contexts::variables_of(&site.context).iter().copied());
        }
        environment.declare_variables(site.bindings.iter().map(String::as_str));
        match environment.compile(&site.source) {
            Ok(program) => expressions.push(DislExpression {
                pointer: site.pointer,
                context: site.context,
                program,
            }),
            Err(e) => diagnostics.push(DislDiagnostic::error(
                site.pointer,
                format!(
                    "{e} (context {})",
                    if site.context.is_empty() {
                        "unknown"
                    } else {
                        &site.context
                    }
                ),
            )),
        }
    }
    expressions
}
