//! Where each name of a description is declared and used (contracts/language-server.md): the occurrences behind
//! definition, references, rename, highlights, hover and semantic tokens. A name inside a string is not a use.

use std::collections::BTreeMap;

use adp_atd_check::{Checked, Symbols};
use serde_json::Value as Json;

/// One occurrence of a name: its file, its byte range, what it names, and whether it declares it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Occurrence {
    pub file: String,
    pub start: usize,
    pub end: usize,
    /// `type:Node`, `attr:Node.text`, `fn:siblingName`, `op:addChild`, `rule:freeplane.node`, `local:<fn>:<name>`, …
    pub symbol: String,
    pub declaration: bool,
}

/// What a declared name is, for hover.
#[derive(Debug, Clone, Default)]
pub struct Declaration {
    pub detail: String,
    pub doc: Option<String>,
}

/// The occurrences of every name of a description.
#[derive(Debug, Clone, Default)]
pub struct Index {
    pub occurrences: Vec<Occurrence>,
    pub declarations: BTreeMap<String, Declaration>,
}

impl Index {
    /// The occurrence at a byte offset of a file.
    pub fn at(&self, file: &str, offset: usize) -> Option<&Occurrence> {
        self.occurrences
            .iter()
            .find(|o| o.file == file && o.start <= offset && offset <= o.end)
    }

    pub fn of(&self, symbol: &str) -> impl Iterator<Item = &Occurrence> {
        self.occurrences.iter().filter(move |o| o.symbol == symbol)
    }

    pub fn declaration(&self, symbol: &str) -> Option<&Occurrence> {
        self.of(symbol).find(|o| o.declaration)
    }
}

fn text(name: &Json) -> &str {
    name["text"].as_str().unwrap_or("")
}

fn span(node: &Json) -> (String, usize, usize) {
    let s = &node["span"];
    (
        s["file"].as_str().unwrap_or("").to_owned(),
        s["start"].as_u64().unwrap_or(0) as usize,
        s["end"].as_u64().unwrap_or(0) as usize,
    )
}

struct Builder<'a> {
    symbols: &'a Symbols,
    index: Index,
    /// The types `self` is, where an expression stands.
    me: Vec<String>,
    /// The locals in scope, by name, with the function they belong to and their type.
    locals: Vec<(String, String, Option<String>)>,
    function: String,
}

/// The index of a checked description.
pub fn build(checked: &Checked) -> Index {
    let Some(symbols) = &checked.symbols else {
        return Index::default();
    };
    let mut builder = Builder {
        symbols,
        index: Index::default(),
        me: Vec::new(),
        locals: Vec::new(),
        function: String::new(),
    };
    for member in checked.description.members() {
        builder.member(member);
    }
    builder.index
}

