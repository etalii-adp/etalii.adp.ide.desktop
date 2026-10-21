//! The standalone host's plugin tests at 8b6ccfc, ported: `Plugins/PluginBody.Tests.cs`,
//! `Plugins/PluginOnlyChanges.Tests.cs`, `Plugins/FolderBody.Tests.cs`, `Plugins/FolderBodySettle.Tests.cs`,
//! `Plugins/FolderBodyUnopenedFile.Tests.cs` and the plugin case of `Routing/Templates.Tests.cs`. No plugin is
//! implemented by this crate, so each runs with a hand-written fake, and the folder tests over a folder on disk. The
//! standalone host's manual clock becomes instants the test moves; its `Changed` event the value `poll` returns.

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use adp_cel::{Map, Value};

use super::*;
use crate::documents::{self, ProblemSeverity};
use crate::history::{DRIFT_UNDO, OpenBody, UndoResult};
use crate::model::{FblElement, FblModel, FblOptions, Finding, FindingSeverity, SourceLocation, codes};
use crate::planning::{ModelChange, PlanResult};
use crate::splice::{Operation, Splice};
use crate::text::Span;

// ---- support ----

fn vendored(document: &str, binding: &str) -> Rc<FblBinding> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor/etalii.adp/specifications/fbl")
        .join(document);
    let (problems, loaded) = documents::load_file(&path);
    assert!(
        !problems.iter().any(|p| p.severity == ProblemSeverity::Error),
        "{document}: {problems:?}"
    );
    Rc::new(loaded.unwrap().binding(binding).unwrap().clone())
}

fn loaded(json: &str, binding: &str) -> Rc<FblBinding> {
    let (problems, document) = documents::load(json.as_bytes());
    assert!(
        !problems.iter().any(|p| p.severity == ProblemSeverity::Error),
        "{problems:?}"
    );
    Rc::new(document.unwrap().binding(binding).unwrap().clone())
}

fn map(pairs: &[(&str, &str)]) -> Map {
    let mut map = Map::default();
    for (k, v) in pairs {
        map.insert(*k, Value::from(*v));
    }
    map
}

fn element(id: &str, type_name: &str, rule: &str, attributes: Map, span: Span, line: usize) -> FblElement {
    FblElement {
        id: id.into(),
        id_is_stored: true,
        type_name: type_name.into(),
        rule: rule.into(),
        is_relation: false,
        attributes,
        parent_id: None,
        parent_slot: None,
        source: None,
        target: None,
        own_span: span,
        line,
        file: None,
    }
}

/// A folder on disk, removed when dropped.
struct TemporaryFolder(tempfile::TempDir);

impl TemporaryFolder {
    fn new() -> Self {
        TemporaryFolder(tempfile::tempdir().unwrap())
    }

    fn path(&self) -> PathBuf {
        std::path::absolute(self.0.path()).unwrap()
    }

    fn full(&self, relative: &str) -> PathBuf {
        self.path().join(relative.replace('/', std::path::MAIN_SEPARATOR_STR))
    }

    /// Writes a file, its folders created, and gives its full path.
    fn write(&self, relative: &str, text: &str) -> PathBuf {
        let path = self.full(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        path
    }

    /// Links a folder at `relative` to `target`: a junction on Windows, which needs no privilege, a symbolic link
    /// elsewhere.
    fn link(&self, relative: &str, target: &Path) {
        let path = self.full(relative);
        #[cfg(windows)]
        {
            let status = std::process::Command::new("cmd")
                .args(["/C", "mklink", "/J"])
                .arg(&path)
                .arg(target)
                .output()
                .unwrap();
            assert!(status.status.success(), "{status:?}");
        }
        #[cfg(not(windows))]
        std::os::unix::fs::symlink(target, &path).unwrap();
    }
}

// ---- PluginBody.Tests.cs ----

const TURTLE_ID: &str = "net.etalii.adp.w3c.turtle";

/// A fake persistence plugin: every line `label <text>` is an element `line<n>` with a writable `label`; it plans
/// label changes and refuses everything else.
struct FakePlugin {
    id: String,
    plans: Cell<usize>,
}

impl FakePlugin {
    fn new(id: &str) -> Rc<FakePlugin> {
        Rc::new(FakePlugin {
            id: id.into(),
            plans: Cell::new(0),
        })
    }
}

impl PersistencePlugin for FakePlugin {
    fn id(&self) -> &str {
        &self.id
    }

    fn read(&self, request: &PluginReadRequest) -> PluginReadResult {
        let bytes = &request.files[0].bytes;
        let mut elements = Vec::new();
        let mut start = 0;
        let mut number = 0;
        for i in 0..=bytes.len() {
            if i < bytes.len() && bytes[i] != b'\n' {
                continue;
            }
            if i > start {
                number += 1;
                let text = String::from_utf8_lossy(&bytes[start..i]).into_owned();
                elements.push(element(
                    &format!("line{number}"),
                    "Line",
                    "line",
                    map(&[("label", &text[6..])]),
                    Span::new(start, i),
                    number,
                ));
            }
            start = i + 1;
        }
        PluginReadResult::new(elements, Vec::new(), false)
    }

    fn plan(&self, request: &PluginPlanRequest<'_>) -> PluginPlanResult {
        self.plans.set(self.plans.get() + 1);
        let ModelChange::Set { id, attributes } = request.change else {
            return PluginPlanResult::Refused("The fake plugin removes nothing.".into());
        };
        let element = request.last.find(id).unwrap();
        let start = element.own_span.start + 6;
        PluginPlanResult::Planned(vec![PluginSplice {
            file: String::new(),
            splice: Splice {
                operation: Operation::ReplaceValue,
                start,
                end: element.own_span.end,
                text: attributes.get("label").and_then(Value::as_str).unwrap().to_owned(),
            },
        }])
    }

