//! CEL's values (the standalone host's `CelValues`, `CelMap`, `CelType`, `CelOptional` and `ICelObject`): null,
//! bool, int, double, string, list, map, type, optional and a host object. A list and a map are shared, so handing a
//! large value to many evaluations copies nothing.

use std::any::Any;
use std::fmt;
use std::rc::Rc;

/// A host value an expression can work with as it does with a map (the standalone host's `ICelObject`): select a
/// member (`e.name`, `e.?name`), test one with `has(e.name)`, and call a method on it (`e.isA('Trend')`). A method
/// is called on the object first; a function the environment registers under the same name for receivers is the
/// fallback. An opaque host value, such as DISL's colour, answers false from [`Object::is_selectable`]: it has no
/// members and no methods, only the functions that take it.
pub trait Object: Any + fmt::Debug {
    /// The object as `Any`, for [`Value::downcast`].
    fn into_any(self: Rc<Self>) -> Rc<dyn Any>;

    /// The object as `Any`, by reference.
    fn as_any(&self) -> &dyn Any;

    /// Whether members are selected from it, `has()` tested on it and methods called on it.
    fn is_selectable(&self) -> bool {
        true
    }

    /// The member's value, none when the object has no such member.
    fn member(&self, _name: &str) -> Option<Value> {
        None
    }

    /// Whether `has()` holds for the member: present, as a map key is present.
    fn has_member(&self, _name: &str) -> bool {
        false
    }

    /// The result of the method, none when the object does not implement it; `arguments` exclude the object itself.
    /// An error is a call it implements but cannot complete.
    fn invoke(&self, _name: &str, _arguments: &[Value]) -> Option<Result<Value, String>> {
        None
    }

    /// CEL's equality with another object: identity unless the object says otherwise.
    fn equals(&self, other: &dyn Object) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const dyn Object)
    }

    /// The object as `string()` writes it.
    fn text(&self) -> String;
}

/// A CEL value. FBL hands its readings to CEL in these, and reads attribute values back in them.
#[derive(Debug, Clone, Default)]
pub enum Value {
    #[default]
    Null,
    Bool(bool),
    Int(i64),
    Double(f64),
    String(String),
    List(Rc<Vec<Value>>),
    Map(Rc<Map>),
    /// What `type(x)` returns, and what `map`, `list`, `string`, `int`, `double`, `bool`, `null_type` and `type` denote.
    Type(&'static str),
    /// CEL's optional value: what `x.?f`, `m[?k]`, `optional.of(v)` and `optional.none()` give.
    Optional(Option<Rc<Value>>),
    /// A host object (the standalone host's `ICelObject`), or an opaque host value.
    Object(Rc<dyn Object>),
}

/// The tolerance the standalone host shares as `double.Tolerance` (`0.000001f`), within which an int equals a double.
const TOLERANCE: f64 = 0.000_001_f32 as f64;

/// The names a type is known by.
pub(crate) const TYPE_NAMES: [&str; 8] = ["map", "list", "string", "int", "double", "bool", "null_type", "type"];

impl Value {
    pub fn list(items: Vec<Value>) -> Value {
        Value::List(Rc::new(items))
    }

    pub fn map(map: Map) -> Value {
        Value::Map(Rc::new(map))
    }

    /// `optional.of(value)`.
    pub fn some(value: Value) -> Value {
        Value::Optional(Some(Rc::new(value)))
    }

    /// `optional.none()`.
    pub fn none() -> Value {
        Value::Optional(None)
    }

    /// A host object as a value.
    pub fn object<T: Object>(object: T) -> Value {
        Value::Object(Rc::new(object))
    }

    /// The host object of type `T` this value holds, if it holds one.
    pub fn downcast<T: Object>(&self) -> Option<Rc<T>> {
        match self {
            Value::Object(object) => Rc::clone(object).into_any().downcast::<T>().ok(),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[Value]> {
        match self {
            Value::List(l) => Some(l),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<&Map> {
        match self {
            Value::Map(m) => Some(m),
            _ => None,
        }
    }

    /// The number a value holds, an int or a double, as a double.
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Int(i) => Some(*i as f64),
            Value::Double(d) => Some(*d),
            _ => None,
        }
    }

    /// The name of the value's type, as CEL names it.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null_type",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Double(_) => "double",
            Value::String(_) => "string",
            Value::List(_) => "list",
            Value::Map(_) => "map",
            Value::Type(_) => "type",
            Value::Optional(_) => "optional_type",
            Value::Object(_) => "object",
        }
    }

    /// CEL's equality (`CelValues.Equal`): an int and a double are equal within the tolerance, lists and maps by
    /// their members, everything else by its value.
    pub fn cel_equals(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Int(x), Value::Double(y)) | (Value::Double(y), Value::Int(x)) => (*x as f64 - y).abs() < TOLERANCE,
            (Value::Int(x), Value::Int(y)) => x == y,
            // .NET's double.Equals holds NaN equal to itself.
            (Value::Double(x), Value::Double(y)) => x == y || (x.is_nan() && y.is_nan()),
            (Value::Bool(x), Value::Bool(y)) => x == y,
            (Value::String(x), Value::String(y)) => x == y,
            (Value::List(x), Value::List(y)) => {
                x.len() == y.len() && x.iter().zip(y.iter()).all(|(a, b)| a.cel_equals(b))
            }
            (Value::Map(x), Value::Map(y)) => {
                x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|o| v.cel_equals(o)))
            }
            (Value::Type(x), Value::Type(y)) => x == y,
            (Value::Optional(x), Value::Optional(y)) => match (x, y) {
                (None, None) => true,
                (Some(x), Some(y)) => x.cel_equals(y),
                _ => false,
            },
            (Value::Object(x), Value::Object(y)) => x.equals(y.as_ref()),
            _ => false,
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        self.cel_equals(other)
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Value::String(value.to_owned())
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::String(value)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Bool(value)
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Value::Int(value)
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Value::Double(value)
    }
}

