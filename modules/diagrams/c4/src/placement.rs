//! Where a new element or view goes in a Structurizr DSL document, and how it gets there (etalii.adp spec 030, task
//! T061).
//!
//! Ported from the standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4/Commands/C4Placement.cs` and the
//! `C4ViewBlock` of `Commands/AddC4ViewCommand.cs` at `8b6ccfc`. C4 says what may contain what, and this is the one
//! place that turns those rules into line numbers: an element may have to go inside another, and the element it goes
//! inside may not have a block yet, so putting a component in `web = container "Web" "x" "y"` gives it braces it
//! never had, and undoing that add takes them away again.

use crate::lines::{C4Line, LineDocument};
use crate::model::{C4Element, C4ElementKind, C4View, C4ViewKind, C4Workspace};
use crate::rules::spell;

/// Why `kind` cannot go inside `parent_id` (empty for the top of the model), or none when it can.
pub fn refuse(workspace: &C4Workspace, kind: C4ElementKind, parent_id: &str) -> Option<String> {
    let parent = if parent_id.is_empty() {
        None
    } else {
        workspace.find(parent_id)
    };
    if !parent_id.is_empty() && parent.is_none() {
        return Some("The element it would go inside is no longer in this model.".to_owned());
    }
    match (kind, parent) {
        // Dropping a person or a software system onto something else is taken as "at the top".
        (C4ElementKind::Person | C4ElementKind::SoftwareSystem, _) => None,
        (C4ElementKind::Container, None) => {
            Some("A container lives inside a software system. Drop it on one.".to_owned())
        }
        (C4ElementKind::Container, Some(parent)) if parent.kind != C4ElementKind::SoftwareSystem => Some(format!(
            "A container lives inside a software system, not inside a {}.",
            spell(parent.kind)
        )),
        (C4ElementKind::Component, None) => Some("A component lives inside a container. Drop it on one.".to_owned()),
        (C4ElementKind::Component, Some(parent)) if parent.kind != C4ElementKind::Container => Some(format!(
            "A component lives inside a container, not inside a {}.",
            spell(parent.kind)
        )),
        (C4ElementKind::Container | C4ElementKind::Component, _) => None,
        // Deployment nodes, infrastructure nodes and instances live in a deployment environment.
        _ => Some(format!(
            "Adding a {} from the toolbox is not supported yet.",
            spell(kind)
        )),
    }
}

/// Writes the declaration of a new element, opening a block on its parent when it has none: whether it opened one,
/// or why it could not be written.
pub fn try_insert(
    document: &mut LineDocument,
    workspace: &C4Workspace,
    kind: C4ElementKind,
    name: &str,
    identifier: &str,
    parent_id: &str,
) -> Result<bool, String> {
    let parent = if parent_id.is_empty() {
        None
    } else {
        workspace.find(parent_id)
    };
    let Some(parent) = parent else {
        let end_of_model = end_of_block(document, |line| {
            let code = line.code().to_lowercase();
            code == "model {" || code.starts_with("model {")
        })
        .ok_or("This document has no model block to add an element to.")?;
        document.insert_line(end_of_model, &declare(kind, name, identifier, 8))?;
        return Ok(false);
    };

    let Some(declaration) = document
        .code_lines()
        .into_iter()
        .find(|line| line.number == parent.line)
    else {
        return Err("The element it would go inside is no longer in this model.".to_owned());
    };
    let indent = indent_of(&declaration.text) + 4;
    if declaration.code().ends_with('{') {
        // Already has a block: the new line goes just above its closing brace.
        let close = end_of_block(document, |line| line.number == parent.line)
            .ok_or("That element's block is not closed, so nothing can be added to it.")?;
        document.insert_line(close, &declare(kind, name, identifier, indent))?;
        return Ok(false);
    }

    // No block yet: the declaration gains a brace, the child goes under it, and a closing brace follows.
    document.replace_line(declaration.number, &format!("{} {{", declaration.text.trim_end()))?;
    document.insert_line(declaration.number + 1, &declare(kind, name, identifier, indent))?;
    document.insert_line(
        declaration.number + 2,
        &format!("{}}}", " ".repeat(indent_of(&declaration.text))),
    )?;
    Ok(true)
}

/// Closes a block this module opened, putting the parent back on one line; a block with any text left inside it,
/// a comment included, is left as it is.
pub fn collapse_empty_block(document: &mut LineDocument, parent: &C4Element) -> Result<(), String> {
    let Some(declaration) = document
        .code_lines()
        .into_iter()
        .find(|line| line.number == parent.line && line.code().ends_with('{'))
    else {
        return Ok(());
    };
    let Some(close) = end_of_block(document, |line| line.number == parent.line) else {
        return Ok(());
    };
    let has_content = document
        .code_lines()
        .iter()
        .any(|line| line.number > declaration.number && line.number < close && !line.text.trim().is_empty());
    if has_content {
        return Ok(());
    }
    document.remove_lines(declaration.number + 1, close)?;
    document.replace_line(
        declaration.number,
        declaration.text.trim_end().trim_end_matches('{').trim_end(),
    )
}

/// Removes an element declared on one line; one with a block of its own is refused rather than taking its children
/// with it.
pub fn try_remove(document: &mut LineDocument, element: &C4Element) -> Result<(), String> {
    let Some(declaration) = document
        .code_lines()
        .into_iter()
        .find(|line| line.number == element.line)
    else {
        return Err("That element is no longer in this model.".to_owned());
    };
    if declaration.code().ends_with('{') {
        return Err("That element has things inside it, so removing it here would take them too.".to_owned());
    }
    document.remove_lines(declaration.number, declaration.number)
}

