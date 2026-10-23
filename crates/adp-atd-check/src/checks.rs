//! The checks of ATD §14.2 to §14.5 and §14.7: names and duplicates, the constructs a kind admits, the names a
//! surface and a binding use, the capabilities a description uses, the functions its expressions call and the types
//! of the expressions that must be `bool`.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value as Json;

use crate::cel::{self, Scope, Ty};
use crate::description::Description;
use crate::suggest::{closest, did_you_mean};
use crate::symbols::{BUILT_IN_FIELDS, Symbols};
use crate::{Finding, finding_at};

/// The types an attribute may have besides the description's own (DISL §4.2, DESL §4.2, CEL's).
const PRIMITIVES: &[&str] = &[
    "string",
    "text",
    "int",
    "integer",
    "number",
    "double",
    "float",
    "bool",
    "boolean",
    "date",
    "datetime",
    "time",
    "timestamp",
    "duration",
    "color",
    "uri",
    "url",
    "path",
    "expression",
    "json",
    "binary",
    "bytes",
    "dyn",
    "any",
    "yearMonth",
    "timezone",
    "list",
    "map",
    "uint",
    "Element",
];

/// Names that stand for the document itself where a construct names types it applies to.
const DOCUMENT_TYPES: &[&str] = &["document", "diagram"];

fn text(name: &Json) -> &str {
    name["text"].as_str().unwrap_or("")
}

pub fn run(description: &Description, symbols: &Symbols, findings: &mut Vec<Finding>) {
    let kind = description.kind().unwrap_or("diagram");
    duplicates(symbols, findings);
    kinds(description, kind, findings);
    let mut checker = Checker {
        symbols,
        findings,
        catalogue: crate::embedded::catalogue().into_iter().map(|(n, _)| n).collect(),
    };
    for member in description.members() {
        checker.member(member);
    }
    checker.calls(description);
}

fn duplicates(symbols: &Symbols, findings: &mut Vec<Finding>) {
    let mut seen: BTreeMap<(&str, String), &Json> = BTreeMap::new();
    for (namespace, name, node) in &symbols.declared {
        let key = (*namespace, name.to_lowercase());
        if let std::collections::btree_map::Entry::Vacant(e) = seen.entry(key) {
            e.insert(node);
        } else {
            findings.push(finding_at(
                "ATD2006",
                node,
                format!("`{name}` is declared twice in the description's {namespace}s."),
            ));
        }
    }
}

/// The table of kinds (ATD §14.4).
fn kinds(description: &Description, kind: &str, findings: &mut Vec<Finding>) {
    let article = |k: &str| {
        if k == "editor" {
            "An editor"
        } else if k == "designer" {
            "A designer"
        } else {
            "A diagram"
        }
    };
    let mut surfaces = 0;
    for member in description.members() {
        let member_kind = member["kind"].as_str().unwrap_or("");
        let refuse = |code: &'static str, what: &str, admitted: &str| {
            finding_at(
                code,
                member,
                format!(
                    "{} has no {what}; `{member_kind}` is admitted in {admitted} only.",
                    article(kind)
                ),
            )
        };
        match (member_kind, kind) {
            ("coordinates" | "notation" | "toolbox" | "layout" | "viewpoint", "designer" | "editor") => {
                let surface = if kind == "editor" {
                    "a text surface"
                } else {
                    "a form surface"
                };
                findings.push(finding_at(
                    "ATD3002",
                    member,
                    format!(
                        "{} has {surface}; `{member_kind}` belongs to the canvas of a diagram.",
                        article(kind)
                    ),
                ));
            }
            ("form", "editor") => findings.push(refuse("ATD3002", "forms", "diagrams and designers")),
            ("tableSurface" | "hierarchySurface", "diagram" | "editor") => {
                findings.push(refuse("ATD3002", "form surface", "designers"))
            }
            ("textSurface", "diagram" | "designer") => findings.push(refuse("ATD3002", "text surface", "editors")),
            ("hook", "designer") => findings.push(refuse("ATD3001", "hooks", "diagrams and editors")),
            _ => {}
        }
        if matches!(member_kind, "tableSurface" | "hierarchySurface" | "textSurface") {
            surfaces += 1;
        }
        if member_kind == "model" && kind != "diagram" {
            for item in member["members"].as_array().into_iter().flatten() {
                if item["kind"] == "relation" {
                    let name = text(&item["name"]);
                    findings.push(finding_at(
                        "ATD3001",
                        item,
                        format!(
                            "{} has no relation types; `relation {name}` is admitted in diagrams only.",
                            article(kind)
                        ),
                    ));
                }
                for port in item["members"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|m| m["kind"] == "port")
                {
                    findings.push(finding_at(
                        "ATD3001",
                        port,
                        format!("{} has no ports; `port` is admitted in diagrams only.", article(kind)),
                    ));
                }
            }
        }
    }
    if let Some(tool) = description.tool()
        && kind != "diagram"
        && surfaces != 1
    {
        let surface = if kind == "editor" {
            "`surface text`"
        } else {
            "`surface table` or `surface hierarchy`"
        };
        findings.push(finding_at(
            "ATD3003",
            tool,
            format!(
                "{} has exactly one surface, {surface}; this one has {surfaces}.",
                article(kind)
            ),
        ));
    }
}