impl fmt::Display for Value {
    /// `string(x)` (`CelValues.Format`): a double as CEL's reference implementation writes it.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => f.write_str("null"),
            Value::Bool(b) => f.write_str(if *b { "true" } else { "false" }),
            Value::Int(i) => write!(f, "{i}"),
            Value::Double(d) => f.write_str(&double_text(*d)),
            Value::String(s) => f.write_str(s),
            Value::List(items) => {
                f.write_str("[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{item}")?;
                }
                f.write_str("]")
            }
            Value::Map(_) => f.write_str("map"),
            Value::Type(name) => f.write_str(name),
            Value::Optional(Some(value)) => write!(f, "optional.of({value})"),
            Value::Optional(None) => f.write_str("optional.none()"),
            Value::Object(object) => f.write_str(&object.text()),
        }
    }
}

/// The map type CEL values use: insertion-ordered, string keys.
#[derive(Debug, Clone, Default)]
pub struct Map {
    entries: Vec<(String, Value)>,
}

impl Map {
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.entries.iter().any(|(k, _)| k == key)
    }

    /// Sets `key`, keeping its place when it is there already.
    pub fn insert(&mut self, key: impl Into<String>, value: Value) {
        let key = key.into();
        if let Some(entry) = self.entries.iter_mut().find(|(k, _)| *k == key) {
            entry.1 = value;
        } else {
            self.entries.push((key, value));
        }
    }

    /// Takes `key` out, giving its value when it was there.
    pub fn remove(&mut self, key: &str) -> Option<Value> {
        let index = self.entries.iter().position(|(k, _)| k == key)?;
        Some(self.entries.remove(index).1)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.entries.iter().map(|(k, v)| (k, v))
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.entries.iter().map(|(k, _)| k)
    }

    pub fn values(&self) -> impl Iterator<Item = &Value> {
        self.entries.iter().map(|(_, v)| v)
    }
}

impl PartialEq for Map {
    /// CEL's equality of two maps: the same keys, each with an equal value.
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().all(|(k, v)| other.get(k).is_some_and(|o| v.cel_equals(o)))
    }
}

impl<K: Into<String>> FromIterator<(K, Value)> for Map {
    fn from_iter<T: IntoIterator<Item = (K, Value)>>(iter: T) -> Self {
        let mut map = Map::default();
        for (k, v) in iter {
            map.insert(k, v);
        }
        map
    }
}

impl From<Map> for Value {
    fn from(value: Map) -> Self {
        Value::map(value)
    }
}

impl From<Vec<Value>> for Value {
    fn from(value: Vec<Value>) -> Self {
        Value::list(value)
    }
}

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(value: Option<T>) -> Self {
        value.map(Into::into).unwrap_or(Value::Null)
    }
}

/// `string(double)` as CEL's reference implementation writes it, Go's `%g`: the shortest digits that read back as
/// the same double, in decimal notation for exponents from -4 to 5 and in `d.ddde±XX` notation outside them.
pub(crate) fn double_text(value: f64) -> String {
    if value.is_nan() {
        return "NaN".into();
    }
    if value.is_infinite() {
        return if value > 0.0 { "+Inf".into() } else { "-Inf".into() };
    }
    if value == 0.0 {
        return if value.is_sign_negative() {
            "-0".into()
        } else {
            "0".into()
        };
    }
    // Rust's `{:e}` gives the shortest round-tripping digits, as Go's shortest formatting does.
    let scientific = format!("{:e}", value.abs());
    let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
    let exponent: i32 = exponent.parse().unwrap_or(0);
    let digits: String = mantissa.chars().filter(|c| c.is_ascii_digit()).collect();
    let digits = digits.trim_end_matches('0');
    let digits = if digits.is_empty() { "0" } else { digits };
    let point = exponent + 1;
    let mut text = String::new();
    if value < 0.0 {
        text.push('-');
    }
    if !(-4..6).contains(&exponent) {
        text.push_str(&digits[..1]);
        if digits.len() > 1 {
            text.push('.');
            text.push_str(&digits[1..]);
        }
        text.push('e');
        text.push(if exponent < 0 { '-' } else { '+' });
        text.push_str(&format!("{:02}", exponent.abs()));
    } else if point <= 0 {
        text.push_str("0.");
        text.push_str(&"0".repeat((-point) as usize));
        text.push_str(digits);
    } else if point as usize >= digits.len() {
        text.push_str(digits);
        text.push_str(&"0".repeat(point as usize - digits.len()));
    } else {
        text.push_str(&digits[..point as usize]);
        text.push('.');
        text.push_str(&digits[point as usize..]);
    }
    text
}
