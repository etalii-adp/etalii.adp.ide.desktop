//! A file an agent wrote that breaks a rule (agent-activity-diagram Requirements 2.8, 2.9, 3.8, 5.6, 8.4, 8.5, 8.7,
//! 8.8 and 10.6): one fixture per finding, each of which still opens, draws what it can, says what is wrong where, and
//! is written back as it was. Ported from the standalone host's `AadFindings.Tests.cs` and the rule test of
//! `AadSurface.Tests.cs` at `8b6ccfc`.

mod common;

use adp_diagram::{DiagramProblemLocation, DiagramProblemSeverity, ElementKind};
use adp_diagram_agent_activity::body::AadBody;
use adp_diagram_agent_activity::commands::AadEdit;
use adp_diagram_agent_activity::{mapper, validator};
use common::{fixture, one_day, open, read};

fn findings(name: &str) -> String {
    fixture(&format!("findings/{name}.aad"))
}

#[test]
fn a_breach_is_said_where_it_is_at_its_weight_and_the_file_still_opens_and_is_kept() {
    for (name, rule, severity, line) in [
        (
            "dangling-reference",
            "fbl.dangling-reference",
            DiagramProblemSeverity::Error,
            6,
        ),
        ("duplicate-id", "std.duplicateId", DiagramProblemSeverity::Error, 6),
        (
            "link-not-openable",
            "aad.link-not-openable",
            DiagramProblemSeverity::Warning,
            4,
        ),
        ("unknown-key", "aad.unknown-key", DiagramProblemSeverity::Info, 4),
        ("missing-id", "std.missingId", DiagramProblemSeverity::Info, 4),
        ("newer-version", "fbl.header-mismatch", DiagramProblemSeverity::Error, 1),
    ] {
        let text = findings(name);
        let document = read(&text);
        let problems = validator::validate(&document.body);
        let found: Vec<_> = problems.iter().filter(|p| p.rule_id == rule).collect();
        assert_eq!(1, found.len(), "{name}: {problems:?}");
        assert_eq!(severity, found[0].severity, "{name}");
        assert_eq!(Some(DiagramProblemLocation::Line(line)), found[0].location, "{name}");
        assert!(document.is_usable(), "{name}: {:?}", document.unreadable);
        assert!(!document.model.elements.is_empty(), "{name}");
        assert_eq!(text, document.body.text(), "{name}");
    }
}

#[test]
fn a_relation_to_an_element_of_the_wrong_type_is_an_error_and_is_drawn_marked_as_a_breach() {
    let text = findings("wrong-type-reference");
    let document = read(&text);
    let problems = validator::validate(&document.body);
    let drawn = mapper::visible(&document);
    let errors: Vec<_> = problems
        .iter()
        .filter(|p| p.severity == DiagramProblemSeverity::Error)
        .collect();
    assert_eq!(1, errors.len(), "{problems:?}");
    assert_eq!(Some(DiagramProblemLocation::Line(9)), errors[0].location);
    assert!(drawn.iter().any(|e| e.id == "p") && drawn.iter().any(|e| e.id == "a"));
    let relations: Vec<_> = drawn.iter().filter(|e| e.kind == ElementKind::Relation).collect();
    assert_eq!(1, relations.len());
    let relation = relations[0];
    assert_eq!(
        (Some("p"), Some("a"), "SpecificationAgent", "true"),
        (
            relation.source.as_deref(),
            relation.target.as_deref(),
            relation.attribute("type"),
            relation.attribute("breach")
        )
    );
    assert_eq!(text, document.body.text());
}

#[test]
fn a_relation_the_rules_allow_is_not_marked() {
    let text = one_day().replace("  showArchived: false\r\n", "  showArchived: true\r\n");
    let relations: Vec<_> = mapper::visible(&read(&text))
        .into_iter()
        .filter(|e| e.kind == ElementKind::Relation)
        .collect();
    assert!(!relations.is_empty());
    for relation in relations {
        assert_eq!("false", relation.attribute("breach"), "{relation:?}");
    }
}