impl Builder<'_> {
    fn add(&mut self, node: &Json, symbol: String, declaration: bool) {
        let (file, start, end) = span(node);
        if file.is_empty() {
            return;
        }
        self.index.occurrences.push(Occurrence {
            file,
            start,
            end,
            symbol,
            declaration,
        });
    }

    fn add_range(&mut self, file: &str, start: usize, end: usize, symbol: String) {
        self.index.occurrences.push(Occurrence {
            file: file.to_owned(),
            start,
            end,
            symbol,
            declaration: false,
        });
    }

    fn declare(&mut self, name: &Json, symbol: String, detail: String, doc: Option<&Json>) {
        self.add(name, symbol.clone(), true);
        self.index.declarations.insert(
            symbol,
            Declaration {
                detail,
                doc: doc.and_then(Json::as_str).map(str::to_owned),
            },
        );
    }

    /// The symbol a type name stands for: an element or relation type, an enum or a data type.
    fn type_symbol(&self, name: &str) -> Option<String> {
        (self.symbols.is_type(name)
            || self.symbols.enums.contains_key(name)
            || self.symbols.datatypes.contains_key(name))
        .then(|| format!("type:{name}"))
    }

    fn type_ref(&mut self, name: &Json) {
        if let Some(symbol) = self.type_symbol(text(name)) {
            self.add(name, symbol, false);
        }
    }

    fn type_refs(&mut self, list: &Json) {
        for name in list.as_array().into_iter().flatten() {
            self.type_ref(name);
        }
    }

    fn declared_type(&mut self, type_ref: &Json) {
        self.type_ref(&type_ref["name"]);
        for argument in type_ref["arguments"].as_array().into_iter().flatten() {
            self.declared_type(argument);
        }
    }

    /// The type that declares attribute `name` of `owner`, or of one of its supertypes.
    fn attribute_symbol(&self, owner: &str, name: &str) -> Option<String> {
        let mut todo = vec![owner.to_owned()];
        let mut seen = Vec::new();
        while let Some(current) = todo.pop() {
            if seen.contains(&current) {
                continue;
            }
            seen.push(current.clone());
            if let Some(decl) = self.symbols.types.get(&current) {
                if decl.attributes.contains_key(name) {
                    return Some(format!("attr:{current}.{name}"));
                }
                todo.extend(decl.extends.iter().cloned());
            }
        }
        if self.symbols.document.contains_key(name) && (owner == "document" || owner == "diagram") {
            return Some(format!("attr:document.{name}"));
        }
        None
    }

    fn attribute_ref(&mut self, owners: &[String], name: &str, file: &str, start: usize, end: usize) {
        for owner in owners {
            if let Some(symbol) = self.attribute_symbol(owner, name) {
                self.add_range(file, start, end, symbol);
                return;
            }
        }
    }

    fn names_of(list: &Json) -> Vec<String> {
        list.as_array()
            .into_iter()
            .flatten()
            .map(|n| text(n).to_owned())
            .collect()
    }

    fn member(&mut self, member: &Json) {
        let doc = member.get("doc");
        match member["kind"].as_str().unwrap_or("") {
            "model" => {
                for item in member["members"].as_array().into_iter().flatten() {
                    self.model_item(item);
                }
            }
            "constraints" => {
                for item in member["members"].as_array().into_iter().flatten() {
                    self.member(item);
                }
            }
            "constraint" => {
                self.me = Self::names_of(&member["for"]);
                self.type_refs(&member["for"]);
                for key in ["when", "rule", "message", "label"] {
                    if let Some(value) = member.get(key) {
                        self.value(value);
                    }
                }
                for fix in member["fixes"].as_array().into_iter().flatten() {
                    self.statements(&fix["body"]);
                }
                self.me.clear();
            }
            "operation" => {
                let name = text(&member["name"]).to_owned();
                let title = member["title"].as_str().map(|t| format!(" — {t}")).unwrap_or_default();
                self.declare(
                    &member["name"],
                    format!("op:{name}"),
                    format!("operation {name}{title}"),
                    doc,
                );
                self.me = Self::names_of(&member["for"]);
                self.type_refs(&member["for"]);
                self.function = format!("op:{name}");
                for key in ["enabled", "unavailable", "label"] {
                    if let Some(value) = member.get(key) {
                        self.value(value);
                    }
                }
                for param in member["params"].as_array().into_iter().flatten() {
                    self.declared_type(&param["type"]);
                }
                self.settings(&member["settings"]);
                self.statements(&member["body"]);
                self.me.clear();
                self.locals.clear();
            }
            "hook" => {
                self.me = Self::names_of(&member["for"]);
                self.type_refs(&member["for"]);
                self.function = format!("hook:{}", text(&member["name"]));
                if let Some(when) = member.get("when") {
                    self.value(when);
                }
                self.statements(&member["body"]);
                self.me.clear();
                self.locals.clear();
            }
            "fn" => {
                let name = text(&member["name"]).to_owned();
                self.function = format!("fn:{name}");
                let mut signature = Vec::new();
                for param in member["params"].as_array().into_iter().flatten() {
                    let param_name = text(&param["name"]).to_owned();
                    let param_type = text(&param["type"]["name"]).to_owned();
                    signature.push(format!("{param_name}: {param_type}"));
                    self.declare(
                        &param["name"],
                        format!("local:{}:{param_name}", self.function),
                        format!("parameter {param_name}: {param_type}"),
                        None,
                    );
                    self.locals.push((param_name, self.function.clone(), Some(param_type)));
                    self.declared_type(&param["type"]);
                }
                let returns = member
                    .get("returns")
                    .map(|r| format!(" -> {}", text(&r["name"])))
                    .unwrap_or_default();
                self.declare(
                    &member["name"],
                    format!("fn:{name}"),
                    format!("fn {name}({}){returns}", signature.join(", ")),
                    doc,
                );
                if let Some(returns) = member.get("returns") {
                    self.declared_type(returns);
                }
                self.statements(&member["body"]);
                self.locals.clear();
            }
            "reason" => {
                let name = text(&member["name"]).to_owned();
                self.declare(&member["name"], format!("reason:{name}"), format!("reason {name}"), doc);
            }
            "form" => {
                let name = text(&member["name"]).to_owned();
                self.declare(&member["name"], format!("form:{name}"), format!("form {name}"), doc);
                let owners = Self::names_of(&member["for"]);
                self.type_refs(&member["for"]);
                for item in member["members"].as_array().into_iter().flatten() {
                    if item["kind"] == "field"
                        && let Some(field) = item.get("name")
                    {
                        let (file, start, end) = span(field);
                        self.attribute_ref(&owners, text(field), &file, start, end);
                    }
                }
            }
            "requires" => {
                let name = text(&member["capability"]).to_owned();
                self.declare(
                    &member["capability"],
                    format!("capability:{name}"),
                    format!("capability {name}"),
                    doc,
                );
            }
            "notation" | "toolbox" | "layout" | "coordinates" | "tableSurface" | "hierarchySurface" | "textSurface" => {
                self.generic(member)
            }
            "binding" => self.binding(member),
            "setting" => self.value(&member["setting"]["value"]),
            _ => {}
        }
    }

    fn model_item(&mut self, item: &Json) {
        let doc = item.get("doc");
        match item["kind"].as_str().unwrap_or("") {
            "element" | "relation" => {
                let name = text(&item["name"]).to_owned();
                let kind = if item["kind"] == "relation" {
                    "relation type"
                } else {
                    "element type"
                };
                self.declare(&item["name"], format!("type:{name}"), format!("{kind} {name}"), doc);
                self.type_refs(&item["extends"]);
                self.type_refs(&item["from"]);
                self.type_refs(&item["to"]);
                for member in item["members"].as_array().into_iter().flatten() {
                    match member["kind"].as_str().unwrap_or("") {
                        "attribute" => {
                            let attribute = text(&member["name"]).to_owned();
                            let type_name = text(&member["type"]["name"]).to_owned();
                            self.declare(
                                &member["name"],
                                format!("attr:{name}.{attribute}"),
                                format!("attribute {name}.{attribute}: {type_name}"),
                                member.get("doc"),
                            );
                            self.declared_type(&member["type"]);
                        }
                        "contains" => self.type_refs(&member["types"]),
                        "setting" => {
                            let setting = &member["setting"];
                            if setting["key"] == "labelAttribute"
                                && let Some(literal) = setting["value"]["literal"].as_str()
                            {
                                let (file, start, end) = span(&setting["value"]);
                                self.attribute_ref(std::slice::from_ref(&name), literal, &file, start, end);
                            }
                        }
                        _ => {}
                    }
                }
            }
            "enum" | "datatype" => {
                let name = text(&item["name"]).to_owned();
                self.declare(
                    &item["name"],
                    format!("type:{name}"),
                    format!("{} {name}", item["kind"].as_str().unwrap_or("")),
                    doc,
                );
            }
            "document" => {
                for member in item["members"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|m| m["kind"] == "attribute")
                {
                    let attribute = text(&member["name"]).to_owned();
                    self.declare(
                        &member["name"],
                        format!("attr:document.{attribute}"),
                        format!("attribute of the document {attribute}"),
                        member.get("doc"),
                    );
                }
            }
            _ => {}
        }
    }

    /// A surface's members: types, attribute references, styles, operations named by settings, and capabilities.
    fn generic(&mut self, node: &Json) {
        for item in node["members"].as_array().into_iter().flatten() {
            let kind = item["kind"].as_str().unwrap_or("");
            let saved = self.me.clone();
            match kind {
                "node" | "edge" | "port" | "context" | "outline" | "fold" => {
                    self.type_refs(&item["names"]);
                    self.me = Self::names_of(&item["names"]);
                    if kind == "outline"
                        && let Some(by) = item.get("name")
                    {
                        let (file, start, end) = span(by);
                        let owners = self.me.clone();
                        self.attribute_ref(&owners, text(by), &file, start, end);
                    }
                }
                "style" => {
                    let name = text(&item["name"]).to_owned();
                    self.declare(
                        &item["name"],
                        format!("style:{name}"),
                        format!("style {name}"),
                        item.get("doc"),
                    );
                }
                "algorithm" => {
                    let name = text(&item["name"]).to_owned();
                    self.declare(
                        &item["name"],
                        format!("algorithm:{name}"),
                        format!("layout algorithm {name}"),
                        item.get("doc"),
                    );
                }
                "rows" | "columns" | "cells" | "views" | "levels" | "entries" | "notes" | "filters" => {
                    if let Some(type_ref) = item.get("type") {
                        self.type_ref(&type_ref["name"]);
                    }
                }
                "fix" => self.statements(&item["body"]),
                _ => {}
            }
            if let Some(capability) = item
                .get("capability")
                .or_else(|| item.get("source").and_then(|s| s.get("capability")))
            {
                let name = text(capability).to_owned();
                self.add(capability, format!("capability:{name}"), false);
            }
            for key in ["value", "when"] {
                if let Some(value) = item.get(key) {
                    self.value(value);
                }
            }
            self.settings(&item["settings"]);
            if item.get("setting").is_some() {
                self.setting(&item["setting"]);
            }
            if item.get("members").is_some() {
                self.generic(item);
            }
            self.me = saved;
        }
    }

    fn settings(&mut self, list: &Json) {
        for setting in list.as_array().into_iter().flatten() {
            self.setting(setting);
        }
    }

    /// A setting: its value's expressions and references, and the names some keys take (`operation`, `style`).
    fn setting(&mut self, setting: &Json) {
        let key = setting["key"].as_str().unwrap_or("");
        let value = &setting["value"];
        if let Some(literal) = value["literal"].as_str() {
            let symbol = match key {
                "operation" if self.symbols.operations.contains_key(literal) => Some(format!("op:{literal}")),
                "style" if self.symbols.styles.contains(literal) => Some(format!("style:{literal}")),
                _ => None,
            };
            if let Some(symbol) = symbol {
                self.add(value, symbol, false);
            }
        }
        self.value(value);
    }

    fn value(&mut self, value: &Json) {
        if let Some(attribute) = value.get("attribute").and_then(Json::as_str) {
            let (file, start, end) = span(value);
            let first = attribute.split('.').next().unwrap_or(attribute);
            let owners = self.me.clone();
            self.attribute_ref(&owners, first, &file, start + 1, (start + 1 + first.len()).min(end));
            return;
        }
        if let Some(ast) = value.get("ast") {
            self.cel(ast);
            return;
        }
        for entry in value["settings"].as_array().into_iter().flatten() {
            self.setting(entry);
        }
        for item in value["list"].as_array().into_iter().flatten() {
            self.value(item);
        }
    }

    fn local(&self, name: &str) -> Option<&(String, String, Option<String>)> {
        self.locals
            .iter()
            .rev()
            .find(|(n, f, _)| n == name && *f == self.function)
    }

    /// The element type an expression is, where it can be told: `self`, a typed local, a field of either.
    fn element_type(&self, node: &Json) -> Option<String> {
        if let Some(ident) = node.get("ident").and_then(Json::as_str) {
            if ident == "self" {
                return match self.me.as_slice() {
                    [only] => Some(only.clone()),
                    _ => None,
                };
            }
            return self
                .local(ident)
                .and_then(|(_, _, t)| t.clone())
                .filter(|t| self.symbols.is_type(t));
        }
        if let Some(field) = node.get("select").and_then(Json::as_str) {
            let owner = self.element_type(&node["operand"])?;
            if field == "parent" {
                return self.symbols.container_of(&owner);
            }
            let attribute = self.symbols.attributes_of(&owner).get(field)?.clone();
            let type_name = text(&attribute["type"]["name"]).to_owned();
            return self.symbols.is_type(&type_name).then_some(type_name);
        }
        None
    }

    fn cel(&mut self, node: &Json) {
        match node {
            Json::Object(map) => {
                if let Some(field) = map.get("select").and_then(Json::as_str) {
                    if let Some(owner) = self.element_type(&node["operand"]) {
                        let (file, _, end) = span(node);
                        let start = end - field.len();
                        self.attribute_ref(&[owner], field, &file, start, end);
                    }
                } else if let Some(call) = map.get("call").and_then(Json::as_str)
                    && map.get("target").is_none()
                    && self.symbols.functions.contains_key(call)
                {
                    let (file, start, _) = span(node);
                    self.add_range(&file, start, start + call.len(), format!("fn:{call}"));
                } else if let Some(ident) = map.get("ident").and_then(Json::as_str)
                    && let Some((name, function, _)) = self.local(ident).cloned()
                {
                    self.add(node, format!("local:{function}:{name}"), false);
                }
                for (key, value) in map {
                    if key != "span" {
                        self.cel(value);
                    }
                }
            }
            Json::Array(items) => {
                for item in items {
                    self.cel(item);
                }
            }
            _ => {}
        }
    }

    fn statements(&mut self, body: &Json) {
        for statement in body.as_array().into_iter().flatten() {
            let kind = statement["kind"].as_str().unwrap_or("");
            for key in [
                "value",
                "target",
                "condition",
                "iterable",
                "parent",
                "after",
                "before",
                "at",
                "size",
                "source",
                "by",
                "x",
                "y",
                "width",
                "height",
                "on",
            ] {
                if let Some(expr) = statement.get(key).and_then(|e| e.get("ast")) {
                    self.cel(expr);
                }
            }
            if let Some(to) = statement.get("to") {
                if let Some(ast) = to.get("ast") {
                    self.cel(ast);
                } else {
                    self.declared_type(to);
                }
            }
            match kind {
                "let" | "var" => {
                    let name = text(&statement["name"]).to_owned();
                    let value_type = statement["value"]["ast"]
                        .get("ident")
                        .and_then(Json::as_str)
                        .and_then(|i| self.local(i))
                        .and_then(|(_, _, t)| t.clone());
                    self.declare(
                        &statement["name"],
                        format!("local:{}:{name}", self.function),
                        format!("{kind} {name}"),
                        None,
                    );
                    self.locals.push((name, self.function.clone(), value_type));
                }
                "for" => {
                    for name in statement["names"].as_array().into_iter().flatten() {
                        let local = text(name).to_owned();
                        self.declare(
                            name,
                            format!("local:{}:{local}", self.function),
                            format!("loop variable {local}"),
                            None,
                        );
                        self.locals.push((local, self.function.clone(), None));
                    }
                }
                "assign" => {
                    let name = text(&statement["name"]).to_owned();
                    if let Some((local, function, _)) = self.local(&name).cloned() {
                        self.add(&statement["name"], format!("local:{function}:{local}"), false);
                    }
                }
                "create" | "connect" => {
                    let type_name = text(&statement["type"]["name"]).to_owned();
                    self.declared_type(&statement["type"]);
                    for assignment in statement["assignments"].as_array().into_iter().flatten() {
                        let key = assignment["key"].as_str().unwrap_or("").to_owned();
                        let (file, start, _) = span(assignment);
                        self.attribute_ref(std::slice::from_ref(&type_name), &key, &file, start, start + key.len());
                        self.cel(&assignment["value"]["ast"]);
                    }
                    if let Some(name) = statement.get("as") {
                        let local = text(name).to_owned();
                        self.declare(
                            name,
                            format!("local:{}:{local}", self.function),
                            format!("the new {type_name} {local}"),
                            None,
                        );
                        self.locals.push((local, self.function.clone(), Some(type_name)));
                    }
                }
                "set" | "unset" => {
                    let path: Vec<&Json> = statement["path"]
                        .as_array()
                        .map(|p| p.iter().collect())
                        .unwrap_or_default();
                    if let Some(last) = path.last() {
                        let owner = if path.len() == 1 {
                            self.me.first().cloned()
                        } else {
                            let head = text(path[0]);
                            if head == "self" {
                                self.me.first().cloned()
                            } else {
                                self.local(head).and_then(|(_, _, t)| t.clone())
                            }
                        };
                        if path.len() > 1
                            && let Some((local, function, _)) = self.local(text(path[0])).cloned()
                        {
                            self.add(path[0], format!("local:{function}:{local}"), false);
                        }
                        if let Some(owner) = owner {
                            let (file, start, end) = span(last);
                            self.attribute_ref(&[owner], text(last), &file, start, end);
                        }
                    }
                }
                "view" => {
                    for assignment in statement["assignments"].as_array().into_iter().flatten() {
                        self.cel(&assignment["value"]["ast"]);
                    }
                }
                "call" => {
                    let name = text(&statement["operation"]).to_owned();
                    let symbol = if self.symbols.operations.contains_key(&name) {
                        Some(format!("op:{name}"))
                    } else if self.symbols.functions.contains_key(&name) {
                        Some(format!("fn:{name}"))
                    } else {
                        None
                    };
                    if let Some(symbol) = symbol {
                        self.add(&statement["operation"], symbol, false);
                    }
                    for arg in statement["args"].as_array().into_iter().flatten() {
                        self.cel(&arg["value"]["ast"]);
                    }
                }
                "openForm" => {
                    let name = text(&statement["form"]).to_owned();
                    if self.symbols.forms.contains_key(&name) {
                        self.add(&statement["form"], format!("form:{name}"), false);
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

    fn binding(&mut self, binding: &Json) {
        let name = text(&binding["name"]).to_owned();
        self.declare(
            &binding["name"],
            format!("binding:{name}"),
            format!("binding {name}"),
            binding.get("doc"),
        );
        if let Some(capability) = binding.get("capability") {
            let capability_name = text(capability).to_owned();
            self.add(capability, format!("capability:{capability_name}"), false);
        }
        for member in binding["members"].as_array().into_iter().flatten() {
            if member["kind"] != "rule" {
                continue;
            }
            let type_name = text(&member["type"]["name"]).to_owned();
            self.type_ref(&member["type"]["name"]);
            if let Some(alias) = member.get("as") {
                let rule = text(alias).to_owned();
                self.declare(
                    alias,
                    format!("rule:{name}.{rule}"),
                    format!("rule {rule} of binding {name}"),
                    member.get("doc"),
                );
            }
            for item in member["members"].as_array().into_iter().flatten() {
                match item["kind"].as_str().unwrap_or("") {
                    "attribute" => {
                        if let Some(attribute) = item.get("name") {
                            let (file, start, end) = span(attribute);
                            self.attribute_ref(std::slice::from_ref(&type_name), text(attribute), &file, start, end);
                        }
                        if let Some(references) = item.get("references") {
                            for rule in references["rules"].as_array().into_iter().flatten() {
                                self.add(rule, format!("rule:{name}.{}", text(rule)), false);
                            }
                        }
                    }
                    "parent" | "within" => {
                        for rule in item["rules"].as_array().into_iter().flatten() {
                            self.add(rule, format!("rule:{name}.{}", text(rule)), false);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
