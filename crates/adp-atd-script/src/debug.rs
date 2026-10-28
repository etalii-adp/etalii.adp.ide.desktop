//! The debugger (etalii.adp spec 035, contracts/debugger.md, research R21): breakpoints on the lines of a
//! description's statements, with an optional CEL condition; continue, step over, into and out, pause and stop; and at
//! each pause the call stack and the scopes of its frames.
//!
//! A gesture under the debugger runs on a worker thread ([`Session::start`]): its script, host and transaction are made
//! there, and a pause parks the thread on a channel, so the shell that holds the [`Session`] stays responsive. What a
//! pause shows is sent as text ([`Variable`]), since the script's values do not leave the thread they were made on.

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread::JoinHandle;
use std::time::Duration;

use adp_atd_syntax::Span;
use adp_cel::{Environment, Value};
use serde_json::Value as Json;

use crate::host::{Control, Event};
use crate::stop::{Kind, Stop};
use crate::{FRAMES, Gesture};

/// The most items of a list or entries of a map a variable expands to.
const CHILDREN: usize = 1_000;

/// A breakpoint on a line of a script function, an operation or a hook: `verified` once the line has a statement
/// ([`verify`]); with a condition, it stops only where the condition, a CEL expression over the frame's names, holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Breakpoint {
    pub file: String,
    pub line: usize,
    pub condition: Option<String>,
    pub verified: bool,
}

impl Breakpoint {
    pub fn new(file: impl Into<String>, line: usize) -> Self {
        Breakpoint {
            file: file.into(),
            line,
            condition: None,
            verified: false,
        }
    }

    /// This breakpoint, stopping only where `condition` holds.
    pub fn when(mut self, condition: impl Into<String>) -> Self {
        self.condition = Some(condition.into());
        self
    }
}

/// The lines of the statements in a checked tree (a description's, or one of its functions, operations or hooks), by
/// file: the lines a breakpoint can be verified on.
pub fn statement_lines(tree: &Json) -> BTreeSet<(String, usize)> {
    let mut lines = BTreeSet::new();
    collect_statements(tree, &mut lines);
    lines
}

fn collect_statements(node: &Json, lines: &mut BTreeSet<(String, usize)>) {
    match node {
        Json::Object(object) => {
            for (key, value) in object {
                if matches!(key.as_str(), "body" | "then" | "else") {
                    let statements = match value {
                        Json::Array(items) => items.iter().collect(),
                        Json::Object(_) => vec![value],
                        _ => Vec::new(),
                    };
                    for statement in statements {
                        if let Some(span) = Span::of(statement) {
                            lines.insert((span.file, span.line));
                        }
                    }
                }
                collect_statements(value, lines);
            }
        }
        Json::Array(items) => items.iter().for_each(|i| collect_statements(i, lines)),
        _ => {}
    }
}

/// Marks each breakpoint verified when its line has a statement among `lines`.
pub fn verify(breakpoints: &mut [Breakpoint], lines: &BTreeSet<(String, usize)>) {
    for breakpoint in breakpoints {
        breakpoint.verified = lines.contains(&(breakpoint.file.clone(), breakpoint.line));
    }
}

/// What the person asks of a gesture under the debugger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugControl {
    Continue,
    StepOver,
    StepInto,
    StepOut,
    Pause,
    Stop,
}

/// Why a gesture paused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseReason {
    Breakpoint,
    Step,
    Entry,
    /// The person asked to pause while it ran.
    Pause,
}

/// Why a gesture was stopped: its transaction is discarded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    User,
    Budget,
    Recursion,
}

/// What a gesture under the debugger tells the shell.
#[derive(Debug, Clone, PartialEq)]
pub enum DebugEvent {
    /// The gesture is parked: its call stack, innermost first, and the scopes of every frame. The gesture's elements,
    /// transaction and budget are given once, with the innermost frame.
    Paused {
        reason: PauseReason,
        frames: Vec<Frame>,
        scopes: Vec<Scope>,
    },
    Continued,
    /// The gesture ended: its transaction landed (`committed`) or was discarded.
    Finished {
        committed: bool,
    },
    Stopped {
        reason: StopReason,
    },
    /// What the gesture says (`notify`), or why it ended without committing.
    Output {
        text: String,
    },
    /// A breakpoint's condition did not evaluate to a bool, reported once; the breakpoint is treated as not holding.
    ConditionFailed {
        file: String,
        line: usize,
        message: String,
    },
}

