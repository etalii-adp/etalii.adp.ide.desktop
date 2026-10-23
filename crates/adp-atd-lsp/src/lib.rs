//! The ATD language server core (etalii.adp spec 035, research R10, contracts/language-server.md): a transport-free
//! [`Server::handle`] that takes a Language Server Protocol message and gives the messages to send back, over the
//! open documents of a workspace and the descriptions they belong to. Diagnostics are the checker's findings, by the
//! same code (FR-054). The transports are in [`transport`]: in process for the desktop host, stdio for IntelliJ, and
//! WebAssembly for VS Code.

mod complete;
mod hover;
pub mod index;
mod keywords;
mod positions;
mod semantic;
pub mod transport;

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use adp_atd_check::{Checked, Files, FsFiles, Overlay, Severity, check};
use serde_json::{Value as Json, json};

pub use positions::Encoding;

/// The files a server reads besides its open documents. Natively the server may run on a thread of its own, so they
/// cross threads; in WebAssembly they are the JavaScript host's, which stay on its one thread.
#[cfg(not(target_arch = "wasm32"))]
pub type HostFiles = Box<dyn Files + Send>;
#[cfg(target_arch = "wasm32")]
pub type HostFiles = Box<dyn Files>;

/// The language server: its documents, the encoding it agreed with the client, and the descriptions waiting to be
/// checked again.
pub struct Server {
    documents: BTreeMap<PathBuf, (String, i64)>,
    encoding: Encoding,
    /// Files whose descriptions have changed since diagnostics were last published.
    dirty: BTreeSet<PathBuf>,
    /// Files diagnostics were published for, by description root, so that a file that no longer has findings gets
    /// an empty list.
    published: BTreeMap<PathBuf, BTreeSet<PathBuf>>,
    files: HostFiles,
    shutdown: bool,
    /// The last check of each file's description, with the generation of the open documents it was made for: the
    /// requests between two edits reuse the check the diagnostics made (SC-006).
    checked: RefCell<BTreeMap<PathBuf, (u64, Arc<Checked>)>>,
    /// Counts the changes to the open documents.
    generation: u64,
}

impl Default for Server {
    fn default() -> Self {
        Server::new()
    }
}

/// A response to request `id`.
pub fn response(id: &Json, result: Json) -> Json {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

fn error_response(id: &Json, code: i64, message: &str) -> Json {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

fn notification(method: &str, params: Json) -> Json {
    json!({"jsonrpc": "2.0", "method": method, "params": params})
}

/// A file URI's path.
pub fn uri_to_path(uri: &str) -> PathBuf {
    let rest = uri.strip_prefix("file://").unwrap_or(uri);
    let raw = rest.as_bytes();
    let mut bytes = Vec::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        if raw[i] == b'%'
            && i + 2 < raw.len()
            && let Ok(byte) = u8::from_str_radix(&rest[i + 1..i + 3], 16)
        {
            bytes.push(byte);
            i += 3;
            continue;
        }
        bytes.push(raw[i]);
        i += 1;
    }
    let decoded = String::from_utf8_lossy(&bytes).into_owned();
    // `file:///C:/x` on Windows.
    let windows = decoded.len() > 3 && decoded.as_bytes()[0] == b'/' && decoded.as_bytes()[2] == b':';
    PathBuf::from(if windows { decoded[1..].to_owned() } else { decoded })
}

/// A path's file URI.
pub fn path_to_uri(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    let mut out = String::from("file://");
    if !text.starts_with('/') {
        out.push('/');
    }
    for c in text.chars() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '/' | '-' | '_' | '.' | '~' | ':' => out.push(c),
            other => {
                let mut buffer = [0u8; 4];
                for byte in other.encode_utf8(&mut buffer).bytes() {
                    out.push_str(&format!("%{byte:02X}"));
                }
            }
        }
    }
    out
}

/// The open texts, over the files of the platform.
struct OpenFiles<'a> {
    documents: &'a BTreeMap<PathBuf, (String, i64)>,
    under: &'a dyn Files,
}

impl Files for OpenFiles<'_> {
    fn read(&self, path: &Path) -> Option<String> {
        self.documents
            .get(path)
            .map(|(t, _)| t.clone())
            .or_else(|| self.under.read(path))
    }

    fn list(&self, folder: &Path) -> Vec<PathBuf> {
        let overlay = Overlay {
            open: self
                .documents
                .iter()
                .map(|(p, (t, _))| (p.clone(), t.clone()))
                .collect(),
            under: self.under,
        };
        overlay.list(folder)
    }
}

