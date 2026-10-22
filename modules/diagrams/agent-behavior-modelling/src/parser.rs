//! Reads the behavior tree out of a Markdown file: the first bullet list under the first heading called Behavior
//! (etalii.adp spec 030, task T074). Never fails, and never reads anything else in the file.
//!
//! Ported from the standalone host's `AbmParser.cs` at `8b6ccfc`, its regular expressions written out as the code
//! they match:
//!
//! - Everything outside the list is the author's (the title, the prose about the agent, other sections, code blocks)
//!   and is neither modelled nor touched; it survives every edit by never being spliced.
//! - Nesting is indentation, as Markdown means it: an item indented further than the one before it is that item's
//!   child, an item at or left of an earlier one closes it, and a plain line indented under an item is that item's
//!   notes. A tab counts as four columns.
//! - An item without a keyword is read as a Do rather than dropped, so a list written by hand still draws, and the
//!   problem it records says how it was read.

use crate::kinds;
use crate::lines::{LineDocument, LineRange};
use crate::model::{Model, Node, Problem, rule_ids};

/// The model of `document`.
pub fn parse(document: &LineDocument) -> Model {
    let lines: Vec<&str> = document.lines().iter().map(|l| l.text.as_str()).collect();
    let fenced = fenced_lines(&lines);

    // The section: the first Behavior heading, up to the next heading of its level or above.
    let mut section = None;
    for (index, line) in lines.iter().enumerate() {
        if !fenced[index]
            && let Some((level, text)) = heading(line)
            && is_behavior(text)
        {
            section = Some((index, level));
            break;
        }
    }
    let Some((start, level)) = section else {
        return Model::default();
    };

    let mut section_end = lines.len().saturating_sub(1);
    for index in start + 1..lines.len() {
        if !fenced[index] && heading(lines[index]).is_some_and(|(l, _)| l <= level) {
            section_end = index - 1;
            break;
        }
    }

    // The list: its first item sets the column the roots sit at.
    let list_start = (start + 1..=section_end).find(|i| *i < lines.len() && !fenced[*i] && item(lines[*i]).is_some());
    let Some(list_start) = list_start else {
        return Model::new(Vec::new(), Some(start), section_end, 0, Vec::new());
    };

    let base_indent = column_of(lines[list_start]);
    let mut raws: Vec<RawItem> = Vec::new();
    for (index, text) in lines.iter().enumerate().take(section_end + 1).skip(list_start) {
        if text.trim().is_empty() {
            continue; // a blank line inside a list is a loose list, not its end
        }
        if let Some(found) = item(text)
            && column_of(text) >= base_indent
        {
            raws.push(RawItem {
                line: index,
                text: (*text).to_owned(),
                item: found,
                note_lines: Vec::new(),
            });
            continue;
        }
        if column_of(text) > base_indent
            && let Some(last) = raws.last_mut()
        {
            last.note_lines.push(index);
            continue;
        }
        break; // a line at the list's own column that is not an item ends the list
    }

    build(&raws, &lines, start, section_end, base_indent)
}

/// The column `text`'s first non-blank character sits at, a tab counting four.
pub fn column_of(text: &str) -> usize {
    let mut column = 0;
    for character in text.chars() {
        match character {
            ' ' => column += 1,
            '\t' => column += 4 - (column % 4),
            _ => break,
        }
    }
    column
}

/// One list item as the item pattern splits it.
#[derive(Debug, Clone)]
struct Item {
    marker: char,
    /// The whitespace between the marker and the text.
    space: String,
    /// Everything after that whitespace.
    text: String,
}

struct RawItem {
    line: usize,
    text: String,
    item: Item,
    note_lines: Vec<usize>,
}

