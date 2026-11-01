//! The model a reading builds (DISL sections 4.11, 11.5 and 14.2): binding types mapped by the type map, values
//! typed by their attributes, the derived ids and the derived relations computed (the standalone host's
//! `DislModelBuilderTests`).

mod common;

use std::rc::Rc;
use std::sync::LazyLock;

use adp_cel::{Map, Value};
use adp_disl::model::builder::{self, DERIVED_FAILED, DERIVED_ID};
use adp_disl::{DislDiagram, DislSpecification};
use adp_fbl::model::codes;
use adp_fbl::plugins::FolderBody;
use adp_fbl::plugins::{
    PersistencePlugin, PluginPlanRequest, PluginPlanResult, PluginReadRequest, PluginReadResult, PluginTemplateRequest,
};
use adp_fbl::text::Span;
use adp_fbl::{FblElement, FblModel, FblOptions, Finding, FindingSeverity};
use common::{attrs, loaded, with};

static MAPPED: LazyLock<String> = LazyLock::new(|| {
    with(
        r#""metamodel": {
          "diagram": { "attributes": { "scale": { "type": "Scale", "default": "small" } } },
          "enums": { "Scale": { "values": { "small": { "value": "S" }, "large": { "value": "L" } } } },
          "types": {
            "Box": { "attributes": {
              "name": { "type": "string" }, "count": { "type": "int", "default": 5 }, "weight": { "type": "number" },
              "open": { "type": "bool" }, "when": { "type": "yearMonth" }, "size": { "type": "Scale" },
              "tags": { "type": "int", "many": true } },
              "children": { "allowed": ["Box"] } } },
          "relations": { "Arrow": { "source": "Box", "target": "Box", "attributes": { "label": { "type": "string" } } } } },
        "persistence": { "typeMap": {
          "Head": { "as": "header" },
          "Setting": { "as": "diagram", "attributes": { "value": "scale" } },
          "Item": { "as": "Box", "attributes": { "title": "name" }, "hostAttributes": ["raw"] },
          "Link": { "as": "Arrow", "attributes": { "from": "source", "to": "target" } },
          "Junk": { "as": "unreadable" } } }"#,
    )
});

fn mapped() -> DislSpecification {
    loaded(&MAPPED)
}

/// An FBL element as a reading has it.
#[allow(clippy::too_many_arguments)]
pub fn fbl(
    id: &str,
    type_name: &str,
    attributes: Map,
    parent: Option<&str>,
    relation: bool,
    source: Option<&str>,
    target: Option<&str>,
    stored: bool,
) -> FblElement {
    FblElement {
        id: id.to_owned(),
        id_is_stored: stored,
        type_name: type_name.to_owned(),
        rule: type_name.to_lowercase(),
        is_relation: relation,
        attributes,
        parent_id: parent.map(str::to_owned),
        parent_slot: parent.map(|_| "children".to_owned()),
        source: source.map(str::to_owned),
        target: target.map(str::to_owned),
        referenced: Vec::new(),
        own_span: Span::new(0, 0),
        line: 1,
        file: None,
    }
}

fn element(id: &str, type_name: &str, attributes: Map) -> FblElement {
    fbl(id, type_name, attributes, None, false, None, None, true)
}

fn reading(elements: Vec<FblElement>, findings: Vec<Finding>, unreadable: bool) -> FblModel {
    FblModel {
        elements,
        findings,
        unreadable,
        ..Default::default()
    }
}

fn warning(code: &str, message: &str) -> Finding {
    Finding {
        code: code.to_owned(),
        severity: FindingSeverity::Warning,
        message: message.to_owned(),
        location: None,
        detail: None,
    }
}

fn ids(elements: &[adp_disl::DislElement]) -> Vec<String> {
    elements.iter().map(|e| e.id()).collect()
}

