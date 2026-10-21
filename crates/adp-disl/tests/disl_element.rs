//! The element model as DISL's CEL reads it (DISL section 12.2): members, containment, relation ends, type queries
//! (the standalone host's `DislElementTests`).

mod common;

use adp_cel::{Map, Value};
use adp_disl::{DislDiagram, DislElement, DislSpecification, contexts};
use common::{attrs, loaded, strings, vars, with};

struct Fixture {
    specification: DislSpecification,
    diagram: DislDiagram,
    outer: DislElement,
    circle: DislElement,
    strong: DislElement,
    weak: DislElement,
}

fn fixture() -> Fixture {
    let specification = loaded(&with(
        r#""metamodel": {
          "diagram": { "attributes": { "unit": { "type": "string", "default": "month" }, "title": { "type": "string" } } },
          "types": {
            "Shape": { "abstract": true, "labelAttribute": "name", "attributes": {
              "name": { "type": "string" }, "size": { "type": "int", "default": 3 }, "weight": { "type": "number" },
              "when": { "type": "yearMonth" }, "tags": { "type": "string", "many": true }, "buddy": { "type": "Shape" } } },
            "Box": { "extends": "Shape", "children": { "allowed": ["Shape"] } },
            "Circle": { "extends": "Shape" } },
          "relations": {
            "Link": { "abstract": true, "source": "Shape", "target": "Shape" },
            "Strong": { "extends": "Link" },
            "Weak": { "extends": "Link" } } }"#,
    ));
    let diagram = DislDiagram::with_attributes(&specification, attrs([("title", "Shapes".into())])).unwrap();
    let outer = diagram
        .add_node(
            "Box",
            "outer",
            attrs([("name", "Outer".into()), ("when", Value::Int(24320)), ("buddy", "c".into())]),
        )
        .unwrap();
    let inner = diagram
        .add_child("Box", "inner", attrs([("size", Value::Int(7))]), &outer, None)
        .unwrap();
    let circle = diagram
        .add_child(
            "Circle",
            "c",
            attrs([("name", "Round".into()), ("tags", strings(&["a"]))]),
            &inner,
            None,
        )
        .unwrap();
    let twin = diagram.add_child("Circle", "c", Map::default(), &outer, None).unwrap();
    let strong = diagram
        .add_relation("Strong", "s", Some(&outer), Some(&circle), Map::default())
        .unwrap();
    let weak = diagram
        .add_relation("Weak", "w", Some(&twin), Some(&circle), Map::default())
        .unwrap();
    Fixture {
        specification,
        diagram,
        outer,
        circle,
        strong,
        weak,
    }
}

impl Fixture {
    fn evaluate(&self, expression: &str) -> Result<Value, String> {
        self.specification
            .environment(contexts::ELEMENT, &[])
            .compile(expression)
            .unwrap()
            .evaluate(&vars([
                ("self", self.outer.to_value()),
                ("diagram", self.diagram.to_value()),
                ("env", Value::map(Map::default())),
            ]))
    }
}

#[test]
fn the_members_read_the_element() {
    let f = fixture();
    for (expression, expected) in [
        ("self.id", "outer"),
        ("self.type", "Box"),
        ("self.kind", "node"),
        ("self.name", "Outer"),
        ("self.label()", "Outer"),
        ("diagram.title", "Shapes"),
        ("diagram.unit", "month"),
        ("diagram.kind", "diagram"),
    ] {
        assert_eq!(Ok(Value::from(expected)), f.evaluate(expression), "{expression}");
    }
}

#[test]
fn an_unset_attribute_reads_as_its_default_or_its_zero_value() {
    let f = fixture();
    for (expression, expected) in [
        ("self.size", 3),
        ("self.when", 24320),
        ("self.when.month()", 9),
        ("self.children[0].size", 7),
        ("self.children[0].when", 0),
        ("self.weight == 0.0 ? 1 : 0", 1),
        ("size(self.tags)", 0),
        ("size(self.children[0].children[0].tags)", 1),
    ] {
        assert!(matches!(f.evaluate(expression), Ok(Value::Int(v)) if v == expected), "{expression}");
    }
}

#[test]
fn has_holds_for_a_stored_value_only() {
    let f = fixture();
    for (expression, expected) in [
        ("has(self.name)", true),
        ("has(self.size)", false),
        ("has(self.children[0].size)", true),
        ("has(diagram.title)", true),
        ("has(diagram.unit)", false),
        ("has(self.parent)", false),
        ("has(self.children[0].parent)", true),
    ] {
        assert_eq!(Ok(Value::Bool(expected)), f.evaluate(expression), "{expression}");
    }
}