    fn template(&self, request: &PluginTemplateRequest) -> Vec<u8> {
        format!("template for {} ({})", request.name, request.placeholders["base"]).into_bytes()
    }
}

fn remove(id: &str) -> ModelChange {
    ModelChange::Remove { id: id.into() }
}

fn set(id: &str, attributes: Map) -> ModelChange {
    ModelChange::Set {
        id: id.into(),
        attributes,
    }
}

#[test]
fn a_missing_plugin_opens_the_body_read_only_with_a_finding() {
    let turtle = vendored("w3c-turtle.fbl", "turtle");

    let mut body = PluginBody::open(b"ex:a ex:b ex:c .\n".to_vec(), turtle, None, "a.ttl").unwrap();

    assert!(body.is_read_only());
    let model = body.model();
    assert_eq!(model.findings.len(), 1);
    let finding = &model.findings[0];
    assert_eq!(finding.code, codes::PLUGIN_MISSING);
    assert!(finding.message.contains(TURTLE_ID));
    assert!(matches!(body.change(&remove("x")), PlanResult::Refused(_)));
    assert_eq!(
        body.save(|_| panic!("A read-only body is never written.")),
        Err("A read-only body is never written.".into())
    );
}

#[test]
fn a_plugin_with_another_id_counts_as_missing() {
    let body = PluginBody::open(
        b"x\n".to_vec(),
        vendored("w3c-turtle.fbl", "turtle"),
        Some(FakePlugin::new("net.example.other")),
        "body",
    )
    .unwrap();

    assert!(body.is_read_only());
    let findings = body.model().findings;
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].code, codes::PLUGIN_MISSING);
}

#[test]
fn the_plugins_splices_are_applied_recorded_and_undone() {
    let original = b"label a\nlabel b\n".to_vec();
    let mut body = PluginBody::open(
        original.clone(),
        vendored("w3c-turtle.fbl", "turtle"),
        Some(FakePlugin::new(TURTLE_ID)),
        "body",
    )
    .unwrap();

    let result = body.change(&set("line1", map(&[("label", "renamed")])));

    // The host applied the plugin's splice and read again through the plugin.
    assert!(matches!(result, PlanResult::Planned(_)));
    assert_eq!(body.bytes(), b"label renamed\nlabel b\n");
    assert_eq!(
        body.model().find("line1").unwrap().attributes.get("label"),
        Some(&Value::from("renamed"))
    );
    assert_eq!(body.undo(Some(b"drifted")), UndoResult::Refused(DRIFT_UNDO.into()));
    assert!(matches!(body.undo(None), UndoResult::Done(_)));
    assert_eq!(body.bytes(), original.as_slice());
}

#[test]
fn a_plugins_refusal_writes_nothing() {
    let original = b"label a\n".to_vec();
    let mut body = PluginBody::open(
        original.clone(),
        vendored("w3c-turtle.fbl", "turtle"),
        Some(FakePlugin::new(TURTLE_ID)),
        "body",
    )
    .unwrap();

    let result = body.change(&remove("line1"));

    assert_eq!(result, PlanResult::Refused("The fake plugin removes nothing.".into()));
    assert_eq!(body.bytes(), original.as_slice());
}

#[test]
fn a_read_only_bindings_plugin_is_never_asked_to_plan() {
    // The Helm chart binding is read-only.
    let plugin = FakePlugin::new("net.etalii.adp.helm.chartFolder");
    let mut body = PluginBody::open(
        b"label a\n".to_vec(),
        vendored("helm-chart.fbl", "chart"),
        Some(plugin.clone()),
        "body",
    )
    .unwrap();

    let result = body.change(&set("line1", map(&[("label", "x")])));

    assert!(matches!(result, PlanResult::Refused(_)));
    assert_eq!(plugin.plans.get(), 0);
}

#[test]
fn a_binding_read_by_its_declared_rules_is_not_opened_as_a_plugin_body() {
    let timeline = vendored("timeline.fbl", "timeline");

    let opened = PluginBody::open(Vec::new(), timeline, None, "plan.tml");

    assert_eq!(
        opened.err(),
        Some("The binding 'timeline' is read by its declared rules, not by a plugin.".into())
    );
}

#[test]
fn overlapping_splices_are_the_plugins_error() {
    struct Overlapping;
    impl PersistencePlugin for Overlapping {
        fn id(&self) -> &str {
            TURTLE_ID
        }
        fn read(&self, _: &PluginReadRequest) -> PluginReadResult {
            PluginReadResult::default()
        }
        fn plan(&self, _: &PluginPlanRequest<'_>) -> PluginPlanResult {
            let splice = |start, end| PluginSplice {
                file: String::new(),
                splice: Splice {
                    operation: Operation::ReplaceValue,
                    start,
                    end,
                    text: "x".into(),
                },
            };
            PluginPlanResult::Planned(vec![splice(2, 4), splice(0, 3)])
        }
        fn template(&self, _: &PluginTemplateRequest) -> Vec<u8> {
            Vec::new()
        }
    }
    let body = PluginBody::open(
        b"label a\n".to_vec(),
        vendored("w3c-turtle.fbl", "turtle"),
        Some(Rc::new(Overlapping)),
        "body",
    )
    .unwrap();

    let result = body.plan(&remove("x"));

    assert_eq!(
        result,
        PlanResult::Invalid(
            "The plugin planned overlapping splices: Splice { Operation = ReplaceValue, Start = 0, End = 3, Text = x } and Splice { Operation = ReplaceValue, Start = 2, End = 4, Text = x }.".into()
        )
    );
}

