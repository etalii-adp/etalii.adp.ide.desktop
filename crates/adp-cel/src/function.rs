//! What an environment offers besides variables (the standalone host's `CelFunction`, `CelCall`, `CelMacro` and
//! `CelBudget`): functions, global or on a receiver, each with its arity, body and cost; the comprehension macros;
//! and the budget one evaluation may spend, which a function that evaluates another expression passes on.

use std::cell::Cell;
use std::fmt;
use std::rc::Rc;

use crate::eval::Variables;
use crate::value::Value;

/// The message of an evaluation that ran out of budget; a short circuit never absorbs it.
pub(crate) const BUDGET_EXCEEDED: &str = "The expression exceeded its evaluation budget.";

/// How a function is written at its call: `f(x, y)` or `x.f(y)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Style {
    /// `f(x, y)`, also with a dotted name such as `math.round(x)`.
    Global,
    /// `x.f(y)`: the receiver is the first of the arguments the body sees.
    Receiver,
}

/// The evaluation cost an expression may spend (DISL section 2.5, FBL section 16): every step and every function's
/// declared cost is charged, and the evaluation fails once the limit is passed, so no expression runs away.
#[derive(Debug)]
pub struct Budget {
    limit: i64,
    used: Cell<i64>,
}

impl Budget {
    pub fn new(limit: i64) -> Self {
        Budget {
            limit,
            used: Cell::new(0),
        }
    }

    pub fn used(&self) -> i64 {
        self.used.get()
    }

    pub fn charge(&self, cost: i64) -> Result<(), String> {
        let used = self.used.get().saturating_add(cost.max(0));
        self.used.set(used);
        if used > self.limit {
            return Err(BUDGET_EXCEEDED.into());
        }
        Ok(())
    }
}

/// One call of a function: its arguments (a receiver first), the budget it may charge further, and the variables
/// the evaluation it is part of was given - not those a comprehension or `cel.bind` bound. A function that may read
/// a context variable, as a DISL user function with `uses` may read `diagram`, takes it from here.
pub struct Call<'a> {
    pub arguments: &'a [Value],
    pub budget: &'a Budget,
    pub variables: &'a dyn Variables,
}

impl Call<'_> {
    pub fn len(&self) -> usize {
        self.arguments.len()
    }

    pub fn is_empty(&self) -> bool {
        self.arguments.is_empty()
    }
}

impl std::ops::Index<usize> for Call<'_> {
    type Output = Value;

    fn index(&self, index: usize) -> &Value {
        &self.arguments[index]
    }
}

/// A function's body: the value, or the sentence saying why the call failed.
pub type Body = Rc<dyn Fn(&Call<'_>) -> Result<Value, String>>;

/// A function's cost per call, from its arguments.
pub type Cost = Rc<dyn Fn(&[Value]) -> i64>;

/// A function an environment offers: its name, how it is called, how many arguments it takes (the receiver not
/// counted), its body and its cost per call. A function without a body is a method only [`crate::Object`]s
/// implement: declaring it lets an expression call it, and calling it on any other value is an evaluation error.
#[derive(Clone)]
pub struct Function {
    pub name: String,
    pub style: Style,
    pub min_arguments: usize,
    pub max_arguments: usize,
    pub body: Option<Body>,
    cost: Option<Cost>,
}

impl fmt::Debug for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({:?})", self.name, self.style)
    }
}

impl Function {
    /// For `max_arguments`: any number.
    pub const VARIADIC: usize = usize::MAX;

    pub fn new(name: &str, style: Style, min_arguments: usize, max_arguments: usize, body: Option<Body>) -> Self {
        assert!(max_arguments >= min_arguments, "The arity range is empty.");
        Function {
            name: name.to_owned(),
            style,
            min_arguments,
            max_arguments,
            body,
            cost: None,
        }
    }

    /// `f(...)` with a fixed number of arguments.
    pub fn global(name: &str, arguments: usize, body: impl Fn(&[Value]) -> Result<Value, String> + 'static) -> Self {
        Function::new(
            name,
            Style::Global,
            arguments,
            arguments,
            Some(Rc::new(move |call: &Call<'_>| body(call.arguments))),
        )
    }

    /// `x.f(...)` with a fixed number of arguments besides the receiver, which the body sees first.
    pub fn receiver(name: &str, arguments: usize, body: impl Fn(&[Value]) -> Result<Value, String> + 'static) -> Self {
        Function::new(
            name,
            Style::Receiver,
            arguments,
            arguments,
            Some(Rc::new(move |call: &Call<'_>| body(call.arguments))),
        )
    }

    /// `x.f(...)` that only [`crate::Object`]s implement.
    pub fn method(name: &str, min_arguments: usize, max_arguments: usize) -> Self {
        Function::new(name, Style::Receiver, min_arguments, max_arguments, None)
    }

    /// The function with a cost per call.
    pub fn with_cost(mut self, cost: impl Fn(&[Value]) -> i64 + 'static) -> Self {
        self.cost = Some(Rc::new(cost));
        self
    }

    /// The cost one call charges, at least one step; library functions over lists and strings charge their length.
    pub fn cost_of(&self, arguments: &[Value]) -> i64 {
        self.cost.as_ref().map(|c| c(arguments)).unwrap_or(1).max(1)
    }

    pub(crate) fn arity(&self) -> String {
        if self.min_arguments == self.max_arguments {
            self.min_arguments.to_string()
        } else if self.max_arguments == Function::VARIADIC {
            format!("at least {}", self.min_arguments)
        } else {
            format!("{} to {}", self.min_arguments, self.max_arguments)
        }
    }
}

/// A comprehension macro, written `target.name(x, expression)`: `all`, `exists`, `exists_one`, `filter`, `map` and
/// `sortBy`. `has()` and `cel.bind()` are part of the language instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Macro {
    All,
    Exists,
    ExistsOne,
    Filter,
    Map,
    SortBy,
}

impl Macro {
    pub fn name(self) -> &'static str {
        match self {
            Macro::All => "all",
            Macro::Exists => "exists",
            Macro::ExistsOne => "exists_one",
            Macro::Filter => "filter",
            Macro::Map => "map",
            Macro::SortBy => "sortBy",
        }
    }
}
