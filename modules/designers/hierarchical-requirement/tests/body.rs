//! Reading a requirement document (the standalone host's `HierarchicalRequirementBody.Tests.cs` at a7fdf03): every
//! fault of the companion's table "What reading reports" gives its one finding at its one bullet, or a read-only
//! document, with every other entry usable; a placeholder is numbered in memory only; and opening leaves both files'
//! bytes as they were.

mod common;

use std::fs;

use adp_designer_hierarchical_requirement::body::RequirementBody;
use adp_designer_hierarchical_requirement::model::{AssessmentKind, Member};
use common::{examples_folder, fixture, read, read_with, web_shop};

fn codes_and_lines(body: &RequirementBody) -> Vec<(String, usize)> {
    body.document
        .findings
        .iter()
        .map(|f| (f.code.clone(), f.line))
        .collect()
}

#[test]
fn the_example_reads_with_exactly_its_missing_id() {
    let body = web_shop();

    // One finding, at REQ-NEW, which reads as the next number after the mark.
    assert_eq!(vec![("std.missingId".to_owned(), 44)], codes_and_lines(&body));
    let numbered: Vec<_> = body.document.entries.iter().filter(|e| e.placeholder).collect();
    assert_eq!(1, numbered.len());
    assert_eq!(
        ("REQ-0013", Some("ITEM-0003")),
        (numbered[0].id.as_str(), numbered[0].parent_id.as_deref())
    );
    assert_eq!(
        (Some("ITEM-0007"), Some("REQ-0012")),
        (
            body.document.last_item.as_deref(),
            body.document.last_requirement.as_deref()
        )
    );
    assert_eq!("", body.document.read_only_reason);
}

#[test]
fn each_fault_is_one_finding_at_its_bullet_and_every_other_entry_is_read() {
    let body = read(&fixture("hierarchical-requirement-faults/web-shop.md"));

    // The findings of the fixture, in DESL's order: the reader's in reading order, then required, containment and
    // mentions.
    let expected = [
        ("std.duplicateId", 15),
        ("std.missingId", 16),
        ("std.unreadableEntry", 18),
        ("std.missingId", 22),
        ("std.unreadableEntry", 28),
        ("std.required", 17),
        ("std.containment", 14),
        ("std.containment", 21),
        ("std.mentions", 13),
        ("std.mentions", 29),
    ];
    assert_eq!(
        expected.map(|(code, line)| (code.to_owned(), line)).to_vec(),
        codes_and_lines(&body)
    );

    // And the entries around them are read: the duplicate under its written number, the placeholders numbered after
    // the marks.
    let ids: Vec<&str> = body.document.entries.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(
        vec![
            "ITEM-0001",
            "REQ-0001",
            "REQ-0002",
            "REQ-0003",
            "REQ-0002",
            "REQ-0009",
            "REQ-0004",
            "REQ-0006",
            "ITEM-0005",
            "REQ-0007"
        ],
        ids
    );
    let assessments: Vec<&str> = body.document.assessments.iter().map(|a| a.id.as_str()).collect();
    assert_eq!(vec!["assessment/0", "assessment/1"], assessments);
    assert_eq!("", body.document.read_only_reason);
}

#[test]
fn a_file_without_the_envelope_or_not_utf8_is_read_only_with_one_finding() {
    for (path, line) in [
        ("hierarchical-requirement-faults/no-envelope/web-shop.md", 1),
        ("hierarchical-requirement-faults/not-utf8/cafe.md", 3),
    ] {
        let body = read(&fixture(path));
        assert_eq!(
            vec![("std.unparseable".to_owned(), line)],
            codes_and_lines(&body),
            "{path}"
        );
        assert_ne!("", body.document.read_only_reason, "{path}");
        assert!(body.document.entries.is_empty(), "{path}");
    }
}

#[test]
fn a_later_ded_version_is_read_only_with_the_reason_and_still_read() {
    let text = fs::read_to_string(fixture("hierarchical-requirement-read/library.md"))
        .unwrap()
        .replace("ded: 0.2", "ded: 0.9");

    let body = RequirementBody::read(text.into_bytes(), "library.md", None);

    assert!(
        body.document.read_only_reason.contains("DED 0.9"),
        "{}",
        body.document.read_only_reason
    );
    assert_eq!(14, body.document.entries.len());
}

#[test]
fn an_assessments_kind_and_checkbox_are_read_in_any_case() {
    let body = read(&fixture("hierarchical-requirement-assessments/web-shop.md"));
    let assessments = &body.document.assessments;

    assert_eq!(
        vec![
            (AssessmentKind::Question, false),
            (AssessmentKind::Warning, true),
            (AssessmentKind::Remark, false),
            (AssessmentKind::Idea, true),
            (AssessmentKind::Question, false),
        ],
        assessments.iter().map(|a| (a.kind, a.resolved)).collect::<Vec<_>>()
    );
    assert_eq!(
        "The gateway's session ends first, so ours never times out alone.",
        assessments[1].detail
    );
}

#[test]
fn the_lists_are_the_registrations_else_the_initial_and_the_implied_members() {
    let stored = read_with(
        &fixture("hierarchical-requirement-filters/requirements.md"),
        &fixture("hierarchical-requirement-filters/requirements.adp"),
    )
    .document;
    let implied = read(&fixture("hierarchical-requirement-tags/requirements.md")).document;

    // A block the registration holds is the list; without one, DESL's initial members and then those the entries
    // carry.
    assert_eq!(
        vec![Member::new("security", "red"), Member::new("ux", "blue")],
        stored.tags
    );
    let names = |members: &[Member]| members.iter().map(|m| m.name.clone()).collect::<Vec<_>>();
    assert_eq!(vec!["security", "ux", "self-service", "privacy"], names(&implied.tags));
    assert!(implied.tags.iter().all(|tag| tag.colour == "default"));
    assert_eq!(
        vec!["functional", "non-functional", "technical"],
        names(&implied.categories)
    );
    assert_eq!(vec!["proposed", "accepted", "rejected", "done"], names(&implied.states));
    assert!(stored.entries.iter().find(|e| e.id == "ITEM-0002").unwrap().collapsed);
}

#[test]
fn a_registrations_findings_are_its_own() {
    let body = read_with(
        &fixture("hierarchical-requirement-collapsed/stale/lending.md"),
        &fixture("hierarchical-requirement-collapsed/stale/lending.adp"),
    );

    assert_eq!(1, body.document.findings.len());
    let finding = &body.document.findings[0];
    assert_eq!(
        ("fbl.stale-view-data", 4, true),
        (finding.code.as_str(), finding.line, finding.in_registration)
    );
}

#[test]
fn opening_leaves_both_files_bytes_as_they_were() {
    let body = fs::read(examples_folder().join("web-shop.md")).unwrap();
    let registration = fs::read(examples_folder().join("web-shop.adp")).unwrap();

    // The placeholder is numbered, in memory only.
    let read = RequirementBody::read(
        body.clone(),
        "web-shop.md",
        Some((registration.clone(), "web-shop.adp")),
    );

    assert_eq!(body, read.body.bytes());
    assert_eq!(registration, read.registration.as_ref().unwrap().bytes());
    assert!(!read.body.can_undo());
}
