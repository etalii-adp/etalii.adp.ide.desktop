//! Settings against the schema (ATD §6.4, §6.6, §14.6): each setting's JSON value checked against the property of
//! the DISL, DESL or FBL `$defs` entry its construct carries (Appendix C). An unknown key is `ATD5001`, a value the
//! schema does not admit `ATD5002`. A CEL expression, an attribute, a token or a parameter reference is not judged by
//! the schema, which cannot know its value.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Mutex, OnceLock};

use jsonschema::error::ValidationErrorKind;
use jsonschema::{Retrieve, Uri, Validator};
use serde_json::{Map, Value as Json, json};

use crate::description::Description;
use crate::embedded::{DESL_SCHEMA, DISL_SCHEMA, FBL_SCHEMA};
use crate::suggest::{closest, did_you_mean};
use crate::symbols::Symbols;
use crate::{Finding, finding_at};

const DISL: &str = "https://etalii.net/adp/disl/schema/0.2/disl.schema.json";
const DESL: &str = "https://etalii.net/adp/desl/schema/0.1/desl.schema.json";
const FBL: &str = "https://etalii.net/adp/fbl/schema/0.1/fbl.schema.json";

fn schemas() -> &'static BTreeMap<&'static str, Json> {
    static SCHEMAS: OnceLock<BTreeMap<&'static str, Json>> = OnceLock::new();
    SCHEMAS.get_or_init(|| {
        BTreeMap::from([
            (DISL, serde_json::from_str(DISL_SCHEMA).expect("DISL's schema")),
            (DESL, serde_json::from_str(DESL_SCHEMA).expect("DESL's schema")),
            (FBL, serde_json::from_str(FBL_SCHEMA).expect("FBL's schema")),
        ])
    })
}

struct Embedded;

impl Retrieve for Embedded {
    fn retrieve(&self, uri: &Uri<String>) -> Result<Json, Box<dyn std::error::Error + Send + Sync>> {
        let id = uri.as_str().trim_end_matches('#');
        schemas()
            .get(id)
            .cloned()
            .ok_or_else(|| format!("'{id}' is not an embedded schema").into())
    }
}

/// A validator for the schema at `pointer` (an absolute URI with a fragment), compiled once.
fn validator(pointer: &str) -> Option<&'static Validator> {
    static CACHE: OnceLock<Mutex<BTreeMap<String, Option<&'static Validator>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(BTreeMap::new()));
    let mut cache = cache.lock().ok()?;
    cache
        .entry(pointer.to_owned())
        .or_insert_with(|| {
            jsonschema::options()
                .with_retriever(Embedded)
                .build(&json!({"$ref": pointer}))
                .ok()
                .map(|v| &*Box::leak(Box::new(v)))
        })
        .as_ref()
        .copied()
}

/// The schema value a `#/$defs/...` reference names, within a schema document.
fn resolve<'a>(schema: &'a Json, document: &'a Json) -> &'a Json {
    let mut current = schema;
    for _ in 0..16 {
        match current.get("$ref").and_then(Json::as_str) {
            Some(reference) if reference.starts_with("#/") => {
                let mut target = document;
                for part in reference[2..].split('/') {
                    target = &target[part];
                }
                current = target;
            }
            _ => break,
        }
    }
    current
}

/// The properties a schema admits, with the JSON pointer of each, following `$ref`, `allOf`, `anyOf`, `oneOf`,
/// `then` and `else`; and whether it admits others.
fn properties(
    schema: &Json,
    pointer: &str,
    document: &Json,
    out: &mut BTreeMap<String, String>,
    open: &mut bool,
    depth: usize,
) {
    if depth > 8 {
        return;
    }
    if let Some(reference) = schema.get("$ref").and_then(Json::as_str)
        && reference.starts_with("#/")
    {
        let target = resolve(schema, document);
        properties(target, &format!("#{}", &reference[1..]), document, out, open, depth + 1);
        return;
    }
    if let Some(map) = schema.get("properties").and_then(Json::as_object) {
        for key in map.keys() {
            out.entry(key.clone())
                .or_insert_with(|| format!("{pointer}/properties/{}", escape(key)));
        }
    }
    if schema.get("additionalProperties").is_some_and(|a| a != &json!(false)) {
        *open = true;
    }
    if schema.get("type") != Some(&json!("object")) && schema.get("properties").is_none() {
        let composite =
            ["allOf", "anyOf", "oneOf"].iter().any(|k| schema.get(*k).is_some()) || schema.get("then").is_some();
        if !composite {
            *open = true;
        }
    }
    for key in ["allOf", "anyOf", "oneOf"] {
        for (i, branch) in schema
            .get(key)
            .and_then(Json::as_array)
            .into_iter()
            .flatten()
            .enumerate()
        {
            properties(branch, &format!("{pointer}/{key}/{i}"), document, out, open, depth + 1);
        }
    }
    for key in ["then", "else"] {
        if let Some(branch) = schema.get(key) {
            properties(branch, &format!("{pointer}/{key}"), document, out, open, depth + 1);
        }
    }
}

