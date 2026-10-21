//! One window's workspace: the core thread, the calls into it and the events out of it (etalii.adp spec 030,
//! contracts/core-shell.md, task T014).
//!
//! The standalone host's unary calls become methods that block until the core thread answers; its one `Watch`
//! stream per tab becomes one event channel per window, drained by the shell once per frame. The core holds no type
//! of the UI toolkit, and the shell never changes the model except through these methods.

use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use adp_context::{ActionGroup, Prompt, Reference, SelectMode, Selection, SelectionSource};
use adp_history::{Command, History};

/// The project's model, as far as the core holds one yet. Documents, the hierarchy and findings join it with their
/// tasks; until then it carries what the contract's tests need.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Model {
    pub root: String,
    pub notes: Vec<String>,
}

/// The answer to every call: it happened, it was refused with a sentence a person can read, or the person is asked
/// something first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Done,
    Refused(String),
    Prompt(Prompt),
}

/// A message from the core to the shell. The variants for the hierarchy, scenes, tables, documents and findings
/// (contracts/core-shell.md) arrive with the tasks that produce them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// The selection, the actions as they now are, and whether undo and redo can run.
    ContextChanged {
        selection: Selection,
        actions: Vec<ActionGroup>,
        can_undo: bool,
        can_redo: bool,
    },
    /// A prompt the shell must show; it answers with [`Workspace::answer`] or [`Workspace::cancel`].
    PromptRequested(Prompt),
    /// Something to tell the person, such as a warning a successful change carried.
    Notice { message: String },
}

/// What a provider sees and may change while it answers.
pub struct ProviderContext<'a> {
    pub selection: &'a Selection,
    pub model: &'a mut Model,
    history: &'a mut History<Model>,
    notices: &'a mut Vec<String>,
}

impl ProviderContext<'_> {
    /// Runs a command through the project's history, so it can be undone.
    pub fn run(&mut self, command: &dyn Command<Model>) -> Outcome {
        match self.history.execute(self.model, command) {
            adp_history::Outcome::Done { warning, .. } => {
                if !warning.is_empty() {
                    self.notices.push(warning);
                }
                Outcome::Done
            }
            adp_history::Outcome::Refused(refusal) => Outcome::Refused(refusal.0),
        }
    }
}

/// Something that offers actions: the hierarchy, the problems panel, a tool module. Providers are asked in the order
/// they were given; the first that knows an action id runs it.
pub trait ActionProvider: Send {
    fn actions(&self, context: &ProviderContext) -> Vec<ActionGroup>;

    /// Runs the action, or returns `None` when the id is not this provider's.
    fn execute(&mut self, id: &str, context: &mut ProviderContext) -> Option<Outcome>;

    /// Takes the answer to a prompt this provider's action raised.
    fn answer(&mut self, _id: &str, _value: &str, _context: &mut ProviderContext) -> Option<Outcome> {
        None
    }
}

enum Request {
    Select(Vec<Reference>, SelectMode, SelectionSource, Sender<Selection>),
    Execute(String, Sender<Outcome>),
    Answer(String, Sender<Outcome>),
    Cancel(Sender<Outcome>),
    Undo(Sender<Outcome>),
    Redo(Sender<Outcome>),
    ChangedOutside(Option<String>),
    Model(Sender<Model>),
    Stop,
}

/// The shell's handle on one window's core.
pub struct Workspace {
    requests: Sender<Request>,
    thread: Option<JoinHandle<()>>,
}

impl Workspace {
    /// Starts the core thread for the project at `root`, with its action providers, and returns the handle and the
    /// window's event channel.
    pub fn open(root: impl Into<String>, providers: Vec<Box<dyn ActionProvider>>) -> (Self, Receiver<Event>) {
        let (requests, inbox) = mpsc::channel();
        let (events, outbox) = mpsc::channel();
        let mut core = Core {
            model: Model {
                root: root.into(),
                notes: Vec::new(),
            },
            selection: Selection::default(),
            history: History::default(),
            providers,
            pending: None,
            events,
        };
        let thread = thread::Builder::new()
            .name("adp-core".into())
            .spawn(move || core.run(inbox))
            .expect("the core thread starts");
        (
            Self {
                requests,
                thread: Some(thread),
            },
            outbox,
        )
    }

    pub fn select(&self, references: Vec<Reference>, mode: SelectMode, source: SelectionSource) -> Selection {
        self.call(|reply| Request::Select(references, mode, source, reply))
    }

    pub fn execute(&self, action_id: &str) -> Outcome {
        self.call(|reply| Request::Execute(action_id.to_owned(), reply))
    }

    pub fn answer(&self, value: &str) -> Outcome {
        self.call(|reply| Request::Answer(value.to_owned(), reply))
    }

    pub fn cancel(&self) -> Outcome {
        self.call(Request::Cancel)
    }

    pub fn undo(&self) -> Outcome {
        self.call(Request::Undo)
    }

    pub fn redo(&self) -> Outcome {
        self.call(Request::Redo)
    }

    /// Tells the core that the project's files changed outside ADP, which ends the history.
    pub fn changed_outside(&self) {
        let _ = self.requests.send(Request::ChangedOutside(None));
    }

    /// A copy of the model, for tests and for the shell's first frame.
    pub fn model(&self) -> Model {
        self.call(Request::Model)
    }

