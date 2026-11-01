//! The findings of a diagram (DISL sections 8.2, 8.6 and 8.7) and the refusals of a gesture (section 8.4) (the
//! standalone host's `ConstraintEvaluationTests`).

mod common;

use std::collections::HashMap;
use std::rc::Rc;

use adp_cel::{Map, Value};
use adp_disl::derivation::{constraints, gestures};
use adp_disl::{
    DislConstraintOptions, DislDiagram, DislElement, DislFinding, DislReaderFinding, DislSpecification, NewElement,
};
use common::{attrs, loaded, with};

const METAMODEL: &str = r#""metamodel": {
      "types": {
        "Box": { "attributes": { "name": { "type": "string" }, "size": { "type": "int" } } },
        "Lid": { "attributes": { "name": { "type": "string" } } }
      },
      "relations": {
        "Link": { "source": "Box", "target": "Box", "directed": true, "allowSelfLoops": false, "allowParallel": false }
      }
    }"#;

fn line(finding: &DislFinding) -> String {
    format!(
        "{} | {} | {} | {} | {}",
        finding.line.map(|l| l.to_string()).unwrap_or_default(),
        finding.code,
        finding.severity,
        finding.message,
        finding.element_ids.join(",")
    )
}

fn node(diagram: &DislDiagram, type_name: &str, id: &str, line: i32, attributes: Map) -> DislElement {
    let mut new = NewElement::new(type_name, id);
    new.attributes = attributes;
    new.line = Some(line);
    diagram.add(new).unwrap()
}

fn relation(
    diagram: &DislDiagram,
    id: &str,
    source: Option<&DislElement>,
    target: Option<&DislElement>,
    line: i32,
    source_id: Option<&str>,
) {
    let mut new = NewElement::new("Link", id);
    new.line = Some(line);
    new.source = source.cloned();
    new.target = target.cloned();
    new.source_id = source_id.map(str::to_owned).or_else(|| source.map(DislElement::id));
    new.target_id = target.map(DislElement::id);
    diagram.add(new).unwrap();
}

fn specification(rest: &str) -> DislSpecification {
    loaded(&with(&format!("{METAMODEL},{rest}")))
}

fn written(map: &[(&str, &str)]) -> DislConstraintOptions {
    let map: HashMap<String, String> = map.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
    DislConstraintOptions {
        written_id: Some(Rc::new(move |e: &DislElement| {
            map.get(&e.id()).cloned().unwrap_or_else(|| e.id())
        })),
        ..Default::default()
    }
}

#[test]
fn a_live_rule_reports_each_element_its_scope_takes_with_its_code_severity_and_message() {
    let specification = specification(
        r#""constraints": {
          "defaults": { "severity": "warning" },
          "rules": [
            { "id": "named", "code": "box.unnamed", "scope": "Box", "rule": "self.name != ''", "message": { "cel": "'`' + self.id + '` has no name.'" } },
            { "id": "small", "scope": "Box", "when": "has(self.size)", "rule": "self.size < 10", "severity": "error", "message": "Too big." },
            { "id": "off", "scope": "Box", "enabled": false, "rule": "false", "message": "Never." },
            { "id": "saveOnly", "scope": "Box", "timing": ["save"], "rule": "false", "message": "Only on save." }
          ]
        }"#,
    );
    let diagram = DislDiagram::new(&specification);
    node(
        &diagram,
        "Box",
        "a",
        2,
        attrs([("name", "".into()), ("size", Value::Int(12))]),
    );
    node(&diagram, "Box", "b", 5, attrs([("name", "B".into())]));
    node(&diagram, "Lid", "c", 7, attrs([("name", "".into())]));

    let findings = constraints::evaluate(&specification, &diagram, &DislConstraintOptions::default());

    assert_eq!(
        vec![
            "2 | box.unnamed | warning | `a` has no name. | a",
            "2 | small | error | Too big. | a"
        ],
        findings.iter().map(line).collect::<Vec<_>>()
    );
}