#[test]
fn the_type_map_places_every_binding_type() {
    // Arrange.
    let reading = reading(
        vec![
            fbl(
                "head@1",
                "Head",
                attrs([("version", Value::Int(1))]),
                None,
                false,
                None,
                None,
                false,
            ),
            fbl(
                "setting@2",
                "Setting",
                attrs([("value", "L".into())]),
                None,
                false,
                None,
                None,
                false,
            ),
            element(
                "a",
                "Item",
                attrs([("title", "A".into()), ("raw", "kept".into()), ("colour", "red".into())]),
            ),
            fbl(
                "b",
                "Item",
                attrs([("title", "B".into())]),
                Some("a"),
                false,
                None,
                None,
                true,
            ),
            element(
                "l1",
                "Link",
                attrs([("from", "a".into()), ("to", "b".into()), ("label", "go".into())]),
            ),
            element("l2", "Link", attrs([("from", "a".into()), ("to", "nowhere".into())])),
            fbl("l3", "Arrow", Map::default(), None, true, Some("b"), Some("a"), true),
            fbl("junk@9", "Junk", Map::default(), None, false, None, None, false),
            element("x", "Unknown", Map::default()),
        ],
        vec![warning(codes::DUPLICATE_ID, "From the reader.")],
        false,
    );

    // Act.
    let model = builder::from(&reading, &mapped());

    // Assert.
    let diagram = &model.diagram;
    assert_eq!(
        vec!["head@1"],
        model.header.iter().map(|e| e.id.as_str()).collect::<Vec<_>>()
    );
    assert_eq!(
        vec!["junk@9", "x"],
        model.unreadable.iter().map(|e| e.id.as_str()).collect::<Vec<_>>()
    );
    assert_eq!(Value::from("large"), diagram.value_of("scale"));
    let nodes = diagram.nodes();
    assert_eq!(vec!["a", "b"], ids(&nodes));
    assert_eq!(Value::from("A"), nodes[0].value_of("name"));
    assert_eq!(
        attrs([("raw", "kept".into()), ("colour", "red".into())]),
        nodes[0].host_attributes()
    );
    assert_eq!(Some(nodes[0].clone()), nodes[1].parent());
    assert_eq!(Some("children".to_owned()), nodes[1].slot());
    let relations = diagram.relations();
    assert_eq!(vec!["l1", "l2", "l3"], ids(&relations));
    assert_eq!(
        (Some("a".to_owned()), Some("b".to_owned()), Value::from("go")),
        (
            relations[0].source().map(|s| s.id()),
            relations[0].target().map(|t| t.id()),
            relations[0].value_of("label")
        )
    );
    assert_eq!(
        (Some("a".to_owned()), Some("nowhere".to_owned())),
        (relations[1].source_id(), relations[1].target_id())
    );
    assert!(relations[1].target().is_none());
    assert_eq!(
        (Some("b".to_owned()), Some("a".to_owned())),
        (
            relations[2].source().map(|s| s.id()),
            relations[2].target().map(|t| t.id())
        )
    );
    assert_eq!(
        vec!["std.duplicateId"],
        model.findings.iter().map(|f| f.code.as_str()).collect::<Vec<_>>()
    );
    assert!(!model.is_unreadable);
}

#[test]
fn a_child_read_before_its_parent_is_added_after_it() {
    let reading = reading(
        vec![
            fbl("b", "Item", Map::default(), Some("a"), false, None, None, true),
            element("a", "Item", Map::default()),
        ],
        Vec::new(),
        false,
    );

    let diagram = builder::from(&reading, &mapped()).diagram;

    let nodes = diagram.nodes();
    assert_eq!(vec!["a", "b"], ids(&nodes));
    assert_eq!(Some(nodes[0].clone()), nodes[1].parent());
}

#[test]
fn an_unreadable_body_gives_an_empty_model() {
    let model = builder::from(
        &reading(Vec::new(), vec![warning(codes::UNPARSEABLE, "Not YAML.")], true),
        &mapped(),
    );

    assert!(model.is_unreadable);
    assert!(model.diagram.elements().is_empty());
    assert_eq!(
        vec!["std.unparseable"],
        model.findings.iter().map(|f| f.code.as_str()).collect::<Vec<_>>()
    );
}

/// A folder plugin that reads each file as one `Item` in that file, and links the first to every other from the
/// other's file.
struct FilePerItemPlugin;

impl FilePerItemPlugin {
    fn element(id: &str, type_name: &str, file: &str, from: Option<&str>, to: Option<&str>) -> FblElement {
        let attributes = match (from, to) {
            (Some(from), Some(to)) => attrs([("from", from.into()), ("to", to.into())]),
            _ => Map::default(),
        };
        let mut element = fbl(id, type_name, attributes, None, false, None, None, true);
        element.file = Some(file.to_owned());
        element
    }
}

impl PersistencePlugin for FilePerItemPlugin {
    fn id(&self) -> &str {
        "net.example.files"
    }

