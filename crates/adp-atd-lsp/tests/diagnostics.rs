//! The language server's diagnostics are the checker's findings (FR-054, spec 035 T098): for every example and every
//! refused file, the same codes at the same places; and a file with a syntax error still answers hover and
//! completion in its other parts (FR-053).

use std::path::PathBuf;

use adp_atd_lsp::{Server, path_to_uri};
use serde_json::{Value, json};

fn spec_dir() -> PathBuf {
    match std::env::var_os("ATD_SPEC_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/specifications/atd"),
    }
}

fn atd_files(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "atd"))
        .collect();
    found.sort();
    found
}

#[test]
fn diagnostics_equal_the_checkers_findings() {
    let mut files = atd_files(&spec_dir());
    files.extend(atd_files(&spec_dir().join("refused")));
    for path in files {
        let text = std::fs::read_to_string(&path).unwrap();
        let mut server = Server::new();
        server.handle(&json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"capabilities": {}}}));
        let messages = server.handle(&json!({"jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {"uri": path_to_uri(&path), "languageId": "atd", "version": 1, "text": text}}}));
        let mut published: Vec<(String, u64, u64)> = Vec::new();
        for message in &messages {
            let uri = message["params"]["uri"].as_str().unwrap_or("");
            let file = uri.rsplit('/').next().unwrap_or("").to_owned();
            for diagnostic in message["params"]["diagnostics"].as_array().into_iter().flatten() {
                let start = &diagnostic["range"]["start"];
                published.push((
                    format!("{file}:{}", diagnostic["code"].as_str().unwrap()),
                    start["line"].as_u64().unwrap() + 1,
                    start["character"].as_u64().unwrap(),
                ));
            }
        }
        let checked = adp_atd_check::check(&adp_atd_check::FsFiles, &path);
        let mut expected: Vec<(String, u64, u64)> = checked
            .findings
            .iter()
            .map(|f| {
                let file = f.file.rsplit('/').next().unwrap_or("").to_owned();
                let source = std::fs::read_to_string(&f.file).unwrap_or_default();
                let line_start = source[..f.start.min(source.len())]
                    .rfind('\n')
                    .map(|i| i + 1)
                    .unwrap_or(0);
                let utf16 = source[line_start..f.start.min(source.len())].encode_utf16().count() as u64;
                (format!("{file}:{}", f.code), f.line as u64, utf16)
            })
            .collect();
        published.sort();
        expected.sort();
        assert_eq!(published, expected, "{}", path.display());
    }
}

#[test]
fn a_broken_file_still_answers_in_its_other_parts() {
    let path = spec_dir().join("mindmap.atd");
    let text = std::fs::read_to_string(&path)
        .unwrap()
        .replacen("attribute link: string", "attribute link string", 1);
    let mut server = Server::new();
    server.handle(&json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"capabilities": {}}}));
    server.handle(&json!({"jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {"textDocument": {"uri": path_to_uri(&path), "languageId": "atd", "version": 1, "text": text.clone()}}}));
    let line = text.lines().position(|l| l.contains("operation promote")).unwrap() as u64;
    let hover: Value = server.handle(&json!({"jsonrpc": "2.0", "id": 2, "method": "textDocument/hover",
        "params": {"textDocument": {"uri": path_to_uri(&path)}, "position": {"line": line, "character": 14}}}))[0]["result"].clone();
    assert!(
        hover["contents"]["value"]
            .as_str()
            .unwrap_or("")
            .contains("Makes the node a sibling"),
        "{hover}"
    );
    let symbols: Value = server.handle(
        &json!({"jsonrpc": "2.0", "id": 3, "method": "textDocument/documentSymbol",
        "params": {"textDocument": {"uri": path_to_uri(&path)}}}),
    )[0]["result"]
        .clone();
    assert!(symbols.to_string().contains("operation promote"), "{symbols}");
    let tokens: Value = server.handle(
        &json!({"jsonrpc": "2.0", "id": 4, "method": "textDocument/semanticTokens/full",
        "params": {"textDocument": {"uri": path_to_uri(&path)}}}),
    )[0]["result"]
        .clone();
    assert!(tokens["data"].as_array().unwrap().len() > 500);
}

/// `atd/validate` (T128, contracts/language-server.md): the cases handed in, their files in base64, replayed through
/// the description's runtime; each difference placed at its construct by uri and range.
#[test]
fn validate_answers_each_case_with_its_differences_at_their_constructs() {
    use base64::Engine;
    let diagrams = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/definitions/diagrams");
    let folder = diagrams.join("conformance/mindmap/fold");
    let files: serde_json::Map<String, Value> = std::fs::read_dir(&folder)
        .unwrap()
        .flatten()
        .map(|e| {
            let content = base64::engine::general_purpose::STANDARD.encode(std::fs::read(e.path()).unwrap());
            (e.file_name().to_string_lossy().into_owned(), json!(content))
        })
        .collect();
    let mut server = Server::new();
    server.handle(&json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"capabilities": {}}}));
    let root = path_to_uri(&diagrams.join("mindmap.atd").canonicalize().unwrap());
    let answer = server.handle(&json!({"jsonrpc": "2.0", "id": 2, "method": "atd/validate",
        "params": {"root": root, "cases": [{"name": "fold", "files": files}]}}));
    let case = &answer[0]["result"]["cases"][0];
    assert_eq!(case["name"], "fold", "{answer:?}");
    assert_eq!(case["passed"], false);
    let drawing = case["differences"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["part"] == "drawing")
        .unwrap();
    assert_eq!(drawing["step"], 1);
    assert!(
        drawing["at"]["uri"]
            .as_str()
            .unwrap()
            .ends_with("/mindmap.notation.atd")
    );
    // The notation's `node`, on line 58 (from 0 in the protocol).
    assert_eq!(drawing["at"]["range"]["start"], json!({"line": 57, "character": 2}));

    let refused = server.handle(&json!({"jsonrpc": "2.0", "id": 3, "method": "atd/validate",
        "params": {"root": root, "cases": [{"name": "broken", "files": {"conformance.json": "not base64!"}}]}}));
    assert_eq!(refused[0]["error"]["code"], -32803);
}
