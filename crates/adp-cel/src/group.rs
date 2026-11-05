//! A list grouped by key (DISL 0.9, section 12.4): the value `l.groupBy(x, key)` gives, which `get(k)` and `keys()`
//! read, and the groupings a runtime keeps while nothing they read has changed (section 12.5).
//!
//! A grouping is kept by the identity of its list and by its key expression, and only when the key read nothing but
//! its own item's stored values, literals and pure functions; the evaluator decides that during the pass. A kept
//! grouping charges the cost its pass spent again, so whether it was kept never changes a budget.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use crate::value::{Object, Value};

/// A key other than a string as a grouping compares it: an int and a double of one value are one key, and so are 0.0
/// and -0.0.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Key {
    Int(i64),
    /// A double that is no int, by its bits.
    Double(u64),
    Bool(bool),
}

/// The sentence of a key of a kind a grouping does not take.
const KEY_KIND: &str = "A grouping's key must be a string, a number or a bool.";

impl Key {
    /// The key `value` is, a string being none of these.
    fn of(value: &Value) -> Result<Key, String> {
        Ok(match value {
            Value::Int(i) => Key::Int(*i),
            Value::Bool(b) => Key::Bool(*b),
            Value::Double(d) if d.is_nan() => return Err("A grouping's key may not be NaN.".into()),
            Value::Double(d) => {
                // -0.0 and 0.0 are one key; an integral double in range is the int of its value.
                if d.fract() == 0.0 && *d >= -9_223_372_036_854_775_808.0 && *d < 9_223_372_036_854_775_808.0 {
                    Key::Int(*d as i64)
                } else {
                    Key::Double(d.to_bits())
                }
            }
            _ => return Err(KEY_KIND.into()),
        })
    }
}

/// What `l.groupBy(x, key)` gives: one group per distinct key, keys in order of first appearance, each group's items
/// in list order. It is no map: it cannot be indexed, compared or stored, and `get` gives a new list every time.
#[derive(Debug)]
pub struct Grouping {
    /// The first value met of each key, in order of first appearance.
    keys: Rc<Vec<Value>>,
    groups: Vec<Rc<Vec<Value>>>,
    strings: HashMap<String, usize>,
    others: HashMap<Key, usize>,
}

impl Grouping {
    /// Groups `items` by their `keys`, one each, in order.
    pub(crate) fn new(items: &[Value], keys: Vec<Value>) -> Result<Grouping, String> {
        let mut strings: HashMap<String, usize> = HashMap::new();
        let mut others: HashMap<Key, usize> = HashMap::new();
        let mut firsts: Vec<Value> = Vec::new();
        let mut members: Vec<Vec<Value>> = Vec::new();
        for (item, key) in items.iter().zip(keys) {
            let known = match &key {
                Value::String(s) => strings.get(s.as_str()).copied(),
                other => others.get(&Key::of(other)?).copied(),
            };
            let at = match known {
                Some(at) => at,
                None => {
                    let at = members.len();
                    match &key {
                        Value::String(s) => strings.insert(s.clone(), at),
                        other => others.insert(Key::of(other)?, at),
                    };
                    firsts.push(key);
                    members.push(Vec::new());
                    at
                }
            };
            members[at].push(item.clone());
        }
        Ok(Grouping {
            keys: Rc::new(firsts),
            groups: members.into_iter().map(Rc::new).collect(),
            strings,
            others,
        })
    }

    /// Where the group of `key` stands, none for an absent key.
    fn position(&self, key: &Value) -> Result<Option<usize>, String> {
        Ok(match key {
            Value::String(s) => self.strings.get(s.as_str()).copied(),
            other => self.others.get(&Key::of(other)?).copied(),
        })
    }

    /// `g.get(k)`: the items with key `k`, in list order; `[]` for a key that is absent. The list is shared, which no
    /// expression can tell, since lists are values.
    pub fn get(&self, key: &Value) -> Result<Value, String> {
        Ok(match self.position(key)? {
            Some(at) => Value::List(Rc::clone(&self.groups[at])),
            None => Value::list(Vec::new()),
        })
    }

