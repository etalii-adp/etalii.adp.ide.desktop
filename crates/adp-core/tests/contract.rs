//! The core's side of etalii.adp spec 030 contracts/core-shell.md (task T013): every call returns `Done`,
//! `Refused(reason)` or `Prompt`, and events reach the shell in order, never dropped.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use adp_context::{Action, ActionGroup, Prompt, Reference, SelectMode, SelectionSource, Shortcut};
use adp_core::{ActionProvider, Event, Outcome, ProviderContext, Workspace};
use adp_history::{Applied, Command, Refusal};

/// A provider with three actions over the project's notes: one that adds a note, one that is never available, and one
/// that asks for a name first.
struct Notes {
    asked: Arc<Mutex<Vec<String>>>,
}

struct AddNote(String);
struct RemoveNote(String);

impl Command<adp_core::Model> for AddNote {
    fn label(&self) -> String {
        "Add note".into()
    }
    fn apply(&self, model: &mut adp_core::Model) -> Result<Applied<adp_core::Model>, Refusal> {
        model.notes.push(self.0.clone());
        Ok(Applied::undoable(RemoveNote(self.0.clone())))
    }
}

impl Command<adp_core::Model> for RemoveNote {
    fn label(&self) -> String {
        "Remove note".into()
    }
    fn apply(&self, model: &mut adp_core::Model) -> Result<Applied<adp_core::Model>, Refusal> {
        model.notes.retain(|n| *n != self.0);
        Ok(Applied::undoable(AddNote(self.0.clone())))
    }
}

impl ActionProvider for Notes {
    fn actions(&self, context: &ProviderContext) -> Vec<ActionGroup> {
        let mut add = Action::new("notes.add", "Add note").with_shortcut(Shortcut::primary("n"));
        if context.selection.is_empty() {
            add = add.unavailable("Select something to add a note to.");
        }
        vec![ActionGroup {
            actions: vec![
                add,
                Action::new("notes.name", "Name note…"),
                Action::new("notes.never", "Never").unavailable("This action is never available."),
            ],
        }]
    }

    fn execute(&mut self, id: &str, context: &mut ProviderContext) -> Option<Outcome> {
        match id {
            "notes.add" => Some(context.run(&AddNote("note".into()))),
            "notes.name" => Some(Outcome::Prompt(Prompt::Input {
                title: "Name the note".into(),
                icon: String::new(),
                label: "Name".into(),
                initial_value: String::new(),
                confirm_label: "Add".into(),
            })),
            _ => None,
        }
    }

    fn answer(&mut self, id: &str, value: &str, context: &mut ProviderContext) -> Option<Outcome> {
        (id == "notes.name").then(|| {
            self.asked.lock().unwrap().push(value.to_owned());
            context.run(&AddNote(value.to_owned()))
        })
    }
}

fn open() -> (Workspace, std::sync::mpsc::Receiver<Event>, Arc<Mutex<Vec<String>>>) {
    let asked = Arc::new(Mutex::new(Vec::new()));
    let (workspace, events) = Workspace::open("project", vec![Box::new(Notes { asked: asked.clone() })]);
    (workspace, events, asked)
}

fn drain(events: &std::sync::mpsc::Receiver<Event>) -> Vec<Event> {
    let mut all = Vec::new();
    while let Ok(event) = events.recv_timeout(Duration::from_millis(200)) {
        all.push(event);
    }
    all
}

#[test]
fn an_unavailable_action_is_refused_with_its_reason() {
    let (workspace, _events, _) = open();
    assert_eq!(
        workspace.execute("notes.add"),
        Outcome::Refused("Select something to add a note to.".into())
    );
    assert_eq!(
        workspace.execute("notes.never"),
        Outcome::Refused("This action is never available.".into())
    );
    assert_eq!(
        workspace.execute("no.such.action"),
        Outcome::Refused("There is no action no.such.action.".into())
    );
}

#[test]
fn selecting_raises_one_context_event_with_the_actions_as_they_now_are() {
    let (workspace, events, _) = open();
    let selection = workspace.select(
        vec![Reference::Entry("a.md".into())],
        SelectMode::Replace,
        SelectionSource::Explorer,
    );
    assert_eq!(selection.references(), [Reference::Entry("a.md".into())]);
    // Selecting the same again changes nothing and raises nothing.
    workspace.select(
        vec![Reference::Entry("a.md".into())],
        SelectMode::Replace,
        SelectionSource::Explorer,
    );
    let context_events: Vec<_> = drain(&events)
        .into_iter()
        .filter(|e| matches!(e, Event::ContextChanged { .. }))
        .collect();
    assert_eq!(context_events.len(), 1);
    let Event::ContextChanged { actions, .. } = &context_events[0] else {
        unreachable!()
    };
    assert!(actions[0].actions[0].is_available());
}

#[test]
fn an_action_is_done_and_undo_and_redo_walk_it() {
    let (workspace, _events, _) = open();
    workspace.select(vec![Reference::Project], SelectMode::Replace, SelectionSource::Explorer);
    assert_eq!(workspace.execute("notes.add"), Outcome::Done);
    assert_eq!(workspace.model().notes, ["note"]);
    assert_eq!(workspace.undo(), Outcome::Done);
    assert!(workspace.model().notes.is_empty());
    assert_eq!(workspace.redo(), Outcome::Done);
    assert_eq!(workspace.model().notes, ["note"]);
    assert_eq!(workspace.redo(), Outcome::Refused("There is nothing to redo.".into()));
}

#[test]
fn a_prompt_is_answered_or_cancelled() {
    let (workspace, events, asked) = open();
    let Outcome::Prompt(_) = workspace.execute("notes.name") else {
        panic!("expected a prompt")
    };
    assert_eq!(workspace.answer("Groceries"), Outcome::Done);
    assert_eq!(*asked.lock().unwrap(), ["Groceries"]);
    assert_eq!(workspace.model().notes, ["Groceries"]);
    assert_eq!(
        workspace.answer("Again"),
        Outcome::Refused("Nothing is being asked.".into())
    );
    let Outcome::Prompt(_) = workspace.execute("notes.name") else {
        panic!("expected a prompt")
    };
    assert_eq!(workspace.cancel(), Outcome::Done);
    assert_eq!(workspace.model().notes, ["Groceries"]);
    assert!(drain(&events).iter().any(|e| matches!(e, Event::PromptRequested(_))));
}

#[test]
fn events_arrive_in_the_order_the_calls_were_made() {
    let (workspace, events, _) = open();
    for path in ["a", "b", "c"] {
        workspace.select(
            vec![Reference::Entry(path.into())],
            SelectMode::Replace,
            SelectionSource::Explorer,
        );
    }
    let selected: Vec<String> = drain(&events)
        .into_iter()
        .filter_map(|e| match e {
            Event::ContextChanged { selection, .. } => match selection.primary() {
                Some(Reference::Entry(path)) => Some(path.clone()),
                _ => None,
            },
            _ => None,
        })
        .collect();
    assert_eq!(selected, ["a", "b", "c"]);
}

#[test]
fn a_change_from_outside_ends_the_history() {
    let (workspace, _events, _) = open();
    workspace.select(vec![Reference::Project], SelectMode::Replace, SelectionSource::Explorer);
    workspace.execute("notes.add");
    workspace.changed_outside();
    assert!(matches!(workspace.undo(), Outcome::Refused(reason) if reason.contains("changed outside ADP")));
}