    fn read(&self, request: &PluginReadRequest) -> PluginReadResult {
        let files = &request.files;
        let mut elements: Vec<FblElement> = files
            .iter()
            .map(|f| Self::element(&f.relative_path, "Item", &f.relative_path, None, None))
            .collect();
        for file in files.iter().skip(1) {
            elements.push(Self::element(
                &format!("{}|{}", files[0].relative_path, file.relative_path),
                "Link",
                &file.relative_path,
                Some(&files[0].relative_path),
                Some(&file.relative_path),
            ));
        }
        PluginReadResult::new(elements, Vec::new(), false)
    }

    fn plan(&self, _request: &PluginPlanRequest<'_>) -> PluginPlanResult {
        PluginPlanResult::Refused("Nothing is planned.".into())
    }

    fn template(&self, _request: &PluginTemplateRequest) -> Vec<u8> {
        Vec::new()
    }
}

#[test]
fn an_element_of_a_folder_subject_keeps_the_file_it_was_read_in() {
    // Arrange: a folder on disk read by a folder body, whose plugin says which file each element is in.
    let folder = std::env::temp_dir().join(format!("disl-{}", std::process::id()));
    std::fs::create_dir_all(folder.join("pages")).unwrap();
    std::fs::write(folder.join("index.txt"), "index").unwrap();
    std::fs::write(folder.join("pages").join("a.txt"), "a").unwrap();
    let (problems, document) = adp_fbl::documents::load(
        br#"{ "fbl": "0.1", "bindings": { "notes": {
          "claims": { "names": ["index.txt"] },
          "body": { "kind": "folder", "recognise": { "all": ["index.txt"] },
            "files": [ { "name": "all", "glob": "**", "family": "lines" } ] },
          "reader": { "plugin": "net.example.files" } } } }"#,
    );
    assert!(problems.is_empty(), "{problems:?}");
    let binding = Rc::new(document.unwrap().binding("notes").unwrap().clone());
    let body = FolderBody::open(
        &folder,
        binding,
        Some(Rc::new(FilePerItemPlugin)),
        Rc::new(FblOptions::default()),
        None,
    );

    // Act.
    let diagram = body.map(|b| builder::from(&b.model(), &mapped()).diagram);
    std::fs::remove_dir_all(&folder).unwrap();

    // Assert.
    let diagram = diagram.unwrap();
    assert_eq!(
        vec![
            ("index.txt".to_owned(), Some("index.txt".to_owned())),
            ("pages/a.txt".to_owned(), Some("pages/a.txt".to_owned()))
        ],
        diagram.nodes().iter().map(|n| (n.id(), n.file())).collect::<Vec<_>>()
    );
    let relations = diagram.relations();
    assert_eq!(1, relations.len());
    assert_eq!(
        ("index.txt|pages/a.txt".to_owned(), Some("pages/a.txt".to_owned())),
        (relations[0].id(), relations[0].file())
    );
}

/// Each read value against what the attribute stores: the value in its CEL type, or nothing when it does not fit.
#[test]
fn a_read_value_is_typed_by_its_attribute() {
    let cases: Vec<(&str, Value, Option<Value>)> = vec![
        ("count", "7".into(), Some(Value::Int(7))),
        ("count", Value::Int(7), Some(Value::Int(7))),
        ("count", "seven".into(), None),
        ("count", "7.5".into(), None),
        ("weight", "2.5".into(), Some(Value::Double(2.5))),
        ("weight", Value::Int(3), Some(Value::Double(3.0))),
        ("weight", "heavy".into(), None),
        ("open", Value::Bool(true), Some(Value::Bool(true))),
        ("open", "true".into(), Some(Value::Bool(true))),
        ("open", "yes".into(), None),
        ("when", "2026-09".into(), Some(Value::Int(2026 * 12 + 8))),
        ("when", "-3200-01".into(), Some(Value::Int(-3200 * 12))),
        ("when", "2026-00".into(), None),
        ("when", "2026-13".into(), None),
        ("when", "September".into(), None),
        ("size", "S".into(), Some("small".into())),
        ("size", "huge".into(), Some("huge".into())),
        ("name", Value::Int(42), Some("42".into())),
        ("name", Value::Double(2.5), Some("2.5".into())),
        ("name", Value::Bool(false), Some("false".into())),
        ("name", Value::Null, Some("".into())),
    ];
    for (attribute, read, expected) in cases {
        let reading = reading(
            vec![element("a", "Item", attrs([(attribute, read.clone())]))],
            Vec::new(),
            false,
        );

        let nodes = builder::from(&reading, &mapped()).diagram.nodes();

        assert_eq!(1, nodes.len());
        let stored = nodes[0].attributes().get(attribute).cloned();
        assert_eq!(
            expected.as_ref().map(Value::type_name),
            stored.as_ref().map(Value::type_name),
            "{attribute} {read}"
        );
        assert_eq!(expected, stored, "{attribute} {read}");
    }
}

