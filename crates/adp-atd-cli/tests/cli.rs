//! `atd` against contracts/checker.md (spec 035, T037, T151): exit codes, the text form, `--json` valid against
//! `$defs/Findings` with fixes as data, `--expect`, `--corpus`, `tree`, `tables`, `--spec-dir` and `--version`.

use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;

fn spec_dir() -> PathBuf {
    match std::env::var_os("ATD_SPEC_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/specifications/atd"),
    }
}

fn atd(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_atd"))
        .args(args)
        .output()
        .expect("atd runs");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn path(relative: &str) -> String {
    spec_dir().join(relative).to_string_lossy().into_owned()
}

/// The schema `$defs` entry `name` of atd.schema.json, with the other specifications' schemas beside it.
fn validator(name: &str) -> jsonschema::Validator {
    let mut schema: Value =
        serde_json::from_str(&std::fs::read_to_string(spec_dir().join("atd.schema.json")).unwrap()).unwrap();
    schema["$ref"] = Value::String(format!("#/$defs/{name}"));
    jsonschema::options()
        .with_retriever(Vendored)
        .build(&schema)
        .expect("the schema compiles")
}

/// The other specifications' schemas, beside ATD's, and nothing from the network.
struct Vendored;

impl jsonschema::Retrieve for Vendored {
    fn retrieve(&self, uri: &jsonschema::Uri<String>) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let name = uri.as_str().rsplit('/').next().unwrap_or("").trim_end_matches('#');
        let language = name.trim_end_matches(".schema.json");
        let folder = spec_dir().parent().unwrap().join(language);
        Ok(serde_json::from_str(&std::fs::read_to_string(folder.join(name))?)?)
    }
}

#[test]
fn the_examples_check_clean_with_exit_code_0() {
    let examples: Vec<String> = ["mindmap.atd", "mindmap.notation.atd", "reading-list.atd", "notes.atd"]
        .iter()
        .map(|f| path(f))
        .collect();
    let args: Vec<&str> = std::iter::once("check")
        .chain(examples.iter().map(String::as_str))
        .collect();
    let (code, stdout, _) = atd(&args);
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.is_empty(), "{stdout}");
}

#[test]
fn a_refused_file_exits_1_with_findings_in_the_text_form() {
    let (code, stdout, _) = atd(&["check", &path("refused/unknown-attribute.atd")]);
    assert_eq!(code, 1);
    let line = stdout.lines().next().expect("a finding");
    let pattern = regex_lite(line);
    assert!(pattern, "not `file:line:column: severity CODE: message`: {line}");
    assert!(line.contains("error ATD2001"), "{line}");
}

fn regex_lite(line: &str) -> bool {
    let mut parts = line.splitn(4, ':');
    let (Some(_file), Some(line_no), Some(column), Some(rest)) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    line_no.parse::<u32>().is_ok()
        && column.parse::<u32>().is_ok()
        && rest.starts_with(" error ATD") | rest.starts_with(" warning ATD") | rest.starts_with(" information ATD")
}

#[test]
fn a_usage_or_io_error_exits_2() {
    assert_eq!(atd(&["check", "/no/such/file.atd"]).0, 2);
    assert_eq!(atd(&["frobnicate"]).0, 2);
    assert_eq!(atd(&[]).0, 2);
}

#[test]
fn json_findings_are_valid_and_carry_their_fixes_as_data() {
    let validator = validator("Findings");
    let refused = spec_dir().join("refused");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&refused)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "atd"))
        .collect();
    files.sort();
    for file in files {
        let (_, stdout, _) = atd(&["check", "--json", &file.to_string_lossy()]);
        let value: Value =
            serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("{}: {e}\n{stdout}", file.display()));
        let errors: Vec<String> = validator.iter_errors(&value).map(|e| e.to_string()).collect();
        assert!(errors.is_empty(), "{}: {errors:?}", file.display());
        for finding in value["findings"].as_array().unwrap() {
            for key in ["code", "severity", "file", "line", "column", "message"] {
                assert!(finding.get(key).is_some(), "{}: no {key}", file.display());
            }
        }
    }
    let (_, stdout, _) = atd(&["check", "--json", &path("refused/unknown-attribute.atd")]);
    let value: Value = serde_json::from_str(&stdout).unwrap();
    let fix = &value["findings"][0]["fixes"][0];
    assert_eq!(fix["title"], "Change to `title`");
    assert_eq!(fix["edits"][0]["text"], "@title");
}

#[test]
fn expect_and_corpus_compare_with_the_expected_findings() {
    let (code, stdout, _) = atd(&[
        "check",
        "--expect",
        &path("refused/cel-type.findings.json"),
        &path("refused/cel-type.atd"),
    ]);
    assert_eq!(code, 0, "{stdout}");
    let (code, stdout, _) = atd(&[
        "check",
        "--expect",
        &path("refused/cel-type.findings.json"),
        &path("refused/unknown-function.atd"),
    ]);
    assert_eq!(code, 1, "{stdout}");
    let (code, stdout, _) = atd(&["check", "--corpus", &path("refused")]);
    assert_eq!(code, 0, "{stdout}");
}

#[test]
fn tree_prints_a_tree_valid_against_the_schema() {
    let (code, stdout, stderr) = atd(&["tree", &path("reading-list.atd")]);
    assert_eq!(code, 0, "{stderr}");
    let tree: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(tree["description"]["toolKind"], "designer");
}

#[test]
fn tables_equal_larks_from_the_compiled_grammar_and_from_a_spec_dir() {
    let larks = std::fs::read_to_string(spec_dir().join("grammar/atd.tables.json")).unwrap();
    let (code, compiled, _) = atd(&["tables"]);
    assert_eq!(code, 0);
    assert_eq!(compiled, larks);
    let (code, from_dir, _) = atd(&["--spec-dir", &spec_dir().to_string_lossy(), "tables"]);
    assert_eq!(code, 0);
    assert_eq!(from_dir, larks);
}

#[test]
fn version_names_the_tool_and_the_versions_of_atd() {
    let (code, stdout, _) = atd(&["--version"]);
    assert_eq!(code, 0);
    assert!(stdout.starts_with("atd ") && stdout.contains("ATD 0.1"), "{stdout}");
}
