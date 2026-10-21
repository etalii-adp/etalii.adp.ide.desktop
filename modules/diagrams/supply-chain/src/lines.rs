//! A `.supply` document as the lines it is made of, with every edit a splice of whole lines (etalii.adp spec 030, task
//! T076).
//!
//! Ported from the standalone host's core at `8b6ccfc`: `src/backend/EtAlii.Adp.Documents/LineDocument.cs`,
//! `LineRange.cs`, `LineSplice.cs` (finding a key, matching the document's own indentation, opening a flow-empty
//! section, and quoting a value only where YAML needs it) and `DslNumber.cs`. Every line keeps its own terminator, so a
//! document nothing has spliced is its own bytes again, and a line spliced in takes the terminator most of the
//! document's lines use, CRLF on a tie.

/// One line: its text without its terminator, and the terminator (`\n`, `\r\n`, or none for a last line without one).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub text: String,
    pub ending: &'static str,
}

/// An inclusive range of 0-based line indices: the lines that declare one entry.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct LineRange {
    pub start: usize,
    pub end: usize,
}

impl LineRange {
    pub const fn new(start: usize, end: usize) -> Self {
        LineRange { start, end }
    }

    /// How many lines the range covers.
    pub fn len(&self) -> usize {
        self.end + 1 - self.start
    }

    /// Never: a range holds at least its first line.
    pub fn is_empty(&self) -> bool {
        false
    }

    /// The range grown by `lines` at its end.
    pub fn extended_by(self, lines: usize) -> LineRange {
        LineRange::new(self.start, self.end + lines)
    }
}

/// A hand-authored file as its lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineDocument {
    lines: Vec<Line>,
    dominant_ending: &'static str,
}

impl LineDocument {
    /// Splits `text` into lines, each keeping its own terminator. A file that ends without a newline gives a last line
    /// with no terminator; one that ends with a newline gains no empty line. A tie between LF and CRLF, an empty
    /// document included, goes to CRLF, the standalone repository's house style.
    pub fn parse(text: &str) -> LineDocument {
        let mut lines = Vec::new();
        let (mut crlf, mut lf) = (0, 0);
        let mut start = 0;
        let bytes = text.as_bytes();
        for (index, byte) in bytes.iter().enumerate() {
            if *byte != b'\n' {
                continue;
            }
            let carriage_return = index > start && bytes[index - 1] == b'\r';
            let end = if carriage_return { index - 1 } else { index };
            lines.push(Line {
                text: text[start..end].to_owned(),
                ending: if carriage_return { "\r\n" } else { "\n" },
            });
            if carriage_return {
                crlf += 1;
            } else {
                lf += 1;
            }
            start = index + 1;
        }
        if start < text.len() {
            lines.push(Line {
                text: text[start..].to_owned(),
                ending: "",
            });
        }
        LineDocument {
            lines,
            dominant_ending: if lf > crlf { "\n" } else { "\r\n" },
        }
    }

    pub fn lines(&self) -> &[Line] {
        &self.lines
    }

    /// The text of line `index`.
    pub fn line(&self, index: usize) -> &str {
        &self.lines[index].text
    }

    /// The document as text: byte for byte what [`LineDocument::parse`] was given, for a document nothing spliced.
    pub fn text(&self) -> String {
        let mut text = String::new();
        for line in &self.lines {
            text.push_str(&line.text);
            text.push_str(line.ending);
        }
        text
    }

    /// Replaces the lines of `range` with `replacement`, whose last line keeps the terminator the range's last line
    /// had and the others the dominant one.
    pub fn replace(&mut self, range: LineRange, replacement: &[String]) {
        self.guard(range);
        let ending = self.lines[range.end].ending;
        let count = replacement.len();
        let lines: Vec<Line> = replacement
            .iter()
            .enumerate()
            .map(|(index, text)| Line {
                text: text.clone(),
                ending: if index + 1 == count {
                    ending
                } else {
                    self.dominant_ending
                },
            })
            .collect();
        self.lines.splice(range.start..=range.end, lines);
    }

    /// Inserts `lines` before `index`. Appending after a last line without a terminator gives that line the dominant
    /// one and the new last line none, so an append and its undo come back byte for byte.
    ///
    /// # Panics
    ///
    /// When `index` is past the end, as the standalone host throws there.
    pub fn insert(&mut self, index: usize, lines: &[String]) {
        assert!(
            index <= self.lines.len(),
            "Line {index} is outside a document of {} lines.",
            self.lines.len()
        );
        let appending_to_unterminated =
            index == self.lines.len() && self.lines.last().is_some_and(|line| line.ending.is_empty());
        if appending_to_unterminated && let Some(last) = self.lines.last_mut() {
            last.ending = self.dominant_ending;
        }
        let mut inserted: Vec<Line> = lines
            .iter()
            .map(|text| Line {
                text: text.clone(),
                ending: self.dominant_ending,
            })
            .collect();
        if appending_to_unterminated && let Some(last) = inserted.last_mut() {
            last.ending = "";
        }
        self.lines.splice(index..index, inserted);
    }