/// An identifier nothing in the model uses yet, made from the name's letters and digits, the first lowered: `web`,
/// then `web2`, `web3`; `_` before a leading digit, and `element` when nothing is left.
pub fn identifier_for(workspace: &C4Workspace, name: &str) -> String {
    let mut candidate = String::new();
    for character in name.chars().filter(|c| c.is_alphanumeric()) {
        if candidate.is_empty() {
            candidate.extend(character.to_lowercase());
        } else {
            candidate.push(character);
        }
    }
    if candidate.is_empty() {
        candidate = "element".to_owned();
    }
    if candidate.chars().next().is_some_and(char::is_numeric) {
        candidate = format!("_{candidate}");
    }
    if workspace.find(&candidate).is_none() {
        return candidate;
    }
    (2..)
        .map(|suffix| format!("{candidate}{suffix}"))
        .find(|numbered| workspace.find(numbered).is_none())
        .expect("some suffix is free")
}

/// One declaration line: `identifier = keyword "Name" "Describe Name."`, with a placeholder technology for what C4
/// gives one to.
fn declare(kind: C4ElementKind, name: &str, identifier: &str, indent: usize) -> String {
    let keyword = match kind {
        C4ElementKind::Person => "person",
        C4ElementKind::SoftwareSystem => "softwareSystem",
        C4ElementKind::Container => "container",
        _ => "component",
    };
    let declaration = format!(
        "{}{identifier} = {keyword} \"{}\" \"Describe {}.\"",
        " ".repeat(indent),
        escape(name),
        escape(name)
    );
    if matches!(kind, C4ElementKind::Container | C4ElementKind::Component) {
        format!("{declaration} \"Technology\"")
    } else {
        declaration
    }
}

/// The line the closing brace of a block sits on, given how to find the line that opens it.
fn end_of_block(document: &LineDocument, opens: impl Fn(&C4Line) -> bool) -> Option<u32> {
    let mut depth: i64 = 0;
    let mut inside = false;
    for line in document.code_lines() {
        let code = line.code();
        if !inside {
            if !opens(&line) || !code.ends_with('{') {
                continue;
            }
            inside = true;
            depth = 1;
            continue;
        }
        depth += braces(code);
        if depth <= 0 {
            return Some(line.number);
        }
    }
    None
}

/// The opening braces of `code` less its closing ones.
fn braces(code: &str) -> i64 {
    code.chars().filter(|c| *c == '{').count() as i64 - code.chars().filter(|c| *c == '}').count() as i64
}

fn indent_of(text: &str) -> usize {
    text.len() - text.trim_start().len()
}

fn escape(value: &str) -> String {
    value.replace('"', "\\\"")
}

/// The line the closing brace of the `views` block sits on: where a new view goes above.
pub fn end_of_views_block(document: &LineDocument) -> Option<u32> {
    let mut depth: i64 = 0;
    let mut in_views = false;
    for line in document.code_lines() {
        let code = line.code();
        if !in_views && code.to_lowercase().starts_with("views") && code.ends_with('{') {
            in_views = true;
            depth = 1;
            continue;
        }
        if !in_views {
            continue;
        }
        depth += braces(code);
        if depth <= 0 {
            return Some(line.number);
        }
    }
    None
}

/// The lines one view declaration occupies, indented as the document's views are.
pub fn declare_view(kind: C4ViewKind, key: &str, scope: Option<&str>, workspace: &C4Workspace) -> Vec<String> {
    let scope = scope.unwrap_or_default();
    let header = match kind {
        C4ViewKind::SystemLandscape => format!("        systemLandscape \"{key}\" {{"),
        C4ViewKind::SystemContext => format!("        systemContext {scope} \"{key}\" {{"),
        C4ViewKind::Container => format!("        container {scope} \"{key}\" {{"),
        C4ViewKind::Component => format!("        component {} \"{key}\" {{", component_scope(workspace, scope)),
        C4ViewKind::Dynamic => format!("        dynamic {scope} \"{key}\" {{"),
        C4ViewKind::Deployment => format!("        deployment {scope} \"Production\" \"{key}\" {{"),
    };
    // A dynamic view's body is its ordered interactions, and a new one has none yet.
    if kind == C4ViewKind::Dynamic {
        return vec![header, "            autoLayout lr".to_owned(), "        }".to_owned()];
    }
    vec![
        header,
        "            include *".to_owned(),
        "            autoLayout lr".to_owned(),
        "        }".to_owned(),
    ]
}

/// A component view is scoped to a container: the first the model declares, else the fallback.
fn component_scope(workspace: &C4Workspace, fallback: &str) -> String {
    workspace
        .elements
        .iter()
        .find(|e| e.kind == C4ElementKind::Container)
        .map_or_else(|| fallback.to_owned(), |e| e.id.clone())
}

/// Removes a view's declaration, from its header line to its closing brace.
pub fn remove_view(document: &mut LineDocument, view: &C4View) -> Result<(), String> {
    let first = view.line;
    let mut last = first;
    let mut depth: i64 = 0;
    for line in document.code_lines().into_iter().filter(|line| line.number >= first) {
        depth += braces(line.code());
        last = line.number;
        if depth <= 0 {
            break;
        }
    }
    document.remove_lines(first, last)
}
