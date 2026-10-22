//! The `layout:` block of a behavior model's `.adp` registration: the heights an author dragged rows to, under the
//! place ids of the row and everything beneath it (etalii.adp spec 030, task T074).
//!
//! Ported from the standalone host's `EtAlii.Adp.Hierarchy/RegistrationLayout.cs` at `8b6ccfc`, which the behavior
//! model's drag and its Arrange diagram wrote through: a drag replaces every stored position at once (`Replace`), and
//! Arrange drops them all (`Prune` with no live id). The block is ADP's own, so it is written whole: a `layout:` line,
//! then one `  <id>: <x> <y>` entry per position in ordinal order of the ids, each number with at most three decimals,
//! every new line ended with CRLF (`AdpFileWriter.NewLine`). Everything above and below the block keeps its bytes, and
//! an empty set removes the block.

use crate::layout::Positions;

const BLOCK_HEADER: &str = "layout:";

/// The line ending of a line the block writes (`AdpFileWriter.NewLine`).
const NEW_LINE: &str = "\r\n";

/// One line with its own terminator.
struct RawLine<'a> {
    content: &'a str,
    terminator: &'a str,
}

/// The stored positions; empty when the block is absent. A malformed entry is ignored, and the block ends at the
/// first line that is neither an entry nor blank.
pub fn read(registration: &[u8]) -> Positions {
    let text = String::from_utf8_lossy(registration);
    let mut positions = Positions::new();
    let mut in_block = false;
    for line in raw_lines(&text) {
        if !in_block {
            in_block = line.content.trim_end() == BLOCK_HEADER;
            continue;
        }
        if let Some((id, position)) = entry(line.content) {
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

/// `registration` with its block replaced by one holding `positions`, every byte above and below it kept; the block
/// lands where it was, or after the header region when there was none, and an empty set removes it.
pub fn with_block(registration: &[u8], positions: &Positions) -> Vec<u8> {
    let text = String::from_utf8_lossy(registration);
    let lines = raw_lines(&text);
    let (block_start, block_end) = find_block(&lines);
    let insert_at = block_start.unwrap_or_else(|| end_of_header_region(&lines));
    let mut rebuilt = String::new();
    for (index, line) in lines.iter().enumerate() {
        if index == insert_at {
            append_block(&mut rebuilt, positions);
        }
        if block_start.is_some_and(|start| index >= start && index < block_end) {
            continue;
        }
        rebuilt.push_str(line.content);
        rebuilt.push_str(line.terminator);
    }
    if insert_at >= lines.len() {
        append_block(&mut rebuilt, positions);
    }
    rebuilt.into_bytes()
}

fn append_block(builder: &mut String, positions: &Positions) {
    if positions.is_empty() {
        return;
    }
    builder.push_str(BLOCK_HEADER);
    builder.push_str(NEW_LINE);
    let mut ids: Vec<&String> = positions.keys().collect();
    ids.sort();
    for id in ids {
        let (x, y) = positions[id];
        builder.push_str(&format!("  {id}: {} {}{NEW_LINE}", number(x), number(y)));
    }
}

/// A number as .NET's `ToString("0.###", InvariantCulture)` writes it.
pub fn number(value: f64) -> String {
    let rounded = (value * 1000.0).round() / 1000.0;
    if rounded == 0.0 {
        return if value < 0.0 { "-0".to_owned() } else { "0".to_owned() };
    }
    let text = format!("{rounded:.3}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// The block's line range `[start, end)`, or none.
fn find_block(lines: &[RawLine<'_>]) -> (Option<usize>, usize) {
    for (index, line) in lines.iter().enumerate() {
        if line.content.trim_end() != BLOCK_HEADER {
            continue;
        }
        let mut end = index + 1;
        while end < lines.len() && (entry(lines[end].content).is_some() || lines[end].content.trim().is_empty()) {
            end += 1;
        }
        return (Some(index), end);
    }
    (None, 0)
}

/// Where a fresh block goes: after the MIME line and the contiguous run of headers that follow it.
fn end_of_header_region(lines: &[RawLine<'_>]) -> usize {
    let mut index = lines.len().min(1);
    while index < lines.len() && is_header_line(lines[index].content) {
        index += 1;
    }
    index
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

/// An indented `<id>: <x> <y>` entry.
fn entry(content: &str) -> Option<(String, (f64, f64))> {
    if !content.chars().next().is_some_and(char::is_whitespace) {
        return None;
    }
    let trimmed = content.trim();
    let colon = trimmed.rfind(": ").filter(|c| *c > 0)?;
    let parts: Vec<&str> = trimmed[colon + 2..].split(' ').filter(|p| !p.is_empty()).collect();
    let [x, y] = parts.as_slice() else {
        return None;
    };
    let (x, y) = (x.parse::<f64>().ok()?, y.parse::<f64>().ok()?);
    Some((trimmed[..colon].to_owned(), (x, y)))
}

fn raw_lines(text: &str) -> Vec<RawLine<'_>> {
    let mut lines = Vec::new();
    let bytes = text.as_bytes();
    let mut start = 0;
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b'\n' {
            let carriage = index > start && bytes[index - 1] == b'\r';
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_number_has_at_most_three_decimals_and_no_trailing_zeros() {
        assert_eq!(number(360.0), "360");
        assert_eq!(number(12.5), "12.5");
        assert_eq!(number(1.0 / 3.0), "0.333");
        assert_eq!(number(-14.0), "-14");
    }

    #[test]
    fn the_block_is_read_and_written_whole_and_an_empty_one_is_removed() {
        let registration = b"etalii/agent-behavior-modelling\nlayout:\n  1.2: 360 160\n";
        assert_eq!(read(registration).get("1.2"), Some(&(360.0, 160.0)));
        assert_eq!(
            with_block(registration, &Positions::new()),
            b"etalii/agent-behavior-modelling\n".to_vec()
        );
        let mut positions = Positions::new();
        positions.insert("1.10".into(), (0.0, 1.5));
        positions.insert("1.2".into(), (-3.0, 4.0));
        assert_eq!(
            String::from_utf8(with_block(b"etalii/agent-behavior-modelling\r\n", &positions)).unwrap(),
            "etalii/agent-behavior-modelling\r\nlayout:\r\n  1.10: 0 1.5\r\n  1.2: -3 4\r\n"
        );
    }
}