    /// Removes the lines of `range`; when the last line goes, its terminator passes to the line that takes its place.
    pub fn remove(&mut self, range: LineRange) {
        self.guard(range);
        let removing_the_end = range.end == self.lines.len() - 1;
        let ending = self.lines[range.end].ending;
        self.lines.drain(range.start..=range.end);
        if removing_the_end && let Some(last) = self.lines.last_mut() {
            last.ending = ending;
        }
    }

    /// Refuses a range that is not entirely inside the document, rather than clamping it: a clamped range splices the
    /// wrong line.
    fn guard(&self, range: LineRange) {
        assert!(
            range.end >= range.start,
            "Line {} cannot precede line {}.",
            range.end,
            range.start
        );
        assert!(
            range.end < self.lines.len(),
            "Lines {}-{} are outside a document of {} lines.",
            range.start,
            range.end,
            self.lines.len()
        );
    }
}

/// The item indentation used when a document has no entry to copy.
pub const DEFAULT_ITEM_INDENT: &str = "  ";

/// The key indentation used when a document has no entry to copy.
pub const DEFAULT_KEY_INDENT: &str = "    ";

/// The leading whitespace of `text`, as `string.TrimStart` counts it.
fn indent_of(text: &str) -> &str {
    &text[..text.len() - text.trim_start().len()]
}

/// The line carrying `key` within `range`, or none; a key written on the item's own `- id:` line is found too
/// (`LineSplice.FindKey`).
pub fn find_key(document: &LineDocument, range: LineRange, key: &str) -> Option<usize> {
    let wanted = format!("{key}:");
    (range.start..=range.end)
        .take_while(|i| *i < document.lines().len())
        .find(|i| {
            let trimmed = document.line(*i).trim_start();
            let trimmed = trimmed.strip_prefix("- ").unwrap_or(trimmed);
            trimmed.starts_with(&wanted)
        })
}

/// Replaces one key's value inside `range`, keeping the line's own indentation, or adds the key directly after the
/// range's first line when it is not there, indented as its neighbours are (`LineSplice.SetKey`).
pub fn set_key(document: &mut LineDocument, range: LineRange, key: &str, value: &str) {
    if let Some(index) = find_key(document, range, key) {
        let existing = document.line(index).to_owned();
        let indent = indent_of(&existing);
        // The dash belongs to the sequence, not to the key, so a key on the `- id:` line keeps its dash.
        let prefix = if existing.trim_start().starts_with("- ") {
            "- "
        } else {
            ""
        };
        document.replace(
            LineRange::new(index, index),
            &[format!("{indent}{prefix}{key}: {value}")],
        );
        return;
    }
    let key_indent = key_indent_within(document, range);
    document.insert(range.start + 1, &[format!("{key_indent}{key}: {value}")]);
}

/// Removes one key's line from within `range`, if it is there (`LineSplice.RemoveKey`).
pub fn remove_key(document: &mut LineDocument, range: LineRange, key: &str) {
    if let Some(index) = find_key(document, range, key) {
        document.remove(LineRange::new(index, index));
    }
}

/// The indentation the keys inside `range` already use; for a one-line entry, the dash's own and two spaces
/// (`LineSplice.KeyIndentWithin`).
pub fn key_indent_within(document: &LineDocument, range: LineRange) -> String {
    for i in range.start + 1..=range.end {
        if i >= document.lines().len() {
            break;
        }
        let text = document.line(i);
        if !text.trim().is_empty() {
            return indent_of(text).to_owned();
        }
    }
    format!("{}{DEFAULT_ITEM_INDENT}", indent_of(document.line(range.start)))
}

/// The item indentation, the gap after the dash and the key indentation the first of `ranges` uses, or the defaults
/// when there is none to copy (`LineSplice.IndentOf`).
pub fn indent_from(document: &LineDocument, ranges: &[LineRange]) -> (String, String, String) {
    let Some(range) = ranges.first() else {
        return (
            DEFAULT_ITEM_INDENT.to_owned(),
            " ".to_owned(),
            DEFAULT_KEY_INDENT.to_owned(),
        );
    };
    let dash = document.line(range.start);
    let item_indent = indent_of(dash).to_owned();
    let after_dash = dash.trim_start();
    let mut gap = " ".to_owned();
    if let Some(rest) = after_dash.strip_prefix('-') {
        gap = indent_of(rest).to_owned();
        if gap.is_empty() {
            gap = " ".to_owned();
        }
    }
    (item_indent, gap, key_indent_within(document, *range))
}

