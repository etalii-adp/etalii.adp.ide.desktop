//! Rewrites the statements an edit touches, and nothing else (etalii.adp spec 030, task T070).
//!
//! Ported from the standalone host's `src/diagrams/wardley-map/backend/EtAlii.Adp.Diagram.WardleyMap/WardleyWriter.cs`
//! at `8b6ccfc`. Every function splices a span inside one line rather than rebuilding the line from the model, so the
//! trailing comment, the decorators, the label offset and the author's spacing survive. Names are matched by span,
//! never by substring replacement: renaming `Tea` must not touch `Cup of Tea`. A function that cannot find what it
//! is to change answers false and changes nothing, and the caller refuses the edit.

use std::sync::LazyLock;

use regex::Regex;

use crate::lines::{LineDocument, format_number};
use crate::model::{
    WardleyComponent, WardleyCoordinate, WardleyLink, WardleyMap, WardleyPipeline, WardleyPipelineChild,
    WardleyPipelineForm,
};
use crate::parser::inertia_word;

fn compile(pattern: &str) -> Regex {
    Regex::new(pattern).expect("a valid expression")
}

/// The two numbers of a coordinate pair, captured separately so the separator survives.
static PAIR: LazyLock<Regex> = LazyLock::new(|| compile(r"\[\s*(?<first>-?[\d.]+)\s*,\s*(?<second>-?[\d.]+)\s*\]"));
/// The single number a pipeline child carries.
static SINGLE_VALUE: LazyLock<Regex> = LazyLock::new(|| compile(r"\[\s*(?<value>-?[\d.]+)\s*\]"));
/// The name of a `component`, `anchor` or `submap`, up to its coordinates.
static DECLARATION_NAME: LazyLock<Regex> =
    LazyLock::new(|| compile(r"(?i)^\s*(?:component|anchor|submap)\s+(?<name>.*?)\s*\["));
/// The component an `evolve` names, before any `->` rename or trailing maturity.
static EVOLVE_NAME: LazyLock<Regex> = LazyLock::new(|| compile(r"(?i)^\s*evolve\s+(?<name>.*?)\s*(?:->|-?[\d.]+\s*$)"));
/// The parent a `pipeline` names, in either form.
static PIPELINE_PARENT: LazyLock<Regex> = LazyLock::new(|| compile(r"(?i)^\s*pipeline\s+(?<name>.*?)\s*(?:\[|\{|$)"));
/// A link's two endpoints, captured separately for span-precise renaming.
static LINK_PARTS: LazyLock<Regex> =
    LazyLock::new(|| compile(r"^(?<source>[^-+;]+?)\s*(?:->|\+>)\s*(?<target>[^;/]+?)\s*(?:;|//|$)"));
/// Any parenthesised decorator, for finding where the last one ends.
static ANY_DECORATOR: LazyLock<Regex> = LazyLock::new(|| compile(r"\(\s*[A-Za-z]+\s*\)"));

/// One named decorator in parentheses, for adding or removing exactly that one.
fn named_decorator(decorator: &str) -> Regex {
    compile(&format!(r"(?i)\(\s*{}\s*\)", regex::escape(decorator)))
}

fn splice(line: &str, start: usize, length: usize, replacement: &str) -> String {
    format!("{}{replacement}{}", &line[..start], &line[start + length..])
}

/// Moves one element by rewriting its coordinate pair in place; false when its line no longer looks like the
/// statement the model described.
pub fn set_position(document: &mut LineDocument, component: &WardleyComponent, position: WardleyCoordinate) -> bool {
    set_coordinate_pair(document, component.line, position)
}

/// Moves a pipeline child, which carries an evolution position only.
pub fn set_pipeline_child_maturity(document: &mut LineDocument, child: &WardleyPipelineChild, maturity: f64) -> bool {
    let Some(line) = document.line_at(child.line).map(str::to_owned) else {
        return false;
    };
    let Some(value) = SINGLE_VALUE.captures(&line).and_then(|c| c.name("value")) else {
        return false;
    };
    let updated = splice(&line, value.start(), value.len(), &format_number(maturity));
    replace_line(document, child.line, &updated)
}

/// Moves a legacy-form pipeline, whose two coordinates sit on the `pipeline` line itself.
pub fn set_pipeline_extent(document: &mut LineDocument, pipeline: &WardleyPipeline, extent: WardleyCoordinate) -> bool {
    pipeline.form == WardleyPipelineForm::Legacy && set_coordinate_pair(document, pipeline.line, extent)
}

/// Renames an element and every statement that names it (links, `evolve`, pipeline parents and children) in one
/// pass: how many lines were rewritten.
pub fn rename(document: &mut LineDocument, map: &WardleyMap, old_name: &str, new_name: &str) -> usize {
    if old_name == new_name {
        return 0;
    }
    let mut changed = 0;
    for component in map.components.iter().filter(|c| c.name == old_name) {
        changed += usize::from(rename_in(document, component.line, &DECLARATION_NAME, new_name));
    }
    for link in map
        .links
        .iter()
        .filter(|l| l.source == old_name || l.target == old_name)
    {
        changed += usize::from(rename_link(document, link, old_name, new_name));
    }
    for evolve in map.evolves.iter().filter(|e| e.name == old_name) {
        changed += usize::from(rename_in(document, evolve.line, &EVOLVE_NAME, new_name));
    }
    for pipeline in map.pipelines.iter().filter(|p| p.parent == old_name) {
        changed += usize::from(rename_in(document, pipeline.line, &PIPELINE_PARENT, new_name));
    }
    for child in map
        .pipelines
        .iter()
        .flat_map(|p| &p.children)
        .filter(|c| c.name == old_name)
    {
        changed += usize::from(rename_in(document, child.line, &DECLARATION_NAME, new_name));
    }
    changed
}

