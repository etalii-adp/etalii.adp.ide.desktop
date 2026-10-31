//! The grammars of `grammar` bindings (FBL 0.9 section 4.9, ATD §11.9): each is built with the desktop host's table
//! builder, on `etalii-lark`; one that is not LALR(1), or in which two terminals a state accepts match the same text,
//! is `ATD8001` at the head of the definition of the rule in conflict, one the notation does not read is `ATD1001` where
//! it stops, and a file `grammar from` names that is not there is `ATD2009`.

use std::collections::BTreeMap;
use std::path::Path;

use adp_atd_grammar::BuildError;
use serde_json::Value as Json;

use crate::description::{Description, FileUnit};
use crate::{Files, Finding, Severity, finding_at};

/// Checks the grammar of every `grammar` binding of the description.
pub(crate) fn run(files: &dyn Files, description: &Description, findings: &mut Vec<Finding>) {
    for unit in &description.files {
        for member in unit.members().iter().filter(|m| m["kind"] == "binding") {
            if member["family"].as_str() != Some("grammar") {
                continue;
            }
            check_binding(files, unit, member, findings);
        }
    }
}

/// The modules of each grammar binding of `unit`, by the binding's name: an inline grammar under `grammar`, a file and
/// what it imports under their names. A file that is not there is left out, for [`run`] to report.
pub(crate) fn read(files: &dyn Files, unit: &FileUnit) -> BTreeMap<String, BTreeMap<String, String>> {
    let mut grammars = BTreeMap::new();
    for binding in unit.members().iter().filter(|m| m["kind"] == "binding") {
        if binding["family"].as_str() != Some("grammar") {
            continue;
        }
        let mut modules = BTreeMap::new();
        for member in binding["members"].as_array().into_iter().flatten() {
            match member["kind"].as_str() {
                Some("grammar_inline") => {
                    if let Some((text, _, _)) = inline_text(unit, member) {
                        modules.insert("grammar".to_owned(), text);
                    }
                }
                Some("grammar_file") => {
                    let name = member["title"].as_str().unwrap_or_default();
                    let folder = unit.path.parent().unwrap_or(Path::new("."));
                    let mut wanted = vec![name.trim_end_matches(".lark").to_owned()];
                    while let Some(module) = wanted.pop() {
                        if modules.contains_key(&module) {
                            continue;
                        }
                        let Some(text) = files.read(&folder.join(format!("{module}.lark"))) else {
                            continue;
                        };
                        wanted.extend(text.lines().filter_map(imported));
                        modules.insert(module, text);
                    }
                }
                _ => {}
            }
        }
        let name = binding["name"]["text"].as_str().unwrap_or_default().to_owned();
        grammars.insert(name, modules);
    }
    grammars
}

/// The module a line `%import .<module>…` names.
fn imported(line: &str) -> Option<String> {
    let rest = line
        .trim_start()
        .strip_prefix("%import")?
        .trim_start()
        .strip_prefix('.')?;
    let module: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect();
    Some(module)
}

/// An inline grammar's text, dedented, with where the text starts in the file and what is written from there.
fn inline_text<'a>(unit: &'a FileUnit, member: &Json) -> Option<(String, usize, &'a str)> {
    let text_node = member["members"]
        .as_array()?
        .iter()
        .find(|n| n["kind"] == "grammar_text")?;
    let span = adp_atd_syntax::Span::of(text_node).unwrap_or_default();
    // From the start of the grammar's first line, so that every line has the indentation they share.
    let from = unit
        .text
        .get(..span.start)
        .and_then(|t| t.rfind('\n'))
        .map_or(0, |n| n + 1);
    let written = unit.text.get(from..span.end)?;
    Some((adp_atd_grammar::dedent(written), from, written))
}

