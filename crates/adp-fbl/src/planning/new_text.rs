//! The rules of FBL section 6.3 that are the same for every family (the standalone host's `Planning/NewText`):
//! numbers, times, maps and the `emit` templates of new entries. A value is written as the standalone host writes
//! it, so that the same edit gives the same bytes in both hosts.

use adp_cel::Value;

use crate::documents::types::AttributeBinding;

/// A number as FBL section 6.3 writes it: with `shortest`, an integer without a fraction and any other number in
/// ECMAScript's `Number.prototype.toString` form; with `{decimals: n}`, rounded halves away from zero, trailing zeros
/// and a trailing point dropped.
pub(crate) fn number(value: f64, decimals: Option<i32>) -> String {
    if let Some(n) = decimals {
        let n = n.max(0);
        let scale = 10f64.powi(n);
        let rounded = (value * scale).round() / scale;
        let mut text = format!("{rounded:.*}", n as usize);
        if text.contains('.') {
            text = text.trim_end_matches('0').trim_end_matches('.').to_owned();
        }
        return if text == "-0" { "0".into() } else { text };
    }
    if !value.is_finite() {
        return value.to_string();
    }
    if value == 0.0 {
        return "0".into();
    }
    if (value.floor() - value).abs() < f64::from(0.000_001_f32) && value.abs() < 1e21 {
        return decimal_text(value);
    }
    let (digits, exponent) = shortest(value.abs());
    let sign = if value < 0.0 { "-" } else { "" };
    if (-6..21).contains(&exponent) {
        return format!("{sign}{}", positional(&digits, exponent));
    }
    let mantissa = if digits.len() > 1 {
        format!("{}.{}", &digits[..1], &digits[1..])
    } else {
        digits.clone()
    };
    format!(
        "{sign}{mantissa}e{}{}",
        if exponent < 0 { "-" } else { "+" },
        exponent.abs()
    )
}

/// The shortest round-tripping significant digits of a positive double, and the exponent of its first digit.
fn shortest(value: f64) -> (String, i32) {
    let scientific = format!("{value:e}");
    let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let digits = digits.trim_end_matches('0');
    (
        if digits.is_empty() {
            "0".into()
        } else {
            digits.to_owned()
        },
        exponent.parse().unwrap_or(0),
    )
}

/// Digits whose first is at `exponent`, written without an exponent.
fn positional(digits: &str, exponent: i32) -> String {
    let point = exponent + 1;
    if point <= 0 {
        format!("0.{}{digits}", "0".repeat((-point) as usize))
    } else if point as usize >= digits.len() {
        format!("{digits}{}", "0".repeat(point as usize - digits.len()))
    } else {
        format!("{}.{}", &digits[..point as usize], &digits[point as usize..])
    }
}

/// .NET's `((decimal)value).ToString()`: the double rounded to 15 significant digits, written without an exponent
/// and without trailing zeros.
fn decimal_text(value: f64) -> String {
    let scientific = format!("{:.14e}", value.abs());
    let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let digits = digits.trim_end_matches('0');
    let digits = if digits.is_empty() { "0" } else { digits };
    let text = positional(digits, exponent.parse().unwrap_or(0));
    if value < 0.0 { format!("-{text}") } else { text }
}

/// .NET's `double.ToString()`: the shortest round-tripping digits, in `E` notation from an exponent of 15 up and
/// below -5.
pub(crate) fn dotnet_double(value: f64) -> String {
    if value.is_nan() {
        return "NaN".into();
    }
    if value.is_infinite() {
        return if value > 0.0 { "∞".into() } else { "-∞".into() };
    }
    if value == 0.0 {
        return if value.is_sign_negative() {
            "-0".into()
        } else {
            "0".into()
        };
    }
    let (digits, exponent) = shortest(value.abs());
    let sign = if value < 0.0 { "-" } else { "" };
    if (-5..15).contains(&exponent) {
        return format!("{sign}{}", positional(&digits, exponent));
    }
    let mantissa = if digits.len() > 1 {
        format!("{}.{}", &digits[..1], &digits[1..])
    } else {
        digits.clone()
    };
    format!(
        "{sign}{mantissa}E{}{:02}",
        if exponent < 0 { "-" } else { "+" },
        exponent.abs()
    )
}

/// .NET's `Convert.ToString(value, CultureInfo.InvariantCulture)` of a value the standalone host keeps as an object.
pub(crate) fn dotnet_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(b) => if *b { "True" } else { "False" }.into(),
        Value::Int(i) => i.to_string(),
        Value::Double(d) => dotnet_double(*d),
        Value::String(s) => s.clone(),
        Value::List(_) => "System.Collections.Generic.List`1[System.Object]".into(),
        Value::Map(_) => "EtAlii.Adp.Specification.Cel.CelMap".into(),
        Value::Type(name) => (*name).to_owned(),
    }
}

