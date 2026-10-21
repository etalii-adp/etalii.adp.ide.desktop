//! The errors and warnings (etalii.adp spec 030, FR-019, task T028), as the standalone host's `ProblemStore` keeps them.

use std::path::Path;

use adp_core::state::StateFolder;
use adp_problems::{Finding, ProblemStore, Scope, SetState, Severity, Validator};

/// Reports an error on every line of a `.todo` file that contains TODO.
struct Todos;

impl Validator for Todos {
    fn applies_to(&self, file: &str) -> bool {
        file.ends_with(".todo")
    }
    fn validate(&self, root: &Path, file: &str) -> Vec<Finding> {
        let text = std::fs::read_to_string(root.join(file)).unwrap();
        text.lines()
            .enumerate()
            .filter(|(_, l)| l.contains("TODO"))
            .map(|(i, _)| Finding {
                severity: Severity::Error,
                message: "Something is left to do.".into(),
                file: file.into(),
                line: Some(i as u32 + 1),
                column: None,
                element: None,
                rule: "test.todo".into(),
            })
            .collect()
    }
}

fn project() -> (tempfile::TempDir, tempfile::TempDir) {
    let (state, root) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    std::fs::create_dir(root.path().join("docs")).unwrap();
    std::fs::write(root.path().join("a.todo"), "TODO one\nfine\nTODO two\n").unwrap();
    std::fs::write(root.path().join("docs/b.todo"), "TODO three\n").unwrap();
    std::fs::write(root.path().join("c.md"), "TODO not checked\n").unwrap();
    (state, root)
}

#[test]
fn a_project_never_validated_says_so_and_validating_it_counts_its_findings() {
    let (state, root) = project();
    let mut store = ProblemStore::load(StateFolder::at(state.path()), "p", root.path());
    assert_eq!(store.get().state, SetState::NeverValidated);
    store.validate(&Scope::Project, &[&Todos]).unwrap();
    let set = store.get();
    assert_eq!(set.state, SetState::Validated);
    assert_eq!(set.errors, 3);
    assert_eq!(
        set.problems.iter().map(|p| p.finding.file.as_str()).collect::<Vec<_>>(),
        ["a.todo", "a.todo", "docs/b.todo"]
    );
}

#[test]
fn findings_survive_a_restart_and_turn_stale_when_their_file_changes() {
    let (state, root) = project();
    ProblemStore::load(StateFolder::at(state.path()), "p", root.path())
        .validate(&Scope::Project, &[&Todos])
        .unwrap();
    let store = ProblemStore::load(StateFolder::at(state.path()), "p", root.path());
    assert_eq!(store.get().errors, 3);
    assert!(store.get().problems.iter().all(|p| !p.stale));
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(root.path().join("a.todo"), "fixed\n").unwrap();
    let set = store.get();
    assert!(
        set.problems
            .iter()
            .filter(|p| p.finding.file == "a.todo")
            .all(|p| p.stale)
    );
    assert!(
        set.problems
            .iter()
            .filter(|p| p.finding.file == "docs/b.todo")
            .all(|p| !p.stale)
    );
}

#[test]
fn validating_a_scope_replaces_only_that_scopes_findings() {
    let (state, root) = project();
    let mut store = ProblemStore::load(StateFolder::at(state.path()), "p", root.path());
    store.validate(&Scope::Project, &[&Todos]).unwrap();
    std::fs::write(root.path().join("a.todo"), "fixed\n").unwrap();
    store.validate(&Scope::File("a.todo".into()), &[&Todos]).unwrap();
    assert_eq!(store.get().errors, 1);
    std::fs::write(root.path().join("docs/b.todo"), "fixed\n").unwrap();
    store.validate(&Scope::Folder("docs".into()), &[&Todos]).unwrap();
    assert_eq!(store.get().errors, 0);
}

#[test]
fn a_damaged_cache_reads_as_never_validated() {
    let (state, root) = project();
    let folder = StateFolder::at(state.path());
    std::fs::create_dir_all(folder.problems("p").parent().unwrap()).unwrap();
    std::fs::write(folder.problems("p"), "{ damaged").unwrap();
    assert_eq!(
        ProblemStore::load(folder, "p", root.path()).get().state,
        SetState::NeverValidated
    );
}