/// A frame of the call stack: a function, operation or hook, and the statement it is at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// 0 for the innermost frame, counting outwards.
    pub id: usize,
    pub name: String,
    pub file: String,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeKind {
    Locals,
    Arguments,
    Elements,
    Transaction,
    Budget,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scope {
    pub frame: usize,
    pub kind: ScopeKind,
    pub variables: Vec<Variable>,
}

/// A value as the debugger shows it: a list's items and a map's entries are its children.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variable {
    pub name: String,
    pub value: String,
    pub type_name: String,
    pub children: Vec<Variable>,
}

impl Variable {
    /// A variable that is only text, as an element's attribute or a command of the transaction is shown.
    pub fn text(name: impl Into<String>, value: impl Into<String>) -> Self {
        Variable {
            name: name.into(),
            value: value.into(),
            type_name: String::new(),
            children: Vec::new(),
        }
    }

    /// A value of the script.
    pub fn of(name: impl Into<String>, value: &Value) -> Self {
        let mut children = Vec::new();
        let shown = match value {
            Value::List(items) => {
                children.extend(
                    items
                        .iter()
                        .take(CHILDREN)
                        .enumerate()
                        .map(|(i, item)| Variable::of(format!("[{i}]"), item)),
                );
                format!("list of {}", items.len())
            }
            Value::Map(map) => {
                children.extend(
                    map.iter()
                        .take(CHILDREN)
                        .map(|(key, item)| Variable::of(key.clone(), item)),
                );
                format!("map of {}", map.len())
            }
            Value::Optional(Some(inner)) => {
                children.push(Variable::of("value", inner));
                "optional".to_owned()
            }
            other => other.to_string(),
        };
        Variable {
            name: name.into(),
            value: shown,
            type_name: value.type_name().to_owned(),
            children,
        }
    }
}

/// What the host shows the debugger of the gesture's model: the elements in the working copy, each expanding to its
/// attributes, and the transaction's commands in the order they were applied.
pub trait Inspect {
    fn elements(&self) -> Vec<Variable> {
        Vec::new()
    }

    fn transaction(&self) -> Vec<Variable> {
        Vec::new()
    }
}

/// A host that shows the debugger nothing of its model.
impl Inspect for () {}

/// Where a gesture's output goes, for its host's `notify`.
#[derive(Clone)]
pub struct Output(Sender<DebugEvent>);

impl Output {
    pub fn send(&self, text: impl Into<String>) {
        let _ = self.0.send(DebugEvent::Output { text: text.into() });
    }
}

/// The debugger's side on the worker thread: attached to each gesture it debugs, one after another, its breakpoints and
/// the step it was asked for carried from one to the next.
pub struct Debugger {
    events: Sender<DebugEvent>,
    state: Rc<RefCell<State>>,
}

impl Debugger {
    fn new(
        events: Sender<DebugEvent>,
        controls: Receiver<DebugControl>,
        breakpoints: Vec<Breakpoint>,
        stop_on_entry: bool,
    ) -> Self {
        let state = State {
            events: events.clone(),
            controls,
            breakpoints: armed(breakpoints),
            environment: Environment::empty(),
            allowed: 0,
            mode: if stop_on_entry { Mode::Into } else { Mode::Run },
            entry: stop_on_entry,
            stack: Vec::new(),
            inspector: Box::new(()),
        };
        Debugger {
            events,
            state: Rc::new(RefCell::new(state)),
        }
    }

    /// This debugger with `breakpoints` instead of those it was started with: a host sets them once it has checked the
    /// description and verified them.
    pub fn with_breakpoints(self, breakpoints: Vec<Breakpoint>) -> Self {
        self.state.borrow_mut().breakpoints = armed(breakpoints);
        self
    }

    /// Where the gesture's output goes.
    pub fn output(&self) -> Output {
        Output(self.events.clone())
    }

    /// Debugs `gesture`: its statements stop where a breakpoint or a step says, and `inspector` shows its model.
    pub fn attach(&self, gesture: &Gesture<'_>, inspector: impl Inspect + 'static) {
        {
            let mut state = self.state.borrow_mut();
            state.environment = gesture.inner.environment.clone();
            state.allowed = gesture.inner.options.steps;
            state.inspector = Box::new(inspector);
        }
        let state = Rc::clone(&self.state);
        gesture.on_statement(move |event| state.borrow_mut().statement(event));
    }
}

/// The verified breakpoints, each to report a condition that does not evaluate once.
fn armed(breakpoints: Vec<Breakpoint>) -> Vec<Armed> {
    breakpoints
        .into_iter()
        .filter(|b| b.verified)
        .map(|breakpoint| Armed {
            breakpoint,
            reported: false,
        })
        .collect()
}

struct Armed {
    breakpoint: Breakpoint,
    reported: bool,
}

/// How far the gesture runs before it pauses again.
#[derive(Debug, Clone, Copy)]
enum Mode {
    /// To a breakpoint.
    Run,
    /// To the next statement.
    Into,
    /// To the next statement no deeper than this.
    Over(usize),
    /// To the next statement shallower than this.
    Out(usize),
    /// To the next statement, as the person asked.
    Pause,
}

