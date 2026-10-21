//! Evaluating a notation's CEL where a scene is drawn (DISL sections 6.2 and 12.3), with the DISL runtime's own
//! machinery: each expression is compiled once in its context by the specification's environment
//! ([`DislSpecification::environment`], its library and user functions), and kept for as long as the evaluator
//! lives.
//!
//! This is where the standalone host and this one part ways on purpose. The standalone canvas library has no CEL: its
//! `compileNotation.ts` turns each expression a notation binds into a template over a payload its backend computed,
//! through a module's table of expression-to-path entries, and refuses any expression the table does not name. This
//! host has the runtime beside the scene, so a notation's expressions are evaluated as written, and none of those
//! tables exists. The one thing the runtime leaves to the place a diagram is drawn is `token(name)`, which it refuses
//! headless: here it is the theme token's value in the active mode (DISL section 6.2).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use adp_cel::{Function, Program, Value};
use adp_disl::expressions::contexts;
use adp_disl::{DislDiagram, DislElement, DislEnv, DislSpecification};

use crate::style::Theme;

/// The variables of one evaluation.
pub type Vars = HashMap<String, Value>;

/// A compiled program's key: its context, its source and the names bound beside the context's.
type ProgramKey = (String, String, String);

/// Evaluates a specification's notation expressions, with `token()` resolved in one theme mode.
pub struct Evaluator {
    specification: DislSpecification,
    theme: Rc<Theme>,
    env: DislEnv,
    compiled: RefCell<HashMap<ProgramKey, Result<Program, String>>>,
}

impl std::fmt::Debug for Evaluator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Evaluator({:?}, {:?})", self.specification, self.theme.mode())
    }
}

impl Evaluator {
    pub fn new(specification: &DislSpecification, theme: Theme, env: DislEnv) -> Self {
        Evaluator {
            specification: specification.clone(),
            theme: Rc::new(theme),
            env,
            compiled: RefCell::new(HashMap::new()),
        }
    }

    pub fn specification(&self) -> &DislSpecification {
        &self.specification
    }

    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    pub fn env(&self) -> &DislEnv {
        &self.env
    }

    /// The value of `source` compiled in `context` (DISL section 12.3), with `variables`; a variable the context does
    /// not declare (a list's `item`) is declared as bound.
    pub fn evaluate(&self, context: &str, source: &str, variables: &Vars) -> Result<Value, String> {
        let declared = contexts::variables_of(context);
        let mut bound: Vec<&str> = variables
            .keys()
            .map(String::as_str)
            .filter(|name| !declared.contains(name))
            .collect();
        bound.sort_unstable();
        let key = (context.to_owned(), source.to_owned(), bound.join(","));
        let program = {
            let mut compiled = self.compiled.borrow_mut();
            compiled
                .entry(key)
                .or_insert_with(|| {
                    let mut environment = self.specification.environment(context, &bound);
                    let theme = Rc::clone(&self.theme);
                    environment.add_function(Function::global("token", 1, move |arguments| {
                        let name = arguments[0].as_str().ok_or("token() takes a token name.")?;
                        theme
                            .value_of(name)
                            .ok_or_else(|| format!("The theme has no token '{name}'."))
                    }));
                    environment.compile(source)
                })
                .clone()
        };
        program?.evaluate(variables)
    }

    /// The `env` an expression reads.
    pub fn env_value(&self) -> Value {
        Value::map(self.env.to_cel())
    }

    /// The element context's variables for `element`: `self`, `diagram` and `env`.
    pub fn element_vars(&self, element: &DislElement) -> Vars {
        let mut vars = Vars::new();
        vars.insert("self".to_owned(), element.to_value());
        vars.insert("diagram".to_owned(), element.diagram().to_value());
        vars.insert("env".to_owned(), self.env_value());
        vars
    }

    /// The variables of an expression over the diagram alone: `diagram` and `env`.
    pub fn diagram_vars(&self, diagram: &DislDiagram) -> Vars {
        let mut vars = Vars::new();
        vars.insert("diagram".to_owned(), diagram.to_value());
        vars.insert("env".to_owned(), self.env_value());
        vars
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::Theme;
    use adp_disl::DislLoader;

    fn specification() -> DislSpecification {
        DislLoader::load(
            r##"{ "disl": "0.3", "language": { "id": "test", "version": "1.0.0" },
                 "metamodel": { "types": { "Card": { "attributes": { "name": { "type": "string" } } } } },
                 "notation": { "theme": { "tokens": { "color.accent": "#112233" }, "modes": { "dark": { "color.accent": "#445566" } } },
                               "nodes": { "Card": { "shape": "rect" } } } }"##,
        )
        .specification
        .expect("the specification loads")
    }

    #[test]
    fn an_element_expression_reads_the_element_and_a_token_reads_the_active_mode() {
        let specification = specification();
        let diagram = DislDiagram::new(&specification);
        let card = diagram
            .add_node("Card", "c1", [("name", Value::from("One"))].into_iter().collect())
            .unwrap();
        let light = Evaluator::new(&specification, Theme::of(&specification, None), DislEnv::default());
        let vars = light.element_vars(&card);
        assert_eq!(
            Ok(Value::from("One!")),
            light.evaluate(contexts::ELEMENT, "self.name + '!'", &vars)
        );
        assert_eq!(
            Ok(Value::from("#112233")),
            light.evaluate(contexts::ELEMENT, "token('color.accent')", &vars)
        );
        let dark = Evaluator::new(
            &specification,
            Theme::of(&specification, Some("dark")),
            DislEnv::default(),
        );
        assert_eq!(
            Ok(Value::from("#445566")),
            dark.evaluate(contexts::ELEMENT, "token('color.accent')", &vars)
        );
        assert!(light.evaluate(contexts::ELEMENT, "token('color.none')", &vars).is_err());
    }
}
