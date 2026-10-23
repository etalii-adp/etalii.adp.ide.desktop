//! The interpreter of ATD's script (etalii.adp spec 035, ATD §12, research R13, R21).
//!
//! A [`Script`] holds a description's functions over the checked tree, each callable from CEL by its name. A
//! [`Gesture`] runs them, and the statements of operations and hooks, for one gesture of the user: its step budget is
//! the gesture's, its actions go to the [`Host`] in the order they run, and a call that ends other than by returning
//! (a `fail`, a CEL error, a refused action, a loop at its limit, a budget or recursion overrun, the debugger) is a
//! [`Stop`] naming the function, on which the host discards the [`Transaction`] and refuses the gesture.
//!
//! Steps are counted as ATD §12.6 states: one per statement executed, per loop iteration, per expression evaluated
//! and per call of a function of the description.

mod frame;
mod host;
mod stop;
mod transaction;

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::rc::{Rc, Weak};

use adp_atd_check::Symbols;
use adp_cel::{Call, Environment, Function, Map, Program, Style, Value};
use serde_json::Value as Json;

pub use host::{Action, Control, Event, Host, Performed};
pub use stop::{Kind, Stop};
pub use transaction::Transaction;

use frame::Frame;

/// The steps of a gesture unless the description's `limits.steps` sets fewer (ATD §12.6).
pub const STEPS: u64 = 100_000;
/// The deepest the functions of a gesture may call one another (ATD §12.6).
pub const FRAMES: usize = 64;
/// The most elements or code points a value may hold (ATD §12.5).
pub const LENGTH: usize = 1_000_000;

/// The limits a script runs under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// The gesture's step budget: [`STEPS`], or the description's lower `limits.steps`.
    pub steps: u64,
}

impl Default for Options {
    fn default() -> Self {
        Options { steps: STEPS }
    }
}

/// A description's functions, compiled once and run by any number of gestures.
#[derive(Clone)]
pub struct Script {
    inner: Rc<Inner>,
}

pub(crate) struct Inner {
    pub(crate) functions: BTreeMap<String, Json>,
    environment: Environment,
    programs: RefCell<HashMap<String, Program>>,
    pub(crate) options: Options,
    /// The gesture running now, for a function CEL calls.
    current: RefCell<Option<Rc<Run>>>,
}

/// The debugger's hook (research R21).
type Hook = Box<dyn FnMut(&Event<'_>) -> Control>;

/// The state of one gesture.
pub(crate) struct Run {
    pub(crate) host: Rc<RefCell<dyn Host>>,
    pub(crate) steps: Cell<u64>,
    /// The function of every frame on the stack.
    pub(crate) frames: RefCell<Vec<String>>,
    /// A stop inside a function CEL called, kept until the evaluation that called it returns.
    pub(crate) stopped: RefCell<Option<Stop>>,
    pub(crate) hook: RefCell<Option<Hook>>,
}

impl Script {
    /// The functions of `symbols` over `environment`, the CEL environment of the description's expressions (DISL
    /// §12.4's library and the host's functions). `range` is added when the environment has none.
    pub fn new(symbols: &Symbols, environment: Environment, options: Options) -> Self {
        Self::with_functions(symbols.functions.clone(), environment, options)
    }

    /// The functions, each a `fn` node of the checked tree by its name.
    pub fn with_functions(functions: BTreeMap<String, Json>, mut environment: Environment, options: Options) -> Self {
        let inner = Rc::new_cyclic(|weak: &Weak<Inner>| {
            if environment.function("range", Style::Global).is_none() {
                environment.add_function(range());
            }
            for (name, function) in &functions {
                let arity = function["params"].as_array().map_or(0, Vec::len);
                let weak = weak.clone();
                let called = name.clone();
                let body = move |call: &Call<'_>| -> Result<Value, String> {
                    let inner = weak.upgrade().ok_or("The script is gone.")?;
                    inner.called(&called, call.arguments)
                };
                environment.add_function(Function::new(name, Style::Global, arity, arity, Some(Rc::new(body))));
            }
            Inner {
                functions,
                environment,
                programs: RefCell::new(HashMap::new()),
                options,
                current: RefCell::new(None),
            }
        });
        Script { inner }
    }

    /// A gesture in `host`, with a budget of its own.
    pub fn gesture(&self, host: Rc<RefCell<dyn Host>>) -> Gesture {
        Gesture {
            inner: Rc::clone(&self.inner),
            run: Rc::new(Run {
                host,
                steps: Cell::new(0),
                frames: RefCell::new(Vec::new()),
                stopped: RefCell::new(None),
                hook: RefCell::new(None),
            }),
        }
    }
}