/// A value's written form for families without typed scalars: numbers by the rule above, booleans in lower case.
pub(crate) fn plain(value: &Value, binding: Option<&AttributeBinding>) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Bool(b) => if *b { "true" } else { "false" }.into(),
        Value::Int(_) | Value::Double(_) => number(value.as_number().unwrap_or(0.0), binding.and_then(|b| b.decimals)),
        other => dotnet_string(other),
    }
}

pub(crate) fn is_empty(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(s) => s.is_empty(),
        Value::List(l) => l.is_empty(),
        Value::Map(m) => m.is_empty(),
        _ => false,
    }
}

/// The wire value for a model value through a `map` (FBL section 5.2): the first key in document order whose value
/// matches, unless the value being replaced already maps to it.
pub(crate) fn wire(binding: &AttributeBinding, value: &Value, replaced: Option<&str>) -> Option<String> {
    let map = binding.map.as_ref()?;
    let model = plain(value, Some(binding));
    if let Some(replaced) = replaced
        && map.iter().any(|(k, v)| k == replaced && *v == model)
    {
        return Some(replaced.to_owned());
    }
    map.iter().find(|(_, mapped)| *mapped == model).map(|(w, _)| w.clone())
}

/// A date or a date-time as `keep-precision` reads one: its clock, its fraction's digits and its offset as written.
struct Time {
    date: String,
    clock: Option<String>,
    fraction: String,
    offset: Option<String>,
}

fn parse_time(text: &str) -> Option<Time> {
    let b = text.as_bytes();
    if b.len() < 10 || b[4] != b'-' || b[7] != b'-' || !crate::rules::scalar_conversion::is_date(&text[..10]) {
        return None;
    }
    if b.len() == 10 {
        return Some(Time {
            date: text.to_owned(),
            clock: None,
            fraction: String::new(),
            offset: None,
        });
    }
    if !matches!(b[10], b'T' | b't' | b' ') || b.len() < 19 {
        return None;
    }
    let clock = &text[11..19];
    let cb = clock.as_bytes();
    if cb[2] != b':'
        || cb[5] != b':'
        || !clock
            .bytes()
            .enumerate()
            .all(|(i, c)| i == 2 || i == 5 || c.is_ascii_digit())
    {
        return None;
    }
    let rest = &text[19..];
    let mut fraction = String::new();
    let mut after = rest;
    if let Some(digits) = rest.strip_prefix('.') {
        let count = digits.bytes().take_while(u8::is_ascii_digit).count();
        fraction = digits[..count].to_owned();
        after = &digits[count..];
    }
    let offset = after.trim();
    let offset = if offset.is_empty() {
        None
    } else if offset == "Z" || offset == "z" {
        Some("Z".to_owned())
    } else {
        let o = offset.as_bytes();
        if o.len() != 6 || !matches!(o[0], b'+' | b'-') || o[3] != b':' {
            return None;
        }
        Some(offset.to_owned())
    };
    Some(Time {
        date: text[..10].to_owned(),
        clock: Some(clock.to_owned()),
        fraction,
        offset,
    })
}

/// A date or date-time written with the precision of the value it replaces (FBL section 6.3,
/// `time: "keep-precision"`); none when either is not a date or date-time.
pub(crate) fn keep_precision(replaced: &str, value: &str) -> Option<String> {
    let time = parse_time(value)?;
    let shape = parse_time(replaced)?;
    if shape.clock.is_none() {
        return Some(time.date);
    }
    let mut out = format!("{}T{}", time.date, time.clock.as_deref().unwrap_or("00:00:00"));
    if !shape.fraction.is_empty() {
        let digits = shape.fraction.len().min(7);
        let mut fraction = time.fraction.clone();
        while fraction.len() < 7 {
            fraction.push('0');
        }
        out.push('.');
        out.push_str(&fraction[..digits]);
    }
    if let Some(offset) = &shape.offset {
        if offset == "Z" {
            out.push('Z');
        } else {
            // The new value's own offset; a value written without one is taken as universal time.
            match time.offset.as_deref() {
                Some("Z") | None => out.push_str("+00:00"),
                Some(own) => out.push_str(own),
            }
        }
    }
    Some(out)
}

fn is_placeholder_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | ':' | '-'))
}

