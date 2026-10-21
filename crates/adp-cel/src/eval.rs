//! Evaluating a compiled expression (the standalone host's `CelNode`, `CelScope`, `CelProgram`, `CelValues` and
//! `CelInt`): every node with its value, an evaluation error as a sentence, and a budget so that an expression
//! cannot run without end.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::rc::Rc;

use crate::Environment;
use crate::function::{BUDGET_EXCEEDED, Budget, Call, Function, Macro};
use crate::value::{Map, TYPE_NAMES, Value};

#[derive(Debug, Clone)]
pub(crate) enum Node {
    Literal(Value),
    Ident(String),
    List(Vec<Node>),
    MapLiteral(Vec<(Node, Node)>),
    /// `x.name`, and `x.?name` when optional.
    Member(Box<Node>, String, bool),
    /// `x[key]`, and `x[?key]` when optional.
    Index(Box<Node>, Box<Node>, bool),
    Has(Box<Node>, String),
    Unary(&'static str, Box<Node>),
    Binary(&'static str, Box<Node>, Box<Node>),
    Conditional(Box<Node>, Box<Node>, Box<Node>),
    /// `target.macro(variable, body)`: a comprehension over a list, or over a map's keys.
    Comprehension(Macro, Box<Node>, String, Box<Node>),
    /// `cel.bind(variable, init, body)`.
    Bind(String, Box<Node>, Box<Node>),
    /// A call; for a receiver call, the receiver is the first argument the function's body sees.
    Call(Rc<Function>, Option<Box<Node>>, Vec<Node>),
}

/// Refuses an identifier that names no variable, no variable a comprehension binds and no type.
pub(crate) fn check_variables(node: &Node, environment: &Environment, bound: &mut Vec<String>) -> Result<(), String> {
    match node {
        Node::Ident(name) => {
            if !environment.is_variable(name) && !bound.contains(name) && !TYPE_NAMES.contains(&name.as_str()) {
                return Err(format!(
                    "'{name}' is not a variable here; available: {}.",
                    environment.variables().join(", ")
                ));
            }
            Ok(())
        }
        Node::Comprehension(_, target, variable, body) | Node::Bind(variable, target, body) => {
            check_variables(target, environment, bound)?;
            bound.push(variable.clone());
            let result = check_variables(body, environment, bound);
            bound.pop();
            result
        }
        Node::Literal(_) => Ok(()),
        Node::List(items) => items.iter().try_for_each(|i| check_variables(i, environment, bound)),
        Node::MapLiteral(entries) => entries.iter().try_for_each(|(k, v)| {
            check_variables(k, environment, bound)?;
            check_variables(v, environment, bound)
        }),
        Node::Member(target, _, _) | Node::Has(target, _) | Node::Unary(_, target) => {
            check_variables(target, environment, bound)
        }
        Node::Index(a, b, _) | Node::Binary(_, a, b) => {
            check_variables(a, environment, bound)?;
            check_variables(b, environment, bound)
        }
        Node::Conditional(a, b, c) => {
            check_variables(a, environment, bound)?;
            check_variables(b, environment, bound)?;
            check_variables(c, environment, bound)
        }
        Node::Call(_, receiver, arguments) => {
            if let Some(r) = receiver {
                check_variables(r, environment, bound)?;
            }
            arguments
                .iter()
                .try_for_each(|a| check_variables(a, environment, bound))
        }
    }
}

/// The values of the variables of one evaluation. A value may be made only when the expression reads it.
pub trait Variables {
    fn get(&self, name: &str) -> Option<Value>;
}

impl Variables for HashMap<String, Value> {
    fn get(&self, name: &str) -> Option<Value> {
        HashMap::get(self, name).cloned()
    }
}

impl Variables for Map {
    fn get(&self, name: &str) -> Option<Value> {
        Map::get(self, name).cloned()
    }
}

/// A compiled CEL expression.
#[derive(Debug, Clone)]
pub struct Program {
    root: Rc<Node>,
    budget: i64,
}

struct Scope<'a> {
    variables: &'a dyn Variables,
    bound: Vec<(String, Value)>,
    budget: &'a Budget,
}

impl Scope<'_> {
    fn step(&self) -> Result<(), String> {
        self.budget.charge(1)
    }