fn escape(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

/// A `$defs` entry: its document's id and its name.
#[derive(Debug, Clone, Copy)]
struct Def(&'static str, &'static str);

impl Def {
    fn properties(self) -> (BTreeMap<String, String>, bool) {
        let document = &schemas()[self.0];
        let mut out = BTreeMap::new();
        let mut open = false;
        let schema = &document["$defs"][self.1];
        if schema.is_null() {
            return (out, true);
        }
        properties(schema, &format!("#/$defs/{}", self.1), document, &mut out, &mut open, 0);
        (out, open)
    }
}

/// A setting's value as JSON, with the spans of its parts by JSON pointer and the pointers of values the schema
/// cannot judge (CEL and references).
fn to_json(value: &Json, pointer: &str, spans: &mut Vec<(String, Json, bool)>) -> Json {
    let judged = !(value.get("cel").is_some()
        || value.get("attribute").is_some()
        || value.get("token").is_some()
        || value.get("param").is_some());
    spans.push((pointer.to_owned(), value.clone(), judged));
    if let Some(literal) = value.get("literal") {
        return literal.clone();
    }
    if let Some(cel) = value.get("cel") {
        return json!({"cel": cel});
    }
    for key in ["attribute", "token", "param"] {
        if let Some(name) = value.get(key) {
            return json!({key: name});
        }
    }
    if let Some(entries) = value.get("settings").and_then(Json::as_array) {
        let mut object = Map::new();
        for entry in entries {
            let key = entry["key"].as_str().unwrap_or("");
            let child = format!("{pointer}/{}", escape(key));
            spans.push((format!("{child}#key"), entry.clone(), true));
            object.insert(key.to_owned(), to_json(&entry["value"], &child, spans));
        }
        return Json::Object(object);
    }
    if let Some(items) = value.get("list").and_then(Json::as_array) {
        return Json::Array(
            items
                .iter()
                .enumerate()
                .map(|(i, item)| to_json(item, &format!("{pointer}/{i}"), spans))
                .collect(),
        );
    }
    Json::Null
}

pub fn run(description: &Description, symbols: &Symbols, findings: &mut Vec<Finding>) {
    let designer = description.kind() == Some("designer");
    let _ = symbols;
    let mut checker = SettingsChecker { designer, findings };
    if let Some(tool) = description.tool() {
        let language = if designer {
            Def(DESL, "Language")
        } else {
            Def(DISL, "Language")
        };
        checker.members(tool, &[language]);
    }
    for member in description.members() {
        checker.member(member);
    }
}

struct SettingsChecker<'a> {
    designer: bool,
    findings: &'a mut Vec<Finding>,
}

