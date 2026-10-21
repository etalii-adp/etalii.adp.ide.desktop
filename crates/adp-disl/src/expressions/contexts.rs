//! The CEL contexts of DISL section 12.3 and the gesture contexts of section 8.4 (the standalone host's
//! `DislContexts`): the variables an expression in each may read. An expression is compiled in exactly one of them,
//! so a variable its context does not bind is refused at load, naming the variables it does.

pub const ELEMENT: &str = "element";
pub const COMPARTMENT_ITEM: &str = "compartmentItem";
pub const SHAPE: &str = "shape";
pub const MARKER: &str = "marker";
pub const HANDLE: &str = "handle";
pub const HANDLE_SNAP: &str = "handleSnap";
pub const HANDLE_WRITE: &str = "handleWrite";
pub const PLACEMENT: &str = "placement";
pub const PLACEMENT_WRITE: &str = "placementWrite";
pub const SNAP: &str = "snap";
pub const CATEGORIES: &str = "categories";
pub const CONSTRAINT: &str = "constraint";
pub const BUILT_IN_MESSAGE: &str = "constraint+detail";
pub const BUILT_IN_REFUSAL: &str = "refusal";
pub const IDENTITY: &str = "identity";
pub const CREATE: &str = "create";
pub const FORM: &str = "form";
pub const HOOK: &str = "hook";
pub const OPERATION: &str = "operation";
pub const DERIVE: &str = "derive";
pub const DERIVE_ITEM: &str = "deriveItem";
pub const FILTER: &str = "filter";
pub const FILTER_OPTIONS: &str = "filterOptions";
pub const LEGEND: &str = "legend";
pub const CHROME: &str = "chrome";
pub const BUDGET: &str = "budget";
pub const CONNECTION: &str = "connection";
pub const SIMULATION: &str = "simulation";
pub const TEMPLATE: &str = "template";
pub const MIGRATION: &str = "migration";
pub const RETYPE: &str = "retype";
pub const LABEL_PARSE: &str = "labelParse";
pub const DATA_TYPE_DISPLAY: &str = "dataTypeDisplay";
pub const GESTURE_CONNECT: &str = "gesture:connect";
pub const GESTURE_CONTAINMENT: &str = "gesture:containment";
pub const GESTURE_CREATE: &str = "gesture:create";
pub const GESTURE_DELETE: &str = "gesture:delete";
pub const GESTURE_PLACEMENT: &str = "gesture:placement";
pub const GESTURE_CHANGE: &str = "gesture:change";
pub const GESTURE_REORDER: &str = "gesture:reorder";

