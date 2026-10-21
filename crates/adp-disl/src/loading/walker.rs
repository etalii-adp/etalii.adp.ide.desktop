//! Finds every CEL expression of a specification and the context it is evaluated in (DISL sections 2.5 and 12.3;
//! the standalone host's `DislExpressionWalker`), for step 8 of loading (section 14.1).
//!
//! Which strings are CEL is the schema's, written out here: the `cel` member of a Message or Bindable (section 2.5
//! b), the properties typed Expression, every value position of an action but its keywords and names (section 2.5
//! d), and the GeomExprs of shapes (section 2.5 c) that are not numbers or percentages. Which context is the
//! property's, per the table of section 12.3 and the gesture kinds of section 8.4; a CEL string found where this
//! walker names no context is reported, as a warning, and compiled with every variable any context binds. `x-`
//! extensions (section 2.8) and `doc` objects (section 2.4) are not walked, and the `functions` block is compiled
//! over each function's parameters instead. The DISL 0.3 expressions this runtime does not evaluate - a
//! ParentPlacement's candidates and rank, a `rows` layout's extents and a `rowPacked` layout's `rowsCovered` - are
//! walked as literals, so a mistake in one is not reported at load.

use regex::Regex;
use std::sync::LazyLock;

use crate::expressions::contexts;
use crate::json::{self, Json};

/// One CEL expression of a specification: where it is, its source, its context and the names actions bind around
/// it.
#[derive(Debug, Clone)]
pub(crate) struct Site {
    pub pointer: String,
    pub source: String,
    /// Empty where the walker knows no context.
    pub context: String,
    pub bindings: Vec<String>,
}

/// Properties typed Expression wherever they occur (section 2.5 a).
const EXPRESSION_KEYS: [&str; 8] = [
    "when",
    "visible",
    "rule",
    "keep",
    "forEach",
    "enabled",
    "atMaxDepth",
    "members",
];

/// The keywords and names of an action, which are never CEL (section 2.5 d).
const ACTION_NAMES: [&str; 8] = [
    "as",
    "algorithm",
    "unset",
    "call",
    "plugin",
    "severity",
    "form",
    "place",
];

/// The coordinates a shape states as GeomExprs (sections 2.5 c and 6.8).
const GEOMETRY_KEYS: [&str; 13] = ["x", "y", "w", "h", "rx", "ry", "x1", "y1", "x2", "y2", "cx", "cy", "r"];

static PERCENTAGE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^-?\d+(\.\d+)?%$").expect("the percentage pattern compiles"));

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// A bare string is a literal; only a `{cel}` object is CEL (section 2.5 b).
    Literal,
    /// The value is an Expression: a bare string, or the `cel` of an object (section 2.5 a).
    Expression,
    /// Inside an action: every value position is an Expression but keywords and names (section 2.5 d).
    Action,
    /// Inside a shape: coordinates are GeomExprs (section 2.5 c).
    Geometry,
}

/// What a value is walked as: its context, the names bound around it, and whether a bare string in it is CEL.
#[derive(Debug, Clone)]
struct Frame {
    context: Option<&'static str>,
    bindings: Vec<String>,
    mode: Mode,
}

impl Frame {
    fn of(context: &'static str) -> Frame {
        Frame {
            context: Some(context),
            bindings: Vec::new(),
            mode: Mode::Literal,
        }
    }

    fn unknown() -> Frame {
        Frame {
            context: None,
            bindings: Vec::new(),
            mode: Mode::Literal,
        }
    }

    fn within(&self, context: &'static str) -> Frame {
        Frame {
            context: Some(context),
            ..self.clone()
        }
    }

    fn as_mode(&self, mode: Mode) -> Frame {
        Frame { mode, ..self.clone() }
    }

    fn binding<I: IntoIterator<Item = String>>(&self, names: I) -> Frame {
        let mut bindings = self.bindings.clone();
        bindings.extend(names);
        Frame {
            bindings,
            ..self.clone()
        }
    }
}

fn skipped(name: &str) -> bool {
    name.starts_with("x-") || name == "doc"
}

