//! The standalone host's round-trip tests, run against the desktop host for every tool type whose module is ported
//! (etalii.adp spec 030, FR-034, T051). At `8b6ccfc` the standalone host proves its byte-for-byte promise per module:
//! `TimelineRoundTripTests`, `CausalLoopRoundTripTests` and `DependencyGraphRoundTripTests` (a document saved without
//! a change is written back byte for byte with no splice, every value set to what it already holds writes the same
//! bytes, and every edit undone puts back the bytes it started from), and the other modules' `EveryFixture_RoundTrips…`,
//! `AnUnchangedDocument_ComesBack…`, `…Undo…ByteForByte` and the editors' and the Knowledge designer's
//! `EveryExample_…RoundTripsByteIdentically` tests over their examples. Here one test per tool type, named after it,
//! runs those three round trips over the same corpus: the module's `examples/` and its test project's `Fixtures/`,
//! vendored byte for byte, as the standalone host checked them out, under `tests/round_trip/` with their paths under
//! the standalone host's `src/`.
//!
//! A diagram is opened through the tool types as the shell composes them, in a copy of its folder, and checked as the
//! standalone host checks its module:
//!
//! - opening it, and saving it without a change, writes no file of its folder, and the save plans no splice (FBL
//!   section 15.3);
//! - for the three types whose standalone tests prove it, setting every editable property of every element to the
//!   value it shows writes nothing, except a value the text does not write as shown (a row `twelve`, read as row 0),
//!   which is then written as shown;
//! - every edit the session offers, once each against the first element that offers it (every property changed and
//!   cleared, every action of the element's and the canvas's menu with its prompt answered, a move of the element),
//!   is one undo away and, undone, puts back every byte of the body and the registration, as the standalone host's
//!   `…Undo…` tests compare them; a refused edit writes no file of the folder.
//!
//! The tool types whose module does not yet keep one of these promises are pending (`#[ignore]`), each with what is
//! left, so their count shows what is left of FR-034.
//!
//! An editor's example is opened and saved unchanged through the text file every editor saves with, and a Knowledge
//! designer document is read and written unchanged through its body.

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use adp_context::{ActionGroup, Prompt, PromptOption, PropertyEditor};
use adp_core::Catalogue;
use adp_diagram::session::{ActionOutcome, ActionTarget};
use adp_diagram::tool::DiagramTools;
use adp_diagram::workspace::{DiagramWorkspace, is_registration};
use adp_diagram::{DiagramChange, DiagramSession, ElementKind};
use adp_disl::{DislPluginFunctions, IdSource};
use adp_fbl::{ModelChange, RegistrationDocument};

/// The corpus, under the standalone host's `src/`.
const CORPUS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/round_trip");
const VENDOR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../vendor");

/// The diagram tool types as the shell composes them (`adp-shell`'s `DiagramHost::with_vendor`).
fn tools() -> DiagramTools {
    let tools = adp_diagram_mindmap::register(DiagramTools::new(Catalogue::load(Path::new(VENDOR)), |_| {
        DislPluginFunctions::none()
    }))
    .with_module(adp_diagram_timeline::ORIGIN, adp_diagram_timeline::module())
    .with_module(adp_diagram_hype_cycle::ORIGIN, adp_diagram_hype_cycle::module())
    .with_module(adp_diagram_causal_loop::ORIGIN, adp_diagram_causal_loop::module())
    .with_module(adp_diagram_helm_chart::ORIGIN, adp_diagram_helm_chart::module())
    .with_module(adp_diagram_sparql::ORIGIN, adp_diagram_sparql::module())
    .with_module(
        adp_diagram_functional_decomposition_graph::ORIGIN,
        adp_diagram_functional_decomposition_graph::module(),
    )
    .with_module(
        adp_diagram_agent_behavior_modelling::ORIGIN,
        adp_diagram_agent_behavior_modelling::module(),
    );
    let tools = adp_diagram_dependency_graph::register(tools);
    let tools = adp_diagram_c4::register(tools);
    let tools = adp_diagram_sankey::register(tools);
    let tools = adp_diagram_supply_chain::register(tools);
    let tools = adp_diagram_wardley_map::register(tools);
    let tools = adp_diagram_agent_activity::register(tools);
    let tools = adp_diagram_ansible_structure::register(tools);
    let tools = adp_diagram_azure_devops_pipeline::register(tools);
    let tools = adp_diagram_dotnet_dependency_graph::register(tools);
    let tools = adp_diagram_databricks::register(tools);
    adp_diagram_rdf::register(tools)
}

