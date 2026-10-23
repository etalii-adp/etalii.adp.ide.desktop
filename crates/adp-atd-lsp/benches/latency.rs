//! The language server's latency on a large description (etalii.adp spec 035, SC-006, T099): a description of 5,000
//! lines or more, made from the constructs of the vendored mind map example, is opened in the core; then a keystroke
//! is typed and its diagnostics computed, and another is typed and completion asked for inside an element at once. Diagnostics, with the
//! 150 ms the transports wait for typing to pause, must come within 1 s, and completion within 0.5 s. Each is
//! measured five times and the slowest is printed and held to its bound.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use adp_atd_lsp::{Server, path_to_uri};
use serde_json::{Value, json};

const DEBOUNCE: Duration = Duration::from_millis(150);
const DIAGNOSTICS_BOUND: Duration = Duration::from_secs(1);
const COMPLETION_BOUND: Duration = Duration::from_millis(500);

fn spec_dir() -> PathBuf {
    match std::env::var_os("ATD_SPEC_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/specifications/atd"),
    }
}

/// The mind map example with element types in its model and operations after them, until it has `lines` lines.
fn description(lines: usize) -> String {
    let base = std::fs::read_to_string(spec_dir().join("mindmap.atd")).unwrap();
    let (mut elements, mut operations) = (String::new(), String::new());
    let mut i = 0;
    while base.lines().count() + elements.lines().count() + operations.lines().count() < lines {
        i += 1;
        elements.push_str(&format!(
            "    /// Generated topic {i}.\n    element Topic{i} {{\n      label \"Topic {i}\"\n      attribute text: string default \"\" {{ label: \"Text\" order: 1 }}\n      attribute notes: text default \"\" {{ markup: plain label: \"Notes\" order: 2 }}\n      attribute flagged: bool default false {{ label: \"Flagged\" order: 3 }}\n      contains Node ordered\n      labelAttribute: text\n    }}\n\n"
        ));
        operations.push_str(&format!(
            "  operation grow{i} \"Grow {i}\" for Node {{\n    doc \"Adds node {i} under this one.\"\n    enabled self.parent != null && self.children.size() < {i}\n    create Node {{ text: siblingName(self, \"Node {i}\") }} in self as added\n    select added\n  }}\n\n"
        ));
    }
    base.replacen("  model {\n", &format!("  model {{\n{elements}"), 1)
        .replacen("  reason atStart", &format!("{operations}  reason atStart"), 1)
}

fn request(server: &mut Server, id: i64, method: &str, params: Value) -> Value {
    let out = server.handle(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
    out.into_iter().next().unwrap()["result"].clone()
}

fn main() {
    let folder = std::env::temp_dir().join(format!("atd-latency-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::copy(
        spec_dir().join("mindmap.notation.atd"),
        folder.join("mindmap.notation.atd"),
    )
    .unwrap();
    let path = folder.join("mindmap.atd");
    let text = description(5_000);
    std::fs::write(&path, &text).unwrap();
    let lines = text.lines().count();
    let uri = path_to_uri(&path);

    let mut server = Server::new();
    request(&mut server, 1, "initialize", json!({"capabilities": {}}));
    let opened = server.handle(&json!({"jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {"textDocument": {"uri": uri, "languageId": "atd", "version": 1, "text": text}}}));
    let findings: usize = opened
        .iter()
        .filter(|m| m["method"] == "textDocument/publishDiagnostics")
        .map(|m| m["params"]["diagnostics"].as_array().map_or(0, Vec::len))
        .sum();
    assert_eq!(findings, 0, "the generated description checks clean");

    // The keystroke: a letter typed into the first generated topic's documentation comment.
    let (line, character) = text
        .lines()
        .enumerate()
        .find_map(|(n, l)| l.find("/// Generated topic 1.").map(|c| (n, c + 4)))
        .unwrap();
    // The completion: the start of a new line inside the last generated element.
    let inside = text.lines().position(|l| l.trim() == "labelAttribute: text").unwrap();
    let mut slowest_diagnostics = Duration::ZERO;
    let mut slowest_completion = Duration::ZERO;
    for round in 0..5 {
        let started = Instant::now();
        server.handle(
            &json!({"jsonrpc": "2.0", "method": "textDocument/didChange", "params": {
            "textDocument": {"uri": uri, "version": round + 2},
            "contentChanges": [{"range": {"start": {"line": line, "character": character},
                "end": {"line": line, "character": character}}, "text": "x"}]}}),
        );
        let published = server.flush();
        let elapsed = started.elapsed() + DEBOUNCE;
        assert!(
            published
                .iter()
                .any(|m| m["method"] == "textDocument/publishDiagnostics")
        );
        slowest_diagnostics = slowest_diagnostics.max(elapsed);

        // Completion right after another keystroke, before the diagnostics of it are made: the worst case, since
        // the server otherwise reuses the check its diagnostics made.
        server.handle(
            &json!({"jsonrpc": "2.0", "method": "textDocument/didChange", "params": {
            "textDocument": {"uri": uri, "version": round + 100},
            "contentChanges": [{"range": {"start": {"line": line, "character": character},
                "end": {"line": line, "character": character}}, "text": "y"}]}}),
        );
        let started = Instant::now();
        let result = request(
            &mut server,
            10 + round,
            "textDocument/completion",
            json!({"textDocument": {"uri": uri}, "position": {"line": inside, "character": 6}}),
        );
        slowest_completion = slowest_completion.max(started.elapsed());
        assert!(
            result["items"].as_array().is_some_and(|i| !i.is_empty()),
            "completion offers items"
        );
    }
    let _ = std::fs::remove_dir_all(&folder);

    println!("latency on a {lines}-line description (slowest of 5):");
    println!(
        "  diagnostics after a keystroke: {slowest_diagnostics:?} (150 ms debounce included; bound {DIAGNOSTICS_BOUND:?})"
    );
    println!("  completion: {slowest_completion:?} (bound {COMPLETION_BOUND:?})");
    assert!(lines >= 5_000);
    assert!(
        slowest_diagnostics < DIAGNOSTICS_BOUND,
        "diagnostics took {slowest_diagnostics:?}"
    );
    assert!(
        slowest_completion < COMPLETION_BOUND,
        "completion took {slowest_completion:?}"
    );
}
