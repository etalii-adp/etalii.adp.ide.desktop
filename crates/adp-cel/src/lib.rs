//! The CEL facade (spec 030, research R7): the standalone host's CEL engine (`EtAlii.Adp.Specification.Cel` at
//! 8b6ccfc), ported to Rust. An expression is compiled against an environment, which names the variables it may
//! read, the functions it may call (global and on a receiver, each with its arity and cost) and the comprehension
//! macros, and evaluated against the variables' values under a budget; anything the environment does not offer is
//! refused at compile time, naming it. FBL section 2.4 shares the standard environment with DISL; section 2.5's
//! regular expression subset lives here too, because `matches()` uses it. A language builds its contexts on the
//! standard environment: FBL's in `adp-fbl`, DISL's in `adp-disl`, which adds its function library and the host
//! objects its elements are ([`Object`]).

mod eval;
mod function;
mod library;
mod parser;
pub mod regex_subset;
mod value;

use std::collections::HashMap;
use std::rc::Rc;

pub use eval::{Program, Variables};
pub use function::{Body, Budget, Call, Cost, Function, Macro, Style};
pub use regex_subset::BoundedRegex;
pub use value::{Map, Object, Value};

/// The steps one evaluation may take unless an environment says otherwise.
const DEFAULT_BUDGET: i64 = 100_000;

/// What an expression may use: the variables it may read, the functions it may call, the comprehension macros, and
/// the budget of one evaluation.
#[derive(Debug, Clone)]
pub struct Environment {
    variables: Vec<String>,
    functions: HashMap<(String, Style), Rc<Function>>,
    macros: Vec<Macro>,
    /// Whether an identifier that names no declared variable compiles anyway, failing only when it has no value at
    /// evaluation. For tools that parse expressions whose context they do not know.
    pub allows_undeclared_variables: bool,
    /// The cost one evaluation may spend.
    pub budget: i64,
}

impl Default for Environment {
    fn default() -> Self {
        Environment::empty()
    }
}

impl Environment {
    /// An environment with nothing in it: no variables, functions or macros.
    pub fn empty() -> Self {
        Environment {
            variables: Vec::new(),
            functions: HashMap::new(),
            macros: Vec::new(),
            allows_undeclared_variables: false,
            budget: DEFAULT_BUDGET,
        }
    }

    /// The CEL standard environment: the core functions and conversions, the strings, math, lists and optional
    /// libraries, and the macros `all`, `exists`, `exists_one`, `filter`, `map` and `sortBy`. No variables.
    pub fn standard_library() -> Self {
        let mut environment = Environment::empty();
        library::register(&mut environment);
        environment
    }

    /// The CEL standard environment with the variables `names`.
    pub fn standard(names: &[&str]) -> Self {
        let mut environment = Environment::standard_library();
        environment.declare_variables(names.iter().copied());
        environment
    }

    pub fn variables(&self) -> &[String] {
        &self.variables
    }

    pub fn is_variable(&self, name: &str) -> bool {
        self.variables.iter().any(|v| v == name)
    }

    pub fn declare_variable(&mut self, name: &str) -> &mut Self {
        if !self.is_variable(name) {
            self.variables.push(name.to_owned());
        }
        self
    }