const CONTEXTS: &[(&str, &[&str])] = &[
    (ELEMENT, &["self", "diagram", "env"]),
    (COMPARTMENT_ITEM, &["self", "item", "index", "diagram", "env"]),
    (SHAPE, &["w", "h", "p", "self", "env"]),
    (MARKER, &["w", "h", "p", "self", "env", "sw"]),
    (HANDLE, &["w", "h", "p", "self", "env", "px", "py"]),
    // A handle's snap is a CEL snap rule (section 6.8) in the handle context: the snap rule's value, axis and zoom besides.
    (
        HANDLE_SNAP,
        &[
            "w", "h", "p", "self", "env", "px", "py", "value", "axis", "zoom", "diagram",
        ],
    ),
    (
        HANDLE_WRITE,
        &["self", "diagram", "env", "p", "w", "h", "value", "yValue"],
    ),
    (PLACEMENT, &["self", "diagram", "env", "parent", "axis"]),
    (PLACEMENT_WRITE, &["self", "diagram", "env", "parent", "axis", "value"]),
    (SNAP, &["value", "axis", "zoom", "self", "parent", "diagram"]),
    (CATEGORIES, &["diagram", "env"]),
    (CONSTRAINT, &["self", "diagram", "env"]),
    (BUILT_IN_MESSAGE, &["self", "diagram", "env", "detail"]),
    // A built-in's refusal (section 8.4) sees the gesture's variables plus violation; the union of the gesture kinds a built-in refuses.
    (
        BUILT_IN_REFUSAL,
        &[
            "self",
            "diagram",
            "env",
            "violation",
            "relationType",
            "source",
            "target",
            "sourcePort",
            "targetPort",
            "child",
            "parent",
            "slot",
            "attribute",
            "oldValue",
            "newValue",
            "oldBounds",
            "newBounds",
            "newParent",
            "gesture",
        ],
    ),
    (IDENTITY, &["self", "diagram"]),
    (
        CREATE,
        &[
            "diagram",
            "env",
            "parent",
            "elementType",
            "position",
            "dropTarget",
            "tool",
        ],
    ),
    (FORM, &["self", "value", "diagram", "env"]),
    (HOOK, &["self", "old", "event", "diagram", "env"]),
    (OPERATION, &["self", "selection", "p", "diagram", "env", "position"]),
    (DERIVE, &["diagram", "env"]),
    (DERIVE_ITEM, &["item", "group", "index", "diagram", "env"]),
    (FILTER, &["self", "value", "match", "diagram", "env"]),
    (FILTER_OPTIONS, &["value", "match", "diagram", "env"]),
    (LEGEND, &["self", "diagram", "env"]),
    (CHROME, &["diagram", "env"]),
    (BUDGET, &["self", "index", "diagram", "env"]),
    (
        CONNECTION,
        &["source", "target", "sourceAnchor", "position", "diagram", "env"],
    ),
    (SIMULATION, &["self", "state", "step", "p", "diagram", "env"]),
    (TEMPLATE, &["p", "diagram", "env", "refs"]),
    (MIGRATION, &["element", "value", "document", "from", "to"]),
    // A retype's attribute mapping (section 9.5) is an Expression over the element as it was.
    (RETYPE, &["self", "old", "diagram", "env"]),
    // A label's parse (section 6.12): the element's context with value bound to the entered text.
    (LABEL_PARSE, &["self", "diagram", "env", "value"]),
    (DATA_TYPE_DISPLAY, &["self", "diagram", "env", "value"]),
    (
        GESTURE_CONNECT,
        &[
            "relationType",
            "source",
            "target",
            "sourcePort",
            "targetPort",
            "self",
            "diagram",
            "env",
        ],
    ),
    (GESTURE_CONTAINMENT, &["child", "parent", "slot", "diagram", "env"]),
    (
        GESTURE_CREATE,
        &[
            "elementType",
            "parent",
            "position",
            "dropTarget",
            "tool",
            "diagram",
            "env",
        ],
    ),
    (GESTURE_DELETE, &["self", "selection", "diagram", "env"]),
    (
        GESTURE_PLACEMENT,
        &[
            "self",
            "oldBounds",
            "newBounds",
            "newParent",
            "gesture",
            "diagram",
            "env",
        ],
    ),
    (
        GESTURE_CHANGE,
        &["self", "attribute", "oldValue", "newValue", "diagram", "env"],
    ),
    (
        GESTURE_REORDER,
        &["self", "parent", "oldIndex", "newIndex", "siblings", "diagram", "env"],
    ),
];

/// Every context this runtime knows.
pub fn all() -> impl Iterator<Item = &'static str> {
    CONTEXTS.iter().map(|(c, _)| *c)
}

/// The variables of `context`.
///
/// # Panics
/// When there is no such context: a context is the runtime's own word, never the specification's.
pub fn variables_of(context: &str) -> &'static [&'static str] {
    CONTEXTS
        .iter()
        .find(|(c, _)| *c == context)
        .map(|(_, v)| *v)
        .unwrap_or_else(|| panic!("There is no CEL context '{context}'."))
}

/// The gesture context of a constraint of `kind` (section 8.4), or the constraint context for an invariant.
pub fn of_constraint_kind(kind: Option<&str>) -> &'static str {
    match kind {
        Some("connect") => GESTURE_CONNECT,
        Some("containment") => GESTURE_CONTAINMENT,
        Some("create") => GESTURE_CREATE,
        Some("delete") => GESTURE_DELETE,
        Some("placement") => GESTURE_PLACEMENT,
        Some("change") => GESTURE_CHANGE,
        Some("reorder") => GESTURE_REORDER,
        _ => CONSTRAINT,
    }
}