    fn lookup(&self, name: &str) -> Result<Value, String> {
        if let Some((_, value)) = self.bound.iter().rev().find(|(n, _)| n == name) {
            return Ok(value.clone());
        }
        // A type's name denotes the type only where nothing else has the name.
        if let Some(value) = self.variables.get(name) {
            return Ok(value);
        }
        if let Some(name) = TYPE_NAMES.iter().find(|t| **t == name) {
            return Ok(Value::Type(name));
        }
        Err(format!("'{name}' has no value."))
    }
}

impl Program {
    pub(crate) fn new(root: Node, budget: i64) -> Self {
        Program {
            root: Rc::new(root),
            budget,
        }
    }

    /// The cost one evaluation may spend.
    pub fn budget(&self) -> i64 {
        self.budget
    }

    /// Evaluates with `variables`: the value, or the sentence saying why the evaluation failed.
    pub fn evaluate(&self, variables: &dyn Variables) -> Result<Value, String> {
        let budget = Budget::new(self.budget);
        self.evaluate_within(variables, &budget)
    }

    /// Evaluates with `variables`, charging `budget`: how a function that evaluates another expression keeps the
    /// whole evaluation within one limit.
    pub fn evaluate_within(&self, variables: &dyn Variables, budget: &Budget) -> Result<Value, String> {
        let mut scope = Scope {
            variables,
            bound: Vec::new(),
            budget,
        };
        evaluate(&self.root, &mut scope)
    }

    /// True only when the expression evaluates to true.
    pub fn is_true(&self, variables: &dyn Variables) -> bool {
        matches!(self.evaluate(variables), Ok(Value::Bool(true)))
    }
}

pub(crate) fn as_string(value: &Value) -> Result<&str, String> {
    value.as_str().ok_or_else(|| "A string was expected.".to_owned())
}

pub(crate) fn as_int(value: &Value) -> Result<i64, String> {
    match value {
        Value::Int(i) => Ok(*i),
        _ => Err("An int was expected.".into()),
    }
}

pub(crate) fn as_double(value: &Value) -> Result<f64, String> {
    value.as_number().ok_or_else(|| "A number was expected.".to_owned())
}

pub(crate) fn as_list(value: &Value) -> Result<&[Value], String> {
    value.as_list().ok_or_else(|| "A list was expected.".to_owned())
}

fn overflow() -> String {
    "Integer overflow.".into()
}

