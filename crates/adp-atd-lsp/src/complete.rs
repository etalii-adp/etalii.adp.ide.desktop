//! Completion (FR-051, story 3 scenario 1): from the parser's state at the cursor, the keywords the grammar admits
//! there and the description's kind allows; where a name is expected, the names of the namespace the position names;
//! after `.` in CEL, the members of the receiver's type.

use std::collections::BTreeSet;

use adp_atd_check::{Checked, Symbols};
use serde_json::{Value as Json, json};

use crate::{Request, Server};

/// The keywords a kind's description does not admit at the top (ATD §14.4).
fn refused_by_kind(kind: &str, keyword: &str) -> bool {
    match kind {
        "designer" => matches!(
            keyword,
            "coordinates" | "notation" | "toolbox" | "layout" | "viewpoint" | "hook" | "relation" | "port"
        ),
        "editor" => matches!(
            keyword,
            "coordinates" | "notation" | "toolbox" | "layout" | "viewpoint" | "form" | "relation" | "port"
        ),
        _ => false,
    }
}

const BUILT_IN_FIELDS: &[&str] = &[
    "id",
    "type",
    "parent",
    "children",
    "descendants",
    "ancestors",
    "incoming",
    "outgoing",
    "view",
];

pub fn completion(server: &Server, request: &Request) -> Json {
    let expected = adp_atd_syntax::expected_at(&request.text, request.offset);
    let checked = server.described(&request.path);
    let keywords: Vec<&str> = expected
        .iter()
        .filter_map(|t| t.strip_prefix('"').and_then(|t| t.strip_suffix('"')))
        .filter(|t| t.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
        .collect();
    let name_position = expected.contains(&"NAME") && keywords.len() > 100;
    let mut items: Vec<Json> = Vec::new();
    if name_position || expected.contains(&"NAME") {
        let names = names_for(&checked, request);
        for (label, kind) in names {
            items.push(json!({"label": label, "kind": kind}));
        }
    }
    if !name_position {
        let kind = checked.description.kind().unwrap_or("diagram").to_owned();
        for keyword in keywords {
            if !refused_by_kind(&kind, keyword) {
                items.push(json!({"label": keyword, "kind": 14}));
            }
        }
    }
    let mut seen = BTreeSet::new();
    items.retain(|i| seen.insert(i["label"].as_str().unwrap_or("").to_owned()));
    json!({"isIncomplete": false, "items": items})
}

/// The significant words before the cursor, nearest first: what the position is.
fn words_before(text: &str, offset: usize) -> Vec<String> {
    let mut end = offset.min(text.len());
    while end > 0 && text[..end].ends_with(|c: char| c.is_alphanumeric() || c == '_') {
        end -= text[..end].chars().next_back().map(char::len_utf8).unwrap_or(1);
    }
    let before = &text[..end];
    let mut words = Vec::new();
    let mut current = String::new();
    for c in before.chars().rev() {
        if c.is_alphanumeric() || c == '_' {
            current.insert(0, c);
            continue;
        }
        if !current.is_empty() {
            words.push(std::mem::take(&mut current));
            if words.len() >= 6 {
                break;
            }
        }
        if !c.is_whitespace() {
            words.push(c.to_string());
            if words.len() >= 6 {
                break;
            }
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

/// The names a name position takes, each with its completion kind.
fn names_for(checked: &Checked, request: &Request) -> Vec<(String, u32)> {
    let Some(symbols) = &checked.symbols else {
        return Vec::new();
    };
    let words = words_before(&request.text, request.offset);
    let first = words.first().map(String::as_str).unwrap_or("");
    let types = || -> Vec<(String, u32)> { symbols.types.keys().map(|t| (t.clone(), 7)).collect() };
    let attributes_of = |owners: &[String]| -> Vec<(String, u32)> {
        let mut out: Vec<(String, u32)> = Vec::new();
        for owner in owners {
            for name in symbols.attributes_of(owner).into_keys() {
                out.push((name, 10));
            }
        }
        out.sort();
        out
    };
    match first {
        "." => {
            let owner = receiver_type(checked, symbols, request, &words);
            let mut out: Vec<(String, u32)> = BUILT_IN_FIELDS.iter().map(|f| ((*f).to_owned(), 5)).collect();
            if let Some(owner) = owner {
                out.extend(attributes_of(&[owner]));
            }
            out
        }
        "attribute" if enclosing(checked, request, "rule").is_some() => {
            let rule = enclosing(checked, request, "rule").unwrap_or_default();
            let type_name = rule["type"]["name"]["text"].as_str().unwrap_or("").to_owned();
            attributes_of(&[type_name])
        }
        "node" | "edge" | "port" | "for" | "extends" | "contains" | "create" | "connect" | "from" | "to" | "rows"
        | "columns" | "cells" | "views" | "levels" | "entries" | "fold" | "outline" | "," => types(),
        "by" => {
            let owners: Vec<String> = words
                .iter()
                .skip(1)
                .take_while(|w| w.as_str() != "outline")
                .filter(|w| symbols.is_type(w))
                .cloned()
                .collect();
            attributes_of(&owners)
        }
        "capability" => adp_atd_check::embedded::catalogue()
            .into_iter()
            .map(|(n, _)| (n, 9))
            .collect(),
        "parent" | "within" | "references" => symbols
            .bindings
            .values()
            .flat_map(|b| b.rules.keys().cloned())
            .map(|r| (r, 6))
            .collect(),
        _ => {
            let mut out = types();
            out.extend(symbols.functions.keys().map(|f| (f.clone(), 3)));
            out
        }
    }
}

/// The innermost construct of `kind` around the cursor.
fn enclosing(checked: &Checked, request: &Request, kind: &str) -> Option<Json> {
    let file = checked.description.files.iter().find(|f| f.path == request.path)?;
    let mut found: Option<Json> = None;
    adp_atd_syntax::ast::walk(file.tree.as_ref()?, &mut |node| {
        if node["kind"] == kind
            && let (Some(start), Some(end)) = (node["span"]["start"].as_u64(), node["span"]["end"].as_u64())
            && start as usize <= request.offset
            && request.offset <= end as usize
        {
            found = Some(node.clone());
        }
    });
    found
}

/// The element type of the expression before a `.`: `self` in a construct for one type, then each field.
fn receiver_type(checked: &Checked, symbols: &Symbols, request: &Request, words: &[String]) -> Option<String> {
    // words: ".", field, ".", field, …, root
    let mut chain: Vec<&str> = Vec::new();
    let mut i = 1;
    while i < words.len() {
        let word = words[i].as_str();
        if !word.chars().all(|c| c.is_alphanumeric() || c == '_') {
            break;
        }
        chain.push(word);
        if words.get(i + 1).map(String::as_str) == Some(".") {
            i += 2;
        } else {
            break;
        }
    }
    chain.reverse();
    let root = chain.first()?;
    let mut current = if *root == "self" {
        let construct = ["constraint", "operation", "hook", "node", "edge"]
            .iter()
            .find_map(|k| enclosing(checked, request, k))?;
        let names = construct.get("for").or_else(|| construct.get("names"))?;
        match names.as_array()?.as_slice() {
            [only] => only["text"].as_str()?.to_owned(),
            _ => return None,
        }
    } else {
        return None;
    };
    for field in chain.iter().skip(1) {
        current = if *field == "parent" {
            symbols.container_of(&current)?
        } else {
            let attribute = symbols.attributes_of(&current).get(*field)?.clone();
            let type_name = attribute["type"]["name"]["text"].as_str()?.to_owned();
            if !symbols.is_type(&type_name) {
                return None;
            }
            type_name
        };
    }
    Some(current)
}
