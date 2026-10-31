//! Loads an FBL document as FBL section 14.1 says (the standalone host's `Documents/FblDocumentLoader`), apart from
//! validating against the JSON Schema, which is left to etalii.adp's own CI: parse rejecting duplicate keys, check
//! the version, map every construct, resolve names, compile every regular expression and CEL expression, and check
//! the rules. Every problem is collected; a document with an error yields no document.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use serde_json::Value as Json;

use crate::documents::types::*;
use crate::expressions::{self, CelContext};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProblemSeverity {
    Warning,
    Error,
}

/// A problem in an FBL document, at the JSON Pointer of its location (FBL section 14.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadProblem {
    pub pointer: String,
    pub severity: ProblemSeverity,
    pub message: String,
}

impl fmt::Display for LoadProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pointer = if self.pointer.is_empty() { "/" } else { &self.pointer };
        write!(f, "{:?} at {pointer}: {}", self.severity, self.message)
    }
}

fn problem(pointer: impl Into<String>, severity: ProblemSeverity, message: impl Into<String>) -> LoadProblem {
    LoadProblem {
        pointer: pointer.into(),
        severity,
        message: message.into(),
    }
}

fn error(pointer: impl Into<String>, message: impl Into<String>) -> LoadProblem {
    problem(pointer, ProblemSeverity::Error, message)
}

const SUPPORTED_MAJOR: i64 = 0;

/// The newest minor version this library reads in full; a document of a newer one loads with a warning.
const KNOWN_MINOR: i64 = 14;

/// Loads the document at `path`.
pub fn load_file(path: &Path) -> (Vec<LoadProblem>, Option<FblDocument>) {
    match std::fs::read(path) {
        Ok(bytes) => {
            let (mut problems, document) = load(&bytes);
            let Some(mut document) = document else {
                return (problems, None);
            };
            let folder = path.parent().unwrap_or(Path::new("."));
            for (name, binding) in &mut document.bindings {
                if let Some(grammar) = &mut binding.body.grammar
                    && grammar.file.is_some()
                {
                    let pointer = format!("/bindings/{}/body/grammar", escape(name));
                    read_grammar_files(grammar, folder, &pointer, &mut problems);
                    check_grammar(grammar, &pointer, &mut problems);
                }
            }
            if problems.iter().any(|p| p.severity == ProblemSeverity::Error) {
                return (problems, None);
            }
            (problems, Some(document))
        }
        Err(e) => (vec![error("", format!("The document cannot be read: {e}"))], None),
    }
}

/// Reads a grammar's file beside the document and the modules it imports from beside it (FBL 0.9 section 4.9).
fn read_grammar_files(grammar: &mut GrammarSettings, folder: &Path, pointer: &str, problems: &mut Vec<LoadProblem>) {
    let mut wanted = vec![grammar.root.clone()];
    while let Some(module) = wanted.pop() {
        if grammar.modules.contains_key(&module) {
            continue;
        }
        let file = folder.join(format!("{module}.lark"));
        match std::fs::read_to_string(&file) {
            Ok(text) => {
                for line in text.lines() {
                    if let Some(rest) = line.trim_start().strip_prefix("%import")
                        && let Some(name) = rest.trim_start().strip_prefix('.')
                    {
                        let name: String = name
                            .chars()
                            .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
                            .collect();
                        wanted.push(name);
                    }
                }
                grammar.modules.insert(module, text);
            }
            Err(e) => {
                problems.push(error(
                    pointer,
                    format!("The grammar file '{}' cannot be read: {e}", file.display()),
                ));
                return;
            }
        }
    }
}

/// Refuses a grammar that does not build as LALR(1) without a conflict (FBL 0.9 section 14.1).
fn check_grammar(grammar: &GrammarSettings, pointer: &str, problems: &mut Vec<LoadProblem>) {
    if let Err(reason) = crate::grammar::build(grammar) {
        problems.push(error(pointer, reason));
    }
}

/// Loads a document from its bytes: its problems, and the document when none of them is an error.
pub fn load(json: &[u8]) -> (Vec<LoadProblem>, Option<FblDocument>) {
    let mut problems = Vec::new();
    find_duplicate_keys(json, &mut problems);
    if !problems.is_empty() {
        return (problems, None);
    }
    let root: Json = match serde_json::from_slice(json) {
        Ok(root) => root,
        Err(e) => {
            problems.push(error("", format!("The document is not JSON: {e}")));
            return (problems, None);
        }
    };
    let Some(object) = root.as_object() else {
        problems.push(error("", "An FBL document is a JSON object."));
        return (problems, None);
    };
    let Some(version) = object.get("fbl").and_then(Json::as_str) else {
        problems.push(error(
            "/fbl",
            "The document does not say which FBL version it is written in.",
        ));
        return (problems, None);
    };
    let parts: Vec<&str> = version.split('.').collect();
    let (major, minor) = match (
        parts.first().and_then(|p| p.parse::<i64>().ok()),
        parts.get(1).and_then(|p| p.parse::<i64>().ok()),
    ) {
        (Some(major), Some(minor)) if parts.len() >= 2 => (major, minor),
        _ => {
            problems.push(error("/fbl", format!("'{version}' is not an FBL version.")));
            return (problems, None);
        }
    };
    if major != SUPPORTED_MAJOR {
        problems.push(error(
            "/fbl",
            format!("FBL {version} is not supported; this library reads FBL {SUPPORTED_MAJOR}.x."),
        ));
        return (problems, None);
    }
    if minor > KNOWN_MINOR {
        problems.push(problem(
            "/fbl",
            ProblemSeverity::Warning,
            format!(
                "FBL {version} is newer than {SUPPORTED_MAJOR}.{KNOWN_MINOR}; what this library does not know is ignored."
            ),
        ));
    }
    let bindings = match object.get("bindings").and_then(Json::as_object) {
        Some(b) if !b.is_empty() => b,
        _ => {
            problems.push(error("/bindings", "A document has at least one binding."));
            return (problems, None);
        }
    };
    let mut result = Vec::new();
    for (name, value) in bindings {
        let pointer = format!("/bindings/{}", escape(name));
        if !is_name(name) {
            problems.push(error(pointer.clone(), format!("'{name}' is not a valid binding name.")));
        }
        if let Some(binding) = read_binding(name, value, &pointer, &mut problems) {
            check(&binding, &pointer, &mut problems);
            if let Some(grammar) = binding.body.grammar.as_ref().filter(|g| g.file.is_none()) {
                check_grammar(grammar, &format!("{pointer}/body/grammar"), &mut problems);
            }
            result.push((name.clone(), binding));
        }
    }
    if problems.iter().any(|p| p.severity == ProblemSeverity::Error) {
        return (problems, None);
    }
    (
        problems,
        Some(FblDocument {
            version: version.to_owned(),
            bindings: result,
        }),
    )
}

