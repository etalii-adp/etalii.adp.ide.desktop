//! The per-user state folder (etalii.adp spec 030, FR-036, task T017).

use adp_core::state::StateFolder;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Projects {
    paths: Vec<String>,
}

#[test]
fn a_saved_file_reads_back_and_a_missing_one_reads_as_none() {
    let folder = tempfile::tempdir().unwrap();
    let state = StateFolder::at(folder.path());
    assert_eq!(state.load::<Projects>(&state.projects()).unwrap(), None);
    let projects = Projects {
        paths: vec!["C:/git/etalii.adp".into()],
    };
    state.save(&state.projects(), &projects).unwrap();
    assert_eq!(state.load::<Projects>(&state.projects()).unwrap(), Some(projects));
}

#[test]
fn a_save_replaces_the_file_and_leaves_no_temporary_file() {
    let folder = tempfile::tempdir().unwrap();
    let state = StateFolder::at(folder.path());
    let problems = state.problems("p1");
    state.save(&problems, &vec![1, 2]).unwrap();
    state.save(&problems, &vec![3]).unwrap();
    assert_eq!(state.load::<Vec<i32>>(&problems).unwrap(), Some(vec![3]));
    let names: Vec<_> = std::fs::read_dir(problems.parent().unwrap())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(names, ["p1.json"]);
}

#[test]
fn a_damaged_file_is_an_error_naming_the_file() {
    let folder = tempfile::tempdir().unwrap();
    let state = StateFolder::at(folder.path());
    std::fs::write(state.layout(), b"{ not json").unwrap();
    let error = state.load::<Projects>(&state.layout()).unwrap_err();
    assert!(error.to_string().contains("layout.json"));
}

#[test]
fn the_platform_folder_is_the_desktop_hosts_own() {
    let folder = StateFolder::for_platform().unwrap();
    let path = folder.root().to_string_lossy().to_string();
    assert!(
        path.ends_with("ADP Desktop")
            || path.ends_with("net.etalii.adp.desktop")
            || path.ends_with("etalii-adp-desktop"),
        "{path}"
    );
}