pub(crate) fn sites(root: &Json) -> Vec<Site> {
    let mut sites = Vec::new();
    let Some(object) = root.as_object() else {
        return sites;
    };
    for (name, value) in object {
        if skipped(name)
            || matches!(
                name.as_str(),
                "functions" | "$schema" | "disl" | "dedl" | "imports" | "plugins"
            )
        {
            continue;
        }
        let frame = match name.as_str() {
            "language" => Frame::of(contexts::BUDGET),
            "metamodel" | "notation" | "toolbox" | "behavior" | "layout" | "persistence" | "viewpoints" => {
                Frame::of(contexts::ELEMENT)
            }
            "coordinates" => Frame::of(contexts::PLACEMENT),
            "forms" => Frame::of(contexts::FORM),
            "constraints" => Frame::of(contexts::CONSTRAINT),
            _ => Frame::unknown(),
        };
        let mut walk = Walk {
            sites: &mut sites,
            section: name,
        };
        walk.value(value, &json::pointer("", name), &frame, &[name.clone()]);
    }
    sites
}

struct Walk<'a> {
    sites: &'a mut Vec<Site>,
    section: &'a str,
}

impl Walk<'_> {
    fn value(&mut self, value: &Json, pointer: &str, frame: &Frame, path: &[String]) {
        match value {
            Json::Object(_) => self.object(value, pointer, frame, path),
            Json::Array(items) => {
                for (index, item) in items.iter().enumerate() {
                    self.value(item, &json::pointer_at(pointer, index), frame, path);
                }
            }
            Json::String(text) => {
                if matches!(frame.mode, Mode::Expression | Mode::Action) {
                    self.add(pointer, text, frame);
                }
            }
            _ => {}
        }
    }

    fn object(&mut self, value: &Json, pointer: &str, frame: &Frame, path: &[String]) {
        if let Some(cel) = json::string_of(value, "cel") {
            self.add(&json::pointer(pointer, "cel"), cel, frame);
        }
        let Some(members) = value.as_object() else {
            return;
        };
        for (key, member) in members {
            if skipped(key) || key == "cel" || key == "resultType" {
                continue;
            }
            let Some(next) = self.next(frame, key, member, value, path) else {
                continue;
            };
            let mut deeper = path.to_vec();
            deeper.push(key.clone());
            self.value(member, &json::pointer(pointer, key), &next, &deeper);
        }
    }

    /// The frame a property's value is walked in, or none when nothing in it is CEL.
    fn next(&self, frame: &Frame, key: &str, value: &Json, owner: &Json, path: &[String]) -> Option<Frame> {
        let last = path.last().map(String::as_str).unwrap_or("");
        // Inside an action every value is CEL but the keywords and names.
        if frame.mode == Mode::Action {
            // A layout action's refusals are Messages (section 9.4), not value positions.
            if key == "refusals" && last == "layout" {
                return Some(frame.as_mode(Mode::Literal));
            }
            return if ACTION_NAMES.contains(&key) || (key == "label" && last == "editLabel") {
                None
            } else {
                Some(frame.clone())
            };
        }

        if frame.mode == Mode::Geometry
            && GEOMETRY_KEYS.contains(&key)
            && let Some(text) = value.as_str()
        {
            return if PERCENTAGE.is_match(text) {
                None
            } else {
                Some(frame.as_mode(Mode::Expression))
            };
        }

        let next = match self.section {
            "metamodel" => metamodel(frame, key, value, path),
            "persistence" => persistence(frame, key, path),
            "constraints" => constraints(frame, key, owner, path),
            "toolbox" => toolbox(frame, key, owner, path),
            "forms" => forms(frame, key, value),
            "behavior" => behavior(frame, key, path),
            "notation" | "viewpoints" => notation(frame, key, path),
            "coordinates" => coordinates(frame, key),
            "language" => {
                if matches!(key, "order" | "unit" | "measure") && path.iter().any(|p| p == "budgets") {
                    frame.as_mode(Mode::Expression)
                } else {
                    frame.clone()
                }
            }
            _ => frame.clone(),
        };

        // Action lists, wherever they are: hooks, operations, handle and placement writes, label parses.
        if matches!(key, "actions" | "write") && value.is_array() {
            let context = if key == "write" && path.iter().any(|p| p == "handles") {
                Some(contexts::HANDLE_WRITE)
            } else if key == "write" && path.iter().any(|p| p == "placement") {
                Some(contexts::PLACEMENT_WRITE)
            } else {
                next.context
            };
            let mut framed = next.binding(bound(value)).as_mode(Mode::Action);
            framed.context = context;
            return Some(framed);
        }

        Some(if EXPRESSION_KEYS.contains(&key) {
            next.as_mode(Mode::Expression)
        } else {
            next
        })
    }

    fn add(&mut self, pointer: &str, source: &str, frame: &Frame) {
        self.sites.push(Site {
            pointer: pointer.to_owned(),
            source: source.to_owned(),
            context: frame.context.unwrap_or("").to_owned(),
            bindings: frame.bindings.clone(),
        });
    }
}

