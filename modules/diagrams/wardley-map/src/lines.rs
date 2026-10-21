//! An `.owm` document as the lines it is made of, with edits applied by splicing whole lines (etalii.adp spec 030,
//! task T070).
//!
//! Ported from the standalone host at `8b6ccfc`: the core's `EtAlii.Adp.Documents/LineDocument.cs` and `LineRange.cs`,
//! which the Wardley map module moved onto in code-structure T127, `DslNumber.cs`, and `LineLexer.StripComment` with
//! the module's comment policy (`//`, quoted or not, except after a `:`). Every line keeps its own terminator, so a
//! document nothing has spliced is its own bytes again, and a line that is spliced in takes the terminator most of the
//! document's lines use.

/// One line of a document: its text without its terminator, and the terminator (`\n`, `\r\n`, or none for a last
/// line without one).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub text: String,
    pub ending: &'static str,
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

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// The terminator most of the document's lines use, which a line spliced in takes.
    pub fn dominant_ending(&self) -> &'static str {
        self.dominant_ending
    }

    /// The text of line `number` (1-based), or none outside the document.
    pub fn line_at(&self, number: u32) -> Option<&str> {
        let index = (number as usize).checked_sub(1)?;
        self.lines.get(index).map(|line| line.text.as_str())
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

    /// Replaces the lines `start..=end` (0-based) with `replacement`, whose last line keeps the terminator the range's
    /// last line had and the others the dominant one.
    pub fn replace(&mut self, start: usize, end: usize, replacement: &[String]) -> Result<(), String> {
        self.guard(start, end)?;
        let ending = self.lines[end].ending;
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
        self.lines.splice(start..=end, lines);
        Ok(())
    }

    /// Inserts `lines` before the 0-based `index`. Appending after a last line without a terminator gives that line
    /// the dominant one and the new last line none, so an append and its undo come back byte for byte.
    pub fn insert(&mut self, index: usize, lines: &[String]) -> Result<(), String> {
        if index > self.lines.len() {
            return Err(format!(
                "Line {index} is outside a document of {} lines.",
                self.lines.len()
            ));
        }
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
        Ok(())
    }

    /// Removes the lines `start..=end` (0-based); when the last line goes, its terminator passes to the line that
    /// takes its place.
    pub fn remove(&mut self, start: usize, end: usize) -> Result<(), String> {
        self.guard(start, end)?;
        let removing_the_end = end == self.lines.len() - 1;
        let ending = self.lines[end].ending;
        self.lines.drain(start..=end);
        if removing_the_end && let Some(last) = self.lines.last_mut() {
            last.ending = ending;
        }
        Ok(())
    }

    /// Refuses a range that is not entirely inside the document, rather than clamping it: a clamped range splices the
    /// wrong line.
    fn guard(&self, start: usize, end: usize) -> Result<(), String> {
        if end < start {
            return Err(format!("Line {end} cannot precede line {start}."));
        }
        if end >= self.lines.len() {
            return Err(format!(
                "Lines {start}-{end} are outside a document of {} lines.",
                self.lines.len()
            ));
        }
        Ok(())
    }
}

/// `text` up to its first comment: a `//`, quoted or not, unless a `:` stands straight before it, which makes it the
/// `//` of a URL (the module's `CommentPolicy(["//"], OutsideQuotesOnly: false, NotAfter: ':')`).
pub fn strip_comment(text: &str) -> &str {
    let bytes = text.as_bytes();
    let mut index = 0;
    while index + 1 < bytes.len() {
        if bytes[index] == b'/' && bytes[index + 1] == b'/' {
            if index > 0 && bytes[index - 1] == b':' {
                index += 2;
                continue;
            }
            return &text[..index];
        }
        index += 1;
    }
    text
}

