//! The text editor's language client for `.atd` (etalii.adp spec 035, T105, contracts/language-server.md): the ATD
//! language server runs on a worker thread of this process over `lsp-server`'s in-memory connection, so no socket is
//! opened (the desktop host's Principle I). Its diagnostics are the editor's underlines and its status line, its
//! semantic tokens colour over the `syntect` colouring of the vendored `atd.sublime-syntax`, and its completion list,
//! hover, definition, references, rename, formatting and outline are the editor's, each drawn with egui widgets that
//! AccessKit hands to screen readers.
//!
//! The client offers only UTF-32 positions, so the server counts characters as the editor's buffer does.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use adp_atd_lsp::{path_to_uri, uri_to_path};
use adp_theme::Theme;
use eframe::egui::accesskit::Role;
use eframe::egui::{self, Color32, Key, RichText};
use lsp_server::{Connection, Message};
use serde_json::{Value as Json, json};

use crate::documents::OpenText;
use crate::editor::{Command, Completion, Mark, MarkLevel, SemanticRun};
use crate::highlight::Highlighter;
use crate::theme::color;

/// How long a request may take before the editor gives up on it.
const TIMEOUT: Duration = Duration::from_secs(5);
/// How long the pointer rests on a name before its hover shows.
const HOVER_DELAY: Duration = Duration::from_millis(500);

/// A diagnostic of the server, by lines and characters from 0.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub start: (usize, usize),
    pub end: (usize, usize),
    pub level: MarkLevel,
    pub code: String,
    pub message: String,
}

/// A place in a file, by lines and characters from 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    pub path: PathBuf,
    pub start: (usize, usize),
    pub end: (usize, usize),
}

/// An edit of a file, by lines and characters from 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    pub start: (usize, usize),
    pub end: (usize, usize),
    pub text: String,
}

/// A symbol of the outline, at its depth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    pub detail: String,
    pub depth: usize,
    pub start: (usize, usize),
}

/// A semantic token: its line, its characters and its type's name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub line: usize,
    pub start: usize,
    pub length: usize,
    pub kind: String,
}

/// The client's end of the connection, the documents it has opened, and the diagnostics the server sent.
pub struct Client {
    connection: Connection,
    thread: Option<JoinHandle<()>>,
    next_id: i32,
    /// Per open document: the version sent and the buffer revision it was.
    versions: BTreeMap<PathBuf, (i32, u64)>,
    diagnostics: BTreeMap<PathBuf, Vec<Diagnostic>>,
    /// Documents whose diagnostics came since they were last taken.
    fresh: BTreeSet<PathBuf>,
    /// Documents changed since their diagnostics last came.
    waiting: BTreeSet<PathBuf>,
    legend: Vec<String>,
}

static CLIENT: OnceLock<Mutex<Client>> = OnceLock::new();

/// Tells the server a `.atd` tab closed, when the client was started: the file on disk counts again.
pub fn closed(path: &Path) {
    if let Some(client) = CLIENT.get() {
        client.lock().unwrap_or_else(|p| p.into_inner()).close(path);
    }
}

