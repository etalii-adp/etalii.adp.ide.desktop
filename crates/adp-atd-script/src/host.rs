//! What a script asks of the host that runs it: the values of the names it reads besides its locals, and its actions
//! (ATD §12.3), which the host turns into commands of the gesture's transaction.

use adp_cel::{Map, Value};
use serde_json::Value as Json;

/// One action of the script with its arguments evaluated.
#[derive(Debug)]
pub struct Action<'a> {
    /// The action as the tree names it: `create`, `set`, `select`, `call`, `plugin` and so on.
    pub kind: &'a str,
    /// The action's node in the checked tree, for what is not an expression: `type`, `slot`, `form`, `severity`, …
    pub node: &'a Json,
    /// Every argument that is an expression, by its key in the tree (`target`, `parent`, `after`, `value`, …), in the
    /// order they are written; `assignments` and `settings` as maps and `args` as a list. For `set` and `unset`,
    /// `target` is the element the path starts at: `self` when the path is one name.
    pub values: Map,
    /// For `set` and `unset`: the attribute path after the target (`text` in `set self.text = …`).
    pub path: Vec<String>,
}

/// What the host did with an action.
pub enum Performed {
    /// The action is done; the value is what its `as` binds (the element `create` made), if anything.
    Done(Option<Value>),
    /// The action is another script to run in its place, in the same transaction: an operation `call` runs, or a hook.
    /// `bindings` are the names its statements see first (`self`, `p`).
    Run { name: String, body: Json, bindings: Map },
}

/// The host a gesture runs in: the model, the transaction and what the user sees.
pub trait Host {
    /// The value of a name an expression reads that is not a local: `self`, `p`, `diagram`, `env`. It is asked again
    /// after every action, so every expression sees the effects of the actions before it (ATD §12.4).
    fn variable(&self, name: &str) -> Option<Value>;

    /// Performs an action: adds its commands to the gesture's transaction, or changes what the user sees. An error is
    /// a refusal, which stops the call.
    fn perform(&mut self, action: &Action<'_>) -> Result<Performed, String>;
}

/// What the debugger sees before each statement (research R21).
pub struct Event<'a> {
    /// The function, operation or hook the statement is in.
    pub function: &'a str,
    pub statement: &'a Json,
    /// The locals in scope, innermost last.
    pub locals: Vec<(&'a str, &'a Value)>,
    /// The frames on the stack, this one included.
    pub depth: usize,
    /// The steps the gesture has taken.
    pub steps: u64,
}

/// What the debugger answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    Continue,
    Stop,
}
