//! What a runtime that lowers another language onto DISL adds to it (etalii.adp spec 035, research R16), with no tool
//! type named: functions in the environment every expression compiles in, and an `extension` action whose body the
//! runtime runs - an operation's or a hook's script, which acts through the same actions as the rest of DISL, in the
//! same transaction.
//!
//! An `extension` action is `{ "extension": "<name>" }`: the name is the runtime's own, never CEL. A specification
//! loaded without extensions refuses the action as any action this runtime cannot run.

use std::rc::Rc;

use adp_cel::{Environment, Value};

use super::actions::ActionRunner;
use crate::derivation::evaluation::Vars;
use crate::json::{self, Json};
use crate::loading::specification::DislSpecification;
use crate::model::diagram::DislDiagram;

/// Extends the environment, after DISL's library and the plugins' functions and before the specification's own
/// functions, so that both they and every expression may call what it adds.
pub type DislEnvironmentExtension = Box<dyn FnOnce(Environment) -> Environment>;

/// What a runtime adds to a specification it loads.
#[derive(Default)]
pub struct DislExtensions {
    pub environment: Option<DislEnvironmentExtension>,
    pub actions: Option<Rc<dyn DislActionExtension>>,
}

/// The runtime that runs `extension` actions.
pub trait DislActionExtension {
    /// Runs the extension action `name`, acting through `actions`; an error refuses the transaction with it.
    fn run(&self, name: &str, actions: &mut DislActions<'_, '_>) -> Result<(), String>;
}

/// The transaction an `extension` action runs in: its variables, its working diagram, and the actions it takes.
pub struct DislActions<'r, 'a> {
    runner: &'r mut ActionRunner<'a>,
    variables: &'r mut Vars,
    at: String,
}

impl<'r, 'a> DislActions<'r, 'a> {
    pub(crate) fn new(runner: &'r mut ActionRunner<'a>, variables: &'r mut Vars, at: String) -> Self {
        DislActions { runner, variables, at }
    }

    pub fn specification(&self) -> &DislSpecification {
        self.runner.specification()
    }

    /// The transaction's working diagram, with every action taken so far.
    pub fn diagram(&self) -> &DislDiagram {
        self.runner.diagram()
    }

    /// A variable of the action's context (`self`, `p`, `diagram`), or one an action bound with `as`.
    pub fn variable(&self, name: &str) -> Option<Value> {
        self.variables.get(name).cloned()
    }

    /// Binds a variable for the actions taken after: a value an expression of an action reads by name.
    pub fn bind(&mut self, name: &str, value: Value) {
        self.variables.insert(name.to_owned(), value);
    }

    /// Takes one action (DISL section 9.4), its expressions evaluated with the variables; a refusal is the
    /// transaction's. A name the action binds with `as` is readable with [`DislActions::variable`] after.
    pub fn perform(&mut self, action: &Json) -> Result<(), String> {
        let at = json::pointer(&self.at, "extension");
        if self.runner.run(&Json::Array(vec![action.clone()]), &at, self.variables) {
            Ok(())
        } else {
            Err(self.runner.refusal().unwrap_or_default())
        }
    }
}