// ---- PluginOnlyChanges.Tests.cs ----

const TIMELINE: &str = "elements:\n  - id: a\n    label: Alpha\n    start: 2026-01-01\n";

/// A plugin that records the change it is asked to plan, and plans it as no splice.
struct ChangeRecordingPlugin {
    received: RefCell<Option<ModelChange>>,
}

impl PersistencePlugin for ChangeRecordingPlugin {
    fn id(&self) -> &str {
        TURTLE_ID
    }

    fn read(&self, _: &PluginReadRequest) -> PluginReadResult {
        PluginReadResult::default()
    }

    fn plan(&self, request: &PluginPlanRequest<'_>) -> PluginPlanResult {
        *self.received.borrow_mut() = Some(request.change.clone());
        PluginPlanResult::Planned(Vec::new())
    }

    fn template(&self, _: &PluginTemplateRequest) -> Vec<u8> {
        Vec::new()
    }
}

fn change_of(name: &str) -> ModelChange {
    match name {
        "move" => ModelChange::Move {
            id: "a".into(),
            new_parent_id: None,
            index: 0,
        },
        "move to another parent" => ModelChange::Move {
            id: "a".into(),
            new_parent_id: Some("a".into()),
            index: 0,
        },
        "retype" => ModelChange::Retype {
            id: "a".into(),
            type_name: "Period".into(),
            attributes: map(&[("end", "2026-02-01")]),
        },
        _ => ModelChange::Add {
            type_name: "Moment".into(),
            id: Some("b".into()),
            attributes: map(&[("label", "Beta"), ("start", "2026-02-01")]),
            parent_id: None,
            index: Some(0),
            slot: None,
        },
    }
}

fn timeline_body(text: &str) -> OpenBody {
    OpenBody::open(
        text.as_bytes().to_vec(),
        vendored("timeline.fbl", "timeline"),
        Rc::new(FblOptions {
            file_name: "plan.tml".into(),
            ..FblOptions::default()
        }),
    )
}

#[test]
fn a_plugin_body_hands_the_change_to_its_plugin_as_it_was_made() {
    for name in ["move", "move to another parent", "retype", "add at an index"] {
        let plugin = Rc::new(ChangeRecordingPlugin {
            received: RefCell::new(None),
        });
        let body = PluginBody::open(
            b"label a\n".to_vec(),
            vendored("w3c-turtle.fbl", "turtle"),
            Some(plugin.clone()),
            "body",
        )
        .unwrap();
        let change = change_of(name);

        let result = body.plan(&change);

        assert!(matches!(result, PlanResult::Planned(_)), "{name}");
        assert_eq!(
            format!("{:?}", plugin.received.borrow().as_ref().unwrap()),
            format!("{change:?}"),
            "{name}"
        );
    }
}

#[test]
fn a_declared_binding_refuses_the_change_and_writes_nothing() {
    for name in ["move to another parent", "retype", "add at an index"] {
        let mut body = timeline_body(TIMELINE);

        let result = body.change(&change_of(name));

        let PlanResult::Refused(reason) = result else {
            panic!("{name}: {result:?}");
        };
        assert!(reason.contains("only a persistence plugin can"), "{name}: {reason}");
        assert_eq!(body.bytes(), TIMELINE.as_bytes());
    }
}

#[test]
fn a_declared_binding_moves_an_element_among_its_siblings_carrying_its_bytes() {
    // Two moments, the second with a comment of its own above it.
    let two = "elements:\n  - id: a\n    label: Alpha\n    start: 2026-01-01\n  # kept with b\n  - id: b\n    label: Beta\n    start: 2026-02-01\n";
    let mut body = timeline_body(two);

    // b becomes the first.
    let result = body.change(&ModelChange::Move {
        id: "b".into(),
        new_parent_id: None,
        index: 0,
    });

    // One removal and one insertion, and b's comment went with it.
    let PlanResult::Planned(edit) = result else {
        panic!("{result:?}");
    };
    assert_eq!(
        edit.splices.iter().map(|s| s.operation).collect::<Vec<_>>(),
        [Operation::InsertEntry, Operation::RemoveEntry]
    );
    assert_eq!(
        String::from_utf8_lossy(body.bytes()),
        "elements:\n  # kept with b\n  - id: b\n    label: Beta\n    start: 2026-02-01\n  - id: a\n    label: Alpha\n    start: 2026-01-01\n"
    );
}

#[test]
fn a_declared_binding_moving_an_element_to_where_it_is_writes_nothing() {
    let mut body = timeline_body(TIMELINE);

    let result = body.change(&change_of("move"));

    let PlanResult::Planned(edit) = result else {
        panic!("{result:?}");
    };
    assert!(edit.splices.is_empty());
    assert_eq!(body.bytes(), TIMELINE.as_bytes());
}

#[test]
fn a_declared_binding_still_adds_without_an_index() {
    let mut body = timeline_body(TIMELINE);

    let result = body.change(&ModelChange::Add {
        type_name: "Moment".into(),
        id: Some("b".into()),
        attributes: map(&[("label", "Beta"), ("start", "2026-02-01")]),
        parent_id: None,
        index: None,
        slot: None,
    });

    assert!(matches!(result, PlanResult::Planned(_)), "{result:?}");
    assert!(String::from_utf8_lossy(body.bytes()).contains("Beta"));
}

// ---- Templates.Tests.cs ----