impl SettingsChecker<'_> {
    fn pick(&self, disl: &'static str, desl: &'static str) -> Def {
        if self.designer {
            Def(DESL, desl)
        } else {
            Def(DISL, disl)
        }
    }

    /// The settings among a construct's members (`kind: setting`), against `defs`.
    fn members(&mut self, node: &Json, defs: &[Def]) {
        for member in node["members"].as_array().into_iter().flatten() {
            if member["kind"] == "setting" {
                self.setting(&member["setting"], defs);
            }
        }
    }

    /// A construct's `settings` list, against `defs`.
    fn settings(&mut self, node: &Json, defs: &[Def]) {
        for setting in node["settings"].as_array().into_iter().flatten() {
            self.setting(setting, defs);
        }
    }

    fn setting(&mut self, setting: &Json, defs: &[Def]) {
        let key = setting["key"].as_str().unwrap_or("");
        if key.starts_with("x-") || defs.is_empty() {
            return;
        }
        let mut known: BTreeMap<String, (Def, String)> = BTreeMap::new();
        let mut open = false;
        for def in defs {
            let (props, def_open) = def.properties();
            open |= def_open;
            for (k, pointer) in props {
                known.entry(k).or_insert((*def, pointer));
            }
        }
        let candidates: Vec<(Def, String)> = defs
            .iter()
            .filter_map(|def| def.properties().0.get(key).map(|pointer| (*def, pointer.clone())))
            .collect();
        let Some((def, pointer)) = known.get(key).cloned() else {
            if !open {
                let suggestions = closest(key, known.keys().map(String::as_str).collect::<Vec<_>>());
                let mut finding = finding_at(
                    "ATD5001",
                    setting,
                    format!(
                        "`{key}` is not a setting of {} (`$defs/{}`).{}",
                        describe(defs),
                        defs[0].1,
                        did_you_mean(&suggestions)
                    ),
                );
                finding.end = finding.start + key.len();
                finding.end_column = finding.column + key.chars().count();
                finding.suggestions = suggestions;
                self.findings.push(finding);
            }
            return;
        };
        let mut spans = Vec::new();
        let instance = to_json(&setting["value"], "", &mut spans);
        // A key two defs share (a node's own and its inline style's) is right when either admits its value.
        if candidates.len() > 1
            && candidates
                .iter()
                .any(|(d, p)| validator(&format!("{}{p}", d.0)).is_some_and(|v| v.is_valid(&instance)))
        {
            return;
        }
        let Some(validator) = validator(&format!("{}{pointer}", def.0)) else {
            return;
        };
        for error in validator.iter_errors(&instance) {
            let path = error.instance_path().to_string();
            // An error at, or under, a value the schema cannot judge is not the description's.
            if spans
                .iter()
                .any(|(p, _, judged)| !judged && (path == *p || path.starts_with(&format!("{p}/"))))
            {
                continue;
            }
            match error.kind() {
                ValidationErrorKind::AdditionalProperties { unexpected }
                | ValidationErrorKind::UnevaluatedProperties { unexpected } => {
                    for name in unexpected {
                        let at = format!("{path}/{}#key", escape(name));
                        let node = spans
                            .iter()
                            .find(|(p, _, _)| *p == at)
                            .map(|(_, n, _)| n.clone())
                            .unwrap_or_else(|| setting.clone());
                        let siblings = self.siblings(def, &pointer, &path);
                        let suggestions = closest(name, siblings.iter().map(String::as_str).collect::<Vec<_>>());
                        let mut finding = finding_at(
                            "ATD5001",
                            &node,
                            format!(
                                "`{name}` is not a setting of `{key}`{}.{}",
                                where_(&path),
                                did_you_mean(&suggestions)
                            ),
                        );
                        finding.end = finding.start + name.len();
                        finding.end_column = finding.column + name.chars().count();
                        finding.suggestions = suggestions;
                        self.findings.push(finding);
                    }
                }
                _ => {
                    let node = spans
                        .iter()
                        .filter(|(p, _, _)| path.starts_with(p.as_str()) && !p.ends_with("#key"))
                        .max_by_key(|(p, _, _)| p.len())
                        .map(|(_, n, _)| n.clone())
                        .unwrap_or_else(|| setting["value"].clone());
                    self.findings.push(finding_at(
                        "ATD5002",
                        &node,
                        format!(
                            "The value of `{key}`{} is not one the schema admits: {error}.",
                            where_(&path)
                        ),
                    ));
                }
            }
        }
    }

    /// The property names beside an unknown one: of the object the schema expects at `path` within `pointer`.
    fn siblings(&self, def: Def, pointer: &str, path: &str) -> Vec<String> {
        let document = &schemas()[def.0];
        let mut schema = resolve(lookup(document, pointer), document);
        for part in path.split('/').filter(|p| !p.is_empty()) {
            let mut props = BTreeMap::new();
            let mut open = false;
            properties(schema, "", document, &mut props, &mut open, 0);
            let Some(next) = props.get(part) else { break };
            schema = resolve(lookup_relative(schema, document, next), document);
        }
        let mut props = BTreeMap::new();
        let mut open = false;
        properties(schema, "", document, &mut props, &mut open, 0);
        props.into_keys().collect()
    }

    fn member(&mut self, member: &Json) {
        let kind = member["kind"].as_str().unwrap_or("");
        match kind {
            "model" => {
                self.members(member, &[self.pick("Metamodel", "Metamodel")]);
                for item in member["members"].as_array().into_iter().flatten() {
                    self.model_item(item);
                }
            }
            "constraints" => {
                self.members(member, &[self.pick("Constraints", "Constraints")]);
                for item in member["members"].as_array().into_iter().flatten() {
                    self.member(item);
                }
            }
            "constraint" => {
                self.settings(member, &[self.pick("Constraint", "Constraint")]);
                for fix in member["fixes"].as_array().into_iter().flatten() {
                    self.settings(fix, &[Def(DISL, "QuickFix")]);
                }
            }
            "builtin" => self.settings(member, &[Def(DISL, "BuiltInSetting")]),
            "operation" => {
                self.settings(member, &[self.pick("Operation", "Operation")]);
                for param in member["params"].as_array().into_iter().flatten() {
                    self.settings(param, &[self.pick("Attribute", "Attribute")]);
                }
                if let Some(confirm) = member["confirm"].as_array() {
                    for setting in confirm {
                        self.setting(setting, &[self.pick("Confirmation", "Confirmation")]);
                    }
                }
            }
            "hook" => self.settings(member, &[Def(DISL, "Hook")]),
            "behavior" => self.settings(member, &[Def(DISL, "Behavior")]),
            "persistence" => self.settings(member, &[self.pick("Persistence", "Persistence")]),
            "plugin" => self.settings(member, &[Def(DISL, "Plugin")]),
            "import" => self.settings(member, &[Def(DISL, "Import")]),
            "viewpoint" => self.settings(member, &[Def(DISL, "Viewpoint")]),
            "coordinates" => self.generic(member, &[Def(DISL, "Coordinates")], &coordinates_def),
            "notation" => self.generic(member, &[Def(DISL, "Notation")], &notation_def),
            "toolbox" => self.generic(member, &[Def(DISL, "Toolbox")], &toolbox_def),
            "form" => self.generic(member, &[Def(DISL, "Form")], &|_, _| vec![Def(DISL, "FormItem")]),
            "layout" => self.layout(member),
            "tableSurface" => self.generic(member, &[Def(DESL, "Table")], &table_def),
            "hierarchySurface" => self.generic(member, &[Def(DESL, "Hierarchy")], &hierarchy_def),
            "binding" => self.binding(member),
            _ => {}
        }
    }

    fn model_item(&mut self, item: &Json) {
        match item["kind"].as_str().unwrap_or("") {
            "element" | "document" => {
                self.members(item, &[self.pick("NodeType", "ElementType")]);
                self.element_members(item);
            }
            "relation" => {
                self.members(item, &[Def(DISL, "RelationType")]);
                self.element_members(item);
            }
            "enum" => {
                self.settings(item, &[self.pick("Enum", "Enum")]);
                for value in item["values"].as_array().into_iter().flatten() {
                    self.settings(value, &[self.pick("EnumValue", "EnumValue")]);
                }
            }
            "datatype" => self.settings(item, &[Def(DISL, "DataType")]),
            _ => {}
        }
    }

    fn element_members(&mut self, item: &Json) {
        for member in item["members"].as_array().into_iter().flatten() {
            match member["kind"].as_str().unwrap_or("") {
                "attribute" => self.settings(member, &[self.pick("Attribute", "Attribute")]),
                "contains" => self.settings(member, &[self.pick("Containment", "Containment")]),
                "port" => self.settings(member, &[Def(DISL, "PortType")]),
                _ => {}
            }
        }
    }

    /// A surface construct whose members are generic: its own settings against `own`, each member's against the
    /// defs `defs_of` names for its kind and parent, recursively.
    fn generic(&mut self, node: &Json, own: &[Def], defs_of: &dyn Fn(&str, &Json) -> Vec<Def>) {
        self.members(node, own);
        let parent = node["kind"].as_str().unwrap_or("");
        for member in node["members"].as_array().into_iter().flatten() {
            let kind = member["kind"].as_str().unwrap_or("");
            if kind == "setting" {
                continue;
            }
            let defs = if parent == "context" && kind == "tool" {
                vec![Def(DISL, "ContextTool")]
            } else {
                defs_of(kind, member)
            };
            self.settings(member, &defs);
            if member.get("members").is_some() {
                self.generic(member, &defs, defs_of);
            }
        }
    }

    fn layout(&mut self, node: &Json) {
        self.members(node, &[Def(DISL, "Layout")]);
        for member in node["members"].as_array().into_iter().flatten() {
            if member["kind"] != "algorithm" {
                continue;
            }
            match member.get("capability").and_then(|c| c["text"].as_str()) {
                Some(capability) => self.arguments(
                    member["settings"].as_array().map(Vec::as_slice).unwrap_or(&[]),
                    capability,
                ),
                None => self.settings(member, &[Def(DISL, "LayoutConfig")]),
            }
        }
    }

    /// Settings that are a capability's arguments, against its contract's `args` (ATD §13.3).
    fn arguments(&mut self, settings: &[Json], capability: &str) {
        let Some(contract) = crate::embedded::contract(capability) else {
            return;
        };
        let args: BTreeSet<String> = contract["args"]
            .as_object()
            .map(|a| a.keys().cloned().collect())
            .unwrap_or_default();
        for setting in settings {
            let key = setting["key"].as_str().unwrap_or("");
            if key.starts_with("x-") || args.contains(key) {
                continue;
            }
            let suggestions = closest(key, args.iter().map(String::as_str).collect::<Vec<_>>());
            let mut finding = finding_at(
                "ATD5001",
                setting,
                format!(
                    "`{key}` is not an argument of the capability `{capability}`.{}",
                    did_you_mean(&suggestions)
                ),
            );
            finding.end = finding.start + key.len();
            finding.end_column = finding.column + key.chars().count();
            finding.suggestions = suggestions;
            self.findings.push(finding);
        }
    }

    fn binding(&mut self, binding: &Json) {
        let capability = binding
            .get("capability")
            .and_then(|c| c["text"].as_str())
            .map(str::to_owned);
        for member in binding["members"].as_array().into_iter().flatten() {
            let kind = member["kind"].as_str().unwrap_or("");
            match kind {
                "setting" => {
                    let setting = &member["setting"];
                    if setting["key"] == "args"
                        && let Some(capability) = &capability
                    {
                        let entries = setting["value"]["settings"].as_array().cloned().unwrap_or_default();
                        self.arguments(&entries, capability);
                    } else {
                        self.setting(setting, &[Def(FBL, "Binding")]);
                    }
                }
                "claims" => self.settings(member, &[Def(FBL, "Claims")]),
                "body" => self.settings(member, &[Def(FBL, "Body")]),
                "text" => self.settings(member, &[Def(FBL, "TextDefaults")]),
                "header" => self.settings(member, &[Def(FBL, "Header")]),
                "registration" => self.settings(member, &[Def(FBL, "RegistrationSettings")]),
                "block" => self.settings(member, &[Def(FBL, "BlockRule")]),
                "section" => self.settings(member, &[Def(FBL, "SectionRule")]),
                "rule" => {
                    let rule = if member["ruleKind"] == "relation" {
                        "RelationRule"
                    } else {
                        "ElementRule"
                    };
                    for item in member["members"].as_array().into_iter().flatten() {
                        let defs: Vec<Def> = match item["kind"].as_str().unwrap_or("") {
                            "setting" => {
                                self.setting(&item["setting"], &[Def(FBL, rule)]);
                                continue;
                            }
                            "attribute" | "source" | "target" => vec![Def(FBL, "AttributeBinding")],
                            "id" => vec![Def(FBL, "IdBinding")],
                            "insert" => vec![Def(FBL, "Insert")],
                            "remove" => vec![Def(FBL, "Remove")],
                            _ => continue,
                        };
                        self.settings(item, &defs);
                    }
                }
                _ => {}
            }
        }
    }
}

