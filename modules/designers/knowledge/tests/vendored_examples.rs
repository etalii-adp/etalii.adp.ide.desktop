//! Every example document vendored from etalii.adp under `definitions/designers/examples/`, opened as the desktop
//! host opens it: its DED envelope says which designer type it is, and a knowledge file is read by this module with
//! nothing to report, the same table in each of its three formats. The Markdown documents are the Hierarchical
//! Requirement Designer's and the post-incident review designer's, which the standalone host at 8b6ccfc has no module
//! for; each is recognised and opens read-only, with the reason.

mod common;

use std::path::PathBuf;

use adp_designer_knowledge::{KnowledgeBody, KnowledgeSession, ORIGIN, validator};
use adp_desl::DedEnvelope;
use adp_desl::table::model::TableChange;
use adp_desl::table::session::DesignerSession;
use common::describe;

fn examples() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(common::vendor().join("definitions/designers/examples"))
        .expect("vendored")
        .map(|e| e.expect("an entry").path())
        .filter(|p| p.extension().is_some_and(|e| e != "adp"))
        .collect();
    files.sort();
    files
}

#[test]
fn every_vendored_example_document_names_its_designer_type() {
    let all = examples();
    assert_eq!(7, all.len(), "{all:?}");
    for path in all {
        let name = path.file_name().and_then(|n| n.to_str()).expect("a name").to_owned();
        let envelope = DedEnvelope::read(&std::fs::read(&path).expect("read"), &name).expect("an envelope");
        let installed = |origin: &str| origin == ORIGIN;
        match envelope.designer.as_str() {
            ORIGIN => assert_eq!(None, envelope.read_only_reason(installed), "{name}"),
            // The Markdown designers the standalone host at 8b6ccfc has no module for: recognised, and read-only.
            designer @ ("etalii/hierarchical-requirement" | "etalii/post-incident-review") => {
                assert!(name.ends_with(".md"));
                let reason = envelope.read_only_reason(installed).expect("read-only");
                assert!(reason.contains(&format!("'{designer}'")), "{reason}");
            }
            other => panic!("{name} names {other}"),
        }
    }
}

#[test]
fn every_vendored_knowledge_file_reads_with_nothing_to_report_and_is_one_table() {
    let mut tables = Vec::new();
    // The cities in each of the three formats; provinces.yaml is the table their relation names (spec 089).
    for path in examples()
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e != "md"))
        .filter(|p| p.file_stem().is_some_and(|s| s == "cities"))
    {
        let bytes = std::fs::read(&path).expect("read");
        let body = KnowledgeBody::read(bytes.clone(), &path.to_string_lossy());
        assert_eq!("", body.read_only_reason(), "{}", path.display());
        assert!(
            body.model().findings.is_empty(),
            "{}: {:?}",
            path.display(),
            body.model().findings
        );
        assert!(validator::validate(body.table(), body.model(), None).is_empty());
        assert_eq!(9, body.table().rows.len());
        assert_eq!(Ok(bytes), body.change(&[]));
        tables.push(describe(body.table()));
    }
    assert_eq!(3, tables.len());
    assert_eq!(tables[0], tables[1]);
    assert_eq!(tables[0], tables[2]);
}

#[test]
fn a_vendored_knowledge_file_opens_in_a_session_and_its_relation_resolves_by_link_beside_it() {
    // The vendored examples hold `provinces.yaml` too, which the cities' relation names by a link to its document.
    let mut session = KnowledgeSession::new(common::vendor().join("definitions/designers/examples/cities.yaml"));
    let baseline = session.baseline();
    assert_eq!("Cities", baseline.title);
    assert_eq!("v1", baseline.settings.view_id);
    assert_eq!("p2", baseline.settings.group_by);
    assert!(baseline.findings.is_empty(), "{:?}", baseline.findings);
    // The related province is shown by its title.
    session.set_window(0, usize::MAX);
    let labels: Vec<String> = session
        .take_changes()
        .into_iter()
        .filter_map(|c| match c {
            TableChange::Rows { rows, .. } => Some(rows),
            _ => None,
        })
        .flatten()
        .filter_map(|r| r.cell("p5").map(|c| c.labels.clone()))
        .flatten()
        .collect();
    assert!(labels.contains(&"Noord-Holland".to_owned()), "{labels:?}");
}
