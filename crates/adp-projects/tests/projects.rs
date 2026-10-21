//! The project list (etalii.adp spec 030, Story 1 scenario 2, FR-011, task T022).

use adp_core::state::StateFolder;
use adp_projects::{ProjectList, Refusal};

fn list(state: &tempfile::TempDir) -> ProjectList {
    ProjectList::load(StateFolder::at(state.path())).unwrap()
}

#[test]
fn an_added_folder_is_listed_with_its_name_and_kept_across_restarts() {
    let (state, folder) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let project = list(&state).add(folder.path()).unwrap();
    assert_eq!(project.name, folder.path().file_name().unwrap().to_string_lossy());
    let again = list(&state);
    assert_eq!(again.projects(), [project]);
}

#[test]
fn adding_validates_the_path() {
    let (state, folder) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let mut projects = list(&state);
    let file = folder.path().join("notes.md");
    std::fs::write(&file, "x").unwrap();
    assert!(matches!(projects.add(folder.path().join("missing")), Err(Refusal(r)) if r.contains("does not exist")));
    assert!(matches!(projects.add(&file), Err(Refusal(r)) if r.contains("is not a folder")));
    projects.add(folder.path()).unwrap();
    assert!(matches!(projects.add(folder.path()), Err(Refusal(r)) if r.contains("already in the list")));
    assert_eq!(projects.projects().len(), 1);
}

#[test]
fn a_folder_that_is_gone_is_reported_when_opened_and_not_dropped() {
    let state = tempfile::tempdir().unwrap();
    let folder = tempfile::tempdir().unwrap();
    let id = list(&state).add(folder.path()).unwrap().id;
    let path = folder.path().to_path_buf();
    folder.close().unwrap();
    let mut projects = list(&state);
    assert!(matches!(projects.open(&id), Err(Refusal(r)) if r.contains(&path.display().to_string())));
    assert_eq!(projects.projects().len(), 1);
}

#[test]
fn a_removed_project_is_gone_after_a_restart_and_its_folder_is_untouched() {
    let (state, folder) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let id = list(&state).add(folder.path()).unwrap().id;
    list(&state).remove(&id).unwrap();
    assert!(list(&state).projects().is_empty());
    assert!(folder.path().is_dir());
    assert!(matches!(list(&state).remove(&id), Err(Refusal(_))));
}

#[test]
fn opening_marks_when_the_project_was_last_opened() {
    let (state, folder) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let mut projects = list(&state);
    let id = projects.add(folder.path()).unwrap().id;
    let opened = projects.open(&id).unwrap();
    assert!(opened.opened_at.is_some());
    assert_eq!(list(&state).projects()[0].opened_at, opened.opened_at);
}