/// Resolves a binding reference, `doc.fbl#name` or `#name`, against `referrer` (FBL section 2.3).
pub fn resolve_reference(reference: &str, referrer: &Path) -> Result<(FblBinding, Vec<LoadProblem>), String> {
    let hash = reference
        .rfind('#')
        .ok_or_else(|| format!("'{reference}' is not a binding reference."))?;
    let (file, name) = (&reference[..hash], &reference[hash + 1..]);
    let path: PathBuf = if file.is_empty() {
        referrer.to_path_buf()
    } else {
        referrer.parent().unwrap_or(Path::new(".")).join(file)
    };
    let (problems, document) = load_file(&path);
    let Some(document) = document else {
        let all: Vec<String> = problems.iter().map(ToString::to_string).collect();
        return Err(format!(
            "The FBL document '{}' does not load: {}",
            path.display(),
            all.join("; ")
        ));
    };
    match document.binding(name) {
        Some(binding) => Ok((binding.clone(), problems)),
        None => Err(format!(
            "The FBL document '{}' has no binding '{name}'.",
            path.display()
        )),
    }
}

pub(crate) fn escape(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

fn is_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

// ---- duplicate keys ----

/// Reports every key that appears twice in one object, at its JSON Pointer; a document that is not JSON is one
/// problem.
fn find_duplicate_keys(json: &[u8], problems: &mut Vec<LoadProblem>) {
    let mut scanner = Scanner { bytes: json, at: 0 };
    scanner.space();
    if let Err(message) = scanner.value("", problems) {
        problems.push(error("", format!("The document is not JSON: {message}")));
    }
}

struct Scanner<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Scanner<'_> {
    fn space(&mut self) {
        while self.at < self.bytes.len() && matches!(self.bytes[self.at], b' ' | b'\t' | b'\r' | b'\n') {
            self.at += 1;
        }
    }

    fn value(&mut self, pointer: &str, problems: &mut Vec<LoadProblem>) -> Result<(), String> {
        match self.bytes.get(self.at) {
            Some(b'{') => {
                self.at += 1;
                let mut keys: Vec<String> = Vec::new();
                self.space();
                if self.bytes.get(self.at) == Some(&b'}') {
                    self.at += 1;
                    return Ok(());
                }
                loop {
                    self.space();
                    let key = self.string()?;
                    let here = format!("{pointer}/{}", escape(&key));
                    if keys.contains(&key) {
                        problems.push(error(here.clone(), format!("The key '{key}' appears twice.")));
                    } else {
                        keys.push(key);
                    }
                    self.space();
                    if self.bytes.get(self.at) != Some(&b':') {
                        return Err(format!("':' was expected at byte {}.", self.at));
                    }
                    self.at += 1;
                    self.space();
                    self.value(&here, problems)?;
                    self.space();
                    match self.bytes.get(self.at) {
                        Some(b',') => self.at += 1,
                        Some(b'}') => {
                            self.at += 1;
                            return Ok(());
                        }
                        _ => return Err(format!("',' or '}}' was expected at byte {}.", self.at)),
                    }
                }
            }
            Some(b'[') => {
                self.at += 1;
                self.space();
                if self.bytes.get(self.at) == Some(&b']') {
                    self.at += 1;
                    return Ok(());
                }
                let mut index = 0;
                loop {
                    self.space();
                    self.value(&format!("{pointer}/{index}"), problems)?;
                    index += 1;
                    self.space();
                    match self.bytes.get(self.at) {
                        Some(b',') => self.at += 1,
                        Some(b']') => {
                            self.at += 1;
                            return Ok(());
                        }
                        _ => return Err(format!("',' or ']' was expected at byte {}.", self.at)),
                    }
                }
            }
            Some(b'"') => self.string().map(|_| ()),
            Some(_) => {
                let start = self.at;
                while self.at < self.bytes.len()
                    && !matches!(self.bytes[self.at], b',' | b'}' | b']' | b' ' | b'\t' | b'\r' | b'\n')
                {
                    self.at += 1;
                }
                if start == self.at {
                    return Err(format!("A value was expected at byte {start}."));
                }
                Ok(())
            }
            None => Err("The document ends where a value was expected.".into()),
        }
    }

    fn string(&mut self) -> Result<String, String> {
        if self.bytes.get(self.at) != Some(&b'"') {
            return Err(format!("A string was expected at byte {}.", self.at));
        }
        let start = self.at;
        self.at += 1;
        while self.at < self.bytes.len() {
            match self.bytes[self.at] {
                b'\\' => self.at += 2,
                b'"' => {
                    self.at += 1;
                    return serde_json::from_slice::<String>(&self.bytes[start..self.at]).map_err(|e| e.to_string());
                }
                _ => self.at += 1,
            }
        }
        Err("A string is never closed.".into())
    }
}

// ---- mapping the binding ----

fn s(json: &Json, name: &str) -> Option<String> {
    json.get(name).and_then(Json::as_str).map(str::to_owned)
}

