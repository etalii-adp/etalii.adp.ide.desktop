//! Scalars as the YAML 1.2 core schema reads them, and as FBL section 6.3 writes them (the standalone host's
//! `Files/Yaml/YamlScalars`): typing a plain scalar, deciding whether a string can be written plain, quoting, and
//! decoding quoted scalars with YAML's line folding.

use std::sync::LazyLock;

use adp_cel::Value;
use regex::Regex;

fn pattern(expression: &str) -> Regex {
    Regex::new(expression).expect("the scalar patterns compile")
}

static DECIMAL: LazyLock<Regex> = LazyLock::new(|| pattern("^[-+]?[0-9]+$"));
static OCTAL: LazyLock<Regex> = LazyLock::new(|| pattern("^0o[0-7]+$"));
static HEXADECIMAL: LazyLock<Regex> = LazyLock::new(|| pattern("^0x[0-9a-fA-F]+$"));
static FLOAT: LazyLock<Regex> = LazyLock::new(|| pattern(r"^[-+]?(\.[0-9]+|[0-9]+(\.[0-9]*)?)([eE][-+]?[0-9]+)?$"));
static SPECIAL: LazyLock<Regex> = LazyLock::new(|| pattern(r"^[-+]?(\.inf|\.Inf|\.INF)$|^(\.nan|\.NaN|\.NAN)$"));
static TIMESTAMP: LazyLock<Regex> = LazyLock::new(|| pattern("^[0-9][0-9][0-9][0-9]-[0-9][0-9]?-[0-9][0-9]?([Tt ]|$)"));
static YAML11_NUMBER: LazyLock<Regex> = LazyLock::new(|| {
    pattern(
        r"^[-+]?([0-9][0-9_]*)?\.?[0-9_]*([eE][-+]?[0-9]+)?$|^0b[01_]+$|^[-+]?0[0-7_]+$|^[-+]?[0-9][0-9_]*(:[0-5]?[0-9])+(\.[0-9_]*)?$",
    )
});

/// A plain scalar's value by the YAML 1.2 core schema: null, bool, int, float, else the string.
pub(crate) fn typed(plain: &str) -> Value {
    match plain {
        "" | "~" | "null" | "Null" | "NULL" => return Value::Null,
        "true" | "True" | "TRUE" => return Value::Bool(true),
        "false" | "False" | "FALSE" => return Value::Bool(false),
        _ => {}
    }
    if DECIMAL.is_match(plain)
        && let Ok(l) = plain.parse::<i64>()
    {
        return Value::Int(l);
    }
    if OCTAL.is_match(plain)
        && let Ok(l) = i64::from_str_radix(&plain[2..], 8)
    {
        return Value::Int(l);
    }
    if HEXADECIMAL.is_match(plain)
        && let Ok(l) = u64::from_str_radix(&plain[2..], 16)
    {
        return Value::Int(l as i64);
    }
    if FLOAT.is_match(plain)
        && let Ok(d) = plain.parse::<f64>()
    {
        return Value::Double(d);
    }
    if SPECIAL.is_match(plain) {
        if plain.to_lowercase().contains("nan") {
            return Value::Double(f64::NAN);
        }
        return Value::Double(if plain.starts_with('-') {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        });
    }
    Value::from(plain)
}

/// FBL section 6.3: a string is plain-safe when it is not empty; has no leading or trailing whitespace; no line
/// break or control character; does not start with an indicator; contains neither `: ` nor ` #` and does not end
/// with `:`; and reads back as the same string under the YAML 1.2 core schema and as a string under YAML 1.1,
/// unless the attribute's type is what it would read as (`time_typed` for a date or date-time attribute).
pub(crate) fn is_plain_safe(value: &str, time_typed: bool) -> bool {
    if !is_plain_syntax(value) {
        return false;
    }
    if !matches!(typed(value), Value::String(_)) {
        return false;
    }
    let lower = value.to_lowercase();
    if matches!(
        lower.as_str(),
        "yes" | "no" | "on" | "off" | "y" | "n" | "true" | "false" | "null" | "~"
    ) {
        return false;
    }
    if YAML11_NUMBER.is_match(value) && value.chars().any(|c| c.is_ascii_digit()) {
        return false;
    }
    !TIMESTAMP.is_match(value) || time_typed
}

/// Whether YAML's syntax lets `value` be written plain, whatever a schema then types it as: the plain-safe test
/// without its reading back as a string (FBL 0.5 section 6.3).
pub(crate) fn is_plain_syntax(value: &str) -> bool {
    let (Some(first), Some(last)) = (value.chars().next(), value.chars().last()) else {
        return false;
    };
    if first.is_whitespace() || last.is_whitespace() {
        return false;
    }
    if value.chars().any(|c| c == '\n' || c == '\r' || c.is_control()) {
        return false;
    }
    if "-?:,[]{}#&*!|>'\"%@`".contains(first) {
        return false;
    }
    !value.contains(": ") && !value.contains(" #") && !value.ends_with(':')
}

