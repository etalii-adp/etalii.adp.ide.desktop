//! One frame of a call: its locals in nested scopes, its statements (ATD §12.2) and its actions (§12.3).

use std::rc::Rc;

use adp_cel::{Budget, Map, Value, Variables};
use serde_json::Value as Json;

use crate::host::{Action, Control, Event, Performed};
use crate::stop::{Kind, Stop};
use crate::{Inner, LENGTH, Run, text};

type Done<T> = Result<T, Stop>;

struct Local {
    name: String,
    value: Value,
    mutable: bool,
}

/// How a statement left its block.
enum Flow {
    Next,
    Break,
    Continue,
    Return(Option<Value>),
}

pub(crate) struct Frame<'a> {
    inner: &'a Inner,
    run: &'a Rc<Run>,
    function: String,
    scopes: Vec<Vec<Local>>,
}

impl<'a> Frame<'a> {
    pub(crate) fn new(inner: &'a Inner, run: &'a Rc<Run>, function: &str, bindings: Map) -> Self {
        run.frames.borrow_mut().push(function.to_owned());
        let locals = bindings
            .iter()
            .map(|(name, value)| Local {
                name: name.clone(),
                value: value.clone(),
                mutable: false,
            })
            .collect();
        Frame {
            inner,
            run,
            function: function.to_owned(),
            scopes: vec![locals],
        }
    }

    /// Runs a function's or an operation's block: the value it returned, if any.
    pub(crate) fn body(&mut self, body: &Json) -> Done<Option<Value>> {
        let statements = body.as_array().map(Vec::as_slice).unwrap_or_default();
        match self.block(statements)? {
            Flow::Return(value) => Ok(value),
            Flow::Next | Flow::Break | Flow::Continue => Ok(None),
        }
    }

    /// One step of the gesture's budget, at `node`.
    pub(crate) fn charge(&self, node: &Json) -> Done<()> {
        let steps = self.run.steps.get() + 1;
        self.run.steps.set(steps);
        let limit = self.inner.options.steps;
        if steps > limit {
            return Err(self.stop(
                Kind::Budget,
                node,
                format!("`{}` ran past the gesture's budget of {limit} steps.", self.function),
            ));
        }
        Ok(())
    }

    /// Evaluates a `{cel}` node: one step, and the stop of a function it called, if one stopped.
    pub(crate) fn evaluate(&self, expression: &Json) -> Done<Value> {
        self.charge(expression)?;
        let program = self
            .inner
            .program(expression, || self.names())
            .map_err(|e| self.stop(Kind::Error, expression, e))?;
        let budget = Budget::new(program.budget());
        let result = program.evaluate_within(self, &budget);
        if let Some(stop) = self.run.stopped.borrow_mut().take() {
            return Err(stop);
        }
        let value = result.map_err(|e| {
            // CEL's own budget of one expression, run out, is the script's budget overrun too.
            let kind = if budget.used() > program.budget() {
                Kind::Budget
            } else {
                Kind::Error
            };
            self.stop(kind, expression, format!("`{}`: {e}", self.function))
        })?;
        self.bounded(value, expression)
    }

    fn block(&mut self, statements: &[Json]) -> Done<Flow> {
        self.scopes.push(Vec::new());
        let mut flow = Ok(Flow::Next);
        for statement in statements {
            flow = self.statement(statement);
            if !matches!(flow, Ok(Flow::Next)) {
                break;
            }
        }
        self.scopes.pop();
        flow
    }