fn flag(json: &Json, name: &str) -> bool {
    json.get(name).and_then(Json::as_bool) == Some(true)
}

fn strings(json: &Json, name: &str) -> Vec<String> {
    json.get(name)
        .and_then(Json::as_array)
        .map(|a| a.iter().filter_map(Json::as_str).map(str::to_owned).collect())
        .unwrap_or_default()
}

fn optional_strings(json: &Json, name: &str) -> Option<Vec<String>> {
    json.get(name).map(|_| strings(json, name))
}

fn array<'a>(json: &'a Json, name: &str) -> &'a [Json] {
    json.get(name)
        .and_then(Json::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn localized_value(value: &Json) -> Option<String> {
    if let Some(en) = value.get("en") {
        return en.as_str().map(str::to_owned);
    }
    value
        .as_object()?
        .values()
        .next()
        .and_then(Json::as_str)
        .map(str::to_owned)
}

fn localized(json: &Json, name: &str) -> Option<String> {
    let value = json.get(name)?;
    match value.as_str() {
        Some(text) => Some(text.to_owned()),
        None => localized_value(value),
    }
}

fn read_only(json: &Json) -> Option<String> {
    match json.get("readOnly")? {
        Json::Bool(true) => Some(String::new()),
        Json::String(text) => Some(text.clone()),
        object @ Json::Object(_) => localized_value(object),
        _ => None,
    }
}

/// A JSON scalar as the text FBL compares it by: a string as itself, a bool in lower case, null as nothing.
pub(crate) fn scalar_text(value: &Json) -> String {
    match value {
        Json::String(text) => text.clone(),
        Json::Bool(b) => if *b { "true" } else { "false" }.into(),
        Json::Null => String::new(),
        other => other.to_string(),
    }
}

fn parse_family(name: Option<&str>) -> Option<Family> {
    match name? {
        "yaml" => Some(Family::Yaml),
        "json" => Some(Family::Json),
        "xml" => Some(Family::Xml),
        "lines" => Some(Family::Lines),
        "blocks" => Some(Family::Blocks),
        "bullets" => Some(Family::Bullets),
        "grammar" => Some(Family::Grammar),
        _ => None,
    }
}

fn read_binding(name: &str, json: &Json, pointer: &str, problems: &mut Vec<LoadProblem>) -> Option<FblBinding> {
    if !json.is_object() {
        problems.push(error(pointer, "A binding is an object."));
        return None;
    }
    let before = problems.len();
    if json.get("claims").is_none() {
        problems.push(error(format!("{pointer}/claims"), "A binding has claims."));
    }
    if json.get("body").is_none() {
        problems.push(error(format!("{pointer}/body"), "A binding has a body."));
    }
    if json.get("reader").is_none() {
        problems.push(error(format!("{pointer}/reader"), "A binding has a reader."));
    }
    if problems.len() > before {
        return None;
    }
    let reader = &json["reader"];
    let mut plugin = None;
    if reader.is_object() {
        plugin = Some(PluginReader {
            plugin: s(reader, "plugin").unwrap_or_default(),
            version: s(reader, "version"),
        });
    } else if reader.as_str() != Some("declared") {
        problems.push(error(
            format!("{pointer}/reader"),
            "The reader is \"declared\" or a plugin.",
        ));
    }
    let mut rules: Vec<Rule> = array(json, "elements").iter().map(|e| read_rule(e, false)).collect();
    rules.extend(array(json, "relations").iter().map(|e| read_rule(e, true)));
    Some(FblBinding {
        name: name.to_owned(),
        title: localized(json, "title"),
        claims: read_claims(&json["claims"]),
        body: read_body(&json["body"], &format!("{pointer}/body"), problems),
        plugin,
        read_only: read_only(json),
        text: read_text(json),
        header: json.get("header").map(|h| HeaderSettings {
            key: s(h, "key"),
            value: h.get("value").cloned(),
            line: s(h, "line"),
            required: flag(h, "required"),
        }),
        comment: s(json, "comment"),
        report_unmatched: s(json, "unmatched").as_deref() == Some("report"),
        blocks: array(json, "blocks")
            .iter()
            .map(|b| BlockRule {
                name: s(b, "name").unwrap_or_default(),
                line: s(b, "line").unwrap_or_default(),
                within: optional_strings(b, "within"),
                case_insensitive: flag(b, "caseInsensitive"),
                view: s(b, "view"),
                at: s(b, "at"),
                view_slot: b.get("view").filter(|v| v.is_object()).map(read_slot),
                origins: strings(b, "origins"),
                insert: b.get("insert").map(read_insert),
            })
            .collect(),
        sections: array(json, "sections")
            .iter()
            .map(|section| SectionRule {
                name: s(section, "name").unwrap_or_default(),
                heading: s(section, "heading").unwrap_or_default(),
                case_insensitive: flag(section, "caseInsensitive"),
            })
            .collect(),
        rules,
        registration: read_registration(json, pointer, problems),
        template: json.get("template").map(|t| TemplateSettings {
            text: s(t, "text").unwrap_or_default(),
            by_origin: t
                .get("byOrigin")
                .and_then(Json::as_object)
                .map(|o| {
                    o.iter()
                        .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_owned()))
                        .collect()
                })
                .unwrap_or_default(),
        }),
    })
}