    pub fn declare_variables<'a>(&mut self, names: impl IntoIterator<Item = &'a str>) -> &mut Self {
        for name in names {
            self.declare_variable(name);
        }
        self
    }

    /// Adds `function`, replacing one of the same name and call style.
    pub fn add_function(&mut self, function: Function) -> &mut Self {
        self.functions
            .insert((function.name.clone(), function.style), Rc::new(function));
        self
    }

    /// Adds the comprehension macro `kind`.
    pub fn add_macro(&mut self, kind: Macro) -> &mut Self {
        if !self.macros.contains(&kind) {
            self.macros.push(kind);
        }
        self
    }

    /// The function `name` called in `style`, if the environment offers it.
    pub fn function(&self, name: &str, style: Style) -> Option<&Rc<Function>> {
        self.functions.get(&(name.to_owned(), style))
    }

    /// The macro `name`, if the environment offers it.
    pub fn macro_named(&self, name: &str) -> Option<Macro> {
        self.macros.iter().copied().find(|m| m.name() == name)
    }

    /// Compiles `expression`; the error names the first thing it cannot use.
    pub fn compile(&self, expression: &str) -> Result<Program, String> {
        let node = parser::parse(expression, self)?;
        if !self.allows_undeclared_variables {
            eval::check_variables(&node, self, &mut Vec::new())?;
        }
        Ok(Program::new(node, self.budget))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn eval(expression: &str, variables: &[(&str, Value)]) -> Result<Value, String> {
        let names: Vec<&str> = variables.iter().map(|(n, _)| *n).collect();
        let program = Environment::standard(&names).compile(expression)?;
        let map: HashMap<String, Value> = variables.iter().map(|(n, v)| ((*n).to_owned(), v.clone())).collect();
        program.evaluate(&map)
    }

    fn entry() -> Value {
        let mut map = Map::default();
        map.insert("id", Value::from("a"));
        map.insert("name", Value::from("x"));
        map.insert("view", Value::Map(std::rc::Rc::new(Map::default())));
        Value::Map(std::rc::Rc::new(map))
    }

    #[test]
    fn the_bindings_expressions_evaluate() {
        let e = [("entry", entry())];
        assert_eq!(
            eval("has(entry.name) || has(entry.libraries)", &e).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(eval("entry.all(k, true)", &e).unwrap(), Value::Bool(true));
        assert_eq!(
            eval("entry.filter(k, !(k in ['id', 'view']))", &e).unwrap(),
            Value::list(vec![Value::from("name")])
        );
        assert_eq!(
            eval("has(entry.view) && type(entry.view) == map ? 'm' : 'o'", &e).unwrap(),
            Value::from("m")
        );
        assert_eq!(
            eval("['a_task', 'b'].filter(k, k in ['a_task'])[0].replace('_task', '')", &e).unwrap(),
            Value::from("a")
        );
        assert!(eval("entry.missing", &e).is_err());
        assert!(eval("has(entry.name.x)", &e).is_err());
        assert_eq!(eval("1 + 2 * 3 == 7", &[]).unwrap(), Value::Bool(true));
        assert_eq!(eval("string(1.0) + string(1e6)", &[]).unwrap(), Value::from("11e+06"));
    }

    #[test]
    fn an_unknown_variable_or_function_is_refused_at_compile_time() {
        let env = Environment::standard(&["entry"]);
        assert_eq!(
            env.compile("other.x").unwrap_err(),
            "'other' is not a variable here; available: entry."
        );
        assert!(env.compile("nope(entry)").unwrap_err().contains("not supported"));
    }

    #[test]
    fn the_rest_of_the_standard_environment_evaluates() {
        assert_eq!(eval("{'a': 1}.?a", &[]).unwrap(), Value::some(Value::Int(1)));
        assert_eq!(eval("{'a': 1}.?b.orValue(2)", &[]).unwrap(), Value::Int(2));
        assert_eq!(eval("[1][?3].hasValue()", &[]).unwrap(), Value::Bool(false));
        assert_eq!(
            eval("[3, 1, 2].sort()", &[]).unwrap(),
            Value::list(vec![1i64.into(), 2i64.into(), 3i64.into()])
        );
        assert_eq!(
            eval("['bb', 'a'].sortBy(s, size(s))[0]", &[]).unwrap(),
            Value::from("a")
        );
        assert_eq!(eval("math.greatest(1, 2.5, 2)", &[]).unwrap(), Value::Double(2.5));
        assert_eq!(eval("math.round(-2.5)", &[]).unwrap(), Value::Double(-3.0));
        assert_eq!(eval("[[1], [2, [3]]].flatten().size()", &[]).unwrap(), Value::Int(3));
        assert_eq!(
            eval("[1, 1, 2].distinct()", &[]).unwrap(),
            Value::list(vec![1i64.into(), 2i64.into()])
        );
        assert_eq!(eval("[].first()", &[]).unwrap(), Value::none());
        assert!(eval("[1, 'a'].sort()", &[]).is_err());
    }

    #[test]
    fn an_environment_takes_functions_and_a_budget() {
        let mut environment = Environment::standard(&["n"]);
        environment.add_function(Function::global("twice", 1, |a| {
            Ok(Value::Int(a[0].as_int().unwrap_or(0) * 2))
        }));
        environment.budget = 3;
        let program = environment.compile("twice(n)").unwrap();
        let variables: HashMap<String, Value> = [("n".to_owned(), Value::Int(4))].into_iter().collect();
        assert_eq!(program.evaluate(&variables).unwrap(), Value::Int(8));
        let costly = environment.compile("twice(twice(twice(n)))").unwrap();
        assert_eq!(
            costly.evaluate(&variables).unwrap_err(),
            "The expression exceeded its evaluation budget."
        );
        assert_eq!(
            environment.compile("twice(1, 2)").unwrap_err(),
            "The function 'twice' takes 1 argument, not 2, in 'twice(1, 2)'."
        );
    }
}