fn evaluate(node: &Node, scope: &mut Scope<'_>) -> Result<Value, String> {
    match node {
        Node::Literal(value) => Ok(value.clone()),
        Node::Ident(name) => {
            scope.step()?;
            scope.lookup(name)
        }
        Node::List(items) => {
            let mut list = Vec::with_capacity(items.len());
            for item in items {
                list.push(evaluate(item, scope)?);
            }
            Ok(Value::list(list))
        }
        Node::MapLiteral(entries) => {
            let mut map = Map::default();
            for (k, v) in entries {
                let key = evaluate(k, scope)?;
                let key = as_string(&key)?.to_owned();
                let value = evaluate(v, scope)?;
                map.insert(key, value);
            }
            Ok(Value::map(map))
        }
        Node::Member(target, name, optional) => {
            scope.step()?;
            let target = evaluate(target, scope)?;
            match &target {
                // Selection on an optional chains: none stays none, a value is selected from.
                Value::Optional(Some(value)) => select(value, name, true),
                Value::Optional(None) => Ok(Value::none()),
                _ => select(&target, name, *optional),
            }
        }
        Node::Index(target, key, optional) => {
            scope.step()?;
            let target = evaluate(target, scope)?;
            let key = evaluate(key, scope)?;
            match &target {
                Value::Optional(Some(value)) => index(value, &key, true),
                Value::Optional(None) => Ok(Value::none()),
                _ => index(&target, &key, *optional),
            }
        }
        Node::Has(target, name) => {
            scope.step()?;
            match evaluate(target, scope)? {
                Value::Map(map) => Ok(Value::Bool(map.contains_key(name))),
                Value::Object(object) if object.is_selectable() => Ok(Value::Bool(object.has_member(name))),
                _ => Err("has() needs a map.".into()),
            }
        }
        Node::Unary(op, operand) => {
            scope.step()?;
            let value = evaluate(operand, scope)?;
            match (*op, value) {
                ("!", Value::Bool(b)) => Ok(Value::Bool(!b)),
                ("!", _) => Err("'!' needs a bool.".into()),
                ("-", Value::Int(i)) => i.checked_neg().map(Value::Int).ok_or_else(overflow),
                ("-", Value::Double(d)) => Ok(Value::Double(-d)),
                ("-", _) => Err("'-' needs a number.".into()),
                (op, _) => Err(format!("Unknown operator '{op}'.")),
            }
        }
        Node::Binary(op, left, right) => {
            scope.step()?;
            binary(op, left, right, scope)
        }
        Node::Conditional(condition, then, otherwise) => {
            scope.step()?;
            match evaluate(condition, scope)? {
                Value::Bool(true) => evaluate(then, scope),
                Value::Bool(false) => evaluate(otherwise, scope),
                _ => Err("A conditional needs a bool.".into()),
            }
        }
        Node::Comprehension(kind, target, variable, body) => {
            scope.step()?;
            let target = evaluate(target, scope)?;
            let items: Vec<Value> = match &target {
                Value::List(list) => list.iter().cloned().collect(),
                Value::Map(map) => map.keys().map(|k| Value::String(k.clone())).collect(),
                _ => return Err(format!("'{}' needs a list or a map.", kind.name())),
            };
            comprehension(*kind, items, variable, body, scope)
        }
        Node::Bind(variable, init, body) => {
            scope.step()?;
            let value = evaluate(init, scope)?;
            scope.bound.push((variable.clone(), value));
            let result = evaluate(body, scope);
            scope.bound.pop();
            result
        }
        Node::Call(function, receiver, arguments) => {
            scope.step()?;
            let receiver = match receiver {
                Some(r) => Some(evaluate(r, scope)?),
                None => None,
            };
            let mut args = Vec::with_capacity(arguments.len() + 1);
            for argument in arguments {
                args.push(evaluate(argument, scope)?);
            }
            if let Some(receiver) = receiver {
                if let Value::Object(object) = &receiver
                    && object.is_selectable()
                    && let Some(result) = object.invoke(&function.name, &args)
                {
                    let value = result?;
                    scope.budget.charge(function.cost_of(&args))?;
                    return Ok(value);
                }
                if function.body.is_none() {
                    return Err(format!("'{}()' is not a method of this value.", function.name));
                }
                args.insert(0, receiver);
            }
            scope.budget.charge(function.cost_of(&args))?;
            let Some(body) = function.body.as_ref() else {
                return Err(format!("'{}()' is not a method of this value.", function.name));
            };
            body(&Call {
                arguments: &args,
                budget: scope.budget,
                variables: scope.variables,
            })
        }
    }
}

fn select(target: &Value, name: &str, optional: bool) -> Result<Value, String> {
    match target {
        Value::Map(map) => match map.get(name) {
            Some(value) if optional => Ok(Value::some(value.clone())),
            Some(value) => Ok(value.clone()),
            None if optional => Ok(Value::none()),
            None => Err(format!("No such key: '{name}'.")),
        },
        Value::Object(object) if object.is_selectable() => {
            if optional {
                return Ok(if object.has_member(name) {
                    object.member(name).map(Value::some).unwrap_or_else(Value::none)
                } else {
                    Value::none()
                });
            }
            object.member(name).ok_or_else(|| format!("No such field: '{name}'."))
        }
        _ => Err(format!("'{name}' is selected from a value that is not a map.")),
    }
}