/// A frame as the debugger last saw it: the statement it is at and its names.
struct Level {
    function: String,
    span: Option<Span>,
    locals: Vec<(String, Value)>,
    arguments: usize,
}

struct State {
    events: Sender<DebugEvent>,
    controls: Receiver<DebugControl>,
    breakpoints: Vec<Armed>,
    environment: Environment,
    allowed: u64,
    mode: Mode,
    entry: bool,
    stack: Vec<Level>,
    inspector: Box<dyn Inspect>,
}

impl State {
    fn statement(&mut self, event: &Event<'_>) -> Control {
        let span = Span::of(event.statement);
        let level = Level {
            function: event.function.to_owned(),
            span: span.clone(),
            locals: event
                .locals
                .iter()
                .map(|(n, v)| ((*n).to_owned(), (*v).clone()))
                .collect(),
            arguments: event.arguments,
        };
        // The frames outside this one keep the statement they called from; a frame that ran no statement (an
        // expression's) is shown without one.
        self.stack.truncate(event.depth.saturating_sub(1));
        while self.stack.len() + 1 < event.depth {
            self.stack.push(Level {
                function: "(expression)".to_owned(),
                span: None,
                locals: Vec::new(),
                arguments: 0,
            });
        }
        self.stack.push(level);

        match self.controls.try_recv() {
            Ok(DebugControl::Stop) | Err(TryRecvError::Disconnected) => return Control::Stop,
            Ok(DebugControl::Pause) => self.mode = Mode::Pause,
            Ok(_) | Err(TryRecvError::Empty) => {}
        }

        let reason = match self.mode {
            Mode::Into if self.entry => Some(PauseReason::Entry),
            Mode::Pause => Some(PauseReason::Pause),
            Mode::Into => Some(PauseReason::Step),
            Mode::Over(depth) if event.depth <= depth => Some(PauseReason::Step),
            Mode::Out(depth) if event.depth < depth => Some(PauseReason::Step),
            _ => None,
        };
        let reason = reason.or_else(|| {
            let span = span.as_ref()?;
            self.hits(span, &event.locals).then_some(PauseReason::Breakpoint)
        });
        self.entry = false;
        match reason {
            Some(reason) => self.pause(reason, event),
            None => Control::Continue,
        }
    }

    /// Whether a breakpoint on the statement's line holds.
    fn hits(&mut self, span: &Span, locals: &[(&str, &Value)]) -> bool {
        let mut hit = false;
        for armed in &mut self.breakpoints {
            let breakpoint = &armed.breakpoint;
            if breakpoint.file != span.file || breakpoint.line != span.line {
                continue;
            }
            let Some(condition) = &breakpoint.condition else {
                hit = true;
                continue;
            };
            match holds(&self.environment, condition, locals) {
                Ok(holds) => hit |= holds,
                Err(message) => {
                    if !armed.reported {
                        armed.reported = true;
                        let _ = self.events.send(DebugEvent::ConditionFailed {
                            file: breakpoint.file.clone(),
                            line: breakpoint.line,
                            message,
                        });
                    }
                }
            }
        }
        hit
    }

    /// Parks the gesture until the person says how it goes on.
    fn pause(&mut self, reason: PauseReason, event: &Event<'_>) -> Control {
        let paused = DebugEvent::Paused {
            reason,
            frames: self.frames(),
            scopes: self.scopes(event),
        };
        if self.events.send(paused).is_err() {
            return Control::Stop;
        }
        loop {
            self.mode = match self.controls.recv() {
                Ok(DebugControl::Continue) => Mode::Run,
                Ok(DebugControl::StepOver) => Mode::Over(event.depth),
                Ok(DebugControl::StepInto) => Mode::Into,
                Ok(DebugControl::StepOut) => Mode::Out(event.depth),
                Ok(DebugControl::Pause) => continue,
                Ok(DebugControl::Stop) | Err(_) => return Control::Stop,
            };
            let _ = self.events.send(DebugEvent::Continued);
            return Control::Continue;
        }
    }

    fn frames(&self) -> Vec<Frame> {
        self.stack
            .iter()
            .rev()
            .enumerate()
            .map(|(id, level)| {
                let span = level.span.clone().unwrap_or_default();
                Frame {
                    id,
                    name: level.function.clone(),
                    file: span.file,
                    line: span.line,
                    column: span.column,
                }
            })
            .collect()
    }