/// The one client of the window, started when the first `.atd` file opens.
pub fn client() -> MutexGuard<'static, Client> {
    CLIENT
        .get_or_init(|| Mutex::new(Client::start()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// A document as the server names it back: the path its URI gives, so that a path the client holds and a path in the
/// server's messages are one key, whatever separators or prefixes the platform wrote the first with.
fn key(path: &Path) -> PathBuf {
    uri_to_path(&path_to_uri(path))
}

fn position(line: usize, character: usize) -> Json {
    json!({"line": line, "character": character})
}

fn point(value: &Json) -> (usize, usize) {
    (
        value["line"].as_u64().unwrap_or(0) as usize,
        value["character"].as_u64().unwrap_or(0) as usize,
    )
}

fn edits_of(value: &Json) -> Vec<TextEdit> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .map(|e| TextEdit {
            start: point(&e["range"]["start"]),
            end: point(&e["range"]["end"]),
            text: e["newText"].as_str().unwrap_or("").to_owned(),
        })
        .collect()
}

fn location_of(value: &Json) -> Option<Location> {
    Some(Location {
        path: uri_to_path(value["uri"].as_str()?),
        start: point(&value["range"]["start"]),
        end: point(&value["range"]["end"]),
    })
}

impl Client {
    /// Starts the server on its thread and initializes it, offering UTF-32 positions only.
    pub fn start() -> Client {
        let (connection, thread) = adp_atd_lsp::transport::spawn_in_memory();
        let mut client = Client {
            connection,
            thread: Some(thread),
            next_id: 0,
            versions: BTreeMap::new(),
            diagnostics: BTreeMap::new(),
            fresh: BTreeSet::new(),
            waiting: BTreeSet::new(),
            legend: Vec::new(),
        };
        let result = client.request(
            "initialize",
            json!({"processId": null, "capabilities": {"general": {"positionEncodings": ["utf-32"]}}}),
        );
        client.legend = result["capabilities"]["semanticTokensProvider"]["legend"]["tokenTypes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|t| t.as_str().map(str::to_owned))
            .collect();
        client.notify("initialized", json!({}));
        client
    }

    fn send(&self, value: Json) {
        if let Ok(message) = serde_json::from_value::<Message>(value) {
            let _ = self.connection.sender.send(message);
        }
    }

    fn notify(&self, method: &str, params: Json) {
        self.send(json!({"jsonrpc": "2.0", "method": method, "params": params}));
    }

    /// Sends a request and waits for its response, keeping the notifications that come before it.
    pub fn request(&mut self, method: &str, params: Json) -> Json {
        self.next_id += 1;
        let id = self.next_id;
        self.send(json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        let deadline = Instant::now() + TIMEOUT;
        while let Some(left) = deadline.checked_duration_since(Instant::now()) {
            match self.connection.receiver.recv_timeout(left) {
                Ok(Message::Response(response)) if response.id == id.into() => {
                    return response.result.unwrap_or(Json::Null);
                }
                Ok(Message::Notification(notification)) => self.receive(&notification.method, notification.params),
                Ok(_) => {}
                Err(_) => break,
            }
        }
        Json::Null
    }

    fn receive(&mut self, method: &str, params: Json) {
        if method != "textDocument/publishDiagnostics" {
            return;
        }
        let path = uri_to_path(params["uri"].as_str().unwrap_or(""));
        let diagnostics = params["diagnostics"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|d| Diagnostic {
                start: point(&d["range"]["start"]),
                end: point(&d["range"]["end"]),
                level: match d["severity"].as_u64() {
                    Some(1) => MarkLevel::Error,
                    Some(2) => MarkLevel::Warning,
                    _ => MarkLevel::Info,
                },
                code: d["code"].as_str().unwrap_or("").to_owned(),
                message: d["message"].as_str().unwrap_or("").to_owned(),
            })
            .collect();
        // Diagnostics of an earlier version than the one last sent are not yet the change's.
        let current = self.versions.get(&path).map(|(version, _)| i64::from(*version));
        if params["version"]
            .as_i64()
            .zip(current)
            .is_none_or(|(published, sent)| published >= sent)
        {
            self.waiting.remove(&path);
        }
        self.fresh.insert(path.clone());
        self.diagnostics.insert(path, diagnostics);
    }

    /// Takes the notifications that have come.
    pub fn pump(&mut self) {
        while let Ok(message) = self.connection.receiver.try_recv() {
            if let Message::Notification(notification) = message {
                self.receive(&notification.method, notification.params);
            }
        }
    }

    /// Waits up to `limit` for the diagnostics of a changed document, then until the server has been quiet for
    /// `quiet`: the diagnostics of the other files of its description come in the same flush.
    #[cfg(test)]
    pub fn wait(&mut self, path: &Path, limit: Duration) {
        let path = &key(path);
        let deadline = Instant::now() + limit;
        while self.waiting.contains(path)
            && let Some(left) = deadline.checked_duration_since(Instant::now())
        {
            match self.connection.receiver.recv_timeout(left) {
                Ok(Message::Notification(notification)) => self.receive(&notification.method, notification.params),
                Ok(_) => {}
                Err(_) => break,
            }
        }
        while let Ok(message) = self.connection.receiver.recv_timeout(Duration::from_millis(200)) {
            if let Message::Notification(notification) = message {
                self.receive(&notification.method, notification.params);
            }
        }
    }

    /// The codes of the diagnostics last published for every file of the description `path` belongs to, sorted: the
    /// root `<name>.atd` and its parts `<name>.<part>.atd` beside it (ATD §5), as the walk counts them.
    #[cfg(test)]
    pub fn description_codes(&self, path: &Path) -> Vec<String> {
        let path = &key(path);
        let name = |p: &Path| {
            p.file_name()
                .map(|n| n.to_string_lossy().split('.').next().unwrap_or("").to_owned())
        };
        let mut codes: Vec<String> = self
            .diagnostics
            .iter()
            .filter(|(p, _)| p.parent() == path.parent() && name(p) == name(path))
            .flat_map(|(_, d)| d.iter().map(|d| d.code.clone()))
            .collect();
        codes.sort();
        codes
    }

    /// Gives the server a document's text, when it is new to it or its revision changed.
    pub fn sync(&mut self, path: &Path, text: &str, revision: u64) {
        let path = &key(path);
        let uri = path_to_uri(path);
        match self.versions.get(path).copied() {
            None => {
                self.notify(
                    "textDocument/didOpen",
                    json!({"textDocument": {"uri": uri, "languageId": "atd", "version": 1, "text": text}}),
                );
                self.versions.insert(path.to_path_buf(), (1, revision));
            }
            Some((_, sent)) if sent == revision => return,
            Some((version, _)) => {
                self.notify(
                    "textDocument/didChange",
                    json!({"textDocument": {"uri": uri, "version": version + 1}, "contentChanges": [{"text": text}]}),
                );
                self.versions.insert(path.to_path_buf(), (version + 1, revision));
            }
        }
        self.waiting.insert(path.to_path_buf());
    }

    /// Tells the server a document was saved, which publishes its description's diagnostics again.
    #[cfg(test)]
    pub fn saved(&mut self, path: &Path) {
        let path = &key(path);
        if self.versions.contains_key(path) {
            self.notify(
                "textDocument/didSave",
                json!({"textDocument": {"uri": path_to_uri(path)}}),
            );
            self.waiting.insert(path.to_path_buf());
        }
    }

    /// Tells the server a document's tab closed: the file on disk counts again.
    pub fn close(&mut self, path: &Path) {
        let path = &key(path);
        if self.versions.remove(path).is_some() {
            self.notify(
                "textDocument/didClose",
                json!({"textDocument": {"uri": path_to_uri(path)}}),
            );
        }
        self.waiting.remove(path);
    }

    /// Whether a changed document's diagnostics have yet to come.
    pub fn waiting(&self, path: &Path) -> bool {
        let path = &key(path);
        self.waiting.contains(path)
    }

    /// The diagnostics of a document, when new ones came since this was last asked.
    pub fn take_diagnostics(&mut self, path: &Path) -> Option<Vec<Diagnostic>> {
        let path = &key(path);
        self.pump();
        self.fresh
            .remove(path)
            .then(|| self.diagnostics.get(path).cloned().unwrap_or_default())
    }

    fn at(path: &Path, at: (usize, usize)) -> Json {
        json!({"textDocument": {"uri": path_to_uri(path)}, "position": position(at.0, at.1)})
    }

    /// The labels the server completes with at a position.
    pub fn completion(&mut self, path: &Path, at: (usize, usize)) -> Vec<String> {
        let result = self.request("textDocument/completion", Self::at(path, at));
        result["items"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|i| i["label"].as_str().map(str::to_owned))
            .collect()
    }

    /// The hover text at a position.
    pub fn hover(&mut self, path: &Path, at: (usize, usize)) -> Option<String> {
        let result = self.request("textDocument/hover", Self::at(path, at));
        result["contents"]["value"]
            .as_str()
            .filter(|v| !v.is_empty())
            .map(str::to_owned)
    }

    /// Where the name at a position is declared.
    pub fn definition(&mut self, path: &Path, at: (usize, usize)) -> Option<Location> {
        let result = self.request("textDocument/definition", Self::at(path, at));
        match &result {
            Json::Array(locations) => locations.first().and_then(location_of),
            other => location_of(other),
        }
    }

    /// Every place the name at a position is declared or used.
    pub fn references(&mut self, path: &Path, at: (usize, usize)) -> Vec<Location> {
        let mut params = Self::at(path, at);
        params["context"] = json!({"includeDeclaration": true});
        let result = self.request("textDocument/references", params);
        result
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(location_of)
            .collect()
    }

    /// The name at a position that a rename would change, or none when there is no name there.
    pub fn prepare_rename(&mut self, path: &Path, at: (usize, usize)) -> Option<String> {
        let result = self.request("textDocument/prepareRename", Self::at(path, at));
        result["placeholder"].as_str().map(str::to_owned).or_else(|| {
            result
                .get("start")
                .or_else(|| result.get("range").map(|r| &r["start"]))
                .map(|_| String::new())
        })
    }

    /// The edits of a rename, by file.
    pub fn rename(&mut self, path: &Path, at: (usize, usize), new_name: &str) -> BTreeMap<PathBuf, Vec<TextEdit>> {
        let mut params = Self::at(path, at);
        params["newName"] = json!(new_name);
        let result = self.request("textDocument/rename", params);
        result["changes"]
            .as_object()
            .into_iter()
            .flatten()
            .map(|(uri, edits)| (uri_to_path(uri), edits_of(edits)))
            .collect()
    }

    /// The edits that format a document; none when it does not parse.
    pub fn formatting(&mut self, path: &Path) -> Vec<TextEdit> {
        let result = self.request(
            "textDocument/formatting",
            json!({"textDocument": {"uri": path_to_uri(path)}, "options": {"tabSize": 2, "insertSpaces": true}}),
        );
        edits_of(&result)
    }

    /// The outline of a document, depth first.
    pub fn symbols(&mut self, path: &Path) -> Vec<Symbol> {
        fn flatten(symbols: &Json, depth: usize, out: &mut Vec<Symbol>) {
            for symbol in symbols.as_array().into_iter().flatten() {
                out.push(Symbol {
                    name: symbol["name"].as_str().unwrap_or("").to_owned(),
                    detail: symbol["detail"].as_str().unwrap_or("").to_owned(),
                    depth,
                    start: point(&symbol["selectionRange"]["start"]),
                });
                flatten(&symbol["children"], depth + 1, out);
            }
        }
        let result = self.request(
            "textDocument/documentSymbol",
            json!({"textDocument": {"uri": path_to_uri(path)}}),
        );
        let mut out = Vec::new();
        flatten(&result, 0, &mut out);
        out
    }

    /// The semantic tokens of a document, decoded from the protocol's relative integers.
    pub fn tokens(&mut self, path: &Path) -> Vec<Token> {
        let result = self.request(
            "textDocument/semanticTokens/full",
            json!({"textDocument": {"uri": path_to_uri(path)}}),
        );
        let data: Vec<usize> = result["data"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|n| n.as_u64().unwrap_or(0) as usize)
            .collect();
        let (mut line, mut start) = (0, 0);
        data.as_chunks::<5>()
            .0
            .iter()
            .map(|chunk| {
                if chunk[0] > 0 {
                    line += chunk[0];
                    start = chunk[1];
                } else {
                    start += chunk[1];
                }
                Token {
                    line,
                    start,
                    length: chunk[2],
                    kind: self.legend.get(chunk[3]).cloned().unwrap_or_default(),
                }
            })
            .collect()
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.request("shutdown", Json::Null);
        self.notify("exit", Json::Null);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// The TextMate scope VS Code colours a semantic token type by when the theme has no semantic colours.
fn scope_of(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "keyword" => "keyword.control",
        "type" => "entity.name.type",
        "property" => "variable.other.property",
        "function" => "entity.name.function",
        "variable" => "variable.other.readwrite",
        "parameter" => "variable.parameter",
        "string" => "string",
        "number" => "constant.numeric",
        "comment" => "comment",
        "operator" => "keyword.operator",
        "namespace" => "entity.name.namespace",
        "enumMember" => "variable.other.enummember",
        "macro" => "entity.name.function.preprocessor",
        _ => return None,
    })
}

/// What the editor's language client asks of the window: to open a file at a place, or to edit a file that is not
/// this tab's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Open {
        path: PathBuf,
        line: usize,
        character: usize,
    },
    Edit {
        path: PathBuf,
        edits: Vec<TextEdit>,
    },
}

/// What a `.atd` tab keeps of its language client: the revision its colouring and outline were made for, the
/// outline, the hover under the pointer and the popup open beside the cursor.
#[derive(Default)]
pub struct State {
    coloured: Option<u64>,
    outline: Vec<Symbol>,
    hover: Option<Hover>,
    popup: Option<Popup>,
}

struct Hover {
    at: usize,
    since: Instant,
    text: Option<Option<String>>,
}

enum Popup {
    /// Hover at the cursor, or a message.
    Info(String),
    References(Vec<Location>),
    Rename {
        at: (usize, usize),
        name: String,
        focus: bool,
    },
}

/// The buffer's character offset of a line and character.
fn offset(open: &OpenText, at: (usize, usize)) -> usize {
    open.editor.buffer.offset(at.0, at.1)
}

/// Applies edits to a buffer, the last first so that the earlier positions still hold.
pub fn apply(buffer: &mut adp_text::Buffer, edits: &[TextEdit]) {
    let mut edits = edits.to_vec();
    edits.sort_by_key(|e| std::cmp::Reverse(e.start));
    for edit in edits {
        let start = buffer.offset(edit.start.0, edit.start.1);
        let end = if edit.end.0 >= buffer.line_count() {
            buffer.len()
        } else {
            buffer.offset(edit.end.0, edit.end.1)
        };
        buffer.replace_range(start, end.max(start), &edit.text);
    }
}

/// Gives the server the tab's text when it changed (from [`OpenText::analyse`]).
pub fn sync(open: &OpenText) {
    let text = open.editor.buffer.text();
    client().sync(open.file.path(), &text, open.editor.buffer.revision());
}

/// Takes the diagnostics that came for the tab: its underlines and its status line.
fn take_diagnostics(open: &mut OpenText, client: &mut Client) -> bool {
    let Some(diagnostics) = client.take_diagnostics(open.file.path()) else {
        return false;
    };
    let marks: Vec<Mark> = diagnostics
        .iter()
        .map(|d| Mark {
            start: offset(open, d.start),
            end: offset(open, d.end),
            level: d.level,
            message: format!("{}: {}", d.code, d.message),
        })
        .collect();
    let errors = marks.iter().filter(|m| m.level == MarkLevel::Error).count();
    let warnings = marks.iter().filter(|m| m.level == MarkLevel::Warning).count();
    open.editor.marks = marks;
    open.status = format!("ATD · {errors} errors, {warnings} warnings");
    true
}

/// The tab's colouring and outline, made again once the server has the revision the editor shows.
fn refresh(open: &mut OpenText, client: &mut Client, highlighter: &Highlighter, theme: &Theme) {
    let revision = open.editor.buffer.revision();
    if open.atd.coloured == Some(revision) || client.waiting(open.file.path()) {
        return;
    }
    open.atd.coloured = Some(revision);
    let path = open.file.path().to_path_buf();
    let mut lines: Vec<Vec<SemanticRun>> = vec![Vec::new(); open.editor.buffer.line_count()];
    for token in client.tokens(&path) {
        let Some((color, italic)) = scope_of(&token.kind).and_then(|s| highlighter.scope_style(s, theme.kind)) else {
            continue;
        };
        if let Some(line) = lines.get_mut(token.line) {
            line.push(SemanticRun {
                start: token.start,
                end: token.start + token.length,
                color,
                italic,
            });
        }
    }
    open.editor.semantic = Some((revision, lines));
    open.atd.outline = client.symbols(&path);
}

/// Carries out what the person asked: completion, hover, definition, references, rename and formatting.
fn commands(open: &mut OpenText, client: &mut Client, actions: &mut Vec<Action>) {
    let path = open.file.path().to_path_buf();
    for command in std::mem::take(&mut open.editor.commands) {
        let cursor = open.editor.buffer.cursor();
        let at = open.editor.buffer.position(cursor);
        match command {
            Command::Complete => {
                // The list completes the word the cursor is in, from its start.
                let line = open.editor.buffer.line(at.0);
                let word = line
                    .chars()
                    .take(at.1)
                    .collect::<Vec<_>>()
                    .iter()
                    .rev()
                    .take_while(|c| c.is_alphanumeric() || **c == '_')
                    .count();
                let items = client.completion(&path, at);
                open.editor.completion = (!items.is_empty()).then(|| Completion {
                    items,
                    selected: 0,
                    start: cursor - word,
                });
            }
            Command::Accept(label) => {
                if let Some(completion) = open.editor.completion.take() {
                    open.editor.buffer.replace_range(completion.start, cursor, &label);
                } else {
                    let line = open.editor.buffer.line(at.0);
                    let word = line
                        .chars()
                        .take(at.1)
                        .collect::<Vec<_>>()
                        .iter()
                        .rev()
                        .take_while(|c| c.is_alphanumeric() || **c == '_')
                        .count();
                    open.editor.buffer.replace_range(cursor - word, cursor, &label);
                }
            }
            Command::Hover => {
                let text = client
                    .hover(&path, at)
                    .unwrap_or_else(|| "Nothing is known about this.".into());
                open.atd.popup = Some(Popup::Info(text));
            }
            Command::Definition => match client.definition(&path, at) {
                Some(location) if key(&location.path) == key(&path) => {
                    let target = offset(open, location.start);
                    open.editor.buffer.set_cursor(target);
                }
                Some(location) => actions.push(Action::Open {
                    path: location.path,
                    line: location.start.0,
                    character: location.start.1,
                }),
                None => open.atd.popup = Some(Popup::Info("No definition found.".into())),
            },
            Command::References => {
                let references = client.references(&path, at);
                open.atd.popup = Some(if references.is_empty() {
                    Popup::Info("No references found.".into())
                } else {
                    Popup::References(references)
                });
            }
            Command::Rename => {
                open.atd.popup = Some(match client.prepare_rename(&path, at) {
                    Some(name) => Popup::Rename { at, name, focus: true },
                    None => Popup::Info("The element can't be renamed.".into()),
                });
            }
            Command::Format => {
                let edits = client.formatting(&path);
                apply(&mut open.editor.buffer, &edits);
            }
        }
    }
}

/// Renames the name at `at` to `name` across every file of the description: this tab's edits here, the others'
/// as actions.
fn rename(open: &mut OpenText, client: &mut Client, at: (usize, usize), name: &str, actions: &mut Vec<Action>) {
    let path = open.file.path().to_path_buf();
    for (file, edits) in client.rename(&path, at, name) {
        if key(&file) == key(&path) {
            apply(&mut open.editor.buffer, &edits);
        } else {
            actions.push(Action::Edit { path: file, edits });
        }
    }
}

/// Draws a `.atd` tab: its outline, its editor and the language client's popups; returns what the window must do.
pub fn show(ui: &mut egui::Ui, open: &mut OpenText, theme: &Theme, highlighter: &Highlighter) -> Vec<Action> {
    let mut actions = Vec::new();
    let mut client = client();
    take_diagnostics(open, &mut client);
    refresh(open, &mut client, highlighter, theme);
    commands(open, &mut client, &mut actions);
    if client.waiting(open.file.path()) {
        ui.ctx().request_repaint_after(Duration::from_millis(100));
    }

    let foreground = color(theme, &["foreground"], Color32::GRAY);
    let muted = color(theme, &["descriptionForeground"], Color32::GRAY);
    egui::Panel::left("atd-outline")
        .default_size(200.0)
        .resizable(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("OUTLINE").color(muted));
                if ui
                    .button("Format")
                    .on_hover_text("Format the file (Shift+Alt+F)")
                    .clicked()
                {
                    open.editor.commands.push(Command::Format);
                }
            });
            // To a screen reader the outline is a tree named Outline, each symbol an item of it.
            let tree = egui::UiBuilder::new()
                .accessibility_role(Role::Tree)
                .accessibility_label("Outline");
            ui.scope_builder(tree, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("atd-outline")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for symbol in &open.atd.outline {
                            ui.horizontal(|ui| {
                                ui.add_space(12.0 * symbol.depth as f32);
                                let label = RichText::new(&symbol.name).color(foreground);
                                let response = ui
                                    .add(egui::Button::new(label).frame(false))
                                    .on_hover_text(&symbol.detail);
                                if response.clicked() {
                                    let target = open.editor.buffer.offset(symbol.start.0, symbol.start.1);
                                    open.editor.buffer.set_cursor(target);
                                    ui.memory_mut(|m| m.request_focus(open.editor.id()));
                                }
                            });
                        }
                    });
            });
        });
    egui::CentralPanel::default().show(ui, |ui| {
        open.editor.show(ui, theme, highlighter);
    });

    hover(ui, open, &mut client, theme);
    popups(ui, open, &mut client, theme, &mut actions);
    actions
}