fn read_claims(json: &Json) -> Claims {
    let marker = json.get("marker").map(|m| Marker {
        root_key: s(m, "rootKey"),
        root_value: m.get("value").cloned(),
        first_line: s(m, "firstLine"),
        pattern: s(m, "pattern"),
        lines: m.get("lines").and_then(Json::as_i64).unwrap_or(20),
    });
    let readings = json
        .get("readings")
        .and_then(Json::as_object)
        .map(|r| {
            r.iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        ReadingClaim {
                            bare: flag(v, "bare"),
                            suggest: v.get("suggest").map(|s| strings(s, "contains")).unwrap_or_default(),
                        },
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    Claims {
        extensions: strings(json, "extensions"),
        names: strings(json, "names"),
        shared: flag(json, "shared"),
        registration_only: flag(json, "registrationOnly"),
        marker,
        suggest: json.get("suggest").map(|s| strings(s, "contains")).unwrap_or_default(),
        origins: strings(json, "origins"),
        readings,
    }
}

fn read_body(json: &Json, pointer: &str, problems: &mut Vec<LoadProblem>) -> BodySettings {
    let kind = s(json, "kind");
    if !matches!(kind.as_deref(), Some("file" | "folder")) {
        problems.push(error(
            format!("{pointer}/kind"),
            "A body's kind is \"file\" or \"folder\".",
        ));
    }
    let recognise = json.get("recognise").filter(|r| r.is_object());
    let mut settle = DEFAULT_SETTLE;
    if let Some(t) = json.get("settle") {
        match t.as_i64() {
            Some(n) if n >= 0 && n <= i64::from(i32::MAX) => settle = n,
            _ => problems.push(error(
                format!("{pointer}/settle"),
                "A body's settle is a whole number of milliseconds, 0 or more.",
            )),
        }
    }
    BodySettings {
        is_folder: kind.as_deref() == Some("folder"),
        family: parse_family(json.get("family").and_then(Json::as_str)),
        also_read: strings(json, "alsoRead")
            .iter()
            .filter_map(|f| parse_family(Some(f)))
            .collect(),
        recognise_all: recognise.map(|r| strings(r, "all")).unwrap_or_default(),
        recognise_any: recognise.map(|r| strings(r, "any")).unwrap_or_default(),
        recognise_none: recognise.map(|r| strings(r, "none")).unwrap_or_default(),
        files: array(json, "files")
            .iter()
            .map(|f| FileRule {
                name: s(f, "name"),
                glob: s(f, "glob").unwrap_or_default(),
                family: parse_family(f.get("family").and_then(Json::as_str)),
            })
            .collect(),
        ignore: strings(json, "ignore"),
        settle,
        grammar: read_grammar(json, pointer, problems),
    }
}

/// A `grammar` body's grammar (FBL 0.9 section 4.9): its text, or the file beside the document it is in, whose
/// modules [`load_file`] reads.
fn read_grammar(json: &Json, pointer: &str, problems: &mut Vec<LoadProblem>) -> Option<GrammarSettings> {
    let is_grammar = s(json, "family").as_deref() == Some("grammar");
    let start = s(json, "start").unwrap_or_else(|| "start".to_owned());
    let recover = json.get("recover").map(|r| Recover {
        unit: s(r, "unit").unwrap_or_default(),
        sync: strings(r, "sync"),
        nested: r.get("nested").map(|n| NestedRecover {
            open: s(n, "open").unwrap_or_default(),
            close: s(n, "close").unwrap_or_default(),
        }),
        unclosed: r
            .get("unclosed")
            .and_then(Json::as_object)
            .map(|u| {
                u.iter()
                    .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_owned()))
                    .collect()
            })
            .unwrap_or_default(),
    });
    let settings = match json.get("grammar") {
        Some(Json::String(text)) => GrammarSettings {
            root: "grammar".to_owned(),
            file: None,
            modules: BTreeMap::from([("grammar".to_owned(), text.clone())]),
            start,
            recover,
        },
        Some(g) if s(g, "file").is_some() => {
            let file = s(g, "file").unwrap_or_default();
            GrammarSettings {
                root: file.trim_end_matches(".lark").to_owned(),
                file: Some(file),
                modules: BTreeMap::new(),
                start,
                recover,
            }
        }
        Some(_) => {
            problems.push(error(
                format!("{pointer}/grammar"),
                "A grammar is its text or {\"file\": \"<name>.lark\"}.",
            ));
            return None;
        }
        None => {
            if is_grammar {
                problems.push(error(format!("{pointer}/grammar"), "A grammar body has a grammar."));
            }
            return None;
        }
    };
    if !is_grammar {
        problems.push(error(
            format!("{pointer}/grammar"),
            "Only a body of the grammar family has a grammar.",
        ));
    }
    Some(settings)
}

fn read_text(json: &Json) -> TextDefaults {
    let Some(t) = json.get("text") else {
        return TextDefaults::default();
    };
    let indent = match t.get("indent") {
        None => 2,
        Some(i) => i.as_u64().map(|n| n as usize).unwrap_or(0),
    };
    TextDefaults {
        newline: match s(t, "newline").as_deref() {
            Some("crlf") => "\r\n",
            Some("cr") => "\r",
            _ => "\n",
        }
        .into(),
        indent,
        sequence_flush: s(t, "sequenceIndent").as_deref() == Some("flush"),
        quote: s(t, "quote").unwrap_or_else(|| "double".into()),
    }
}

fn read_insert(ins: &Json) -> InsertSettings {
    // {before: key} is recognised so every family can refuse it; only a new member is placed by it (FBL 0.14).
    let before = ins.get("place").and_then(|p| s(p, "before"));
    let place = match ins.get("place") {
        Some(Json::Object(_)) => "before".to_owned(),
        Some(Json::String(p)) => p.clone(),
        _ => "end".to_owned(),
    };
    let create = ins.get("create").map(|c| match c.get("at") {
        Some(Json::String(at)) => CreateContainer {
            at: at.clone(),
            argument: None,
            text: s(c, "text"),
        },
        Some(Json::Object(at)) => {
            let (name, value) = at
                .iter()
                .next()
                .map(|(k, v)| (k.clone(), v.as_str().map(str::to_owned)))
                .unwrap_or_default();
            CreateContainer {
                at: name,
                argument: value,
                text: s(c, "text"),
            }
        }
        _ => CreateContainer {
            at: String::new(),
            argument: None,
            text: s(c, "text"),
        },
    });
    InsertSettings {
        place,
        before,
        container: s(ins, "container"),
        create,
        keys: strings(ins, "keys"),
        emit: s(ins, "emit"),
        skeleton: s(ins, "skeleton"),
        when: s(ins, "when"),
        refusal_when: ins.get("refusals").and_then(|r| localized(r, "when")),
        refusal_container: ins.get("refusals").and_then(|r| localized(r, "container")),
        refusal_key: ins.get("refusals").and_then(|r| localized(r, "key")),
        refusal_duplicate: ins.get("refusals").and_then(|r| localized(r, "duplicate")),
    }
}