    fn scopes(&self, event: &Event<'_>) -> Vec<Scope> {
        let mut scopes = Vec::new();
        for (id, level) in self.stack.iter().rev().enumerate() {
            let (arguments, locals) = level.locals.split_at(level.arguments.min(level.locals.len()));
            let variables = |names: &[(String, Value)]| names.iter().map(|(n, v)| Variable::of(n.clone(), v)).collect();
            scopes.push(Scope {
                frame: id,
                kind: ScopeKind::Locals,
                variables: variables(locals),
            });
            scopes.push(Scope {
                frame: id,
                kind: ScopeKind::Arguments,
                variables: variables(arguments),
            });
        }
        scopes.push(Scope {
            frame: 0,
            kind: ScopeKind::Elements,
            variables: self.inspector.elements(),
        });
        scopes.push(Scope {
            frame: 0,
            kind: ScopeKind::Transaction,
            variables: self.inspector.transaction(),
        });
        scopes.push(Scope {
            frame: 0,
            kind: ScopeKind::Budget,
            variables: vec![
                Variable::text("steps", format!("{} of {}", event.steps, self.allowed)),
                Variable::text("depth", format!("{} of {FRAMES}", event.depth)),
            ],
        });
        scopes
    }
}

/// Whether a breakpoint's condition holds over the frame's names, under the script's environment; an error, or a
/// value other than a bool, is the reason it does not evaluate.
fn holds(environment: &Environment, condition: &str, locals: &[(&str, &Value)]) -> Result<bool, String> {
    let mut environment = environment.clone();
    environment.declare_variables(locals.iter().map(|(n, _)| *n));
    let program = environment.compile(condition)?;
    let names: HashMap<String, Value> = locals.iter().map(|(n, v)| ((*n).to_owned(), (*v).clone())).collect();
    match program.evaluate(&names)? {
        Value::Bool(holds) => Ok(holds),
        other => Err(format!("The condition is a bool; this one is {}.", other.type_name())),
    }
}

/// A gesture under the debugger, from the shell's side.
pub struct Session {
    events: Receiver<DebugEvent>,
    controls: Sender<DebugControl>,
    worker: Option<JoinHandle<()>>,
}

impl Session {
    /// Runs `gesture` on a worker thread under the debugger: it makes its script, host and transaction there, attaches
    /// the [`Debugger`] it is given to its gesture, and returns how the gesture ended, having landed its transaction
    /// or discarded it. Paused first at its first statement when `stop_on_entry`.
    pub fn start(
        breakpoints: Vec<Breakpoint>,
        stop_on_entry: bool,
        gesture: impl FnOnce(Debugger) -> Result<(), Stop> + Send + 'static,
    ) -> Session {
        let (events, events_in) = mpsc::channel();
        let (controls_out, controls) = mpsc::channel();
        let ended = events.clone();
        let worker = std::thread::spawn(move || {
            let debugger = Debugger::new(events, controls, breakpoints, stop_on_entry);
            // A gesture that panics ends as one that failed, so that the shell waiting on it hears that it ended.
            let ran = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| gesture(debugger)));
            let end = match ran {
                Err(_) => vec![
                    DebugEvent::Output {
                        text: "The gesture failed inside the host.".to_owned(),
                    },
                    DebugEvent::Finished { committed: false },
                ],
                Ok(Ok(())) => vec![DebugEvent::Finished { committed: true }],
                Ok(Err(stop)) => match stop.kind {
                    Kind::Debugger => vec![DebugEvent::Stopped {
                        reason: StopReason::User,
                    }],
                    Kind::Budget => vec![
                        DebugEvent::Output { text: stop.to_string() },
                        DebugEvent::Stopped {
                            reason: StopReason::Budget,
                        },
                    ],
                    Kind::Recursion => vec![
                        DebugEvent::Output { text: stop.to_string() },
                        DebugEvent::Stopped {
                            reason: StopReason::Recursion,
                        },
                    ],
                    Kind::Fail | Kind::Error | Kind::Limit => vec![
                        DebugEvent::Output { text: stop.to_string() },
                        DebugEvent::Finished { committed: false },
                    ],
                },
            };
            for event in end {
                let _ = ended.send(event);
            }
        });
        Session {
            events: events_in,
            controls: controls_out,
            worker: Some(worker),
        }
    }

    pub fn control(&self, control: DebugControl) {
        let _ = self.controls.send(control);
    }

    /// The next event, waiting at most `timeout`.
    pub fn next_event(&self, timeout: Duration) -> Option<DebugEvent> {
        self.events.recv_timeout(timeout).ok()
    }

    /// The next event if there is one, without waiting.
    pub fn try_event(&self) -> Option<DebugEvent> {
        self.events.try_recv().ok()
    }
}

/// A session dropped stops its gesture, which discards its transaction, and waits for the worker to end.
impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.controls.send(DebugControl::Stop);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