fn build(raws: &[RawItem], lines: &[&str], section_line: usize, section_end: usize, base_indent: usize) -> Model {
    let mut problems = Vec::new();
    let mut parents: Vec<Option<usize>> = vec![None; raws.len()];
    let mut child_counts = vec![0usize; raws.len()];
    let mut ids = vec![String::new(); raws.len()];
    let mut stack: Vec<usize> = Vec::new();
    let mut root_count = 0;

    for index in 0..raws.len() {
        let indent = column_of(&raws[index].text);
        while stack.last().is_some_and(|top| column_of(&raws[*top].text) >= indent) {
            stack.pop();
        }
        match stack.last().copied() {
            None => {
                root_count += 1;
                ids[index] = root_count.to_string();
            }
            Some(parent) => {
                parents[index] = Some(parent);
                child_counts[parent] += 1;
                ids[index] = format!("{}.{}", ids[parent], child_counts[parent]);
            }
        }
        stack.push(index);
    }

    // A node's last line: its notes, then whatever its last descendant reaches.
    let mut ends: Vec<usize> = raws
        .iter()
        .map(|raw| raw.note_lines.last().copied().unwrap_or(raw.line))
        .collect();
    for index in (0..raws.len()).rev() {
        if let Some(parent) = parents[index] {
            ends[parent] = ends[parent].max(ends[index]);
        }
    }

    let mut nodes = Vec::with_capacity(raws.len());
    for (index, raw) in raws.iter().enumerate() {
        let indent = column_of(&raw.text);
        let content_indent = indent + 1 + column_width(&raw.item.space, indent + 1);
        let (kind, retry_count, label, keyword_text, has_keyword) = read_text(raw.item.text.trim_end());
        if !has_keyword {
            problems.push(Problem {
                rule_id: rule_ids::NO_KEYWORD.to_owned(),
                message: format!(
                    "\"{}\" has no keyword, so it is read as a Do. Start it with one, such as **Do:** or **Check:**.",
                    shorten(&label)
                ),
                line: raw.line,
            });
        }
        let notes = match (raw.note_lines.first(), raw.note_lines.last()) {
            (Some(first), Some(last)) => (*first..=*last)
                .map(|line| dedent_text(lines[line], content_indent))
                .collect::<Vec<_>>()
                .join("\n")
                .trim_matches('\n')
                .to_owned(),
            _ => String::new(),
        };
        nodes.push(Node {
            id: ids[index].clone(),
            kind: kind.to_owned(),
            label,
            retry_count,
            notes,
            has_keyword,
            line: raw.line,
            notes_range: match (raw.note_lines.first(), raw.note_lines.last()) {
                (Some(first), Some(last)) => Some(LineRange::new(*first, *last)),
                _ => None,
            },
            subtree_end: ends[index],
            indent,
            content_indent,
            marker: raw.item.marker,
            keyword_text,
            parent_id: parents[index].map(|p| ids[p].clone()),
            child_ids: (0..raws.len())
                .filter(|child| parents[*child] == Some(index))
                .map(|child| ids[child].clone())
                .collect(),
        });
    }

    Model::new(nodes, Some(section_line), section_end, base_indent, problems)
}

/// The item's kind, retry count, label, keyword as written and whether it has one; a Do labelled with the whole text
/// when it carries no keyword.
fn read_text(text: &str) -> (&'static str, i32, String, String, bool) {
    if let Some((keyword, label)) = bold_lead(text) {
        let mut keyword = keyword.trim().to_owned();
        let mut label = label.to_owned();
        if let Some(stripped) = keyword.strip_suffix(':') {
            keyword = stripped.trim_end().to_owned();
        } else if let Some(stripped) = label.strip_prefix(':') {
            label = stripped.to_owned();
        }
        if let Some((kind, retry_count)) = kinds::from_keyword(&keyword) {
            return (kind.id, retry_count, label.trim().to_owned(), keyword, true);
        }
    }
    (kinds::ACTION, 0, text.trim().to_owned(), String::new(), false)
}

/// `^\*\*(?<keyword>[^*]+?)\*\*(?<label>.*)$`: the keyword in bold at the start of the item, and what follows it.
fn bold_lead(text: &str) -> Option<(&str, &str)> {
    let rest = text.strip_prefix("**")?;
    let star = rest.find('*')?;
    if star == 0 || !rest[star..].starts_with("**") {
        return None;
    }
    Some((&rest[..star], &rest[star + 2..]))
}

/// Removes up to `columns` columns of leading whitespace, and the trailing whitespace.
fn dedent_text(text: &str, columns: usize) -> String {
    let mut column = 0;
    let mut index = 0;
    let bytes = text.as_bytes();
    while index < bytes.len() && column < columns && (bytes[index] == b' ' || bytes[index] == b'\t') {
        column += if bytes[index] == b'\t' { 4 - (column % 4) } else { 1 };
        index += 1;
    }
    text[index..].trim_end().to_owned()
}

