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

/// What a context tool set opens on besides element types (DISL §7.3): the empty canvas and a pending connection.
const CONTEXT_TARGETS: &[&str] = &["diagram", "connection"];

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
        kind: kind.to_owned(),
        type_map: type_map(description),
    };
    for member in description.members() {
        checker.member(member);
    }
    checker.persistence(description);
    checker.calls(description);
    for member in description.members() {
        checker.scoped(member, &[]);
    }
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
    /// The description's kind: `diagram`, `designer` or `editor`.
    kind: String,
    /// `persistence`'s `typeMap`: each binding type it names, with what it maps it as and its attribute renames.
    type_map: BTreeMap<String, TypeMapEntry>,
}

/// One entry of `persistence`'s `typeMap` (DISL §11.2, DESL §5.2).
struct TypeMapEntry {
    /// `document`, `header`, `unreadable`, or the name of a type.
    as_: String,
    /// The binding's attribute names, each with the model's name it stands for: an attribute, `source`, `target`,
    /// `id`, or `null` for one the model does not have.
    attributes: BTreeMap<String, String>,
    /// The attributes the binding reads and writes that are not the model's (DISL §11.2's `hostAttributes`).
    host_attributes: BTreeSet<String>,
}

/// The entry of a list of settings (`[{key, value, span}]`) under `key`.
fn entry<'a>(settings: &'a Json, key: &str) -> Option<&'a Json> {
    settings.as_array().into_iter().flatten().find(|s| s["key"] == key)
}

/// The settings of a settings map's value, or an empty list.
fn map_of(value: &Json) -> &Json {
    static EMPTY: Json = Json::Array(Vec::new());
    value.get("settings").unwrap_or(&EMPTY)
}

/// A bare word's or a string's text.
fn word(value: &Json) -> Option<&str> {
    value.get("literal").and_then(Json::as_str)
}

/// The settings of the description's `persistence` constructs.
fn persistence_settings(description: &Description) -> Vec<&Json> {
    description
        .members()
        .filter(|m| m["kind"] == "persistence")
        .flat_map(|m| m["settings"].as_array().into_iter().flatten())
        .collect()
}