fn ends_with(path: &[String], tail: &[&str]) -> bool {
    path.len() >= tail.len() && path[path.len() - tail.len()..].iter().zip(tail).all(|(a, b)| a == b)
}

/// Whether the path's second-to-last element is `name` (C#'s `[.., name, _]`).
fn parent_is(path: &[String], names: &[&str]) -> bool {
    path.len() >= 2 && names.contains(&path[path.len() - 2].as_str())
}

fn metamodel(frame: &Frame, key: &str, value: &Json, path: &[String]) -> Frame {
    // An attribute's derived expression is an Expression in the element context; its CEL default is the create context's.
    if key == "derived" && parent_is(path, &["attributes"]) {
        return frame.within(contexts::ELEMENT).as_mode(Mode::Expression);
    }
    if key == "default" && parent_is(path, &["attributes"]) {
        return frame.within(contexts::CREATE).as_mode(Mode::Literal);
    }
    if key == "display" {
        return frame.within(contexts::DATA_TYPE_DISPLAY).as_mode(Mode::Literal);
    }

    // A derived type (section 4.11): from is the derive context, every other expression the deriveItem context.
    if key == "derived" && parent_is(path, &["types", "relations"]) {
        return if value.is_string() {
            frame.within(contexts::DERIVE).as_mode(Mode::Expression)
        } else {
            frame.within(contexts::DERIVE_ITEM).as_mode(Mode::Literal)
        };
    }
    if ends_with(path, &["derived"]) {
        return match key {
            "from" => frame.within(contexts::DERIVE).as_mode(Mode::Expression),
            "key" | "id" | "source" | "target" | "owner" | "sources" | "parent" | "slot" | "attributes" => {
                frame.within(contexts::DERIVE_ITEM).as_mode(Mode::Expression)
            }
            "reason" => frame.within(contexts::ELEMENT).as_mode(Mode::Literal),
            _ => frame.as_mode(Mode::Literal),
        };
    }

    // Each member of a derived type's attributes is an Expression of the deriveItem context (section 4.11.3).
    if ends_with(path, &["derived", "attributes"]) {
        return frame.within(contexts::DERIVE_ITEM).as_mode(Mode::Expression);
    }

    frame.as_mode(Mode::Literal)
}

fn persistence(frame: &Frame, key: &str, path: &[String]) -> Frame {
    if !path.iter().any(|p| p == "ids") {
        return if key == "migrations" {
            frame.within(contexts::MIGRATION)
        } else {
            frame.as_mode(Mode::Literal)
        };
    }
    match key {
        // A derived id is the identity context's; a cel id is made on creation, the create context's.
        "expression" | "ephemeral" => frame.within(contexts::IDENTITY).as_mode(Mode::Expression),
        "reason" => frame.within(contexts::ELEMENT).as_mode(Mode::Literal),
        "migrations" => frame.within(contexts::MIGRATION),
        _ => frame.as_mode(Mode::Literal),
    }
}