/// Ids for what an edit creates: `roundtrip1`, `roundtrip2`, ….
#[derive(Default)]
struct RoundTripIds(Cell<usize>);

impl IdSource for RoundTripIds {
    fn next(&self, _type_name: &str) -> Result<String, String> {
        self.0.set(self.0.get() + 1);
        Ok(format!("roundtrip{}", self.0.get()))
    }
}

/// Every file under `root`, recursively, by its path.
fn files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(folder) = pending.pop() {
        for entry in fs::read_dir(&folder).expect("the folder reads").flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// Every file under `root` with its bytes.
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    files(root)
        .into_iter()
        .map(|path| {
            let bytes = fs::read(&path).expect("the file reads");
            (path.strip_prefix(root).expect("under the root").to_path_buf(), bytes)
        })
        .collect()
}

/// The lines that differ between two versions of a file: those between the first and the last line they share, with
/// one line around them.
fn difference(was: &[u8], is: &[u8]) -> String {
    let was: Vec<&str> = std::str::from_utf8(was)
        .map(|t| t.split_inclusive('\n').collect())
        .unwrap_or_default();
    let is: Vec<&str> = std::str::from_utf8(is)
        .map(|t| t.split_inclusive('\n').collect())
        .unwrap_or_default();
    let start = was.iter().zip(&is).take_while(|(a, b)| a == b).count();
    let end = was[start..]
        .iter()
        .rev()
        .zip(is[start..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    let from = start.saturating_sub(1);
    let mut out = format!("at line {}:\n", start + 1);
    for line in &was[from..was.len() - end] {
        out.push_str(&format!("- {line:?}\n"));
    }
    for line in &is[from..is.len() - end] {
        out.push_str(&format!("+ {line:?}\n"));
    }
    out
}

/// The value a `key: value` line writes, without its quotes and its comment, as the standalone host's
/// `TimelineRoundTripTests.WrittenRow` reads it.
fn written_value(line: &str) -> &str {
    let value = line.split_once(':').map_or(line, |(_, value)| value);
    let value = value.split_once(" #").map_or(value, |(value, _)| value);
    value.trim().trim_matches(|c| c == '"' || c == '\'')
}

/// The lines of `was` that `is` no longer has between the lines the two share at their start and their end.
fn changed_lines(was: &[u8], is: &[u8]) -> Vec<String> {
    let was: Vec<&str> = std::str::from_utf8(was)
        .map(|t| t.split_inclusive('\n').collect())
        .unwrap_or_default();
    let is: Vec<&str> = std::str::from_utf8(is)
        .map(|t| t.split_inclusive('\n').collect())
        .unwrap_or_default();
    let start = was.iter().zip(&is).take_while(|(a, b)| a == b).count();
    let end = was[start..]
        .iter()
        .rev()
        .zip(is[start..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    was[start..was.len() - end]
        .iter()
        .map(|line| (*line).to_owned())
        .collect()
}

fn copy_folder(from: &Path, to: &Path) {
    for file in files(from) {
        let target = to.join(file.strip_prefix(from).expect("under the folder"));
        fs::create_dir_all(target.parent().expect("a parent")).expect("the folder is made");
        fs::copy(&file, &target).expect("the file is copied");
    }
}

/// The folder a corpus file is opened in, as the standalone host's tests and its explorer open it: the example's own
/// folder under `examples/`, else the folder it is in.
fn project_of(file: &Path) -> PathBuf {
    let mut folder = file.parent().expect("a folder").to_path_buf();
    let mut at = folder.clone();
    while let Some(parent) = at.parent() {
        if parent.file_name().is_some_and(|n| n == "examples") {
            folder = at;
            break;
        }
        if parent == Path::new(CORPUS) {
            break;
        }
        at = parent.to_path_buf();
    }
    folder
}

/// The documents of the corpus under `module` that tool type `origin` opens, as (folder, path in it): each
/// registration naming `origin`, then each body routed to `origin` first that no such registration beside it opens.
fn documents(tools: &DiagramTools, module: &str, origin: &str) -> Vec<(PathBuf, PathBuf)> {
    let all = files(&Path::new(CORPUS).join(module));
    let relative = |file: &Path| {
        let project = project_of(file);
        let path = file.strip_prefix(&project).expect("in its folder").to_path_buf();
        (project, path)
    };
    let mut found = Vec::new();
    let mut registered = HashSet::new();
    for file in all.iter().filter(|f| is_registration(f)) {
        let registration = RegistrationDocument::read(fs::read(file).expect("the registration reads"));
        if registration.origin == origin {
            let stem = file.file_stem().map(|s| s.to_string_lossy().into_owned());
            registered.insert((file.parent().expect("a folder").to_path_buf(), stem));
            found.push(relative(file));
        }
    }
    for file in all.iter().filter(|f| !is_registration(f)) {
        let name = file.file_name().expect("a name").to_string_lossy().into_owned();
        if tools.for_file(&name).first().map(String::as_str) != Some(origin) {
            continue;
        }
        let stem = file.file_stem().map(|s| s.to_string_lossy().into_owned());
        if registered.contains(&(file.parent().expect("a folder").to_path_buf(), stem)) {
            continue;
        }
        found.push(relative(file));
    }
    found
}

/// What a tool type's round trips did, for the message that says they prove something.
#[derive(Default, Debug)]
struct Tally {
    documents: usize,
    opened: usize,
    values: usize,
    edits: usize,
}

/// The available actions of a menu, submenus included, as (id, label).
fn available(groups: &[ActionGroup]) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for group in groups {
        for action in &group.actions {
            if !action.items.is_empty() {
                found.extend(available(&action.items));
            } else if action.is_available() {
                found.push((action.id.clone(), action.label.clone()));
            }
        }
    }
    found
}

/// The first selectable option, depth first.
fn first_option(options: &[PromptOption]) -> Option<String> {
    options.iter().find_map(|option| {
        if option.selectable && option.unavailable_reason.is_empty() {
            Some(option.id.clone())
        } else {
            first_option(&option.children)
        }
    })
}

/// The values a property is changed to: another of its candidates or the other of yes and no, else a changed text
/// and an emptied one.
fn changed_values(editor: &PropertyEditor, value: &str, candidates: &[String]) -> Vec<String> {
    match editor {
        PropertyEditor::Toggle => vec![if value == "true" { "false" } else { "true" }.to_owned()],
        PropertyEditor::Choice | PropertyEditor::Slider => candidates
            .iter()
            .find(|candidate| candidate.as_str() != value)
            .cloned()
            .into_iter()
            .collect(),
        _ if value.is_empty() => vec!["Round trip".to_owned()],
        _ => vec![format!("{value} round trip"), String::new()],
    }
}

/// One document's round trips, its failures added to `failures`.
struct Run<'a> {
    name: String,
    root: PathBuf,
    before: BTreeMap<PathBuf, Vec<u8>>,
    workspace: DiagramWorkspace,
    session: DiagramSession,
    ids: RoundTripIds,
    /// The body and the registration, as paths in the folder: what an undo puts back.
    documents: BTreeSet<PathBuf>,
    /// The changes on the history before the edit being checked.
    depth: usize,
    /// Whether the edit being checked asked a question after it had written, and the question was answered.
    answered_after_write: bool,
    failures: &'a mut Vec<String>,
}

impl Run<'_> {
    /// Whether every file is as it was; a failure naming `what` and the first file that differs when not.
    fn unchanged(&mut self, what: &str) -> bool {
        self.compare(what, |_| true)
    }

    /// Whether the body and the registration are as they were; a failure naming `what` when not. Any other file of
    /// the folder is put back without a failure.
    fn documents_unchanged(&mut self, what: &str) -> bool {
        let documents = self.documents.clone();
        self.compare(what, |path| documents.contains(path))
    }

    /// Compares the files `checked` picks with what they were, a failure naming `what` and the first that differs,
    /// and puts every file back.
    fn compare(&mut self, what: &str, checked: impl Fn(&Path) -> bool) -> bool {
        // What the next edit is counted from.
        self.depth = self.workspace.history().availability().undo_count;
        let after = snapshot(&self.root);
        if after == self.before {
            return true;
        }
        let differs = self
            .before
            .keys()
            .chain(after.keys())
            .find(|path| checked(path) && self.before.get(*path) != after.get(*path));
        let same = differs.is_none();
        if let Some(differs) = differs {
            let text = match (self.before.get(differs), after.get(differs)) {
                (Some(was), Some(is)) => difference(was, is),
                (_, None) => "(gone)".to_owned(),
                (None, Some(_)) => "(new)".to_owned(),
            };
            self.failures
                .push(format!("{}: {what} changed {}:\n{text}", self.name, differs.display()));
        }
        self.restore(&after);
        same
    }

    /// Whether setting a value to what it shows only wrote it as shown: every file as it was but the body, whose
    /// changed lines all wrote a text other than `value` (a row `twelve`, read as 0, written as 0), as the standalone
    /// host's `TimelineRoundTripTests` leaves such a row out.
    fn written_as_shown(&mut self, value: &str) -> bool {
        let after = snapshot(&self.root);
        let rewritten = self.before.iter().all(|(path, was)| match after.get(path) {
            Some(is) if is == was => true,
            Some(is) if self.documents.contains(path) => {
                let removed = changed_lines(was, is);
                !removed.is_empty() && removed.iter().all(|line| written_value(line) != value)
            }
            _ => false,
        }) && after.len() == self.before.len();
        if rewritten {
            self.restore(&after);
        }
        rewritten
    }

    /// Puts every file of the folder back as it was, so what follows starts from the document again.
    fn restore(&mut self, after: &BTreeMap<PathBuf, Vec<u8>>) {
        if *after == self.before {
            return;
        }
        for (path, bytes) in &self.before {
            fs::write(self.root.join(path), bytes).expect("the file is put back");
        }
        for path in after.keys().filter(|path| !self.before.contains_key(*path)) {
            fs::remove_file(self.root.join(path)).expect("the new file is removed");
        }
        for path in self.before.keys() {
            self.workspace.reconcile(&self.root.join(path));
        }
        self.depth = self.workspace.history().availability().undo_count;
    }

    /// The views follow what an edit or its undo changed.
    fn refresh(&mut self) {
        for path in self.workspace.take_changed() {
            self.session.on_document_changed(self.workspace.documents(), &path);
        }
    }

    /// Undoes the edit `what` that ran, and checks that every byte came back.
    fn undo(&mut self, what: &str) {
        self.refresh();
        // An edit is one undo away (the standalone host's `AnEditTravelsAsACommand_AndIsOneUndoAway`), and the answer to
        // a question it asked once it had written, such as the label of what it added, is the person's next edit.
        let entries = self
            .workspace
            .history()
            .availability()
            .undo_count
            .saturating_sub(self.depth);
        let expected = 1 + usize::from(std::mem::take(&mut self.answered_after_write));
        if entries == 0 {
            // A change of the view only, such as a fold, writes nothing and is not recorded.
            self.unchanged(&format!("{what}, recorded as no change,"));
            return;
        }
        if entries != expected {
            self.failures.push(format!(
                "{}: {what} recorded {entries} changes, not {expected}",
                self.name
            ));
        }
        let mut undone = self.workspace.undo();
        while undone.is_done() && self.workspace.history().availability().undo_count > self.depth {
            undone = self.workspace.undo();
        }
        self.depth = self.workspace.history().availability().undo_count;
        self.refresh();
        match undone {
            adp_history::Outcome::Done { .. } => {
                self.documents_unchanged(&format!("{what}, undone,"));
            }
            adp_history::Outcome::Refused(refusal) => {
                self.documents_unchanged(&format!("{what}, its undo refused ({}),", refusal.0));
            }
        }
    }

    /// Runs action `id` on `target`, its prompt answered as a person would; whether it did something.
    fn action(&mut self, target: &ActionTarget, id: &str) -> bool {
        let outcome = self.session.execute(&mut self.workspace, target, id, &self.ids);
        let outcome = match outcome {
            ActionOutcome::Prompt(prompt) => {
                let written = self.workspace.history().availability().undo_count > self.depth;
                let answer = match prompt {
                    Prompt::Input { .. } => Some("Round trip".to_owned()),
                    Prompt::Confirm { .. } => Some(String::new()),
                    Prompt::Choice { options, .. } => first_option(&options),
                    Prompt::File { .. } | Prompt::Closed { .. } => None,
                };
                self.answered_after_write = written && answer.is_some();
                match answer {
                    Some(answer) => self
                        .session
                        .answer_with_ids(&mut self.workspace, target, id, &answer, &self.ids),
                    None => ActionOutcome::Refused("dismissed".to_owned()),
                }
            }
            other => other,
        };
        matches!(outcome, ActionOutcome::Done)
    }
}

/// The round trips of tool type `origin` over the corpus under `module`; `holds` adds setting every value to what it
/// shows.
fn diagram_round_trips(module: &str, origin: &str, holds: bool) -> Tally {
    let mut tools = tools();
    let mut tally = Tally::default();
    let mut failures = Vec::new();
    for (project, path) in documents(&tools, module, origin) {
        tally.documents += 1;
        let folder = tempfile::tempdir().expect("a temporary folder");
        copy_folder(&project, folder.path());
        let name = format!(
            "{}/{}",
            project.strip_prefix(CORPUS).unwrap_or(&project).display(),
            path.display()
        );
        let before = snapshot(folder.path());
        let mut workspace = DiagramWorkspace::new(folder.path());
        let opened = workspace.open(&mut tools, &path);
        let (opened, mut session) = match opened {
            Ok(opened) => opened,
            Err(_) => {
                // A document that cannot be read is not written at all.
                if snapshot(folder.path()) != before {
                    failures.push(format!("{name}: refusing to open it changed its folder"));
                }
                continue;
            }
        };
        tally.opened += 1;
        let _ = session.baseline(workspace.documents());
        let mut run = Run {
            name,
            root: folder.path().to_path_buf(),
            before,
            workspace,
            session,
            ids: RoundTripIds::default(),
            documents: [Some(&opened.body_path), opened.registration_path.as_ref()]
                .into_iter()
                .flatten()
                .map(|path| {
                    let path = if path.is_absolute() {
                        path.clone()
                    } else {
                        folder.path().join(path)
                    };
                    path.strip_prefix(folder.path()).map(Path::to_path_buf).unwrap_or(path)
                })
                .collect(),
            depth: 0,
            answered_after_write: false,
            failures: &mut failures,
        };
        run.unchanged("opening it");

        // A document saved without a change plans no splice and is written back byte for byte.
        let saved = run
            .workspace
            .documents()
            .get(&opened.body_path)
            .map(|document| document.plan("Save", &[DiagramChange::Body(ModelChange::Save)]));
        if let Some(Ok(edit)) = saved {
            if !edit.is_empty() {
                run.failures
                    .push(format!("{}: a save without a change planned {edit:?}", run.name));
            }
            let _ = run.workspace.execute(edit);
            run.unchanged("saving it without a change");
        }

        let elements = run.session.render(run.workspace.documents()).unwrap_or_default();

        // Every value set to what it holds writes the same bytes.
        for element in elements.iter().filter(|_| holds) {
            let rows = run
                .session
                .properties(run.workspace.documents(), &[element.id.as_str()]);
            for row in rows.iter().filter(|row| row.read_only_reason.is_empty()) {
                // Done or refused, nothing is written that was not already there.
                let _ = run
                    .session
                    .set_property(&mut run.workspace, &[element.id.as_str()], &row.id, &row.value);
                tally.values += 1;
                run.refresh();
                if !run.written_as_shown(&row.value) {
                    run.unchanged(&format!("setting {} of {} to what it holds", row.id, element.id));
                }
            }
        }

        // Every edit, once each against the first element that offers it, undone, restores the bytes.
        let mut properties = BTreeSet::new();
        let mut actions = BTreeSet::new();
        let mut moved = false;
        let mut targets: Vec<ActionTarget> = elements
            .iter()
            .map(|element| ActionTarget::Element(element.id.clone()))
            .collect();
        targets.push(ActionTarget::Canvas { x: 40.0, y: 40.0 });
        for target in targets {
            if let ActionTarget::Element(id) = &target {
                let rows = run.session.properties(run.workspace.documents(), &[id.as_str()]);
                for row in rows.iter().filter(|row| row.read_only_reason.is_empty()) {
                    if !properties.insert(row.id.clone()) {
                        continue;
                    }
                    for value in changed_values(&row.editor, &row.value, &row.candidates) {
                        let what = format!("setting {} of {id} to '{value}'", row.id);
                        match run
                            .session
                            .set_property(&mut run.workspace, &[id.as_str()], &row.id, &value)
                        {
                            Ok(()) => {
                                tally.edits += 1;
                                run.undo(&what);
                            }
                            Err(_) => {
                                run.unchanged(&format!("{what}, refused,"));
                            }
                        }
                    }
                }
                let node = elements
                    .iter()
                    .find(|element| &element.id == id)
                    .filter(|element| element.kind == ElementKind::Node);
                if let Some(element) = node
                    && !moved
                {
                    moved = true;
                    let what = format!("moving {id}");
                    let moves = [(id.clone(), element.x + 24.0, element.y + 24.0)];
                    match run.session.move_elements(&mut run.workspace, &moves) {
                        Ok(()) => {
                            tally.edits += 1;
                            run.undo(&what);
                        }
                        Err(_) => {
                            run.unchanged(&format!("{what}, refused,"));
                        }
                    }
                }
            }
            let menu = run.session.actions(run.workspace.documents(), &target);
            for (action, label) in available(&menu) {
                if !actions.insert(action.clone()) {
                    continue;
                }
                let what = format!("'{label}' ({action}) on {target:?}");
                if run.action(&target, &action) {
                    tally.edits += 1;
                    run.undo(&what);
                } else {
                    run.refresh();
                    run.unchanged(&format!("{what}, refused or dismissed,"));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{origin}: {}", failures.join("\n\n"));
    assert!(
        tally.documents > 0,
        "{origin}: the corpus under {module} has no document of it"
    );
    assert!(tally.opened > 0, "{origin}: no document of the corpus opens: {tally:?}");
    tally
}

/// The standalone host's `EveryExample_OpensInThisEditor_AndRoundTripsByteIdentically` for editor `editor`: every
/// file of its examples, opened and saved unchanged through the text file every editor saves with, keeps its bytes.
fn editor_round_trips(editor: &str) {
    let examples = files(&Path::new(CORPUS).join("editors").join(editor).join("examples"));
    assert!(!examples.is_empty(), "editor/{editor} has no examples");
    let mut failures = Vec::new();
    for example in &examples {
        let original = fs::read(example).expect("the example reads");
        let folder = tempfile::tempdir().expect("a temporary folder");
        let path = folder.path().join(example.file_name().expect("a name"));
        fs::write(&path, &original).expect("the copy is written");
        let saved = adp_documents::TextFile::open(&path).and_then(|mut text| {
            let content = text.content().to_owned();
            text.save(&content)
        });
        let after = fs::read(&path).expect("the copy reads");
        if let Err(reason) = saved {
            failures.push(format!("{}: {reason}", example.display()));
        } else if after != original {
            failures.push(format!("{}: saving it unchanged changed its bytes", example.display()));
        }
    }
    assert!(failures.is_empty(), "editor/{editor}: {}", failures.join("\n"));
}

// ---- the diagrams --------------------------------------------------------------------------------------------------

#[test]
fn c4_context() {
    diagram_round_trips("diagrams/c4", "c4/context", false);
}

#[test]
fn c4_container() {
    diagram_round_trips("diagrams/c4", "c4/container", false);
}

#[test]
#[ignore = "pending: undoing a description or technology added to an element without one leaves an empty \"\" behind"]
fn c4_component() {
    diagram_round_trips("diagrams/c4", "c4/component", false);
}

#[test]
fn c4_system_landscape() {
    diagram_round_trips("diagrams/c4", "c4/system-landscape", false);
}

#[test]
fn c4_dynamic() {
    diagram_round_trips("diagrams/c4", "c4/dynamic", false);
}

#[test]
#[ignore = "pending: undoing a description or technology added to an element without one leaves an empty \"\" behind, and a renamed container instance cannot be undone"]
fn c4_deployment() {
    diagram_round_trips("diagrams/c4", "c4/deployment", false);
}

#[test]
#[ignore = "pending: a label that YAML reads as another type (yes, 1.0, null) or a quoted row set to what it holds is rewritten, and an unreadable document takes edits"]
fn generic_timeline() {
    diagram_round_trips("diagrams/timeline", "generic/timeline", true);
}

#[test]
fn generic_dependencies() {
    diagram_round_trips("diagrams/dependency-graph", "generic/dependencies", true);
}

#[test]
fn etalii_functional_decomposition_graph() {
    diagram_round_trips(
        "diagrams/functional-decomposition-graph",
        "etalii/functional-decomposition-graph",
        false,
    );
}

#[test]
fn etalii_agent_behavior_modelling() {
    diagram_round_trips(
        "diagrams/agent-behavior-modelling",
        "etalii/agent-behavior-modelling",
        false,
    );
}

#[test]
fn etalii_agent_activity_diagram() {
    diagram_round_trips(
        "diagrams/agent-activity-diagram",
        "etalii/agent-activity-diagram",
        false,
    );
}

#[test]
fn etalii_supply_chain() {
    diagram_round_trips("diagrams/supply-chain", "etalii/supply-chain", false);
}

#[test]
fn etalii_sankey() {
    diagram_round_trips("diagrams/sankey", "etalii/sankey", false);
}

#[test]
#[ignore = "pending: undoing 'Use the default order' moves dependsOn after a merge key, and undoing a dependency of a broken graph is refused"]
fn azure_devops_pipeline() {
    diagram_round_trips("diagrams/azure-devops-pipeline", "azure-devops/pipeline", false);
}

#[test]
fn ansible_structure() {
    diagram_round_trips("diagrams/ansible-structure", "ansible/structure", false);
}

#[test]
fn helm_chart() {
    diagram_round_trips("diagrams/helm-chart", "helm/chart", false);
}

#[test]
fn dotnet_dependency_graph() {
    diagram_round_trips("diagrams/dotnet-dependency-graph", "dotnet/dependency-graph", false);
}

#[test]
#[ignore = "pending: undoing an edit drops the byte order mark"]
fn wardley_map() {
    diagram_round_trips("diagrams/wardley-map", "wardley/map", false);
}

#[test]
fn gartner_hypecycle_graph() {
    diagram_round_trips("diagrams/gartner-hype-cycle-graph", "gartner/hypecycle-graph", false);
}

#[test]
fn freeplane_mindmap() {
    diagram_round_trips("diagrams/mindmap", "freeplane/mindmap", false);
}

#[test]
fn systems_causal_loop_diagram() {
    diagram_round_trips("diagrams/causal-loop-diagram", "systems/causal-loop-diagram", true);
}

#[test]
fn databricks_bundle() {
    diagram_round_trips("diagrams/databricks", "databricks/bundle", false);
}

#[test]
fn databricks_job() {
    diagram_round_trips("diagrams/databricks", "databricks/job", false);
}

#[test]
fn databricks_pipeline() {
    diagram_round_trips("diagrams/databricks", "databricks/pipeline", false);
}

#[test]
fn w3c_rdf() {
    diagram_round_trips("diagrams/rdf", "w3c/rdf", false);
}

#[test]
fn w3c_owl() {
    diagram_round_trips("diagrams/rdf", "w3c/owl", false);
}

#[test]
fn w3c_skos() {
    diagram_round_trips("diagrams/rdf", "w3c/skos", false);
}

#[test]
fn w3c_shacl() {
    diagram_round_trips("diagrams/rdf", "w3c/shacl", false);
}

#[test]
fn w3c_sparql() {
    diagram_round_trips("diagrams/sparql", "w3c/sparql", false);
}

// ---- the designer --------------------------------------------------------------------------------------------------

/// The standalone host's `AnExample_ReadsWithNothingToReport_AndWrittenUnchangedKeepsItsBytes`: every Knowledge
/// designer document of its examples is read with nothing unreadable and written unchanged as the same bytes.
#[test]
fn etalii_knowledge() {
    let examples: Vec<PathBuf> = files(&Path::new(CORPUS).join("designers/knowledge/examples"))
        .into_iter()
        .filter(|path| {
            path.extension()
                .is_some_and(|e| ["yaml", "yml", "json", "xml"].contains(&e.to_string_lossy().as_ref()))
        })
        .collect();
    assert!(!examples.is_empty(), "etalii/knowledge has no examples");
    for example in examples {
        let bytes = fs::read(&example).expect("the example reads");
        let body = adp_designer_knowledge::KnowledgeBody::read(bytes.clone(), &example.to_string_lossy());
        assert_eq!("", body.unreadable(), "{}", example.display());
        assert_eq!(Ok(bytes.clone()), body.change(&[]), "{}", example.display());
        assert_eq!(Ok(bytes), body.change(&[ModelChange::Save]), "{}", example.display());
    }
}

// ---- the editors ---------------------------------------------------------------------------------------------------

#[test]
fn editor_json() {
    editor_round_trips("json");
}

#[test]
fn editor_markdown() {
    editor_round_trips("markdown");
}

#[test]
fn editor_plain() {
    editor_round_trips("plain");
}

#[test]
fn editor_xml() {
    editor_round_trips("xml");
}

#[test]
fn editor_yaml() {
    editor_round_trips("yaml");
}