fn coordinates_def(kind: &str, _: &Json) -> Vec<Def> {
    match kind {
        "system" => vec![Def(DISL, "CoordinateSystem")],
        "axis" => vec![Def(DISL, "Axis")],
        "snapping" => vec![Def(DISL, "Snapping")],
        _ => Vec::new(),
    }
}

fn notation_def(kind: &str, member: &Json) -> Vec<Def> {
    let _ = member;
    match kind {
        "theme" => vec![Def(DISL, "Theme")],
        "style" | "state" => vec![Def(DISL, "Style")],
        "shape" => vec![Def(DISL, "ShapeDef")],
        "marker" => vec![Def(DISL, "MarkerDef")],
        "icon" => vec![Def(DISL, "IconDef")],
        "canvas" => vec![Def(DISL, "Canvas")],
        // A node's and an edge's settings that are a style's are its inline style (ATD §8.4, §8.5).
        "node" => vec![Def(DISL, "NodeNotation"), Def(DISL, "Style")],
        "edge" => vec![Def(DISL, "EdgeNotation"), Def(DISL, "Style")],
        "variant" => vec![Def(DISL, "NodeVariant"), Def(DISL, "EdgeVariant"), Def(DISL, "Style")],
        "label" => vec![Def(DISL, "Label")],
        "compartment" => vec![Def(DISL, "Compartment")],
        "badge" => vec![Def(DISL, "Badge")],
        "port" => vec![Def(DISL, "PortNotation")],
        "line" => vec![Def(DISL, "LineSpec")],
        _ => Vec::new(),
    }
}

