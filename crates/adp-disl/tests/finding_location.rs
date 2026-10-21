//! Where a finding is and what it is about (DISL sections 8.2 and 8.6): an element's `location()` (section 12.2), a
//! rule's `location` and `subject`, and a file that does not parse silencing only the findings located in it (the
//! standalone host's `FindingLocationTests`).

mod common;

use adp_cel::{Map, Value};
use adp_disl::derivation::constraints;
use adp_disl::{
    DislConstraintOptions, DislDiagram, DislElement, DislFinding, DislReaderFinding, DislSpecification, NewElement,
    contexts,
};
use common::{attrs, loaded, vars, with};

const METAMODEL: &str = r#""metamodel": {
      "types": {
        "Box": { "attributes": { "name": { "type": "string" }, "size": { "type": "int" } } }
      }
    }"#;

fn line(finding: &DislFinding) -> String {
    format!(
        "{}:{} | {} | {} | {} | {}",
        finding.file.as_deref().unwrap_or("-"),
        finding.line.map(|l| l.to_string()).unwrap_or_else(|| "-".into()),
        finding.code,
        finding.message,
        finding.element_ids.join(","),
        finding.subject.as_deref().unwrap_or("-")
    )
}

fn a_box(diagram: &DislDiagram, id: &str, file: Option<&str>, line: Option<i32>, attributes: Map) -> DislElement {
    let mut new = NewElement::new("Box", id);
    new.attributes = attributes;
    new.file = file.map(str::to_owned);
    new.line = line;
    diagram.add(new).unwrap()
}

fn with_rules(rules: &str) -> DislSpecification {
    loaded(&with(&format!(
        r#"{METAMODEL}, "constraints": {{ "rules": [ {rules} ] }}"#
    )))
}

fn reading(finding: DislReaderFinding) -> DislConstraintOptions {
    DislConstraintOptions {
        reader_findings: vec![finding],
        ..Default::default()
    }
}

#[test]
fn an_elements_location_is_the_file_and_line_it_was_read_at_or_none() {
    // Arrange.
    let specification = loaded(&with(METAMODEL));
    let diagram = DislDiagram::new(&specification);
    diagram.set_file(Some("main.yaml"));
    let placed = a_box(&diagram, "placed", Some("charts/a.yaml"), Some(4), Map::default());
    let whole_file = a_box(&diagram, "wholeFile", Some("b.yaml"), None, Map::default());
    let primary = a_box(&diagram, "primary", None, Some(9), Map::default());
    let unplaced = a_box(
        &DislDiagram::new(&specification),
        "unplaced",
        None,
        None,
        Map::default(),
    );
    let program = specification
        .environment(contexts::ELEMENT, &[])
        .compile("self.location()")
        .unwrap();

    // Act.
    let location_of = |element: &DislElement| program.evaluate(&vars([("self", element.to_value())])).unwrap();

    // Assert.
    assert_eq!(
        Value::some(Value::map(attrs([
            ("file", "charts/a.yaml".into()),
            ("line", Value::Int(4))
        ]))),
        location_of(&placed)
    );
    assert_eq!(
        Value::some(Value::map(attrs([("file", "b.yaml".into())]))),
        location_of(&whole_file)
    );
    assert_eq!(
        Value::some(Value::map(attrs([
            ("file", "main.yaml".into()),
            ("line", Value::Int(9))
        ]))),
        location_of(&primary)
    );
    assert_eq!(Value::none(), location_of(&unplaced));
}

