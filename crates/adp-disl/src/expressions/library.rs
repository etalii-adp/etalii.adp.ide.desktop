//! The DISL function library (DISL section 12.4; the standalone host's `DislCelLibrary`) and the element methods of
//! section 12.2 this runtime offers, on top of CEL's standard environment. A specification that calls anything else
//! is refused at load, naming it. The element methods are declared only: `isA`, `descendants` and the rest are
//! implemented by the model's elements, and calling one on any other value is an evaluation error.

use std::rc::Rc;

use adp_cel::{Environment, Function, Value};
use indexmap::IndexMap;

use super::color::DislColor;
use super::number_format;
use super::year_month;
use crate::loading::metamodel::DislEnum;
use crate::model::diagram::DiagramData;

/// The methods of DISL's Element and Diagram types (section 12.2) this runtime implements, with their arities.
const METHODS: [(&str, usize, usize); 13] = [
    ("isA", 1, 1),
    ("descendants", 0, 0),
    ("ancestors", 0, 0),
    ("childrenOfType", 1, 1),
    ("incomingOf", 1, 1),
    ("outgoingOf", 1, 1),
    ("positionIn", 1, 1),
    ("nodesOfType", 1, 2),
    ("relationsOfType", 1, 1),
    ("elementById", 1, 1),
    ("label", 0, 0),
    ("other", 1, 1),
    ("location", 0, 0),
];

fn number(value: &Value) -> Result<f64, String> {
    match value {
        Value::Int(i) => Ok(*i as f64),
        Value::Double(d) => Ok(*d),
        _ => Err("A number was expected.".into()),
    }
}

fn int(value: &Value) -> Result<i64, String> {
    value.as_int().ok_or_else(|| "An int was expected.".to_owned())
}

fn text(value: &Value) -> Result<&str, String> {
    value.as_str().ok_or_else(|| "A string was expected.".to_owned())
}