fn read_rule(json: &Json, relation: bool) -> Rule {
    let id = json.get("id").map(|id| IdBinding {
        from: id.get("from").map(read_slot),
        sidecar_key: id.get("sidecar").and_then(|sc| s(sc, "key")),
        sidecar_kind: id.get("sidecar").and_then(|sc| s(sc, "kind")),
        sidecar_ordinal: id.get("sidecar").and_then(|sc| s(sc, "ordinal")),
    });
    let parent = json.get("parent").map(|p| ParentBinding {
        rules: strings(p, "rules"),
        slot: s(p, "slot"),
    });
    let attributes = json
        .get("attributes")
        .and_then(Json::as_object)
        .map(|a| a.iter().map(|(k, v)| (k.clone(), read_attribute(v))).collect())
        .unwrap_or_default();
    let insert = json.get("insert").map(read_insert);
    let remove = json.get("remove").map(|rem| {
        let container = s(rem, "container");
        RemoveSettings {
            cascade: strings(rem, "cascade"),
            remove_container_when_empty: matches!(
                container.as_deref(),
                Some("remove-when-empty" | "remove-empty-levels")
            ),
            remove_empty_levels: container.as_deref() == Some("remove-empty-levels"),
        }
    });
    Rule {
        name: s(json, "name").unwrap_or_default(),
        type_name: s(json, "type").unwrap_or_default(),
        is_relation: relation,
        origins: strings(json, "origins"),
        at: s(json, "at"),
        line: s(json, "line"),
        within: optional_strings(json, "within"),
        opens: flag(json, "opens"),
        opens_text: json.get("opens").filter(|o| o.is_object()).map(|o| OpensText {
            open: s(o, "open").unwrap_or_default(),
            close: s(o, "close").unwrap_or_default(),
        }),
        selected_view: flag(json, "selectedView"),
        case_insensitive: flag(json, "caseInsensitive"),
        files: strings(json, "files"),
        when: s(json, "when"),
        id,
        parent,
        attributes,
        source: json.get("source").map(read_attribute),
        target: json.get("target").map(read_attribute),
        insert,
        remove,
        snapshot_undo: s(json, "undo").as_deref() == Some("snapshot"),
        read_only: read_only(json),
    }
}

fn read_slot(json: &Json) -> Slot {
    Slot {
        key: s(json, "key"),
        xml_attribute: s(json, "attribute"),
        text: flag(json, "text"),
        child: s(json, "child"),
        group: s(json, "group"),
        parent: s(json, "parent"),
        capture: s(json, "capture"),
        value: s(json, "value"),
        continuation: flag(json, "continuation"),
        word: s(json, "word"),
        flag: flag(json, "flag"),
        node: s(json, "node"),
        node_match: s(json, "match"),
        escape: json.get("escape").and_then(read_escape),
        split: s(json, "split"),
        items: json
            .get("items")
            .and_then(Json::as_object)
            .map(|items| items.iter().map(|(k, v)| (k.clone(), read_slot(v))).collect())
            .unwrap_or_default(),
        separator: s(json, "separator"),
    }
}

fn read_escape(json: &Json) -> Option<SlotEscape> {
    let one = |name: &str| s(json, name).and_then(|text| text.chars().next());
    Some(SlotEscape {
        with: one("with")?,
        quote: one("quote")?,
    })
}

fn read_attribute(json: &Json) -> AttributeBinding {
    let decimals = json
        .get("number")
        .filter(|n| n.is_object())
        .and_then(|n| n.get("decimals"))
        .and_then(Json::as_i64)
        .map(|d| d as i32);
    let map = json
        .get("map")
        .and_then(Json::as_object)
        .map(|m| m.iter().map(|(k, v)| (k.clone(), scalar_text(v))).collect());
    let create = json.get("create").map(|c| {
        let place = c.get("place");
        if s(c, "at").as_deref() == Some("end") {
            CreateChild {
                emit: String::new(),
                place: "last".into(),
                before: None,
                at_end: true,
                after: Vec::new(),
                pad: false,
            }
        } else if let Some(Json::Object(_)) = place {
            CreateChild {
                emit: s(c, "emit").unwrap_or_default(),
                place: "before".into(),
                before: place.and_then(|p| s(p, "before")),
                at_end: false,
                after: Vec::new(),
                pad: false,
            }
        } else {
            CreateChild {
                emit: s(c, "emit").unwrap_or_default(),
                place: place.and_then(Json::as_str).unwrap_or("last").to_owned(),
                before: None,
                at_end: false,
                after: strings(c, "after"),
                pad: flag(c, "pad"),
            }
        }
    });
    AttributeBinding {
        slot: read_slot(json),
        empty: s(json, "empty"),
        absent: json
            .get("absent")
            .and_then(Json::as_object)
            .map(|a| {
                a.iter()
                    .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("insert").to_owned()))
                    .collect()
            })
            .unwrap_or_default(),
        default: json.get("default").cloned(),
        decimals,
        keep_time_precision: s(json, "time").as_deref() == Some("keep-precision"),
        style: s(json, "style"),
        reference: json.get("reference").map(|r| ReferenceBinding {
            to: strings(r, "to"),
            by: s(r, "by").unwrap_or_default(),
        }),
        map,
        override_slot: json.get("override").map(read_slot),
        html_paragraphs: s(json, "content").as_deref() == Some("html-paragraphs"),
        create,
        read_only: read_only(json),
        read: s(json, "read"),
        quoted: s(json, "quoted"),
        fallback: json.get("fallback").filter(|f| f.is_object()).map(|f| FallbackBinding {
            value: f.get("value").cloned().unwrap_or(Json::Null),
            report: f.get("report").and_then(Json::as_bool) != Some(false),
        }),
        gather: array(json, "gather").iter().map(read_slot).collect(),
        secondary: json.get("secondary").map(|slot| {
            Box::new(AttributeBinding {
                empty: Some("remove".into()),
                ..AttributeBinding::of_slot(read_slot(slot))
            })
        }),
        refusal_empty: json.get("refusals").and_then(|r| localized(r, "empty")),
        refusal_absent: json.get("refusals").and_then(|r| localized(r, "absent")),
    }
}

