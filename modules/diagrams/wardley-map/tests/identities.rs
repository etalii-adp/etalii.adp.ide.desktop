//! The identity sidecar and the reconciliation of recorded identities with a parsed map (etalii.adp spec 030, task
//! T070), ported from the standalone host's `WardleyIdentities.Tests.cs` and `WardleyIdentities.Reconcile.Tests.cs` at
//! `8b6ccfc`. `Read_RefusesABodyPathThatIsNotOne` and `Reconcile_RejectsNulls` have no counterpart: a Rust path or
//! reference is never null.

mod common;

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use adp_diagram_wardley_map::identities::{self, ShortGuids, WardleyIdentityEntry, kind};
use adp_diagram_wardley_map::model::WardleyMap;
use common::{parse_fixture, parse_text};

fn entry(id: &str, kind: &str, key: &str) -> WardleyIdentityEntry {
    WardleyIdentityEntry::new(id, kind, key)
}

fn reconcile(map: &WardleyMap, recorded: &[WardleyIdentityEntry]) -> Vec<WardleyIdentityEntry> {
    identities::reconcile(map, recorded, &ShortGuids)
}

#[test]
fn path_for_puts_the_sidecar_beside_the_body_with_the_same_base_name() {
    assert_eq!(
        identities::path_for(&Path::new("some").join("folder").join("supply-chain.owm")),
        Path::new("some").join("folder").join("supply-chain.identities.json")
    );
}

#[test]
fn path_for_handles_a_body_with_no_folder() {
    assert_eq!(
        identities::path_for(Path::new("map.owm")),
        Path::new("map.identities.json")
    );
}

#[test]
fn read_returns_nothing_when_there_is_no_sidecar() {
    let folder = tempfile::tempdir().unwrap();
    assert!(identities::read(&folder.path().join("map.owm")).is_empty());
}

#[test]
fn write_then_read_round_trips_the_entries() {
    let folder = tempfile::tempdir().unwrap();
    let body = folder.path().join("map.owm");
    let written = vec![
        entry("abc123", "component", "Cup of Tea"),
        entry("def456", "link", "Cup of Tea->Kettle"),
        entry("ghi789", "annotation", "1"),
    ];
    assert_eq!(identities::write(&body, &written), "");
    assert_eq!(identities::read(&body), written);
}

#[test]
fn write_replaces_what_was_there_rather_than_appending() {
    let folder = tempfile::tempdir().unwrap();
    let body = folder.path().join("map.owm");
    identities::write(&body, &[entry("abc123", "component", "Old")]);
    identities::write(&body, &[entry("def456", "component", "New")]);
    assert_eq!(identities::read(&body), [entry("def456", "component", "New")]);
}

#[test]
fn write_leaves_no_temporary_file_behind() {
    let folder = tempfile::tempdir().unwrap();
    identities::write(
        &folder.path().join("map.owm"),
        &[entry("abc123", "component", "Cup of Tea")],
    );
    let names: Vec<String> = fs::read_dir(folder.path())
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["map.identities.json"]);
}

#[test]
fn write_removes_the_sidecar_when_there_is_nothing_left_to_keep() {
    let folder = tempfile::tempdir().unwrap();
    let body = folder.path().join("map.owm");
    identities::write(&body, &[entry("abc123", "component", "Cup of Tea")]);
    identities::write(&body, &[]);
    assert!(!identities::path_for(&body).exists());
}

#[test]
fn read_returns_nothing_when_the_sidecar_is_not_json() {
    let folder = tempfile::tempdir().unwrap();
    let body = folder.path().join("map.owm");
    fs::write(identities::path_for(&body), "this is not json {{{").unwrap();
    assert!(identities::read(&body).is_empty());
}

