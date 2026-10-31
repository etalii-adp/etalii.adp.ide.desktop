//! The lowering of a parsed file to its abstract tree, `atd.schema.json#/$defs/Tree` (ATD §5.5, §6.6).

use serde_json::{Map, Value as Json, json};

use crate::parse::{Child, Node, Parse, TokenKind};
use crate::tables::terminals;
use crate::text::LineIndex;

/// Why a file has no abstract tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LowerError {
    pub message: String,
}

struct Lowerer<'a> {
    parse: &'a Parse,
    file: &'a str,
    lines: LineIndex,
}

/// The abstract tree of a parsed file, named `file`. A file whose tree was not built has none.
pub fn lower(parse: &Parse, file: &str) -> Result<Json, LowerError> {
    let root = parse.root.as_ref().ok_or(LowerError {
        message: "the file did not parse far enough to have a description".into(),
    })?;
    let lowerer = Lowerer {
        parse,
        file,
        lines: LineIndex::new(&parse.text),
    };
    lowerer.description(root)
}

const MACROS: &[&str] = &["has", "all", "exists", "exists_one", "map", "filter", "bind"];

impl Lowerer<'_> {
    // ------------------------------------------------------------------------------------------------ helpers

    fn span(&self, start: usize, end: usize) -> Json {
        let (line, column) = self.lines.position(&self.parse.text, start);
        let (end_line, end_column) = self.lines.position(&self.parse.text, end);
        json!({"file": self.file, "start": start, "end": end, "line": line, "column": column,
               "endLine": end_line, "endColumn": end_column})
    }

    fn node_span(&self, node: &Node) -> Json {
        self.span(node.start, node.end)
    }

    fn token_text(&self, token: usize) -> &str {
        self.parse.token_text(token)
    }

    fn token_span(&self, token: usize) -> Json {
        let t = &self.parse.tokens[token];
        self.span(t.start, t.end)
    }

    /// The node, through `?rule`s that hold one child node and nothing but filtered tokens.
    fn unwrap<'n>(&self, mut node: &'n Node) -> &'n Node {
        loop {
            if !node.inline() {
                return node;
            }
            let nodes: Vec<&Node> = node.nodes().collect();
            let filtered_only = node.tokens().all(|t| self.is_filtered(t));
            if nodes.len() == 1 && filtered_only {
                node = nodes[0];
            } else {
                return node;
            }
        }
    }

    /// An anonymous string terminal, which Lark filters from the tree.
    fn is_filtered(&self, token: usize) -> bool {
        let t = &self.parse.tokens[token];
        t.terminal != u16::MAX && terminals()[t.terminal as usize].name.starts_with('"')
    }

    fn child_nodes<'n>(&self, node: &'n Node) -> Vec<&'n Node> {
        node.nodes().map(|n| self.unwrap(n)).collect()
    }

    fn find<'n>(&self, node: &'n Node, kind: &str) -> Option<&'n Node> {
        self.child_nodes(node).into_iter().find(|n| n.kind() == kind)
    }

    fn find_all<'n>(&self, node: &'n Node, kind: &str) -> Vec<&'n Node> {
        self.child_nodes(node)
            .into_iter()
            .filter(|n| n.kind() == kind)
            .collect()
    }

    /// The first token of the node whose terminal is `name` (a named terminal, or a keyword's quoted string).
    fn token_named(&self, node: &Node, name: &str) -> Option<usize> {
        node.tokens().find(|t| {
            let terminal = self.parse.tokens[*t].terminal;
            terminal != u16::MAX && terminals()[terminal as usize].name == name
        })
    }

    fn has_keyword(&self, node: &Node, keyword: &str) -> bool {
        self.token_named(node, &format!("\"{keyword}\"")).is_some()
    }

    /// The `///` comments directly above the node, joined (ATD §3.2).
    fn doc_comments(&self, node: &Node) -> Option<String> {
        // The tokens are in text order: the node's first is found by halving, not by a scan.
        let tokens = &self.parse.tokens;
        let from = tokens.partition_point(|t| t.start < node.start);
        let first = (from..tokens.len())
            .take_while(|i| tokens[*i].start == node.start)
            .find(|i| tokens[*i].kind == TokenKind::Normal)?;
        let mut lines = Vec::new();
        let mut i = first;
        while i > 0 {
            i -= 1;
            let token = &self.parse.tokens[i];
            let text = token.text(&self.parse.text);
            if token.kind != TokenKind::Trivia {
                break;
            }
            if text.trim().is_empty() {
                if text.matches('\n').count() > 1 {
                    break;
                }
                continue;
            }
            if let Some(rest) = text.strip_prefix("///") {
                lines.push(rest.strip_prefix(' ').unwrap_or(rest).trim_end().to_owned());
            } else {
                break;
            }
        }
        if lines.is_empty() {
            return None;
        }
        lines.reverse();
        Some(lines.join("\n"))
    }

    fn with_doc(&self, node: &Node, mut object: Map<String, Json>, extra: Option<String>) -> Json {
        let doc = match (self.doc_comments(node), extra) {
            (Some(a), Some(b)) => Some(format!("{a}\n{b}")),
            (a, b) => a.or(b),
        };
        if let Some(doc) = doc {
            object.insert("doc".into(), Json::String(doc));
        }
        Json::Object(object)
    }

    fn object(&self, kind: &str, node: &Node) -> Map<String, Json> {
        let mut object = Map::new();
        object.insert("kind".into(), json!(kind));
        object.insert("span".into(), self.node_span(node));
        object
    }

    // ------------------------------------------------------------------------------------------------ names

    fn name(&self, node: &Node) -> Json {
        let token = self.name_token(node);
        json!({"text": self.token_text(token), "span": self.token_span(token)})
    }

    fn name_text(&self, node: &Node) -> String {
        self.token_text(self.name_token(node)).to_owned()
    }

    fn name_token(&self, node: &Node) -> usize {
        let node = self.unwrap(node);
        if let Some(token) = node.tokens().next() {
            return token;
        }
        let inner = node.nodes().next().expect("a name holds a word or a keyword");
        self.name_token(inner)
    }

    fn qualified(&self, node: &Node) -> Json {
        let parts: Vec<String> = self.find_all(node, "name").iter().map(|n| self.name_text(n)).collect();
        json!({"text": parts.join("."), "span": self.node_span(node)})
    }

    fn names(&self, node: &Node) -> Json {
        Json::Array(
            self.find_all(node, "qualified")
                .iter()
                .map(|q| self.qualified(q))
                .collect(),
        )
    }

    /// A binding's, rule's, block's, section's or file rule's name (ATD §3.6, §11.3): names and dashed words, joined
    /// by `.` as FBL's names are.
    fn rule_name(&self, node: &Node) -> Json {
        let mut parts = Vec::new();
        self.collect_words(node, &mut parts);
        json!({"text": parts.join("."), "span": self.node_span(node)})
    }

    fn collect_words(&self, node: &Node, parts: &mut Vec<String>) {
        for child in &node.children {
            match child {
                Child::Token(t) => {
                    let text = self.token_text(*t);
                    if text != "." && self.parse.tokens[*t].kind != TokenKind::Trivia {
                        parts.push(text.to_owned());
                    }
                }
                Child::Node(n) => self.collect_words(n, parts),
            }
        }
    }

    /// The names a `rule_names` node lists; a grammar before dashed rule names lists them as `qualified` names.
    fn rule_names(&self, node: &Node) -> Json {
        let mut names = self.find_all(node, "rule_name");
        if names.is_empty() {
            names = self.find_all(node, "qualified");
        }
        Json::Array(names.iter().map(|n| self.rule_name(n)).collect())
    }

    /// A construct's own `rule_name`, or its `name` in a grammar before dashed rule names.
    fn own_rule_name<'n>(&self, node: &'n Node) -> Option<&'n Node> {
        self.find(node, "rule_name").or_else(|| self.find(node, "name"))
    }

    /// A construct's `rule_names`, or its `names` in a grammar before dashed rule names.
    fn own_rule_names<'n>(&self, node: &'n Node) -> Option<&'n Node> {
        self.find(node, "rule_names").or_else(|| self.find(node, "names"))
    }

    fn type_ref(&self, node: &Node) -> Json {
        let mut object = Map::new();
        object.insert(
            "name".into(),
            self.qualified(self.find(node, "qualified").expect("a type has a name")),
        );
        let arguments: Vec<Json> = self
            .find_all(node, "type_ref")
            .iter()
            .map(|t| self.type_ref(t))
            .collect();
        if !arguments.is_empty() {
            object.insert("arguments".into(), Json::Array(arguments));
        }
        object.insert("span".into(), self.node_span(node));
        Json::Object(object)
    }

    fn string_token(&self, node: &Node) -> Option<String> {
        self.token_named(node, "STRING")
            .map(|t| decode_string(self.token_text(t)))
    }

    // ------------------------------------------------------------------------------------------------ values

    fn value(&self, node: &Node) -> Json {
        let node = self.unwrap(node);
        let span = self.node_span(node);
        match node.kind() {
            "value" => {
                let inner = node.nodes().next().expect("a value holds its form");
                self.value(inner)
            }
            "cel_value" => {
                let expr = self.child_nodes(node).into_iter().next().expect("an expression");
                json!({"cel": self.text_of(expr), "ast": self.cel(expr), "span": span})
            }
            "attribute_ref" => json!({"attribute": self.first_token_text(node)[1..], "span": span}),
            "token_ref" => json!({"token": self.first_token_text(node)[1..], "span": span}),
            "param_ref" => json!({"param": self.first_token_text(node)[1..], "span": span}),
            "settings_map" => json!({"settings": self.map_entries(node), "span": span}),
            "value_list" => {
                let items: Vec<Json> = self.find_all(node, "value").iter().map(|v| self.value(v)).collect();
                json!({"list": items, "span": span})
            }
            "word_literal" => {
                let text = match node.nodes().next() {
                    Some(name) => self.name_text(name),
                    None => self.first_token_text(node).to_owned(),
                };
                let literal = match text.as_str() {
                    "true" => json!(true),
                    "false" => json!(false),
                    "null" => Json::Null,
                    other => json!(other),
                };
                json!({"literal": literal, "form": "word", "span": span})
            }
            "string_literal" => {
                json!({"literal": decode_string(self.first_token_text(node)), "form": "string", "span": span})
            }
            "number_literal" => json!({"literal": number(self.first_token_text(node)), "form": "number", "span": span}),
            "negative_literal" => {
                let digits = self.parse.token_text(node.tokens().last().expect("a number"));
                json!({"literal": negate(number(digits)), "form": "number", "span": span})
            }
            "length_literal" => json!({"literal": self.first_token_text(node), "form": "length", "span": span}),
            other => json!({"literal": format!("<{other}>"), "form": "word", "span": span}),
        }
    }

    fn first_token_text(&self, node: &Node) -> &str {
        self.token_text(node.tokens().next().expect("a token"))
    }

    fn text_of(&self, node: &Node) -> String {
        self.parse.text[node.start..node.end].to_owned()
    }

    fn map_entries(&self, node: &Node) -> Json {
        Json::Array(
            self.child_nodes(node)
                .into_iter()
                .filter(|n| matches!(n.kind(), "map_entry" | "quoted_entry"))
                .map(|entry| {
                    let value = self.value(
                        self.find(entry, "value")
                            .or_else(|| self.value_child(entry))
                            .expect("a value"),
                    );
                    let (key, quoted) = if entry.kind() == "quoted_entry" {
                        (self.string_token(entry).unwrap_or_default(), true)
                    } else {
                        (
                            key_text(self.token_text(self.token_named(entry, "KEY").expect("a key"))),
                            false,
                        )
                    };
                    let mut object = Map::new();
                    object.insert("key".into(), json!(key));
                    if quoted {
                        object.insert("quoted".into(), json!(true));
                    }
                    object.insert("value".into(), value);
                    object.insert("span".into(), self.node_span(entry));
                    Json::Object(object)
                })
                .collect(),
        )
    }

    /// The value child of a node whose value is a `cel_value` or a reference rather than a `value` node.
    fn value_child<'n>(&self, node: &'n Node) -> Option<&'n Node> {
        self.child_nodes(node).into_iter().find(|n| {
            matches!(
                n.kind(),
                "value" | "cel_value" | "attribute_ref" | "token_ref" | "param_ref" | "settings_map" | "value_list"
            )
        })
    }

    fn setting(&self, node: &Node) -> Json {
        let key = key_text(self.token_text(self.token_named(node, "KEY").expect("a setting has a key")));
        let value = self.value(self.value_child(node).expect("a setting has a value"));
        json!({"key": key, "value": value, "span": self.node_span(node)})
    }

    /// A settings block's settings, with its `label` members as `label` settings, and its `doc` members' text.
    fn settings_block(&self, node: Option<&Node>) -> (Vec<Json>, Option<String>) {
        let mut settings = Vec::new();
        let mut doc: Vec<String> = Vec::new();
        let Some(node) = node else { return (settings, None) };
        for member in self.child_nodes(node) {
            match member.kind() {
                "setting" => settings.push(self.setting(member)),
                "label_member" => {
                    let value = self.value(self.value_child(member).expect("a label has a value"));
                    settings.push(json!({"key": "label", "value": value, "span": self.node_span(member)}));
                }
                "doc_member" => doc.push(self.doc_text(member)),
                _ => {}
            }
        }
        (settings, (!doc.is_empty()).then(|| doc.join("\n")))
    }

    fn doc_text(&self, member: &Node) -> String {
        let value = self.value(self.value_child(member).expect("a doc has a value"));
        match value.get("literal") {
            Some(Json::String(text)) => text.clone(),
            _ => value.get("cel").and_then(Json::as_str).unwrap_or_default().to_owned(),
        }
    }

    fn expr(&self, node: &Node) -> Json {
        let node = self.unwrap(node);
        json!({"cel": self.text_of(node), "ast": self.cel(node), "span": self.node_span(node)})
    }

    // ------------------------------------------------------------------------------------------------ CEL

    fn cel(&self, node: &Node) -> Json {
        let node = self.unwrap(node);
        let span = self.node_span(node);
        let kids = self.child_nodes(node);
        let call = |name: &str, args: Vec<Json>| json!({"call": name, "args": args, "span": span});
        let binary = |name: &str| call(name, vec![self.cel(kids[0]), self.cel(kids[1])]);
        match node.kind() {
            "cel_conditional" => call("_?_:_", kids.iter().map(|k| self.cel(k)).collect()),
            "cel_or" => binary("_||_"),
            "cel_and" => binary("_&&_"),
            "cel_less" => binary("_<_"),
            "cel_less_equal" => binary("_<=_"),
            "cel_greater_equal" => binary("_>=_"),
            "cel_greater" => binary("_>_"),
            "cel_equal" => binary("_==_"),
            "cel_not_equal" => binary("_!=_"),
            "cel_in" => binary("@in"),
            "cel_add" => binary("_+_"),
            "cel_subtract" => binary("_-_"),
            "cel_multiply" => binary("_*_"),
            "cel_divide" => binary("_/_"),
            "cel_modulo" => binary("_%_"),
            "cel_not" => call("!_", vec![self.cel(kids[0])]),
            "cel_negate" => call("-_", vec![self.cel(kids[0])]),
            "cel_index" => binary("_[_]"),
            "cel_optional_index" => binary("_[?_]"),
            "cel_select" => json!({"select": self.name_text(kids[1]), "operand": self.cel(kids[0]), "span": span}),
            "cel_optional_select" => {
                json!({"select": self.name_text(kids[1]), "operand": self.cel(kids[0]), "optional": true, "span": span})
            }
            "cel_method" => {
                let name = self.name_text(kids[1]);
                let args = kids.get(2).map(|a| self.cel_args(a)).unwrap_or_default();
                let mut object = json!({"call": name, "target": self.cel(kids[0]), "args": args, "span": span});
                if MACROS.contains(&name.as_str()) {
                    object["macro"] = json!(true);
                }
                object
            }
            "cel_call" | "cel_root_call" => {
                let name = self.name_text(kids[0]);
                let args = kids.get(1).map(|a| self.cel_args(a)).unwrap_or_default();
                let mut object = json!({"call": name, "args": args, "span": span});
                if name == "has" {
                    object["macro"] = json!(true);
                }
                object
            }
            "cel_ident_expr" => json!({"ident": self.name_text(kids[0]), "span": span}),
            "cel_root_ident" => json!({"ident": self.name_text(kids[0]), "root": true, "span": span}),
            "cel_list" => {
                let mut items = Vec::new();
                let mut optional = Vec::new();
                if let Some(elements) = kids.first() {
                    for (i, element) in self.child_nodes(elements).into_iter().enumerate() {
                        if element.kind() == "cel_optional_element" {
                            optional.push(json!(i));
                        }
                        let inner = self.child_nodes(element).into_iter().next().unwrap_or(element);
                        items.push(self.cel(inner));
                    }
                }
                let mut object = json!({"list": items, "span": span});
                if !optional.is_empty() {
                    object["optional"] = Json::Array(optional);
                }
                object
            }
            "cel_map" => {
                let mut entries = Vec::new();
                if let Some(list) = kids.first() {
                    for entry in self.child_nodes(list) {
                        let parts = self.child_nodes(entry);
                        let mut object = json!({"key": self.cel(parts[0]), "value": self.cel(parts[1])});
                        if entry.kind() == "cel_optional_entry" {
                            object["optional"] = json!(true);
                        }
                        entries.push(object);
                    }
                }
                json!({"map": entries, "span": span})
            }
            "cel_literal" | "cel_true" | "cel_false" | "cel_null" => self.cel_literal(node, span),
            "expr" | "primary" => kids
                .first()
                .map(|k| self.cel(k))
                .unwrap_or(json!({"ident": "", "span": span})),
            _ => json!({"ident": self.text_of(node), "span": span}),
        }
    }

    fn cel_args(&self, node: &Node) -> Vec<Json> {
        self.child_nodes(node).iter().map(|a| self.cel(a)).collect()
    }

    fn cel_literal(&self, node: &Node, span: Json) -> Json {
        match node.kind() {
            "cel_true" => return json!({"literal": true, "type": "bool", "text": "true", "span": span}),
            "cel_false" => return json!({"literal": false, "type": "bool", "text": "false", "span": span}),
            "cel_null" => return json!({"literal": null, "type": "null_type", "text": "null", "span": span}),
            _ => {}
        }
        let token = node.tokens().next().expect("a literal");
        let text = self.token_text(token);
        let (literal, kind) = match self.parse.tokens[token].name() {
            "INT" => (number(text), "int"),
            "UINT" => (number(&text[..text.len() - 1]), "uint"),
            "DOUBLE" => (number(text), "double"),
            "BYTES" => (
                json!(decode_string(text.trim_start_matches(['b', 'B', 'r', 'R']))),
                "bytes",
            ),
            _ => (json!(decode_string(text)), "string"),
        };
        json!({"literal": literal, "type": kind, "text": text, "span": span})
    }

    // ------------------------------------------------------------------------------------------------ the description

    fn description(&self, root: &Node) -> Result<Json, LowerError> {
        let header = self.find(root, "header").ok_or(LowerError {
            message: "the file has no header".into(),
        })?;
        let version = self.token_text(self.token_named(header, "VERSION").expect("a version"));
        let body = self
            .child_nodes(root)
            .into_iter()
            .find(|n| matches!(n.kind(), "tool" | "part"))
            .ok_or(LowerError {
                message: "the file has neither a tool nor a part".into(),
            })?;
        let origin = self.token_text(self.token_named(body, "ORIGIN").expect("an origin"));
        let members: Vec<Json> = self
            .child_nodes(body)
            .into_iter()
            .filter(|m| !matches!(m.kind(), "diagram" | "designer" | "editor"))
            .map(|m| self.member(m))
            .collect();
        let mut object = self.object(body.kind(), body);
        if body.kind() == "tool" {
            let kind = self
                .child_nodes(body)
                .into_iter()
                .find(|n| matches!(n.kind(), "diagram" | "designer" | "editor"));
            object.insert("toolKind".into(), json!(kind.map(|k| k.kind()).unwrap_or("diagram")));
            object.insert("origin".into(), json!(origin));
            object.insert("version".into(), json!(self.string_token(body).unwrap_or_default()));
        } else {
            object.insert("origin".into(), json!(origin));
        }
        object.insert("members".into(), Json::Array(members));
        let description = self.with_doc(body, object, None);
        Ok(json!({"atd": version, "file": self.file, "description": description}))
    }

    fn member(&self, node: &Node) -> Json {
        let node = self.unwrap(node);
        match node.kind() {
            "label_member" => {
                let mut o = self.object("label", node);
                o.insert("value".into(), self.value(self.value_child(node).expect("a value")));
                self.with_doc(node, o, None)
            }
            "doc_member" => {
                let mut o = self.object("doc", node);
                o.insert("value".into(), self.value(self.value_child(node).expect("a value")));
                Json::Object(o)
            }
            "setting" => {
                let mut o = self.object("setting", node);
                o.insert("setting".into(), self.setting(node));
                Json::Object(o)
            }
            "import_decl" => {
                let mut o = self.object("import", node);
                o.insert("path".into(), json!(self.string_token(node).unwrap_or_default()));
                if let Some(name) = self.find(node, "name") {
                    o.insert("as".into(), self.name(name));
                }
                let (settings, doc) = self.settings_block(self.find(node, "settings_block"));
                if !settings.is_empty() {
                    o.insert("settings".into(), Json::Array(settings));
                }
                self.with_doc(node, o, doc)
            }
            "requires_decl" => {
                let mut o = self.object("requires", node);
                o.insert(
                    "capability".into(),
                    self.qualified(self.find(node, "qualified").expect("a capability")),
                );
                let range = self.token_text(self.token_named(node, "CAPABILITY_RANGE").expect("a range"));
                o.insert("range".into(), json!(&range[1..]));
                self.with_doc(node, o, None)
            }
            "model" => {
                let mut o = self.object("model", node);
                let members: Vec<Json> = self.child_nodes(node).iter().map(|m| self.model_member(m)).collect();
                o.insert("members".into(), Json::Array(members));
                self.with_doc(node, o, None)
            }
            "constraints" => {
                let mut o = self.object("constraints", node);
                let members: Vec<Json> = self.child_nodes(node).iter().map(|m| self.member(m)).collect();
                o.insert("members".into(), Json::Array(members));
                self.with_doc(node, o, None)
            }
            "constraint" => self.constraint(node),
            "builtin" => {
                let mut o = self.object("builtin", node);
                o.insert(
                    "name".into(),
                    self.qualified(self.find(node, "qualified").expect("a name")),
                );
                let (settings, doc) = self.settings_block(self.find(node, "settings_block"));
                o.insert("settings".into(), Json::Array(settings));
                self.with_doc(node, o, doc)
            }
            "operation" => self.operation(node),
            "hook" => self.hook(node),
            "reason" => self.reason(node),
            "message_decl" => {
                let mut o = self.object("message", node);
                o.insert(
                    "name".into(),
                    self.qualified(self.find(node, "qualified").expect("a name")),
                );
                o.insert("value".into(), self.value(self.value_child(node).expect("a value")));
                self.with_doc(node, o, None)
            }
            "behavior" | "persistence" => {
                let mut o = self.object(node.kind(), node);
                let (settings, doc) = self.settings_block(self.find(node, "settings_block"));
                o.insert("settings".into(), Json::Array(settings));
                self.with_doc(node, o, doc)
            }
            "plugin" => {
                let mut o = self.object("plugin", node);
                o.insert(
                    "name".into(),
                    self.qualified(self.find(node, "qualified").expect("a name")),
                );
                let (settings, doc) = self.settings_block(self.find(node, "settings_block"));
                o.insert("settings".into(), Json::Array(settings));
                self.with_doc(node, o, doc)
            }
            "fn_decl" => self.function(node),
            "coordinates" | "notation" | "toolbox" | "layout" => {
                let mut o = self.object(node.kind(), node);
                o.insert("members".into(), self.generic_members(node));
                self.with_doc(node, o, None)
            }
            "table_surface" | "hierarchy_surface" | "text_surface" => {
                let kind = match node.kind() {
                    "table_surface" => "tableSurface",
                    "hierarchy_surface" => "hierarchySurface",
                    _ => "textSurface",
                };
                let mut o = self.object(kind, node);
                o.insert("members".into(), self.generic_members(node));
                self.with_doc(node, o, None)
            }
            "form" => {
                let mut o = self.object("form", node);
                o.insert("name".into(), self.name(self.find(node, "name").expect("a name")));
                if let Some(title) = self.string_token(node) {
                    o.insert("title".into(), json!(title));
                }
                if let Some(names) = self.find(node, "names") {
                    o.insert("for".into(), self.names(names));
                }
                o.insert("members".into(), self.generic_members(node));
                self.with_doc(node, o, None)
            }
            "viewpoint" => {
                let mut o = self.object("viewpoint", node);
                o.insert("name".into(), self.name(self.find(node, "name").expect("a name")));
                if let Some(title) = self.string_token(node) {
                    o.insert("title".into(), json!(title));
                }
                let (settings, doc) = self.settings_block(self.find(node, "settings_block"));
                o.insert("settings".into(), Json::Array(settings));
                self.with_doc(node, o, doc)
            }
            "binding" => self.binding(node),
            other => {
                let mut o = self.object("setting", node);
                o.insert(
                    "setting".into(),
                    json!({"key": format!("x-unknown-{other}"), "value": {"literal": null, "form": "word", "span": self.node_span(node)}, "span": self.node_span(node)}),
                );
                Json::Object(o)
            }
        }
    }

    fn model_member(&self, node: &Node) -> Json {
        match node.kind() {
            "element" | "relation" | "document_decl" => {
                let kind = if node.kind() == "document_decl" {
                    "document"
                } else {
                    node.kind()
                };
                let mut o = self.object(kind, node);
                if let Some(name) = self.find(node, "name") {
                    o.insert("name".into(), self.name(name));
                }
                if let Some(extends) = self.find(node, "extends") {
                    o.insert(
                        "extends".into(),
                        self.names(self.find(extends, "names").expect("names")),
                    );
                }
                if node.kind() == "relation" {
                    let all = self.find_all(node, "names");
                    if all.len() == 2 {
                        o.insert("from".into(), self.names(all[0]));
                        o.insert("to".into(), self.names(all[1]));
                    }
                }
                let mut doc = Vec::new();
                let members: Vec<Json> = self
                    .child_nodes(node)
                    .into_iter()
                    .filter(|m| !matches!(m.kind(), "name" | "extends" | "names"))
                    .filter_map(|m| {
                        if m.kind() == "doc_member" {
                            doc.push(self.doc_text(m));
                            return None;
                        }
                        Some(self.element_member(m))
                    })
                    .collect();
                o.insert("members".into(), Json::Array(members));
                self.with_doc(node, o, (!doc.is_empty()).then(|| doc.join("\n")))
            }
            "enum_decl" => {
                let mut o = self.object("enum", node);
                o.insert("name".into(), self.name(self.find(node, "name").expect("a name")));
                if let Some(label) = self.string_token(node) {
                    o.insert("label".into(), json!(label));
                }
                let mut values = Vec::new();
                let mut settings = Vec::new();
                for member in self.child_nodes(node) {
                    match member.kind() {
                        "enum_value" | "quoted_enum_value" => {
                            let (name, quoted) = match self.find(member, "name") {
                                Some(n) => (self.name_text(n), false),
                                None => (self.string_token(member).unwrap_or_default(), true),
                            };
                            let mut v = Map::new();
                            v.insert("name".into(), json!(name));
                            if quoted {
                                v.insert("quoted".into(), json!(true));
                            }
                            v.insert("span".into(), self.node_span(member));
                            let (s, _) = self.settings_block(self.find(member, "settings_block"));
                            if !s.is_empty() {
                                v.insert("settings".into(), Json::Array(s));
                            }
                            values.push(Json::Object(v));
                        }
                        "setting" => settings.push(self.setting(member)),
                        _ => {}
                    }
                }
                o.insert("values".into(), Json::Array(values));
                if !settings.is_empty() {
                    o.insert("settings".into(), Json::Array(settings));
                }
                self.with_doc(node, o, None)
            }
            "datatype" => {
                let mut o = self.object("datatype", node);
                o.insert("name".into(), self.name(self.find(node, "name").expect("a name")));
                if let Some(base) = self.find(node, "type_ref") {
                    o.insert("base".into(), self.type_ref(base));
                }
                let (settings, doc) = self.settings_block(self.find(node, "settings_block"));
                o.insert("settings".into(), Json::Array(settings));
                self.with_doc(node, o, doc)
            }
            _ => self.member(node),
        }
    }

    fn element_member(&self, node: &Node) -> Json {
        match node.kind() {
            "attribute" => self.attribute_like("attribute", node),
            "contains" => {
                let mut o = self.object("contains", node);
                o.insert("types".into(), self.names(self.find(node, "names").expect("types")));
                if let Some(slot) = self.find(node, "name") {
                    o.insert("slot".into(), self.name(slot));
                }
                if self.has_keyword(node, "ordered") {
                    o.insert("ordered".into(), json!(true));
                }
                let (settings, doc) = self.settings_block(self.find(node, "settings_block"));
                if !settings.is_empty() {
                    o.insert("settings".into(), Json::Array(settings));
                }
                self.with_doc(node, o, doc)
            }
            "port" => {
                let mut o = self.object("port", node);
                o.insert("name".into(), self.name(self.find(node, "name").expect("a name")));
                let (settings, doc) = self.settings_block(self.find(node, "settings_block"));
                if !settings.is_empty() {
                    o.insert("settings".into(), Json::Array(settings));
                }
                self.with_doc(node, o, doc)
            }
            _ => self.member(node),
        }
    }

    /// An attribute or a parameter: its name, type, flags and settings.
    fn attribute_like(&self, kind: &str, node: &Node) -> Json {
        let mut o = self.object(kind, node);
        o.insert("name".into(), self.name(self.find(node, "name").expect("a name")));
        o.insert(
            "type".into(),
            self.type_ref(self.find(node, "type_ref").expect("a type")),
        );
        for flag in self.child_nodes(node) {
            match flag.kind() {
                "required" | "many" | "readonly" => {
                    o.insert(flag.kind().into(), json!(true));
                }
                "default" => {
                    o.insert("default".into(), self.value(self.value_child(flag).expect("a value")));
                }
                _ => {}
            }
        }
        let (settings, doc) = self.settings_block(self.find(node, "settings_block"));
        if !settings.is_empty() {
            o.insert("settings".into(), Json::Array(settings));
        }
        self.with_doc(node, o, doc)
    }

    fn constraint(&self, node: &Node) -> Json {
        let mut o = self.object("constraint", node);
        o.insert("name".into(), self.name(self.find(node, "name").expect("a name")));
        if let Some(names) = self.find(node, "names") {
            o.insert("for".into(), self.names(names));
        }
        let mut settings = Vec::new();
        let mut fixes = Vec::new();
        let mut doc = Vec::new();
        for member in self.child_nodes(node) {
            match member.kind() {
                "when" | "rule" => {
                    o.insert(member.kind().into(), self.expr(self.child_nodes(member)[0]));
                }
                "message" => {
                    o.insert("message".into(), self.value(self.value_child(member).expect("a value")));
                }
                "severity" => {
                    o.insert(
                        "severity".into(),
                        self.name(self.find(member, "name").expect("a severity")),
                    );
                }
                "fix" => fixes.push(self.fix(member)),
                "setting" => settings.push(self.setting(member)),
                "label_member" => {
                    o.insert("label".into(), self.value(self.value_child(member).expect("a value")));
                }
                "doc_member" => doc.push(self.doc_text(member)),
                _ => {}
            }
        }
        if !fixes.is_empty() {
            o.insert("fixes".into(), Json::Array(fixes));
        }
        if !settings.is_empty() {
            o.insert("settings".into(), Json::Array(settings));
        }
        self.with_doc(node, o, (!doc.is_empty()).then(|| doc.join("\n")))
    }

    fn fix(&self, node: &Node) -> Json {
        let mut o = self.object("fix", node);
        o.insert("name".into(), self.name(self.find(node, "name").expect("a name")));
        if let Some(title) = self.string_token(node) {
            o.insert("title".into(), json!(title));
        }
        let mut body = Vec::new();
        let mut settings = Vec::new();
        let mut doc = Vec::new();
        for member in self.child_nodes(node) {
            match member.kind() {
                "when" => {
                    o.insert("when".into(), self.expr(self.child_nodes(member)[0]));
                }
                "setting" => settings.push(self.setting(member)),
                "label_member" => settings.push(json!({"key": "label", "value": self.value(self.value_child(member).expect("a value")), "span": self.node_span(member)})),
                "doc_member" => doc.push(self.doc_text(member)),
                "name" => {}
                "void_return" => body.push(json!({"kind": "return", "span": self.node_span(member)})),
                _ => body.push(self.statement(member)),
            }
        }
        o.insert("body".into(), Json::Array(body));
        if !settings.is_empty() {
            o.insert("settings".into(), Json::Array(settings));
        }
        self.with_doc(node, o, (!doc.is_empty()).then(|| doc.join("\n")))
    }

    fn operation(&self, node: &Node) -> Json {
        let mut o = self.object("operation", node);
        o.insert("name".into(), self.name(self.find(node, "name").expect("a name")));
        if let Some(title) = self.string_token(node) {
            o.insert("title".into(), json!(title));
        }
        if let Some(names) = self.find(node, "names") {
            o.insert("for".into(), self.names(names));
        }
        let mut body = Vec::new();
        let mut params = Vec::new();
        let mut settings = Vec::new();
        let mut doc = Vec::new();
        for member in self.child_nodes(node) {
            match member.kind() {
                "name" | "names" => {}
                "enabled" => {
                    o.insert("enabled".into(), self.expr(self.child_nodes(member)[0]));
                }
                "unavailable" => {
                    o.insert(
                        "unavailable".into(),
                        self.value(self.value_child(member).expect("a value")),
                    );
                }
                "param" => params.push(self.attribute_like("param", member)),
                "confirm" => {
                    let (s, _) = self.settings_block(self.find(member, "settings_block"));
                    o.insert("confirm".into(), Json::Array(s));
                }
                "setting" => settings.push(self.setting(member)),
                "label_member" => {
                    o.insert("label".into(), self.value(self.value_child(member).expect("a value")));
                }
                "doc_member" => doc.push(self.doc_text(member)),
                "void_return" => body.push(json!({"kind": "return", "span": self.node_span(member)})),
                _ => body.push(self.statement(member)),
            }
        }
        if !params.is_empty() {
            o.insert("params".into(), Json::Array(params));
        }
        o.insert("body".into(), Json::Array(body));
        if !settings.is_empty() {
            o.insert("settings".into(), Json::Array(settings));
        }
        self.with_doc(node, o, (!doc.is_empty()).then(|| doc.join("\n")))
    }

    fn hook(&self, node: &Node) -> Json {
        let mut o = self.object("hook", node);
        let names = self.find_all(node, "name");
        o.insert("name".into(), self.name(names[0]));
        o.insert("on".into(), self.name(names[1]));
        if let Some(types) = self.find(node, "names") {
            o.insert("for".into(), self.names(types));
        }
        let mut body = Vec::new();
        let mut settings = Vec::new();
        let mut doc = Vec::new();
        for member in self.child_nodes(node) {
            match member.kind() {
                "name" | "names" => {}
                "when" => {
                    o.insert("when".into(), self.expr(self.child_nodes(member)[0]));
                }
                "setting" => settings.push(self.setting(member)),
                "doc_member" => doc.push(self.doc_text(member)),
                "void_return" => body.push(json!({"kind": "return", "span": self.node_span(member)})),
                _ => body.push(self.statement(member)),
            }
        }
        o.insert("body".into(), Json::Array(body));
        if !settings.is_empty() {
            o.insert("settings".into(), Json::Array(settings));
        }
        self.with_doc(node, o, (!doc.is_empty()).then(|| doc.join("\n")))
    }

    fn reason(&self, node: &Node) -> Json {
        let mut o = self.object("reason", node);
        o.insert("name".into(), self.name(self.find(node, "name").expect("a name")));
        if let Some(text) = self.token_named(node, "STRING") {
            let t = &self.parse.tokens[text];
            o.insert(
                "message".into(),
                json!({"literal": decode_string(self.token_text(text)), "form": "string", "span": self.span(t.start, t.end)}),
            );
        }
        let mut settings = Vec::new();
        let mut doc = Vec::new();
        for member in self.child_nodes(node) {
            match member.kind() {
                "when" => {
                    o.insert("when".into(), self.expr(self.child_nodes(member)[0]));
                }
                "message" => {
                    o.insert("message".into(), self.value(self.value_child(member).expect("a value")));
                }
                "setting" => settings.push(self.setting(member)),
                "doc_member" => doc.push(self.doc_text(member)),
                _ => {}
            }
        }
        if !settings.is_empty() {
            o.insert("settings".into(), Json::Array(settings));
        }
        self.with_doc(node, o, (!doc.is_empty()).then(|| doc.join("\n")))
    }

    fn function(&self, node: &Node) -> Json {
        let mut o = self.object("fn", node);
        o.insert("name".into(), self.name(self.find(node, "name").expect("a name")));
        let params: Vec<Json> = self
            .find_all(node, "fn_param")
            .iter()
            .map(|p| {
                json!({"name": self.name(self.find(p, "name").expect("a name")),
                       "type": self.type_ref(self.find(p, "type_ref").expect("a type"))})
            })
            .collect();
        o.insert("params".into(), Json::Array(params));
        if let Some(returns) = self.find(node, "type_ref") {
            o.insert("returns".into(), self.type_ref(returns));
        }
        if let Some(int) = self.token_named(node, "INT") {
            o.insert("recursion".into(), number(self.token_text(int)));
        }
        o.insert(
            "body".into(),
            Json::Array(self.block(self.find(node, "stmt_block").expect("a block"))),
        );
        self.with_doc(node, o, None)
    }

    fn block(&self, node: &Node) -> Vec<Json> {
        self.child_nodes(node)
            .into_iter()
            .map(|s| {
                if s.kind() == "void_return" {
                    json!({"kind": "return", "span": self.node_span(s)})
                } else {
                    self.statement(s)
                }
            })
            .collect()
    }

    // ------------------------------------------------------------------------------------------------ statements

    fn statement(&self, node: &Node) -> Json {
        let node = self.unwrap(node);
        let kids = self.child_nodes(node);
        let exprs: Vec<&Node> = kids
            .iter()
            .copied()
            .filter(|k| {
                !matches!(
                    k.kind(),
                    "name"
                        | "type_ref"
                        | "path"
                        | "assignments"
                        | "stmt_block"
                        | "if_stmt"
                        | "settings_block"
                        | "qualified"
                        | "call_arg"
                        | "named_arg"
                        | "after"
                        | "before"
                        | "slot"
                        | "at"
                        | "size"
                        | "place"
                        | "by"
                )
            })
            .collect();
        let mut o = Map::new();
        let kind = match node.kind() {
            "let_stmt" => "let",
            "var_stmt" => "var",
            "assign_stmt" => "assign",
            "if_stmt" => "if",
            "for_stmt" => "for",
            "while_stmt" => "while",
            "return_stmt" => "return",
            "break_stmt" => "break",
            "continue_stmt" => "continue",
            "fail_stmt" => "fail",
            "set_action" => "set",
            "unset_action" => "unset",
            "create_action" => "create",
            "connect_action" => "connect",
            "delete_action" => "delete",
            "move_action" => "move",
            "resize_action" => "resize",
            "retype_action" => "retype",
            "reparent_action" => "reparent",
            "reorder_action" => "reorder",
            "view_action" => "view",
            "open_action" => "open",
            "open_form_action" => "openForm",
            "edit_label_action" => "editLabel",
            "select_action" => "select",
            "reveal_action" => "reveal",
            "highlight_action" => "highlight",
            "notify_action" => "notify",
            "layout_action" => "layout",
            "call_action" => "call",
            "plugin_action" => "plugin",
            _ => "fail",
        };
        o.insert("kind".into(), json!(kind));
        o.insert("span".into(), self.node_span(node));
        let names = self.find_all(node, "name");
        match kind {
            "let" | "var" => {
                o.insert("name".into(), self.name(names[0]));
                if let Some(t) = self.find(node, "type_ref") {
                    o.insert("type".into(), self.type_ref(t));
                }
                o.insert("value".into(), self.expr(exprs[0]));
            }
            "assign" => {
                let token = self.token_named(node, "ASSIGN").expect("an assignment");
                let text = self.token_text(token);
                let name = text.trim_end_matches('=').trim_end();
                let t = &self.parse.tokens[token];
                o.insert(
                    "name".into(),
                    json!({"text": name, "span": self.span(t.start, t.start + name.len())}),
                );
                o.insert("value".into(), self.expr(exprs[0]));
            }
            "if" => {
                o.insert("condition".into(), self.expr(exprs[0]));
                let blocks = self.find_all(node, "stmt_block");
                o.insert("then".into(), Json::Array(self.block(blocks[0])));
                if let Some(else_if) = self.find(node, "if_stmt") {
                    o.insert("else".into(), Json::Array(vec![self.statement(else_if)]));
                } else if let Some(else_block) = blocks.get(1) {
                    o.insert("else".into(), Json::Array(self.block(else_block)));
                }
            }
            "for" => {
                o.insert(
                    "names".into(),
                    Json::Array(names.iter().map(|n| self.name(n)).collect()),
                );
                o.insert("iterable".into(), self.expr(exprs[0]));
                o.insert("limit".into(), self.limit(node));
                o.insert(
                    "body".into(),
                    Json::Array(self.block(self.find(node, "stmt_block").expect("a block"))),
                );
            }
            "while" => {
                o.insert("condition".into(), self.expr(exprs[0]));
                o.insert("limit".into(), self.limit(node));
                o.insert(
                    "body".into(),
                    Json::Array(self.block(self.find(node, "stmt_block").expect("a block"))),
                );
            }
            "return" | "fail" | "delete" | "open" | "select" | "reveal" => {
                if let Some(e) = exprs.first() {
                    let key = if matches!(kind, "return" | "fail") {
                        "value"
                    } else {
                        "target"
                    };
                    o.insert(key.into(), self.expr(e));
                }
            }
            "set" | "unset" => {
                let path = self.find(node, "path").expect("a path");
                o.insert(
                    "path".into(),
                    Json::Array(self.find_all(path, "name").iter().map(|n| self.name(n)).collect()),
                );
                if kind == "set" {
                    o.insert("value".into(), self.expr(exprs[0]));
                }
            }
            "create" | "connect" => {
                o.insert(
                    "type".into(),
                    self.type_ref(self.find(node, "type_ref").expect("a type")),
                );
                if let Some(assignments) = self.find(node, "assignments") {
                    o.insert("assignments".into(), self.assignments(assignments));
                }
                if kind == "create" {
                    if let Some(parent) = exprs.first() {
                        o.insert("parent".into(), self.expr(parent));
                    }
                    self.options(node, &mut o);
                } else {
                    o.insert("source".into(), self.expr(exprs[0]));
                    o.insert("to".into(), self.expr(exprs[1]));
                }
                if let Some(name) = names.last() {
                    o.insert("as".into(), self.name(name));
                }
            }
            "move" => {
                o.insert("target".into(), self.expr(exprs[0]));
                let has_to = self.has_keyword(node, "to");
                let has_into = self.has_keyword(node, "into");
                if has_to {
                    o.insert("x".into(), self.expr(exprs[1]));
                    o.insert("y".into(), self.expr(exprs[2]));
                }
                if has_into {
                    o.insert("parent".into(), self.expr(exprs[exprs.len() - 1]));
                }
                self.options(node, &mut o);
            }
            "resize" => {
                o.insert("target".into(), self.expr(exprs[0]));
                o.insert("width".into(), self.expr(exprs[1]));
                o.insert("height".into(), self.expr(exprs[2]));
            }
            "retype" => {
                o.insert("target".into(), self.expr(exprs[0]));
                o.insert("to".into(), self.type_ref(self.find(node, "type_ref").expect("a type")));
            }
            "reparent" => {
                o.insert("target".into(), self.expr(exprs[0]));
                o.insert("parent".into(), self.expr(exprs[1]));
                self.options(node, &mut o);
            }
            "reorder" => {
                o.insert("target".into(), self.expr(exprs[0]));
                self.options(node, &mut o);
            }
            "view" => {
                o.insert("target".into(), self.expr(exprs[0]));
                o.insert(
                    "assignments".into(),
                    self.assignments(self.find(node, "assignments").expect("assignments")),
                );
            }
            "openForm" => {
                o.insert("form".into(), self.name(names[0]));
                o.insert("target".into(), self.expr(exprs[0]));
            }
            "editLabel" | "highlight" => {
                o.insert("target".into(), self.expr(exprs[0]));
                let (settings, _) = self.settings_block(self.find(node, "settings_block"));
                if !settings.is_empty() {
                    o.insert("settings".into(), Json::Array(settings));
                }
            }
            "notify" => {
                o.insert("value".into(), self.expr(exprs[0]));
                if let Some(severity) = names.first() {
                    o.insert("severity".into(), self.name(severity));
                }
            }
            "layout" => {
                o.insert("name".into(), self.name(names[0]));
                if let Some(scope) = exprs.first() {
                    o.insert("on".into(), self.expr(scope));
                }
                let (settings, _) = self.settings_block(self.find(node, "settings_block"));
                if !settings.is_empty() {
                    o.insert("settings".into(), Json::Array(settings));
                }
            }
            "call" | "plugin" => {
                o.insert(
                    "operation".into(),
                    self.qualified(self.find(node, "qualified").expect("a name")),
                );
                let args: Vec<Json> = kids
                    .iter()
                    .filter(|k| matches!(k.kind(), "call_arg" | "named_arg"))
                    .map(|a| {
                        let inner = self.child_nodes(a);
                        let value = self.expr(inner.first().copied().unwrap_or(a));
                        match self.token_named(a, "KEY") {
                            Some(key) => json!({"key": key_text(self.token_text(key)), "value": value}),
                            None => json!({"value": value}),
                        }
                    })
                    .collect();
                o.insert("args".into(), Json::Array(args));
            }
            _ => {}
        }
        Json::Object(o)
    }

    fn limit(&self, node: &Node) -> Json {
        self.token_named(node, "INT")
            .map(|t| number(self.token_text(t)))
            .unwrap_or(json!(0))
    }

    fn assignments(&self, node: &Node) -> Json {
        Json::Array(
            self.find_all(node, "assignment")
                .iter()
                .map(|a| {
                    let key = key_text(self.token_text(self.token_named(a, "KEY").expect("a key")));
                    json!({"key": key, "value": self.expr(self.child_nodes(a)[0]), "span": self.node_span(a)})
                })
                .collect(),
        )
    }

    fn options(&self, node: &Node, o: &mut Map<String, Json>) {
        for option in self.child_nodes(node) {
            let kind = option.kind();
            match kind {
                "after" | "before" | "at" | "size" | "by" => {
                    o.insert(kind.into(), self.expr(self.child_nodes(option)[0]));
                }
                "slot" | "place" => {
                    o.insert(kind.into(), self.name(self.find(option, "name").expect("a name")));
                }
                _ => {}
            }
        }
    }

    // ------------------------------------------------------------------------------------------------ surfaces and bindings

    /// The members of a surface construct, each by its keyword (ATD `$defs/Coordinates` and the others).
    fn generic_members(&self, node: &Node) -> Json {
        Json::Array(
            self.child_nodes(node)
                .into_iter()
                .filter(|m| !matches!(m.kind(), "name" | "names" | "qualified"))
                .map(|m| self.generic(m))
                .collect(),
        )
    }

    fn generic(&self, node: &Node) -> Json {
        let kind = match node.kind() {
            "node_notation" => "node",
            "edge_notation" => "edge",
            "port_notation" | "node_port" => "port",
            "shape_use" | "shape_definition" => "shape",
            "label_decl" | "label_member" => "label",
            "marker_definition" => "marker",
            "icon_definition" => "icon",
            "coordinate_system" => "system",
            "snapping_profile" => "snapping",
            "default_system" | "default_layout" => "default",
            "tool_group" => "group",
            "context_tools" => "context",
            "context_tool" => "tool",
            "form_item" => "item",
            "value_type" => "valueType",
            "token_class" => "token",
            "doc_member" => "doc",
            other => other,
        };
        let mut o = self.object(kind, node);
        match node.kind() {
            "setting" => {
                o.insert("setting".into(), self.setting(node));
                return Json::Object(o);
            }
            "fix" => return self.fix(node),
            _ => {}
        }
        if let Some(names) = self.find(node, "names") {
            o.insert("names".into(), self.names(names));
        }
        if let Some(extends) = self.find(node, "extends") {
            o.insert(
                "extends".into(),
                self.names(self.find(extends, "names").expect("names")),
            );
        }
        let own_names = self.find_all(node, "name");
        if let Some(name) = own_names.first() {
            o.insert("name".into(), self.name(name));
        }
        if let Some(qualified) = self.find(node, "qualified") {
            o.insert("capability".into(), self.qualified(qualified));
        }
        if let Some(type_ref) = self.find(node, "type_ref") {
            o.insert("type".into(), self.type_ref(type_ref));
        }
        let strings: Vec<String> = node
            .tokens()
            .filter(|t| self.parse.tokens[*t].name() == "STRING")
            .map(|t| decode_string(self.token_text(t)))
            .collect();
        if node.kind() == "files" {
            o.insert("files".into(), json!(strings));
        } else if let Some(title) = strings.first() {
            o.insert("title".into(), json!(title));
        }
        if let Some(value) = self.value_child(node).filter(|v| v.kind() != "settings_block") {
            o.insert("value".into(), self.value(value));
        }
        if let Some(when) = self.child_nodes(node).into_iter().find(|n| self.is_expression(n)) {
            o.insert("when".into(), self.expr(when));
        }
        if node.kind() == "marker"
            && let Some(end) = self
                .child_nodes(node)
                .into_iter()
                .find(|n| matches!(n.kind(), "source" | "target" | "mid"))
        {
            o.insert("end".into(), json!(end.kind()));
        }
        for source in ["capability_source", "named_source"] {
            if let Some(found) = self.find(node, source) {
                let value = match source {
                    "capability_source" => {
                        json!({"capability": self.qualified(self.find(found, "qualified").expect("a name"))})
                    }
                    _ => json!({"named": self.string_token(found).unwrap_or_default()}),
                };
                o.insert("source".into(), value);
            }
        }
        let (settings, doc) = self.settings_block(self.find(node, "settings_block"));
        if !settings.is_empty() {
            o.insert("settings".into(), Json::Array(settings));
        }
        let members: Vec<Json> = self
            .child_nodes(node)
            .into_iter()
            .filter(|m| self.is_member(m))
            .map(|m| self.generic(m))
            .collect();
        if !members.is_empty() {
            o.insert("members".into(), Json::Array(members));
        }
        self.with_doc(node, o, doc)
    }

    fn is_expression(&self, node: &Node) -> bool {
        node.kind().starts_with("cel_") && node.kind() != "cel_value" || node.kind() == "expr"
    }

    fn is_member(&self, node: &Node) -> bool {
        !matches!(
            node.kind(),
            "name"
                | "names"
                | "qualified"
                | "type_ref"
                | "settings_block"
                | "value"
                | "cel_value"
                | "attribute_ref"
                | "token_ref"
                | "param_ref"
                | "settings_map"
                | "value_list"
                | "extends"
                | "source"
                | "target"
                | "mid"
                | "capability_source"
                | "named_source"
        ) && !self.is_expression(node)
    }

    fn binding(&self, node: &Node) -> Json {
        let mut o = self.object("binding", node);
        o.insert("name".into(), self.rule_name(self.own_rule_name(node).expect("a name")));
        if let Some(family) = self.find(node, "declared_family") {
            o.insert(
                "family".into(),
                json!(self.name_text(self.find(family, "name").expect("a family"))),
            );
        }
        if let Some(family) = self.find(node, "capability_family") {
            o.insert("family".into(), json!("capability"));
            o.insert(
                "capability".into(),
                self.qualified(self.find(family, "qualified").expect("a capability")),
            );
        }
        let members: Vec<Json> = self
            .child_nodes(node)
            .into_iter()
            .filter(|m| !matches!(m.kind(), "rule_name" | "name" | "declared_family" | "capability_family"))
            .map(|m| self.binding_member(m))
            .collect();
        o.insert("members".into(), Json::Array(members));
        self.with_doc(node, o, None)
    }

    fn binding_member(&self, node: &Node) -> Json {
        match node.kind() {
            "element_rule" | "relation_rule" => {
                let mut o = self.object("rule", node);
                o.insert(
                    "ruleKind".into(),
                    json!(if node.kind() == "element_rule" {
                        "element"
                    } else {
                        "relation"
                    }),
                );
                o.insert(
                    "type".into(),
                    self.type_ref(self.find(node, "type_ref").expect("a type")),
                );
                if let Some(name) = self.own_rule_name(node) {
                    o.insert("as".into(), self.rule_name(name));
                }
                for place in ["at", "line"] {
                    if let Some(p) = self.find(node, place) {
                        o.insert(place.into(), json!(self.string_token(p).unwrap_or_default()));
                    }
                }
                let members: Vec<Json> = self
                    .child_nodes(node)
                    .into_iter()
                    .filter(|m| !matches!(m.kind(), "type_ref" | "rule_name" | "name" | "at" | "line"))
                    .map(|m| self.rule_member(m))
                    .collect();
                o.insert("members".into(), Json::Array(members));
                self.with_doc(node, o, None)
            }
            "claims_files" => {
                let mut o = self.object("claims", node);
                let files: Vec<String> = node
                    .tokens()
                    .filter(|t| self.parse.tokens[*t].name() == "STRING")
                    .map(|t| decode_string(self.token_text(t)))
                    .collect();
                o.insert("files".into(), json!(files));
                Json::Object(o)
            }
            "text_defaults" => {
                let mut o = self.object("text", node);
                let (settings, doc) = self.settings_block(self.find(node, "settings_block"));
                o.insert("settings".into(), Json::Array(settings));
                self.with_doc(node, o, doc)
            }
            "block_rule" | "section_rule" => {
                let kind = if node.kind() == "block_rule" {
                    "block"
                } else {
                    "section"
                };
                let mut o = self.object(kind, node);
                o.insert("name".into(), self.rule_name(self.own_rule_name(node).expect("a name")));
                if let Some(text) = self.string_token(node) {
                    o.insert(if kind == "block" { "line" } else { "heading" }.into(), json!(text));
                }
                let (settings, doc) = self.settings_block(self.find(node, "settings_block"));
                if !settings.is_empty() {
                    o.insert("settings".into(), Json::Array(settings));
                }
                self.with_doc(node, o, doc)
            }
            _ => self.generic(node),
        }
    }

    fn rule_member(&self, node: &Node) -> Json {
        // A rule's own setting (`readOnly: "…"`), as every construct's setting is in the tree.
        if node.kind() == "setting" {
            let mut o = self.object("setting", node);
            o.insert("setting".into(), self.setting(node));
            return Json::Object(o);
        }
        let kind = match node.kind() {
            "id_from" | "id" => "id",
            "attribute_binding" => "attribute",
            other => other,
        };
        let mut o = self.object(kind, node);
        if let Some(name) = self.find(node, "name") {
            o.insert("name".into(), self.name(name));
        }
        if let Some(slot) = self.find(node, "slot") {
            o.insert("from".into(), self.slot(slot));
        }
        if let Some(names) = self.own_rule_names(node) {
            o.insert("rules".into(), self.rule_names(names));
        }
        if node.kind() == "parent" {
            let names = self.find_all(node, "name");
            if let Some(slot) = names.first() {
                o.insert("slot".into(), self.name(slot));
                o.remove("name");
            }
        }
        if let Some(when) = self.child_nodes(node).into_iter().find(|n| self.is_expression(n)) {
            o.insert("when".into(), self.expr(when));
        }
        let block = self
            .find(node, "settings_block")
            .or_else(|| self.find(node, "attribute_binding_block"));
        let mut settings = Vec::new();
        let mut doc = None;
        if let Some(block) = block {
            if block.kind() == "attribute_binding_block" {
                for member in self.child_nodes(block) {
                    match member.kind() {
                        "setting" => settings.push(self.setting(member)),
                        "references" => {
                            let names = self.find_all(member, "name");
                            o.insert(
                                "references".into(),
                                json!({"rules": self.rule_names(self.own_rule_names(member).expect("rules")),
                                       "by": self.name(names.last().expect("by"))}),
                            );
                        }
                        "doc_member" => doc = Some(self.doc_text(member)),
                        _ => {}
                    }
                }
            } else {
                let (s, d) = self.settings_block(Some(block));
                settings = s;
                doc = d;
            }
        }
        if !settings.is_empty() {
            o.insert("settings".into(), Json::Array(settings));
        }
        self.with_doc(node, o, doc)
    }

    fn slot(&self, node: &Node) -> Json {
        let kind = self.name_text(self.find(node, "name").expect("a slot kind"));
        let mut o = Map::new();
        o.insert("kind".into(), json!(kind));
        let argument = self.find(node, "slot_argument");
        let holder = argument.unwrap_or(node);
        if let Some(text) = self.string_token(holder) {
            o.insert("argument".into(), json!(text));
        } else if let Some(int) = self.token_named(holder, "INT") {
            o.insert("argument".into(), number(self.token_text(int)));
        } else if let Some(cel) = self.find(node, "cel_slot") {
            o.insert("argument".into(), self.expr(self.child_nodes(cel)[0]));
        }
        o.insert("span".into(), self.node_span(node));
        Json::Object(o)
    }
}