/// Appends a statement at the end of the document: the DSL is order-independent, and inserting into the middle of
/// someone's file rearranges a layout they chose.
pub fn append(document: &mut LineDocument, statement: &str) {
    let end = document.len();
    document
        .insert(end, &[statement.to_owned()])
        .expect("the end is inside the document");
}

/// Removes one statement by its 1-based line; a line the document does not have is left alone.
pub fn remove_line(document: &mut LineDocument, number: u32) {
    if number < 1 || number as usize > document.len() {
        return;
    }
    let index = number as usize - 1;
    document.remove(index, index).expect("the line is inside the document");
}

/// Replaces line `number` (1-based) with `text`, keeping that line's own ending; false outside the document.
pub fn replace_line(document: &mut LineDocument, number: u32, text: &str) -> bool {
    let Some(index) = (number as usize).checked_sub(1) else {
        return false;
    };
    document.replace(index, index, &[text.to_owned()]).is_ok()
}

/// Inserts `text` so that it becomes line `number` (1-based).
pub fn insert_line(document: &mut LineDocument, number: u32, text: &str) -> bool {
    let Some(index) = (number as usize).checked_sub(1) else {
        return false;
    };
    document.insert(index, &[text.to_owned()]).is_ok()
}

/// Adds or removes a parenthesised decorator on a component's line, leaving the rest of the line as it was.
pub fn set_decorator(
    document: &mut LineDocument,
    component: &WardleyComponent,
    decorator: &str,
    present: bool,
) -> bool {
    let Some(line) = document.line_at(component.line).map(str::to_owned) else {
        return false;
    };
    let existing = named_decorator(decorator).find(&line);
    if existing.is_some() == present {
        // Already as asked: success without a write keeps an idempotent command from making an empty undo entry.
        return true;
    }
    if present {
        let Some(coordinates) = PAIR.find(&line) else {
            return false;
        };
        // After the last decorator already there, or after the coordinates when there are none.
        let at = ANY_DECORATOR
            .find_iter(&line)
            .last()
            .map_or(coordinates.end(), |last| last.end());
        let updated = splice(&line, at, 0, &format!(" ({decorator})"));
        return replace_line(document, component.line, &updated);
    }
    let existing = existing.expect("present");
    // Take the leading space with it, so removing the last decorator does not leave one.
    let start = if existing.start() > 0 && line.as_bytes()[existing.start() - 1] == b' ' {
        existing.start() - 1
    } else {
        existing.start()
    };
    let updated = splice(&line, start, existing.end() - start, "");
    replace_line(document, component.line, &updated)
}

/// Adds or removes the bare `inertia` word on a component's line.
pub fn set_inertia(document: &mut LineDocument, component: &WardleyComponent, present: bool) -> bool {
    let Some(line) = document.line_at(component.line).map(str::to_owned) else {
        return false;
    };
    let existing = inertia_word(&line);
    if existing.is_some() == present {
        return true;
    }
    if present {
        let Some(coordinates) = PAIR.find(&line) else {
            return false;
        };
        let updated = splice(&line, coordinates.end(), 0, " inertia");
        return replace_line(document, component.line, &updated);
    }
    let (start, end) = existing.expect("present");
    let from = if start > 0 && line.as_bytes()[start - 1] == b' ' {
        start - 1
    } else {
        start
    };
    let updated = splice(&line, from, end - from, "");
    replace_line(document, component.line, &updated)
}

fn set_coordinate_pair(document: &mut LineDocument, number: u32, position: WardleyCoordinate) -> bool {
    let Some(line) = document.line_at(number).map(str::to_owned) else {
        return false;
    };
    let Some(captures) = PAIR.captures(&line) else {
        return false;
    };
    // The two numbers are replaced one by one, the second first, so the separator survives as the author wrote it.
    let (first, second) = (
        captures.name("first").expect("first"),
        captures.name("second").expect("second"),
    );
    let updated = splice(&line, second.start(), second.len(), &format_number(position.maturity));
    let updated = splice(
        &updated,
        first.start(),
        first.len(),
        &format_number(position.visibility),
    );
    replace_line(document, number, &updated)
}

fn rename_link(document: &mut LineDocument, link: &WardleyLink, old_name: &str, new_name: &str) -> bool {
    let Some(line) = document.line_at(link.line).map(str::to_owned) else {
        return false;
    };
    let Some(captures) = LINK_PARTS.captures(&line) else {
        return false;
    };
    // The target first, then the source, so replacing the source cannot shift the target's span.
    let mut updated = line.clone();
    if link.target == old_name {
        let target = captures.name("target").expect("target");
        updated = splice(&updated, target.start(), target.len(), new_name);
    }
    if link.source == old_name {
        let source = captures.name("source").expect("source");
        updated = splice(&updated, source.start(), source.len(), new_name);
    }
    if updated == line {
        return false;
    }
    replace_line(document, link.line, &updated)
}

fn rename_in(document: &mut LineDocument, number: u32, expression: &Regex, new_name: &str) -> bool {
    let Some(line) = document.line_at(number).map(str::to_owned) else {
        return false;
    };
    let Some(name) = expression.captures(&line).and_then(|c| c.name("name")) else {
        return false;
    };
    let updated = splice(&line, name.start(), name.len(), new_name);
    replace_line(document, number, &updated)
}
