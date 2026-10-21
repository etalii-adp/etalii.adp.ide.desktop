//! The CEL facade (spec 030, research R7): the part of the standalone host's CEL engine
//! (`EtAlii.Adp.Specification.Cel` at 8b6ccfc) that FBL's bindings use, ported to Rust. An expression is compiled
//! against an environment, which names the variables it may read, and evaluated against their values; anything the
//! environment does not offer is refused at compile time, naming it. FBL section 2.4 shares the standard environment
//! with DISL; section 2.5's regular expression subset lives here too, because `matches()` uses it.
//!
//! What is left out of the standalone host's engine, because no binding uses it: optional values (`.?`, `[?`,
//! `first()`, `last()`), the math library, `sort()`, `sortBy()`, `distinct()`, `flatten()` and `slice()`.

mod eval;
mod parser;
pub mod regex_subset;
mod value;

pub use eval::{Program, Variables};
pub use regex_subset::BoundedRegex;
pub use value::{Map, Value};

/// What an expression may use: the variables it may read. The functions and macros are the standard environment's.
#[derive(Debug, Clone, Default)]
pub struct Environment {
    variables: Vec<String>,
}

impl Environment {
    /// The CEL standard environment with the variables `names`.
    pub fn standard(names: &[&str]) -> Self {
        Environment {
            variables: names.iter().map(|n| (*n).to_owned()).collect(),
        }
    }

    pub fn variables(&self) -> &[String] {
        &self.variables
    }

    pub fn is_variable(&self, name: &str) -> bool {
        self.variables.iter().any(|v| v == name)
    }

    /// Compiles `expression`; the error names the first thing it cannot use.
    pub fn compile(&self, expression: &str) -> Result<Program, String> {
        let node = parser::parse(expression, self)?;
        eval::check_variables(&node, self, &mut Vec::new())?;
        Ok(Program::new(node))
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
}
