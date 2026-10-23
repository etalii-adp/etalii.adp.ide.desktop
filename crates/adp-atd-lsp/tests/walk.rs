//! The 30-edit walk of SC-005 (`specifications/atd/walk/walk.json`) through the core, `Server::handle`
//! (spec 035, T098): each step's completion, hover, definition, references, rename, formatting and diagnostics as
//! the walk expects them, on copies of the examples.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use adp_atd_lsp::{Server, path_to_uri, uri_to_path};
use serde_json::{Value, json};

fn spec_dir() -> PathBuf {
    match std::env::var_os("ATD_SPEC_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/specifications/atd"),
    }
}

struct Walk {
    server: Server,
    folder: PathBuf,
    next_id: i64,
    versions: BTreeMap<String, i64>,
    diagnostics: BTreeMap<PathBuf, Vec<Value>>,
}

impl Walk {
    fn new(files: &[String]) -> Walk {
        let folder = std::env::temp_dir().join(format!("atd-walk-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        for entry in std::fs::read_dir(spec_dir()).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == "atd") {
                std::fs::copy(&path, folder.join(path.file_name().unwrap())).unwrap();
            }
        }
        let mut walk = Walk {
            server: Server::new(),
            folder,
            next_id: 0,
            versions: BTreeMap::new(),
            diagnostics: BTreeMap::new(),
        };
        walk.request("initialize", json!({"capabilities": {}}));
        for file in files {
            let path = walk.folder.join(file);
            let text = std::fs::read_to_string(&path).unwrap();
            let messages = walk.server.handle(&json!({"jsonrpc": "2.0", "method": "textDocument/didOpen",
                "params": {"textDocument": {"uri": path_to_uri(&path), "languageId": "atd", "version": 1, "text": text}}}));
            walk.record(messages);
            walk.versions.insert(file.clone(), 1);
        }
        walk
    }

    fn uri(&self, file: &str) -> String {
        path_to_uri(&self.folder.join(file))
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        let mut out = self
            .server
            .handle(&json!({"jsonrpc": "2.0", "id": self.next_id, "method": method, "params": params}));
        let response = out.remove(0);
        assert!(response.get("error").is_none(), "{method}: {response}");
        response["result"].clone()
    }

    fn record(&mut self, messages: Vec<Value>) {
        for message in messages {
            if message["method"] == "textDocument/publishDiagnostics" {
                let path = uri_to_path(message["params"]["uri"].as_str().unwrap());
                self.diagnostics
                    .insert(path, message["params"]["diagnostics"].as_array().unwrap().clone());
            }
        }
    }

    /// The codes of the diagnostics of every file of the description `file` belongs to.
    fn codes(&self) -> Vec<String> {
        let mut codes: Vec<String> = self
            .diagnostics
            .values()
            .flatten()
            .map(|d| d["code"].as_str().unwrap().to_owned())
            .collect();
        codes.sort();
        codes
    }

    fn edit(&mut self, file: &str, position: Value, end: Option<Value>, text: &str) {
        let version = {
            let entry = self.versions.entry(file.to_owned()).or_insert(1);
            *entry += 1;
            *entry
        };
        let range = json!({"start": position, "end": end.unwrap_or_else(|| position.clone())});
        let uri = self.uri(file);
        self.server.handle(
            &json!({"jsonrpc": "2.0", "method": "textDocument/didChange", "params": {
            "textDocument": {"uri": uri, "version": version},
            "contentChanges": [{"range": range, "text": text}]}}),
        );
        let messages = self.server.flush();
        self.diagnostics.clear();
        self.record(messages);
    }
}

fn labels(result: &Value) -> Vec<String> {
    let items = result["items"].as_array().cloned().unwrap_or_default();
    let mut labels: Vec<String> = items.iter().map(|i| i["label"].as_str().unwrap().to_owned()).collect();
    labels.sort();
    labels
}