/// A popup's frame in VS Code's colours for the suggest widget and the editor's other widgets.
fn popup_frame(theme: &Theme) -> egui::Frame {
    let background = color(
        theme,
        &[
            "editorSuggestWidget.background",
            "editorWidget.background",
            "editor.background",
        ],
        Color32::from_gray(37),
    );
    let border = color(
        theme,
        &[
            "editorSuggestWidget.border",
            "editorWidget.border",
            "widget.border",
            "panel.border",
        ],
        Color32::from_gray(69),
    );
    egui::Frame::NONE
        .fill(background)
        .stroke(egui::Stroke::new(theme.sizes.border_width.max(1.0), border))
        .corner_radius(3)
        .inner_margin(egui::Margin::same(4))
}

/// The hover of the name the pointer rests on, as a tooltip.
fn hover(ui: &egui::Ui, open: &mut OpenText, client: &mut Client, theme: &Theme) {
    let Some((at, pointer)) = open.editor.pointer else {
        open.atd.hover = None;
        return;
    };
    let state = match &mut open.atd.hover {
        Some(state) if state.at == at => state,
        _ => {
            open.atd.hover = Some(Hover {
                at,
                since: Instant::now(),
                text: None,
            });
            ui.ctx().request_repaint_after(HOVER_DELAY);
            return;
        }
    };
    if state.since.elapsed() < HOVER_DELAY {
        ui.ctx().request_repaint_after(HOVER_DELAY - state.since.elapsed());
        return;
    }
    if state.text.is_none() {
        let position = open.editor.buffer.position(at);
        state.text = Some(client.hover(open.file.path(), position));
    }
    if let Some(Some(text)) = &state.text {
        egui::Area::new(open.editor.id().with("hover"))
            .fade_in(false)
            .order(egui::Order::Tooltip)
            .fixed_pos(pointer + egui::vec2(12.0, 16.0))
            .show(ui.ctx(), |ui| {
                popup_frame(theme).show(ui, |ui| {
                    ui.set_max_width(480.0);
                    ui.label(text);
                });
            });
    }
}