#[test]
fn a_many_value_keeps_the_items_that_fit() {
    let reading = reading(
        vec![
            element(
                "a",
                "Item",
                attrs([(
                    "tags",
                    Value::list(vec![Value::Int(1), "2".into(), "three".into(), Value::Int(4)]),
                )]),
            ),
            element("b", "Item", attrs([("tags", "1".into())])),
        ],
        Vec::new(),
        false,
    );

    let nodes = builder::from(&reading, &mapped()).diagram.nodes();

    assert_eq!(
        Some(&Value::list(vec![Value::Int(1), Value::Int(2), Value::Int(4)])),
        nodes[0].attributes().get("tags")
    );
    assert!(!nodes[1].attributes().contains_key("tags"));
}

// ---- derived ids and derived relations ---------------------------------------------------------

static DERIVED: LazyLock<String> = LazyLock::new(|| {
    with(
        r#""metamodel": {
          "types": {
            "Thing": { "attributes": { "name": { "type": "string" } }, "children": { "allowed": ["Thing"] } },
            "Other": { "attributes": { "name": { "type": "string" } } } },
          "relations": {
            "Next": { "source": "Thing", "target": "Thing",
              "derived": { "from": "diagram.nodesOfType('Thing').filter(n, n.name != 'last') + [diagram.nodes[0]]", "source": "item",
                           "target": "diagram.nodes.filter(n, n.name == 'last')[0]" } },
            "Up": { "source": "Thing", "target": "Thing",
              "derived": { "from": "diagram.nodes", "id": "'up:' + item.id", "source": "item", "target": "item.parent" } } } },
        "persistence": { "ids": { "strategy": "derived", "expression": "self.name == 'fail' ? self.missing : self.name" } }"#,
    )
});

#[test]
fn the_derived_ids_are_computed_and_checked() {
    let diagram = DislDiagram::new(&loaded(&DERIVED));
    diagram.add_node("Thing", "", attrs([("name", "a".into())])).unwrap();
    diagram.add_node("Thing", "", attrs([("name", "a".into())])).unwrap();
    diagram
        .add_node("Thing", "stored", attrs([("name", "fail".into())]))
        .unwrap();
    diagram.add_node("Thing", "", attrs([("name", "last".into())])).unwrap();

    let findings = builder::complete(&diagram);

    assert_eq!(vec!["a", "a", "stored", "last"], ids(&diagram.nodes()));
    assert_eq!(
        vec![
            "std.duplicateId: The id 'a' is computed for two elements; the first keeps it.",
            "std.missingId: The id of this Thing could not be computed: No such field: 'missing'."
        ],
        findings.iter().take(2).map(ToString::to_string).collect::<Vec<_>>()
    );
}

#[test]
fn a_derived_id_computed_twice_is_not_reported_when_the_definition_switches_std_duplicate_id_off() {
    let specification = loaded(&with(
        r#""metamodel": { "types": { "Thing": { "attributes": { "name": { "type": "string" } } } } },
        "persistence": { "ids": { "strategy": "derived", "expression": "self.name == 'fail' ? self.missing : self.name" } },
        "constraints": { "builtIn": { "std.duplicateId": { "enabled": false } } }"#,
    ));
    let diagram = DislDiagram::new(&specification);
    diagram.add_node("Thing", "", attrs([("name", "a".into())])).unwrap();
    diagram.add_node("Thing", "", attrs([("name", "a".into())])).unwrap();
    diagram
        .add_node("Thing", "stored", attrs([("name", "fail".into())]))
        .unwrap();

    let findings = builder::complete(&diagram);

    assert_eq!(vec!["a", "a", "stored"], ids(&diagram.nodes()));
    assert_eq!(
        vec!["std.missingId: The id of this Thing could not be computed: No such field: 'missing'."],
        findings.iter().map(ToString::to_string).collect::<Vec<_>>()
    );
}