    fn statement(&mut self, statement: &Json) -> Done<Flow> {
        self.debug(statement)?;
        self.charge(statement)?;
        let kind = statement["kind"].as_str().unwrap_or_default();
        match kind {
            "let" | "var" => {
                let value = self.evaluate(&statement["value"])?;
                self.declare(text(&statement["name"]), value, kind == "var");
                Ok(Flow::Next)
            }
            "assign" => {
                let value = self.evaluate(&statement["value"])?;
                self.assign(&text(&statement["name"]), value, statement)?;
                Ok(Flow::Next)
            }
            "if" => {
                if self.condition(&statement["condition"])? {
                    self.branch(&statement["then"])
                } else {
                    self.branch(&statement["else"])
                }
            }
            "for" => self.for_loop(statement),
            "while" => self.while_loop(statement),
            "return" => match statement.get("value") {
                Some(value) => Ok(Flow::Return(Some(self.evaluate(value)?))),
                None => Ok(Flow::Return(None)),
            },
            "break" => Ok(Flow::Break),
            "continue" => Ok(Flow::Continue),
            "fail" => {
                let message = match self.evaluate(&statement["value"])? {
                    Value::String(message) => message,
                    other => other.to_string(),
                };
                Err(self.stop(Kind::Fail, statement, message))
            }
            _ => self.act(kind, statement),
        }
    }

    fn branch(&mut self, block: &Json) -> Done<Flow> {
        match block {
            Json::Array(statements) => self.block(statements),
            Json::Object(_) => self.statement(block),
            _ => Ok(Flow::Next),
        }
    }

    fn condition(&self, expression: &Json) -> Done<bool> {
        match self.evaluate(expression)? {
            Value::Bool(b) => Ok(b),
            other => Err(self.stop(
                Kind::Error,
                expression,
                format!("A condition is a bool; this one is {}.", other.type_name()),
            )),
        }
    }

    /// `for <name> [, <name>] in <CEL> limit <n>`: a list's items (with two names, its indexes and items), a map's keys
    /// in ascending order (with two names, its keys and values).
    fn for_loop(&mut self, statement: &Json) -> Done<Flow> {
        let names: Vec<String> = statement["names"].as_array().into_iter().flatten().map(text).collect();
        let iterable = self.evaluate(&statement["iterable"])?;
        let pairs: Vec<(Value, Value)> = match &iterable {
            Value::List(items) => items
                .iter()
                .enumerate()
                .map(|(i, item)| (Value::Int(i as i64), item.clone()))
                .collect(),
            Value::Map(map) => {
                let mut entries: Vec<(&String, &Value)> = map.iter().collect();
                // CEL orders strings by code point, which is the order of their UTF-8 bytes.
                entries.sort_by(|a, b| a.0.cmp(b.0));
                entries
                    .into_iter()
                    .map(|(k, v)| (Value::String(k.clone()), v.clone()))
                    .collect()
            }
            other => {
                return Err(self.stop(
                    Kind::Error,
                    &statement["iterable"],
                    format!("A `for` goes over a list or a map; this is a {}.", other.type_name()),
                ));
            }
        };
        let is_map = matches!(iterable, Value::Map(_));
        let limit = statement["limit"].as_u64().unwrap_or(0);
        for (iteration, (key, value)) in pairs.into_iter().enumerate() {
            self.iterate(iteration as u64, limit, statement)?;
            let mut locals = Vec::new();
            match &names[..] {
                [one] => locals.push((one.clone(), if is_map { key } else { value })),
                [first, second, ..] => {
                    locals.push((first.clone(), key));
                    locals.push((second.clone(), value));
                }
                [] => {}
            }
            match self.looped(locals, &statement["body"])? {
                Flow::Break => break,
                Flow::Return(value) => return Ok(Flow::Return(value)),
                Flow::Next | Flow::Continue => {}
            }
        }
        Ok(Flow::Next)
    }

    fn while_loop(&mut self, statement: &Json) -> Done<Flow> {
        let limit = statement["limit"].as_u64().unwrap_or(0);
        let mut iteration = 0;
        while self.condition(&statement["condition"])? {
            self.iterate(iteration, limit, statement)?;
            iteration += 1;
            match self.looped(Vec::new(), &statement["body"])? {
                Flow::Break => break,
                Flow::Return(value) => return Ok(Flow::Return(value)),
                Flow::Next | Flow::Continue => {}
            }
        }
        Ok(Flow::Next)
    }