/// A setting's key from its `KEY` token: the name without its colon and the spaces before it.
pub fn key_text(token: &str) -> String {
    token.trim_end_matches(':').trim_end().to_owned()
}

fn number(text: &str) -> Json {
    if let Some(hex) = text.strip_prefix("0x") {
        return i64::from_str_radix(hex, 16).map(Json::from).unwrap_or(Json::Null);
    }
    if let Ok(int) = text.parse::<i64>() {
        return Json::from(int);
    }
    text.parse::<f64>()
        .ok()
        .and_then(serde_json::Number::from_f64)
        .map(Json::Number)
        .unwrap_or(Json::Null)
}

fn negate(value: Json) -> Json {
    match value.as_i64() {
        Some(i) => Json::from(-i),
        None => value
            .as_f64()
            .and_then(|f| serde_json::Number::from_f64(-f))
            .map(Json::Number)
            .unwrap_or(Json::Null),
    }
}

/// A string literal's value, as CEL reads it (ATD §3.4): raw strings unescaped, triple quotes across lines, CEL's
/// escapes.
pub fn decode_string(text: &str) -> String {
    let (raw, rest) = match text.strip_prefix(['r', 'R']) {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let body = if rest.starts_with("\"\"\"") || rest.starts_with("'''") {
        &rest[3..rest.len().saturating_sub(3).max(3)]
    } else if rest.len() >= 2 {
        &rest[1..rest.len() - 1]
    } else {
        rest
    };
    if raw {
        return body.to_owned();
    }
    let chars: Vec<char> = body.chars().collect();
    let mut out = String::with_capacity(body.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c != '\\' || i + 1 >= chars.len() {
            out.push(c);
            i += 1;
            continue;
        }
        let next = chars[i + 1];
        i += 2;
        match next {
            'a' => out.push('\u{7}'),
            'b' => out.push('\u{8}'),
            'f' => out.push('\u{c}'),
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            'v' => out.push('\u{b}'),
            'x' | 'u' | 'U' => {
                let digits = match next {
                    'x' => 2,
                    'u' => 4,
                    _ => 8,
                };
                let hex: String = chars.iter().skip(i).take(digits).collect();
                if let Some(c) = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                    out.push(c);
                }
                i += digits;
            }
            '0'..='3' => {
                let octal: String = std::iter::once(next)
                    .chain(chars.iter().skip(i).take(2).copied())
                    .collect();
                if let Some(c) = u32::from_str_radix(&octal, 8).ok().and_then(char::from_u32) {
                    out.push(c);
                }
                i += 2;
            }
            other => out.push(other),
        }
    }
    out
}

#[allow(dead_code)]
fn children_of(node: &Node) -> &[Child] {
    &node.children
}