#[test]
fn a_derived_relation_has_its_default_id_with_repeats_numbered_and_drops_what_its_ends_do_not_allow() {
    let diagram = DislDiagram::new(&loaded(&DERIVED));
    let a = diagram.add_node("Thing", "", attrs([("name", "a".into())])).unwrap();
    diagram
        .add_child("Thing", "", attrs([("name", "b".into())]), &a, None)
        .unwrap();
    diagram.add_node("Thing", "", attrs([("name", "last".into())])).unwrap();
    diagram.add_node("Other", "", attrs([("name", "o".into())])).unwrap();

    let findings = builder::complete(&diagram);

    let relations = diagram.relations();
    assert_eq!(
        vec!["Next:a->last", "Next:b->last", "Next:a->last#2", "up:b"],
        ids(&relations)
    );
    assert_eq!(Some(a), relations[3].target());
    let mut lines: Vec<String> = findings.iter().map(ToString::to_string).collect();
    lines.sort();
    assert_eq!(
        vec![
            "std.derivedEnds: 1 item(s) of Up were dropped: their source is not one its declaration allows.",
            "std.derivedEnds: 2 item(s) of Up were dropped: their target is not one its declaration allows."
        ],
        lines
    );
    assert!(
        relations
            .iter()
            .all(|r| r.is_derived() && r.sources().len() == 1 && !r.id_is_stored())
    );
}

#[test]
fn a_derived_id_taken_already_is_not_drawn() {
    let specification = loaded(&with(
        r#""metamodel": {
          "types": { "Thing": { "attributes": { "name": { "type": "string" } } } },
          "relations": { "Self": { "source": "Thing", "target": "Thing",
            "derived": { "from": "diagram.nodes", "id": "'t'", "source": "item", "target": "item" } } } }"#,
    ));
    let diagram = DislDiagram::new(&specification);
    diagram.add_node("Thing", "t", Map::default()).unwrap();
    diagram.add_node("Thing", "u", Map::default()).unwrap();

    let findings = builder::complete(&diagram);

    assert!(diagram.relations().is_empty());
    assert!(findings.iter().all(|f| f.code == DERIVED_ID));
    assert_eq!(2, findings.len());
}

#[test]
fn a_derived_relation_takes_the_attributes_its_declaration_computes() {
    // DISL 4.11.3: each member of a DerivedRelation's attributes is an Expression in the deriveItem context,
    // evaluated per item; a literal written as CEL is an Expression too.
    let specification = loaded(&with(
        r#""metamodel": {
          "types": { "Thing": { "attributes": { "name": { "type": "string" } } } },
          "relations": { "Named": { "source": "Thing", "target": "Thing",
            "attributes": { "label": { "type": "string" }, "fixed": { "type": "string" } },
            "derived": { "from": "diagram.nodes", "source": "item", "target": "item",
                         "attributes": { "label": "item.name + '!'", "fixed": "''" } } } } }"#,
    ));
    let diagram = DislDiagram::new(&specification);
    diagram.add_node("Thing", "t", attrs([("name", "a".into())])).unwrap();

    assert!(builder::complete(&diagram).is_empty());

    let relations = diagram.relations();
    assert_eq!(1, relations.len());
    assert_eq!(Value::from("a!"), relations[0].value_of("label"));
    assert_eq!(Value::from(""), relations[0].value_of("fixed"));
}

#[test]
fn a_failing_from_derives_nothing() {
    let specification = loaded(&with(
        r#""metamodel": {
          "types": { "Thing": { "attributes": { "name": { "type": "string" } } } },
          "relations": { "Self": { "source": "Thing", "target": "Thing",
            "derived": { "from": "diagram.nodes.map(n, 1 / 0)", "source": "item", "target": "item" } } } }"#,
    ));
    let diagram = DislDiagram::new(&specification);
    diagram.add_node("Thing", "t", Map::default()).unwrap();

    let findings = builder::complete(&diagram);

    assert_eq!(1, findings.len());
    assert_eq!(DERIVED_FAILED, findings[0].code);
    assert!(diagram.relations().is_empty());
}

