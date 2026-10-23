//! Traces (etalii.adp spec 035, T048, ATD §2.5, data-model.md §10): `atd check --trace` reads a `<name>.trace.json`,
//! checks the description it names, and reports at the trace's own entries every `statedBy` and `divergesFrom` that
//! names no construct of the description (`ATD2011`), every host concern that is not one of ATD §17's, and every item
//! of the companion's named sections that is not traced exactly once, as `validate-examples.py` does.

use std::path::{Path, PathBuf};

use regex::Regex;
use serde_json::Value as Json;

use crate::{Files, Finding, Severity, check};

/// The host concerns of ATD §17, as a trace names them.
pub const HOST_CONCERNS: &[&str] = &[
    "rendering",
    "gestures",
    "text-measurement",
    "file-dialogs",
    "opening-links",
    "undo-stack",
];

/// The items of a companion's named sections: each `###` heading and each top-level list item, with its id
/// `<section-slug>/<n>` and its text (ATD §2.5).
pub fn companion_items(text: &str, sections: &[String]) -> Vec<(String, String)> {
    let heading = Regex::new(r"^(#{1,6})\s+(.*?)\s*$").expect("a heading pattern");
    let bullet = Regex::new(r"^(?:[-*+]|[0-9]+[.)])\s+(.*)$").expect("a list item pattern");
    let mut items = Vec::new();
    let mut current: Option<String> = None;
    let (mut level, mut count) = (0, 0);
    let mut fenced = false;
    for line in text.lines() {
        if line.starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        if let Some(found) = heading.captures(line) {
            let depth = found[1].len();
            let title = found[2].to_owned();
            if current.is_some() && depth <= level {
                current = None;
            }
            if sections.contains(&title) {
                current = Some(slug(&title));
                (level, count) = (depth, 0);
                continue;
            }
            if let Some(section) = &current
                && depth == 3
            {
                count += 1;
                items.push((format!("{section}/{count}"), title));
            }
            continue;
        }
        if let (Some(section), Some(found)) = (&current, bullet.captures(line)) {
            count += 1;
            items.push((format!("{section}/{count}"), found[1].to_owned()));
        }
    }
    items
}

/// A heading's slug: lowercase, every run of other characters than letters and digits one `-`.
fn slug(title: &str) -> String {
    let mut out = String::new();
    for c in title.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_owned()
}

/// The names a construct is known by: its name, its names, what a binding rule reads it `as`, and its type.
fn names_of(node: &Json) -> Vec<String> {
    let text = |v: &Json| -> Option<String> {
        v["text"]
            .as_str()
            .or_else(|| v["name"]["text"].as_str())
            .or_else(|| v.as_str())
            .map(str::to_owned)
    };
    let mut out = Vec::new();
    for key in ["name", "as", "type"] {
        if let Some(value) = node.get(key).and_then(text) {
            out.push(value);
        }
    }
    for value in node["names"].as_array().into_iter().flatten() {
        if let Some(value) = text(value) {
            out.push(value);
        }
    }
    if let Some(capability) = node.get("capability").and_then(text) {
        out.push(capability);
    }
    out
}

/// Whether a construct is spelled by a keyword: its kind, or a binding rule's kind; `capability` names a
/// `requires capability`, and any construct that names a capability.
fn spelled(node: &Json, keyword: &str) -> bool {
    node["kind"] == keyword
        || node["ruleKind"] == keyword
        || keyword == "capability" && node.get("capability").is_some()
}