#[test]
fn the_walk_gives_its_expected_results() {
    let walk_file: Value =
        serde_json::from_str(&std::fs::read_to_string(spec_dir().join("walk/walk.json")).unwrap()).unwrap();
    let files: Vec<String> = walk_file["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f.as_str().unwrap().to_owned())
        .collect();
    let mut walk = Walk::new(&files);
    let mut failures = Vec::new();
    for step in walk_file["steps"].as_array().unwrap() {
        let id = step["id"].as_u64().unwrap();
        let file = step["file"].as_str().unwrap().to_owned();
        let position =
            json!({"line": step["line"].as_u64().unwrap() - 1, "character": step["utf16"].as_u64().unwrap()});
        let document = json!({"uri": walk.uri(&file)});
        let expect = &step["expect"];
        let problem: Option<String> = match step["request"].as_str().unwrap() {
            "completion" => {
                let got = labels(&walk.request(
                    "textDocument/completion",
                    json!({"textDocument": document, "position": position}),
                ));
                if let Some(exactly) = expect["exactly"].as_array() {
                    let mut want: Vec<String> = exactly.iter().map(|e| e.as_str().unwrap().to_owned()).collect();
                    want.sort();
                    (got != want).then(|| format!("completion {got:?}, expected exactly {want:?}"))
                } else {
                    let missing: Vec<&str> = expect["includes"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|e| e.as_str())
                        .filter(|e| !got.iter().any(|g| g == e))
                        .collect();
                    let unwanted: Vec<&str> = expect["excludes"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|e| e.as_str())
                        .filter(|e| got.iter().any(|g| g == e))
                        .collect();
                    (!missing.is_empty() || !unwanted.is_empty())
                        .then(|| format!("completion lacks {missing:?}, has {unwanted:?}: {got:?}"))
                }
            }
            "hover" => {
                let result = walk.request(
                    "textDocument/hover",
                    json!({"textDocument": document, "position": position}),
                );
                let value = result["contents"]["value"].as_str().unwrap_or("").to_owned();
                let want = expect["contains"].as_str().unwrap();
                (!value.contains(want)).then(|| format!("hover {value:?} lacks {want:?}"))
            }
            "definition" => {
                let result = walk.request(
                    "textDocument/definition",
                    json!({"textDocument": document, "position": position}),
                );
                let uri = result["uri"].as_str().unwrap_or("");
                let start = &result["range"]["start"];
                let ok = uri.ends_with(expect["file"].as_str().unwrap())
                    && start["line"].as_u64() == Some(expect["line"].as_u64().unwrap() - 1)
                    && start["character"].as_u64() == expect["utf16"].as_u64();
                (!ok).then(|| format!("definition {result}, expected {expect}"))
            }
            "references" => {
                let result = walk.request(
                    "textDocument/references",
                    json!({"textDocument": document, "position": position, "context": {"includeDeclaration": true}}),
                );
                let count = result.as_array().map(Vec::len).unwrap_or(0) as u64;
                (Some(count) != expect["count"].as_u64())
                    .then(|| format!("{count} references, expected {}", expect["count"]))
            }
            "rename" => {
                let new_name = step["newName"].as_str().unwrap();
                let result = walk.request(
                    "textDocument/rename",
                    json!({"textDocument": document, "position": position, "newName": new_name}),
                );
                let mut counts: BTreeMap<String, u64> = BTreeMap::new();
                for (uri, edits) in result["changes"].as_object().into_iter().flatten() {
                    let name = Path::new(uri.as_str())
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .into_owned();
                    counts.insert(name, edits.as_array().unwrap().len() as u64);
                }
                let want: BTreeMap<String, u64> = expect["files"]
                    .as_object()
                    .unwrap()
                    .iter()
                    .map(|(k, v)| (k.clone(), v.as_u64().unwrap()))
                    .collect();
                (counts != want).then(|| format!("rename edits {counts:?}, expected {want:?}"))
            }
            "format" => {
                let result = walk.request(
                    "textDocument/formatting",
                    json!({"textDocument": document, "options": {"tabSize": 2, "insertSpaces": true}}),
                );
                if let Some(edit) = result.as_array().and_then(|a| a.first()) {
                    let text = edit["newText"].as_str().unwrap().to_owned();
                    let old = walk.server.text(&walk.folder.join(&file)).unwrap();
                    let lines = old.matches('\n').count();
                    walk.edit(
                        &file,
                        json!({"line": 0, "character": 0}),
                        Some(json!({"line": lines + 1, "character": 0})),
                        &text,
                    );
                }
                let again = walk.request(
                    "textDocument/formatting",
                    json!({"textDocument": document, "options": {"tabSize": 2, "insertSpaces": true}}),
                );
                (again != json!([])).then(|| format!("the second formatting changed the file: {again}"))
            }
            "diagnostics" => {
                let messages = walk.server.handle(
                    &json!({"jsonrpc": "2.0", "method": "textDocument/didSave", "params": {"textDocument": document}}),
                );
                walk.diagnostics.clear();
                walk.record(messages);
                let got = walk.codes();
                let mut want: Vec<String> = expect["codes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|c| c.as_str().unwrap().to_owned())
                    .collect();
                want.sort();
                (got != want).then(|| format!("diagnostics {got:?}, expected {want:?}"))
            }
            "edit" => {
                let edit = &step["edit"];
                let end = edit.get("endLine").map(|_| {
                    let text = walk.server.text(&walk.folder.join(&file)).unwrap();
                    let line = edit["endLine"].as_u64().unwrap() as usize;
                    let column = edit["endColumn"].as_u64().unwrap() as usize;
                    let line_text = text.lines().nth(line - 1).unwrap_or("");
                    let utf16: usize = line_text.chars().take(column - 1).map(char::len_utf16).sum();
                    json!({"line": line - 1, "character": utf16})
                });
                walk.edit(&file, position, end, edit["text"].as_str().unwrap());
                let got = walk.codes();
                let want: Vec<String> = expect["diagnostics"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|d| d["code"].as_str().unwrap().to_owned())
                    .collect();
                (got != want).then(|| format!("diagnostics after the edit {got:?}, expected {want:?}"))
            }
            other => Some(format!("an unknown request `{other}`")),
        };
        if let Some(problem) = problem {
            failures.push(format!("step {id} ({}): {problem}", step["doc"].as_str().unwrap_or("")));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of 30 steps failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