#[test]
fn the_built_ins_report_endpoints_references_and_duplicate_ids_in_the_specifications_order() {
    let specification = specification(
        r#""constraints": {
          "builtIn": {
            "std.endpoints": { "message": { "cel": "detail.violation + ' ' + self.id" } },
            "std.references": { "message": { "cel": "detail.end + ' ' + detail.missingId" } },
            "std.duplicateId": { "message": { "cel": "detail.id + ' x' + string(detail.count)" } }
          }
        }"#,
    );
    let diagram = DislDiagram::new(&specification);
    let a = node(&diagram, "Box", "a", 1, Map::default());
    let b = node(&diagram, "Box", "b", 2, Map::default());
    let lid = node(&diagram, "Lid", "lid", 3, Map::default());
    node(&diagram, "Box", "a@4", 4, Map::default());
    node(&diagram, "Box", "a@5", 5, Map::default());
    relation(&diagram, "loop", Some(&a), Some(&a), 6, None);
    relation(&diagram, "ab1", Some(&a), Some(&b), 7, None);
    relation(&diagram, "ab2", Some(&a), Some(&b), 8, None);
    relation(&diagram, "toLid", Some(&a), Some(&lid), 9, None);
    relation(&diagram, "dangling", None, Some(&b), 10, Some("gone"));

    let findings = constraints::evaluate(&specification, &diagram, &written(&[("a@4", "a"), ("a@5", "a")]));

    assert_eq!(
        vec![
            "4 | std.duplicateId | warning | a x3 | a",
            "5 | std.duplicateId | warning | a x3 | a",
            "6 | std.endpoints | error | selfLoop loop | loop",
            "8 | std.endpoints | error | parallel ab2 | ab2",
            "9 | std.endpoints | error | target toLid | toLid",
            "10 | std.references | error | source gone | dangling",
        ],
        findings.iter().map(line).collect::<Vec<_>>()
    );
}

#[test]
fn once_per_group_and_x_order_report_a_group_once_and_sort_by_code_then_line() {
    let specification = specification(
        r#""constraints": {
          "order": ["dup", "par"],
          "builtIn": {
            "std.endpoints": { "code": "par", "message": "parallel", "oncePerGroup": "second" },
            "std.duplicateId": { "code": "dup", "message": { "cel": "detail.id" }, "oncePerGroup": "last" }
          },
          "rules": [ { "id": "late", "code": "early", "scope": "Box", "rule": "false", "message": "first in no order" } ]
        }"#,
    );
    let diagram = DislDiagram::new(&specification);
    let a = node(&diagram, "Box", "a", 1, Map::default());
    let b = node(&diagram, "Box", "b", 2, Map::default());
    relation(&diagram, "ab1", Some(&a), Some(&b), 3, None);
    relation(&diagram, "ab2", Some(&a), Some(&b), 4, None);
    relation(&diagram, "ab3", Some(&a), Some(&b), 5, None);
    node(&diagram, "Box", "b@6", 6, Map::default());
    node(&diagram, "Box", "b@7", 7, Map::default());

    let findings = constraints::evaluate(&specification, &diagram, &written(&[("b@6", "b"), ("b@7", "b")]));

    assert_eq!(
        vec![
            "7 | dup | warning | b | b",
            "4 | par | error | parallel | ab2",
            "1 | early | error | first in no order | a",
            "2 | early | error | first in no order | b",
            "6 | early | error | first in no order | b",
            "7 | early | error | first in no order | b",
        ],
        findings.iter().map(line).collect::<Vec<_>>()
    );
}