/// A request at a position.
pub struct Request {
    pub path: PathBuf,
    pub text: String,
    pub offset: usize,
    pub params: Json,
}

impl Server {
    pub fn new() -> Self {
        Server::with_files(Box::new(FsFiles))
    }

    /// A server over other files than the file system's: those a client hands it (WebAssembly).
    pub fn with_files(files: HostFiles) -> Self {
        Server {
            documents: BTreeMap::new(),
            encoding: Encoding::Utf16,
            dirty: BTreeSet::new(),
            published: BTreeMap::new(),
            files,
            shutdown: false,
            checked: RefCell::new(BTreeMap::new()),
            generation: 0,
        }
    }

    pub fn encoding(&self) -> Encoding {
        self.encoding
    }

    /// The text of an open document, or of the file.
    pub fn text(&self, path: &Path) -> Option<String> {
        self.documents
            .get(path)
            .map(|(t, _)| t.clone())
            .or_else(|| self.files.read(path))
    }

    /// Checks the description a file belongs to, over the open texts; the last check is reused until a document
    /// changes.
    pub fn check(&self, path: &Path) -> Arc<Checked> {
        if let Some((generation, checked)) = self.checked.borrow().get(path)
            && *generation == self.generation
        {
            return checked.clone();
        }
        let files = OpenFiles {
            documents: &self.documents,
            under: self.files.as_ref(),
        };
        let checked = Arc::new(check(&files, path));
        self.checked
            .borrow_mut()
            .insert(path.to_path_buf(), (self.generation, checked.clone()));
        checked
    }

    /// A document changed: every check made before is stale.
    fn changed(&mut self) {
        self.generation += 1;
        self.checked.get_mut().clear();
    }

    /// Whether descriptions wait to be checked again: the transport calls [`Server::flush`] 150 ms after the last
    /// change.
    pub fn has_pending(&self) -> bool {
        !self.dirty.is_empty()
    }

