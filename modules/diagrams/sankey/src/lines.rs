//! A `.skv` document as the lines it is made of, and the line splices its writer makes (etalii.adp spec 030, task
//! T077).
//!
//! Ported from the standalone host at `8b6ccfc`: the core's `EtAlii.Adp.Documents/LineDocument.cs`, `LineRange.cs`,
//! `LineSplice.cs` and `DslNumber.cs`, which the Sankey module wrote through. Every line keeps its own terminator, so
//! a document nothing has spliced is its own bytes again, a line spliced in takes the terminator most of the
//! document's lines use, and how the file ends is a property of the file rather than of its last line. The splices
//! assume the shape `.skv` has, a sequence of `- key: value` items under a section key; they are not a YAML editor.

/// One line: its text without its terminator, and the terminator (`\n`, `\r\n`, or none for a last line without one).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub text: String,
    pub ending: &'static str,
}

/// An inclusive range of zero-based line indices: the lines that declare one construct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LineRange {
    pub start: usize,
    pub end: usize,
}

impl LineRange {
    pub fn new(start: usize, end: usize) -> Self {
        LineRange { start, end }
    }

    /// How many lines the range covers.
    pub fn len(&self) -> usize {
        self.end + 1 - self.start
    }

    /// Never: a range covers at least its first line.
    pub fn is_empty(&self) -> bool {
        false
    }
}

/// A hand-authored file as its lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineDocument {
    lines: Vec<Line>,
    dominant_ending: &'static str,
}

impl LineDocument {
    /// Splits `text` into lines, each keeping its own terminator. A file that ends without a newline gives a last
    /// line with no terminator; one that ends with a newline gains no empty line. A tie between LF and CRLF, an empty
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

    /// The terminator most of the document's lines use, which a line spliced in takes.
    pub fn dominant_ending(&self) -> &'static str {
        self.dominant_ending
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
    ///
    /// # Panics
    ///
    /// When `range` is not inside the document, as the standalone host throws: a clamped range splices the wrong line.
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