fn read_registration(json: &Json, pointer: &str, problems: &mut Vec<LoadProblem>) -> RegistrationSettings {
    let Some(r) = json.get("registration") else {
        return RegistrationSettings::default();
    };
    let blocks = read_blocks(r, &format!("{pointer}/registration"), problems);
    RegistrationSettings {
        headers: r
            .get("headers")
            .and_then(Json::as_object)
            .map(|h| h.keys().cloned().collect())
            .unwrap_or_default(),
        create_on_first_placement: flag(r, "createOnFirstPlacement"),
        blocks,
        resource_capture: r.get("resource").and_then(|res| s(res, "capture")),
        resource_by_origin: r
            .get("resource")
            .and_then(|res| res.get("byOrigin"))
            .and_then(Json::as_object)
            .map(|origins| origins.iter().map(|(o, c)| (o.clone(), s(c, "capture"))).collect())
            .unwrap_or_default(),
        legacy_layout: s(r, "legacyLayout"),
        legacy_identities: s(r, "legacyIdentities"),
    }
}

/// A registration's declared blocks (FBL 0.7 section 8.1): each a Name, with its order, its keys and whether it is in
/// the history.
fn read_blocks(
    registration: &Json,
    pointer: &str,
    problems: &mut Vec<LoadProblem>,
) -> Vec<RegistrationBlockDeclaration> {
    let mut blocks = Vec::new();
    let Some(declared) = registration.get("blocks") else {
        return blocks;
    };
    let Some(declared) = declared.as_object() else {
        problems.push(error(
            format!("{pointer}/blocks"),
            "A registration's blocks is an object.",
        ));
        return blocks;
    };
    for (name, block) in declared {
        let at = format!("{pointer}/blocks/{}", escape(name));
        if !block.is_object() {
            problems.push(error(at, "A declared block is an object."));
            continue;
        }
        if !is_name(name) || name == "layout" || name == "identities" {
            problems.push(error(
                at,
                format!(
                    "'{name}' cannot name a declared block: it is a Name, and neither layout nor identities (FBL 8.1)."
                ),
            ));
            continue;
        }
        let order = s(block, "order").unwrap_or_else(|| "ordinal".into());
        if order != "ordinal" && order != "kept" {
            problems.push(error(
                format!("{at}/order"),
                "A block's order is \"ordinal\" or \"kept\".",
            ));
        }
        let keys = s(block, "keys").unwrap_or_else(|| "names".into());
        if keys != "names" && keys != "ids" {
            problems.push(error(format!("{at}/keys"), "A block's keys are \"names\" or \"ids\"."));
        }
        blocks.push(RegistrationBlockDeclaration {
            name: name.clone(),
            kept: order == "kept",
            keyed_by_ids: keys == "ids",
            in_history: block.get("history") != Some(&Json::Bool(false)),
        });
    }
    blocks
}

// ---- the checks of FBL section 14.1 steps 4 to 6 that need no DISL specification ----