/// Where a new entry goes: after the last of `ranges`, or right after the section key when there is none, a
/// flow-empty `section: []` first opened into a bare `section:`; none when the section is not there at all
/// (`LineSplice.InsertionPointFor`).
pub fn insertion_point(document: &mut LineDocument, ranges: &[LineRange], section_key: &str) -> Option<usize> {
    if let Some(last) = ranges.last() {
        return Some(last.end + 1);
    }
    for i in 0..document.lines().len() {
        let text = document.line(i).to_owned();
        let trimmed = text.trim_start();
        if !trimmed.starts_with(section_key) {
            continue;
        }
        let value = trimmed[section_key.len()..].trim();
        if value == "[]" || value == "[ ]" {
            let indent = indent_of(&text);
            document.replace(LineRange::new(i, i), &[format!("{indent}{section_key}")]);
        }
        return Some(i + 1);
    }
    None
}

/// `value` quoted only where YAML needs it, so an ordinary label stays unquoted (`LineSplice.Quote`). A value with a
/// line break is always double-quoted, each break written as its escape.
pub fn quote(value: &str) -> String {
    if value.is_empty() {
        return "\"\"".to_owned();
    }
    let needs_quoting = value.contains(':')
        || value.contains('#')
        || value.starts_with('-')
        || value.starts_with(' ')
        || value.ends_with(' ')
        || value.starts_with('"')
        || value.starts_with('\'')
        || starts_with_indicator(value)
        || value.contains('\n')
        || value.contains('\r');
    if !needs_quoting {
        return value.to_owned();
    }
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\r', "\\r")
        .replace('\n', "\\n");
    format!("\"{escaped}\"")
}

/// `value` quoted so YAML reads it back as that text: as [`quote`] does, and also where a plain scalar would resolve
/// to a number, a boolean or null (`LineSplice.QuoteText`).
pub fn quote_text(value: &str) -> String {
    let quoted = quote(value);
    if quoted.starts_with('"') || !reads_as_other_than_text(value) {
        return quoted;
    }
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn reads_as_other_than_text(value: &str) -> bool {
    parse_number(value).is_some()
        || ["true", "false", "yes", "no", "null"]
            .iter()
            .any(|word| value.eq_ignore_ascii_case(word))
        || value == "~"
}

/// A plain scalar may not begin with one of YAML's indicators.
fn starts_with_indicator(value: &str) -> bool {
    value.chars().next().is_some_and(|c| "[]{}&*!|>%@`?,".contains(c))
}

/// `text` read as `double.TryParse` reads it with `NumberStyles.Float` in the invariant culture: surrounding white
/// space, a sign, digits with a decimal point and an exponent, or the infinity and NaN symbols; none for anything
/// else. Finite or not, as read: the callers that need a finite number say so.
pub fn parse_number(text: &str) -> Option<f64> {
    let trimmed = text.trim_matches(|c: char| c.is_whitespace());
    let (negative, body) = match trimmed.as_bytes().first() {
        Some(b'-') => (true, &trimmed[1..]),
        Some(b'+') => (false, &trimmed[1..]),
        _ => (false, trimmed),
    };
    let sign = if negative { -1.0 } else { 1.0 };
    if body.eq_ignore_ascii_case("infinity") || body == "∞" {
        return Some(sign * f64::INFINITY);
    }
    if body.eq_ignore_ascii_case("nan") {
        return Some(f64::NAN);
    }
    let bytes = body.as_bytes();
    let mut at = 0;
    let digits = |at: &mut usize| {
        let from = *at;
        while *at < bytes.len() && bytes[*at].is_ascii_digit() {
            *at += 1;
        }
        *at - from
    };
    let whole = digits(&mut at);
    let mut fraction = 0;
    if at < bytes.len() && bytes[at] == b'.' {
        at += 1;
        fraction = digits(&mut at);
    }
    if whole + fraction == 0 {
        return None;
    }
    if at < bytes.len() && (bytes[at] == b'e' || bytes[at] == b'E') {
        at += 1;
        if at < bytes.len() && (bytes[at] == b'+' || bytes[at] == b'-') {
            at += 1;
        }
        if digits(&mut at) == 0 {
            return None;
        }
    }
    if at != bytes.len() {
        return None;
    }
    let mut normalised = body.to_owned();
    if normalised.ends_with('.') {
        normalised.push('0');
    }
    if normalised.starts_with('.') {
        normalised.insert(0, '0');
    }
    let value: f64 = normalised.replace(".e", ".0e").replace(".E", ".0E").parse().ok()?;
    Some(sign * value)
}

/// `Math.Round(value)`: to the nearest whole number, halves to the even one.
pub fn round(value: f64) -> f64 {
    value.round_ties_even()
}

/// `Math.Round(value, 4)`: to four decimals, halves to the even one.
pub fn round4(value: f64) -> f64 {
    if value.abs() >= 1e16 {
        return value;
    }
    (value * 10_000.0).round_ties_even() / 10_000.0
}

/// A number as document text (`DslNumber.Format`, `0.####` in the invariant culture): at most four decimals, rounded
/// half away from zero from the number's fifteen significant digits, and a whole number without a decimal point.
pub fn format_number(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_owned();
    }
    if value.is_infinite() {
        return if value > 0.0 { "Infinity" } else { "-Infinity" }.to_owned();
    }
    let negative = value.is_sign_negative();
    // Fifteen significant digits, as .NET formats a double for a custom pattern.
    let scientific = format!("{:.14e}", value.abs());
    let (mantissa, exponent) = scientific.split_once('e').expect("scientific notation");
    let exponent: i64 = exponent.parse().expect("an exponent");
    let digits: Vec<u8> = mantissa.bytes().filter(u8::is_ascii_digit).map(|b| b - b'0').collect();
    // The digits as a fixed-point number with the point after `point` digits.
    let point = exponent + 1;
    let mut integer: Vec<u8> = Vec::new();
    let mut fraction: Vec<u8> = Vec::new();
    for (index, digit) in digits.iter().enumerate() {
        if (index as i64) < point {
            integer.push(*digit);
        } else {
            fraction.push(*digit);
        }
    }
    if point > digits.len() as i64 {
        integer.extend(std::iter::repeat_n(0, (point - digits.len() as i64) as usize));
    }
    if point < 0 {
        let mut leading = vec![0u8; (-point) as usize];
        leading.extend(fraction);
        fraction = leading;
    }
    if fraction.len() > 4 {
        let up = fraction[4] >= 5;
        fraction.truncate(4);
        if up {
            let mut whole: Vec<u8> = integer.iter().chain(fraction.iter()).copied().collect();
            let mut at = whole.len();
            loop {
                if at == 0 {
                    whole.insert(0, 1);
                    break;
                }
                at -= 1;
                if whole[at] == 9 {
                    whole[at] = 0;
                } else {
                    whole[at] += 1;
                    break;
                }
            }
            let split = whole.len() - 4;
            integer = whole[..split].to_vec();
            fraction = whole[split..].to_vec();
        }
    }
    while fraction.last() == Some(&0) {
        fraction.pop();
    }
    let mut integer: String = integer
        .iter()
        .map(|d| char::from(b'0' + d))
        .collect::<String>()
        .trim_start_matches('0')
        .to_owned();
    if integer.is_empty() {
        integer.push('0');
    }
    let mut text = if negative { format!("-{integer}") } else { integer };
    if !fraction.is_empty() {
        text.push('.');
        text.extend(fraction.iter().map(|d| char::from(b'0' + d)));
    }
    text
}