#[test]
fn a_plugin_without_template_text_is_asked_for_one() {
    let chart = vendored("helm-chart.fbl", "chart");
    let plugin = FakePlugin::new("net.etalii.adp.helm.chartFolder");

    let bytes = produce(
        &chart,
        Some("helm/chart"),
        "web.yaml",
        &|_| "x".into(),
        Some(plugin.as_ref()),
    );

    assert_eq!(bytes, Some(b"template for web.yaml (web)".to_vec()));
    assert_eq!(
        produce(&chart, Some("helm/chart"), "web.yaml", &|_| "x".into(), None),
        None
    );
}

// ---- FolderBody.Tests.cs ----

const FOLDER_PLUGIN_ID: &str = "net.example.folder";

/// A folder binding of no tool type: `index.txt` recognises it, and it reads that file and everything under
/// `pages/`, with `settle` and `ignore` as each test file of the standalone host gives them.
fn folder_binding(read_only: Option<&str>, settle: Option<i64>, ignore: bool) -> Rc<FblBinding> {
    let read_only = read_only
        .map(|r| format!(", \"readOnly\": \"{r}\""))
        .unwrap_or_default();
    let settle = settle.map(|s| format!(",\n    \"settle\": {s}")).unwrap_or_default();
    let ignore = if ignore {
        ",\n    \"ignore\": [\"pages/**/.hidden\"]"
    } else {
        ""
    };
    loaded(
        &format!(
            r#"{{ "fbl": "0.1", "bindings": {{ "notes": {{
  "claims": {{ "names": ["index.txt"] }},
  "body": {{
    "kind": "folder",
    "recognise": {{ "all": ["index.txt"] }},
    "files": [
      {{ "name": "index", "glob": "index.txt", "family": "lines" }},
      {{ "name": "pages", "glob": "pages/**", "family": "lines" }}
    ]{ignore}{settle}
  }},
  "reader": {{ "plugin": "{FOLDER_PLUGIN_ID}", "version": "^0.1.0" }}{read_only}
}} }} }}"#
        ),
        "notes",
    )
}

fn binding() -> Rc<FblBinding> {
    folder_binding(None, None, true)
}

/// A recording folder plugin: every file it is handed is one element, its id the file's path and its file the
/// same, at line 3; it records every request and plans nothing.
struct RecordingFolderPlugin {
    id: String,
    reads: RefCell<Vec<PluginReadRequest>>,
    plans: Cell<usize>,
    findings: Vec<Finding>,
    watched: Vec<String>,
}

impl RecordingFolderPlugin {
    fn new(id: &str) -> Self {
        RecordingFolderPlugin {
            id: id.into(),
            reads: RefCell::new(Vec::new()),
            plans: Cell::new(0),
            findings: Vec::new(),
            watched: Vec::new(),
        }
    }

    fn reads(&self) -> usize {
        self.reads.borrow().len()
    }

    fn paths(&self, read: usize) -> Vec<String> {
        self.reads.borrow()[read]
            .files
            .iter()
            .map(|f| f.relative_path.clone())
            .collect()
    }
}

impl PersistencePlugin for RecordingFolderPlugin {
    fn id(&self) -> &str {
        &self.id
    }

    fn read(&self, request: &PluginReadRequest) -> PluginReadResult {
        self.reads.borrow_mut().push(request.clone());
        let elements = request
            .files
            .iter()
            .map(|file| FblElement {
                file: Some(file.relative_path.clone()),
                ..element(
                    &file.relative_path,
                    "Page",
                    "page",
                    map(&[("text", &String::from_utf8_lossy(&file.bytes))]),
                    Span::new(0, file.bytes.len()),
                    3,
                )
            })
            .collect();
        PluginReadResult::new(elements, self.findings.clone(), false)
    }

    fn plan(&self, _: &PluginPlanRequest<'_>) -> PluginPlanResult {
        self.plans.set(self.plans.get() + 1);
        PluginPlanResult::Refused("The recording plugin plans nothing.".into())
    }

    fn template(&self, _: &PluginTemplateRequest) -> Vec<u8> {
        panic!("A folder is never created here.")
    }

    fn watch(&self, _: &PluginReadResult) -> Vec<String> {
        self.watched.clone()
    }
}

fn open(folder: &Path, binding: Rc<FblBinding>, plugin: &Rc<RecordingFolderPlugin>) -> FolderBody {
    let plugin: Rc<dyn PersistencePlugin> = plugin.clone();
    FolderBody::open(folder, binding, Some(plugin), Rc::new(FblOptions::default()), None).unwrap()
}

#[test]
fn every_selected_file_is_handed_to_the_plugin_in_one_read_in_ordinal_order() {
    // Written out of order, with an ignored and an unselected file.
    let folder = TemporaryFolder::new();
    folder.write("pages/b/c.txt", "c");
    folder.write("pages/a.txt", "a");
    folder.write("index.txt", "index");
    folder.write("pages/Z.txt", "Z");
    folder.write("pages/b/.hidden", "never");
    folder.write("notes.md", "never");
    let plugin = Rc::new(RecordingFolderPlugin::new(FOLDER_PLUGIN_ID));

    let body = open(&folder.path(), binding(), &plugin);

    assert_eq!(plugin.reads(), 1);
    assert_eq!(
        plugin.paths(0),
        ["index.txt", "pages/Z.txt", "pages/a.txt", "pages/b/c.txt"]
    );
    let texts: Vec<String> = plugin.reads.borrow()[0]
        .files
        .iter()
        .map(|f| String::from_utf8_lossy(&f.bytes).into_owned())
        .collect();
    assert_eq!(texts, ["index", "Z", "a", "c"]);
    assert_eq!(body.model().elements.len(), 4);
    assert!(!body.model().unreadable);
}