#[test]
fn a_file_of_a_newer_version_is_shown_and_never_changed() {
    let text = findings("newer-version");
    let mut diagram = open(&text);
    let refused = diagram
        .run(AadEdit::SetAttribute {
            element_id: "p".into(),
            attribute: "name".into(),
            value: "Renamed".into(),
        })
        .unwrap_err();
    assert!(refused.contains("version 2"), "{refused}");
    assert_eq!(text, diagram.text());
}

#[test]
fn what_was_set_for_an_element_an_agent_removed_is_not_reported_and_goes_with_the_next_write() {
    let text = findings("orphaned-lock");
    assert!(
        validator::validate(&AadBody::parse(&text))
            .iter()
            .all(|p| p.rule_id != "fbl.dangling-reference")
    );
    let mut diagram = open(&text);
    diagram
        .run(AadEdit::SetAttribute {
            element_id: "p".into(),
            attribute: "name".into(),
            value: "Renamed".into(),
        })
        .unwrap();
    // The edit, and the two entries about nothing gone with it; the lock that names an element stays.
    assert_eq!(
        "agent-activity-diagram: 1\r\n# A lock and a group state left behind by an element an agent removed.\r\nprojects:\r\n  - id: p\r\n    name: Renamed\r\nview:\r\n  placements:\r\n    - element: p\r\n      x: 10\r\n      y: 20\r\n",
        diagram.text()
    );
}

#[test]
fn a_file_that_can_no_longer_be_read_keeps_showing_what_was_last_read_and_is_not_written() {
    let mut diagram = open(&one_day());
    let before = diagram.session.render(diagram.workspace.documents()).unwrap().len();
    let broken = "agent-activity-diagram: 1\r\nprojects: [\r\n";
    diagram.agent_writes(broken);
    let shown = diagram.session.render(diagram.workspace.documents()).unwrap().len();
    let edit = diagram.run(AadEdit::SetAttribute {
        element_id: "p-adp".into(),
        attribute: "name".into(),
        value: "Renamed".into(),
    });
    assert!(before > 5);
    assert_eq!(before, shown);
    // The reader's own sentence says why: the standalone host's YAML library named itself in it, this one names the
    // construct that is not closed.
    assert!(
        validator::validate(&AadBody::parse(broken))
            .iter()
            .any(|p| p.severity == DiagramProblemSeverity::Error && p.rule_id == "std.unparseable"),
        "{:?}",
        validator::validate(&AadBody::parse(broken))
    );
    assert!(edit.is_err());
    assert_eq!(broken, diagram.text());
}

#[test]
fn a_file_that_breaks_a_rule_still_reads_and_says_where_it_breaks_it() {
    // An agent with no specification and no location, and a project nothing belongs to.
    let body = AadBody::parse(
        "agent-activity-diagram: 1\r\nprojects:\r\n  - id: p\r\n    name: Alone\r\nagents:\r\n  - id: a\r\n    name: Idle\r\n    specification: s-gone\r\n",
    );
    let problems = validator::validate(&body);
    // Nothing is an error - a file an agent wrote opens whatever it says.
    assert!(
        problems
            .iter()
            .filter(|p| p.rule_id.starts_with("aad."))
            .all(|p| p.severity != DiagramProblemSeverity::Error)
    );
    assert!(
        problems
            .iter()
            .any(|p| p.rule_id == "aad.project-without-specification" && p.message.contains("Alone")),
        "{problems:?}"
    );
    assert!(
        problems
            .iter()
            .any(|p| p.rule_id == "aad.agent-without-location" && p.location == Some(DiagramProblemLocation::Line(6)))
    );
    // The reference to a specification the file does not have is the reader's finding, on its line.
    assert!(
        problems
            .iter()
            .any(|p| p.message.contains("s-gone") && p.location == Some(DiagramProblemLocation::Line(8)))
    );
}
