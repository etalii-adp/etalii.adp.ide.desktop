//! The CEL standard environment's functions (the standalone host's `CelCore`, `CelStrings`, `CelMath`, `CelLists`
//! and `CelOptionals`): `size`, `matches` over the regular expression subset of FBL section 2.5, the conversions
//! `int`, `double`, `string` and `type`; the strings extension, with DISL's global `lower(s)` and `upper(s)`; the math
//! extension; the lists extension and CEL's optional values. Indexes into strings count code points.

use std::rc::Rc;

use crate::Environment;
use crate::eval::{as_double, as_int, as_list, as_string, compare, sort};
use crate::function::{Call, Function, Macro, Style};
use crate::regex_subset;
use crate::value::Value;

/// A cost proportional to the length of the first argument, for a function that walks it.
pub(crate) fn length(arguments: &[Value]) -> i64 {
    match arguments.first() {
        Some(Value::String(s)) => 1 + (s.encode_utf16().count() as i64 / 16),
        Some(Value::List(l)) => 1 + l.len() as i64,
        Some(Value::Map(m)) => 1 + m.len() as i64,
        _ => 1,
    }
}

fn ranged(
    name: &str,
    style: Style,
    min: usize,
    max: usize,
    body: impl Fn(&[Value]) -> Result<Value, String> + 'static,
) -> Function {
    Function::new(
        name,
        style,
        min,
        max,
        Some(Rc::new(move |call: &Call<'_>| body(call.arguments))),
    )
}

pub(crate) fn register(environment: &mut Environment) {
    core(environment);
    strings(environment);
    math(environment);
    lists(environment);
    optionals(environment);
}

fn core(environment: &mut Environment) {
    for kind in [Macro::All, Macro::Exists, Macro::ExistsOne, Macro::Filter, Macro::Map] {
        environment.add_macro(kind);
    }
    environment.add_function(Function::global("size", 1, |a| size(&a[0])));
    environment.add_function(Function::receiver("size", 0, |a| size(&a[0])));
    environment.add_function(Function::global("matches", 2, |a| matches(&a[0], &a[1])).with_cost(length));
    environment.add_function(Function::receiver("matches", 1, |a| matches(&a[0], &a[1])).with_cost(length));
    environment.add_function(Function::global("int", 1, |a| int(&a[0])));
    environment.add_function(Function::global("double", 1, |a| double(&a[0])));
    environment.add_function(Function::global("string", 1, |a| Ok(Value::String(a[0].to_string()))));
    environment.add_function(Function::global("type", 1, |a| type_of(&a[0])));
}

fn size(value: &Value) -> Result<Value, String> {
    Ok(Value::Int(match value {
        Value::String(s) => s.chars().count() as i64,
        Value::List(l) => l.len() as i64,
        Value::Map(m) => m.len() as i64,
        _ => return Err("size() needs a string, a list or a map.".into()),
    }))
}

// A pattern that does not parse is an error value like any failed call.
fn matches(text: &Value, pattern: &Value) -> Result<Value, String> {
    let expression = regex_subset::to_rust(as_string(pattern)?, false);
    let regex = regex::Regex::new(&expression).map_err(|e| format!("matches() cannot use its pattern: {e}"))?;
    Ok(Value::Bool(regex.is_match(as_string(text)?)))
}

fn int(value: &Value) -> Result<Value, String> {
    Ok(Value::Int(match value {
        Value::Int(i) => *i,
        Value::Double(d) if d.is_finite() && *d > i64::MIN as f64 && *d < i64::MAX as f64 => d.trunc() as i64,
        Value::String(s) => s
            .trim()
            .parse()
            .map_err(|_| "int() cannot convert this value.".to_owned())?,
        _ => return Err("int() cannot convert this value.".into()),
    }))
}

fn double(value: &Value) -> Result<Value, String> {
    Ok(Value::Double(match value {
        Value::String(s) => s
            .trim()
            .parse()
            .map_err(|_| format!("double() cannot convert '{s}'."))?,
        other => as_double(other)?,
    }))
}

fn type_of(value: &Value) -> Result<Value, String> {
    match value {
        Value::Optional(_) | Value::Object(_) => Err("type() does not know this value.".into()),
        other => Ok(Value::Type(other.type_name())),
    }
}

fn strings(environment: &mut Environment) {
    let text = |value: &Value| as_string(value).map(str::to_owned);
    environment.add_function(
        Function::receiver("startsWith", 1, move |a| {
            Ok(Value::Bool(text(&a[0])?.starts_with(as_string(&a[1])?)))
        })
        .with_cost(length),
    );
    environment.add_function(
        Function::receiver("endsWith", 1, |a| {
            Ok(Value::Bool(as_string(&a[0])?.ends_with(as_string(&a[1])?)))
        })
        .with_cost(length),
    );
    environment.add_function(
        Function::receiver("contains", 1, |a| {
            Ok(Value::Bool(as_string(&a[0])?.contains(as_string(&a[1])?)))
        })
        .with_cost(length),
    );
    environment.add_function(ranged("replace", Style::Receiver, 2, 3, replace).with_cost(length));
    environment.add_function(
        Function::receiver("lowerAscii", 0, |a| {
            Ok(Value::String(as_string(&a[0])?.to_ascii_lowercase()))
        })
        .with_cost(length),
    );
    environment.add_function(
        Function::receiver("upperAscii", 0, |a| {
            Ok(Value::String(as_string(&a[0])?.to_ascii_uppercase()))
        })
        .with_cost(length),
    );
    environment.add_function(
        Function::global("lower", 1, |a| Ok(Value::String(as_string(&a[0])?.to_lowercase()))).with_cost(length),
    );
    environment.add_function(
        Function::global("upper", 1, |a| Ok(Value::String(as_string(&a[0])?.to_uppercase()))).with_cost(length),
    );
    environment.add_function(Function::receiver("charAt", 1, char_at).with_cost(length));
    environment.add_function(ranged("indexOf", Style::Receiver, 1, 2, |a| index_of(a, false)).with_cost(length));
    environment.add_function(ranged("lastIndexOf", Style::Receiver, 1, 2, |a| index_of(a, true)).with_cost(length));
    environment.add_function(ranged("substring", Style::Receiver, 1, 2, substring).with_cost(length));
    environment.add_function(ranged("split", Style::Receiver, 1, 2, split).with_cost(length));
    environment.add_function(
        Function::receiver("trim", 0, |a| Ok(Value::String(as_string(&a[0])?.trim().to_owned()))).with_cost(length),
    );
    environment.add_function(ranged("join", Style::Receiver, 0, 1, join).with_cost(length));
    environment.add_function(Function::receiver("reverse", 0, reverse).with_cost(length));
}

fn replace(a: &[Value]) -> Result<Value, String> {
    let limit = if a.len() > 3 { as_int(&a[3])? } else { -1 };
    let (text, old, new) = (as_string(&a[0])?, as_string(&a[1])?, as_string(&a[2])?);
    Ok(Value::String(if limit < 0 {
        text.replace(old, new)
    } else {
        text.replacen(old, new, limit as usize)
    }))
}

fn char_at(a: &[Value]) -> Result<Value, String> {
    let points: Vec<char> = as_string(&a[0])?.chars().collect();
    let index = as_int(&a[1])?;
    if index < 0 || index as usize > points.len() {
        return Err(format!(
            "charAt({index}) is out of range for a string of {}.",
            points.len()
        ));
    }
    Ok(Value::String(
        points.get(index as usize).map(|c| c.to_string()).unwrap_or_default(),
    ))
}

fn index_of(a: &[Value], last: bool) -> Result<Value, String> {
    let name = if last { "lastIndexOf" } else { "indexOf" };
    if let Value::List(list) = &a[0] {
        // DISL section 12.4 l.indexOf(v): the first index, or -1.
        let found = if last {
            list.iter().rposition(|v| v.cel_equals(&a[1]))
        } else {
            list.iter().position(|v| v.cel_equals(&a[1]))
        };
        return Ok(Value::Int(found.map(|i| i as i64).unwrap_or(-1)));
    }
    let points: Vec<char> = as_string(&a[0])?.chars().collect();
    let sought: Vec<char> = as_string(&a[1])?.chars().collect();
    let start = if a.len() > 2 {
        as_int(&a[2])?
    } else if last {
        points.len() as i64
    } else {
        0
    };
    if start < 0 || start as usize > points.len() {
        return Err(format!(
            "{name}() start {start} is out of range for a string of {}.",
            points.len()
        ));
    }
    let at = |i: usize| i + sought.len() <= points.len() && points[i..i + sought.len()] == sought[..];
    let found = if last {
        if sought.len() > points.len() {
            None
        } else {
            (0..=(start as usize).min(points.len() - sought.len()))
                .rev()
                .find(|i| at(*i))
        }
    } else {
        (start as usize..=points.len()).find(|i| at(*i))
    };
    Ok(Value::Int(found.map(|i| i as i64).unwrap_or(-1)))
}

fn substring(a: &[Value]) -> Result<Value, String> {
    let points: Vec<char> = as_string(&a[0])?.chars().collect();
    let start = as_int(&a[1])?;
    let end = if a.len() > 2 {
        as_int(&a[2])?
    } else {
        points.len() as i64
    };
    if start < 0 || start as usize > points.len() || end < start || end as usize > points.len() {
        return Err(format!(
            "substring({start}, {end}) is out of range for a string of {}.",
            points.len()
        ));
    }
    Ok(Value::String(points[start as usize..end as usize].iter().collect()))
}

fn split(a: &[Value]) -> Result<Value, String> {
    let limit = if a.len() > 2 { as_int(&a[2])? } else { -1 };
    let (t, separator) = (as_string(&a[0])?, as_string(&a[1])?);
    if limit == 0 {
        return Ok(Value::list(Vec::new()));
    }
    if separator.is_empty() {
        let points: Vec<char> = t.chars().collect();
        let mut parts = Vec::new();
        for (i, c) in points.iter().enumerate() {
            if limit > 0 && parts.len() as i64 == limit - 1 {
                parts.push(Value::String(points[i..].iter().collect()));
                return Ok(Value::list(parts));
            }
            parts.push(Value::String(c.to_string()));
        }
        return Ok(Value::list(parts));
    }
    Ok(if limit < 0 {
        Value::list(t.split(separator).map(Value::from).collect())
    } else {
        Value::list(t.splitn(limit as usize, separator).map(Value::from).collect())
    })
}

fn join(a: &[Value]) -> Result<Value, String> {
    let list = as_list(&a[0])?;
    let separator = if a.len() > 1 { as_string(&a[1])? } else { "" };
    let mut parts = Vec::with_capacity(list.len());
    for item in list {
        parts.push(item.as_str().ok_or("join() needs a list of strings.")?);
    }
    Ok(Value::String(parts.join(separator)))
}

fn reverse(a: &[Value]) -> Result<Value, String> {
    Ok(match &a[0] {
        Value::List(list) => Value::list(list.iter().rev().cloned().collect()),
        other => Value::String(as_string(other)?.chars().rev().collect()),
    })
}

fn math(environment: &mut Environment) {
    environment.add_function(Function::global("math.round", 1, |a| {
        Ok(Value::Double(as_double(&a[0])?.round()))
    }));
    environment.add_function(Function::global("math.floor", 1, |a| {
        Ok(Value::Double(as_double(&a[0])?.floor()))
    }));
    environment.add_function(Function::global("math.ceil", 1, |a| {
        Ok(Value::Double(as_double(&a[0])?.ceil()))
    }));
    environment.add_function(Function::global("math.trunc", 1, |a| {
        Ok(Value::Double(as_double(&a[0])?.trunc()))
    }));
    environment.add_function(Function::global("math.abs", 1, |a| match &a[0] {
        Value::Int(i64::MIN) => Err("math.abs() overflows an int.".into()),
        Value::Int(i) => Ok(Value::Int(i.abs())),
        Value::Double(d) => Ok(Value::Double(d.abs())),
        _ => Err("math.abs() needs a number.".into()),
    }));
    environment.add_function(
        ranged("math.greatest", Style::Global, 1, Function::VARIADIC, |a| {
            extreme("math.greatest", a, 1)
        })
        .with_cost(count),
    );
    environment.add_function(
        ranged("math.least", Style::Global, 1, Function::VARIADIC, |a| {
            extreme("math.least", a, -1)
        })
        .with_cost(count),
    );
}

fn extreme(name: &str, arguments: &[Value], sign: i32) -> Result<Value, String> {
    let values: &[Value] = match arguments {
        [Value::List(list)] => list,
        _ => arguments,
    };
    let Some(first) = values.first() else {
        return Err(format!("{name}() needs at least one number."));
    };
    let mut best = first.clone();
    as_double(&best)?;
    for value in &values[1..] {
        let order = compare(value, &best)? as i32;
        if order * sign > 0 {
            best = value.clone();
        }
    }
    Ok(best)
}

fn count(arguments: &[Value]) -> i64 {
    match arguments {
        [Value::List(list)] => 1 + list.len() as i64,
        _ => arguments.len() as i64,
    }
}

fn lists(environment: &mut Environment) {
    environment.add_function(
        Function::global("lists.range", 1, |a| {
            Ok(Value::list((0..as_int(&a[0])?.max(0)).map(Value::Int).collect()))
        })
        .with_cost(|a| match a.first() {
            Some(Value::Int(n)) => 1 + (*n).max(0),
            _ => 1,
        }),
    );
    environment.add_function(
        Function::receiver("distinct", 0, |a| {
            let mut kept: Vec<Value> = Vec::new();
            for item in as_list(&a[0])? {
                if !kept.iter().any(|k| k.cel_equals(item)) {
                    kept.push(item.clone());
                }
            }
            Ok(Value::list(kept))
        })
        .with_cost(|a| match a.first() {
            Some(Value::List(l)) => 1 + (l.len() as i64) * (l.len() as i64),
            _ => 1,
        }),
    );
    environment.add_function(
        ranged("flatten", Style::Receiver, 0, 1, |a| {
            let depth = if a.len() > 1 { as_int(&a[1])? } else { 1 };
            flatten(as_list(&a[0])?, depth).map(Value::list)
        })
        .with_cost(length),
    );
    environment.add_function(
        Function::receiver("slice", 2, |a| {
            let list = as_list(&a[0])?;
            let (start, end) = (as_int(&a[1])?, as_int(&a[2])?);
            if start < 0 || end < start || end > list.len() as i64 {
                return Err(format!(
                    "slice({start}, {end}) is out of range for a list of {}.",
                    list.len()
                ));
            }
            Ok(Value::list(list[start as usize..end as usize].to_vec()))
        })
        .with_cost(length),
    );
    environment.add_function(
        Function::receiver("sort", 0, |a| {
            sort(
                as_list(&a[0])?
                    .iter()
                    .map(|item| (item.clone(), item.clone()))
                    .collect(),
            )
            .map(Value::list)
        })
        .with_cost(|a| match a.first() {
            Some(Value::List(l)) if l.len() > 1 => 1 + (l.len() as f64 * (l.len() as f64).log2()) as i64,
            _ => 1,
        }),
    );
    environment.add_macro(Macro::SortBy);
    environment.add_function(Function::receiver("first", 0, |a| {
        Ok(as_list(&a[0])?
            .first()
            .cloned()
            .map(Value::some)
            .unwrap_or_else(Value::none))
    }));
    environment.add_function(Function::receiver("last", 0, |a| {
        Ok(as_list(&a[0])?
            .last()
            .cloned()
            .map(Value::some)
            .unwrap_or_else(Value::none))
    }));
}

fn flatten(list: &[Value], depth: i64) -> Result<Vec<Value>, String> {
    if depth < 0 {
        return Err("flatten() needs a depth of 0 or more.".into());
    }
    let mut flat = Vec::new();
    for item in list {
        match item {
            Value::List(inner) if depth > 0 => flat.extend(flatten(inner, depth - 1)?),
            other => flat.push(other.clone()),
        }
    }
    Ok(flat)
}

fn optional(value: &Value) -> Result<Option<&Value>, String> {
    match value {
        Value::Optional(inner) => Ok(inner.as_deref()),
        _ => Err("An optional value was expected.".into()),
    }
}

fn optionals(environment: &mut Environment) {
    environment.add_function(Function::global("optional.of", 1, |a| Ok(Value::some(a[0].clone()))));
    environment.add_function(Function::global("optional.none", 0, |_| Ok(Value::none())));
    environment.add_function(Function::receiver("hasValue", 0, |a| {
        Ok(Value::Bool(optional(&a[0])?.is_some()))
    }));
    environment.add_function(Function::receiver("value", 0, |a| {
        optional(&a[0])?
            .cloned()
            .ok_or_else(|| "optional.none() has no value.".to_owned())
    }));
    environment.add_function(Function::receiver("orValue", 1, |a| {
        Ok(optional(&a[0])?.cloned().unwrap_or_else(|| a[1].clone()))
    }));
    environment.add_function(Function::receiver("or", 1, |a| {
        if optional(&a[0])?.is_some() {
            return Ok(a[0].clone());
        }
        optional(&a[1])?;
        Ok(a[1].clone())
    }));
}