fn check(binding: &FblBinding, pointer: &str, problems: &mut Vec<LoadProblem>) {
    let rules = binding.all_rules();
    let mut names: Vec<&str> = Vec::new();
    let rule_names: Vec<&str> = rules
        .iter()
        .map(|r| r.name.as_str())
        .chain(binding.blocks.iter().map(|b| b.name.as_str()))
        .chain(binding.sections.iter().map(|section| section.name.as_str()))
        .collect();
    let file_rules: Vec<&str> = binding.body.files.iter().filter_map(|f| f.name.as_deref()).collect();

    for (i, block) in binding.blocks.iter().enumerate() {
        let at = format!("{pointer}/blocks/{i}");
        if names.contains(&block.name.as_str()) {
            problems.push(error(
                format!("{at}/name"),
                format!("The rule name '{}' is used twice.", block.name),
            ));
        } else {
            names.push(&block.name);
        }
        check_regex(&block.line, &format!("{at}/line"), problems);
        check_origins(&block.origins, binding, &format!("{at}/origins"), problems);
        check_within(block.within.as_deref(), &rule_names, &format!("{at}/within"), problems);
    }

    check_sections(binding, pointer, problems);
    let element_count = binding.elements().count();
    for (i, rule) in rules.iter().enumerate() {
        let at = if rule.is_relation {
            format!("{pointer}/relations/{}", i - element_count)
        } else {
            format!("{pointer}/elements/{i}")
        };
        if !is_name(&rule.name) {
            problems.push(error(
                format!("{at}/name"),
                format!("'{}' is not a valid rule name.", rule.name),
            ));
        }
        if names.contains(&rule.name.as_str()) {
            problems.push(error(
                format!("{at}/name"),
                format!("The rule name '{}' is used twice.", rule.name),
            ));
        } else {
            names.push(&rule.name);
        }
        check_origins(&rule.origins, binding, &format!("{at}/origins"), problems);
        if rule.at.is_none() == rule.line.is_none() && binding.plugin.is_none() {
            problems.push(error(at.clone(), "A rule has exactly one of 'at' and 'line'."));
        }
        if let Some(line) = &rule.line {
            check_regex(line, &format!("{at}/line"), problems);
        }
        check_within(rule.within.as_deref(), &rule_names, &format!("{at}/within"), problems);
        for file in &rule.files {
            if !file_rules.contains(&file.as_str()) {
                problems.push(error(format!("{at}/files"), format!("No file rule is named '{file}'.")));
            }
        }
        if let Some(parent) = &rule.parent {
            for p in &parent.rules {
                require(p, rules, &format!("{at}/parent/rules"), problems);
            }
        }
        if let Some(remove) = &rule.remove {
            for c in &remove.cascade {
                require(c, rules, &format!("{at}/remove/cascade"), problems);
            }
        }
        let families = families_of(binding, rule);
        let scalars = !families.is_empty() && families.iter().all(|f| matches!(f, Family::Yaml | Family::Json));
        let context = if rule.at.is_none() {
            CelContext::Lines
        } else if scalars {
            CelContext::Scalars
        } else {
            CelContext::Tree
        };
        compile(rule.when.as_deref(), context, &format!("{at}/when"), problems);
        compile(
            rule.insert.as_ref().and_then(|i| i.when.as_deref()),
            CelContext::Insert,
            &format!("{at}/insert/when"),
            problems,
        );
        if let Some(from) = rule.id.as_ref().and_then(|i| i.from.as_ref()) {
            check_slot(from, context, &format!("{at}/id/from"), problems);
        }
        if let Some(sidecar) = rule.id.as_ref().and_then(|i| i.sidecar_key.as_deref()) {
            compile(Some(sidecar), context, &format!("{at}/id/sidecar/key"), problems);
            if rule
                .id
                .as_ref()
                .and_then(|i| i.sidecar_ordinal.as_deref())
                .is_some_and(|ordinal| ordinal.matches("{n}").count() != 1)
            {
                problems.push(error(
                    format!("{at}/id/sidecar/ordinal"),
                    "'ordinal' holds {n} once (FBL 8.6).",
                ));
            }
        }
        if rule.is_relation && (rule.source.is_none() || rule.target.is_none()) {
            problems.push(error(at.clone(), "A relation rule has a source and a target."));
        }
        for (name, attribute) in &rule.attributes {
            let a = format!("{at}/attributes/{}", escape(name));
            check_slot(attribute, context, &a, problems);
            if let Some(reference) = &attribute.reference {
                for to in &reference.to {
                    require(to, rules, &format!("{a}/reference/to"), problems);
                }
            }
            if let Some(o) = &attribute.override_slot {
                check_slot(o, context, &format!("{a}/override"), problems);
            }
            if let Some(secondary) = &attribute.secondary {
                check_slot(secondary, context, &format!("{a}/secondary"), problems);
                if attribute.override_slot.is_some() {
                    problems.push(error(
                        format!("{a}/secondary"),
                        "An attribute binding has no 'secondary' beside 'override' (FBL section 14.1).",
                    ));
                }
            }
            check_scalar_options(attribute, scalars, &a, problems);
            check_grammar_options(attribute, &a, problems);
            if count_slots(attribute) != 1 {
                problems.push(error(
                    a.clone(),
                    "An attribute binding names exactly one slot (FBL §3.3).",
                ));
            }
        }
        check_new_member(binding, rule, &at, problems);
        if let Some(source) = &rule.source {
            check_slot(source, context, &format!("{at}/source"), problems);
        }
        if let Some(target) = &rule.target {
            check_slot(target, context, &format!("{at}/target"), problems);
        }
    }

    for (origin, _) in &binding.registration.resource_by_origin {
        check_origins(
            std::slice::from_ref(origin),
            binding,
            &format!("{pointer}/registration/resource/byOrigin"),
            problems,
        );
    }

    if binding.plugin.is_none() {
        if !binding.body.is_folder && binding.body.family.is_none() {
            problems.push(error(
                format!("{pointer}/body/family"),
                "A declared file body names its family.",
            ));
        }
        if rules.is_empty() {
            problems.push(error(
                pointer,
                "A declared binding has at least one element or relation rule.",
            ));
        }
    } else if !rules.is_empty() {
        problems.push(error(
            pointer,
            "A binding read by a plugin has no rules; its model is what the plugin reads.",
        ));
    }
    if binding.claims.shared && binding.claims.marker.is_none() && !binding.claims.registration_only {
        problems.push(error(
            format!("{pointer}/claims"),
            "A shared claim has a marker or is registrationOnly.",
        ));
    }
    if binding.claims.readings.iter().filter(|(_, r)| r.bare).count() > 1 {
        problems.push(error(
            format!("{pointer}/claims/readings"),
            "At most one reading is bare.",
        ));
    }
    if let Some(comment) = &binding.comment {
        check_regex(comment, &format!("{pointer}/comment"), problems);
    }
    if let Some(line) = binding.header.as_ref().and_then(|h| h.line.as_deref()) {
        check_regex(line, &format!("{pointer}/header/line"), problems);
    }
    if let Some(pattern) = binding.claims.marker.as_ref().and_then(|m| m.pattern.as_deref()) {
        check_regex(pattern, &format!("{pointer}/claims/marker/pattern"), problems);
    }
}

/// *(FBL 0.10)* Every origin a rule or `resource.byOrigin` names is one of the binding's claims (FBL section 14.1).
fn check_origins(origins: &[String], binding: &FblBinding, pointer: &str, problems: &mut Vec<LoadProblem>) {
    for origin in origins.iter().filter(|o| !binding.claims.origins.contains(o)) {
        problems.push(error(
            pointer,
            format!("The origin '{origin}' is not one of the binding's claims."),
        ));
    }
}

