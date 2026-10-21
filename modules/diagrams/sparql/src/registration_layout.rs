//! The `layout:` block of a registration: the positions a person dragged nodes and regions to, the one thing a SPARQL
//! query diagram writes (etalii.adp spec 030, task T084; the same block the Ansible structure diagram writes, T079).
//!
//! Ported from the standalone host's core at `8b6ccfc`: `src/backend/EtAlii.Adp.Hierarchy/RegistrationLayout.cs`, as
//! its `SetRegistrationLayoutCommand` writes it for this module. The block is rewritten whole: a `layout:` line and an
//! indented `<id>: <x> <y>` line per position, in ordinal order of ids, numbers written as .NET's `0.###` writes them,
//! every new line ended with CRLF (`AdpFileWriter.NewLine`); every byte above and below it is kept, each line with its
//! own ending. A block with no entries is removed. The block lands where it was, or after the header region: the
//! first line and the contiguous `key: value` and blank lines after it. Malformed entries are ignored on reading: the
//! block is metadata, and a hand-mangled line never stops a diagram from opening.

use std::collections::BTreeMap;

/// The line every new line of the block ends with, on every platform (`AdpFileWriter.NewLine`).
const NEW_LINE: &str = "\r\n";

const BLOCK_HEADER: &str = "layout:";

/// One line of the registration: its content and its own terminator.
struct RawLine<'a> {
    content: &'a str,
    terminator: &'a str,
}

fn lines_of(text: &str) -> Vec<RawLine<'_>> {
    let mut lines = Vec::new();
    let mut start = 0;
    for (index, byte) in text.bytes().enumerate() {
        if byte == b'\n' {
            let carriage = index > start && text.as_bytes()[index - 1] == b'\r';
            let end = if carriage { index - 1 } else { index };
            lines.push(RawLine {
                content: &text[start..end],
                terminator: if carriage { "\r\n" } else { "\n" },
            });
            start = index + 1;
        }
    }
    if start < text.len() {
        lines.push(RawLine {
            content: &text[start..],
            terminator: "",
        });
    }
    lines
}

/// The entry a line is, when it is one: indented, an id, `": "` and two numbers.
fn entry_of(content: &str) -> Option<(String, (f64, f64))> {
    if !content.starts_with(char::is_whitespace) {
        return None;
    }
    let trimmed = content.trim();
    let colon = trimmed.rfind(": ").filter(|&c| c > 0)?;
    let parts: Vec<&str> = trimmed[colon + 2..].split(' ').filter(|p| !p.is_empty()).collect();
    let [x, y] = parts.as_slice() else {
        return None;
    };
    Some((trimmed[..colon].to_owned(), (parse(x)?, parse(y)?)))
}

/// A number as .NET's `double.TryParse` with `NumberStyles.Float` reads it, in the invariant culture.
fn parse(text: &str) -> Option<f64> {
    let lower = text.to_ascii_lowercase();
    if lower.contains("inf") || lower.contains("nan") {
        return None;
    }
    text.parse().ok()
}

/// The stored positions of a registration's text, by id; none when the block is absent.
pub fn read(text: &str) -> BTreeMap<String, (f64, f64)> {
    let mut positions = BTreeMap::new();
    let mut in_block = false;
    for line in lines_of(text) {
        if !in_block {
            in_block = line.content.trim_end() == BLOCK_HEADER;
            continue;
        }
        if let Some((id, position)) = entry_of(line.content) {
            positions.insert(id, position);
            continue;
        }
        if line.content.trim().is_empty() {
            continue;
        }
        break;
    }
    positions
}

/// `text` with the position of `id` set, and the position it replaced, none when it had none.
pub fn set_position(text: &str, id: &str, position: (f64, f64)) -> (String, Option<(f64, f64)>) {
    let mut positions = read(text);
    let previous = positions.insert(id.to_owned(), position);
    (write_block(text, &positions), previous)
}

/// `text` without the position of `id`; none when it had none. Removing the last one removes the block.
pub fn remove_position(text: &str, id: &str) -> Option<String> {
    let mut positions = read(text);
    positions.remove(id)?;
    Some(write_block(text, &positions))
}

/// The block's line range, start inclusive and end exclusive, or none when the text has none.
fn find_block(lines: &[RawLine<'_>]) -> Option<(usize, usize)> {
    let start = lines.iter().position(|l| l.content.trim_end() == BLOCK_HEADER)?;
    let mut end = start + 1;
    while end < lines.len() && (entry_of(lines[end].content).is_some() || lines[end].content.trim().is_empty()) {
        end += 1;
    }
    Some((start, end))
}

fn is_header_line(content: &str) -> bool {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return true;
    }
    match trimmed.find(':') {
        Some(colon) if colon > 0 => {
            !trimmed.starts_with(BLOCK_HEADER)
                && trimmed[..colon]
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        }
        _ => false,
    }
}