#[test]
fn a_reparse_point_is_never_followed() {
    let folder = TemporaryFolder::new();
    folder.write("index.txt", "index");
    folder.write("pages/a.txt", "a");
    let outside = TemporaryFolder::new();
    outside.write("secret.txt", "outside");
    folder.link("pages/linked", &outside.path());
    let plugin = Rc::new(RecordingFolderPlugin::new(FOLDER_PLUGIN_ID));

    open(&folder.path(), binding(), &plugin);

    assert_eq!(plugin.reads(), 1);
    assert_eq!(plugin.paths(0), ["index.txt", "pages/a.txt"]);
}

#[test]
fn a_folder_that_fails_recognise_or_is_gone_is_unreadable_and_the_plugin_is_not_asked() {
    // No index.txt.
    let folder = TemporaryFolder::new();
    folder.write("pages/a.txt", "a");
    let plugin = Rc::new(RecordingFolderPlugin::new(FOLDER_PLUGIN_ID));

    let mut body = open(&folder.path(), binding(), &plugin);

    let model = body.model();
    assert!(model.unreadable);
    assert!(body.is_read_only());
    assert!(model.elements.is_empty());
    assert_eq!(model.findings.len(), 1);
    assert_eq!(model.findings[0].code, codes::UNPARSEABLE);
    assert_eq!(plugin.reads(), 0);

    // A folder recognised once and gone since is unreadable when read again.
    folder.write("index.txt", "index");
    body.reread();
    assert!(!body.model().unreadable);
    std::fs::remove_dir_all(folder.path()).unwrap();
    body.reread();
    let model = body.model();
    assert!(model.unreadable);
    assert_eq!(model.findings.len(), 1);
    assert!(model.findings[0].message.contains("no longer exists"));
    assert_eq!(plugin.reads(), 1);
}

#[test]
fn a_missing_plugin_opens_the_folder_read_only_with_a_finding() {
    let folder = TemporaryFolder::new();
    folder.write("index.txt", "index");
    let other = Rc::new(RecordingFolderPlugin::new("net.example.other"));

    let body = open(&folder.path(), binding(), &other);

    assert!(body.is_read_only());
    let model = body.model();
    assert!(!model.unreadable);
    assert_eq!(model.findings.len(), 1);
    assert_eq!(model.findings[0].code, codes::PLUGIN_MISSING);
    assert!(model.findings[0].message.contains(FOLDER_PLUGIN_ID));
    assert_eq!(other.reads(), 0);
    assert!(matches!(body.plan(&remove("index.txt")), PlanResult::Refused(_)));
}

#[test]
fn a_finding_of_a_folder_subject_has_a_file_and_a_line() {
    // The plugin reports three findings about pages/a.txt, two of them without a file.
    let folder = TemporaryFolder::new();
    folder.write("index.txt", "index");
    folder.write("pages/a.txt", "a");
    let subject = Some(vec![(SUBJECT_DETAIL.to_owned(), "pages/a.txt".to_owned())]);
    let about = |code: &str, message: &str, location: Option<SourceLocation>| Finding {
        code: code.into(),
        severity: FindingSeverity::Warning,
        message: message.into(),
        location,
        detail: subject.clone(),
    };
    let location = |file: &str, line, column, length| SourceLocation {
        file: file.into(),
        line,
        column,
        length,
    };
    let plugin = Rc::new(RecordingFolderPlugin {
        findings: vec![
            about("x.no-location", "No location.", None),
            about("x.no-file", "A line, no file.", Some(location("", 7, 2, 3))),
            about("x.own-file", "Its own file.", Some(location("index.txt", 1, 1, 0))),
        ],
        ..RecordingFolderPlugin::new(FOLDER_PLUGIN_ID)
    });

    let body = open(&folder.path(), binding(), &plugin);

    // Every element carries its file, and every finding has a file and a line.
    let model = body.model();
    assert_eq!(
        model
            .elements
            .iter()
            .map(|e| e.file.clone().unwrap())
            .collect::<Vec<_>>(),
        ["index.txt", "pages/a.txt"]
    );
    assert_eq!(
        model
            .findings
            .iter()
            .map(|f| f.location.clone().unwrap())
            .collect::<Vec<_>>(),
        [
            location("pages/a.txt", 3, 1, 0),
            location("pages/a.txt", 7, 2, 3),
            location("index.txt", 1, 1, 0)
        ]
    );
}

#[test]
fn a_file_over_the_size_limit_is_handed_by_its_path_with_no_bytes() {
    let folder = TemporaryFolder::new();
    folder.write("index.txt", "index");
    folder.write("pages/large.txt", &"x".repeat(11));
    folder.write("pages/small.txt", &"x".repeat(10));
    let plugin = Rc::new(RecordingFolderPlugin::new(FOLDER_PLUGIN_ID));
    let options = FblOptions {
        max_body_bytes: 10,
        ..FblOptions::default()
    };

    let dynamic: Rc<dyn PersistencePlugin> = plugin.clone();
    FolderBody::open(&folder.path(), binding(), Some(dynamic), Rc::new(options), None).unwrap();

    let reads = plugin.reads.borrow();
    assert_eq!(reads.len(), 1);
    let files = &reads[0].files;
    assert_eq!(
        files.iter().map(|f| f.relative_path.as_str()).collect::<Vec<_>>(),
        ["index.txt", "pages/large.txt", "pages/small.txt"]
    );
    assert_eq!(
        files.iter().map(|f| f.is_oversized).collect::<Vec<_>>(),
        [false, true, false]
    );
    assert_eq!(files.iter().map(|f| f.bytes.len()).collect::<Vec<_>>(), [5, 0, 10]);
}

