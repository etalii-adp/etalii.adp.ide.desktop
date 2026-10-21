//! Evaluating a compiled expression (the standalone host's `CelNode`, `CelScope`, `CelProgram` and the core and
//! strings libraries): every node with its value, an evaluation error as a sentence, and a budget of steps so that
//! an expression cannot run without end.

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::Environment;
use crate::regex_subset;
use crate::value::{Map, TYPE_NAMES, Value};

/// The steps one evaluation may take.
const BUDGET: u64 = 100_000;

const BUDGET_EXCEEDED: &str = "The expression exceeded its evaluation budget.";

#[derive(Debug, Clone)]
pub(crate) enum Node {
    Literal(Value),
    Ident(String),
    List(Vec<Node>),
    MapLiteral(Vec<(Node, Node)>),
    Member(Box<Node>, String),
    Index(Box<Node>, Box<Node>),
    Has(Box<Node>, String),
    Unary(&'static str, Box<Node>),
    Binary(&'static str, Box<Node>, Box<Node>),
    Conditional(Box<Node>, Box<Node>, Box<Node>),
    /// `target.macro(variable, body)`: a comprehension over a list, or over a map's keys.
    Comprehension(String, Box<Node>, String, Box<Node>),
    /// `cel.bind(variable, init, body)`.
    Bind(String, Box<Node>, Box<Node>),
    /// A call; for a receiver call, the receiver is the first argument the function sees.
    Call(String, Option<Box<Node>>, Vec<Node>),
}

/// The comprehension macros of the standard environment.
pub(crate) fn is_macro(name: &str) -> bool {
    matches!(name, "all" | "exists" | "exists_one" | "filter" | "map")
}

/// The number of arguments a function of the standard environment takes, the receiver not counted; none when there
/// is no such function in that call style.
pub(crate) fn arity(name: &str, receiver: bool) -> Option<(usize, usize)> {
    if receiver {
        match name {
            "size" | "lowerAscii" | "upperAscii" | "trim" | "reverse" => Some((0, 0)),
            "matches" | "startsWith" | "endsWith" | "contains" | "charAt" => Some((1, 1)),
            "replace" => Some((2, 3)),
            "indexOf" | "lastIndexOf" | "substring" | "split" => Some((1, 2)),
            "join" => Some((0, 1)),
            _ => None,
        }
    } else {
        match name {
            "size" | "int" | "double" | "string" | "type" | "lower" | "upper" | "lists.range" => Some((1, 1)),
            "matches" => Some((2, 2)),
            _ => None,
        }
    }
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
        Node::Comprehension(_, target, variable, body) => {
            check_variables(target, environment, bound)?;
            bound.push(variable.clone());
            let result = check_variables(body, environment, bound);
            bound.pop();
            result
        }
        Node::Bind(variable, init, body) => {
            check_variables(init, environment, bound)?;
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
        Node::Member(target, _) | Node::Has(target, _) | Node::Unary(_, target) => {
            check_variables(target, environment, bound)
        }
        Node::Index(a, b) | Node::Binary(_, a, b) => {
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

/// A compiled CEL expression.
#[derive(Debug, Clone)]
pub struct Program {
    root: Rc<Node>,
}

struct Scope<'a> {
    variables: &'a dyn Variables,
    bound: Vec<(String, Value)>,
    steps: Cell<u64>,
}

impl Scope<'_> {
    fn step(&self) -> Result<(), String> {
        let steps = self.steps.get() + 1;
        self.steps.set(steps);
        if steps > BUDGET {
            return Err(BUDGET_EXCEEDED.into());
        }
        Ok(())
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
    pub(crate) fn new(root: Node) -> Self {
        Program { root: Rc::new(root) }
    }

    /// Evaluates with `variables`: the value, or the sentence saying why the evaluation failed.
    pub fn evaluate(&self, variables: &dyn Variables) -> Result<Value, String> {
        let mut scope = Scope {
            variables,
            bound: Vec::new(),
            steps: Cell::new(0),
        };
        evaluate(&self.root, &mut scope)
    }

    /// True only when the expression evaluates to true.
    pub fn is_true(&self, variables: &dyn Variables) -> bool {
        matches!(self.evaluate(variables), Ok(Value::Bool(true)))
    }
}

fn as_string(value: &Value) -> Result<&str, String> {
    value.as_str().ok_or_else(|| "A string was expected.".to_owned())
}

fn as_int(value: &Value) -> Result<i64, String> {
    match value {
        Value::Int(i) => Ok(*i),
        _ => Err("An int was expected.".into()),
    }
}

fn as_double(value: &Value) -> Result<f64, String> {
    value.as_number().ok_or_else(|| "A number was expected.".to_owned())
}

fn as_list(value: &Value) -> Result<&[Value], String> {
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
        Node::Member(target, name) => {
            scope.step()?;
            let target = evaluate(target, scope)?;
            match &target {
                Value::Map(map) => map.get(name).cloned().ok_or_else(|| format!("No such key: '{name}'.")),
                _ => Err(format!("'{name}' is selected from a value that is not a map.")),
            }
        }
        Node::Index(target, key) => {
            scope.step()?;
            let target = evaluate(target, scope)?;
            let key = evaluate(key, scope)?;
            match &target {
                Value::List(list) => {
                    let i = as_int(&key)?;
                    if i >= 0 && (i as usize) < list.len() {
                        Ok(list[i as usize].clone())
                    } else {
                        Err(format!("Index {i} is out of range."))
                    }
                }
                Value::Map(map) => {
                    let k = as_string(&key)?;
                    map.get(k).cloned().ok_or_else(|| format!("No such key: '{k}'."))
                }
                _ => Err("Only a list or a map can be indexed.".into()),
            }
        }
        Node::Has(target, name) => {
            scope.step()?;
            match evaluate(target, scope)? {
                Value::Map(map) => Ok(Value::Bool(map.contains_key(name))),
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
        Node::Comprehension(name, target, variable, body) => {
            scope.step()?;
            let target = evaluate(target, scope)?;
            let items: Vec<Value> = match &target {
                Value::List(list) => list.iter().cloned().collect(),
                Value::Map(map) => map.keys().map(|k| Value::String(k.clone())).collect(),
                _ => return Err(format!("'{name}' needs a list or a map.")),
            };
            comprehension(name, items, variable, body, scope)
        }
        Node::Bind(variable, init, body) => {
            scope.step()?;
            let value = evaluate(init, scope)?;
            scope.bound.push((variable.clone(), value));
            let result = evaluate(body, scope);
            scope.bound.pop();
            result
        }
        Node::Call(name, receiver, arguments) => {
            scope.step()?;
            let mut args = Vec::with_capacity(arguments.len() + 1);
            if let Some(receiver) = receiver {
                args.push(evaluate(receiver, scope)?);
            }
            for argument in arguments {
                args.push(evaluate(argument, scope)?);
            }
            call(name, receiver.is_some(), &args)
        }
    }
}

fn comprehension(
    name: &str,
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
    match name {
        "all" => {
            for item in items {
                if matches!(run(item, scope)?, Value::Bool(false)) {
                    return Ok(Value::Bool(false));
                }
            }
            Ok(Value::Bool(true))
        }
        "exists" => {
            for item in items {
                if matches!(run(item, scope)?, Value::Bool(true)) {
                    return Ok(Value::Bool(true));
                }
            }
            Ok(Value::Bool(false))
        }
        "exists_one" => {
            let mut count = 0;
            for item in items {
                if matches!(run(item, scope)?, Value::Bool(true)) {
                    count += 1;
                }
            }
            Ok(Value::Bool(count == 1))
        }
        "filter" => {
            let mut kept = Vec::new();
            for item in items {
                if matches!(run(item.clone(), scope)?, Value::Bool(true)) {
                    kept.push(item);
                }
            }
            Ok(Value::list(kept))
        }
        _ => {
            let mut mapped = Vec::with_capacity(items.len());
            for item in items {
                mapped.push(run(item, scope)?);
            }
            Ok(Value::list(mapped))
        }
    }
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

fn compare(a: &Value, b: &Value) -> Result<std::cmp::Ordering, String> {
    Ok(match (a, b) {
        (Value::String(x), Value::String(y)) => x.cmp(y),
        (Value::Bool(x), Value::Bool(y)) => x.cmp(y),
        _ => as_double(a)?
            .partial_cmp(&as_double(b)?)
            .unwrap_or(std::cmp::Ordering::Less),
    })
}

fn size(value: &Value) -> Result<Value, String> {
    Ok(Value::Int(match value {
        Value::String(s) => s.chars().count() as i64,
        Value::List(l) => l.len() as i64,
        Value::Map(m) => m.len() as i64,
        _ => return Err("size() needs a string, a list or a map.".into()),
    }))
}

fn call(name: &str, receiver: bool, a: &[Value]) -> Result<Value, String> {
    let text = |i: usize| as_string(&a[i]);
    Ok(match (name, receiver) {
        ("size", _) => size(&a[0])?,
        ("matches", _) => {
            let expression = regex_subset::to_rust(text(1)?, false);
            let regex = regex::Regex::new(&expression).map_err(|e| format!("matches() cannot use its pattern: {e}"))?;
            Value::Bool(regex.is_match(text(0)?))
        }
        ("int", false) => Value::Int(match &a[0] {
            Value::Int(i) => *i,
            Value::Double(d) if d.is_finite() && *d > i64::MIN as f64 && *d < i64::MAX as f64 => d.trunc() as i64,
            Value::String(s) => s.parse().map_err(|_| "int() cannot convert this value.".to_owned())?,
            _ => return Err("int() cannot convert this value.".into()),
        }),
        ("double", false) => Value::Double(match &a[0] {
            Value::String(s) => s.parse().map_err(|_| format!("double() cannot convert '{s}'."))?,
            other => as_double(other)?,
        }),
        ("string", false) => Value::String(a[0].to_string()),
        ("type", false) => Value::Type(a[0].type_name()),
        ("lower", false) => Value::String(text(0)?.to_lowercase()),
        ("upper", false) => Value::String(text(0)?.to_uppercase()),
        ("lists.range", false) => Value::list((0..as_int(&a[0])?.max(0)).map(Value::Int).collect()),
        ("startsWith", true) => Value::Bool(text(0)?.starts_with(text(1)?)),
        ("endsWith", true) => Value::Bool(text(0)?.ends_with(text(1)?)),
        ("contains", true) => Value::Bool(text(0)?.contains(text(1)?)),
        ("replace", true) => {
            let limit = if a.len() > 3 { as_int(&a[3])? } else { -1 };
            if limit < 0 {
                Value::String(text(0)?.replace(text(1)?, text(2)?))
            } else {
                Value::String(text(0)?.replacen(text(1)?, text(2)?, limit as usize))
            }
        }
        ("lowerAscii", true) => Value::String(text(0)?.to_ascii_lowercase()),
        ("upperAscii", true) => Value::String(text(0)?.to_ascii_uppercase()),
        ("trim", true) => Value::String(text(0)?.trim().to_owned()),
        ("charAt", true) => {
            let points: Vec<char> = text(0)?.chars().collect();
            let index = as_int(&a[1])?;
            if index < 0 || index as usize > points.len() {
                return Err(format!(
                    "charAt({index}) is out of range for a string of {}.",
                    points.len()
                ));
            }
            Value::String(points.get(index as usize).map(|c| c.to_string()).unwrap_or_default())
        }
        ("indexOf", true) | ("lastIndexOf", true) => {
            let last = name == "lastIndexOf";
            if let Value::List(list) = &a[0] {
                let found = if last {
                    list.iter().rposition(|v| v.cel_equals(&a[1]))
                } else {
                    list.iter().position(|v| v.cel_equals(&a[1]))
                };
                return Ok(Value::Int(found.map(|i| i as i64).unwrap_or(-1)));
            }
            let points: Vec<char> = text(0)?.chars().collect();
            let sought: Vec<char> = text(1)?.chars().collect();
            let start = if a.len() > 2 {
                as_int(&a[2])?
            } else if last {
                points.len() as i64
            } else {
                0
            };
            if start < 0 || start as usize > points.len() {
                return Err(format!(
                    "{name}() start {start} is out of range for a string of {}.",
                    points.len()
                ));
            }
            let at = |i: usize| i + sought.len() <= points.len() && points[i..i + sought.len()] == sought[..];
            let found = if last {
                (0..=(start as usize).min(points.len().saturating_sub(sought.len())))
                    .rev()
                    .find(|i| at(*i))
            } else {
                (start as usize..=points.len()).find(|i| at(*i))
            };
            Value::Int(found.map(|i| i as i64).unwrap_or(-1))
        }
        ("substring", true) => {
            let points: Vec<char> = text(0)?.chars().collect();
            let start = as_int(&a[1])?;
            let end = if a.len() > 2 {
                as_int(&a[2])?
            } else {
                points.len() as i64
            };
            if start < 0 || start as usize > points.len() || end < start || end as usize > points.len() {
                return Err(format!(
                    "substring({start}, {end}) is out of range for a string of {}.",
                    points.len()
                ));
            }
            Value::String(points[start as usize..end as usize].iter().collect())
        }
        ("split", true) => {
            let limit = if a.len() > 2 { as_int(&a[2])? } else { -1 };
            let (t, separator) = (text(0)?, text(1)?);
            if limit == 0 {
                Value::list(Vec::new())
            } else if separator.is_empty() {
                let points: Vec<char> = t.chars().collect();
                let mut parts = Vec::new();
                for (i, c) in points.iter().enumerate() {
                    if limit > 0 && parts.len() as i64 == limit - 1 {
                        parts.push(Value::String(points[i..].iter().collect()));
                        return Ok(Value::list(parts));
                    }
                    parts.push(Value::String(c.to_string()));
                }
                Value::list(parts)
            } else if limit < 0 {
                Value::list(t.split(separator).map(Value::from).collect())
            } else {
                Value::list(t.splitn(limit as usize, separator).map(Value::from).collect())
            }
        }
        ("join", true) => {
            let list = as_list(&a[0])?;
            let separator = if a.len() > 1 { text(1)? } else { "" };
            let mut parts = Vec::with_capacity(list.len());
            for item in list {
                parts.push(item.as_str().ok_or("join() needs a list of strings.")?);
            }
            Value::String(parts.join(separator))
        }
        ("reverse", true) => match &a[0] {
            Value::List(list) => Value::list(list.iter().rev().cloned().collect()),
            other => Value::String(as_string(other)?.chars().rev().collect()),
        },
        _ => return Err(format!("'{name}()' is not a method of this value.")),
    })
}