/// Items with equal keys yield one element, the first of them its `item`, all of them its `group` (section 4.11.3).
#[test]
fn a_derived_relation_with_a_key_is_one_element_per_key() {
    let specification = loaded(&with(
        r#""metamodel": { "types": { "Thing": { "attributes": { "name": { "type": "string" } } } },
            "relations": { "Pair": { "from": "Thing", "to": "Thing", "attributes": { "count": { "type": "int" } },
              "derived": { "from": "diagram.nodes", "key": "item.name.substring(0, 1)", "id": "'pair:' + item.name",
                           "source": "item", "target": "item", "attributes": { "count": "group.size()" } } } } }"#,
    ));
    let diagram = DislDiagram::new(&specification);
    for name in ["a1", "b1", "a2"] {
        diagram.add_node("Thing", name, attrs([("name", name.into())])).unwrap();
    }

    let findings = builder::complete(&diagram);

    assert!(findings.is_empty(), "{findings:?}");
    assert_eq!(vec!["pair:a1", "pair:b1"], ids(&diagram.relations()));
    assert_eq!(Value::Int(2), diagram.relations()[0].value_of("count"));
}

/// A relation's end may name a derived node (section 4.9, DISL 0.8): the placeholder for what a relation names and
/// nothing declares, here under its type's prefix.
fn placeholders() -> DislSpecification {
    loaded(&with(
        r#""metamodel": {
          "types": {
            "Task": { "attributes": { "key": { "type": "string" } } },
            "Placeholder": { "extends": "Task",
              "derived": {
                "from": "diagram.relationsOfType('Needs').filter(d, !has(d.source))",
                "key": "item.written",
                "id": "'task:' + item.written",
                "attributes": { "key": "item.written" } } },
            "Other": {
              "derived": { "from": "['elsewhere']", "id": "item" } } },
          "relations": {
            "Needs": { "source": "Task", "target": { "types": ["Task"], "optional": true },
                       "attributes": { "written": { "type": "string" } } },
            "Points": { "source": "Task", "target": "Task" } } },
        "persistence": { "ids": { "strategy": "natural", "prefix": { "Task": "task:" } },
                         "typeMap": { "Task": { "as": "Task" },
                                      "Needs": { "as": "Needs", "attributes": { "from": "source", "to": "target" } },
                                      "Points": { "as": "Points", "attributes": { "from": "source", "to": "target" } } } }"#,
    ))
}

fn needs(id: &str, from: &str, to: &str) -> FblElement {
    fbl(
        id,
        "Needs",
        attrs([
            ("from", Value::from(from)),
            ("to", Value::from(to)),
            ("written", Value::from(from)),
        ]),
        None,
        true,
        None,
        None,
        true,
    )
}

#[test]
fn an_end_naming_a_derived_node_resolves_to_it_and_stays_as_written() {
    let waiter = fbl("waiter", "Task", attrs([]), None, false, None, None, true);
    let model = builder::from(
        &reading(vec![waiter, needs("n", "ghost", "waiter")], vec![], false),
        &placeholders(),
    );
    let relation = model.diagram.relations().remove(0);
    let source = relation.source().expect("the end names the placeholder");
    assert_eq!(source.id(), "task:ghost");
    assert!(source.is_derived());
    assert_eq!(relation.source_id().as_deref(), Some("ghost"), "as written");
    assert_eq!(ids(&source.outgoing()), vec!["n"]);
    assert_eq!(model.findings, vec![]);
}

#[test]
fn an_end_naming_a_derived_node_its_type_does_not_allow_stays_unset() {
    let waiter = fbl("waiter", "Task", attrs([]), None, false, None, None, true);
    let pointer = fbl(
        "p",
        "Points",
        attrs([("from", Value::from("elsewhere")), ("to", Value::from("waiter"))]),
        None,
        true,
        None,
        None,
        true,
    );
    let model = builder::from(&reading(vec![waiter, pointer], vec![], false), &placeholders());
    let relation = model.diagram.relations().remove(0);
    assert_eq!(
        relation.source(),
        None,
        "`elsewhere` is an `Other`, which the end does not allow"
    );
    assert_eq!(relation.source_id().as_deref(), Some("elsewhere"));
    assert_eq!(relation.target().map(|t| t.id()).as_deref(), Some("task:waiter"));
}

#[test]
fn an_end_naming_nothing_stored_or_derived_stays_unset() {
    let waiter = fbl("waiter", "Task", attrs([]), None, false, None, None, true);
    let model = builder::from(
        &reading(vec![waiter, needs("n", "waiter", "nowhere")], vec![], false),
        &placeholders(),
    );
    let relation = model.diagram.relations().remove(0);
    assert_eq!(relation.target(), None);
    assert_eq!(relation.target_id().as_deref(), Some("nowhere"));
}