fn all_true(f: &Fixture, expressions: &[(&str, bool)]) {
    for (expression, expected) in expressions {
        assert_eq!(Ok(Value::Bool(*expected)), f.evaluate(expression), "{expression}");
    }
}

#[test]
fn is_a_includes_subtypes() {
    all_true(
        &fixture(),
        &[
            ("self.isA('Box')", true),
            ("self.isA('Shape')", true),
            ("self.isA('Circle')", false),
            ("diagram.relations[0].isA('Link')", true),
            ("diagram.relations[0].isA('Weak')", false),
        ],
    );
}

#[test]
fn containment_is_read_parents_first() {
    all_true(
        &fixture(),
        &[
            ("self.parent == null", true),
            ("self.children.map(c, c.id) == ['inner', 'c']", true),
            ("self.descendants().map(c, c.id) == ['inner', 'c', 'c']", true),
            ("self.descendants()[1].ancestors().map(c, c.id) == ['inner', 'outer']", true),
            ("self.childrenOfType('Circle').size() == 1", true),
            ("self.children[0].parent == self", true),
            ("self.owner == diagram", true),
        ],
    );
}

#[test]
fn relations_are_read_from_their_ends() {
    all_true(
        &fixture(),
        &[
            ("self.outgoing.map(r, r.id) == ['s']", true),
            ("self.incoming.size() == 0", true),
            ("self.descendants()[1].incoming.map(r, r.id) == ['s', 'w']", true),
            ("self.descendants()[1].incomingOf('Link').size() == 2", true),
            ("self.descendants()[1].incomingOf('Weak').map(r, r.source.id) == ['c']", true),
            ("self.outgoingOf('Weak').size() == 0", true),
            ("diagram.relations[0].source == self", true),
            ("diagram.relations[0].target.name == 'Round'", true),
            ("diagram.relations[0].other(self).name == 'Round'", true),
            ("diagram.relations[0].kind == 'relation'", true),
        ],
    );
}

#[test]
fn position_in_compares_by_identity_not_by_id() {
    let f = fixture();
    for (expression, expected) in [
        ("self.descendants()[1].positionIn(self.descendants())", 1),
        ("self.descendants()[2].positionIn(self.descendants())", 2),
        ("self.positionIn(self.children)", -1),
        ("diagram.nodes.filter(n, n.id == 'c').size()", 2),
    ] {
        assert!(matches!(f.evaluate(expression), Ok(Value::Int(v)) if v == expected), "{expression}");
    }
}

#[test]
fn the_diagram_lists_its_elements_in_model_order() {
    all_true(
        &fixture(),
        &[
            ("diagram.nodes.map(n, n.id) == ['outer', 'inner', 'c', 'c']", true),
            ("diagram.nodesOfType('Shape').size() == 4", true),
            ("diagram.nodesOfType('Circle').size() == 2", true),
            ("diagram.nodesOfType('Circle', true).size() == 2", true),
            ("diagram.relationsOfType('Link').map(r, r.id) == ['s', 'w']", true),
            ("diagram.elementById('c').name == 'Round'", true),
            ("diagram.elementById('none') == null", true),
            ("diagram.elements.size() == 6", true),
        ],
    );
}

#[test]
fn a_reference_reads_as_the_element_it_names() {
    let f = fixture();
    assert_eq!(Some(f.circle.clone()), DislElement::from_value(&f.evaluate("self.buddy").unwrap()));
}

#[test]
fn an_element_is_not_its_twin() {
    assert_eq!(Ok(Value::Bool(false)), fixture().evaluate("diagram.nodes[2] == diagram.nodes[3]"));
}

#[test]
fn a_member_a_node_lacks_is_an_error() {
    assert_eq!(Err("No such field: 'source'.".to_owned()), fixture().evaluate("self.source"));
}

#[test]
fn a_relation_method_on_a_node_is_an_error() {
    assert_eq!(
        Err("'other()' is not a method of this value.".to_owned()),
        fixture().evaluate("self.other(self)")
    );
}

#[test]
fn an_abstract_type_or_an_undeclared_attribute_cannot_be_added() {
    let f = fixture();
    assert!(f.diagram.add_node("Shape", "x", Map::default()).is_err());
    assert!(f.diagram.add_node("Box", "x", attrs([("colour", "red".into())])).is_err());
    let inner = f.diagram.element_by_id("inner").unwrap();
    assert!(f.diagram.add_relation("Box", "x", Some(&f.outer), Some(&inner), Map::default()).is_err());
    assert_eq!(Some(f.weak.clone()), f.diagram.element_by_id("w"));
    assert_eq!(f.strong, f.diagram.relations()[0]);
}