    /// The size of the group `get(key)` gives, 0 for an absent or failing key.
    pub fn group_size(&self, key: &Value) -> usize {
        match self.position(key) {
            Ok(Some(at)) => self.groups[at].len(),
            _ => 0,
        }
    }

    /// `g.keys()`: the distinct keys in order of first appearance, each as first met.
    pub fn keys(&self) -> Value {
        Value::List(Rc::clone(&self.keys))
    }

    pub fn key_count(&self) -> usize {
        self.keys.len()
    }
}

impl Object for Grouping {
    fn into_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    /// Opaque: only `get` and `keys` take it.
    fn is_selectable(&self) -> bool {
        false
    }

    /// No two groupings are equal, as no expression can compare them.
    fn equals(&self, _other: &dyn Object) -> bool {
        false
    }

    fn text(&self) -> String {
        "grouping".into()
    }
}

/// A grouping kept, with the list it was made of and the cost its pass charged.
struct Kept {
    list: Weak<Vec<Value>>,
    grouping: Rc<Grouping>,
    cost: i64,
}

thread_local! {
    /// The groupings kept, by the address of their list and their key expression's number.
    static KEPT: RefCell<HashMap<(usize, u64), Kept>> = RefCell::new(HashMap::new());
    /// The key expressions, each numbered once by its text, so that one key written twice shares its groupings.
    static SIGNATURES: RefCell<HashMap<String, u64>> = RefCell::new(HashMap::new());
    /// The number of kept groupings at which those whose list is gone are next forgotten.
    static PRUNE: Cell<usize> = const { Cell::new(PRUNE_AT) };
    /// The passes made on this thread: groupings made rather than given again.
    static MADE: Cell<u64> = const { Cell::new(0) };
}

/// How many groupings this thread has made by a pass over their list, rather than given again: what shows whether a
/// grouping was kept, which no expression can tell.
pub fn groupings_made() -> u64 {
    MADE.with(Cell::get)
}

/// Counts one pass.
pub(crate) fn made() {
    MADE.with(|m| m.set(m.get() + 1));
}

/// The fewest kept groupings at which those whose list is gone are forgotten.
const PRUNE_AT: usize = 256;

/// The number of the key expression `signature`.
pub(crate) fn signature(text: String) -> u64 {
    SIGNATURES.with(|signatures| {
        let mut signatures = signatures.borrow_mut();
        let next = signatures.len() as u64;
        *signatures.entry(text).or_insert(next)
    })
}

/// The grouping kept for `list` and the key `signature`, with the cost its pass charged.
pub(crate) fn kept(list: &Rc<Vec<Value>>, signature: u64) -> Option<(Rc<Grouping>, i64)> {
    KEPT.with(|kept| {
        let kept = kept.borrow();
        let entry = kept.get(&(Rc::as_ptr(list) as usize, signature))?;
        // The weak reference keeps the list's address from being reused while the entry lives.
        let alive = entry.list.upgrade()?;
        Rc::ptr_eq(&alive, list).then(|| (Rc::clone(&entry.grouping), entry.cost))
    })
}

/// Keeps `grouping` of `list` by the key `signature`, which charged `cost`.
pub(crate) fn keep(list: &Rc<Vec<Value>>, signature: u64, grouping: Rc<Grouping>, cost: i64) {
    KEPT.with(|kept| {
        let mut kept = kept.borrow_mut();
        if kept.len() >= PRUNE.get() {
            kept.retain(|_, entry| entry.list.strong_count() > 0);
            PRUNE.set((kept.len() * 2).max(PRUNE_AT));
        }
        kept.insert(
            (Rc::as_ptr(list) as usize, signature),
            Kept {
                list: Rc::downgrade(list),
                grouping,
                cost,
            },
        );
    });
}