#[test]
fn an_unparseable_file_has_no_other_finding_and_an_unreadable_entry_is_the_readers() {
    let specification = specification(
        r#""constraints": {
          "builtIn": {
            "std.unreadableEntry": { "code": "unreadable", "message": { "cel": "'cannot read: ' + detail.reason" } },
            "std.unparseable": { "severity": "warning", "code": "unparseable" }
          },
          "rules": [ { "id": "never", "scope": "Box", "rule": "false", "message": "no" } ]
        }"#,
    );
    let diagram = DislDiagram::new(&specification);
    node(&diagram, "Box", "a", 3, Map::default());

    let read = constraints::evaluate(
        &specification,
        &diagram,
        &DislConstraintOptions {
            reader_findings: vec![DislReaderFinding::unreadable("odd key", Some(2), None)],
            ..Default::default()
        },
    );
    let unparsed = constraints::evaluate(
        &specification,
        &diagram,
        &DislConstraintOptions {
            reader_findings: vec![DislReaderFinding::not_parsed("tab", Some(1), None)],
            ..Default::default()
        },
    );

    assert_eq!(
        vec![
            "2 | unreadable | warning | cannot read: odd key | ",
            "3 | never | error | no | a"
        ],
        read.iter().map(line).collect::<Vec<_>>()
    );
    assert_eq!(1, unparsed.len());
    assert_eq!(
        "1 | unparseable | warning",
        line(&unparsed[0]).split(" | ").take(3).collect::<Vec<_>>().join(" | ")
    );
}

fn refused(refusals: &[DislFinding]) -> String {
    refusals
        .iter()
        .map(|r| format!("{}: {}", r.constraint_id, r.message))
        .collect::<Vec<_>>()
        .join(" / ")
}

#[test]
fn a_gesture_refusal_is_every_preventing_rule_of_its_kind_that_does_not_hold_in_declaration_order() {
    let specification = specification(
        r#""constraints": {
          "rules": [
            { "id": "named", "kind": "change", "scope": "Box", "rule": "attribute != 'name' || newValue != ''", "message": "A box needs a name." },
            { "id": "short", "kind": "change", "scope": "Box", "enforcement": "prevent", "rule": "attribute != 'name' || size(newValue) < 4", "message": { "cel": "oldValue + ' to ' + newValue + ' is too long.'" } },
            { "id": "reported", "kind": "change", "scope": "Box", "enforcement": "report", "rule": "false", "message": "Not a refusal." },
            { "id": "sized", "kind": "change", "scope": "Box", "when": "has(self.size)", "rule": "attribute != 'size' || newValue > self.size", "message": "Only bigger." },
            { "id": "wide", "kind": "placement", "scope": "Box", "rule": "gesture != 'resize' || newBounds.width >= oldBounds.width", "message": "Only wider." }
          ]
        }"#,
    );
    let diagram = DislDiagram::new(&specification);
    let b = node(&diagram, "Box", "a", 1, attrs([("name", "Ann".into())]));
    let lid = node(&diagram, "Lid", "l", 2, attrs([("name", "Lid".into())]));

    let change =
        |e: &DislElement, attribute: &str, value: Value| gestures::change(&specification, e, attribute, value, None);
    assert_eq!("", refused(&change(&b, "name", "Bo".into())));
    assert_eq!("named: A box needs a name.", refused(&change(&b, "name", "".into())));
    assert_eq!(
        "short: Ann to Bartholomew is too long.",
        refused(&change(&b, "name", "Bartholomew".into()))
    );
    assert_eq!("", refused(&change(&lid, "name", "".into())));
    assert_eq!("", refused(&change(&b, "size", Value::Int(1))));
    let bounds = attrs([("width", Value::Double(10.0))]);
    let narrower = attrs([("width", Value::Double(5.0))]);
    assert_eq!(
        "wide: Only wider.",
        refused(&gestures::placement(
            &specification,
            &b,
            "resize",
            bounds.clone(),
            narrower.clone(),
            None,
            None
        ))
    );
    assert_eq!(
        "",
        refused(&gestures::placement(
            &specification,
            &b,
            "move",
            bounds,
            narrower,
            None,
            None
        ))
    );
}