fn toolbox_def(kind: &str, _: &Json) -> Vec<Def> {
    match kind {
        "group" => vec![Def(DISL, "ToolGroup")],
        "tool" => vec![Def(DISL, "Tool")],
        "context" => vec![Def(DISL, "ContextToolSet")],
        "template" => vec![Def(DISL, "Template")],
        _ => Vec::new(),
    }
}

fn table_def(kind: &str, _: &Json) -> Vec<Def> {
    match kind {
        "columns" => vec![Def(DESL, "Columns")],
        "cells" => vec![Def(DESL, "Cells")],
        "views" => vec![Def(DESL, "Views")],
        "valueType" => vec![Def(DESL, "ValueType")],
        _ => Vec::new(),
    }
}

fn hierarchy_def(kind: &str, _: &Json) -> Vec<Def> {
    match kind {
        "levels" | "entries" => vec![Def(DESL, "HierarchyMembers")],
        "notes" => vec![Def(DESL, "Notes")],
        "filters" => vec![Def(DESL, "Filters")],
        "widths" => vec![Def(DESL, "Widths")],
        "labels" => vec![Def(DESL, "Labels")],
        _ => Vec::new(),
    }
}

fn describe(defs: &[Def]) -> String {
    let names: Vec<String> = defs.iter().map(|d| format!("{} `{}`", language(d.0), d.1)).collect();
    names.join(" or ")
}

fn language(id: &str) -> &'static str {
    match id {
        DISL => "DISL's",
        DESL => "DESL's",
        _ => "FBL's",
    }
}

fn where_(path: &str) -> String {
    if path.is_empty() {
        String::new()
    } else {
        format!(" at `{path}`")
    }
}

fn lookup<'a>(document: &'a Json, pointer: &str) -> &'a Json {
    let mut current = document;
    for part in pointer.trim_start_matches('#').split('/').filter(|p| !p.is_empty()) {
        current = &current[part.replace("~1", "/").replace("~0", "~")];
    }
    current
}

fn lookup_relative<'a>(schema: &'a Json, document: &'a Json, pointer: &str) -> &'a Json {
    if pointer.starts_with("#/") {
        lookup(document, pointer)
    } else {
        let mut current = schema;
        for part in pointer.split('/').filter(|p| !p.is_empty()) {
            current = &current[part.replace("~1", "/").replace("~0", "~")];
        }
        current
    }
}