#[test]
fn a_supplementary_entry_is_handed_with_no_bytes_beside_the_files() {
    // An empty folder, which no file rule can select.
    let folder = TemporaryFolder::new();
    folder.write("index.txt", "index");
    std::fs::create_dir_all(folder.full("pages/empty")).unwrap();
    let plugin = Rc::new(RecordingFolderPlugin::new(FOLDER_PLUGIN_ID));
    let options = FblOptions {
        supplementary_entries: Some(Rc::new(|root: &Path| {
            std::fs::read_dir(root.join("pages"))
                .unwrap()
                .flatten()
                .filter(|e| e.path().is_dir())
                .map(|e| format!("pages/{}/", e.file_name().to_string_lossy()))
                .collect()
        })),
        ..FblOptions::default()
    };

    let dynamic: Rc<dyn PersistencePlugin> = plugin.clone();
    FolderBody::open(&folder.path(), binding(), Some(dynamic), Rc::new(options), None).unwrap();

    assert_eq!(plugin.reads(), 1);
    assert_eq!(plugin.reads.borrow()[0].supplementary_entries, ["pages/empty/"]);
    assert_eq!(plugin.paths(0), ["index.txt"]);
}

#[test]
fn a_read_only_folder_refuses_every_change_with_the_bindings_reason_and_never_plans() {
    let folder = TemporaryFolder::new();
    folder.write("index.txt", "index");
    let plugin = Rc::new(RecordingFolderPlugin::new(FOLDER_PLUGIN_ID));
    let body = open(
        &folder.path(),
        folder_binding(Some("Change it in its files."), None, true),
        &plugin,
    );
    let changes = [
        set("index.txt", map(&[("text", "x")])),
        remove("index.txt"),
        ModelChange::Add {
            type_name: "Page".into(),
            id: None,
            attributes: Map::default(),
            parent_id: None,
            index: None,
            slot: None,
        },
    ];

    let results: Vec<PlanResult> = changes.iter().map(|c| body.plan(c)).collect();

    assert!(body.is_read_only());
    for result in results {
        assert_eq!(result, PlanResult::Refused("Change it in its files.".into()));
    }
    assert_eq!(plugin.plans.get(), 0);

    // The same folder under a writable binding does ask the plugin, so the count above can fail.
    let writable = open(&folder.path(), binding(), &plugin);
    assert_eq!(
        writable.plan(&changes[1]),
        PlanResult::Refused("The recording plugin plans nothing.".into())
    );
    assert_eq!(plugin.plans.get(), 1);
}

#[test]
fn a_plan_the_plugin_makes_is_refused_because_this_host_writes_no_folder() {
    struct Planning;
    impl PersistencePlugin for Planning {
        fn id(&self) -> &str {
            FOLDER_PLUGIN_ID
        }
        fn read(&self, _: &PluginReadRequest) -> PluginReadResult {
            PluginReadResult::default()
        }
        fn plan(&self, _: &PluginPlanRequest<'_>) -> PluginPlanResult {
            PluginPlanResult::Planned(Vec::new())
        }
        fn template(&self, _: &PluginTemplateRequest) -> Vec<u8> {
            Vec::new()
        }
    }
    let folder = TemporaryFolder::new();
    folder.write("index.txt", "index");
    let body = FolderBody::open(
        &folder.path(),
        binding(),
        Some(Rc::new(Planning)),
        Rc::new(FblOptions::default()),
        None,
    )
    .unwrap();

    assert_eq!(
        body.plan(&remove("index.txt")),
        PlanResult::Refused("The files of a folder are not written by this host, so the change is not applied.".into())
    );
}

#[test]
fn a_file_binding_or_a_declared_one_is_not_opened_as_a_folder_body() {
    let folder = TemporaryFolder::new();
    let options = Rc::new(FblOptions::default());

    let file = FolderBody::open(
        &folder.path(),
        vendored("w3c-turtle.fbl", "turtle"),
        None,
        options.clone(),
        None,
    );
    let declared = FolderBody::open(
        &folder.path(),
        loaded(
            r#"{ "fbl": "0.1", "bindings": { "notes": {
  "claims": { "names": ["index.txt"] },
  "body": { "kind": "folder", "recognise": { "all": ["index.txt"] },
            "files": [ { "name": "index", "glob": "index.txt", "family": "lines" } ] },
  "reader": "declared",
  "elements": [ { "name": "line", "type": "Line", "files": ["index"], "line": "^(?<label>.+)$",
                  "attributes": { "label": { "group": "label" } } } ]
} } }"#,
            "notes",
        ),
        None,
        options,
        None,
    );

    assert_eq!(
        file.err(),
        Some("The binding 'turtle' reads a file, not a folder.".into())
    );
    assert_eq!(
        declared.err(),
        Some("The binding 'notes' is read by its declared rules, not by a plugin.".into())
    );
}

// ---- FolderBodySettle.Tests.cs ----

const SETTLE: u64 = 250;

fn settle_binding() -> Rc<FblBinding> {
    folder_binding(None, Some(SETTLE as i64), true)
}

fn notes() -> TemporaryFolder {
    let folder = TemporaryFolder::new();
    folder.write("index.txt", "index");
    folder.write("pages/a.txt", "a");
    folder.write("pages/b.txt", "b");
    folder
}

/// A clock that moves only when the test moves it, polling the body whenever its deadline falls due on the way, as
/// the host's timer would; it keeps every change a poll returned.
struct ManualTime {
    now: Instant,
    changes: Vec<FolderBodyChange>,
}