fn type_map(description: &Description) -> BTreeMap<String, TypeMapEntry> {
    let mut out = BTreeMap::new();
    // The binding types `view.bind` stores view data as (DISL §11.6): the binding's, not the model's.
    for setting in persistence_settings(description) {
        if setting["key"] != "view" {
            continue;
        }
        let Some(bind) = entry(map_of(&setting["value"]), "bind") else {
            continue;
        };
        for item in map_of(&bind["value"]).as_array().into_iter().flatten() {
            if let Some(type_name) = entry(map_of(&item["value"]), "type").and_then(|e| word(&e["value"])) {
                out.insert(
                    type_name.to_owned(),
                    TypeMapEntry {
                        as_: "view".to_owned(),
                        attributes: BTreeMap::new(),
                        host_attributes: BTreeSet::new(),
                    },
                );
            }
        }
    }
    for setting in persistence_settings(description) {
        if setting["key"] != "typeMap" {
            continue;
        }
        for item in map_of(&setting["value"]).as_array().into_iter().flatten() {
            let settings = map_of(&item["value"]);
            let Some(as_) = entry(settings, "as").and_then(|e| word(&e["value"])) else {
                continue;
            };
            let attributes = entry(settings, "attributes")
                .map(|e| {
                    map_of(&e["value"])
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|a| {
                            let renamed = if a["value"]["literal"].is_null() {
                                "null"
                            } else {
                                word(&a["value"])?
                            };
                            Some((a["key"].as_str()?.to_owned(), renamed.to_owned()))
                        })
                        .collect()
                })
                .unwrap_or_default();
            let host_attributes = entry(settings, "hostAttributes")
                .and_then(|e| e["value"].get("list"))
                .and_then(Json::as_array)
                .into_iter()
                .flatten()
                .filter_map(|a| word(a).map(str::to_owned))
                .collect();
            out.insert(
                item["key"].as_str().unwrap_or("").to_owned(),
                TypeMapEntry {
                    as_: as_.to_owned(),
                    attributes,
                    host_attributes,
                },
            );
        }
    }
    out
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
                        "edge" => {
                            let types = self.edge_types(item);
                            self.references(item, &types);
                        }
                        "node" | "port" => {
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
                let params = self.params_shown_by(text(&member["name"]));
                for item in member["members"].as_array().into_iter().flatten() {
                    if item["kind"] == "field"
                        && let Some(name) = item.get("name")
                    {
                        let name_text = text(name).to_owned();
                        if !params.contains(&name_text) {
                            self.attribute_of(&types, &name_text, name);
                        }
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
                if member["kind"] == "hierarchySurface" {
                    self.hierarchy(member);
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

    /// The parameters of the operations whose `paramsForm` is the form `form`: a field of such a form edits a
    /// parameter of that name, not an attribute (DISL §9.2).
    fn params_shown_by(&self, form: &str) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        for operation in self.symbols.operations.values() {
            let shown = operation["settings"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|setting| setting["key"] == "paramsForm" && setting["value"]["literal"] == form);
            if shown {
                for param in operation["params"].as_array().into_iter().flatten() {
                    out.insert(text(&param["name"]).to_owned());
                }
            }
        }
        out
    }

    /// The types an edge notation draws: relation types, and `<Type>.<attribute>`, the edge DISL §6.10 draws for a
    /// reference attribute, whose attribute must be one of the type's and name an element type. An attribute reference
    /// in such an edge reads the type that holds the attribute.
    fn edge_types(&mut self, node: &Json) -> Vec<String> {
        let mut out = Vec::new();
        for name in node["names"].as_array().into_iter().flatten() {
            let name_text = text(name);
            match name_text.split_once('.') {
                Some((owner, attribute)) if self.symbols.is_type(owner) => {
                    let target = self
                        .symbols
                        .attributes_of(owner)
                        .get(attribute)
                        .map(|decl| text(&decl["type"]["name"]).to_owned());
                    if !target.as_deref().is_some_and(|t| self.symbols.is_type(t)) {
                        self.findings.push(finding_at(
                            "ATD2002",
                            name,
                            format!("`{owner}` has no reference attribute `{attribute}` for an edge to draw."),
                        ));
                    }
                    out.push(owner.to_owned());
                }
                _ => {
                    self.type_ref(name, false);
                    out.push(name_text.to_owned());
                }
            }
        }
        out
    }

    /// The types a context tool set opens on: element types, and DISL's reserved `diagram`, the empty canvas, and
    /// `connection`, a pending connection (DISL §7.3, ATD §8.6).
    fn context_types(&mut self, node: &Json) {
        for name in node["names"].as_array().into_iter().flatten() {
            if !CONTEXT_TARGETS.contains(&text(name)) {
                self.type_ref(name, false);
            }
        }
    }

    fn generic_types(&mut self, node: &Json) {
        for item in node["members"].as_array().into_iter().flatten() {
            if item["kind"] == "context" {
                self.context_types(item);
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
                "move" if statement.get("x").is_some() && self.kind != "diagram" => {
                    self.findings.push(finding_at(
                        "ATD3002",
                        statement,
                        format!(
                            "A {} has no canvas; `move … to x, y` places a node on the canvas of a diagram.",
                            self.kind
                        ),
                    ));
                }
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

    /// The types a construct is written for: what `self` is inside it.
    fn self_types(construct: &Json) -> Option<Vec<String>> {
        let kind = construct["kind"].as_str().unwrap_or("");
        let names = match kind {
            "constraint" | "operation" | "hook" | "context" | "form" => construct.get("for"),
            "node" | "edge" if construct.get("names").is_some() => construct.get("names"),
            _ => None,
        }?;
        Some(
            names
                .as_array()
                .into_iter()
                .flatten()
                .map(|n| text(n).to_owned())
                .collect(),
        )
    }

    /// Every expression of a construct written for one type, and what it holds: a field of `self` the type does not
    /// have (`ATD2001`); and in a node's or an edge's notation, a `style` that names no style (`ATD2011`).
    fn scoped(&mut self, node: &Json, types: &[String]) {
        match node {
            Json::Object(map) => {
                let own = Self::self_types(node);
                let types: Vec<String> = own.unwrap_or_else(|| types.to_vec());
                if node.get("ast").is_some() && !types.is_empty() {
                    let mut found = Vec::new();
                    cel::unknown_members(&node["ast"], &self.self_scope(&types), self.symbols, &mut found);
                    for (at, field, type_name) in found {
                        let known: Vec<String> = self.symbols.attributes_of(&type_name).into_keys().collect();
                        let suggestions = closest(&field, known.iter().map(String::as_str).collect::<Vec<_>>());
                        let mut finding = finding_at(
                            "ATD2001",
                            &at,
                            format!(
                                "No attribute `{field}` in element type `{type_name}`.{}",
                                did_you_mean(&suggestions)
                            ),
                        );
                        finding.suggestions = suggestions;
                        self.findings.push(finding);
                    }
                    return;
                }
                if matches!(node["kind"].as_str(), Some("node" | "edge")) && node.get("names").is_some() {
                    for member in node["members"].as_array().into_iter().flatten() {
                        let setting = &member["setting"];
                        if member["kind"] == "setting"
                            && setting["key"] == "style"
                            && setting["value"]["form"] == "word"
                            && !self
                                .symbols
                                .styles
                                .contains(setting["value"]["literal"].as_str().unwrap_or(""))
                        {
                            let written = setting["value"]["literal"].as_str().unwrap_or("");
                            let suggestions = closest(
                                written,
                                self.symbols.styles.iter().map(String::as_str).collect::<Vec<_>>(),
                            );
                            let mut finding = finding_at(
                                "ATD2011",
                                &setting["value"],
                                format!("No style `{written}`.{}", did_you_mean(&suggestions)),
                            );
                            finding.suggestions = suggestions;
                            self.findings.push(finding);
                        }
                    }
                }
                for (key, value) in map {
                    if key != "span" {
                        self.scoped(value, &types);
                    }
                }
            }
            Json::Array(items) => items.iter().for_each(|item| self.scoped(item, types)),
            _ => {}
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
            let rule = member
                .get("as")
                .map(|a| text(a).to_owned())
                .unwrap_or_else(|| type_name.clone());
            // What the rule's type gives it to read and write (ATD §14.7): a model type its attributes; a key of the
            // type map the document's, its type's, or none, through the entry's renames.
            // A key of the type map maps its type even where a model type has its name (DISL §11.2).
            let (attributes, renames, hosted, mapped_as) = if let Some(mapped) = self.type_map.get(&type_name) {
                let attributes = match mapped.as_.as_str() {
                    "document" => Some(self.symbols.document.clone()),
                    // A header and view data are not in the model, so what their rules read is the binding's alone (a
                    // version line, a pinned position).
                    "header" | "view" => None,
                    "unreadable" => Some(BTreeMap::new()),
                    other if self.symbols.is_type(other) => Some(self.symbols.attributes_of(other)),
                    _ => None,
                };
                (
                    attributes,
                    mapped.attributes.clone(),
                    mapped.host_attributes.clone(),
                    Some(mapped.as_.clone()),
                )
            } else if self.symbols.is_type(&type_name) {
                (
                    Some(self.symbols.attributes_of(&type_name)),
                    BTreeMap::new(),
                    BTreeSet::new(),
                    None,
                )
            } else {
                self.type_ref(&member["type"]["name"], false);
                (None, BTreeMap::new(), BTreeSet::new(), None)
            };
            for item in member["members"].as_array().into_iter().flatten() {
                match item["kind"].as_str().unwrap_or("") {
                    "attribute" if attributes.is_some() => {
                        let attribute = text(&item["name"]);
                        let model_name = renames.get(attribute).map(String::as_str).unwrap_or(attribute);
                        // An end, the id and `null` name no attribute of the model; a host attribute is the binding's.
                        let unmodelled = matches!(model_name, "source" | "target" | "id" | "null")
                            && renames.contains_key(attribute);
                        if !unmodelled
                            && !hosted.contains(attribute)
                            && !attributes.as_ref().is_some_and(|a| a.contains_key(model_name))
                        {
                            let message = match mapped_as.as_deref() {
                                Some(as_ @ ("header" | "unreadable")) => format!(
                                    "`{type_name}` is mapped as {as_}, so its rule `{rule}` binds no attribute; it binds `{attribute}`."
                                ),
                                Some("document") => format!(
                                    "The binding reads `{attribute}`, which the document does not declare (`{type_name}` is mapped as the document)."
                                ),
                                _ => format!(
                                    "The binding reads `{attribute}`, which element type `{type_name}` does not declare."
                                ),
                            };
                            self.findings.push(finding_at("ATD6001", &item["name"], message));
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

    /// A hierarchy surface fits its types, as DESL \§10.1 requires of one (`ATD6002`, ATD \§14.7).
    fn hierarchy(&mut self, surface: &Json) {
        let members: Vec<&Json> = surface["members"].as_array().into_iter().flatten().collect();
        let role = |kind: &str| {
            members
                .iter()
                .find(|m| m["kind"] == kind)
                .map(|m| (text(&m["type"]["name"]).to_owned(), *m))
                .filter(|(t, _)| self.symbols.is_type(t))
        };
        let levels = role("levels");
        let entries = role("entries");
        if let (Some((level, at)), Some((entry_type, _))) = (&levels, &entries) {
            let contained = self.symbols.contained_by(level);
            let missing: Vec<&str> = [level.as_str(), entry_type.as_str()]
                .into_iter()
                .filter(|t| !contained.iter().any(|c| c == t))
                .collect();
            if !missing.is_empty() {
                self.findings.push(finding_at(
                    "ATD6002",
                    at,
                    format!(
                        "The levels' type `{level}` contains itself and the entries' type; it does not contain `{}`.",
                        missing.join("` or `")
                    ),
                ));
            }
        }
        let both: Vec<String> = [&levels, &entries]
            .into_iter()
            .flatten()
            .map(|(t, _)| t.clone())
            .collect();
        for member in &members {
            match member["kind"].as_str().unwrap_or("") {
                "labels" => {
                    let attribute = text(&member["name"]).to_owned();
                    let on = entry(&member["settings"], "on").map(|e| &e["value"]);
                    let types: Vec<String> = on
                        .and_then(|v| v.get("list"))
                        .and_then(Json::as_array)
                        .map(|l| l.iter().filter_map(word).map(str::to_owned).collect())
                        .or_else(|| on.and_then(word).map(|w| vec![w.to_owned()]))
                        .unwrap_or_default();
                    if let Some(lacking) = types
                        .iter()
                        .find(|t| self.symbols.is_type(t) && !self.symbols.attributes_of(t).contains_key(&attribute))
                    {
                        self.findings.push(finding_at(
                            "ATD6002",
                            member,
                            format!("The label attribute `{attribute}` is not an attribute of `{lacking}`, which its `on` names."),
                        ));
                    }
                }
                "setting" if member["setting"]["key"] == "collapsed" => {
                    let Some(attribute) = word(&member["setting"]["value"]) else {
                        continue;
                    };
                    if let Some(lacking) = both
                        .iter()
                        .find(|t| !self.symbols.attributes_of(t).contains_key(attribute))
                    {
                        self.findings.push(finding_at(
                            "ATD6002",
                            member,
                            format!("`collapsed` names `{attribute}`, which is not an attribute of `{lacking}`."),
                        ));
                    }
                }
                "widths" => {
                    let widths = text(&member["type"]["name"]).to_owned();
                    if !self.symbols.is_type(&widths) {
                        continue;
                    }
                    let attributes = self.symbols.attributes_of(&widths);
                    for (part, wanted) in [("level", "string"), ("width", "int")] {
                        let Some(setting) = entry(&member["settings"], part) else {
                            continue;
                        };
                        let Some(name) = word(&setting["value"]) else {
                            continue;
                        };
                        match attributes.get(name) {
                            None => self.findings.push(finding_at(
                                "ATD2001",
                                setting,
                                format!("No attribute `{name}` in element type `{widths}`."),
                            )),
                            Some(attribute) if text(&attribute["type"]["name"]) != wanted => {
                                self.findings.push(finding_at(
                                    "ATD6002",
                                    member,
                                    format!(
                                        "The widths' `{part}` is a `{wanted}` attribute; `{name}` of `{widths}` is not."
                                    ),
                                ));
                            }
                            Some(_) => {}
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// A counter's mark is a `string` attribute of the document (`ATD6003`), and each setting a registration keeps is
    /// a block every binding declares in its registration (`ATD6004`) (ATD \§14.7).
    fn persistence(&mut self, description: &Description) {
        let blocks: Vec<(String, BTreeSet<String>)> = self
            .symbols
            .bindings
            .iter()
            .map(|(name, decl)| {
                let declared = decl.node["members"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|m| m["kind"] == "registration")
                    .filter_map(|m| entry(&m["settings"], "blocks"))
                    .flat_map(|b| map_of(&b["value"]).as_array().into_iter().flatten())
                    .filter_map(|b| b["key"].as_str().map(str::to_owned))
                    .collect();
                (name.clone(), declared)
            })
            .collect();
        for setting in persistence_settings(description) {
            match setting["key"].as_str().unwrap_or("") {
                "ids" => {
                    let Some(types) = entry(map_of(&setting["value"]), "types") else {
                        continue;
                    };
                    for rule in map_of(&types["value"]).as_array().into_iter().flatten() {
                        let settings = map_of(&rule["value"]);
                        if entry(settings, "strategy").and_then(|s| word(&s["value"])) != Some("counter") {
                            continue;
                        }
                        let Some(mark) = entry(settings, "mark") else {
                            continue;
                        };
                        let Some(name) = word(&mark["value"]) else {
                            continue;
                        };
                        let owner = rule["key"].as_str().unwrap_or("");
                        let message = match self.symbols.document.get(name) {
                            Some(attribute) if text(&attribute["type"]["name"]) == "string" => continue,
                            Some(attribute) => format!(
                                "The counter of `{owner}` keeps its mark in `{name}`, which is an `{}`; a mark is a `string` attribute of the document.",
                                text(&attribute["type"]["name"])
                            ),
                            None => format!(
                                "The counter of `{owner}` keeps its mark in `{name}`, which is no attribute of the document; a mark is a `string` attribute of the document."
                            ),
                        };
                        self.findings.push(finding_at("ATD6003", mark, message));
                    }
                }
                "settings" => {
                    for kept in map_of(&setting["value"]).as_array().into_iter().flatten() {
                        let key = kept["key"].as_str().unwrap_or("");
                        if let Some((binding, _)) = blocks.iter().find(|(_, declared)| !declared.contains(key)) {
                            self.findings.push(finding_at(
                                "ATD6004",
                                kept,
                                format!("The registration of binding `{binding}` keeps no block `{key}`, which `persistence` names among its settings."),
                            ));
                        }
                    }
                }
                _ => {}
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