#[test]
fn read_returns_nothing_when_the_sidecar_is_json_of_the_wrong_shape() {
    let folder = tempfile::tempdir().unwrap();
    let body = folder.path().join("map.owm");
    fs::write(identities::path_for(&body), r#"{"not":"a list"}"#).unwrap();
    assert!(identities::read(&body).is_empty());
}

#[test]
fn read_discards_a_half_written_entry_and_keeps_the_rest() {
    let folder = tempfile::tempdir().unwrap();
    let body = folder.path().join("map.owm");
    fs::write(
        identities::path_for(&body),
        r#"[
  { "id": "abc123", "kind": "component", "key": "Cup of Tea" },
  { "id": "", "kind": "component", "key": "No id" },
  { "id": "def456", "kind": "component", "key": "" },
  { "id": "ghi789", "kind": "component", "key": "Kettle" }
]"#,
    )
    .unwrap();
    assert_eq!(
        identities::read(&body),
        [
            entry("abc123", "component", "Cup of Tea"),
            entry("ghi789", "component", "Kettle")
        ]
    );
}

#[test]
fn remove_deletes_the_sidecar() {
    let folder = tempfile::tempdir().unwrap();
    let body = folder.path().join("map.owm");
    identities::write(&body, &[entry("abc123", "component", "Cup of Tea")]);
    identities::remove(&body);
    assert!(!identities::path_for(&body).exists());
}

#[test]
fn remove_is_silent_when_there_is_no_sidecar() {
    let folder = tempfile::tempdir().unwrap();
    let body = folder.path().join("map.owm");
    identities::remove(&body);
    assert!(!identities::path_for(&body).exists());
}

#[test]
fn write_does_not_touch_the_owm_itself() {
    let folder = tempfile::tempdir().unwrap();
    let body = folder.path().join("map.owm");
    fs::write(&body, "title Untouched\n").unwrap();
    identities::write(&body, &[entry("abc123", "component", "Cup of Tea")]);
    assert_eq!(fs::read_to_string(&body).unwrap(), "title Untouched\n");
}

/// The standalone host's own sidecar of its `strategy-vocabulary` example, committed with LF endings, is what
/// `System.Text.Json` writes for its entries: indented, camel-cased, the unit separator escaped `\u001F`.
#[test]
fn write_is_what_system_text_json_writes() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../parity/fixtures/wardley/map/strategy-vocabulary/strategy-vocabulary.owm");
    let entries = identities::read(&fixture);
    assert_eq!(entries.len(), 22);
    assert_eq!(
        identities::to_json(&entries, "\n"),
        fs::read_to_string(identities::path_for(&fixture)).unwrap()
    );
    assert_eq!(identities::json_string("a\u{1F}b+'c"), r#""a\u001Fb\u002B\u0027c""#);
}

// ---- WardleyIdentities.Reconcile.Tests.cs ---------------------------------------------------------------------------

#[test]
fn reconcile_assigns_an_id_to_every_element_when_nothing_is_recorded() {
    let map = parse_fixture("tea-shop.owm");
    let entries = reconcile(&map, &[]);
    assert_eq!(entries.len(), map.components.len() + map.links.len());
    assert!(entries.iter().all(|e| !e.id.is_empty()));
}

#[test]
fn reconcile_gives_every_element_a_distinct_id() {
    let entries = reconcile(&parse_fixture("strategy-vocabulary.owm"), &[]);
    let distinct: HashSet<&str> = entries.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(distinct.len(), entries.len());
}

#[test]
fn reconcile_keeps_the_recorded_id_for_an_element_it_still_recognises() {
    let map = parse_text("component Cup of Tea [0.79, 0.61]\n");
    let first = reconcile(&map, &[]);
    assert_eq!(reconcile(&map, &first), first);
}

#[test]
fn reconcile_keeps_an_id_when_the_element_moves() {
    let recorded = reconcile(&parse_text("component Cup of Tea [0.79, 0.61]\n"), &[]);
    let entries = reconcile(&parse_text("component Cup of Tea [0.20, 0.90]\n"), &recorded);
    assert_eq!(entries[0].id, recorded[0].id);
}