/// One gesture of the user: the calls it makes share its step budget.
pub struct Gesture {
    inner: Rc<Inner>,
    run: Rc<Run>,
}

impl Gesture {
    /// Calls a function of the description.
    pub fn call(&self, function: &str, arguments: &[Value]) -> Result<Value, Stop> {
        let _current = Current::enter(&self.inner, &self.run);
        self.inner.invoke(&self.run, function, arguments)
    }

    /// Runs the statements of an operation, a hook or a fix, named `name` in a stop; `bindings` are the names they see
    /// first (`self`, `p`).
    pub fn run(&self, name: &str, body: &Json, bindings: Map) -> Result<(), Stop> {
        let _current = Current::enter(&self.inner, &self.run);
        self.inner.run_body(&self.run, name, body, bindings).map(|_| ())
    }

    /// Evaluates an expression (`{cel}` node) of the description, which may call its functions.
    pub fn evaluate(&self, name: &str, expression: &Json, bindings: Map) -> Result<Value, Stop> {
        let _current = Current::enter(&self.inner, &self.run);
        let frame = Frame::new(&self.inner, &self.run, name, bindings);
        frame.evaluate(expression)
    }

    /// The steps taken so far.
    pub fn steps(&self) -> u64 {
        self.run.steps.get()
    }

    /// Calls `hook` before each statement; a [`Control::Stop`] stops the call (research R21).
    pub fn on_statement(&self, hook: impl FnMut(&Event<'_>) -> Control + 'static) {
        *self.run.hook.borrow_mut() = Some(Box::new(hook));
    }
}

/// Makes a gesture the one CEL's calls of the description's functions run in, until dropped.
struct Current<'a> {
    inner: &'a Inner,
    previous: Option<Rc<Run>>,
}

impl<'a> Current<'a> {
    fn enter(inner: &'a Inner, run: &Rc<Run>) -> Self {
        let previous = inner.current.replace(Some(Rc::clone(run)));
        Current { inner, previous }
    }
}

impl Drop for Current<'_> {
    fn drop(&mut self) {
        *self.inner.current.borrow_mut() = self.previous.take();
    }
}

impl Inner {
    /// A function of the description, called from CEL: a stop is kept for the evaluation that called it, which CEL
    /// sees as an error.
    fn called(&self, function: &str, arguments: &[Value]) -> Result<Value, String> {
        let run = self
            .current
            .borrow()
            .clone()
            .ok_or_else(|| format!("`{function}` is called outside a gesture."))?;
        self.invoke(&run, function, arguments).map_err(|stop| {
            let message = stop.message.clone();
            run.stopped.borrow_mut().get_or_insert(stop);
            message
        })
    }