    /// Handles one message; gives the messages to send.
    pub fn handle(&mut self, message: &Json) -> Vec<Json> {
        let method = message["method"].as_str().unwrap_or("");
        let params = &message["params"];
        if let Some(id) = message.get("id").filter(|_| !method.is_empty()) {
            if self.shutdown {
                return vec![error_response(id, -32600, "The server is shutting down.")];
            }
            let result = match method {
                "initialize" => Some(self.initialize(params)),
                "shutdown" => {
                    self.shutdown = true;
                    Some(Json::Null)
                }
                "textDocument/completion" => Some(self.with_position(params, complete::completion)),
                "textDocument/hover" => Some(self.with_position(params, hover::hover)),
                "textDocument/definition" => Some(self.with_position(params, Self::definition)),
                "textDocument/references" => Some(self.with_position(params, Self::references)),
                "textDocument/documentHighlight" => Some(self.with_position(params, Self::highlights)),
                "textDocument/prepareRename" => Some(self.with_position(params, Self::prepare_rename)),
                "textDocument/rename" => Some(self.rename(params)),
                "textDocument/formatting" => Some(self.formatting(params)),
                "textDocument/documentSymbol" => Some(self.document_symbols(params)),
                "textDocument/foldingRange" => Some(self.folding(params)),
                "textDocument/codeAction" => Some(self.code_actions(params)),
                "textDocument/semanticTokens/full" => Some(self.semantic_tokens(params, None)),
                "textDocument/semanticTokens/range" => Some(self.semantic_tokens(params, Some(&params["range"]))),
                "atd/check" => Some(self.atd_check(params)),
                "atd/tree" => Some(self.atd_tree(params)),
                _ => None,
            };
            return vec![match result {
                Some(result) => response(id, result),
                None => error_response(id, -32601, &format!("`{method}` is not a method of this server.")),
            }];
        }
        match method {
            "textDocument/didOpen" => {
                let document = &params["textDocument"];
                let path = uri_to_path(document["uri"].as_str().unwrap_or(""));
                self.documents.insert(
                    path.clone(),
                    (
                        document["text"].as_str().unwrap_or("").to_owned(),
                        document["version"].as_i64().unwrap_or(0),
                    ),
                );
                self.dirty.insert(path);
                self.changed();
                self.flush()
            }
            "textDocument/didChange" => {
                let path = uri_to_path(params["textDocument"]["uri"].as_str().unwrap_or(""));
                let version = params["textDocument"]["version"].as_i64().unwrap_or(0);
                let mut text = self.text(&path).unwrap_or_default();
                for change in params["contentChanges"].as_array().into_iter().flatten() {
                    match change.get("range") {
                        None => text = change["text"].as_str().unwrap_or("").to_owned(),
                        Some(range) => {
                            let start = positions::offset(&text, &range["start"], self.encoding);
                            let end = positions::offset(&text, &range["end"], self.encoding).max(start);
                            text.replace_range(start..end, change["text"].as_str().unwrap_or(""));
                        }
                    }
                }
                self.documents.insert(path.clone(), (text, version));
                self.dirty.insert(path);
                self.changed();
                Vec::new()
            }
            "textDocument/didSave" => {
                let path = uri_to_path(params["textDocument"]["uri"].as_str().unwrap_or(""));
                self.dirty.insert(path);
                self.changed();
                self.flush()
            }
            "textDocument/didClose" => {
                let path = uri_to_path(params["textDocument"]["uri"].as_str().unwrap_or(""));
                self.documents.remove(&path);
                self.changed();
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    fn initialize(&mut self, params: &Json) -> Json {
        let offered: Vec<&str> = params["capabilities"]["general"]["positionEncodings"]
            .as_array()
            .map(|a| a.iter().filter_map(Json::as_str).collect())
            .unwrap_or_default();
        self.encoding = if offered.is_empty() || offered.contains(&"utf-16") {
            Encoding::Utf16
        } else if offered.contains(&"utf-32") {
            Encoding::Utf32
        } else if offered.contains(&"utf-8") {
            Encoding::Utf8
        } else {
            Encoding::Utf16
        };
        json!({
            "capabilities": {
                "positionEncoding": self.encoding.name(),
                "textDocumentSync": {"openClose": true, "change": 2, "save": {"includeText": false}},
                "completionProvider": {"triggerCharacters": [".", "@", "$", " "]},
                "hoverProvider": true,
                "definitionProvider": true,
                "referencesProvider": true,
                "documentHighlightProvider": true,
                "renameProvider": {"prepareProvider": true},
                "documentFormattingProvider": true,
                "documentSymbolProvider": true,
                "foldingRangeProvider": true,
                "codeActionProvider": {"codeActionKinds": ["quickfix"]},
                "semanticTokensProvider": {
                    "legend": {"tokenTypes": semantic::TYPES, "tokenModifiers": semantic::MODIFIERS},
                    "full": true,
                    "range": true
                }
            },
            "serverInfo": {"name": "atd-lsp", "version": env!("CARGO_PKG_VERSION")}
        })
    }

    /// Checks the descriptions of the changed files and publishes diagnostics for every file of each (story 5,
    /// scenario 3).
    pub fn flush(&mut self) -> Vec<Json> {
        let dirty: Vec<PathBuf> = std::mem::take(&mut self.dirty).into_iter().collect();
        let mut done: BTreeSet<PathBuf> = BTreeSet::new();
        let mut out = Vec::new();
        for path in dirty {
            if done.contains(&path) {
                continue;
            }
            let checked = self.check(&path);
            let root = checked
                .description
                .files
                .first()
                .map(|f| f.path.clone())
                .unwrap_or_else(|| path.clone());
            let mut by_file: BTreeMap<PathBuf, Vec<Json>> = BTreeMap::new();
            for unit in &checked.description.files {
                done.insert(unit.path.clone());
                by_file.entry(unit.path.clone()).or_default();
            }
            by_file.entry(path.clone()).or_default();
            for finding in &checked.findings {
                let file = checked
                    .description
                    .files
                    .iter()
                    .find(|f| f.name == finding.file)
                    .map(|f| f.path.clone())
                    .unwrap_or_else(|| PathBuf::from(&finding.file));
                let text = self.text(&file).unwrap_or_default();
                by_file.entry(file).or_default().push(self.diagnostic(&text, finding));
            }
            let now: BTreeSet<PathBuf> = by_file.keys().cloned().collect();
            let previous = self.published.insert(root, now.clone()).unwrap_or_default();
            for stale in previous.difference(&now) {
                by_file.insert(stale.clone(), Vec::new());
            }
            for (file, diagnostics) in by_file {
                let version = self.documents.get(&file).map(|(_, v)| json!(v)).unwrap_or(Json::Null);
                out.push(notification(
                    "textDocument/publishDiagnostics",
                    json!({"uri": path_to_uri(&file), "version": version, "diagnostics": diagnostics}),
                ));
            }
        }
        out
    }

    fn diagnostic(&self, text: &str, finding: &adp_atd_check::Finding) -> Json {
        let range = positions::range(text, finding.start, finding.end.max(finding.start), self.encoding);
        let severity = match finding.severity {
            Severity::Error => 1,
            Severity::Warning => 2,
            Severity::Information => 3,
        };
        let fixes: Vec<Json> = finding
            .fixes
            .iter()
            .map(|f| {
                let edits: Vec<Json> = f
                    .edits
                    .iter()
                    .map(|e| json!({"file": e.file, "start": e.start, "end": e.end, "text": e.text}))
                    .collect();
                json!({"title": f.title, "edits": edits})
            })
            .collect();
        json!({
            "range": range, "severity": severity, "code": finding.code, "source": "atd", "message": finding.message,
            "data": {"fixes": fixes, "suggestions": finding.suggestions}
        })
    }

    /// Runs a request that names a position in a document.
    fn with_position(&self, params: &Json, f: fn(&Server, &Request) -> Json) -> Json {
        let path = uri_to_path(params["textDocument"]["uri"].as_str().unwrap_or(""));
        let Some(text) = self.text(&path) else {
            return Json::Null;
        };
        let offset = positions::offset(&text, &params["position"], self.encoding);
        let request = Request {
            path,
            text,
            offset,
            params: params.clone(),
        };
        f(self, &request)
    }

    pub(crate) fn location(&self, file: &str, start: usize, end: usize) -> Json {
        let path = PathBuf::from(file);
        let text = self.text(&path).unwrap_or_default();
        json!({"uri": path_to_uri(&path), "range": positions::range(&text, start, end, self.encoding)})
    }

    fn definition(&self, request: &Request) -> Json {
        let checked = self.check(&request.path);
        let index = index::build(&checked);
        let name = file_name(&checked, &request.path);
        match index
            .at(&name, request.offset)
            .and_then(|o| index.declaration(&o.symbol))
        {
            Some(declaration) => self.location(&declaration.file, declaration.start, declaration.end),
            None => Json::Null,
        }
    }

    fn references(&self, request: &Request) -> Json {
        let checked = self.check(&request.path);
        let index = index::build(&checked);
        let name = file_name(&checked, &request.path);
        let include_declaration = request.params["context"]["includeDeclaration"]
            .as_bool()
            .unwrap_or(true);
        match index.at(&name, request.offset) {
            Some(occurrence) => Json::Array(
                index
                    .of(&occurrence.symbol)
                    .filter(|o| include_declaration || !o.declaration)
                    .map(|o| self.location(&o.file, o.start, o.end))
                    .collect(),
            ),
            None => Json::Array(Vec::new()),
        }
    }

    fn highlights(&self, request: &Request) -> Json {
        let checked = self.check(&request.path);
        let index = index::build(&checked);
        let name = file_name(&checked, &request.path);
        match index.at(&name, request.offset) {
            Some(occurrence) => Json::Array(
                index
                    .of(&occurrence.symbol)
                    .filter(|o| o.file == name)
                    .map(|o| {
                        let kind = if o.declaration { 3 } else { 2 };
                        json!({"range": positions::range(&request.text, o.start, o.end, self.encoding), "kind": kind})
                    })
                    .collect(),
            ),
            None => Json::Null,
        }
    }

    fn prepare_rename(&self, request: &Request) -> Json {
        let checked = self.check(&request.path);
        let index = index::build(&checked);
        let name = file_name(&checked, &request.path);
        match index.at(&name, request.offset) {
            Some(o) if !o.symbol.starts_with("capability:") => json!({
                "range": positions::range(&request.text, o.start, o.end, self.encoding),
                "placeholder": &request.text[o.start..o.end]
            }),
            _ => Json::Null,
        }
    }

    /// Renames a name in every file of its description: its uses in CEL included, the contents of strings not
    /// (FR-033: one splice per use).
    fn rename(&self, params: &Json) -> Json {
        let path = uri_to_path(params["textDocument"]["uri"].as_str().unwrap_or(""));
        let Some(text) = self.text(&path) else {
            return Json::Null;
        };
        let offset = positions::offset(&text, &params["position"], self.encoding);
        let new_name = params["newName"].as_str().unwrap_or("");
        let checked = self.check(&path);
        let index = index::build(&checked);
        let name = file_name(&checked, &path);
        let Some(occurrence) = index.at(&name, offset) else {
            return Json::Null;
        };
        let mut changes: BTreeMap<String, Vec<Json>> = BTreeMap::new();
        for o in index.of(&occurrence.symbol) {
            let file = PathBuf::from(&o.file);
            let file_text = self.text(&file).unwrap_or_default();
            changes.entry(path_to_uri(&file)).or_default().push(
                json!({"range": positions::range(&file_text, o.start, o.end, self.encoding), "newText": new_name}),
            );
        }
        json!({"changes": changes})
    }

    fn formatting(&self, params: &Json) -> Json {
        let path = uri_to_path(params["textDocument"]["uri"].as_str().unwrap_or(""));
        let Some(text) = self.text(&path) else {
            return Json::Null;
        };
        match adp_atd_format::format(&text) {
            Ok(formatted) if formatted != text => {
                json!([{"range": positions::range(&text, 0, text.len(), self.encoding), "newText": formatted}])
            }
            Ok(_) => json!([]),
            Err(_) => Json::Null,
        }
    }

    fn document_symbols(&self, params: &Json) -> Json {
        let path = uri_to_path(params["textDocument"]["uri"].as_str().unwrap_or(""));
        let Some(text) = self.text(&path) else { return json!([]) };
        let parsed = adp_atd_syntax::parse(&text);
        let Ok(tree) = adp_atd_syntax::lower(&parsed, &path.to_string_lossy()) else {
            return json!([]);
        };
        let construct = &tree["description"];
        let symbols: Vec<Json> = construct["members"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|m| self.symbol_of(&text, m))
            .collect();
        if construct["kind"] == "tool" {
            let origin = construct["origin"].as_str().unwrap_or("");
            let range = self.json_range(&text, construct);
            return json!([{"name": origin, "detail": construct["toolKind"], "kind": 2, "range": range,
                           "selectionRange": range, "children": symbols}]);
        }
        Json::Array(symbols)
    }

    fn json_range(&self, text: &str, node: &Json) -> Json {
        let start = node["span"]["start"].as_u64().unwrap_or(0) as usize;
        let end = node["span"]["end"].as_u64().unwrap_or(0) as usize;
        positions::range(text, start, end, self.encoding)
    }

    fn symbol_of(&self, text: &str, node: &Json) -> Option<Json> {
        let kind = node["kind"].as_str().unwrap_or("");
        let named = |prefix: &str| node["name"]["text"].as_str().map(|n| format!("{prefix}{n}"));
        let (name, symbol_kind) = match kind {
            "model" => ("model".to_owned(), 3),
            "element" | "relation" => (named("")?, 5),
            "enum" => (named("")?, 10),
            "attribute" => (named("")?, 7),
            "fn" => (named("")?, 12),
            "operation" | "hook" | "constraint" | "reason" | "form" | "binding" | "viewpoint" => {
                (named(&format!("{kind} "))?, 6)
            }
            "notation" | "toolbox" | "layout" | "coordinates" | "constraints" => (kind.to_owned(), 3),
            "tableSurface" | "hierarchySurface" | "textSurface" => ("surface".to_owned(), 3),
            "node" | "edge" => {
                let names: Vec<&str> = node["names"]
                    .as_array()?
                    .iter()
                    .filter_map(|n| n["text"].as_str())
                    .collect();
                (format!("{kind} {}", names.join(", ")), 13)
            }
            "rule" => (
                format!(
                    "{} {}",
                    node["ruleKind"].as_str().unwrap_or(""),
                    node["type"]["name"]["text"].as_str()?
                ),
                6,
            ),
            _ => return None,
        };
        let range = self.json_range(text, node);
        let selection = node
            .get("name")
            .map(|n| self.json_range(text, n))
            .unwrap_or_else(|| range.clone());
        let children: Vec<Json> = node["members"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|m| self.symbol_of(text, m))
            .collect();
        Some(
            json!({"name": name, "kind": symbol_kind, "range": range, "selectionRange": selection, "children": children}),
        )
    }

    /// Blocks that span lines, and runs of comment lines.
    fn folding(&self, params: &Json) -> Json {
        let path = uri_to_path(params["textDocument"]["uri"].as_str().unwrap_or(""));
        let Some(text) = self.text(&path) else { return json!([]) };
        let parsed = adp_atd_syntax::parse(&text);
        let lines = adp_atd_syntax::LineIndex::new(&text);
        let mut ranges = Vec::new();
        let mut stack: Vec<usize> = Vec::new();
        let mut comments: Option<(usize, usize)> = None;
        let close_comments = |run: Option<(usize, usize)>, ranges: &mut Vec<Json>| {
            if let Some((start, end)) = run
                && end > start
            {
                ranges.push(json!({"startLine": start - 1, "endLine": end - 1, "kind": "comment"}));
            }
        };
        for token in &parsed.tokens {
            let token_text = token.text(&text);
            let (line, _) = lines.position(&text, token.start);
            if token_text.starts_with("//") {
                comments = match comments {
                    Some((start, end)) if end + 1 == line => Some((start, line)),
                    other => {
                        close_comments(other, &mut ranges);
                        Some((line, line))
                    }
                };
                continue;
            }
            if token.kind != adp_atd_syntax::TokenKind::Normal {
                continue;
            }
            match token_text {
                "{" => stack.push(line),
                "}" => {
                    if let Some(start) = stack.pop()
                        && line > start + 1
                    {
                        ranges.push(json!({"startLine": start - 1, "endLine": line - 2}));
                    }
                }
                _ => {}
            }
        }
        close_comments(comments, &mut ranges);
        Json::Array(ranges)
    }

    /// The fixes of the diagnostics the client sends back, as quick fixes.
    fn code_actions(&self, params: &Json) -> Json {
        let mut actions = Vec::new();
        for diagnostic in params["context"]["diagnostics"].as_array().into_iter().flatten() {
            for fix in diagnostic["data"]["fixes"].as_array().into_iter().flatten() {
                let mut changes: BTreeMap<String, Vec<Json>> = BTreeMap::new();
                for edit in fix["edits"].as_array().into_iter().flatten() {
                    let file = PathBuf::from(edit["file"].as_str().unwrap_or(""));
                    let text = self.text(&file).unwrap_or_default();
                    let start = edit["start"].as_u64().unwrap_or(0) as usize;
                    let end = edit["end"].as_u64().unwrap_or(0) as usize;
                    changes.entry(path_to_uri(&file)).or_default().push(
                        json!({"range": positions::range(&text, start, end, self.encoding), "newText": edit["text"]}),
                    );
                }
                actions.push(json!({"title": fix["title"], "kind": "quickfix", "diagnostics": [diagnostic], "edit": {"changes": changes}}));
            }
        }
        Json::Array(actions)
    }

    fn semantic_tokens(&self, params: &Json, range: Option<&Json>) -> Json {
        let path = uri_to_path(params["textDocument"]["uri"].as_str().unwrap_or(""));
        let Some(text) = self.text(&path) else {
            return Json::Null;
        };
        let checked = self.check(&path);
        let index = index::build(&checked);
        let name = file_name(&checked, &path);
        let bounds = range.map(|r| {
            (
                positions::offset(&text, &r["start"], self.encoding),
                positions::offset(&text, &r["end"], self.encoding),
            )
        });
        json!({"data": semantic::encode(&text, &name, &index, self.encoding, bounds)})
    }

    fn atd_check(&self, params: &Json) -> Json {
        let path = uri_to_path(params["uri"].as_str().unwrap_or(""));
        adp_atd_check::findings_json(&self.check(&path).findings)
    }

    fn atd_tree(&self, params: &Json) -> Json {
        let path = uri_to_path(params["uri"].as_str().unwrap_or(""));
        let Some(text) = self.text(&path) else {
            return Json::Null;
        };
        let parsed = adp_atd_syntax::parse(&text);
        adp_atd_syntax::lower(&parsed, &path.to_string_lossy()).unwrap_or(Json::Null)
    }
}

/// The name the checked description gives the file at `path`.
pub(crate) fn file_name(checked: &Checked, path: &Path) -> String {
    checked
        .description
        .files
        .iter()
        .find(|f| f.path == path)
        .map(|f| f.name.clone())
        .unwrap_or_else(|| path.to_string_lossy().replace('\\', "/"))
}