fn constraints(frame: &Frame, key: &str, owner: &Json, path: &[String]) -> Frame {
    if path.len() == 2 && path[1] == "rules" {
        // A rule's own properties: its context follows its kind (section 8.4), item and index bound with forEach.
        let context = contexts::of_constraint_kind(json::string_of(owner, "kind"));
        let mut rule = frame.within(context);
        if json::has(owner, "forEach") && key != "forEach" {
            rule = rule.binding(["item".to_owned(), "index".to_owned()]);
        }
        return match key {
            "target" | "location" | "subject" => rule.as_mode(Mode::Expression),
            // Every other key is literal, fixes included: a fix's when and actions are found by their own keys.
            _ => rule.as_mode(Mode::Literal),
        };
    }
    if path.len() == 3 && path[1] == "builtIn" {
        return match key {
            "message" | "code" => frame.within(contexts::BUILT_IN_MESSAGE).as_mode(Mode::Literal),
            "refusal" => frame.within(contexts::BUILT_IN_REFUSAL).as_mode(Mode::Literal),
            _ => frame.as_mode(Mode::Literal),
        };
    }
    frame.clone()
}

fn toolbox(frame: &Frame, key: &str, owner: &Json, path: &[String]) -> Frame {
    if path.len() >= 2 && path[1] == "contextMenus" {
        if path.len() == 2 {
            // A set's own when, and its entries: the connection context for a pending connection (section 7.3), else the element's.
            let for_connection = json::strings_of(owner, "for").iter().any(|f| f == "connection");
            return frame.within(if for_connection {
                contexts::CONNECTION
            } else {
                contexts::ELEMENT
            });
        }
        if path.last().is_some_and(|l| l == "tools") {
            // An entry generated by forEach sees item (or its as name) and index (section 12.3).
            let entry = if json::has(owner, "forEach") && key != "forEach" {
                match json::string_of(owner, "as") {
                    None => frame.binding(["item".to_owned(), "index".to_owned()]),
                    Some(named) => frame.binding([named.to_owned(), "item".to_owned(), "index".to_owned()]),
                }
            } else {
                frame.clone()
            };
            return if key == "args" {
                entry.as_mode(Mode::Expression)
            } else {
                entry
            };
        }
        return frame.clone();
    }

    if path.len() >= 2 && path[1] == "templates" {
        // A template's fragment (section 7.4): its attribute values and positions are Expressions in the template
        // context, which binds refs; everything else in a template is literal.
        if ends_with(path, &["fragment", "nodes", "attributes"])
            || ends_with(path, &["fragment", "relations", "attributes"])
        {
            return frame.clone();
        }
        return if matches!(key, "attributes" | "at")
            && (ends_with(path, &["fragment", "nodes"]) || ends_with(path, &["fragment", "relations"]))
        {
            frame.within(contexts::TEMPLATE).as_mode(Mode::Expression)
        } else {
            frame.within(contexts::TEMPLATE).as_mode(Mode::Literal)
        };
    }

    match key {
        // A palette tool: its initial values in the create context, everything else in the element context on the diagram.
        "initial" | "refusal" => frame.within(contexts::CREATE).as_mode(Mode::Literal),
        _ => frame.clone(),
    }
}

fn forms(frame: &Frame, key: &str, value: &Json) -> Frame {
    match key {
        "initial" => frame.binding(["position".to_owned()]).as_mode(if value.is_string() {
            Mode::Expression
        } else {
            Mode::Literal
        }),
        "confirm" => frame.binding(["count".to_owned()]),
        "options" if value.is_string() => frame.as_mode(Mode::Expression),
        // A type item (DISL 0.3 section 7.5): its optionLabel sees each option as item, its refusals the chosen newValue.
        "optionLabel" => frame.binding(["item".to_owned()]).as_mode(Mode::Expression),
        "refusals" => frame.binding(["newValue".to_owned()]),
        // A field's parse (DISL 0.3 section 7.5): accepts is an Expression over value; its write is an action list.
        "accepts" => frame.as_mode(Mode::Expression),
        _ => frame.clone(),
    }
}