struct Checker<'a> {
    symbols: &'a Symbols,
    findings: &'a mut Vec<Finding>,
    catalogue: BTreeSet<String>,
}

impl Checker<'_> {
    fn type_names(&self) -> impl Iterator<Item = &str> {
        self.symbols.types.keys().map(String::as_str)
    }

    /// A reference to an element or relation type.
    fn type_ref(&mut self, name: &Json, allow_document: bool) {
        let name_text = text(name);
        if self.symbols.is_type(name_text) || (allow_document && DOCUMENT_TYPES.contains(&name_text)) {
            return;
        }
        let suggestions = closest(name_text, self.type_names().collect::<Vec<_>>());
        let mut finding = finding_at(
            "ATD2002",
            name,
            format!("No element type `{name_text}`.{}", did_you_mean(&suggestions)),
        );
        finding.suggestions = suggestions;
        self.findings.push(finding);
    }

    /// A type an attribute or parameter is declared with.
    fn declared_type(&mut self, type_ref: &Json) {
        let name = &type_ref["name"];
        let name_text = text(name);
        let known = PRIMITIVES.contains(&name_text)
            || self.symbols.is_type(name_text)
            || self.symbols.enums.contains_key(name_text)
            || self.symbols.datatypes.contains_key(name_text);
        if !known {
            let candidates: Vec<&str> = PRIMITIVES
                .iter()
                .copied()
                .chain(self.symbols.types.keys().map(String::as_str))
                .chain(self.symbols.enums.keys().map(String::as_str))
                .chain(self.symbols.datatypes.keys().map(String::as_str))
                .collect();
            let suggestions = closest(name_text, candidates);
            let mut finding = finding_at(
                "ATD2002",
                name,
                format!("No type `{name_text}`.{}", did_you_mean(&suggestions)),
            );
            finding.suggestions = suggestions;
            self.findings.push(finding);
        }
        for argument in type_ref["arguments"].as_array().into_iter().flatten() {
            self.declared_type(argument);
        }
    }

    fn attribute_of(&mut self, types: &[String], name: &str, at: &Json) {
        if types.is_empty() || BUILT_IN_FIELDS.contains(&name.split('.').next().unwrap_or(name)) {
            return;
        }
        let first = name.split('.').next().unwrap_or(name);
        let mut known: BTreeSet<String> = BTreeSet::new();
        for t in types {
            if !self.symbols.is_type(t) {
                return;
            }
            known.extend(self.symbols.attributes_of(t).into_keys());
        }
        if known.contains(first) {
            return;
        }
        let suggestions = closest(first, known.iter().map(String::as_str).collect::<Vec<_>>());
        let owner = if types.len() == 1 {
            format!("element type `{}`", types[0])
        } else {
            format!("element types {}", types.join(", "))
        };
        let mut finding = finding_at(
            "ATD2001",
            at,
            format!("No attribute `{first}` in {owner}.{}", did_you_mean(&suggestions)),
        );
        finding.suggestions = suggestions;
        self.findings.push(finding);
    }

    fn capability_used(&mut self, name: &Json) {
        let name_text = text(name);
        if !self.catalogue.contains(name_text) {
            let suggestions = closest(name_text, self.catalogue.iter().map(String::as_str).collect::<Vec<_>>());
            let mut finding = finding_at(
                "ATD2010",
                name,
                format!(
                    "No capability `{name_text}` in the catalogue.{}",
                    did_you_mean(&suggestions)
                ),
            );
            finding.suggestions = suggestions;
            self.findings.push(finding);
        } else if !self.symbols.required.contains_key(name_text) {
            self.findings.push(finding_at(
                "ATD2010",
                name,
                format!("The capability `{name_text}` is used without `requires capability {name_text}@…`."),
            ));
        }
    }

    fn bool_expression(&mut self, expr: &Json, scope: &Scope, what: &str) {
        let ty = cel::infer(&expr["ast"], scope, self.symbols);
        if !matches!(ty, Ty::Bool | Ty::Dyn) {
            self.findings.push(finding_at(
                "ATD4001",
                expr,
                format!(
                    "{what} is a `bool`; this expression is {} `{}`.",
                    article(&ty.name()),
                    ty.name()
                ),
            ));
        }
    }

    fn self_scope(&self, types: &[String]) -> Scope {
        let me = match types {
            [only] if self.symbols.is_type(only) => Ty::Element(only.clone()),
            _ => Ty::Dyn,
        };
        Scope::default().with("self", me)
    }

    fn member(&mut self, member: &Json) {
        match member["kind"].as_str().unwrap_or("") {
            "requires" => {
                let name = text(&member["capability"]);
                if !self.catalogue.contains(name) {
                    self.capability_used(&member["capability"]);
                }
            }
            "model" => {
                for item in member["members"].as_array().into_iter().flatten() {
                    for extends in item["extends"].as_array().into_iter().flatten() {
                        self.type_ref(extends, false);
                    }
                    for end in ["from", "to"] {
                        for t in item[end].as_array().into_iter().flatten() {
                            self.type_ref(t, false);
                        }
                    }
                    for inner in item["members"].as_array().into_iter().flatten() {
                        match inner["kind"].as_str().unwrap_or("") {
                            "attribute" => self.declared_type(&inner["type"]),
                            "contains" => {
                                for t in inner["types"].as_array().into_iter().flatten() {
                                    self.type_ref(t, false);
                                }
                            }
                            _ => {}
                        }
                    }
                    if item["kind"] == "datatype" && item.get("base").is_some() {
                        self.declared_type(&item["base"]);
                    }
                }
            }
            "constraints" => {
                for item in member["members"].as_array().into_iter().flatten() {
                    self.member(item);
                }
            }
            "constraint" => {
                let types = self.types_of(member, "for", true);
                let scope = self.self_scope(&types);
                for key in ["when", "rule"] {
                    if let Some(expr) = member.get(key) {
                        let what = if key == "rule" {
                            "A constraint's rule"
                        } else {
                            "A constraint's `when`"
                        };
                        self.bool_expression(expr, &scope, what);
                    }
                }
            }
            "operation" => {
                let types = self.types_of(member, "for", true);
                let scope = self.self_scope(&types);
                if let Some(expr) = member.get("enabled") {
                    self.bool_expression(expr, &scope, "An operation's `enabled`");
                }
                for param in member["params"].as_array().into_iter().flatten() {
                    self.declared_type(&param["type"]);
                }
                self.statements(&member["body"]);
            }
            "hook" => {
                self.types_of(member, "for", true);
                self.statements(&member["body"]);
            }
            "fn" => {
                for param in member["params"].as_array().into_iter().flatten() {
                    self.declared_type(&param["type"]);
                }
                if let Some(returns) = member.get("returns") {
                    self.declared_type(returns);
                }
                self.statements(&member["body"]);
            }
            "notation" => {
                for item in member["members"].as_array().into_iter().flatten() {
                    match item["kind"].as_str().unwrap_or("") {
                        "node" | "edge" | "port" => {
                            let types = self.types_of(item, "names", false);
                            self.references(item, &types);
                        }
                        _ => {}
                    }
                }
            }
            "toolbox" => self.generic_types(member),
            "form" => {
                let types = self.types_of(member, "for", true);
                for item in member["members"].as_array().into_iter().flatten() {
                    if item["kind"] == "field"
                        && let Some(name) = item.get("name")
                    {
                        let name_text = text(name).to_owned();
                        self.attribute_of(&types, &name_text, name);
                    }
                }
            }
            "layout" => {
                for item in member["members"].as_array().into_iter().flatten() {
                    if let Some(capability) = item.get("capability") {
                        self.capability_used(capability);
                    }
                }
            }
            "tableSurface" | "hierarchySurface" => {
                for item in member["members"].as_array().into_iter().flatten() {
                    if let Some(type_ref) = item.get("type") {
                        self.type_ref(&type_ref["name"], false);
                    }
                }
            }
            "textSurface" => {
                for item in member["members"].as_array().into_iter().flatten() {
                    if matches!(item["kind"].as_str(), Some("outline" | "fold")) {
                        let types = self.types_of(item, "names", false);
                        if item["kind"] == "outline"
                            && let Some(by) = item.get("name")
                        {
                            let by_text = text(by).to_owned();
                            self.attribute_of(&types, &by_text, by);
                        }
                    }
                    if let Some(capability) = item.get("source").and_then(|s| s.get("capability")) {
                        self.capability_used(capability);
                    }
                    if item["kind"] == "fix" {
                        self.statements(&item["body"]);
                    }
                }
            }
            "binding" => self.binding(member),
            _ => {}
        }
    }

    /// The types a construct names under `key`, each checked.
    fn types_of(&mut self, node: &Json, key: &str, allow_document: bool) -> Vec<String> {
        let mut out = Vec::new();
        for name in node[key].as_array().into_iter().flatten() {
            self.type_ref(name, allow_document);
            out.push(text(name).to_owned());
        }
        out
    }

    fn generic_types(&mut self, node: &Json) {
        for item in node["members"].as_array().into_iter().flatten() {
            if item["kind"] == "context" {
                self.types_of(item, "names", false);
            }
            if item.get("members").is_some() {
                self.generic_types(item);
            }
        }
    }

    /// The attribute references (`@name`) in a notation and its members, against its types.
    fn references(&mut self, node: &Json, types: &[String]) {
        let mut found: Vec<Json> = Vec::new();
        collect_attribute_refs(node, &mut found);
        for reference in found {
            let path = reference["attribute"].as_str().unwrap_or("").to_owned();
            self.attribute_of(types, &path, &reference);
        }
    }

    fn statements(&mut self, body: &Json) {
        for statement in body.as_array().into_iter().flatten() {
            match statement["kind"].as_str().unwrap_or("") {
                "create" | "connect" => self.type_ref(&statement["type"]["name"], false),
                "retype" => {
                    if let Some(to) = statement.get("to").filter(|t| t.get("name").is_some()) {
                        self.type_ref(&to["name"], false);
                    }
                }
                "openForm" => {
                    let form = text(&statement["form"]).to_owned();
                    if !self.symbols.forms.contains_key(&form) {
                        self.unknown_name(&statement["form"], "form", self.symbols.forms.keys().cloned().collect());
                    }
                }
                _ => {}
            }
            for key in ["then", "else", "body"] {
                if statement.get(key).is_some() {
                    self.statements(&statement[key]);
                }
            }
        }
    }

    fn unknown_name(&mut self, name: &Json, what: &str, candidates: Vec<String>) {
        let name_text = text(name);
        let suggestions = closest(name_text, candidates.iter().map(String::as_str).collect::<Vec<_>>());
        let mut finding = finding_at(
            "ATD2011",
            name,
            format!("No {what} `{name_text}`.{}", did_you_mean(&suggestions)),
        );
        finding.suggestions = suggestions;
        self.findings.push(finding);
    }

    fn binding(&mut self, binding: &Json) {
        let name = text(&binding["name"]).to_owned();
        if let Some(capability) = binding.get("capability") {
            self.capability_used(capability);
        }
        let Some(decl) = self.symbols.bindings.get(&name).cloned() else {
            return;
        };
        let mut referable: BTreeSet<String> = decl.rules.keys().cloned().collect();
        referable.extend(decl.blocks.iter().cloned());
        for member in binding["members"].as_array().into_iter().flatten() {
            if member["kind"] != "rule" {
                continue;
            }
            let type_name = text(&member["type"]["name"]).to_owned();
            self.type_ref(&member["type"]["name"], false);
            let attributes = self.symbols.attributes_of(&type_name);
            for item in member["members"].as_array().into_iter().flatten() {
                match item["kind"].as_str().unwrap_or("") {
                    "attribute" if self.symbols.is_type(&type_name) => {
                        let attribute = text(&item["name"]);
                        if !attributes.contains_key(attribute) {
                            self.findings.push(finding_at(
                                "ATD6001",
                                &item["name"],
                                format!("The binding reads `{attribute}`, which element type `{type_name}` does not declare."),
                            ));
                        }
                        if let Some(references) = item.get("references") {
                            for rule in references["rules"].as_array().into_iter().flatten() {
                                self.rule_name(rule, &decl.rules.keys().cloned().collect(), &name);
                            }
                        }
                    }
                    "parent" => {
                        for rule in item["rules"].as_array().into_iter().flatten() {
                            self.rule_name(rule, &decl.rules.keys().cloned().collect(), &name);
                        }
                    }
                    "within" => {
                        for rule in item["rules"].as_array().into_iter().flatten() {
                            self.rule_name(rule, &referable, &name);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn rule_name(&mut self, rule: &Json, rules: &BTreeSet<String>, binding: &str) {
        let rule_text = text(rule);
        if rules.contains(rule_text) {
            return;
        }
        let suggestions = closest(rule_text, rules.iter().map(String::as_str).collect::<Vec<_>>());
        let mut finding = finding_at(
            "ATD2004",
            rule,
            format!(
                "No rule `{rule_text}` in binding `{binding}`.{}",
                did_you_mean(&suggestions)
            ),
        );
        finding.suggestions = suggestions;
        self.findings.push(finding);
    }

    /// Every global call of every expression: a function of the library or of the description (ATD §14.8).
    fn calls(&mut self, description: &Description) {
        let mut calls = Vec::new();
        for member in description.members() {
            cel::global_calls(member, &mut calls);
        }
        let mut known: Vec<String> = cel::LIBRARY.iter().map(|s| (*s).to_owned()).collect();
        known.extend(self.symbols.functions.keys().cloned());
        for call in calls {
            let name = call["call"].as_str().unwrap_or("");
            if known.iter().any(|k| k == name) {
                continue;
            }
            if cel::FORBIDDEN.contains(&name) {
                self.findings.push(finding_at(
                    "ATD7001",
                    call,
                    format!(
                        "`{name}` reads what a script may not (a clock, randomness, a file, the network or the host); a script is deterministic and calls only CEL's library and the description's functions."
                    ),
                ));
                continue;
            }
            let functions: Vec<&str> = self.symbols.functions.keys().map(String::as_str).collect();
            let suggestions = closest(name, functions);
            let mut finding = finding_at(
                "ATD2005",
                call,
                format!("No function `{name}`.{}", did_you_mean(&suggestions)),
            );
            finding.suggestions = suggestions;
            self.findings.push(finding);
        }
    }
}

fn article(type_name: &str) -> &'static str {
    if type_name.starts_with(['a', 'e', 'i', 'o', 'u']) {
        "an"
    } else {
        "a"
    }
}

fn collect_attribute_refs(node: &Json, out: &mut Vec<Json>) {
    match node {
        Json::Object(map) => {
            if map.contains_key("attribute") && map.contains_key("span") && map.len() == 2 {
                out.push(node.clone());
                return;
            }
            for (key, value) in map {
                if key != "span" {
                    collect_attribute_refs(value, out);
                }
            }
        }
        Json::Array(items) => {
            for item in items {
                collect_attribute_refs(item, out);
            }
        }
        _ => {}
    }
}
