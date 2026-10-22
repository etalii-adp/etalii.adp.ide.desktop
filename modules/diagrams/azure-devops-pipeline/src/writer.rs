//! Changes one property of one element, or adds, removes or moves a whole one, by splicing the lines that declare it
//! and nothing else (etalii.adp spec 030, task T078).
//!
//! Ported from the standalone host's `src/diagrams/azure-devops-pipeline/backend/EtAlii.Adp.Diagram.AzureDevOpsPipeline/
//! PipelineWriter.cs` at `8b6ccfc`. Three properties are written and no more: `displayName`, `dependsOn` and `enabled`.
//! A property is found inside its element's range at the element's own indentation; a new one goes after the value of
//! the element's first key; a comment trailing the line it replaces is kept; a value is quoted only where leaving it
//! bare would change what it means. Nothing reflows.

use crate::lines::{LineDocument, LineRange};
use crate::model::EditTarget;

/// Writes through the lines of one document.
pub struct PipelineWriter<'a> {
    document: &'a mut LineDocument,
}

fn spaces(count: usize) -> String {
    " ".repeat(count)
}

/// Whether `text` declares `key` at `indent`: the key itself, then its colon.
fn starts_with_key(text: &str, indent: usize, key: &str) -> bool {
    let end = indent + key.len();
    text.len() > end && text.get(indent..end) == Some(key) && text.as_bytes()[end] == b':'
}

/// The trailing comment on a line with the spaces before it, or empty: a `#` after whitespace and outside quotes.
fn comment_on(text: &str) -> String {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut quote: Option<char> = None;
    for (position, (index, character)) in chars.iter().enumerate() {
        if let Some(open) = quote {
            if *character == open {
                quote = None;
            }
            continue;
        }
        if *character == '"' || *character == '\'' {
            quote = Some(*character);
            continue;
        }
        if *character == '#' && position > 0 && chars[position - 1].1.is_whitespace() {
            let before = text[..*index].trim_end_matches(' ');
            return text[before.len()..].to_owned();
        }
    }
    String::new()
}

/// A value as a YAML scalar, quoted only where leaving it bare would change what it means; an expression as written.
pub fn scalarise(value: &str) -> String {
    if value.is_empty() {
        return "\"\"".to_owned();
    }
    if value.starts_with("${{") || value.starts_with("$[") {
        return value.to_owned();
    }
    let first = value.chars().next().unwrap_or_default();
    let needs_quotes = value != value.trim()
        || value.contains(": ")
        || value.ends_with(':')
        || value.contains(" #")
        || value.contains('\n')
        || "-?:,[]{}#&*!|>'\"%@`".contains(first);
    if needs_quotes {
        format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        value.to_owned()
    }
}

impl<'a> PipelineWriter<'a> {
    pub fn new(document: &'a mut LineDocument) -> Self {
        PipelineWriter { document }
    }

    /// Sets an element's `displayName`, or removes it when `value` is empty; whether the document changed.
    pub fn set_display_name(&mut self, element: &EditTarget, value: &str) -> Result<bool, String> {
        self.set_scalar(element, "displayName", value)
    }

    /// Sets an element's `enabled`, or removes it when `value` is empty; whether the document changed.
    pub fn set_enabled(&mut self, element: &EditTarget, value: &str) -> Result<bool, String> {
        self.set_scalar(element, "enabled", value)
    }

    /// Sets an element's `dependsOn`: one name as a scalar, several as a block list, none as `[]`.
    pub fn set_depends_on(&mut self, element: &EditTarget, names: &[String]) -> Result<bool, String> {
        self.guard(element)?;
        let indent = self.property_indent(element);
        let lines: Vec<String> = match names {
            [] => vec![format!("{}dependsOn: []", spaces(indent))],
            [one] => vec![format!("{}dependsOn: {}", spaces(indent), scalarise(one))],
            many => std::iter::once(format!("{}dependsOn:", spaces(indent)))
                .chain(
                    many.iter()
                        .map(|name| format!("{}- {}", spaces(indent + 2), scalarise(name))),
                )
                .collect(),
        };
        Ok(self.write(element, "dependsOn", &lines))
    }

    /// Removes an element's `dependsOn` entirely, putting it back on the schema's default.
    pub fn clear_depends_on(&mut self, element: &EditTarget) -> Result<bool, String> {
        self.guard(element)?;
        Ok(self.write(element, "dependsOn", &[]))
    }

    /// Adds a whole element after `after`, or as the first entry of the list whose key is on `list_key_line`.
    pub fn insert_element(
        &mut self,
        after: Option<&EditTarget>,
        list_key_line: usize,
        lines: &[String],
    ) -> Result<(), String> {
        if lines.is_empty() {
            return Ok(());
        }
        if let Some(after) = after {
            self.guard(after)?;
            self.document.insert(after.lines.end + 1, lines);
            return Ok(());
        }
        if list_key_line >= self.document.lines().len() {
            return Err(format!(
                "Line {list_key_line} is outside a document of {} lines.",
                self.document.lines().len()
            ));
        }
        self.document.insert(list_key_line + 1, lines);
        Ok(())
    }