    /// The start of an iteration: one step, and the loop's limit, the most iterations it may make (ATD §12.2).
    fn iterate(&self, iteration: u64, limit: u64, statement: &Json) -> Done<()> {
        if iteration >= limit {
            let loop_kind = statement["kind"].as_str().unwrap_or("loop");
            return Err(self.stop(
                Kind::Limit,
                statement,
                format!(
                    "A `{loop_kind}` in `{}` reached its limit of {limit} iterations.",
                    self.function
                ),
            ));
        }
        self.charge(statement)
    }

    fn looped(&mut self, locals: Vec<(String, Value)>, body: &Json) -> Done<Flow> {
        self.scopes.push(
            locals
                .into_iter()
                .map(|(name, value)| Local {
                    name,
                    value,
                    mutable: false,
                })
                .collect(),
        );
        let flow = self.branch(body);
        self.scopes.pop();
        flow
    }

    /// An action: its expressions evaluated in the order they are written, handed to the host.
    fn act(&mut self, kind: &str, statement: &Json) -> Done<Flow> {
        let mut values = Map::default();
        let mut path = Vec::new();
        if kind == "set" || kind == "unset" {
            let segments: Vec<String> = statement["path"].as_array().into_iter().flatten().map(text).collect();
            let (target, rest) = match &segments[..] {
                [attribute] => ("self".to_owned(), vec![attribute.clone()]),
                [first, rest @ ..] => (first.clone(), rest.to_vec()),
                [] => ("self".to_owned(), Vec::new()),
            };
            let target = self
                .get(&target)
                .ok_or_else(|| self.stop(Kind::Error, statement, format!("`{target}` has no value here.")))?;
            values.insert("target", target);
            path = rest;
            if kind == "set" {
                values.insert("value", self.evaluate(&statement["value"])?);
            }
        } else {
            for (key, field) in written_order(statement) {
                let value = match key {
                    "assignments" | "settings" => {
                        let mut map = Map::default();
                        for entry in field.as_array().into_iter().flatten() {
                            map.insert(
                                entry["key"].as_str().unwrap_or_default(),
                                self.argument(&entry["value"])?,
                            );
                        }
                        Value::map(map)
                    }
                    "args" => {
                        let mut items = Vec::new();
                        for entry in field.as_array().into_iter().flatten() {
                            items.push(self.argument(&entry["value"])?);
                        }
                        Value::list(items)
                    }
                    _ => self.evaluate(field)?,
                };
                values.insert(key, value);
            }
        }
        let action = Action {
            kind,
            node: statement,
            values,
            path,
        };
        let performed = self.run.host.borrow_mut().perform(&action);
        match performed.map_err(|e| self.stop(Kind::Error, statement, e))? {
            Performed::Done(bound) => {
                if let Some(name) = statement.get("as") {
                    self.declare(text(name), bound.unwrap_or_default(), false);
                }
            }
            Performed::Run { name, body, bindings } => {
                self.inner.run_body(self.run, &name, &body, bindings)?;
            }
        }
        Ok(Flow::Next)
    }

    /// A setting's or an assignment's value: an expression, or a literal of the tree.
    fn argument(&self, node: &Json) -> Done<Value> {
        if node.get("cel").is_some() {
            return self.evaluate(node);
        }
        Ok(literal(node.get("literal").unwrap_or(node)))
    }

    fn debug(&self, statement: &Json) -> Done<()> {
        let mut hook = self.run.hook.borrow_mut();
        let Some(hook) = hook.as_mut() else {
            return Ok(());
        };
        let event = Event {
            function: &self.function,
            statement,
            locals: self
                .scopes
                .iter()
                .flatten()
                .map(|l| (l.name.as_str(), &l.value))
                .collect(),
            depth: self.run.frames.borrow().len(),
            steps: self.run.steps.get(),
        };
        match hook(&event) {
            Control::Continue => Ok(()),
            Control::Stop => Err(self.stop(
                Kind::Debugger,
                statement,
                format!("The debugger stopped `{}`.", self.function),
            )),
        }
    }