/// Whether `value` can be written in plain style, which a kept plain style and an attribute's `style: plain` ask
/// for (FBL section 6.3): plain-safe, or a `-` followed by a non-space that is otherwise plain-safe, which YAML
/// reads as a plain string rather than a sequence entry (`-3200-01`), as long as it reads back as that string
/// under both schemas.
pub(crate) fn is_plain_writable(value: &str, time_typed: bool) -> bool {
    if is_plain_safe(value, time_typed) {
        return true;
    }
    let chars: Vec<char> = value.chars().collect();
    if chars.len() < 2 || chars[0] != '-' || chars[1].is_whitespace() || value.starts_with("---") {
        return false;
    }
    if !is_plain_safe(&format!("x{}", &value[1..]), time_typed) {
        return false;
    }
    matches!(typed(value), Value::String(_))
        && !(YAML11_NUMBER.is_match(value) && value.chars().any(|c| c.is_ascii_digit()))
}

pub(crate) fn double_quoted(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if c.is_control() => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

pub(crate) fn single_quoted(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

/// Decodes a double-quoted scalar's content (without its quotes), folding line breaks as YAML does.
pub(crate) fn decode_double(content: &str) -> String {
    let folded: Vec<char> = fold(content).chars().collect();
    let mut out = String::with_capacity(folded.len());
    let mut i = 0;
    while i < folded.len() {
        let c = folded[i];
        if c != '\\' || i + 1 >= folded.len() {
            out.push(c);
            i += 1;
            continue;
        }
        i += 1;
        let e = folded[i];
        match e {
            'n' => out.push('\n'),
            't' | '\t' => out.push('\t'),
            'r' => out.push('\r'),
            '0' => out.push('\0'),
            'a' => out.push('\u{7}'),
            'b' => out.push('\u{8}'),
            'e' => out.push('\u{1b}'),
            'f' => out.push('\u{c}'),
            'v' => out.push('\u{b}'),
            ' ' => out.push(' '),
            '/' => out.push('/'),
            '"' => out.push('"'),
            '\\' => out.push('\\'),
            'N' => out.push('\u{85}'),
            '_' => out.push(' '),
            'x' => out.push_str(&hex(&folded, &mut i, 2)),
            'u' => out.push_str(&hex(&folded, &mut i, 4)),
            'U' => out.push_str(&hex(&folded, &mut i, 8)),
            other => {
                out.push('\\');
                out.push(other);
            }
        }
        i += 1;
    }
    out
}

fn hex(text: &[char], i: &mut usize, digits: usize) -> String {
    let available = digits.min(text.len().saturating_sub(*i + 1));
    if available == 0 {
        return String::new();
    }
    let slice: String = text[*i + 1..*i + 1 + available].iter().collect();
    let Ok(code) = u32::from_str_radix(&slice, 16) else {
        return String::new();
    };
    *i += available;
    char::from_u32(code).map(|c| c.to_string()).unwrap_or_default()
}

pub(crate) fn decode_single(content: &str) -> String {
    fold(content).replace("''", "'")
}

/// Line folding of flow scalars: a line break and the whitespace around it become a space, an empty line a newline.
pub(crate) fn fold(content: &str) -> String {
    if !content.contains('\n') {
        return content.to_owned();
    }
    let normalised = content.replace("\r\n", "\n");
    let lines: Vec<&str> = normalised.split('\n').collect();
    let mut out = lines[0].trim_end_matches([' ', '\t']).to_owned();
    let mut empty = 0;
    for (i, raw) in lines.iter().enumerate().skip(1) {
        let last = i == lines.len() - 1;
        let line = if last {
            raw.trim_start_matches([' ', '\t'])
        } else {
            raw.trim_matches([' ', '\t'])
        };
        if line.is_empty() && !last {
            empty += 1;
            continue;
        }
        if empty > 0 {
            out.push_str(&"\n".repeat(empty));
        } else {
            out.push(' ');
        }
        empty = 0;
        out.push_str(line);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_core_schema_types_plain_scalars() {
        assert_eq!(typed("~"), Value::Null);
        assert_eq!(typed("42"), Value::Int(42));
        assert_eq!(typed("0x1F"), Value::Int(31));
        assert_eq!(typed("1.5"), Value::Double(1.5));
        assert_eq!(typed("yes"), Value::from("yes"));
    }

    #[test]
    fn a_string_is_written_plain_only_when_it_reads_back_as_itself() {
        assert!(is_plain_safe("hello world", false));
        assert!(!is_plain_safe("yes", false));
        assert!(!is_plain_safe("1_000", false));
        assert!(!is_plain_safe("2024-01-01", false));
        assert!(is_plain_safe("2024-01-01", true));
        assert!(is_plain_writable("-3200-01", false));
        assert_eq!(double_quoted("a\"b\u{1}"), "\"a\\\"b\\u0001\"");
        assert_eq!(decode_double("a\\x41\\u00e9"), "aAé");
        assert_eq!(fold("a\n  b\n\n  c"), "a b\nc");
    }
}