/// Adds the library to `environment`; `enums` are the enumerations `enumLabel` reads.
pub(crate) fn register(environment: &mut Environment, enums: Rc<IndexMap<String, DislEnum>>) {
    for (name, min, max) in METHODS {
        environment.add_function(Function::method(name, min, max));
    }

    environment.add_function(
        Function::receiver("cycles", 2, |a| {
            let (cycles, _) = cycles(a)?;
            Ok(Value::list(
                cycles
                    .into_iter()
                    .map(|cycle| Value::list(cycle.iter().map(|e| e.to_value()).collect()))
                    .collect(),
            ))
        })
        .with_cost(cycles_cost),
    );
    environment.add_function(
        Function::receiver("cyclesTruncated", 2, |a| Ok(Value::Bool(cycles(a)?.1))).with_cost(cycles_cost),
    );
    environment.add_function(Function::global("enumLabel", 2, move |a| {
        let name = text(&a[0])?;
        let enumeration = enums
            .get(name)
            .ok_or_else(|| format!("'{name}' is not an enumeration of this specification."))?;
        let key = text(&a[1])?;
        Ok(Value::String(match enumeration.value_of(key) {
            Some(known) => known.label.clone().unwrap_or_else(|| known.key.clone()),
            None => key.to_owned(),
        }))
    }));
    environment.add_function(
        Function::receiver("avg", 0, |a| average(&a[0])).with_cost(|a| match a.first() {
            Some(Value::List(l)) => l.len() as i64,
            _ => 1,
        }),
    );
    environment.add_function(
        Function::receiver("sum", 0, |a| sum(&a[0])).with_cost(|a| match a.first() {
            Some(Value::List(l)) => l.len() as i64,
            _ => 1,
        }),
    );
    environment.add_function(Function::global("formatNumber", 2, |a| {
        Ok(Value::String(number_format::format_number(
            number(&a[0])?,
            text(&a[1])?,
        )?))
    }));
    environment.add_function(Function::receiver("center", 0, |a| center(&a[0])));
    environment.add_function(Function::global("token", 1, |a| {
        Err(format!(
            "The theme token '{}' is resolved where the diagram is drawn; this runtime has no theme.",
            text(&a[0])?
        ))
    }));
    environment.add_function(Function::global("color", 1, |a| {
        Ok(Value::object(DislColor::of(&a[0])?))
    }));
    environment.add_function(Function::receiver("mix", 2, |a| DislColor::mix(&a[0], &a[1], &a[2])));
    environment.add_function(Function::global("clamp", 3, |a| {
        Ok(if number(&a[0])? < number(&a[1])? {
            a[1].clone()
        } else if number(&a[0])? > number(&a[2])? {
            a[2].clone()
        } else {
            a[0].clone()
        })
    }));
    environment.add_function(Function::new(
        "min",
        adp_cel::Style::Global,
        2,
        4,
        Some(Rc::new(|call: &adp_cel::Call<'_>| extreme(call.arguments, -1))),
    ));
    environment.add_function(Function::new(
        "max",
        adp_cel::Style::Global,
        2,
        4,
        Some(Rc::new(|call: &adp_cel::Call<'_>| extreme(call.arguments, 1))),
    ));
    environment.add_function(Function::global("yearMonth", 2, |a| {
        Ok(Value::Int(year_month::of(int(&a[0])?, int(&a[1])?)?))
    }));
    environment.add_function(Function::global("formatYearMonth", 2, |a| {
        Ok(Value::String(year_month::format(int(&a[0])?, text(&a[1])?)?))
    }));
    environment.add_function(Function::global("parseYearMonth", 1, |a| {
        Ok(match year_month::parse(text(&a[0])?) {
            Some(index) => Value::some(Value::Int(index)),
            None => Value::none(),
        })
    }));
    environment.add_function(Function::receiver("year", 0, |a| {
        Ok(Value::Int(year_month::year_of(int(&a[0])?)))
    }));
    environment.add_function(Function::receiver("month", 0, |a| {
        Ok(Value::Int(year_month::month_of(int(&a[0])?)))
    }));
}

type Cycles = (Vec<Vec<crate::DislElement>>, bool);

/// `diagram.cycles(relType, max)` and `diagram.cyclesTruncated(relType, max)` (section 12.4), on the diagram that
/// receives them.
fn cycles(arguments: &[Value]) -> Result<Cycles, String> {
    let diagram = arguments[0]
        .downcast::<DiagramData>()
        .ok_or("cycles() and cyclesTruncated() are methods of the diagram.")?;
    let max = int(&arguments[2])?;
    if max < 0 {
        return Err(format!("The bound of cycles() is a count, not {max}."));
    }
    Ok(diagram.cycles(text(&arguments[1])?, max))
}

/// The cost section 12.4 sets for the cycle search: proportional to (nodes + relations) × (`max` + 1), so a bound
/// keeps CEL's termination guarantee.
fn cycles_cost(arguments: &[Value]) -> i64 {
    match (
        arguments.first().and_then(|d| d.downcast::<DiagramData>()),
        arguments.get(2),
    ) {
        (Some(diagram), Some(Value::Int(max))) if *max >= 0 => {
            let size = diagram.size() as i64;
            if *max >= i64::MAX / size.max(1) {
                i64::MAX
            } else {
                size * (max + 1)
            }
        }
        _ => 1,
    }
}

/// `l.avg()` (section 12.4): the mean of a list of ints and doubles, as a double; an empty list has none.
fn average(value: &Value) -> Result<Value, String> {
    let list = value.as_list().ok_or("avg() takes a list of numbers.")?;
    if list.is_empty() {
        return Err("avg() of an empty list has no value.".into());
    }
    let mut sum = 0.0;
    for item in list {
        sum += match item {
            Value::Int(i) => *i as f64,
            Value::Double(d) => *d,
            _ => return Err("avg() takes a list of numbers.".into()),
        };
    }
    Ok(Value::Double(sum / list.len() as f64))
}

/// `l.sum()` (section 12.4): the sum of a list of ints and doubles; an int while every item is one, a double once one
/// is a double, and 0 for an empty list.
fn sum(value: &Value) -> Result<Value, String> {
    let list = value.as_list().ok_or("sum() takes a list of numbers.")?;
    let mut total = Value::Int(0);
    for item in list {
        total = match (&total, item) {
            (Value::Int(a), Value::Int(b)) => Value::Int(a.checked_add(*b).ok_or("sum() overflowed an int.")?),
            (Value::Int(a), Value::Double(b)) => Value::Double(*a as f64 + b),
            (Value::Double(a), Value::Int(b)) => Value::Double(a + *b as f64),
            (Value::Double(a), Value::Double(b)) => Value::Double(a + b),
            _ => return Err("sum() takes a list of numbers.".into()),
        };
    }
    Ok(total)
}

/// `b.center()` on a Bounds (section 12.2): `[x + width / 2, y + height / 2]`, as doubles.
fn center(value: &Value) -> Result<Value, String> {
    let refused = || "center() needs bounds with x, y, width and height.".to_owned();
    let bounds = value.as_map().ok_or_else(refused)?;
    let field = |name: &str| match bounds.get(name) {
        Some(Value::Int(i)) => Ok(*i as f64),
        Some(Value::Double(d)) => Ok(*d),
        _ => Err(refused()),
    };
    let (x, y, width, height) = (field("x")?, field("y")?, field("width")?, field("height")?);
    Ok(Value::list(vec![
        Value::Double(x + width / 2.0),
        Value::Double(y + height / 2.0),
    ]))
}

/// The smallest (`sign` -1) or largest (1) argument, itself rather than converted, the first of equals.
fn extreme(arguments: &[Value], sign: i32) -> Result<Value, String> {
    let mut best = &arguments[0];
    for candidate in &arguments[1..] {
        let (c, b) = (number(candidate)?, number(best)?);
        let order = match c.partial_cmp(&b) {
            Some(o) => o as i32,
            None if c.is_nan() && b.is_nan() => 0,
            None if c.is_nan() => -1,
            None => 1,
        };
        if order * sign > 0 {
            best = candidate;
        }
    }
    Ok(best.clone())
}