fn check_binding(files: &dyn Files, unit: &FileUnit, binding: &Json, findings: &mut Vec<Finding>) {
    let members: &[Json] = binding["members"].as_array().map(Vec::as_slice).unwrap_or(&[]);
    let start = members
        .iter()
        .filter(|m| m["kind"] == "body")
        .flat_map(|m| m["settings"].as_array().into_iter().flatten())
        .find(|s| s["key"] == "start")
        .and_then(|s| s["value"]["literal"].as_str())
        .unwrap_or("start")
        .to_owned();
    for member in members {
        match member["kind"].as_str() {
            Some("grammar_inline") => {
                let Some((text, from, written)) = inline_text(unit, member) else {
                    continue;
                };
                if let Err(error) = adp_atd_grammar::build_single(&text, &start) {
                    findings.push(refusal(unit, &error, from, written, member));
                }
            }
            Some("grammar_file") => {
                let name = member["title"].as_str().unwrap_or_default();
                let folder = unit.path.parent().unwrap_or(Path::new("."));
                let root = name.trim_end_matches(".lark");
                let load = |module: &str| files.read(&folder.join(format!("{module}.lark")));
                if load(root).is_none() || name.contains('/') || name.contains('\\') {
                    findings.push(finding_at(
                        "ATD2009",
                        member,
                        format!("The grammar file `{name}` is not there, or not in the description's folder."),
                    ));
                    continue;
                }
                let options = adp_atd_grammar::Options::default();
                if let Err(error) = adp_atd_grammar::build_with_options(root, &start, &load, options) {
                    findings.push(match &error {
                        BuildError::Compose(_) => finding_at(
                            "ATD1001",
                            member,
                            format!(
                                "The grammar file `{name}` is not ATD grammar notation: {}",
                                error.to_string().trim()
                            ),
                        ),
                        _ => finding_at("ATD8001", member, conflict_message(&error)),
                    });
                }
            }
            _ => {}
        }
    }
}

fn conflict_message(error: &BuildError) -> String {
    match error {
        BuildError::Conflicts(conflicts) => {
            let c = &conflicts[0];
            format!(
                "The grammar is not LALR(1): a {} conflict on {} between {}.",
                c.kind,
                c.terminal,
                {
                    let mut rules: Vec<String> = Vec::new();
                    for rule in &c.rules {
                        if !rules.contains(rule) {
                            rules.push(rule.clone());
                        }
                    }
                    rules.iter().map(|r| format!("`{r}`")).collect::<Vec<_>>().join(" and ")
                }
            )
        }
        BuildError::Collisions(collisions) => {
            let c = &collisions[0];
            format!(
                "The grammar is not LALR(1): the terminals {} and {} both match `{}`.",
                c.first, c.second, c.example
            )
        }
        other => format!("The grammar cannot be built: {}", other.to_string().trim()),
    }
}

/// The finding of an inline grammar that does not build, placed in the grammar's text, which starts at `offset` in
/// the file.
fn refusal(unit: &FileUnit, error: &BuildError, offset: usize, written: &str, member: &Json) -> Finding {
    let (code, message, at) = match error {
        BuildError::Compose(compose) => {
            let at = line_start(written, compose.line).unwrap_or(0);
            (
                "ATD1001",
                format!("The grammar is not ATD grammar notation: {}", compose.message),
                at,
            )
        }
        BuildError::Conflicts(conflicts) => {
            // The head of the definition of the rule in conflict that is defined first.
            let at = conflicts[0]
                .rules
                .iter()
                .filter_map(|r| definition(written, r.split(" : ").next().unwrap_or(r).trim()))
                .min()
                .unwrap_or(0);
            ("ATD8001", conflict_message(error), at)
        }
        BuildError::Collisions(collisions) => {
            let at = definition(written, &collisions[0].first).unwrap_or(0);
            ("ATD8001", conflict_message(error), at)
        }
        BuildError::Lexer(_) => ("ATD8001", conflict_message(error), 0),
    };
    let mut finding = finding_at(code, member, message);
    let start = offset + at;
    let end = offset + written[at..].find('\n').map(|n| at + n).unwrap_or(written.len());
    let (line, column) = position(&unit.text, start);
    let (end_line, end_column) = position(&unit.text, end);
    finding.start = start;
    finding.end = end;
    finding.line = line;
    finding.column = column;
    finding.end_line = end_line;
    finding.end_column = end_column;
    finding.severity = Severity::Error;
    finding
}

/// The offset of the head of the definition of `name` in `text`: the line that defines it, `?` or `!` before it.
fn definition(text: &str, name: &str) -> Option<usize> {
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let head = line.trim_start();
        let head = head.trim_start_matches(['?', '!']);
        if let Some(rest) = head.strip_prefix(name)
            && rest.trim_start().starts_with(':')
        {
            return Some(offset + (line.len() - line.trim_start().len()));
        }
        offset += line.len();
    }
    None
}

/// The offset where 1-based line `line` of `text` starts.
fn line_start(text: &str, line: usize) -> Option<usize> {
    if line <= 1 {
        return Some(0);
    }
    text.match_indices('\n').nth(line - 2).map(|(i, _)| i + 1)
}

/// The 1-based line and column, in characters, of byte `offset` of `text`.
fn position(text: &str, offset: usize) -> (usize, usize) {
    let before = &text[..offset.min(text.len())];
    let line = before.matches('\n').count() + 1;
    let column = before.rsplit('\n').next().map(|l| l.chars().count()).unwrap_or(0) + 1;
    (line, column)
}