/// A number as the text of a document or a property row (`DslNumber.Format`, .NET's `0.####`): invariant culture, at
/// most four decimals rounded half away from zero on the number's 15 significant digits, and a whole number without a
/// decimal point.
pub fn format_number(value: f64) -> String {
    if !value.is_finite() {
        return value.to_string();
    }
    if value == 0.0 {
        return "0".to_owned();
    }
    // The 15 significant digits .NET formats a double with before a custom format rounds it.
    let scientific = format!("{:.14e}", value.abs());
    let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
    let exponent: i32 = exponent.parse().unwrap_or(0);
    let digits: Vec<u8> = mantissa.bytes().filter(u8::is_ascii_digit).map(|d| d - b'0').collect();
    // The digits as an integer part and a fraction part, in decimal.
    let point = exponent + 1;
    let (mut whole, mut fraction): (Vec<u8>, Vec<u8>) = if point <= 0 {
        let mut fraction = vec![0; (-point) as usize];
        fraction.extend_from_slice(&digits);
        (vec![0], fraction)
    } else if point as usize >= digits.len() {
        let mut whole = digits.clone();
        whole.extend(std::iter::repeat_n(0, point as usize - digits.len()));
        (whole, Vec::new())
    } else {
        (digits[..point as usize].to_vec(), digits[point as usize..].to_vec())
    };
    if fraction.len() > 4 {
        let round_up = fraction[4] >= 5;
        fraction.truncate(4);
        if round_up {
            let mut carry = true;
            for digit in fraction.iter_mut().rev() {
                if !carry {
                    break;
                }
                if *digit == 9 {
                    *digit = 0;
                } else {
                    *digit += 1;
                    carry = false;
                }
            }
            if carry {
                for digit in whole.iter_mut().rev() {
                    if !carry {
                        break;
                    }
                    if *digit == 9 {
                        *digit = 0;
                    } else {
                        *digit += 1;
                        carry = false;
                    }
                }
                if carry {
                    whole.insert(0, 1);
                }
            }
        }
    }
    while fraction.last() == Some(&0) {
        fraction.pop();
    }
    let first = whole.iter().position(|d| *d != 0).unwrap_or(whole.len() - 1);
    let mut text: String = whole[first..].iter().map(|d| char::from(b'0' + d)).collect();
    if !fraction.is_empty() {
        text.push('.');
        text.extend(fraction.iter().map(|d| char::from(b'0' + d)));
    }
    if value < 0.0 { format!("-{text}") } else { text }
}

/// A number as the document writes it, read in the invariant culture; none when it is not one (`double.TryParse` with
/// `NumberStyles.Float`).
pub fn parse_number(text: &str) -> Option<f64> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let body = text.strip_prefix(['+', '-']).unwrap_or(text);
    // `NumberStyles.Float` takes digits, one point and an exponent; not Rust's `inf` or `nan`.
    if !body
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | '+' | '-'))
        || !body.chars().any(|c| c.is_ascii_digit())
    {
        return None;
    }
    let text = if text.starts_with('.') || text.starts_with("-.") || text.starts_with("+.") {
        text.replacen('.', "0.", 1)
    } else {
        text.to_owned()
    };
    let text = if text.ends_with('.') { format!("{text}0") } else { text };
    text.parse::<f64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_number_is_written_with_at_most_four_decimals_and_no_trailing_zeros() {
        assert_eq!(format_number(0.5), "0.5");
        assert_eq!(format_number(1.0), "1");
        assert_eq!(format_number(0.0), "0");
        assert_eq!(format_number(0.123456), "0.1235");
        assert_eq!(format_number(0.12345), "0.1235");
        assert_eq!(format_number(0.99995), "1");
        assert_eq!(format_number(0.30000000000000004), "0.3");
        assert_eq!(format_number(-57.0), "-57");
        assert_eq!(format_number(0.0001), "0.0001");
        assert_eq!(format_number(0.00004), "0");
        assert_eq!(format_number(12.5), "12.5");
    }

    #[test]
    fn a_comment_starts_at_two_slashes_unless_a_colon_stands_before_them() {
        assert_eq!(
            strip_comment("component A [0.5, 0.5] // why"),
            "component A [0.5, 0.5] "
        );
        assert_eq!(
            strip_comment("url site [https://example.com/x]"),
            "url site [https://example.com/x]"
        );
        assert_eq!(strip_comment("// all of it"), "");
    }

    #[test]
    fn a_number_is_read_in_the_invariant_culture() {
        assert_eq!(parse_number(" 0.79 "), Some(0.79));
        assert_eq!(parse_number("-.5"), Some(-0.5));
        assert_eq!(parse_number("1."), Some(1.0));
        assert_eq!(parse_number("inf"), None);
        assert_eq!(parse_number("0,79"), None);
        assert_eq!(parse_number(""), None);
        assert_eq!(parse_number("."), None);
    }
}
