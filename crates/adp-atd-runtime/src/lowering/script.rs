//! The script inside DISL (ATD §12, research R16): the description's functions in DISL's environment, and the
//! `extension` actions of operations, hooks and fixes whose statements are the script's. Each action the script takes
//! is DISL's own, performed in the same transaction, its evaluated arguments bound as variables.

use std::cell::RefCell;
use std::rc::Rc;

use adp_atd_script::{Action, Host, Performed, Script};
use adp_cel::{Map, Value};
use adp_disl::{DislActionExtension, DislActions};
use serde_json::{Map as JsonMap, Value as Json};

use super::actions::{self, Arguments, Parameters};
use super::values::text;

/// The script of a description, made when DISL's environment is, which its functions compile in.
pub type ScriptCell = Rc<RefCell<Option<Script>>>;

/// Runs the `extension` actions of a lowered specification.
pub struct ScriptActions {
    pub script: ScriptCell,
    pub scripts: JsonMap<String, Json>,
    pub parameters: Parameters,
}

impl DislActionExtension for ScriptActions {
    fn run(&self, name: &str, actions: &mut DislActions<'_, '_>) -> Result<(), String> {
        let body = self
            .scripts
            .get(name)
            .ok_or_else(|| format!("The description has no statements `{name}`."))?;
        let script = self
            .script
            .borrow()
            .clone()
            .ok_or("The description's script is not loaded.")?;
        let mut host = DislHost {
            actions,
            parameters: &self.parameters,
            next: 0,
        };
        let gesture = script.gesture(&mut host);
        gesture
            .run(name.rsplit(':').next().unwrap_or(name), body, Map::default())
            .map_err(|stop| stop.to_string())
    }
}

/// The script's host in a DISL transaction: the names it reads are the action's variables (`self`, `p`, `diagram`),
/// and each action it takes is DISL's.
struct DislHost<'x, 'r, 'a> {
    actions: &'x mut DislActions<'r, 'a>,
    parameters: &'x Parameters,
    next: usize,
}

impl Host for DislHost<'_, '_, '_> {
    fn variable(&self, name: &str) -> Option<Value> {
        self.actions.variable(name)
    }

    fn perform(&mut self, action: &Action<'_>) -> Result<Performed, String> {
        let mut bound = Bound {
            values: &action.values,
            bindings: Vec::new(),
            next: &mut self.next,
        };
        let lowered = actions::action(action.node, self.parameters, &mut bound)
            .ok_or_else(|| format!("This runtime cannot run a `{}` here.", action.kind))?;
        for (name, value) in bound.bindings {
            self.actions.bind(&name, value);
        }
        self.actions.perform(&lowered)?;
        let named = action.node.get("as").map(text);
        Ok(Performed::Done(named.and_then(|n| self.actions.variable(&n))))
    }
}

/// An action's arguments as the script evaluated them, each bound to a variable of its own.
struct Bound<'v> {
    values: &'v Map,
    bindings: Vec<(String, Value)>,
    next: &'v mut usize,
}

impl Arguments for Bound<'_> {
    fn argument(&mut self, key: &str, entry: Option<&str>, _node: &Json) -> Json {
        let value = self.values.get(key).cloned().unwrap_or_default();
        let value = match (entry, &value) {
            (Some(entry), Value::Map(map)) => map.get(entry).cloned().unwrap_or_default(),
            (Some(entry), Value::List(items)) => entry
                .parse::<usize>()
                .ok()
                .and_then(|i| items.get(i).cloned())
                .unwrap_or_default(),
            _ => value,
        };
        *self.next += 1;
        let name = format!("__atd{}", self.next);
        self.bindings.push((name.clone(), value));
        Json::String(name)
    }
}
