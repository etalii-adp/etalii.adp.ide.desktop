//! The history's behaviour, as the standalone host's `HistoryStack` tests state it (etalii.adp spec 030, T015).

use adp_history::{Applied, Command, Composite, History, Outcome, Refusal};

/// A model of one list of words; each test command appends, removes or refuses.
type Words = Vec<String>;

struct Append(&'static str);
struct RemoveLast(&'static str);
struct Refuse;
struct Final;

impl Command<Words> for Append {
    fn label(&self) -> String {
        format!("Add {}", self.0)
    }
    fn apply(&self, model: &mut Words) -> Result<Applied<Words>, Refusal> {
        model.push(self.0.to_owned());
        Ok(Applied::undoable(RemoveLast(self.0)))
    }
}

impl Command<Words> for RemoveLast {
    fn label(&self) -> String {
        format!("Remove {}", self.0)
    }
    fn apply(&self, model: &mut Words) -> Result<Applied<Words>, Refusal> {
        match model.last() {
            Some(last) if last == self.0 => {
                model.pop();
                Ok(Applied::undoable(Append(self.0)))
            }
            _ => Err(Refusal::new(format!("{} is not the last word.", self.0))),
        }
    }
}

impl Command<Words> for Refuse {
    fn label(&self) -> String {
        "Refuse".into()
    }
    fn apply(&self, _: &mut Words) -> Result<Applied<Words>, Refusal> {
        Err(Refusal::new("No."))
    }
}

impl Command<Words> for Final {
    fn label(&self) -> String {
        "Final".into()
    }
    fn apply(&self, model: &mut Words) -> Result<Applied<Words>, Refusal> {
        model.push("final".into());
        Ok(Applied::final_change().with_warning("This cannot be undone."))
    }
}

#[test]
fn undo_and_redo_walk_a_change_back_and_forward() {
    let (mut model, mut history) = (Words::new(), History::default());
    assert!(history.execute(&mut model, &Append("a")).is_done());
    assert!(history.execute(&mut model, &Append("b")).is_done());
    assert_eq!(
        history.undo(&mut model),
        Outcome::Done {
            label: "Add b".into(),
            warning: String::new()
        }
    );
    assert_eq!(model, ["a"]);
    assert!(history.redo(&mut model).is_done());
    assert_eq!(model, ["a", "b"]);
    let availability = history.availability();
    assert!(availability.can_undo && !availability.can_redo);
    assert_eq!(availability.undo_count, 2);
}

#[test]
fn a_new_change_discards_the_redo_side() {
    let (mut model, mut history) = (Words::new(), History::default());
    history.execute(&mut model, &Append("a"));
    history.undo(&mut model);
    history.execute(&mut model, &Append("b"));
    assert!(!history.availability().can_redo);
    assert_eq!(
        history.redo(&mut model),
        Outcome::Refused(Refusal::new("There is nothing to redo."))
    );
}

#[test]
fn a_refused_command_changes_nothing_and_keeps_redo() {
    let (mut model, mut history) = (Words::new(), History::default());
    history.execute(&mut model, &Append("a"));
    history.undo(&mut model);
    assert_eq!(
        history.execute(&mut model, &Refuse),
        Outcome::Refused(Refusal::new("No."))
    );
    assert!(history.availability().can_redo);
    assert!(model.is_empty());
}

#[test]
fn a_change_without_inverse_is_not_recorded_but_its_warning_is_reported() {
    let (mut model, mut history) = (Words::new(), History::default());
    assert_eq!(
        history.execute(&mut model, &Final),
        Outcome::Done {
            label: "Final".into(),
            warning: "This cannot be undone.".into()
        }
    );
    assert!(!history.availability().can_undo);
}

#[test]
fn a_composite_is_one_step_and_undoes_in_reverse_order() {
    let (mut model, mut history) = (Words::new(), History::default());
    let both = Composite::new("Add two", vec![Box::new(Append("a")), Box::new(Append("b"))]);
    history.execute(&mut model, &both);
    assert_eq!(model, ["a", "b"]);
    assert_eq!(history.availability().undo_count, 1);
    history.undo(&mut model);
    assert!(model.is_empty());
    history.redo(&mut model);
    assert_eq!(model, ["a", "b"]);
}

#[test]
fn a_composite_with_a_refusing_part_leaves_the_model_as_it_was() {
    let (mut model, mut history) = (Words::new(), History::default());
    let broken = Composite::new("Add and fail", vec![Box::new(Append("a")), Box::new(Refuse)]);
    assert_eq!(
        history.execute(&mut model, &broken),
        Outcome::Refused(Refusal::new("No."))
    );
    assert!(model.is_empty());
    assert!(!history.availability().can_undo);
}

#[test]
fn undo_stops_at_a_change_from_outside_and_says_why() {
    let (mut model, mut history) = (Words::new(), History::default());
    history.execute(&mut model, &Append("a"));
    history.mark_changed_outside(None);
    history.execute(&mut model, &Append("b"));
    assert!(history.undo(&mut model).is_done());
    assert_eq!(
        history.undo(&mut model),
        Outcome::Refused(Refusal::new(History::<Words>::CHANGED_OUTSIDE))
    );
    assert_eq!(model, ["a"]);
    assert!(!history.availability().can_undo);
}

#[test]
fn the_oldest_change_falls_off_past_the_capacity() {
    let (mut model, mut history) = (Words::new(), History::with_capacity(2));
    for word in ["a", "b", "c"] {
        history.execute(&mut model, &Append(word));
    }
    assert_eq!(history.availability().undo_count, 2);
    history.undo(&mut model);
    history.undo(&mut model);
    assert_eq!(
        history.undo(&mut model),
        Outcome::Refused(Refusal::new("There is nothing to undo."))
    );
    assert_eq!(model, ["a"]);
}

#[test]
fn a_change_applied_already_is_recorded_as_one_step() {
    let (mut model, mut history) = (Words::new(), History::default());
    history.execute(&mut model, &Append("a"));
    history.undo(&mut model);
    assert_eq!(history.availability().redo_count, 1);

    // A script's transaction: its commands applied one by one, their inverses kept, recorded at the end.
    let mut inverses: Vec<Box<dyn Command<Words>>> = Vec::new();
    for word in ["b", "c"] {
        inverses.push(Append(word).apply(&mut model).unwrap().inverse.unwrap());
    }
    inverses.reverse();
    let applied = Applied {
        inverse: Some(Box::new(Composite::new("Add b and c", inverses)) as Box<dyn Command<Words>>),
        warning: String::new(),
    };
    assert_eq!(
        history.record("Add b and c", applied),
        Outcome::Done {
            label: "Add b and c".into(),
            warning: String::new()
        }
    );
    let availability = history.availability();
    assert_eq!(
        (availability.undo_count, availability.redo_count),
        (1, 0),
        "the redo side is gone"
    );
    assert!(history.undo(&mut model).is_done());
    assert!(model.is_empty());
    assert!(history.redo(&mut model).is_done());
    assert_eq!(model, ["b", "c"]);
}
