//! Splices the edits a behavior model supports into its Markdown: the lines a change touches, and none of the others
//! (etalii.adp spec 030, task T074).
//!
//! Ported from the standalone host's `AbmWriter.cs` at `8b6ccfc`. A node is one line, its notes the lines under it,
//! its subtree a contiguous range, which is what makes every edit a splice: a rename rewrites one line, notes replace
//! their own range, a removal cuts the subtree's range and a move cuts it and inserts it elsewhere, re-indented by the
//! difference between the two places. New lines follow the file, not a house style: a new child takes its first
//! sibling's indentation and list marker when it has siblings, and the parent's text column when it has none.

use crate::kinds::{self, Category};
use crate::lines::{LineDocument, LineRange};
use crate::model::{Model, Node};
use crate::parser;

/// What an edit came to: applied, or refused with the sentence the person reads.
pub type Outcome = Result<(), String>;

/// The text of a new behavior model for an agent called `name`, with `line_ending`: a Markdown file an agent can
/// follow from the first save. It explains itself to the agent in a section of its own before the tree, and starts the
/// tree with one Do in order under the Behavior heading.
pub fn empty_document(name: &str, line_ending: &str) -> String {
    let title = one_line(name);
    let mut lines = vec![
        format!("# {}", if title.is_empty() { "Agent" } else { &title }),
        String::new(),
        "Describe the agent here: who it works for, and what it is for.".to_owned(),
        String::new(),
    ];
    lines.extend(legend());
    lines.extend([
        String::new(),
        "## Behavior".to_owned(),
        String::new(),
        item_line("- ", &kinds::keyword_of(kinds::SEQUENCE, 0), "Handle the request"),
    ]);
    lines.join(line_ending) + line_ending
}

/// The section that tells the agent how to follow the tree.
pub fn legend() -> Vec<String> {
    let mut lines = vec![
        "## How to follow the behavior".to_owned(),
        String::new(),
        "The behavior below is a behavior tree, and it is your instructions. Start at its first node and work through it from the top. Each list item is a node; the items indented under a node are its children, in order. Every node ends in success or failure, and its parent decides what happens next:".to_owned(),
        String::new(),
    ];
    lines.extend(kinds::ALL.iter().map(|k| format!("- **{}** {}.", k.keyword, k.meaning)));
    lines.extend([
        String::new(),
        "Lines under a node that are not list items are notes: follow them while you carry out that node.".to_owned(),
    ]);
    lines
}

/// Sets a node's label, keeping its keyword as the author wrote it.
pub fn set_label(document: &mut LineDocument, node: &Node, label: &str) -> Outcome {
    let text = one_line(label);
    let line = if node.has_keyword {
        item_line(&prefix_of(document, node), &node.keyword_text, &text)
    } else {
        prefix_of(document, node) + &text
    };
    document.replace(LineRange::new(node.line, node.line), &[line.trim_end().to_owned()]);
    Ok(())
}

/// Changes a node's kind, refusing a kind that cannot hold the children it already has.
pub fn set_kind(document: &mut LineDocument, node: &Node, kind: &str, retry_count: i32) -> Outcome {
    if !kinds::is_known(kind) {
        return Err(format!("A behavior model has no `{kind}` node."));
    }
    if kind == kinds::RETRY && retry_count < 1 {
        return Err("A Retry allows at least one attempt.".to_owned());
    }
    let category = kinds::of(kind).category;
    let children = node.child_ids.len();
    if category == Category::Leaf && children > 0 {
        return Err(format!(
            "\"{}\" holds no children, and this node has {}.",
            kinds::of(kind).keyword,
            if children == 1 {
                "1 child".to_owned()
            } else {
                format!("{children} children")
            }
        ));
    }
    if category == Category::Decorator && children > 1 {
        return Err(format!(
            "\"{}\" holds exactly one child, and this node has {children}.",
            kinds::keyword_of(kind, retry_count)
        ));
    }
    let keyword = kinds::keyword_of(kind, if kind == kinds::RETRY { retry_count } else { 0 });
    let line = item_line(&prefix_of(document, node), &keyword, &node.label);
    document.replace(LineRange::new(node.line, node.line), &[line.trim_end().to_owned()]);
    Ok(())
}