impl ManualTime {
    fn new() -> Self {
        ManualTime {
            now: Instant::now(),
            changes: Vec::new(),
        }
    }

    fn advance(&mut self, body: &mut FolderBody, by: Duration) {
        let end = self.now + by;
        while let Some(due) = body.deadline().filter(|due| *due <= end) {
            self.now = due;
            if let Some(change) = body.poll(due) {
                self.changes.push(change);
            }
        }
        self.now = end;
    }

    fn millis(&mut self, body: &mut FolderBody, ms: u64) {
        self.advance(body, Duration::from_millis(ms));
    }
}

fn change(folder_read: bool, registration: bool) -> FolderBodyChange {
    FolderBodyChange {
        folder_read,
        registration,
    }
}

#[test]
fn a_burst_of_changes_becomes_one_read_after_the_settle_delay() {
    let folder = notes();
    let mut time = ManualTime::new();
    let plugin = Rc::new(RecordingFolderPlugin::new(FOLDER_PLUGIN_ID));
    let mut body = open(&folder.path(), settle_binding(), &plugin);

    // Four changes of three files, each sooner than the delay after the one before.
    for file in ["pages/a.txt", "pages/b.txt", "index.txt", "pages/a.txt"] {
        let path = folder.write(file, &format!("{file} changed"));
        body.notify(&path, time.now);
        time.millis(&mut body, 100);
    }
    time.millis(&mut body, SETTLE - 101);
    let reads_before_the_delay = plugin.reads();
    time.millis(&mut body, 1);

    // The opening read, then one more, once.
    assert_eq!(reads_before_the_delay, 1);
    assert_eq!(plugin.reads(), 2);
    let a = plugin.reads.borrow()[1]
        .files
        .iter()
        .find(|f| f.relative_path == "pages/a.txt")
        .map(|f| String::from_utf8_lossy(&f.bytes).into_owned());
    assert_eq!(a.as_deref(), Some("pages/a.txt changed"));
    assert_eq!(time.changes, [change(true, false)]);
    time.advance(&mut body, Duration::from_secs(10));
    assert_eq!(plugin.reads(), 2);
    assert_eq!(body.deadline(), None);
}

#[test]
fn a_change_outside_every_dependency_causes_no_read() {
    let folder = notes();
    let outside = TemporaryFolder::new();
    let mut time = ManualTime::new();
    let plugin = Rc::new(RecordingFolderPlugin::new(FOLDER_PLUGIN_ID));
    let mut body = open(&folder.path(), settle_binding(), &plugin);

    // A file no rule selects, a folder no glob covers, an ignored file, and a file elsewhere.
    body.notify(&folder.write("notes.md", "x"), time.now);
    body.notify(&folder.write("other/c.txt", "x"), time.now);
    body.notify(&folder.write("pages/b/.hidden", "x"), time.now);
    body.notify(&outside.write("pages/a.txt", "x"), time.now);
    time.advance(&mut body, Duration::from_secs(10));

    assert_eq!(plugin.reads(), 1);
    assert!(time.changes.is_empty());
    assert_eq!(body.deadline(), None);
    let other = folder.full("other").to_string_lossy().into_owned();
    assert!(
        !body
            .dependencies()
            .iter()
            .any(|p| p.to_string_lossy().starts_with(&other))
    );
}

#[test]
fn a_new_file_in_a_folder_a_glob_covers_causes_one_read() {
    // pages/deep exists and holds no file yet.
    let folder = notes();
    std::fs::create_dir_all(folder.full("pages/deep")).unwrap();
    let mut time = ManualTime::new();
    let plugin = Rc::new(RecordingFolderPlugin::new(FOLDER_PLUGIN_ID));
    let mut body = open(&folder.path(), settle_binding(), &plugin);

    let dependencies = body.dependencies();
    body.notify(&folder.write("pages/deep/new.txt", "new"), time.now);
    time.millis(&mut body, SETTLE);

    // The folder was named for the host to watch, and the new file is read.
    assert_eq!(
        dependencies,
        [
            folder.path(),
            folder.full("index.txt"),
            folder.full("pages"),
            folder.full("pages/a.txt"),
            folder.full("pages/b.txt"),
            folder.full("pages/deep"),
        ]
    );
    assert_eq!(plugin.reads(), 2);
    assert!(plugin.paths(1).contains(&"pages/deep/new.txt".to_owned()));
}

#[test]
fn what_the_plugin_watches_is_a_dependency() {
    // The plugin's reading depends on a file outside the folder.
    let folder = notes();
    let shared = TemporaryFolder::new();
    let watched = shared.write("shared.txt", "shared");
    let mut time = ManualTime::new();
    let plugin = Rc::new(RecordingFolderPlugin {
        watched: vec![watched.to_string_lossy().into_owned()],
        ..RecordingFolderPlugin::new(FOLDER_PLUGIN_ID)
    });
    let mut body = open(&folder.path(), settle_binding(), &plugin);

    body.notify(&watched, time.now);
    time.millis(&mut body, SETTLE);

    assert!(body.dependencies().contains(&watched));
    assert_eq!(plugin.reads(), 2);
}

#[test]
fn a_new_file_under_a_folder_the_plugin_watches_causes_one_read() {
    // The plugin counts what extra/ holds, which no file rule selects.
    let folder = notes();
    std::fs::create_dir_all(folder.full("extra")).unwrap();
    let mut time = ManualTime::new();
    let plugin = Rc::new(RecordingFolderPlugin {
        watched: vec!["extra".into()],
        ..RecordingFolderPlugin::new(FOLDER_PLUGIN_ID)
    });
    let mut body = open(&folder.path(), settle_binding(), &plugin);

    body.notify(&folder.write("extra/new.txt", "new"), time.now);
    time.millis(&mut body, SETTLE);

    // The folder is named for the host to watch, and a file added under it reads the folder again.
    assert!(body.dependencies().contains(&folder.full("extra")));
    assert_eq!(plugin.reads(), 2);
}