/// The constructs directly under a value: the nodes with a kind among its fields, through arrays and objects
/// without a kind.
fn children(node: &Json) -> Vec<&Json> {
    fn gather<'a>(value: &'a Json, out: &mut Vec<&'a Json>) {
        match value {
            Json::Object(map) if map.contains_key("kind") => out.push(value),
            Json::Object(map) => map.values().for_each(|v| gather(v, out)),
            Json::Array(items) => items.iter().for_each(|v| gather(v, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    if let Json::Object(map) = node {
        for (key, value) in map {
            if key != "span" {
                gather(value, &mut out);
            }
        }
    }
    out
}

/// Every construct of a tree, depth first.
fn descendants<'a>(node: &'a Json, out: &mut Vec<&'a Json>) {
    for child in children(node) {
        out.push(child);
        descendants(child, out);
    }
}

/// The words of the description's text in a construct's span.
fn words_in<'a>(node: &Json, texts: &'a [(String, String)]) -> Vec<&'a str> {
    let file = node["span"]["file"].as_str().unwrap_or("");
    let (start, end) = (
        node["span"]["start"].as_u64().unwrap_or(0) as usize,
        node["span"]["end"].as_u64().unwrap_or(0) as usize,
    );
    texts
        .iter()
        .find(|(path, _)| path == file)
        .and_then(|(_, text)| text.get(start..end))
        .map(|t| {
            t.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.'))
                .filter(|w| !w.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// Whether `words` name a construct among `scope` and what it holds: a keyword and, when the construct has a name, its
/// name, then the rest within it. Words left once no construct inside matches name the construct's own members and
/// values, and must be among its words (`notation node Node label text`).
fn resolves(scope: &[&Json], words: &[String], texts: &[(String, String)]) -> bool {
    let Some(keyword) = words.first() else {
        return true;
    };
    let mut candidates = Vec::new();
    for node in scope {
        candidates.push(*node);
        descendants(node, &mut candidates);
    }
    for node in candidates {
        if !spelled(node, keyword) {
            continue;
        }
        let names = names_of(node);
        let mut rest: Vec<String> = words[1..].to_vec();
        if !names.is_empty() {
            let Some(first) = rest.first().cloned() else {
                continue;
            };
            if let Some(name) = names.iter().find(|n| **n == first) {
                let _ = name;
                rest.remove(0);
            } else if let Some(name) = names.iter().find(|n| first.starts_with(&format!("{n}."))) {
                // A dotted path below a named construct: `binding freeplane.registration.layout`.
                let below: Vec<String> = first[name.len() + 1..].split('.').map(str::to_owned).collect();
                rest.remove(0);
                rest.splice(0..0, below);
            } else {
                continue;
            }
        }
        if rest.is_empty() || resolves(&children(node), &rest, texts) {
            return true;
        }
        let words = words_in(node, texts);
        if rest.iter().all(|w| words.contains(&w.as_str())) {
            return true;
        }
    }
    false
}

/// Whether a construct path resolves in a description's trees.
pub fn resolve(trees: &[&Json], texts: &[(String, String)], stated_by: &str) -> bool {
    let words: Vec<String> = stated_by.split_whitespace().map(str::to_owned).collect();
    !words.is_empty() && resolves(trees, &words, texts)
}

/// A finding at a byte range of the trace.
fn at(file: &str, text: &str, start: usize, end: usize, message: String) -> Finding {
    let position = |offset: usize| {
        let before = &text[..offset.min(text.len())];
        let line = before.matches('\n').count() + 1;
        let column = before.rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1;
        (line, column)
    };
    let ((line, column), (end_line, end_column)) = (position(start), position(end));
    Finding {
        code: "ATD2011",
        severity: Severity::Error,
        file: file.to_owned(),
        line,
        column,
        end_line,
        end_column,
        start,
        end,
        message,
        suggestions: Vec::new(),
        fixes: Vec::new(),
    }
}

/// Checks a trace against its description and companion; an error when the trace cannot be read.
pub fn check_trace(files: &dyn Files, path: &Path) -> Result<Vec<Finding>, String> {
    let text = files
        .read(path)
        .ok_or_else(|| format!("{} cannot be read", path.display()))?;
    let trace: Json = serde_json::from_str(&text).map_err(|e| format!("{} is not JSON: {e}", path.display()))?;
    let folder = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let file = path.display().to_string();
    let mut findings = Vec::new();

    let description: PathBuf = folder.join(trace["description"].as_str().unwrap_or(""));
    let checked = check(files, &description);
    let trees: Vec<&Json> = checked
        .description
        .files
        .iter()
        .filter_map(|f| f.tree.as_ref())
        .collect();
    let texts: Vec<(String, String)> = checked
        .description
        .files
        .iter()
        .map(|f| (f.path.display().to_string(), f.text.clone()))
        .collect();
    if trees.is_empty() {
        findings.push(at(
            &file,
            &text,
            0,
            0,
            format!("The description `{}` cannot be read.", description.display()),
        ));
    }

    // Every construct path and host concern, at the place the trace states it.
    let entry = Regex::new(r#""(statedBy|divergesFrom|hostConcern)"\s*:\s*"((?:[^"\\]|\\.)*)""#).expect("a pattern");
    for found in entry.captures_iter(&text) {
        let value = found.get(2).expect("a value");
        let (key, written) = (&found[1], value.as_str());
        let problem = if key == "hostConcern" {
            (!HOST_CONCERNS.contains(&written)).then(|| {
                format!(
                    "`{written}` is not a host concern; ATD §17 names {}.",
                    HOST_CONCERNS.join(", ")
                )
            })
        } else {
            (!trees.is_empty() && !resolve(&trees, &texts, written))
                .then(|| format!("`{written}` names no construct of the description."))
        };
        if let Some(message) = problem {
            findings.push(at(&file, &text, value.start(), value.end(), message));
        }
    }

    // Every item of the companion's named sections, traced exactly once.
    let sections: Vec<String> = trace["companion"]["sections"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|s| s.as_str().map(str::to_owned))
        .collect();
    let companion = folder.join(trace["companion"]["file"].as_str().unwrap_or(""));
    match files.read(&companion) {
        None => findings.push(at(
            &file,
            &text,
            0,
            0,
            format!("The companion `{}` cannot be read.", companion.display()),
        )),
        Some(companion_text) => {
            let expected = companion_items(&companion_text, &sections);
            let traced: Vec<&str> = trace["items"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|i| i["id"].as_str())
                .collect();
            for (id, item) in &expected {
                let times = traced.iter().filter(|t| **t == id).count();
                if times != 1 {
                    findings.push(at(
                        &file,
                        &text,
                        0,
                        0,
                        format!("The companion's item {id} ({item:?}) is traced {times} times, not once."),
                    ));
                }
            }
            for id in traced {
                if !expected.iter().any(|(e, _)| e == id) {
                    let start = text.find(&format!("\"{id}\"")).map_or(0, |i| i + 1);
                    findings.push(at(
                        &file,
                        &text,
                        start,
                        start + id.len(),
                        format!("The trace names the item {id}, which the companion's sections do not have."),
                    ));
                }
            }
        }
    }
    crate::sort(&mut findings);
    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slug_is_lowercase_with_one_dash_for_each_run_of_other_characters() {
        assert_eq!(
            slug("1. What DISL still lacks for this type"),
            "1-what-disl-still-lacks-for-this-type"
        );
    }

    #[test]
    fn items_are_third_level_headings_and_top_level_list_items_of_the_named_sections() {
        let text = "# T\n## 1. Gaps\n### Naming\n- one\n  - nested\n1. two\n```\n- fenced\n```\n## Other\n- not\n";
        let items = companion_items(text, &["1. Gaps".to_owned()]);
        assert_eq!(
            items,
            vec![
                ("1-gaps/1".to_owned(), "Naming".to_owned()),
                ("1-gaps/2".to_owned(), "one".to_owned()),
                ("1-gaps/3".to_owned(), "two".to_owned()),
            ]
        );
    }
}