    /// Removes a whole element, and the blank lines after it where a blank line comes before it too; the lines
    /// removed, which put back where they were undo the removal.
    pub fn remove_element(&mut self, element: &EditTarget) -> Result<Vec<String>, String> {
        self.guard(element)?;
        let start = element.lines.start;
        let mut end = element.lines.end;
        let lines = self.document.lines();
        let separated = start == 0 || lines[start - 1].is_blank();
        while separated && end + 1 < lines.len() && lines[end + 1].is_blank() {
            end += 1;
        }
        let removed: Vec<String> = (start..=end).map(|i| lines[i].text.clone()).collect();
        self.document.remove(LineRange::new(start, end));
        Ok(removed)
    }

    /// Moves one element to sit after another in the same list, or to the front of it; whether the document changed.
    pub fn move_element(
        &mut self,
        element: &EditTarget,
        after: Option<&EditTarget>,
        list_key_line: usize,
    ) -> Result<bool, String> {
        self.guard(element)?;
        if let Some(after) = after {
            self.guard(after)?;
            if after.lines.start == element.lines.start {
                return Ok(false);
            }
        }
        let moved: Vec<String> = (element.lines.start..=element.lines.end)
            .map(|i| self.document.line(i).to_owned())
            .collect();
        let mut target = match after {
            None => list_key_line + 1,
            Some(after) => after.lines.end + 1,
        };
        if target > element.lines.end {
            target -= element.lines.len();
        }
        self.document.remove(element.lines);
        self.document.insert(target, &moved);
        Ok(true)
    }

    fn set_scalar(&mut self, element: &EditTarget, key: &str, value: &str) -> Result<bool, String> {
        self.guard(element)?;
        if value.is_empty() {
            return Ok(self.write(element, key, &[]));
        }
        let indent = self.property_indent(element);
        let trailing = match self.find(element, key, indent) {
            Some(existing) => comment_on(self.document.line(existing.start)),
            None => String::new(),
        };
        Ok(self.write(
            element,
            key,
            &[format!("{}{key}: {}{trailing}", spaces(indent), scalarise(value))],
        ))
    }

    /// Replaces the property's lines, inserts them where it has none, removes them when `lines` is empty.
    fn write(&mut self, element: &EditTarget, key: &str, lines: &[String]) -> bool {
        let indent = self.property_indent(element);
        if let Some(range) = self.find(element, key, indent) {
            if lines.is_empty() {
                self.document.remove(range);
                return true;
            }
            if self.unchanged(range, lines) {
                return false;
            }
            self.document.replace(range, lines);
            return true;
        }
        if lines.is_empty() {
            return false;
        }
        let at = self.end_of_first_value(element) + 1;
        self.document.insert(at, lines);
        true
    }

    /// The last line of the value of the element's first key.
    fn end_of_first_value(&self, element: &EditTarget) -> usize {
        let indent = self.property_indent(element);
        let mut end = element.lines.start;
        for index in end + 1..=element.lines.end {
            let line = &self.document.lines()[index];
            if line.is_blank() {
                continue;
            }
            if line.indent() <= indent {
                break;
            }
            end = index;
        }
        end
    }

    fn unchanged(&self, range: LineRange, lines: &[String]) -> bool {
        range.len() == lines.len()
            && lines
                .iter()
                .enumerate()
                .all(|(offset, line)| self.document.line(range.start + offset) == line)
    }

    /// The lines a property occupies within an element: its key line and everything indented under it.
    fn find(&self, element: &EditTarget, key: &str, indent: usize) -> Option<LineRange> {
        let lines = self.document.lines();
        for index in element.lines.start..=element.lines.end {
            let line = &lines[index];
            if line.indent() != indent || !starts_with_key(&line.text, indent, key) {
                continue;
            }
            let mut end = index;
            while end < element.lines.end && !lines[end + 1].is_blank() && lines[end + 1].indent() > indent {
                end += 1;
            }
            return Some(LineRange::new(index, end));
        }
        None
    }

    /// The column an element's own properties sit at: two past a sequence entry's dash, else where it is.
    fn property_indent(&self, element: &EditTarget) -> usize {
        let first = &self.document.lines()[element.lines.start];
        let indent = first.indent();
        let text = &first.text[indent..];
        if text.starts_with("- ") || text == "-" {
            indent + 2
        } else {
            indent
        }
    }

    fn guard(&self, element: &EditTarget) -> Result<(), String> {
        if element.lines.end >= self.document.lines().len() {
            return Err(format!(
                "{}-{} is outside a document of {} lines.",
                element.lines.start,
                element.lines.end,
                self.document.lines().len()
            ));
        }
        if !element.is_editable() {
            return Err(format!(
                "{} cannot be edited here. {}",
                element.id,
                element.read_only_reason()
            ));
        }
        Ok(())
    }
}