    fn call<T>(&self, request: impl FnOnce(Sender<T>) -> Request) -> T {
        let (reply, answer) = mpsc::channel();
        self.requests
            .send(request(reply))
            .expect("the core thread runs while the workspace is open");
        answer.recv().expect("the core thread answers every call")
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = self.requests.send(Request::Stop);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

struct Core {
    model: Model,
    selection: Selection,
    history: History<Model>,
    providers: Vec<Box<dyn ActionProvider>>,
    /// The action whose prompt is open, if any.
    pending: Option<String>,
    events: Sender<Event>,
}

impl Core {
    fn run(&mut self, inbox: Receiver<Request>) {
        for request in inbox {
            match request {
                Request::Select(references, mode, source, reply) => {
                    if self.selection.select(references, mode, source) {
                        self.context_changed();
                    }
                    let _ = reply.send(self.selection.clone());
                }
                Request::Execute(id, reply) => {
                    let outcome = self.execute(&id);
                    let _ = reply.send(outcome);
                }
                Request::Answer(value, reply) => {
                    let outcome = match self.pending.take() {
                        None => Outcome::Refused("Nothing is being asked.".into()),
                        Some(id) => self.answer(&id, &value),
                    };
                    let _ = reply.send(outcome);
                }
                Request::Cancel(reply) => {
                    self.pending = None;
                    let _ = reply.send(Outcome::Done);
                }
                Request::Undo(reply) => {
                    let outcome = convert(self.history.undo(&mut self.model));
                    self.after_change(&outcome);
                    let _ = reply.send(outcome);
                }
                Request::Redo(reply) => {
                    let outcome = convert(self.history.redo(&mut self.model));
                    self.after_change(&outcome);
                    let _ = reply.send(outcome);
                }
                Request::ChangedOutside(reason) => {
                    self.history.mark_changed_outside(reason.as_deref());
                    self.context_changed();
                }
                Request::Model(reply) => {
                    let _ = reply.send(self.model.clone());
                }
                Request::Stop => break,
            }
        }
    }

    fn execute(&mut self, id: &str) -> Outcome {
        let mut notices = Vec::new();
        let actions = {
            let context = ProviderContext {
                selection: &self.selection,
                model: &mut self.model,
                history: &mut self.history,
                notices: &mut notices,
            };
            self.providers
                .iter()
                .flat_map(|p| p.actions(&context))
                .collect::<Vec<_>>()
        };
        match find(&actions, id) {
            None => return Outcome::Refused(format!("There is no action {id}.")),
            Some(action) if !action.is_available() => return Outcome::Refused(action.unavailable_reason.clone()),
            Some(_) => {}
        }
        let mut outcome = Outcome::Refused(format!("There is no action {id}."));
        for provider in &mut self.providers {
            let mut context = ProviderContext {
                selection: &self.selection,
                model: &mut self.model,
                history: &mut self.history,
                notices: &mut notices,
            };
            if let Some(answer) = provider.execute(id, &mut context) {
                outcome = answer;
                break;
            }
        }
        if let Outcome::Prompt(prompt) = &outcome {
            self.pending = Some(id.to_owned());
            let _ = self.events.send(Event::PromptRequested(prompt.clone()));
        }
        self.notices(notices);
        self.after_change(&outcome);
        outcome
    }

    fn answer(&mut self, id: &str, value: &str) -> Outcome {
        let mut notices = Vec::new();
        let mut outcome = Outcome::Refused("Nothing is being asked.".into());
        for provider in &mut self.providers {
            let mut context = ProviderContext {
                selection: &self.selection,
                model: &mut self.model,
                history: &mut self.history,
                notices: &mut notices,
            };
            if let Some(answer) = provider.answer(id, value, &mut context) {
                outcome = answer;
                break;
            }
        }
        self.notices(notices);
        self.after_change(&outcome);
        outcome
    }

    fn notices(&self, notices: Vec<String>) {
        for message in notices {
            let _ = self.events.send(Event::Notice { message });
        }
    }

    fn after_change(&mut self, outcome: &Outcome) {
        if *outcome == Outcome::Done {
            self.context_changed();
        }
    }

    fn context_changed(&mut self) {
        let mut notices = Vec::new();
        let actions = {
            let context = ProviderContext {
                selection: &self.selection,
                model: &mut self.model,
                history: &mut self.history,
                notices: &mut notices,
            };
            self.providers.iter().flat_map(|p| p.actions(&context)).collect()
        };
        let availability = self.history.availability();
        let _ = self.events.send(Event::ContextChanged {
            selection: self.selection.clone(),
            actions,
            can_undo: availability.can_undo,
            can_redo: availability.can_redo,
        });
    }
}

fn convert(outcome: adp_history::Outcome) -> Outcome {
    match outcome {
        adp_history::Outcome::Done { .. } => Outcome::Done,
        adp_history::Outcome::Refused(refusal) => Outcome::Refused(refusal.0),
    }
}

fn find<'a>(groups: &'a [ActionGroup], id: &str) -> Option<&'a adp_context::Action> {
    groups
        .iter()
        .flat_map(|g| &g.actions)
        .find_map(|a| if a.id == id { Some(a) } else { find(&a.items, id) })
}
