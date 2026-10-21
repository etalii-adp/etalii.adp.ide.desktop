//! The eleven kinds of node a behavior model has, with the keyword the Markdown carries for each (etalii.adp spec 030,
//! task T074).
//!
//! Ported from the standalone host's `_Model/AbmNodeKinds.cs` at `8b6ccfc`. The keyword is plain English on purpose:
//! the file is read by a model that may never have heard of a selector or a sequence, so "Try in order" and "Do in
//! order" tell it what to do, while the ids keep the behavior tree names. A node's DISL type is the one the
//! specification's `x-abm.types` maps to its kind (`agent-behavior-modelling.dis`): `Do` for `action`, and so on.

/// What a node may hold beneath it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    /// One or more children, run by a rule of its own: in order, as fallbacks, together.
    Composite,
    /// Exactly one child, whose run it wraps: retried, repeated, guarded, approved.
    Decorator,
    /// No children: the node is the work, the question or the hand-off itself.
    Leaf,
}

/// One kind of node: its id, the keyword the Markdown carries, what it may hold, what it does, and its DISL type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kind {
    pub id: &'static str,
    pub keyword: &'static str,
    pub category: Category,
    pub meaning: &'static str,
    /// The specification's type name (`x-abm.types`).
    pub type_name: &'static str,
}

pub const SEQUENCE: &str = "sequence";
pub const FALLBACK: &str = "fallback";
pub const PARALLEL: &str = "parallel";
pub const RETRY: &str = "retry";
pub const REPEAT: &str = "repeat";
pub const GUARD: &str = "guard";
pub const APPROVAL: &str = "approval";
pub const CHECK: &str = "check";
pub const ACTION: &str = "action";
pub const ASK: &str = "ask";
pub const DELEGATE: &str = "delegate";

/// How many attempts a new Retry node allows.
pub const DEFAULT_RETRY_COUNT: i32 = 3;

/// Every kind, in the order the toolbox and the menus list them.
pub const ALL: [Kind; 11] = [
    Kind {
        id: SEQUENCE,
        keyword: "Do in order",
        category: Category::Composite,
        meaning: "runs its children one after another, and fails as soon as one fails",
        type_name: "Sequence",
    },
    Kind {
        id: FALLBACK,
        keyword: "Try in order",
        category: Category::Composite,
        meaning: "tries its children one after another, and succeeds as soon as one succeeds",
        type_name: "Fallback",
    },
    Kind {
        id: PARALLEL,
        keyword: "Do together",
        category: Category::Composite,
        meaning: "runs its children independently, at the same time where you can, and succeeds when all of them succeed",
        type_name: "Parallel",
    },
    Kind {
        id: RETRY,
        keyword: "Retry up to N times",
        category: Category::Decorator,
        meaning: "runs its child again when it fails, at most N times in all",
        type_name: "Retry",
    },
    Kind {
        id: REPEAT,
        keyword: "Repeat until",
        category: Category::Decorator,
        meaning: "runs its child again and again until its condition holds",
        type_name: "RepeatUntil",
    },
    Kind {
        id: GUARD,
        keyword: "Only while",
        category: Category::Decorator,
        meaning: "runs its child only while its condition holds, and abandons it when the condition stops holding",
        type_name: "Guard",
    },
    Kind {
        id: APPROVAL,
        keyword: "Ask approval before",
        category: Category::Decorator,
        meaning: "says what its child is about to do and waits for the user's explicit approval; without it, the node fails",
        type_name: "Approval",
    },
    Kind {
        id: CHECK,
        keyword: "Check",
        category: Category::Leaf,
        meaning: "answers its question from the conversation, your memory or a tool result; it succeeds when the answer is yes, and never acts",
        type_name: "Check",
    },
    Kind {
        id: ACTION,
        keyword: "Do",
        category: Category::Leaf,
        meaning: "carries out one piece of work: a tool call, an answer, an edit",
        type_name: "Do",
    },
    Kind {
        id: ASK,
        keyword: "Ask the user",
        category: Category::Leaf,
        meaning: "asks its question and waits for the answer",
        type_name: "AskUser",
    },
    Kind {
        id: DELEGATE,
        keyword: "Delegate",
        category: Category::Leaf,
        meaning: "hands its task to a sub-agent, or follows the behavior file it links to",
        type_name: "Delegate",
    },
];

/// Whether `id` is one of the eleven.
pub fn is_known(id: &str) -> bool {
    ALL.iter().any(|k| k.id == id)
}

/// The kind with `id`.
///
/// # Panics
///
/// For an id that is not one of the eleven, as the standalone host's dictionary throws.
pub fn of(id: &str) -> &'static Kind {
    ALL.iter()
        .find(|k| k.id == id)
        .unwrap_or_else(|| panic!("'{id}' is not a kind of behavior node"))
}

/// The kind of the specification's type `type_name` (`Do` is `action`); none for a type that is not one of the eleven.
pub fn of_type(type_name: &str) -> Option<&'static Kind> {
    ALL.iter().find(|k| k.type_name == type_name)
}

/// The kind a keyword names, with a Retry's count; none when the keyword is none of them. Read without regard to case
/// or surrounding spaces, and a Retry's count from the keyword itself: "Retry up to 2 times", "Retry up to 1 time".
pub fn from_keyword(keyword: &str) -> Option<(&'static Kind, i32)> {
    let normalized = keyword
        .split(' ')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if let Some(count) = retry_count(&normalized) {
        return Some((of(RETRY), count));
    }
    ALL.iter()
        .find(|k| k.id != RETRY && equal_ignoring_case(k.keyword, &normalized))
        .map(|k| (k, 0))
}

/// The keyword a node of `id` is written with: the canonical one, with a Retry's count and the singular `time` for one.
pub fn keyword_of(id: &str, retry_count: i32) -> String {
    if id == RETRY {
        format!(
            "Retry up to {retry_count} {}",
            if retry_count == 1 { "time" } else { "times" }
        )
    } else {
        of(id).keyword.to_owned()
    }
}

/// `^retry up to (?<count>\d{1,6}) times?$`, ignoring case.
fn retry_count(text: &str) -> Option<i32> {
    let lower = text.to_lowercase();
    let rest = lower.strip_prefix("retry up to ")?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() || digits.len() > 6 {
        return None;
    }
    let tail = &rest[digits.len()..];
    (tail == " times" || tail == " time").then(|| digits.parse().ok())?
}

/// Ordinal comparison ignoring case, as .NET's `OrdinalIgnoreCase` compares (by simple upper-casing per character).
fn equal_ignoring_case(a: &str, b: &str) -> bool {
    a.chars().count() == b.chars().count()
        && a.chars()
            .zip(b.chars())
            .all(|(x, y)| x == y || x.to_uppercase().eq(y.to_uppercase()))
}