/// Renders an `emit` template (FBL section 6.2): placeholders `{name}` replaced by `value`; a `[ … ]` segment
/// written only when every placeholder in it is non-empty. Any other `{` or `}` is literal.
pub(crate) fn render(template: &str, value: &dyn Fn(&str) -> Option<String>) -> String {
    let mut output = String::new();
    let mut i = 0;
    while i < template.len() {
        let c = template.as_bytes()[i];
        if c == b'['
            && let Some(close) = template[i + 1..].find(']').map(|p| p + i + 1)
            && close > 0
        {
            let segment = &template[i + 1..close];
            let complete = std::cell::Cell::new(true);
            let rendered = render_part(segment, &|name| {
                let v = value(name);
                if v.as_deref().is_none_or(str::is_empty) {
                    complete.set(false);
                }
                v
            });
            if complete.get() {
                output.push_str(&rendered);
            }
            i = close + 1;
            continue;
        }
        let next = template[i..].find('[').map(|p| p + i);
        let end = next.unwrap_or(template.len());
        output.push_str(&render_part(&template[i..end], value));
        i = end;
        if let Some(next) = next
            && !template[next + 1..].contains(']')
        {
            output.push('[');
            i += 1;
        }
    }
    output
}

fn render_part(part: &str, value: &dyn Fn(&str) -> Option<String>) -> String {
    let mut output = String::new();
    let mut i = 0;
    while i < part.len() {
        if part.as_bytes()[i] == b'{'
            && let Some(close) = part[i + 1..].find('}').map(|p| p + i + 1)
            && close > i + 1
            && is_placeholder_name(&part[i + 1..close])
        {
            output.push_str(&value(&part[i + 1..close]).unwrap_or_default());
            i = close + 1;
            continue;
        }
        let ch = part[i..].chars().next().expect("a character");
        output.push(ch);
        i += ch.len_utf8();
    }
    output
}

/// One part of an emit template: a placeholder or a literal, and the optional segment it is in (none for none).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EmitPart {
    pub placeholder: Option<String>,
    pub literal: Option<String>,
    pub segment: Option<usize>,
}

/// The placeholders of a template in order, each with whether it sits inside an optional segment.
pub(crate) fn parts(template: &str) -> Vec<EmitPart> {
    let mut parts = Vec::new();
    let mut literal = String::new();
    let mut segment: Option<usize> = None;
    let mut segments = 0;
    let flush = |parts: &mut Vec<EmitPart>, literal: &mut String, segment: Option<usize>| {
        if !literal.is_empty() {
            parts.push(EmitPart {
                placeholder: None,
                literal: Some(std::mem::take(literal)),
                segment,
            });
        }
    };
    let mut i = 0;
    while i < template.len() {
        let c = template[i..].chars().next().expect("a character");
        if c == '[' && segment.is_none() && template[i + 1..].find(']').is_some() {
            flush(&mut parts, &mut literal, segment);
            segment = Some(segments);
            segments += 1;
            i += 1;
            continue;
        }
        if c == ']' && segment.is_some() {
            flush(&mut parts, &mut literal, segment);
            segment = None;
            i += 1;
            continue;
        }
        if c == '{'
            && let Some(close) = template[i + 1..].find('}').map(|p| p + i + 1)
            && close > i + 1
            && is_placeholder_name(&template[i + 1..close])
        {
            flush(&mut parts, &mut literal, segment);
            parts.push(EmitPart {
                placeholder: Some(template[i + 1..close].to_owned()),
                literal: None,
                segment,
            });
            i = close + 1;
            continue;
        }
        literal.push(c);
        i += c.len_utf8();
    }
    flush(&mut parts, &mut literal, segment);
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_written_as_ecmascript_writes_them() {
        assert_eq!(number(640.0, Some(3)), "640");
        assert_eq!(number(120.5, Some(3)), "120.5");
        assert_eq!(number(2.0, None), "2");
        assert_eq!(number(0.1, None), "0.1");
        // As the standalone host does, a value within its tolerance of a whole number is written as a decimal.
        assert_eq!(number(1e-7, None), "0.0000001");
        assert_eq!(number(1.5e21, None), "1.5e+21");
        assert_eq!(number(-1.25, None), "-1.25");
        assert_eq!(dotnet_double(1e15), "1E+15");
    }

    #[test]
    fn a_template_writes_an_optional_segment_only_when_it_is_complete() {
        let values = |name: &str| match name {
            "name" => Some("a".to_owned()),
            _ => None,
        };
        assert_eq!(render("node {name}[ label {label}]", &values), "node a");
        assert_eq!(render("{x} {name}", &values), " a");
        let parts = parts("v {name}[ \"{label}\"]");
        assert_eq!(parts.len(), 5);
        assert_eq!(parts[3].placeholder.as_deref(), Some("label"));
        assert_eq!(parts[3].segment, Some(0));
    }

    #[test]
    fn a_time_keeps_the_precision_of_the_value_it_replaces() {
        assert_eq!(
            keep_precision("2024-01-01", "2025-02-03T10:00:00Z").unwrap(),
            "2025-02-03"
        );
        assert_eq!(
            keep_precision("2024-01-01T00:00:00.000Z", "2025-02-03T10:11:12Z").unwrap(),
            "2025-02-03T10:11:12.000Z"
        );
    }
}
