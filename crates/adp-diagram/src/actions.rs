//! One action of a diagram, written once where a tool type would otherwise map it in each place it is reached from
//! (etalii.adp spec 030, FR-015: one list feeds the ribbon, the context menu and the keyboard); ported from the
//! standalone host's `EtAlii.Adp.Diagram.Modules/DiagramAction.cs` and the result records of
//! `DiagramPropertyProvider.cs`, `DocumentDiagramSession.cs` and `DiagramActionProvider.cs` at `8b6ccfc`.
//!
//! A tool type with behaviour the specification cannot state (a module's residue) describes each action once: its
//! id, what it applies to, how it is offered, and the command it makes; the menu, the commit and the property grid
//! ask the same record. Applying and being offered are separate: an action offered only on some subjects still makes
//! its command on another, and the command says why it refuses.

use adp_context::{Action, ActionGroup};

/// What is asked before something is removed with what goes with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovalConfirmation {
    pub title: String,
    /// What else goes, in a sentence.
    pub message: String,
    pub confirm_label: String,
    /// Whether the button is drawn as destructive.
    pub danger: bool,
}

/// Whether an action applies to a subject.
pub type AppliesTo<S> = Box<dyn Fn(&S) -> bool>;
/// The command an action makes on a subject from the value given.
pub type MakeCommand<S, C> = Box<dyn Fn(&S, &str) -> Option<C>>;
/// How an action is offered on a subject.
pub type Offer<S> = Box<dyn Fn(&S) -> Option<Action>>;
/// What is confirmed before an action removes something.
pub type Confirm<S> = Box<dyn Fn(&S) -> Option<RemovalConfirmation>>;
/// Whether a typed value may be committed.
pub type Validate = Box<dyn Fn(&str) -> Result<(), String>>;

/// One action of a diagram on a subject of type `S`, making commands of type `C`.
pub struct DiagramAction<S, C> {
    /// As the shell sends it.
    pub id: String,
    /// Whether the action can run on a subject; one it does not apply to is refused before a command is made.
    pub applies_to: AppliesTo<S>,
    /// The command the action makes on a subject from the value given (empty when it asked for none); none when the
    /// subject no longer holds what it works on.
    pub command: MakeCommand<S, C>,
    /// How the action is offered on a subject, or none where it is not offered.
    pub offer: Offer<S>,
    /// Which of the menu's groups it is offered in; groups keep the order of their first action.
    pub group: String,
    /// What the person confirms before it removes something, or none.
    pub confirm: Confirm<S>,
    /// Whether a typed value may be committed, or the sentence that refuses it.
    pub validate: Validate,
}

impl<S, C> DiagramAction<S, C> {
    /// An action that is never offered, applies to `applies_to` and makes `command`.
    pub fn new(
        id: impl Into<String>,
        applies_to: impl Fn(&S) -> bool + 'static,
        command: impl Fn(&S, &str) -> Option<C> + 'static,
    ) -> Self {
        DiagramAction {
            id: id.into(),
            applies_to: Box::new(applies_to),
            command: Box::new(command),
            offer: Box::new(|_| None),
            group: String::new(),
            confirm: Box::new(|_| None),
            validate: Box::new(|_| Ok(())),
        }
    }

    pub fn offered_as(mut self, group: impl Into<String>, offer: impl Fn(&S) -> Option<Action> + 'static) -> Self {
        self.group = group.into();
        self.offer = Box::new(offer);
        self
    }

    pub fn confirmed_by(mut self, confirm: impl Fn(&S) -> Option<RemovalConfirmation> + 'static) -> Self {
        self.confirm = Box::new(confirm);
        self
    }

    pub fn validated_by(mut self, validate: impl Fn(&str) -> Result<(), String> + 'static) -> Self {
        self.validate = Box::new(validate);
        self
    }
}

/// The actions offered on `subject`, in their groups, groups in the order of their first action.
pub fn offered<S, C>(actions: &[DiagramAction<S, C>], subject: &S) -> Vec<ActionGroup> {
    let mut groups: Vec<(&str, ActionGroup)> = Vec::new();
    for action in actions {
        let Some(offered) = (action.offer)(subject) else {
            continue;
        };
        match groups.iter_mut().find(|(g, _)| *g == action.group) {
            Some((_, group)) => group.actions.push(offered),
            None => groups.push((&action.group, ActionGroup { actions: vec![offered] })),
        }
    }
    groups.into_iter().map(|(_, g)| g).collect()
}

/// The action `id` names where it applies to `subject`; otherwise none.
pub fn applying<'a, S, C>(
    actions: &'a [DiagramAction<S, C>],
    id: &str,
    subject: &S,
) -> Option<&'a DiagramAction<S, C>> {
    actions.iter().find(|a| a.id == id && (a.applies_to)(subject))
}

/// The action `id` names, whatever it applies to; otherwise none.
pub fn named<'a, S, C>(actions: &'a [DiagramAction<S, C>], id: &str) -> Option<&'a DiagramAction<S, C>> {
    actions.iter().find(|a| a.id == id)
}

/// What a new value in the property grid, or a drop on the canvas, is: a command to dispatch, nothing to do, or a
/// refusal (the standalone host's `DiagramPropertyChange` and `DiagramPlacement`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagramChangeOutcome<C> {
    /// Written by the command.
    By(C),
    /// Nothing to write and nothing refused: the value is the one already there.
    Unchanged,
    /// Refused, for the reason.
    Refused(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actions() -> Vec<DiagramAction<i32, String>> {
        vec![
            DiagramAction::new("rename", |_| true, |s: &i32, v| Some(format!("rename {s} to {v}")))
                .offered_as("edit", |_| Some(Action::new("rename", "Rename"))),
            DiagramAction::new("grow", |s: &i32| *s > 0, |s: &i32, _| Some(format!("grow {s}")))
                .offered_as("add", |s: &i32| (*s > 0).then(|| Action::new("grow", "Grow"))),
            DiagramAction::new("remove", |_| true, |s: &i32, _| Some(format!("remove {s}")))
                .offered_as("edit", |_| Some(Action::new("remove", "Remove")))
                .confirmed_by(|_| {
                    Some(RemovalConfirmation {
                        title: "Remove".into(),
                        message: "And its links.".into(),
                        confirm_label: "Remove".into(),
                        danger: true,
                    })
                }),
            DiagramAction::new("stop", |_| true, |_, _| None),
        ]
    }

    #[test]
    fn offered_actions_come_in_their_groups_in_the_order_of_their_first_action() {
        let groups = offered(&actions(), &1);
        let ids: Vec<Vec<&str>> = groups
            .iter()
            .map(|g| g.actions.iter().map(|a| a.id.as_str()).collect())
            .collect();
        assert_eq!(ids, [vec!["rename", "remove"], vec!["grow"]]);
        assert_eq!(offered(&actions(), &0).len(), 1);
    }

    #[test]
    fn an_action_runs_only_where_it_applies_and_is_named_wherever() {
        let actions = actions();
        assert!(applying(&actions, "grow", &0).is_none());
        let grow = applying(&actions, "grow", &2).expect("applies");
        assert_eq!((grow.command)(&2, ""), Some("grow 2".into()));
        assert!(named(&actions, "stop").is_some_and(|a| (a.command)(&1, "").is_none()));
        assert!(named(&actions, "absent").is_none());
        assert!(named(&actions, "remove").is_some_and(|a| (a.confirm)(&1).is_some()));
    }
}