#[test]
fn a_rule_without_a_location_is_located_at_its_target() {
    // Arrange.
    let specification =
        with_rules(r#"{ "id": "named", "scope": "Box", "rule": "self.name != ''", "message": "No name." }"#);
    let diagram = DislDiagram::new(&specification);
    a_box(
        &diagram,
        "a",
        Some("values.yaml"),
        Some(3),
        attrs([("name", "".into())]),
    );
    a_box(&diagram, "b", None, Some(5), attrs([("name", "".into())]));

    // Act.
    let findings = constraints::evaluate(&specification, &diagram, &DislConstraintOptions::default());

    // Assert.
    assert_eq!(
        vec![
            "values.yaml:3 | named | No name. | a | -",
            "-:5 | named | No name. | b | -"
        ],
        findings.iter().map(line).collect::<Vec<_>>()
    );
}

#[test]
fn a_rules_location_and_subject_are_evaluated_per_finding_and_per_item() {
    // Arrange.
    let specification = with_rules(
        r#"{ "id": "elsewhere", "scope": "Box", "rule": "false", "message": "Elsewhere.",
          "location": "{'file': 'Chart.yaml', 'line': self.size}", "subject": "'key ' + self.name" },
        { "id": "nowhere", "scope": "Box", "rule": "false", "message": "Nowhere.", "location": "null" },
        { "id": "perItem", "scope": "Box", "forEach": "[1, 2]", "rule": "false", "message": "Item.",
          "location": "item == 1 ? {'file': 'lock.yaml', 'line': 7} : {'file': 'lock.yaml'}" }"#,
    );
    let diagram = DislDiagram::new(&specification);
    a_box(
        &diagram,
        "a",
        Some("values.yaml"),
        Some(3),
        attrs([("name", "x".into()), ("size", Value::Int(12))]),
    );

    // Act.
    let findings = constraints::evaluate(&specification, &diagram, &DislConstraintOptions::default());

    // Assert.
    assert_eq!(
        vec![
            "Chart.yaml:12 | elsewhere | Elsewhere. | a | key x",
            "-:- | nowhere | Nowhere. | a | -",
            "lock.yaml:7 | perItem | Item. | a | -",
            "lock.yaml:- | perItem | Item. | a | -",
        ],
        findings.iter().map(line).collect::<Vec<_>>()
    );
}

#[test]
fn an_unparseable_file_of_a_folder_silences_only_the_findings_located_in_it() {
    // Arrange.
    let specification = with_rules(
        r#"{ "id": "never", "scope": "Box", "rule": "false", "message": "Never." },
        { "id": "pointsAtA", "scope": "Box", "when": "self.id == 'b'", "rule": "false", "message": "About a.", "location": "{'file': 'a.yaml', 'line': 1}" }"#,
    );
    let diagram = DislDiagram::new(&specification);
    a_box(&diagram, "a", Some("a.yaml"), Some(2), Map::default());
    a_box(&diagram, "b", Some("b.yaml"), Some(2), Map::default());

    // Act.
    let findings = constraints::evaluate(
        &specification,
        &diagram,
        &reading(DislReaderFinding::not_parsed("tab", Some(1), Some("a.yaml"))),
    );

    // Assert.
    assert_eq!(
        vec![
            "a.yaml:1 | std.unparseable | tab |  | -",
            "b.yaml:2 | never | Never. | b | -"
        ],
        findings.iter().map(line).collect::<Vec<_>>()
    );
}

#[test]
fn an_unparseable_primary_file_still_stops_every_constraint() {
    // Arrange.
    let specification = with_rules(r#"{ "id": "never", "scope": "Box", "rule": "false", "message": "Never." }"#);
    let diagram = DislDiagram::new(&specification);
    diagram.set_file(Some("main.yaml"));
    a_box(&diagram, "a", Some("other.yaml"), Some(2), Map::default());

    // Act.
    let named = constraints::evaluate(
        &specification,
        &diagram,
        &reading(DislReaderFinding::not_parsed("tab", Some(1), Some("main.yaml"))),
    );
    let unnamed = constraints::evaluate(
        &specification,
        &diagram,
        &reading(DislReaderFinding::not_parsed("tab", Some(1), None)),
    );

    // Assert.
    assert_eq!(
        vec!["main.yaml:1 | std.unparseable | tab |  | -"],
        named.iter().map(line).collect::<Vec<_>>()
    );
    assert_eq!(
        vec!["-:1 | std.unparseable | tab |  | -"],
        unnamed.iter().map(line).collect::<Vec<_>>()
    );
}