fn index(target: &Value, key: &Value, optional: bool) -> Result<Value, String> {
    match target {
        Value::List(list) => {
            let i = as_int(key)?;
            if i >= 0 && (i as usize) < list.len() {
                let value = list[i as usize].clone();
                Ok(if optional { Value::some(value) } else { value })
            } else if optional {
                Ok(Value::none())
            } else {
                Err(format!("Index {i} is out of range."))
            }
        }
        Value::Map(map) => {
            let k = as_string(key)?;
            match map.get(k) {
                Some(value) if optional => Ok(Value::some(value.clone())),
                Some(value) => Ok(value.clone()),
                None if optional => Ok(Value::none()),
                None => Err(format!("No such key: '{k}'.")),
            }
        }
        _ => Err("Only a list or a map can be indexed.".into()),
    }
}

fn comprehension(
    kind: Macro,
    items: Vec<Value>,
    variable: &str,
    body: &Node,
    scope: &mut Scope<'_>,
) -> Result<Value, String> {
    let run = |item: Value, scope: &mut Scope<'_>| -> Result<Value, String> {
        scope.bound.push((variable.to_owned(), item));
        let result = evaluate(body, scope);
        scope.bound.pop();
        result
    };
    match kind {
        Macro::All => {
            for item in items {
                if matches!(run(item, scope)?, Value::Bool(false)) {
                    return Ok(Value::Bool(false));
                }
            }
            Ok(Value::Bool(true))
        }
        Macro::Exists => {
            for item in items {
                if matches!(run(item, scope)?, Value::Bool(true)) {
                    return Ok(Value::Bool(true));
                }
            }
            Ok(Value::Bool(false))
        }
        Macro::ExistsOne => {
            let mut count = 0;
            for item in items {
                if matches!(run(item, scope)?, Value::Bool(true)) {
                    count += 1;
                }
            }
            Ok(Value::Bool(count == 1))
        }
        Macro::Filter => {
            let mut kept = Vec::new();
            for item in items {
                if matches!(run(item.clone(), scope)?, Value::Bool(true)) {
                    kept.push(item);
                }
            }
            Ok(Value::list(kept))
        }
        Macro::Map => {
            let mut mapped = Vec::with_capacity(items.len());
            for item in items {
                mapped.push(run(item, scope)?);
            }
            Ok(Value::list(mapped))
        }
        Macro::SortBy => {
            let mut keyed = Vec::with_capacity(items.len());
            for item in items {
                let key = run(item.clone(), scope)?;
                keyed.push((item, key));
            }
            sort(keyed).map(Value::list)
        }
    }
}

/// A stable sort by key; the keys must be all strings, all numbers or all bools (DISL section 12.1).
pub(crate) fn sort(mut keyed: Vec<(Value, Value)>) -> Result<Vec<Value>, String> {
    let mut kinds: Vec<&str> = Vec::new();
    for (_, key) in &keyed {
        let kind = match key {
            Value::String(_) => "string",
            Value::Int(_) | Value::Double(_) => "number",
            Value::Bool(_) => "bool",
            _ => return Err("sort() needs strings, numbers or bools.".into()),
        };
        if !kinds.contains(&kind) {
            kinds.push(kind);
        }
    }
    if kinds.len() > 1 {
        return Err("sort() needs values of one type.".into());
    }
    keyed.sort_by(|a, b| compare(&a.1, &b.1).unwrap_or(Ordering::Equal));
    Ok(keyed.into_iter().map(|(item, _)| item).collect())
}

