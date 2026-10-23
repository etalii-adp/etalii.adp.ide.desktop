//! `atd check --trace` (etalii.adp spec 035, T048, ATD §2.5): every `statedBy` and `divergesFrom` of a trace resolves
//! in its description, and one that does not is a finding at the trace's entry; host concerns are ATD §17's; every
//! item of the companion's named sections is traced exactly once.

use std::collections::BTreeMap;
use std::path::PathBuf;

use adp_atd_check::trace::check_trace;
use adp_atd_check::{FsFiles, Overlay};

fn spec_dir() -> PathBuf {
    match std::env::var_os("ATD_SPEC_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/specifications/atd"),
    }
}

/// The findings of the mind map's trace with `edit` applied to its text, as messages with their positions.
fn findings_of(edit: impl Fn(String) -> String) -> Vec<(String, usize, usize, String)> {
    let path = spec_dir().join("mindmap.trace.json");
    let text = edit(std::fs::read_to_string(&path).unwrap());
    let files = Overlay {
        open: BTreeMap::from([(path.clone(), text)]),
        under: &FsFiles,
    };
    check_trace(&files, &path)
        .unwrap()
        .into_iter()
        .map(|f| (f.code.to_owned(), f.line, f.column, f.message))
        .collect()
}

#[test]
fn the_worked_example_has_no_findings() {
    assert_eq!(findings_of(|t| t), vec![]);
}

#[test]
fn the_construct_paths_of_atd_section_2_5_resolve_in_the_mind_map() {
    for stated_by in [
        "fn siblingName",
        "operation addSibling",
        "binding freeplane element node",
        "notation node Node label text",
        "capability layout.twoSidedTree",
        "algorithm mindmap",
        "constraint unnamedCentralTopic",
        "element Node attribute folded",
        "reason atStart",
    ] {
        let found = findings_of(|t| {
            t.replacen(
                "\"statedBy\": \"fn siblingName\"",
                &format!("\"statedBy\": \"{stated_by}\""),
                1,
            )
        });
        assert_eq!(found, vec![], "{stated_by}");
    }
}

#[test]
fn a_path_that_names_nothing_is_a_finding_at_the_trace_entry() {
    let found = findings_of(|t| {
        t.replacen(
            "\"statedBy\": \"fn siblingName\"",
            "\"statedBy\": \"fn siblingNames\"",
            1,
        )
    });
    assert_eq!(found.len(), 1, "{found:?}");
    let (code, line, column, message) = &found[0];
    assert_eq!(code, "ATD2011");
    assert_eq!((*line, *column), (15, 20), "at the value of the first item's statedBy");
    assert!(message.contains("`fn siblingNames` names no construct"), "{message}");

    for wrong in [
        "operation addNephew",
        "binding freeplane element nodes",
        "notation node Nodes",
        "capability layout.tree",
    ] {
        let found = findings_of(|t| {
            t.replacen(
                "\"statedBy\": \"capability layout.twoSidedTree\"",
                &format!("\"statedBy\": \"{wrong}\""),
                1,
            )
        });
        assert_eq!(found.len(), 1, "{wrong}: {found:?}");
    }
}

#[test]
fn a_host_concern_is_one_of_section_17s() {
    let found = findings_of(|t| t.replacen("\"hostConcern\": \"rendering\"", "\"hostConcern\": \"pixels\"", 1));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].3.contains("`pixels` is not a host concern"), "{}", found[0].3);
}

#[test]
fn every_companion_item_is_traced_exactly_once() {
    let missing = findings_of(|t| {
        t.replacen(
            "1-what-disl-still-lacks-for-this-type/2",
            "1-what-disl-still-lacks-for-this-type/9",
            1,
        )
    });
    let messages: Vec<&str> = missing.iter().map(|f| f.3.as_str()).collect();
    assert_eq!(missing.len(), 2, "{messages:?}");
    assert!(
        messages
            .iter()
            .any(|m| m.contains("item 1-what-disl-still-lacks-for-this-type/2") && m.contains("traced 0 times"))
    );
    assert!(
        messages
            .iter()
            .any(|m| m.contains("names the item 1-what-disl-still-lacks-for-this-type/9"))
    );
}