    /// Inserts `lines` before line `index`. Appended after a last line without a terminator, that line gains one and
    /// the new last line goes without, so an append and its undo give back the same bytes.
    ///
    /// # Panics
    ///
    /// When `index` is past the end of the document.
    pub fn insert(&mut self, index: usize, lines: &[String]) {
        assert!(
            index <= self.lines.len(),
            "Line {index} is outside a document of {} lines.",
            self.lines.len()
        );
        let appending_to_unterminated =
            index == self.lines.len() && self.lines.last().is_some_and(|last| last.ending.is_empty());
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

    /// Removes the lines of `range`. When the last line goes, the line that takes its place takes its terminator.
    ///
    /// # Panics
    ///
    /// When `range` is not inside the document.
    pub fn remove(&mut self, range: LineRange) {
        self.guard(range);
        let removing_the_end = range.end == self.lines.len() - 1;
        let ending = self.lines[range.end].ending;
        self.lines.drain(range.start..=range.end);
        if removing_the_end && let Some(last) = self.lines.last_mut() {
            last.ending = ending;
        }
    }

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

/// The leading whitespace of `text`, as `TrimStart` leaves it.
fn indent_of(text: &str) -> &str {
    &text[..text.len() - text.trim_start().len()]
}

/// The index of the first line carrying `section_key` after its indentation, or none.
pub fn find_section(document: &LineDocument, section_key: &str) -> Option<usize> {
    document
        .lines()
        .iter()
        .position(|line| line.text.trim_start().starts_with(section_key))
}

/// The index of the line carrying `key` within `range`, a key written on the item's own `- ` line included.
pub fn find_key(document: &LineDocument, range: LineRange, key: &str) -> Option<usize> {
    let wanted = format!("{key}:");
    (range.start..=range.end.min(document.lines().len().saturating_sub(1))).find(|index| {
        let trimmed = document.line(*index).trim_start();
        let trimmed = trimmed.strip_prefix("- ").unwrap_or(trimmed);
        trimmed.starts_with(&wanted)
    })
}

/// Replaces one key's value within `range`, keeping the line's indentation and its dash, or adds the key after the
/// range's first line, indented as its neighbours are.
pub fn set_key(document: &mut LineDocument, range: LineRange, key: &str, value: &str) {
    if let Some(index) = find_key(document, range, key) {
        let existing = document.line(index);
        let indent = indent_of(existing).to_owned();
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

/// Removes one key's line from within `range`, if it is there.
pub fn remove_key(document: &mut LineDocument, range: LineRange, key: &str) {
    if let Some(index) = find_key(document, range, key) {
        document.remove(LineRange::new(index, index));
    }
}

/// The indentation the keys inside `range` already use: that of its first non-blank line after the first, else the
/// first line's indentation and two spaces, which is what the `- ` occupies.
pub fn key_indent_within(document: &LineDocument, range: LineRange) -> String {
    for index in range.start + 1..=range.end.min(document.lines().len().saturating_sub(1)) {
        let text = document.line(index);
        if !text.trim().is_empty() {
            return indent_of(text).to_owned();
        }
    }
    format!("{}{DEFAULT_ITEM_INDENT}", indent_of(document.line(range.start)))
}

/// The item indentation, the gap after the dash and the key indentation the first of `ranges` uses, or the defaults
/// when there is none to copy.
pub fn indent_of_entries(document: &LineDocument, ranges: &[LineRange]) -> (String, String, String) {
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
        let found = indent_of(rest);
        if !found.is_empty() {
            gap = found.to_owned();
        }
    }
    (item_indent, gap, key_indent_within(document, *range))
}

/// Where a new entry goes: after the last of `ranges`, else on the line after `section_key`, opening a section
/// written flow-empty (`nodes: []`) into a bare key first; none when the section key is absent.
pub fn insertion_point_for(document: &mut LineDocument, ranges: &[LineRange], section_key: &str) -> Option<usize> {
    if let Some(last) = ranges.last() {
        return Some(last.end + 1);
    }
    for index in 0..document.lines().len() {
        let text = document.line(index);
        let trimmed = text.trim_start();
        if !trimmed.starts_with(section_key) {
            continue;
        }
        let value = trimmed[section_key.len()..].trim();
        if value == "[]" || value == "[ ]" {
            let indent = indent_of(text).to_owned();
            document.replace(LineRange::new(index, index), &[format!("{indent}{section_key}")]);
        }
        return Some(index + 1);
    }
    None
}

/// A value quoted only where YAML needs it; a line break is always written double-quoted, as its escape.
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
        || value.starts_with(|c| "[]{}&*!|>%@`?,".contains(c))
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

/// A text value quoted so YAML reads it back as that text: as [`quote`] does, and also where a plain scalar would
/// read as a number, a boolean or null.
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

/// A number as .NET's `double.TryParse` with `NumberStyles.Float` and the invariant culture reads it: surrounding
/// whitespace, a sign, a point and an exponent; `Infinity` and `NaN` too, which every caller refuses as not finite.
pub fn parse_number(text: &str) -> Option<f64> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let unsigned = trimmed.trim_start_matches(['+', '-']);
    if unsigned.len() + 1 < trimmed.len() {
        return None;
    }
    match unsigned {
        "Infinity" | "∞" => {
            return Some(if trimmed.starts_with('-') {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            });
        }
        "NaN" => return Some(f64::NAN),
        _ => {}
    }
    // Rust also reads `inf` and `nan`, which .NET does not.
    if !unsigned.starts_with(|c: char| c.is_ascii_digit() || c == '.') {
        return None;
    }
    trimmed.parse().ok()
}

/// A whole number as `int.TryParse` with `NumberStyles.Integer` reads it: surrounding whitespace and a sign.
pub fn parse_int(text: &str) -> Option<i32> {
    let trimmed = text.trim();
    let digits = trimmed.trim_start_matches(['+', '-']);
    if digits.is_empty() || digits.len() + 1 < trimmed.len() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    trimmed.parse().ok()
}

/// `value` as document text (`DslNumber.Format`, .NET's `0.####`): at most four decimals, none when whole.
pub fn dsl_number(value: f64) -> String {
    custom_number(value, 4, false)
}

/// `value` as a reader reads it (`SankeyFormat`'s `#,0.##`): thousands grouped, at most two decimals, none when whole.
pub fn grouped_number(value: f64) -> String {
    custom_number(value, 2, true)
}

/// `value` written as a .NET custom numeric format of `#,0.##`'s or `0.####`'s shape does: the double taken to the 15
/// significant digits a custom format reads, then rounded half away from zero to `decimals`, trailing zeros dropped.
fn custom_number(value: f64, decimals: usize, grouping: bool) -> String {
    if value.is_nan() {
        return "NaN".to_owned();
    }
    if value.is_infinite() {
        return if value > 0.0 { "∞" } else { "-∞" }.to_owned();
    }
    let negative = value < 0.0;
    // 15 significant digits, as `d.ddddddddddddddde±x`.
    let scientific = format!("{:.14e}", value.abs());
    let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
    let exponent: i32 = exponent.parse().unwrap_or(0);
    let mut digits: Vec<u8> = mantissa.bytes().filter(u8::is_ascii_digit).map(|b| b - b'0').collect();
    // The point sits after `point` digits; pad with zeros on the left when it is before the first.
    let mut point = exponent + 1;
    if point <= 0 {
        let pad = (1 - point) as usize;
        digits.splice(0..0, std::iter::repeat_n(0, pad));
        point = 1;
    }
    let keep = point as usize + decimals;
    if digits.len() < keep {
        digits.resize(keep, 0);
    }
    if digits.len() > keep {
        let up = digits[keep] >= 5;
        digits.truncate(keep);
        if up {
            let mut at = keep;
            let mut carried = true;
            while carried && at > 0 {
                at -= 1;
                if digits[at] == 9 {
                    digits[at] = 0;
                } else {
                    digits[at] += 1;
                    carried = false;
                }
            }
            if carried {
                digits.insert(0, 1);
            }
        }
    }
    let point = digits.len() - decimals;
    let integer: String = digits[..point]
        .iter()
        .map(|d| char::from(b'0' + d))
        .collect::<String>()
        .trim_start_matches('0')
        .to_owned();
    let integer = if integer.is_empty() { "0".to_owned() } else { integer };
    let mut fraction: String = digits[point..].iter().map(|d| char::from(b'0' + d)).collect();
    while fraction.ends_with('0') {
        fraction.pop();
    }
    let integer = if grouping { group(&integer) } else { integer };
    let zero = integer == "0" && fraction.is_empty();
    let sign = if negative && !zero { "-" } else { "" };
    if fraction.is_empty() {
        format!("{sign}{integer}")
    } else {
        format!("{sign}{integer}.{fraction}")
    }
}

fn group(integer: &str) -> String {
    let mut out = String::with_capacity(integer.len() + integer.len() / 3);
    for (index, c) in integer.chars().enumerate() {
        if index > 0 && (integer.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_written_as_the_standalone_host_writes_them() {
        assert_eq!(dsl_number(12.5), "12.5");
        assert_eq!(dsl_number(1.0), "1");
        assert_eq!(dsl_number(1.5625), "1.5625");
        assert_eq!(dsl_number(1.953125), "1.9531");
        assert_eq!(dsl_number(0.00005), "0.0001");
        assert_eq!(dsl_number(0.0), "0");
        assert_eq!(dsl_number(1302.0), "1302");
        assert_eq!(dsl_number(0.1 + 0.2), "0.3");
        assert_eq!(grouped_number(1302.5), "1,302.5");
        assert_eq!(grouped_number(1_234_567.0), "1,234,567");
        assert_eq!(grouped_number(0.25), "0.25");
        assert_eq!(grouped_number(0.005), "0.01");
        assert_eq!(grouped_number(999.999), "1,000");
    }

    #[test]
    fn numbers_read_as_dot_net_reads_them() {
        assert_eq!(parse_number(" 12.5 "), Some(12.5));
        assert_eq!(parse_number("-3"), Some(-3.0));
        assert_eq!(parse_number("1e3"), Some(1000.0));
        assert_eq!(parse_number("lots"), None);
        assert_eq!(parse_number("inf"), None);
        assert_eq!(parse_number(""), None);
        assert_eq!(parse_int(" 7 "), Some(7));
        assert_eq!(parse_int("1.5"), None);
    }
}