/// Where a fresh block goes: after the first line and the contiguous run of headers that follow it.
fn end_of_header_region(lines: &[RawLine<'_>]) -> usize {
    let mut index = lines.len().min(1);
    while index < lines.len() && is_header_line(lines[index].content) {
        index += 1;
    }
    index
}

fn append_block(out: &mut String, positions: &BTreeMap<String, (f64, f64)>) {
    if positions.is_empty() {
        return;
    }
    out.push_str(BLOCK_HEADER);
    out.push_str(NEW_LINE);
    for (id, (x, y)) in positions {
        out.push_str("  ");
        out.push_str(id);
        out.push_str(": ");
        out.push_str(&number(*x));
        out.push(' ');
        out.push_str(&number(*y));
        out.push_str(NEW_LINE);
    }
}

/// `text` with its layout block replaced by one holding `positions`, every other byte kept.
fn write_block(text: &str, positions: &BTreeMap<String, (f64, f64)>) -> String {
    let lines = lines_of(text);
    let block = find_block(&lines);
    let insert_at = block.map_or_else(|| end_of_header_region(&lines), |(start, _)| start);
    let mut out = String::with_capacity(text.len() + 64);
    for (index, line) in lines.iter().enumerate() {
        if index == insert_at {
            append_block(&mut out, positions);
        }
        if block.is_some_and(|(start, end)| index >= start && index < end) {
            continue;
        }
        out.push_str(line.content);
        out.push_str(line.terminator);
    }
    if insert_at >= lines.len() {
        append_block(&mut out, positions);
    }
    out
}

/// `value` as .NET's custom format `0.###` writes it in the invariant culture: the double taken to the 15 significant
/// digits a custom format reads, rounded half away from zero to three decimals, trailing zeros dropped.
pub fn number(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_owned();
    }
    if value.is_infinite() {
        return if value > 0.0 { "∞" } else { "-∞" }.to_owned();
    }
    const DECIMALS: usize = 3;
    let negative = value < 0.0;
    let scientific = format!("{:.14e}", value.abs());
    let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
    let exponent: i32 = exponent.parse().unwrap_or(0);
    let mut digits: Vec<u8> = mantissa.bytes().filter(u8::is_ascii_digit).map(|b| b - b'0').collect();
    let mut point = exponent + 1;
    if point <= 0 {
        let pad = usize::try_from(1 - point).unwrap_or(0);
        digits.splice(0..0, std::iter::repeat_n(0, pad));
        point = 1;
    }
    let keep = usize::try_from(point).unwrap_or(0) + DECIMALS;
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
    let split = digits.len() - DECIMALS;
    let integer: String = digits[..split].iter().map(|d| char::from(b'0' + d)).collect();
    let integer = integer.trim_start_matches('0');
    let integer = if integer.is_empty() { "0" } else { integer };
    let mut fraction: String = digits[split..].iter().map(|d| char::from(b'0' + d)).collect();
    while fraction.ends_with('0') {
        fraction.pop();
    }
    let zero = integer == "0" && fraction.is_empty();
    let sign = if negative && !zero { "-" } else { "" };
    if fraction.is_empty() {
        format!("{sign}{integer}")
    } else {
        format!("{sign}{integer}.{fraction}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_written_as_dotnet_writes_0_hash_hash_hash() {
        assert_eq!(number(640.0), "640");
        assert_eq!(number(769.545_599_999_999_9), "769.546");
        assert_eq!(number(232.1088), "232.109");
        assert_eq!(number(-60.0625), "-60.063");
        assert_eq!(number(0.0001), "0");
        assert_eq!(number(-0.0001), "0");
        assert_eq!(number(363.200_000_000_000_05), "363.2");
    }

    #[test]
    fn a_first_position_adds_the_block_after_the_headers_with_crlf() {
        let (text, previous) = set_position("w3c/sparql\nbody: q.rq\n", "var:x", (10.0, 20.0));
        assert_eq!(text, "w3c/sparql\nbody: q.rq\nlayout:\r\n  var:x: 10 20\r\n");
        assert_eq!(previous, None);
    }

    #[test]
    fn the_block_is_rewritten_in_ordinal_order_and_what_follows_it_is_kept() {
        let text = "a/b\nbody: x\nlayout:\n  z: 1 2\n\nnotes\n";
        let (text, previous) = set_position(text, "a", (3.5, -4.0));
        assert_eq!(text, "a/b\nbody: x\nlayout:\r\n  a: 3.5 -4\r\n  z: 1 2\r\nnotes\n");
        assert_eq!(previous, None);
        let (_, previous) = set_position(&text, "z", (0.0, 0.0));
        assert_eq!(previous, Some((1.0, 2.0)));
    }

    #[test]
    fn removing_the_last_position_removes_the_block() {
        let (text, _) = set_position("a/b\n", "x", (1.0, 1.0));
        assert_eq!(remove_position(&text, "x").as_deref(), Some("a/b\n"));
        assert_eq!(remove_position("a/b\n", "x"), None);
    }

    #[test]
    fn a_mangled_entry_is_ignored_and_ends_nothing_but_the_block() {
        let positions = read("a/b\nlayout:\n  x: 1 2\n  y: one 2\n  z: 3 4\n");
        assert_eq!(positions.len(), 1);
        assert_eq!(positions["x"], (1.0, 2.0));
    }
}
