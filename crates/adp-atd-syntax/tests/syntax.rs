//! `adp-atd-syntax` against the vendored examples and refused corpus (spec 035, T035): every example parses, the
//! lossless tree is the file, a refused syntax error is found where its findings file says, every example's
//! abstract tree is valid against `$defs/Tree`, and a header of an unknown version is told apart.

use std::path::PathBuf;

use serde_json::Value;

fn spec_dir() -> PathBuf {
    match std::env::var_os("ATD_SPEC_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/specifications/atd"),
    }
}

fn files(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "atd"))
        .collect();
    found.sort();
    found
}

#[test]
fn every_example_parses_and_is_lossless() {
    let examples = files(&spec_dir());
    assert!(examples.len() >= 8, "the examples are vendored");
    for path in examples {
        let text = std::fs::read_to_string(&path).unwrap();
        let parse = adp_atd_syntax::parse(&text);
        assert!(parse.errors.is_empty(), "{}: {:?}", path.display(), parse.errors);
        assert!(parse.root.is_some(), "{}", path.display());
        assert_eq!(parse.source(), text, "{} is not lossless", path.display());
    }
}

#[test]
fn every_refused_file_is_lossless_and_its_syntax_error_is_where_its_findings_say() {
    let dir = spec_dir().join("refused");
    for path in files(&dir) {
        let text = std::fs::read_to_string(&path).unwrap();
        if path.file_stem().unwrap() == "unknown-version" {
            let (version, start, _) = adp_atd_syntax::declared_version(&text).unwrap();
            assert_eq!(version, "9.4");
            assert_eq!(start, 4);
            assert!(!adp_atd_syntax::VERSIONS.contains(&version.as_str()));
            continue;
        }
        let parse = adp_atd_syntax::parse(&text);
        assert_eq!(parse.source(), text, "{} is not lossless", path.display());
        let findings: Value =
            serde_json::from_str(&std::fs::read_to_string(path.with_extension("findings.json")).unwrap()).unwrap();
        let syntax: Vec<&Value> =
            findings["findings"].as_array().unwrap().iter().filter(|f| f["code"].as_str().unwrap().starts_with("ATD1")).collect();
        match (syntax.first(), parse.errors.first()) {
            (None, None) => {}
            (Some(expected), Some(error)) => {
                let lines = adp_atd_syntax::LineIndex::new(&text);
                let (line, column) = lines.position(&text, error.start);
                assert_eq!(
                    (line as u64, column as u64),
                    (expected["line"].as_u64().unwrap(), expected["column"].as_u64().unwrap()),
                    "{}: {}",
                    path.display(),
                    error.message
                );
            }
            (expected, error) => panic!("{}: expected {expected:?}, found {error:?}", path.display()),
        }
    }
}

const DEFS: &[(&str, &str)] = &[
    ("tool", "Tool"), ("part", "Part"), ("label", "Label"), ("doc", "DocMember"), ("setting", "SettingMember"),
    ("import", "Import"), ("requires", "Requires"), ("model", "Model"), ("constraints", "Constraints"),
    ("constraint", "Constraint"), ("builtin", "Builtin"), ("operation", "Operation"), ("hook", "Hook"),
    ("reason", "Reason"), ("message", "MessageDecl"), ("behavior", "Behavior"), ("persistence", "Persistence"),
    ("plugin", "Plugin"), ("fn", "Function"), ("coordinates", "Coordinates"), ("notation", "Notation"),
    ("toolbox", "Toolbox"), ("form", "Form"), ("layout", "Layout"), ("viewpoint", "Viewpoint"),
    ("tableSurface", "TableSurface"), ("hierarchySurface", "HierarchySurface"), ("textSurface", "TextSurface"),
    ("binding", "Binding"), ("element", "Element"), ("relation", "Relation"), ("enum", "Enum"),
    ("datatype", "DataType"), ("document", "DocumentDecl"), ("attribute", "Attribute"), ("contains", "Contains"),
    ("port", "Port"), ("fix", "Fix"), ("param", "Param"), ("rule", "Rule"),
];

/// Validates a construct against the `$defs` entry of its kind, and its members likewise, collecting the errors of
/// the innermost constructs that fail.
fn explain(schema: &Value, node: &Value, dir: &std::path::Path, problems: &mut Vec<String>) {
    let kind = node["kind"].as_str().unwrap_or("");
    let Some((_, def)) = DEFS.iter().find(|(k, _)| *k == kind) else { return };
    let mut sub = schema.clone();
    sub["$ref"] = Value::String(format!("#/$defs/{def}"));
    let validator = jsonschema::options().with_retriever(Vendored(dir.parent().unwrap().to_path_buf())).build(&sub).unwrap();
    if validator.is_valid(node) {
        return;
    }
    let before = problems.len();
    for key in ["members", "body", "params", "fixes"] {
        if let Some(items) = node[key].as_array() {
            for item in items {
                explain(schema, item, dir, problems);
            }
        }
    }
    if problems.len() == before {
        let line = node["span"]["line"].as_u64().unwrap_or(0);
        for error in validator.iter_errors(node).take(3) {
            let text = error.to_string();
            let text = if text.len() > 300 { format!("{}…", &text[..300]) } else { text };
            problems.push(format!("  {kind} at line {line} ({def}): {text} at {}", error.instance_path()));
        }
    }
}

/// The other specifications' schemas, from the folder beside ATD's, and nothing from the network.
struct Vendored(PathBuf);

impl jsonschema::Retrieve for Vendored {
    fn retrieve(&self, uri: &jsonschema::Uri<String>) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let name = uri.as_str().rsplit('/').next().unwrap_or("").trim_end_matches('#');
        let language = name.trim_end_matches(".schema.json");
        Ok(serde_json::from_str(&std::fs::read_to_string(self.0.join(language).join(name))?)?)
    }
}

#[test]
fn every_example_tree_is_valid_against_the_schema() {
    let dir = spec_dir();
    let schema: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("atd.schema.json")).unwrap()).unwrap();
    let mut tree_schema = schema.clone();
    tree_schema["$ref"] = Value::String("#/$defs/Tree".into());
    let validator = jsonschema::options()
        .with_retriever(Vendored(dir.parent().unwrap().to_path_buf()))
        .build(&tree_schema)
        .expect("the schema compiles");
    for path in files(&dir) {
        let text = std::fs::read_to_string(&path).unwrap();
        let parse = adp_atd_syntax::parse(&text);
        let name = path.file_name().unwrap().to_string_lossy();
        let tree = adp_atd_syntax::lower(&parse, &name).expect("a tree");
        if validator.is_valid(&tree) {
            continue;
        }
        // Say where: each construct against the `$defs` entry of its kind.
        let mut problems = Vec::new();
        explain(&schema, &tree["description"], &dir, &mut problems);
        panic!("{name} is not valid against $defs/Tree:\n{}", problems.join("\n"));
    }
}

#[test]
fn a_broken_file_still_parses_after_the_error() {
    let text = "atd 0.1\ndiagram a/b version \"1\" {\n  model {\n    element Task {\n      attribute name required\n    }\n  }\n  constraint c for Task {\n    rule self.name != \"\"\n  }\n}\n";
    let parse = adp_atd_syntax::parse(text);
    assert_eq!(parse.errors.len(), 1, "{:?}", parse.errors);
    assert_eq!(parse.source(), text);
    let tree = adp_atd_syntax::lower(&parse, "broken.atd").expect("the rest of the file has a tree");
    let members = tree["description"]["members"].as_array().unwrap();
    assert!(members.iter().any(|m| m["kind"] == "constraint"), "{members:#?}");
}