#[test]
fn a_gesture_rule_that_cannot_be_evaluated_refuses() {
    let specification = specification(
        r#""constraints": { "rules": [ { "id": "sized", "kind": "change", "scope": "Box", "rule": "3 / (newValue - 3) > 0", "message": "Cannot divide." } ] }"#,
    );
    let diagram = DislDiagram::new(&specification);
    let b = node(&diagram, "Box", "a", 1, Map::default());

    let refusals = gestures::change(&specification, &b, "size", Value::Int(3), None);

    assert_eq!(1, refusals.len());
    assert_eq!("Cannot divide.", refusals[0].message);
}

#[test]
fn a_connect_gesture_is_refused_by_the_connect_rules_of_its_relation_type() {
    let specification = specification(
        r#""constraints": {
          "rules": [
            { "id": "noSelf", "kind": "connect", "when": "relationType == 'Link'", "rule": "source.id != target.id", "message": "A box cannot be linked to itself." },
            { "id": "lidsOnly", "kind": "connect", "scope": "Lid", "rule": "false", "message": "Not for links." },
            { "id": "named", "kind": "connect", "scope": "Link", "rule": "target.name != ''", "message": { "cel": "'Link ' + self.name + ' to a named box.'" } }
          ]
        }"#,
    );
    let diagram = DislDiagram::new(&specification);
    let a = node(&diagram, "Box", "a", 1, attrs([("name", "Ann".into())]));
    let b = node(&diagram, "Box", "b", 2, attrs([("name", "".into())]));

    let connect =
        |source: &DislElement, target: &DislElement| gestures::connect(&specification, "Link", source, target, None);
    assert_eq!("noSelf: A box cannot be linked to itself.", refused(&connect(&a, &a)));
    assert_eq!("named: Link Ann to a named box.", refused(&connect(&a, &b)));
    assert_eq!("", refused(&connect(&b, &a)));
}

/// The notation's refusal of a gesture it switches off, for the nearest type that states it (DISL sections 6.9, 8.4).
#[test]
fn the_notation_refuses_a_gesture_for_the_kind_of_element() {
    let specification = specification(
        r#""notation": {
          "nodes": { "Box": { "shape": "rect", "refusals": { "reparent": ["A box stays where it is."] } } }
        }"#,
    );
    let diagram = DislDiagram::new(&specification);
    let b = node(&diagram, "Box", "a", 1, attrs([("name", "Ann".into())]));
    let lid = node(&diagram, "Lid", "l", 2, attrs([("name", "Lid".into())]));
    let refusal = |e: &DislElement, gesture: &str| gestures::notation_refusal(&specification, e, gesture, None);
    assert_eq!(Some("A box stays where it is.".to_owned()), refusal(&b, "reparent"));
    assert_eq!(None, refusal(&b, "move"));
    assert_eq!(None, refusal(&lid, "reparent"));
}

/// A gesture that would store an ephemeral id is refused with its rule's reason (DISL sections 8.4, 11.5.3).
#[test]
fn a_gesture_that_would_store_an_ephemeral_id_is_refused() {
    let specification = specification(
        r#""persistence": {
          "format": "did",
          "ids": {
            "types": {
              "Box": { "ephemeral": "self.name == 'Ann'", "reason": "Its id does not last." },
              "Lid": { "ephemeral": false, "reason": "Never shown." }
            }
          }
        }"#,
    );
    let diagram = DislDiagram::new(&specification);
    let ann = node(&diagram, "Box", "a", 1, attrs([("name", "Ann".into())]));
    let bo = node(&diagram, "Box", "b", 2, attrs([("name", "Bo".into())]));
    let lid = node(&diagram, "Lid", "l", 3, attrs([("name", "Lid".into())]));
    let refusal = |e: &DislElement| gestures::ephemeral(&specification, e, None);
    assert_eq!(Some("Its id does not last.".to_owned()), refusal(&ann));
    assert_eq!(None, refusal(&bo));
    assert_eq!(None, refusal(&lid));
}