/// Replaces a node's notes; empty notes remove them.
pub fn set_notes(document: &mut LineDocument, node: &Node, notes: &str) -> Outcome {
    let indent = " ".repeat(node.content_indent);
    let lines: Vec<String> = notes
        .replace("\r\n", "\n")
        .trim_matches('\n')
        .split('\n')
        .map(|line| {
            if line.trim().is_empty() {
                String::new()
            } else {
                format!("{indent}{}", line.trim_end())
            }
        })
        .collect();
    let empty = lines.iter().all(String::is_empty);
    match node.notes_range {
        Some(range) if empty => document.remove(range),
        Some(range) => document.replace(range, &lines),
        None if !empty => document.insert(node.line + 1, &lines),
        None => {}
    }
    Ok(())
}

/// Adds a node under `parent` (a root when none) before the child now at `index`, or after the last when the index
/// is negative or past them: the edit, and the new node's id when it was applied.
pub fn add(
    document: &mut LineDocument,
    model: &Model,
    parent: Option<&Node>,
    index: i64,
    kind: &str,
    label: &str,
    retry_count: i32,
) -> Result<String, String> {
    if !kinds::is_known(kind) {
        return Err(format!("A behavior model has no `{kind}` node."));
    }
    let retry_count = if kind == kinds::RETRY && retry_count < 1 {
        kinds::DEFAULT_RETRY_COUNT
    } else {
        retry_count
    };
    let keyword = kinds::keyword_of(kind, retry_count);
    let text = one_line(label);

    let Some(parent) = parent else {
        let roots = model.roots();
        if roots.is_empty() {
            add_first_root(document, model, item_line("- ", &keyword, &text).trim_end());
            return Ok("1".to_owned());
        }
        let at = clamp(index, roots.len());
        let line = if at < roots.len() {
            roots[at].line
        } else {
            roots[roots.len() - 1].subtree_end + 1
        };
        document.insert(
            line,
            &[item_line(&prefix_for(document, roots[0]), &keyword, &text)
                .trim_end()
                .to_owned()],
        );
        return Ok((at + 1).to_string());
    };

    if !parent.takes_another_child() {
        return Err(if parent.category() == Category::Leaf {
            format!(
                "\"{}\" holds no children. Add the node under a Do in order, Try in order or Do together.",
                parent.keyword()
            )
        } else {
            format!("\"{}\" holds exactly one child, and already has it.", parent.keyword())
        });
    }
    let children = model.children_of(parent);
    let position = clamp(index, children.len());
    let insert_at = if position < children.len() {
        children[position].line
    } else {
        parent.subtree_end + 1
    };
    let prefix = match children.first() {
        Some(first) => prefix_for(document, first),
        None => format!("{}{} ", " ".repeat(parent.content_indent), parent.marker),
    };
    document.insert(insert_at, &[item_line(&prefix, &keyword, &text).trim_end().to_owned()]);
    Ok(format!("{}.{}", parent.id, position + 1))
}

/// Removes a node, its notes and everything beneath it.
pub fn remove(document: &mut LineDocument, node: &Node) -> Outcome {
    document.remove(node.subtree());
    Ok(())
}