#[test]
fn reconcile_keeps_an_id_when_a_decorator_is_added_or_removed() {
    let recorded = reconcile(&parse_text("component Payment [0.70, 0.72]\n"), &[]);
    let entries = reconcile(&parse_text("component Payment [0.70, 0.72] (buy)\n"), &recorded);
    assert_eq!(entries[0].id, recorded[0].id);
}

#[test]
fn reconcile_discards_an_entry_whose_element_is_gone() {
    let recorded = reconcile(&parse_text("component Gone [0.5, 0.5]\n"), &[]);
    let entries = reconcile(&parse_text("component Present [0.5, 0.5]\n"), &recorded);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].key, "Present");
    assert_ne!(entries[0].id, recorded[0].id);
}

#[test]
fn reconcile_treats_a_rename_as_a_new_element_which_is_why_rename_rewrites_the_key_itself() {
    let recorded = reconcile(&parse_text("component Kettle [0.43, 0.35]\n"), &[]);
    let entries = reconcile(&parse_text("component Boiler [0.43, 0.35]\n"), &recorded);
    assert_eq!(entries.len(), 1);
    assert_ne!(entries[0].id, recorded[0].id);
}

#[test]
fn reconcile_distinguishes_elements_of_different_kinds_sharing_a_key() {
    let entries = reconcile(
        &parse_text("component Renewal [0.5, 0.5]\nnote Renewal [0.2, 0.3]\n"),
        &[],
    );
    assert_eq!(entries.len(), 2);
    assert_ne!(entries[0].id, entries[1].id);
    assert!(entries.iter().any(|e| e.kind == kind::COMPONENT));
    assert!(entries.iter().any(|e| e.kind == kind::NOTE));
}

#[test]
fn reconcile_distinguishes_two_links_between_the_same_pair() {
    let entries = reconcile(&parse_text("Alpha->Beta\nAlpha+>Beta\n"), &[]);
    let links: Vec<_> = entries.iter().filter(|e| e.kind == kind::LINK).collect();
    assert_eq!(links.len(), 2);
    assert_ne!(links[0].key, links[1].key);
}

#[test]
fn reconcile_distinguishes_pipeline_children_sharing_a_name_across_pipelines() {
    let entries = reconcile(
        &parse_text(
            "component Kettle [0.4, 0.4]\npipeline Kettle\n{\n  component Electric [0.6]\n}\ncomponent Stove [0.3, 0.3]\npipeline Stove\n{\n  component Electric [0.7]\n}",
        ),
        &[],
    );
    let children: Vec<_> = entries.iter().filter(|e| e.kind == kind::PIPELINE_CHILD).collect();
    assert_eq!(children.len(), 2);
    assert_ne!(children[0].key, children[1].key);
}

#[test]
fn reconcile_covers_the_strategy_vocabularys_notes_and_annotations() {
    let entries = reconcile(&parse_fixture("annotations-and-labels.owm"), &[]);
    assert!(entries.iter().any(|e| e.kind == kind::NOTE));
    assert_eq!(entries.iter().filter(|e| e.kind == kind::ANNOTATION).count(), 2);
}

#[test]
fn reconcile_is_stable_so_an_unedited_map_produces_nothing_to_write() {
    let map = parse_fixture("strategy-vocabulary.owm");
    let first = reconcile(&map, &[]);
    assert_eq!(reconcile(&map, &first), first);
}

#[test]
fn reconcile_returns_nothing_for_an_empty_map() {
    assert!(reconcile(&WardleyMap::default(), &[]).is_empty());
}

/// The example's sidecar names every element of its map, so reconciling it mints nothing.
#[test]
fn reconcile_of_the_examples_own_sidecar_keeps_every_id() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../parity/fixtures/wardley/map/strategy-vocabulary/strategy-vocabulary.owm");
    let recorded = identities::read(&fixture);
    let map = parse_text(&fs::read_to_string(&fixture).unwrap());
    let minted = adp_disl::behavior::ids::fixed(&[]);
    assert_eq!(identities::reconcile(&map, &recorded, &minted), recorded);
}