/// A number as document text, or empty where there is none (`DslNumber.Format(double?)`).
pub fn format_optional(value: Option<f64>) -> String {
    value.map(format_number).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_number_is_written_with_four_decimals_at_most_and_no_point_when_whole() {
        for (value, text) in [
            (12.5, "12.5"),
            (800.0, "800"),
            (0.1 + 0.2, "0.3"),
            (1.23456, "1.2346"),
            (0.00005, "0.0001"),
            (-3.0, "-3"),
            (-0.0, "-0"),
            (1e20, "100000000000000000000"),
            (2.8, "2.8"),
        ] {
            assert_eq!(text, format_number(value), "{value}");
        }
    }

    #[test]
    fn a_number_is_read_as_the_invariant_culture_reads_one() {
        for (text, value) in [
            ("7.25", Some(7.25)),
            (" 12 ", Some(12.0)),
            ("1e3", Some(1000.0)),
            ("-3", Some(-3.0)),
            (".5", Some(0.5)),
            ("5.", Some(5.0)),
            ("lots", None),
            ("1,5", None),
            ("", None),
        ] {
            assert_eq!(value, parse_number(text), "{text}");
        }
        assert!(parse_number("Infinity").is_some_and(f64::is_infinite));
    }

    #[test]
    fn a_text_that_yaml_would_read_as_something_else_is_quoted() {
        for (value, written) in [
            ("Mine", "Mine"),
            ("[EU]", "\"[EU]\""),
            ("yes", "\"yes\""),
            ("12", "\"12\""),
            ("Ore: crushed", "\"Ore: crushed\""),
            ("# not a comment", "\"# not a comment\""),
            ("null", "\"null\""),
            ("", "\"\""),
            ("two\nlines", "\"two\\nlines\""),
        ] {
            assert_eq!(written, quote_text(value), "{value}");
        }
    }

    #[test]
    fn halves_round_to_even() {
        assert_eq!(2.0, round(2.5));
        assert_eq!(4.0, round(3.5));
        assert_eq!(-0.0, round(-0.4));
        assert_eq!(0.1234, round4(0.12344));
    }
}