/// Moves a node and its subtree under `new_parent` (to the roots when none) before the child now at `index` there,
/// or after the last. The index counts the target's children as they are before the move, the node itself included
/// when it is one of them: "one earlier" is the node's index minus one, and "one later" its index plus two.
pub fn move_node(
    document: &mut LineDocument,
    model: &Model,
    node: &Node,
    new_parent: Option<&Node>,
    index: i64,
) -> Outcome {
    if let Some(parent) = new_parent {
        if Model::is_within(&parent.id, &node.id) {
            return Err("A node cannot move beneath itself.".to_owned());
        }
        let accepts = match parent.category() {
            Category::Composite => true,
            Category::Decorator => parent.child_ids.is_empty() || node.parent_id.as_deref() == Some(&parent.id),
            Category::Leaf => false,
        };
        if !accepts {
            return Err(if parent.category() == Category::Leaf {
                format!("\"{}\" holds no children.", parent.keyword())
            } else {
                format!("\"{}\" holds exactly one child, and already has it.", parent.keyword())
            });
        }
    }

    let siblings = match new_parent {
        Some(parent) => model.children_of(parent),
        None => model.roots(),
    };
    let at = clamp(index, siblings.len());
    let current = if node.parent_id.as_deref() == new_parent.map(|p| p.id.as_str()) {
        siblings.iter().position(|s| s.id == node.id)
    } else {
        None
    };
    if let Some(current) = current
        && (at == current || at == current + 1)
    {
        return Err("That node is already there.".to_owned());
    }

    let target = if at < siblings.len() {
        siblings[at].line
    } else {
        let end = match new_parent {
            Some(parent) => parent.subtree_end,
            None => model.roots().last().map(|r| r.subtree_end).unwrap_or_default(),
        };
        end + 1
    };

    // The indentation it lands at: a sibling's other than itself, or the parent's text column.
    let prefix = match siblings.iter().find(|s| s.id != node.id) {
        Some(neighbour) => prefix_for(document, neighbour),
        None => match new_parent {
            Some(parent) => format!("{}{} ", " ".repeat(parent.content_indent), parent.marker),
            None => format!("{}{} ", " ".repeat(model.list_indent), node.marker),
        },
    };
    let new_indent = parser::column_of(&prefix) as i64;
    let delta = new_indent - node.indent as i64;

    let own_prefix = prefix_of(document, node);
    let mut moved = Vec::new();
    for line in node.line..=node.subtree_end {
        let text = &document.lines()[line].text;
        if line == node.line {
            moved.push(format!("{prefix}{}", &text[own_prefix.len()..]));
        } else if text.trim().is_empty() {
            moved.push(String::new());
        } else if delta >= 0 {
            moved.push(format!("{}{text}", " ".repeat(delta as usize)));
        } else {
            moved.push(outdent(text, (-delta) as usize));
        }
    }

    if target > node.subtree_end {
        document.insert(target, &moved);
        document.remove(node.subtree());
    } else {
        document.remove(node.subtree());
        document.insert(target, &moved);
    }
    Ok(())
}

/// One item line: the prefix, then the keyword in bold with its colon, then the label.
fn item_line(prefix: &str, keyword: &str, label: &str) -> String {
    if label.is_empty() {
        format!("{prefix}**{keyword}:**")
    } else {
        format!("{prefix}**{keyword}:** {label}")
    }
}

/// A label as one line: newlines become spaces, and the ends are trimmed.
pub fn one_line(label: &str) -> String {
    label
        .replace('\r', "")
        .split('\n')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// The first root of a file that has none: under its Behavior heading, or in a new Behavior section at the end.
fn add_first_root(document: &mut LineDocument, model: &Model, item: &str) {
    if let Some(heading) = model.section_line {
        document.insert(heading + 1, &[String::new(), item.to_owned()]);
        return;
    }
    let count = document.lines().len();
    let mut lines = Vec::new();
    if document.lines().last().is_some_and(|l| !l.text.trim().is_empty()) {
        lines.push(String::new());
    }
    lines.extend(["## Behavior".to_owned(), String::new(), item.to_owned()]);
    document.insert(count, &lines);
}

/// The item line's indentation and marker, as written: everything before its text.
fn prefix_of(document: &LineDocument, node: &Node) -> String {
    let text = &document.lines()[node.line].text;
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() && (bytes[index] == b' ' || bytes[index] == b'\t') {
        index += 1;
    }
    index += 1; // the marker
    while index < bytes.len() && (bytes[index] == b' ' || bytes[index] == b'\t') {
        index += 1;
    }
    text[..index.min(text.len())].to_owned()
}

/// A sibling's prefix, normalised to one space after the marker.
fn prefix_for(document: &LineDocument, sibling: &Node) -> String {
    format!("{} ", prefix_of(document, sibling).trim_end())
}

fn outdent(text: &str, columns: usize) -> String {
    let bytes = text.as_bytes();
    let mut column = 0;
    let mut index = 0;
    while index < bytes.len() && column < columns && (bytes[index] == b' ' || bytes[index] == b'\t') {
        column += if bytes[index] == b'\t' { 4 - (column % 4) } else { 1 };
        index += 1;
    }
    text[index..].to_owned()
}

/// `Math.Clamp(index < 0 ? count : index, 0, count)`.
fn clamp(index: i64, count: usize) -> usize {
    if index < 0 { count } else { (index as usize).min(count) }
}