fn binary(op: &str, left: &Node, right: &Node, scope: &mut Scope<'_>) -> Result<Value, String> {
    if op == "&&" || op == "||" {
        // An error on one side is absorbed when the other decides; running out of budget never is.
        let left = absorb(evaluate(left, scope))?;
        if op == "&&" && matches!(left, Ok(Value::Bool(false))) {
            return Ok(Value::Bool(false));
        }
        if op == "||" && matches!(left, Ok(Value::Bool(true))) {
            return Ok(Value::Bool(true));
        }
        let right = absorb(evaluate(right, scope))?;
        if op == "&&" && matches!(right, Ok(Value::Bool(false))) {
            return Ok(Value::Bool(false));
        }
        if op == "||" && matches!(right, Ok(Value::Bool(true))) {
            return Ok(Value::Bool(true));
        }
        if matches!(left, Ok(Value::Bool(_))) && matches!(right, Ok(Value::Bool(_))) {
            return Ok(Value::Bool(op == "&&"));
        }
        return Err(match (left, right) {
            (Err(e), _) | (_, Err(e)) => e,
            _ => format!("'{op}' needs bools."),
        });
    }
    let l = evaluate(left, scope)?;
    let r = evaluate(right, scope)?;
    Ok(match op {
        "==" => Value::Bool(l.cel_equals(&r)),
        "!=" => Value::Bool(!l.cel_equals(&r)),
        "<" => Value::Bool(compare(&l, &r)?.is_lt()),
        "<=" => Value::Bool(compare(&l, &r)?.is_le()),
        ">" => Value::Bool(compare(&l, &r)?.is_gt()),
        ">=" => Value::Bool(compare(&l, &r)?.is_ge()),
        "in" => match &r {
            Value::List(list) => Value::Bool(list.iter().any(|item| item.cel_equals(&l))),
            Value::Map(map) => Value::Bool(l.as_str().is_some_and(|k| map.contains_key(k))),
            _ => return Err("'in' needs a list or a map on its right.".into()),
        },
        "+" => match (&l, &r) {
            (Value::Int(a), Value::Int(b)) => Value::Int(a.checked_add(*b).ok_or_else(overflow)?),
            (Value::String(a), Value::String(b)) => Value::String(format!("{a}{b}")),
            (Value::List(a), Value::List(b)) => Value::list(a.iter().chain(b.iter()).cloned().collect()),
            _ => Value::Double(as_double(&l)? + as_double(&r)?),
        },
        "-" => match (&l, &r) {
            (Value::Int(a), Value::Int(b)) => Value::Int(a.checked_sub(*b).ok_or_else(overflow)?),
            _ => Value::Double(as_double(&l)? - as_double(&r)?),
        },
        "*" => match (&l, &r) {
            (Value::Int(a), Value::Int(b)) => Value::Int(a.checked_mul(*b).ok_or_else(overflow)?),
            _ => Value::Double(as_double(&l)? * as_double(&r)?),
        },
        "/" => match (&l, &r) {
            (Value::Int(_), Value::Int(0)) => return Err("Division by zero.".into()),
            (Value::Int(a), Value::Int(b)) => Value::Int(a.checked_div(*b).ok_or_else(overflow)?),
            _ => Value::Double(as_double(&l)? / as_double(&r)?),
        },
        "%" => match (&l, &r) {
            (Value::Int(_), Value::Int(0)) => return Err("Modulus by zero.".into()),
            (Value::Int(a), Value::Int(b)) => Value::Int(a.checked_rem(*b).ok_or_else(overflow)?),
            _ => return Err("'%' needs ints.".into()),
        },
        _ => return Err(format!("Unknown operator '{op}'.")),
    })
}

/// An evaluation error as a value of the short circuit, unless it is the budget's.
fn absorb(result: Result<Value, String>) -> Result<Result<Value, String>, String> {
    match result {
        Err(e) if e == BUDGET_EXCEEDED => Err(e),
        other => Ok(other),
    }
}

/// CEL's order (`CelValues.Compare`): strings by code point, bools, and numbers as .NET orders doubles, NaN first.
pub(crate) fn compare(a: &Value, b: &Value) -> Result<Ordering, String> {
    Ok(match (a, b) {
        (Value::String(x), Value::String(y)) => x.cmp(y),
        (Value::Bool(x), Value::Bool(y)) => x.cmp(y),
        _ => {
            let (x, y) = (as_double(a)?, as_double(b)?);
            match x.partial_cmp(&y) {
                Some(order) => order,
                None if x.is_nan() && y.is_nan() => Ordering::Equal,
                None if x.is_nan() => Ordering::Less,
                None => Ordering::Greater,
            }
        }
    })
}
