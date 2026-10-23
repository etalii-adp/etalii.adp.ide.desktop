//! The checks of the script (ATD §12, §14.8): purity through the call graph, every path of a function with a result
//! type returns or fails, and the recursion bound.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value as Json;

use crate::cel;
use crate::description::Description;
use crate::symbols::Symbols;
use crate::{Finding, finding_at};

/// The statements that change a model, the transaction or what the user sees: the action set (ATD §12.3).
const ACTIONS: &[&str] = &[
    "set",
    "unset",
    "create",
    "connect",
    "delete",
    "move",
    "resize",
    "retype",
    "reparent",
    "reorder",
    "view",
    "open",
    "openForm",
    "editLabel",
    "select",
    "reveal",
    "highlight",
    "notify",
    "layout",
    "call",
    "plugin",
];

pub fn run(description: &Description, symbols: &Symbols, findings: &mut Vec<Finding>) {
    let impure = impure_functions(symbols);
    for member in description.members() {
        read_only_calls(member, &impure, findings);
    }
    for function in symbols.functions.values() {
        if function.get("returns").is_some() && !terminates(&function["body"]) {
            let name = function["name"]["text"].as_str().unwrap_or("");
            let returns = function["returns"]["name"]["text"].as_str().unwrap_or("");
            findings.push(finding_at(
                "ATD7003",
                &function["name"],
                format!("Not every path of `{name}` returns a `{returns}` or fails."),
            ));
        }
        if let Some(bound) = function["recursion"].as_u64()
            && bound > 64
        {
            let mut finding = finding_at(
                "ATD7004",
                &function["name"],
                format!("A function recurses at most 64 frames deep; `recursion {bound}` asks for more."),
            );
            if let Some(file) = description.file(&finding.file) {
                let span = adp_atd_syntax::Span::of(function).unwrap_or_default();
                if let Some(at) = file.text[span.start..span.end].find("recursion") {
                    let rest = &file.text[span.start + at + "recursion".len()..span.end];
                    let skip = rest.len() - rest.trim_start().len();
                    let start = span.start + at + "recursion".len() + skip;
                    let end = start + rest.trim_start().chars().take_while(char::is_ascii_digit).count();
                    let lines = adp_atd_syntax::LineIndex::new(&file.text);
                    (finding.line, finding.column) = lines.position(&file.text, start);
                    (finding.end_line, finding.end_column) = lines.position(&file.text, end);
                    finding.start = start;
                    finding.end = end;
                }
            }
            findings.push(finding);
        }
    }
}

/// The functions whose statements, or whose callees', change a model (ATD §12.7).
fn impure_functions(symbols: &Symbols) -> BTreeSet<String> {
    let mut acts: BTreeSet<String> = BTreeSet::new();
    let mut calls: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (name, function) in &symbols.functions {
        if has_action(&function["body"]) {
            acts.insert(name.clone());
        }
        let mut found = Vec::new();
        cel::global_calls(&function["body"], &mut found);
        calls.insert(
            name.clone(),
            found
                .iter()
                .filter_map(|c| c["call"].as_str())
                .map(str::to_owned)
                .collect(),
        );
    }
    let mut changed = true;
    while changed {
        changed = false;
        for (name, callees) in &calls {
            if !acts.contains(name) && callees.iter().any(|c| acts.contains(c)) {
                acts.insert(name.clone());
                changed = true;
            }
        }
    }
    acts
}

fn has_action(body: &Json) -> bool {
    body.as_array().into_iter().flatten().any(|statement| {
        let kind = statement["kind"].as_str().unwrap_or("");
        ACTIONS.contains(&kind) || ["then", "else", "body"].iter().any(|k| has_action(&statement[*k]))
    })
}

/// The calls of impure functions where an expression only reads: everywhere but the statements of operations, hooks,
/// fixes and functions.
fn read_only_calls(node: &Json, impure: &BTreeSet<String>, findings: &mut Vec<Finding>) {
    let kind = node.get("kind").and_then(Json::as_str).unwrap_or("");
    match node {
        Json::Object(map) => {
            if node.get("ast").is_some() && node.get("cel").is_some() {
                let mut calls = Vec::new();
                cel::global_calls(&node["ast"], &mut calls);
                for call in calls {
                    let name = call["call"].as_str().unwrap_or("");
                    if impure.contains(name) {
                        let what = "a position that only reads";
                        findings.push(finding_at(
                            "ATD7002",
                            call,
                            format!("`{name}` changes the model, so it cannot be called from {what}, such as a label, a constraint or a setting."),
                        ));
                    }
                }
                return;
            }
            for (key, value) in map {
                if key == "body" && matches!(kind, "operation" | "hook" | "fix" | "fn") {
                    continue;
                }
                if key != "span" {
                    read_only_calls(value, impure, findings);
                }
            }
        }
        Json::Array(items) => {
            for item in items {
                read_only_calls(item, impure, findings);
            }
        }
        _ => {}
    }
}

/// Whether every path of the statements returns or fails.
fn terminates(body: &Json) -> bool {
    body.as_array()
        .into_iter()
        .flatten()
        .any(|statement| match statement["kind"].as_str().unwrap_or("") {
            "return" => statement.get("value").is_some(),
            "fail" => true,
            "if" => statement.get("else").is_some() && terminates(&statement["then"]) && terminates(&statement["else"]),
            _ => false,
        })
}