fn behavior(frame: &Frame, key: &str, path: &[String]) -> Frame {
    if path.len() == 1 {
        return match key {
            "hooks" => frame.within(contexts::HOOK),
            "operations" => frame.within(contexts::OPERATION),
            "deletion" => frame.within(contexts::GESTURE_DELETE),
            "retype" => frame.within(contexts::RETYPE),
            "simulations" => frame.within(contexts::SIMULATION),
            "messages" | "reasons" | "editGate" => frame.within(contexts::ELEMENT),
            _ => frame.clone(),
        };
    }
    match key {
        // A confirmation sees count besides its context (section 9.5); its count is an Expression.
        "confirm" => frame.binding(["count".to_owned()]).as_mode(Mode::Literal),
        "count" if path.last().is_some_and(|l| l == "confirm") => frame.as_mode(Mode::Expression),
        "attributeMapping" => frame.as_mode(Mode::Expression),
        _ => frame.clone(),
    }
}

fn notation(frame: &Frame, key: &str, path: &[String]) -> Frame {
    let contains = |name: &str| path.iter().any(|p| p == name);
    if key == "shapes" {
        return frame.within(contexts::SHAPE).as_mode(Mode::Geometry);
    }
    if key == "markers" {
        return frame.within(contexts::MARKER).as_mode(Mode::Geometry);
    }
    if contains("shapes") || contains("markers") {
        // A handle (section 6.8): its position is geometry, its visibility the element's, its snap a snap rule, its
        // write the handleWrite context.
        if path.last().is_some_and(|l| l == "handles") || parent_is(path, &["handles"]) {
            return match key {
                "visible" => frame.within(contexts::ELEMENT),
                "snap" => frame.within(contexts::HANDLE_SNAP).as_mode(Mode::Literal),
                "label" | "value" | "yValue" => frame.within(contexts::HANDLE).as_mode(Mode::Literal),
                "refusals" => frame.within(contexts::ELEMENT).as_mode(Mode::Literal),
                _ => frame.clone(),
            };
        }
        return frame.clone();
    }
    match key {
        "snapping" => frame.within(contexts::SNAP).as_mode(Mode::Literal),
        "parse" => frame.within(contexts::LABEL_PARSE).as_mode(Mode::Literal),
        "filters" => frame.within(contexts::FILTER).as_mode(Mode::Literal),
        "options" if contains("filters") => frame.within(contexts::FILTER_OPTIONS).as_mode(Mode::Literal),
        "legend" => frame.within(contexts::LEGEND).as_mode(Mode::Literal),
        "notices" | "chrome" | "emptyCanvas" => frame.within(contexts::CHROME).as_mode(Mode::Literal),
        // A compartment (section 6.9): its items, and everything said per item - its text, its icon, its conditions
        // and, since DISL 0.4, its link and its order - see the item.
        "items" | "itemText" | "itemIcon" | "itemConditions" | "itemLink" | "itemOrder" if contains("compartments") => {
            frame.within(contexts::COMPARTMENT_ITEM)
        }
        _ => frame.clone(),
    }
}

fn coordinates(frame: &Frame, key: &str) -> Frame {
    match key {
        "categories" => frame.within(contexts::CATEGORIES).as_mode(Mode::Expression),
        "snapping" | "snap" => frame.within(contexts::SNAP).as_mode(Mode::Literal),
        _ => frame.clone(),
    }
}

/// The names an action list binds (section 9.4): every `let` key and every `as` name in it.
fn bound(actions: &Json) -> Vec<String> {
    let mut names = Vec::new();
    let Some(list) = actions.as_array() else {
        return names;
    };
    for action in list {
        let Some(members) = action.as_object() else {
            continue;
        };
        for (name, value) in members {
            if name == "as"
                && let Some(text) = value.as_str()
            {
                names.push(text.to_owned());
            }
            if name == "let"
                && let Some(lets) = value.as_object()
            {
                names.extend(lets.keys().cloned());
            }
            if matches!(name.as_str(), "actions" | "then" | "else") && value.is_array() {
                names.extend(bound(value));
            }
        }
    }
    names
}