/// The completion list, and the popup of hover, references or rename, beside the cursor.
fn popups(ui: &egui::Ui, open: &mut OpenText, client: &mut Client, theme: &Theme, actions: &mut Vec<Action>) {
    let anchor = open
        .editor
        .cursor_rect
        .map(|r| r.left_bottom())
        .unwrap_or_else(|| ui.max_rect().left_top() + egui::vec2(220.0, 40.0));
    let foreground = color(theme, &["editorSuggestWidget.foreground", "foreground"], Color32::GRAY);
    let selected = color(
        theme,
        &[
            "editorSuggestWidget.selectedForeground",
            "list.activeSelectionForeground",
            "foreground",
        ],
        Color32::WHITE,
    );
    let selected_background = color(
        theme,
        &[
            "editorSuggestWidget.selectedBackground",
            "list.activeSelectionBackground",
        ],
        Color32::from_rgb(4, 57, 94),
    );

    let id = open.editor.id();
    if let Some(completion) = &mut open.editor.completion {
        let shown = completion.shown(&open.editor.buffer);
        if shown.is_empty() {
            open.editor.completion = None;
        } else {
            completion.selected = completion.selected.min(shown.len() - 1);
            let mut chosen = None;
            egui::Area::new(id.with("completion"))
                .fade_in(false)
                .order(egui::Order::Foreground)
                .fixed_pos(anchor)
                .show(ui.ctx(), |ui| {
                    popup_frame(theme).show(ui, |ui| {
                        let list = egui::UiBuilder::new()
                            .accessibility_role(Role::ListBox)
                            .accessibility_label("Suggestions");
                        ui.scope_builder(list, |ui| {
                            egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                                for (i, label) in shown.iter().enumerate() {
                                    let is_selected = i == completion.selected;
                                    let text = RichText::new(label).monospace().color(if is_selected {
                                        selected
                                    } else {
                                        foreground
                                    });
                                    let mut button = egui::Button::selectable(is_selected, text);
                                    if is_selected {
                                        button = button.fill(selected_background);
                                    }
                                    let response = ui.add(button);
                                    if is_selected {
                                        response.scroll_to_me(None);
                                    }
                                    if response.clicked() {
                                        chosen = Some(label.clone());
                                    }
                                }
                            });
                        });
                    });
                });
            if let Some(label) = chosen {
                open.editor.commands.push(Command::Accept(label));
                ui.memory_mut(|m| m.request_focus(id));
            }
        }
    }

    let Some(popup) = &mut open.atd.popup else {
        return;
    };
    let mut close = ui.input(|i| i.key_pressed(Key::Escape));
    let mut rename_to = None;
    egui::Area::new(open.editor.id().with("popup"))
        .fade_in(false)
        .order(egui::Order::Foreground)
        .fixed_pos(anchor)
        .show(ui.ctx(), |ui| {
            popup_frame(theme).show(ui, |ui| {
                ui.set_max_width(520.0);
                match popup {
                    Popup::Info(text) => {
                        ui.label(text.as_str());
                    }
                    Popup::References(references) => {
                        ui.label(RichText::new(format!("{} references", references.len())).color(foreground));
                        let list = egui::UiBuilder::new()
                            .accessibility_role(Role::List)
                            .accessibility_label("References");
                        ui.scope_builder(list, |ui| {
                            egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                                for reference in references.iter() {
                                    let name = reference
                                        .path
                                        .file_name()
                                        .map(|n| n.to_string_lossy().into_owned())
                                        .unwrap_or_default();
                                    let label = format!("{name}:{}:{}", reference.start.0 + 1, reference.start.1 + 1);
                                    if ui.add(egui::Button::new(label).frame(false)).clicked() {
                                        actions.push(Action::Open {
                                            path: reference.path.clone(),
                                            line: reference.start.0,
                                            character: reference.start.1,
                                        });
                                        close = true;
                                    }
                                }
                            });
                        });
                    }
                    Popup::Rename { at, name, focus } => {
                        ui.label("Rename to");
                        let response = ui.add(
                            egui::TextEdit::singleline(name)
                                .desired_width(240.0)
                                .id(open.editor.id().with("rename")),
                        );
                        if std::mem::take(focus) {
                            response.request_focus();
                        }
                        if response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                            rename_to = Some((*at, name.clone()));
                        }
                    }
                }
            });
        });
    if let Some((at, name)) = rename_to {
        rename(open, client, at, &name, actions);
        close = true;
        ui.memory_mut(|m| m.request_focus(open.editor.id()));
    }
    // A popup other than rename closes when the cursor moves or the text changes.
    if !matches!(open.atd.popup, Some(Popup::Rename { .. }))
        && ui.input(|i| !i.events.is_empty() && i.pointer.any_click())
    {
        close = true;
    }
    if close {
        open.atd.popup = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec_dir() -> PathBuf {
        match std::env::var_os("ATD_SPEC_DIR") {
            Some(dir) => PathBuf::from(dir),
            None => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/specifications/atd"),
        }
    }

    fn sorted(values: &Json) -> Vec<String> {
        let mut out: Vec<String> = values
            .as_array()
            .into_iter()
            .flatten()
            .map(|v| v.as_str().or_else(|| v["code"].as_str()).unwrap_or("").to_owned())
            .collect();
        out.sort();
        out
    }

    /// The 30-edit walk of SC-005 through the shell's editor and client (T106): the tabs' buffers are edited as the
    /// person would, each request goes through the client to the server on its thread, and every step gives what
    /// the walk expects, as the core's walk does through `Server::handle` (crates/adp-atd-lsp/tests/walk.rs).
    #[test]
    fn the_walk_through_the_editor_and_its_client_gives_the_cores_results() {
        let walk: Json =
            serde_json::from_str(&std::fs::read_to_string(spec_dir().join("walk/walk.json")).unwrap()).unwrap();
        let folder = std::env::temp_dir().join(format!("atd-shell-walk-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        for entry in std::fs::read_dir(spec_dir()).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == "atd") {
                std::fs::copy(&path, folder.join(path.file_name().unwrap())).unwrap();
            }
        }
        let mut tabs: BTreeMap<String, OpenText> = BTreeMap::new();
        for file in walk["files"].as_array().unwrap() {
            let file = file.as_str().unwrap();
            let mut open = OpenText::open(&folder.join(file), &folder).unwrap();
            open.analyse();
            client().wait(open.file.path(), TIMEOUT);
            tabs.insert(file.to_owned(), open);
        }
        let mut failures = Vec::new();
        for step in walk["steps"].as_array().unwrap() {
            let id = step["id"].as_u64().unwrap();
            let file = step["file"].as_str().unwrap();
            let open = tabs.get_mut(file).unwrap();
            let path = open.file.path().to_path_buf();
            let at = (
                step["line"].as_u64().unwrap() as usize - 1,
                step["column"].as_u64().unwrap() as usize - 1,
            );
            let expect = &step["expect"];
            let mut client = client();
            let problem: Option<String> = match step["request"].as_str().unwrap() {
                "completion" => {
                    let mut got = client.completion(&path, at);
                    got.sort();
                    if expect.get("exactly").is_some() {
                        let want = sorted(&expect["exactly"]);
                        (got != want).then(|| format!("completion {got:?}, expected exactly {want:?}"))
                    } else {
                        let missing: Vec<String> = sorted(&expect["includes"])
                            .into_iter()
                            .filter(|e| !got.contains(e))
                            .collect();
                        let unwanted: Vec<String> = sorted(&expect["excludes"])
                            .into_iter()
                            .filter(|e| got.contains(e))
                            .collect();
                        (!missing.is_empty() || !unwanted.is_empty())
                            .then(|| format!("completion lacks {missing:?}, has {unwanted:?}"))
                    }
                }
                "hover" => {
                    let value = client.hover(&path, at).unwrap_or_default();
                    let want = expect["contains"].as_str().unwrap();
                    (!value.contains(want)).then(|| format!("hover {value:?} lacks {want:?}"))
                }
                "definition" => {
                    let location = client.definition(&path, at);
                    let ok = location.as_ref().is_some_and(|l| {
                        l.path.ends_with(expect["file"].as_str().unwrap())
                            && l.start.0 as u64 == expect["line"].as_u64().unwrap() - 1
                            && l.start.1 as u64 == expect["column"].as_u64().unwrap() - 1
                    });
                    (!ok).then(|| format!("definition {location:?}, expected {expect}"))
                }
                "references" => {
                    let count = client.references(&path, at).len() as u64;
                    (Some(count) != expect["count"].as_u64())
                        .then(|| format!("{count} references, expected {}", expect["count"]))
                }
                "rename" => {
                    let edits = client.rename(&path, at, step["newName"].as_str().unwrap());
                    let counts: BTreeMap<String, u64> = edits
                        .iter()
                        .map(|(p, e)| (p.file_name().unwrap().to_string_lossy().into_owned(), e.len() as u64))
                        .collect();
                    let want: BTreeMap<String, u64> = expect["files"]
                        .as_object()
                        .unwrap()
                        .iter()
                        .map(|(k, v)| (k.clone(), v.as_u64().unwrap()))
                        .collect();
                    (counts != want).then(|| format!("rename edits {counts:?}, expected {want:?}"))
                }
                "format" => {
                    let edits = client.formatting(&path);
                    drop(client);
                    apply(&mut open.editor.buffer, &edits);
                    open.analyse();
                    let mut client = super::client();
                    client.wait(&path, TIMEOUT);
                    let again = client.formatting(&path);
                    (!again.is_empty()).then(|| format!("the second formatting changed the file: {again:?}"))
                }
                "diagnostics" => {
                    client.saved(&path);
                    client.wait(&path, TIMEOUT);
                    let got = client.description_codes(&path);
                    let want = sorted(&expect["codes"]);
                    (got != want).then(|| format!("diagnostics {got:?}, expected {want:?}"))
                }
                "edit" => {
                    let edit = &step["edit"];
                    let start = open.editor.buffer.offset(at.0, at.1);
                    let end = match edit.get("endLine") {
                        Some(line) => open.editor.buffer.offset(
                            line.as_u64().unwrap() as usize - 1,
                            edit["endColumn"].as_u64().unwrap() as usize - 1,
                        ),
                        None => start,
                    };
                    drop(client);
                    open.editor
                        .buffer
                        .replace_range(start, end, edit["text"].as_str().unwrap());
                    open.analyse();
                    let mut client = super::client();
                    client.wait(&path, TIMEOUT);
                    let got = client.description_codes(&path);
                    let want = sorted(&expect["diagnostics"]);
                    (got != want).then(|| format!("diagnostics after the edit {got:?}, expected {want:?}"))
                }
                other => Some(format!("an unknown request `{other}`")),
            };
            if let Some(problem) = problem {
                failures.push(format!("step {id}: {problem}"));
            }
        }
        // The walk's edits leave the tabs unsaved: the copies on disk are as they were.
        drop(tabs);
        let _ = std::fs::remove_dir_all(&folder);
        assert!(
            failures.is_empty(),
            "{} steps failed:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }

    /// A `.atd` tab as drawn, its outline, colouring, underlines and completion list, written under
    /// `target/screenshots` to look at.
    #[test]
    fn an_atd_tab_draws_its_outline_colouring_underlines_and_completion_list() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("mindmap.atd");
        let text = std::fs::read_to_string(spec_dir().join("mindmap.atd"))
            .unwrap()
            .replacen("element Node", "element Node extends Missing", 1);
        std::fs::write(&path, text).unwrap();
        std::fs::copy(
            spec_dir().join("mindmap.notation.atd"),
            folder.path().join("mindmap.notation.atd"),
        )
        .unwrap();
        let mut open = OpenText::open(&path, folder.path()).unwrap();
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        let theme = Theme::dark();
        let highlighter = Highlighter::default();
        let mut textures = std::collections::HashMap::new();
        let mut shapes = Vec::new();
        for frame in 0..6 {
            open.analyse();
            if frame == 1 {
                client().wait(&path, TIMEOUT);
            }
            if frame == 3 {
                ctx.memory_mut(|m| m.request_focus(open.editor.id()));
                let line = open.editor.buffer.line_count() / 6;
                let at = open.editor.buffer.offset(line, 0);
                open.editor.buffer.set_cursor(at);
                open.editor.buffer.insert("\n  ");
                open.analyse();
                client().wait(&path, TIMEOUT);
                open.editor.commands.push(Command::Complete);
            }
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0))),
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| {
                show(ui, &mut open, &theme, &highlighter);
            });
            crate::screenshot::take_textures(&mut textures, &mut output);
            shapes = output.shapes;
        }
        assert!(open.status.starts_with("ATD · 1 errors"), "{}", open.status);
        assert!(!open.atd.outline.is_empty());
        assert!(open.editor.semantic.is_some());
        assert!(open.editor.completion.is_some(), "the completion list is open");
        let screenshots = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/screenshots");
        crate::screenshot::write(
            &ctx,
            &shapes,
            &textures,
            egui::vec2(1280.0, 800.0),
            color(&theme, &["editor.background"], Color32::BLACK),
            &screenshots.join("atd-editor.png"),
        );
    }

    #[test]
    fn edits_apply_from_the_last_so_that_positions_hold() {
        let mut buffer = adp_text::Buffer::new("node A\nnode B\n");
        apply(
            &mut buffer,
            &[
                TextEdit {
                    start: (0, 5),
                    end: (0, 6),
                    text: "Alpha".into(),
                },
                TextEdit {
                    start: (1, 5),
                    end: (1, 6),
                    text: "Beta".into(),
                },
            ],
        );
        assert_eq!(buffer.text(), "node Alpha\nnode Beta\n");
    }
}