    fn declare(&mut self, name: String, value: Value, mutable: bool) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.push(Local { name, value, mutable });
        }
    }

    fn assign(&mut self, name: &str, value: Value, statement: &Json) -> Done<()> {
        let value = self.bounded(value, statement)?;
        let local = self
            .scopes
            .iter_mut()
            .rev()
            .flat_map(|s| s.iter_mut().rev())
            .find(|l| l.name == name);
        match local {
            Some(local) if local.mutable => {
                local.value = value;
                Ok(())
            }
            Some(_) => Err(self.stop(Kind::Error, statement, format!("`{name}` is not a `var`."))),
            None => Err(self.stop(Kind::Error, statement, format!("`{name}` is not a local here."))),
        }
    }

    /// A value within ATD §12.5's length, or a budget overrun.
    fn bounded(&self, value: Value, node: &Json) -> Done<Value> {
        let length = match &value {
            Value::List(items) => items.len(),
            Value::Map(map) => map.len(),
            Value::String(s) if s.len() > LENGTH => s.chars().count(),
            _ => 0,
        };
        if length > LENGTH {
            return Err(self.stop(
                Kind::Budget,
                node,
                format!(
                    "A value in `{}` grew past {LENGTH} elements or code points.",
                    self.function
                ),
            ));
        }
        Ok(value)
    }

    fn get(&self, name: &str) -> Option<Value> {
        let local = self
            .scopes
            .iter()
            .rev()
            .flat_map(|s| s.iter().rev())
            .find(|l| l.name == name);
        match local {
            Some(local) => Some(local.value.clone()),
            None => self.run.host.borrow().variable(name),
        }
    }

    fn names(&self) -> Vec<String> {
        self.scopes.iter().flatten().map(|l| l.name.clone()).collect()
    }

    fn stop(&self, kind: Kind, node: &Json, message: String) -> Stop {
        Stop::new(kind, &self.function, node, message)
    }
}

impl Drop for Frame<'_> {
    fn drop(&mut self) {
        self.run.frames.borrow_mut().pop();
    }
}

impl Variables for Frame<'_> {
    fn get(&self, name: &str) -> Option<Value> {
        Frame::get(self, name)
    }
}

/// The expression-bearing fields of an action, in the order they are written.
fn written_order(statement: &Json) -> Vec<(&str, &Json)> {
    let mut fields: Vec<(usize, &str, &Json)> = statement
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(key, field)| match key.as_str() {
            "assignments" | "settings" | "args" => field.is_array(),
            _ => field.get("cel").is_some(),
        })
        .map(|(key, field)| (start(field), key.as_str(), field))
        .collect();
    fields.sort_by_key(|(start, _, _)| *start);
    fields.into_iter().map(|(_, key, field)| (key, field)).collect()
}

/// Where a field starts in its file: its own span, or its first entry's.
fn start(field: &Json) -> usize {
    let own = field["span"]["start"].as_u64();
    let first = || {
        field
            .as_array()
            .and_then(|items| items.first())
            .and_then(|item| item["span"]["start"].as_u64())
    };
    own.or_else(first).unwrap_or(usize::MAX as u64) as usize
}

/// A literal of the tree as a CEL value.
fn literal(node: &Json) -> Value {
    match node {
        Json::Null => Value::Null,
        Json::Bool(b) => Value::Bool(*b),
        Json::Number(n) => n
            .as_i64()
            .map_or_else(|| Value::Double(n.as_f64().unwrap_or(0.0)), Value::Int),
        Json::String(s) => Value::String(s.clone()),
        Json::Array(items) => Value::list(items.iter().map(literal).collect()),
        Json::Object(object) => match object.get("literal") {
            Some(inner) => literal(inner),
            None => Value::map(object.iter().map(|(k, v)| (k.clone(), literal(v))).collect()),
        },
    }
}