#[test]
fn a_change_of_only_the_registration_raises_the_change_without_a_read() {
    // The registration sits in the folder, and is no file rule's file.
    let folder = notes();
    let registration = folder.full("notes.adp");
    let mut time = ManualTime::new();
    let plugin = Rc::new(RecordingFolderPlugin::new(FOLDER_PLUGIN_ID));
    let dynamic: Rc<dyn PersistencePlugin> = plugin.clone();
    let mut body = FolderBody::open(
        &folder.path(),
        settle_binding(),
        Some(dynamic),
        Rc::new(FblOptions::default()),
        Some(&registration),
    )
    .unwrap();

    // A position written twice, as a placement and its correction.
    folder.write("notes.adp", "position 1");
    body.notify(&registration, time.now);
    folder.write("notes.adp", "position 2");
    body.notify(&registration, time.now);
    time.millis(&mut body, SETTLE);

    // The change is returned once, and the folder is not read again.
    assert!(body.dependencies().contains(&registration));
    assert_eq!(plugin.reads(), 1);
    assert_eq!(time.changes, [change(false, true)]);
}

#[test]
fn a_one_shot_read_or_a_closed_body_leaves_nothing_pending() {
    let folder = notes();
    let mut time = ManualTime::new();
    let plugin = Rc::new(RecordingFolderPlugin::new(FOLDER_PLUGIN_ID));

    // A one-shot read, and an open body closed while a change waits.
    let dynamic: Rc<dyn PersistencePlugin> = plugin.clone();
    let model: FblModel = FolderBody::read_once(
        &folder.path(),
        settle_binding(),
        Some(dynamic),
        Rc::new(FblOptions::default()),
    )
    .unwrap();
    let mut body = open(&folder.path(), settle_binding(), &plugin);
    body.notify(&folder.full("pages/a.txt"), time.now);
    body.close();
    body.notify(&folder.full("pages/b.txt"), time.now);
    time.advance(&mut body, Duration::from_secs(10));

    assert_eq!(model.elements.len(), 3);
    assert_eq!(plugin.reads(), 2);
    assert!(time.changes.is_empty());
    assert_eq!(body.deadline(), None);
}

// ---- FolderBodyUnopenedFile.Tests.cs ----

fn unopened_binding() -> Rc<FblBinding> {
    folder_binding(None, Some(SETTLE as i64), false)
}

fn four_notes() -> TemporaryFolder {
    let folder = notes();
    folder.write("pages/c.txt", "c");
    folder
}

/// Holds the file at `path` open, sharing nothing, so no other reader can open it; on a platform without share
/// modes, takes every permission away instead, and none when the test runs where that does not stop a reader.
fn held(path: &Path) -> Option<Held> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt as _;
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(0)
            .open(path)
            .unwrap();
        Some(Held(file))
    }
    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0)).unwrap();
        if std::fs::File::open(path).is_ok() {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o644)).unwrap();
            return None;
        }
        Some(Held(path.to_path_buf()))
    }
}

#[cfg(windows)]
struct Held(#[allow(dead_code, reason = "the handle holds the file while it lives")] std::fs::File);

#[cfg(not(windows))]
struct Held(PathBuf);

#[cfg(not(windows))]
impl Drop for Held {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt as _;
        let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o644));
    }
}

#[test]
fn a_file_that_cannot_be_opened_is_a_finding_at_that_file_and_the_rest_is_read() {
    let folder = four_notes();
    let plugin = Rc::new(RecordingFolderPlugin::new(FOLDER_PLUGIN_ID));
    let held_path = folder.full("pages/b.txt");

    let Some(hold) = held(&held_path) else { return };
    let body = open(&folder.path(), unopened_binding(), &plugin);
    let model = body.model();
    let dependencies = body.dependencies();
    drop(hold);

    // The plugin is handed the others, the file is a finding at itself, and it is still watched.
    assert_eq!(plugin.reads(), 1);
    assert_eq!(plugin.paths(0), ["index.txt", "pages/a.txt", "pages/c.txt"]);
    assert!(!model.unreadable);
    assert_eq!(model.findings.len(), 1);
    let finding = &model.findings[0];
    assert_eq!(
        (
            finding.code.as_str(),
            finding.severity,
            finding.location.as_ref().map(|l| l.file.as_str())
        ),
        (codes::UNPARSEABLE, FindingSeverity::Error, Some("pages/b.txt"))
    );
    assert_eq!(finding.message, "The file could not be opened, so it is not read.");
    assert!(dependencies.contains(&held_path));
}

#[test]
fn a_file_that_can_be_opened_again_is_read_once_its_change_settles() {
    let folder = four_notes();
    let mut time = ManualTime::new();
    let plugin = Rc::new(RecordingFolderPlugin::new(FOLDER_PLUGIN_ID));
    let held_path = folder.full("pages/b.txt");
    let Some(hold) = held(&held_path) else { return };
    let mut body = open(&folder.path(), unopened_binding(), &plugin);
    drop(hold);

    body.notify(&held_path, time.now);
    time.millis(&mut body, SETTLE);

    assert_eq!(plugin.reads(), 2);
    assert_eq!(
        plugin.paths(1),
        ["index.txt", "pages/a.txt", "pages/b.txt", "pages/c.txt"]
    );
    assert!(body.model().findings.is_empty());
}