fn column_width(whitespace: &str, start_column: usize) -> usize {
    let mut column = start_column;
    for character in whitespace.chars() {
        column += if character == '\t' { 4 - (column % 4) } else { 1 };
    }
    column - start_column
}

/// A label cut to 60 characters for a message.
pub fn shorten(text: &str) -> String {
    if text.chars().count() <= 60 {
        text.to_owned()
    } else {
        format!("{}...", text.chars().take(57).collect::<String>())
    }
}

fn is_behavior(heading_text: &str) -> bool {
    let text = heading_text.trim();
    text.eq_ignore_ascii_case("Behavior") || text.eq_ignore_ascii_case("Behaviour")
}

/// Which lines are inside a fenced code block, fences included: never read as headings or items.
fn fenced_lines(lines: &[&str]) -> Vec<bool> {
    let mut fenced = vec![false; lines.len()];
    let mut open: Option<String> = None;
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        match &open {
            None => {
                if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                    open = Some(trimmed[..3].to_owned());
                    fenced[index] = true;
                }
            }
            Some(fence) => {
                fenced[index] = true;
                if trimmed.starts_with(fence.as_str()) {
                    open = None;
                }
            }
        }
    }
    fenced
}

/// `^ {0,3}(?<hashes>#{1,6})\s+(?<text>.*?)\s*#*\s*$`: a heading's level and its text.
fn heading(line: &str) -> Option<(usize, &str)> {
    let spaces = line.len() - line.trim_start_matches(' ').len();
    if spaces > 3 {
        return None;
    }
    let rest = &line[spaces..];
    let hashes = rest.len() - rest.trim_start_matches('#').len();
    if !(1..=6).contains(&hashes) {
        return None;
    }
    let after = &rest[hashes..];
    let text = after.trim_start();
    if text.len() == after.len() {
        return None; // `\s+` needs at least one
    }
    let text = text.trim_end().trim_end_matches('#').trim_end();
    Some((hashes, text))
}

/// The item pattern: indentation, a marker (`-`, `*` or `+`), whitespace and the text, where neither the line after
/// its indentation nor the text is a CommonMark thematic break (three or more marks of one kind, optionally separated
/// by whitespace).
fn item(line: &str) -> Option<Item> {
    let rest = line.trim_start_matches([' ', '\t']);
    let marker = rest.chars().next()?;
    if !matches!(marker, '-' | '*' | '+') {
        return None;
    }
    if matches!(marker, '-' | '*')
        && rest.chars().all(|c| c == marker || c == ' ' || c == '\t')
        && rest.chars().filter(|c| *c == marker).count() >= 3
    {
        return None;
    }
    let after = &rest[1..];
    let text = after.trim_start_matches([' ', '\t']);
    if text.len() == after.len() {
        return None; // `[ \t]+` needs at least one
    }
    let first = text.chars().next()?;
    if first.is_whitespace() {
        return None;
    }
    if matches!(first, '-' | '*' | '_')
        && text.chars().all(|c| matches!(c, '-' | '*' | '_') || c.is_whitespace())
        && text.chars().filter(|c| matches!(c, '-' | '*' | '_')).count() >= 3
    {
        return None;
    }
    Some(Item {
        marker,
        space: after[..after.len() - text.len()].to_owned(),
        text: text.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_heading_is_read_with_its_level_and_without_its_closing_hashes() {
        assert_eq!(heading("## Behavior"), Some((2, "Behavior")));
        assert_eq!(heading("   ### behaviour ###  "), Some((3, "behaviour")));
        assert_eq!(heading("    ## Behavior"), None);
        assert_eq!(heading("####### Behavior"), None);
        assert_eq!(heading("##Behavior"), None);
    }

    #[test]
    fn the_bold_lead_is_the_text_between_the_first_two_pairs_of_stars() {
        assert_eq!(bold_lead("**Do:** Act"), Some(("Do:", " Act")));
        assert_eq!(bold_lead("**Do**: Act"), Some(("Do", ": Act")));
        assert_eq!(bold_lead("****"), None);
        assert_eq!(bold_lead("**Do*: Act"), None);
    }
}