    pub(crate) fn invoke(&self, run: &Rc<Run>, name: &str, arguments: &[Value]) -> Result<Value, Stop> {
        let Some(function) = self.functions.get(name) else {
            return Err(Stop::new(
                Kind::Error,
                name,
                &Json::Null,
                format!("The description has no function `{name}`."),
            ));
        };
        let bound = function["recursion"]
            .as_u64()
            .map_or(FRAMES, |n| (n as usize).min(FRAMES));
        {
            let frames = run.frames.borrow();
            let deep = frames.iter().filter(|f| *f == name).count();
            if frames.len() >= FRAMES || deep >= bound {
                let message = if deep >= bound {
                    format!("`{name}` recursed deeper than its bound of {bound} frames.")
                } else {
                    format!("`{name}` was called {FRAMES} frames deep, the most a gesture's calls may nest.")
                };
                return Err(Stop::new(Kind::Recursion, name, &function["name"], message));
            }
        }
        let mut bindings = Map::default();
        for (parameter, value) in function["params"].as_array().into_iter().flatten().zip(arguments) {
            bindings.insert(text(&parameter["name"]), value.clone());
        }
        let mut frame = Frame::new(self, run, name, bindings);
        frame.charge(&function["name"])?;
        match frame.body(&function["body"])? {
            Some(value) => Ok(value),
            None if function.get("returns").is_some() => Err(Stop::new(
                Kind::Error,
                name,
                &function["name"],
                format!("`{name}` ended without returning a value."),
            )),
            None => Ok(Value::Null),
        }
    }

    pub(crate) fn run_body(
        &self,
        run: &Rc<Run>,
        name: &str,
        body: &Json,
        bindings: Map,
    ) -> Result<Option<Value>, Stop> {
        let mut frame = Frame::new(self, run, name, bindings);
        frame.body(body)
    }

    /// The compiled expression, its identifiers declared, so that a local, a parameter or a host variable resolves
    /// when it is read.
    pub(crate) fn program(&self, expression: &Json, names: impl FnOnce() -> Vec<String>) -> Result<Program, String> {
        let source = expression["cel"].as_str().unwrap_or_default();
        if let Some(program) = self.programs.borrow().get(source) {
            return Ok(program.clone());
        }
        let mut identifiers = Vec::new();
        match expression.get("ast") {
            Some(ast) => identifiers_of(ast, &mut identifiers),
            None => identifiers = names(),
        }
        let mut environment = self.environment.clone();
        environment.declare_variables(identifiers.iter().map(String::as_str));
        let program = environment.compile(source)?;
        if expression.get("ast").is_some() {
            self.programs.borrow_mut().insert(source.to_owned(), program.clone());
        }
        Ok(program)
    }
}

/// A name of the tree (`{"text": …}`).
pub(crate) fn text(node: &Json) -> String {
    node["text"].as_str().unwrap_or_default().to_owned()
}

fn identifiers_of(ast: &Json, out: &mut Vec<String>) {
    match ast {
        Json::Object(object) => {
            if let Some(name) = object.get("ident").and_then(Json::as_str)
                && !out.iter().any(|n| n == name)
            {
                out.push(name.to_owned());
            }
            for value in object.values() {
                identifiers_of(value, out);
            }
        }
        Json::Array(items) => items.iter().for_each(|i| identifiers_of(i, out)),
        _ => {}
    }
}

/// `range(n)`, the ints from 0 below `n`, and `range(a, b)`, from `a` below `b` (CEL's lists extension).
fn range() -> Function {
    let body = |call: &Call<'_>| -> Result<Value, String> {
        let bounds: Vec<i64> = call
            .arguments
            .iter()
            .map(|a| a.as_int().ok_or("`range` takes ints."))
            .collect::<Result<_, _>>()?;
        let (from, to) = match bounds[..] {
            [to] => (0, to),
            [from, to] => (from, to),
            _ => return Err("`range` takes one or two ints.".into()),
        };
        if to.saturating_sub(from) > LENGTH as i64 {
            return Err(format!("`range({from}, {to})` is longer than {LENGTH} elements."));
        }
        Ok(Value::list((from..to).map(Value::Int).collect()))
    };
    Function::new("range", Style::Global, 1, 2, Some(Rc::new(body))).with_cost(|a| {
        let ints: Vec<i64> = a.iter().filter_map(Value::as_int).collect();
        match ints[..] {
            [to] => to.max(0),
            [from, to] => to.saturating_sub(from).max(0),
            _ => 1,
        }
    })
}
