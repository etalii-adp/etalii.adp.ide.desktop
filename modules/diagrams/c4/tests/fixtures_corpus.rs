//! Guards on the corpus itself, not on any code that reads it: each fixture pins one property, and an edit that
//! strips tabs, normalises line endings or tidies a stray brace would quietly remove what it guards (c4-diagrams
//! task 1). Ported from the standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/Fixtures.Tests.cs`
//! at `8b6ccfc`.

use std::fs;
use std::path::{Path, PathBuf};

const ALL_FIXTURES: [&str; 12] = [
    "minimal.dsl",
    "comments-everywhere.dsl",
    "crlf-line-endings.dsl",
    "lf-line-endings.dsl",
    "no-trailing-newline.dsl",
    "unusual-indentation.dsl",
    "unmodelled-constructs.dsl",
    "deployment-nested.dsl",
    "dynamic-interactions.dsl",
    "big-bank-plc.dsl",
    "aws-deployment.dsl",
    "crowded-component-view.dsl",
];

fn path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

fn bytes(name: &str) -> String {
    fs::read_to_string(path(name)).unwrap()
}

#[test]
fn every_fixture_is_present_and_non_empty() {
    for name in ALL_FIXTURES {
        assert!(path(name).is_file(), "{name} is missing from the corpus");
        assert!(!bytes(name).is_empty(), "{name}");
    }
}

#[test]
fn the_crlf_and_lf_fixtures_differ_only_in_their_line_endings() {
    let crlf = bytes("crlf-line-endings.dsl");
    let lf = bytes("lf-line-endings.dsl");
    assert!(crlf.contains("\r\n"));
    assert!(!lf.contains('\r'));
    assert_eq!(lf, crlf.replace("\r\n", "\n"));
}

#[test]
fn the_no_trailing_newline_fixture_still_ends_without_one() {
    assert!(!bytes("no-trailing-newline.dsl").ends_with('\n'));
}

#[test]
fn the_indentation_fixture_still_mixes_tabs_and_spaces() {
    let text = bytes("unusual-indentation.dsl");
    assert!(text.contains('\t'));
    assert!(text.contains("\n          s = softwareSystem"));
}

#[test]
fn the_comments_fixture_still_carries_every_comment_form() {
    let text = bytes("comments-everywhere.dsl");
    for form in ["//", "#", "/*"] {
        assert!(text.contains(form), "{form}");
    }
}

#[test]
fn the_unmodelled_fixture_still_carries_the_constructs_requirement_3_3_protects() {
    let text = bytes("unmodelled-constructs.dsl");
    for construct in ["!identifiers", "!docs", "!adrs", "styles", "theme", "configuration"] {
        assert!(text.contains(construct), "{construct}");
    }
}

#[test]
fn the_deployment_fixture_still_nests_and_still_reuses_one_container() {
    let text = bytes("deployment-nested.dsl");
    assert!(text.contains("infrastructureNode"));
    assert_eq!(text.matches("containerInstance db").count(), 2);
}

#[test]
fn the_real_world_documents_are_present_and_the_readme_says_where_they_came_from() {
    let readme = bytes("readme.md");
    for name in ["big-bank-plc.dsl", "aws-deployment.dsl"] {
        assert!(path(name).is_file(), "{name} is missing");
        assert!(readme.contains(name), "{name}");
    }
    assert!(readme.to_lowercase().contains("hand-written"));
    assert!(readme.contains("Structurizr CLI"));
    assert!(readme.contains("described as canonical"));
}