/// The families a rule reads: those of the file rules it names, else the body's, else those of every file rule.
fn families_of(binding: &FblBinding, rule: &Rule) -> Vec<Family> {
    if !rule.files.is_empty() {
        return binding
            .body
            .files
            .iter()
            .filter(|f| f.name.as_ref().is_some_and(|n| rule.files.contains(n)))
            .filter_map(|f| f.family)
            .collect();
    }
    if let Some(family) = binding.body.family {
        return vec![family];
    }
    binding.body.files.iter().filter_map(|f| f.family).collect()
}

/// *(FBL 0.14)* A rule of the yaml or json family that adds a member binds an attribute to the capture its `at` ends in,
/// and only such a rule has the sentences of a member's key (FBL section 14.1).
fn check_new_member(binding: &FblBinding, rule: &Rule, pointer: &str, problems: &mut Vec<LoadProblem>) {
    let Some(insert) = &rule.insert else {
        return;
    };
    let tree = families_of(binding, rule)
        .iter()
        .any(|f| matches!(f, Family::Yaml | Family::Json));
    let adds_member = tree && rule.member_capture().is_some();
    if adds_member && rule.member_key_attribute().is_none() {
        problems.push(error(
            format!("{pointer}/at"),
            format!(
                "A rule that adds a member of a mapping binds an attribute to {{capture: {}}} (FBL 0.14 section 14.1).",
                rule.member_capture().unwrap_or_default()
            ),
        ));
    }
    if !adds_member && (insert.refusal_key.is_some() || insert.refusal_duplicate.is_some()) {
        problems.push(error(
            format!("{pointer}/insert/refusals"),
            "'key' and 'duplicate' are the sentences of an insert that adds a member (FBL 0.14 section 14.1).",
        ));
    }
}

/// FBL 0.5 section 5.2: `read`, `quoted` and `fallback` are given only on a slot of the yaml or json family that
/// reads a scalar, so never in another family nor on a `value` or `capture` slot.
fn check_scalar_options(attribute: &AttributeBinding, scalars: bool, pointer: &str, problems: &mut Vec<LoadProblem>) {
    let place = if !scalars {
        "a slot of the xml, lines, blocks or bullets family"
    } else if attribute.value.is_some() {
        "a value slot"
    } else if attribute.capture.is_some() {
        "a capture slot"
    } else {
        return;
    };
    for (option, given) in [
        ("read", attribute.read.is_some()),
        ("quoted", attribute.quoted.is_some()),
        ("fallback", attribute.fallback.is_some()),
    ] {
        if given {
            problems.push(error(
                format!("{pointer}/{option}"),
                format!("'{option}' is not given on {place} (FBL 5.2)."),
            ));
        }
    }
}

/// FBL 0.11 section 14.1: `gather` is given on a read-only binding only; FBL 0.13: `split` holds neither of the
/// characters of the slot's `escape`.
fn check_grammar_options(attribute: &AttributeBinding, pointer: &str, problems: &mut Vec<LoadProblem>) {
    if !attribute.gather.is_empty() && attribute.read_only.is_none() {
        problems.push(error(
            format!("{pointer}/gather"),
            "A binding with 'gather' is read-only (FBL 5.2).",
        ));
    }
    for slot in std::iter::once(&attribute.slot).chain(&attribute.gather) {
        if let (Some(split), Some(escape)) = (&slot.split, &slot.escape)
            && split.contains([escape.with, escape.quote])
        {
            problems.push(error(
                format!("{pointer}/split"),
                "'split' holds neither of the characters of the slot's 'escape' (FBL 4.9).",
            ));
        }
    }
}

fn count_slots(s: &Slot) -> usize {
    usize::from(s.key.is_some())
        + usize::from(s.xml_attribute.is_some())
        + usize::from(s.text)
        + usize::from(s.group.is_some())
        + usize::from(s.parent.is_some())
        + usize::from(s.capture.is_some())
        + usize::from(s.value.is_some())
        + usize::from(s.continuation)
        + usize::from(s.node.is_some())
}

/// The sections of a bullets body (FBL 0.7 section 4.8): a name each, used once, and a heading expression of FBL's
/// subset.
fn check_sections(binding: &FblBinding, pointer: &str, problems: &mut Vec<LoadProblem>) {
    let mut seen: Vec<&str> = Vec::new();
    for (i, section) in binding.sections.iter().enumerate() {
        let at = format!("{pointer}/sections/{i}");
        if seen.contains(&section.name.as_str()) {
            problems.push(error(
                format!("{at}/name"),
                format!("The section name '{}' is used twice.", section.name),
            ));
        } else {
            seen.push(&section.name);
        }
        check_regex(&section.heading, &format!("{at}/heading"), problems);
    }
}

fn check_slot(slot: &Slot, context: CelContext, pointer: &str, problems: &mut Vec<LoadProblem>) {
    compile(slot.value.as_deref(), context, &format!("{pointer}/value"), problems);
    if let Some(word) = &slot.word {
        check_regex(word, &format!("{pointer}/word"), problems);
    }
}

fn require(name: &str, rules: &[Rule], pointer: &str, problems: &mut Vec<LoadProblem>) {
    if rules.iter().all(|r| r.name != name) {
        problems.push(error(pointer, format!("No rule is named '{name}'.")));
    }
}

fn check_within(within: Option<&[String]>, names: &[&str], pointer: &str, problems: &mut Vec<LoadProblem>) {
    for w in within.unwrap_or_default() {
        if w != "^" && !names.contains(&w.as_str()) {
            problems.push(error(pointer, format!("No rule is named '{w}'.")));
        }
    }
}

fn check_regex(expression: &str, pointer: &str, problems: &mut Vec<LoadProblem>) {
    if let Some(message) = adp_cel::regex_subset::check(expression) {
        problems.push(error(pointer, message));
    }
}

fn compile(expression: Option<&str>, context: CelContext, pointer: &str, problems: &mut Vec<LoadProblem>) {
    if let Some(expression) = expression
        && let Err(message) = expressions::compile(expression, context)
    {
        problems.push(error(pointer, message));
    }
}
