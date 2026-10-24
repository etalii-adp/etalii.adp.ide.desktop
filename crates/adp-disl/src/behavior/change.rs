//! What running an action list gives (DISL section 9.4; the standalone host's `DislChange`, `HostAction` and
//! `DislTransaction`): the model changes, in the metamodel's terms, that a host writes - through the FBL writer for
//! an FBL body - the actions the model does not hold that it hands back, and the refusal that rolled the
//! transaction back, when one did. Values are CEL values, as the actions computed them.

use adp_cel::{Map, Value};

/// One model change an action list made.
#[derive(Debug, Clone)]
pub enum DislChange {
    /// A node of `type_name` created with `id` (`create`); `at` is the domain point it was created at, when the
    /// action said one: placement is the host's. `index` is where among its parent's children it goes, when the
    /// action placed it `after` or `before` a sibling; none puts it last.
    Create {
        type_name: String,
        id: String,
        attributes: Map,
        parent_id: Option<String>,
        at: Value,
        index: Option<i64>,
    },
    /// A relation of `type_name` created with `id` from `source_id` to `target_id` (`connect`), with the attributes
    /// its action gave.
    Connect {
        type_name: String,
        id: String,
        source_id: String,
        target_id: String,
        attributes: Map,
    },
    /// Attributes of an element of `type_name` assigned (`set`), or forgotten where the value is null (`unset`).
    Set {
        element_id: String,
        type_name: String,
        attributes: Map,
    },
    /// An element deleted (`delete`, or a deletion policy's cascade).
    Remove { element_id: String },
    /// A node moved under another parent, or to the top level (`reparent`); `index` is where among the new parent's
    /// children it went, counted before the move with the node itself included; negative for last.
    Reparent {
        element_id: String,
        parent_id: Option<String>,
        index: i64,
    },
    /// A node's type changed to `type_name` (`behavior.retype`, section 9.5), with the values its
    /// `attributeMapping` gives; the attributes both types declare are kept by whoever writes it.
    Retype {
        element_id: String,
        type_name: String,
        attributes: Map,
    },
}

/// An action the model does not hold and the interpreter hands back to the host (section 9.4).
#[derive(Debug, Clone)]
pub enum HostAction {
    /// `layout`: run the algorithm (section 10) over the diagram.
    Layout { algorithm: String },
    /// `plugin`: the plugin action with its evaluated arguments.
    Plugin { name: String, arguments: Map },
    /// `select`: the ids of what is to be selected.
    Select { element_ids: Vec<String> },
    /// `editLabel`: open the inline editor on the element's label, or its first.
    EditLabel { element_id: String, label: Option<String> },
}

/// What running an action list gave: the model changes in order, the actions handed back, and the refusal that
/// rolled the transaction back, when one did (`abort`, an unavailable operation, an action this runtime cannot run,
/// or an expression that failed).
#[derive(Debug, Clone, Default)]
pub struct DislTransaction {
    pub changes: Vec<DislChange>,
    pub host_actions: Vec<HostAction>,
    pub refusal: Option<String>,
}

impl DislTransaction {
    /// Whether the transaction stands.
    pub fn was_applied(&self) -> bool {
        self.refusal.is_none()
    }

    /// A transaction refused with `reason`, which changes nothing.
    pub fn refused(reason: impl Into<String>) -> Self {
        DislTransaction {
            changes: Vec::new(),
            host_actions: Vec::new(),
            refusal: Some(reason.into()),
        }
    }
}
