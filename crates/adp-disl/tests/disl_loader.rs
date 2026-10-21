//! The steps of DISL section 14.1 the loader takes: parse, version, names, inheritance, and compiling in context
//! (the standalone host's `DislLoaderTests`).

mod common;

use adp_disl::{DislLoader, DislPluginFunctions, contexts};
use common::{diagnostics, loaded, with};

#[test]
fn a_minimal_specification_loads() {
    let result = DislLoader::load(&with(""));

    assert!(result.diagnostics.is_empty());
    assert_eq!("org.example.test", result.specification.unwrap().language_id());
}

// Step 1.

#[test]
fn a_duplicate_key_is_refused_where_it_repeats() {
    let json =
        r#"{ "disl": "0.2", "language": { "id": "a", "version": "1.0.0", "id": "b" }, "metamodel": { "types": {} } }"#;

    assert_eq!(
        vec!["error at /language/id: The key 'id' appears more than once in one object (DISL §2.1).".to_owned()],
        diagnostics(json)
    );
}

#[test]
fn a_byte_order_mark_is_refused() {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend(with("").as_bytes());

    let result = DislLoader::load_bytes(&bytes, &DislPluginFunctions::none());

    assert!(result.specification.is_none());
    assert_eq!(1, result.diagnostics.len());
    assert_eq!(
        "A specification is UTF-8 without a byte-order mark (DISL §2.1).",
        result.diagnostics[0].message
    );
}

#[test]
fn text_that_is_not_json_is_refused() {
    let result = DislLoader::load("{ \"disl\": ");

    assert!(result.specification.is_none());
    assert_eq!(1, result.diagnostics.len());
    assert!(
        result.diagnostics[0]
            .message
            .starts_with("The specification is not JSON:")
    );
}

// Step 2.

#[test]
fn a_higher_major_version_is_refused() {
    assert_eq!(
        vec!["error at /disl: This runtime reads DISL 0.x; DISL 1.0 is refused (DISL §2.9).".to_owned()],
        diagnostics(&with(r#""disl": "1.0""#))
    );
}

#[test]
fn a_higher_minor_version_is_warned_about_and_loaded() {
    let result = DislLoader::load(&with(r#""disl": "0.4""#));

    assert!(result.specification.is_some());
    assert_eq!(1, result.diagnostics.len());
    assert_eq!(
        "warning at /disl: This runtime knows DISL up to 0.3; what DISL 0.4 adds is not read (DISL §2.9).",
        result.diagnostics[0].to_string()
    );
}

#[test]
fn the_deprecated_dedl_key_is_read_and_warned_about() {
    let json = r#"{ "dedl": "0.1", "language": { "id": "a", "version": "1.0.0" }, "metamodel": { "types": {} } }"#;

    let result = DislLoader::load(json);

    assert_eq!("0.1", result.specification.as_ref().unwrap().disl());
    assert_eq!(
        vec!["warning at /dedl: 'dedl' is the deprecated alias of 'disl' (DISL §18).".to_owned()],
        result.diagnostics.iter().map(ToString::to_string).collect::<Vec<_>>()
    );
}

#[test]
fn a_missing_version_is_refused() {
    let json = r#"{ "language": { "id": "a", "version": "1.0.0" }, "metamodel": { "types": {} } }"#;

    assert_eq!(
        vec!["error at /disl: A specification declares the DISL version it targets (DISL §2.9).".to_owned()],
        diagnostics(json)
    );
}

// Steps 5 and 6.

#[test]
fn a_supertype_that_is_not_declared_is_refused() {
    let json = with(r#""metamodel": { "types": { "Thing": { "extends": "Nothing" } } }"#);

    assert_eq!(
        vec!["error at /metamodel/types/Thing/extends: 'Thing' extends 'Nothing', which is not a node type of this specification (DISL §4.7).".to_owned()],
        diagnostics(&json)
    );
}

#[test]
fn a_cycle_of_supertypes_is_refused() {
    let json = with(r#""metamodel": { "types": { "A": { "extends": "B" }, "B": { "extends": "A" } } }"#);

    assert!(diagnostics(&json).join("\n").contains("is its own supertype"));
    assert!(DislLoader::load(&json).specification.is_none());
}

#[test]
fn a_diamond_is_linearised_by_c3() {
    let json = with(
        r#""metamodel": { "types": {
          "O": { "abstract": true, "attributes": { "o": { "type": "string" } } },
          "A": { "abstract": true, "extends": "O", "attributes": { "a": { "type": "int" } } },
          "B": { "abstract": true, "extends": "O", "attributes": { "b": { "type": "bool" } } },
          "D": { "extends": ["A", "B"], "attributes": { "d": { "type": "number" } } } } }"#,
    );

    let specification = loaded(&json);
    let metamodel = specification.metamodel();

    assert_eq!(vec!["D", "A", "B", "O"], metamodel.types["D"].linearisation);
    assert_eq!(
        vec!["o", "b", "a", "d"],
        metamodel.types["D"].attributes.keys().collect::<Vec<_>>()
    );
    assert!(metamodel.is_a("D", "O"));
    assert!(!metamodel.is_a("A", "B"));
    assert_eq!(
        vec!["O", "A", "B", "D"],
        metamodel
            .subtypes_of("O")
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>()
    );
}

#[test]
fn an_attribute_inherited_differently_from_two_supertypes_is_refused_unless_redeclared() {
    let types = r#""A": { "abstract": true, "attributes": { "x": { "type": "int" } } },
        "B": { "abstract": true, "attributes": { "x": { "type": "string" } } },"#;
    let conflicting = with(&format!(
        r#""metamodel": {{ "types": {{{types} "D": {{ "extends": ["A", "B"] }} }} }}"#
    ));
    let redeclared = with(&format!(
        r#""metamodel": {{ "types": {{{types} "D": {{ "extends": ["A", "B"], "attributes": {{ "x": {{ "type": "int" }} }} }} }} }}"#
    ));

    assert_eq!(
        vec!["error at /metamodel/types/D: 'D' inherits 'x' from both 'B' and 'A', differently, and does not redeclare it (DISL §4.7).".to_owned()],
        diagnostics(&conflicting)
    );
    assert_eq!(
        "int",
        loaded(&redeclared).metamodel().types["D"].attributes["x"].type_name
    );
}

#[test]
fn a_subtype_that_changes_an_attributes_type_is_refused() {
    let json = with(
        r#""metamodel": { "types": { "A": { "attributes": { "x": { "type": "int" } } }, "B": { "extends": "A", "attributes": { "x": { "type": "string" } } } } }"#,
    );

    assert_eq!(
        vec!["error at /metamodel/types/B/attributes/x: 'B' redeclares 'x' as string, changing its type int; a subtype may only narrow it (DISL §4.7).".to_owned()],
        diagnostics(&json)
    );
}

#[test]
fn a_reserved_attribute_name_is_refused() {
    let json = with(r#""metamodel": { "types": { "Thing": { "attributes": { "parent": { "type": "string" } } } } }"#);

    assert_eq!(
        vec!["error at /metamodel/types/Thing/attributes/parent: 'parent' is a reserved name and cannot be an attribute (DISL §2.2).".to_owned()],
        diagnostics(&json)
    );
}

#[test]
fn an_attribute_type_that_names_nothing_is_refused() {
    let json = with(r#""metamodel": { "types": { "Thing": { "attributes": { "size": { "type": "Size" } } } } }"#);

    assert_eq!(
        vec!["error at /metamodel/types/Thing/attributes/size/type: 'Size' is not a primitive, an enum, a data type or a type of this specification (DISL §4.3).".to_owned()],
        diagnostics(&json)
    );
}

#[test]
fn a_relation_end_that_names_nothing_is_refused() {
    let json = with(
        r#""metamodel": { "types": { "Thing": {} }, "relations": { "Link": { "source": "Thing", "target": ["Thing", "Other"] } } }"#,
    );

    assert_eq!(
        vec![
            "error at /metamodel/relations/Link/target: 'Other' is not a type of this specification (DISL §4.9)."
                .to_owned()
        ],
        diagnostics(&json)
    );
}

#[test]
fn the_label_attribute_follows_the_default_rule() {
    let json = with(
        r#""metamodel": { "types": {
          "Keyed": { "attributes": { "title": { "type": "string" }, "code": { "type": "string", "key": true } } },
          "Plain": { "attributes": { "count": { "type": "int" }, "title": { "type": "string" } } },
          "Named": { "labelAttribute": "count", "attributes": { "count": { "type": "int" }, "title": { "type": "string" } } } } }"#,
    );

    let specification = loaded(&json);
    let types = &specification.metamodel().types;

    assert_eq!(Some("code"), types["Keyed"].label_attribute.as_deref());
    assert_eq!(Some("title"), types["Plain"].label_attribute.as_deref());
    assert_eq!(Some("count"), types["Named"].label_attribute.as_deref());
}

#[test]
fn tools_operations_and_scopes_that_name_nothing_are_refused() {
    let json = with(
        r#""metamodel": { "types": { "Thing": {}, "Base": { "abstract": true } } },
        "toolbox": {
          "groups": [ { "id": "g", "tools": [ { "id": "a", "creates": "Base" }, { "id": "b", "creates": "Missing" } ] } ],
          "contextMenus": [ { "for": ["Thing", "Ghost"], "tools": [ { "kind": "operation", "operation": "nothing" } ] } ] },
        "constraints": { "rules": [ { "id": "r", "scope": "Phantom", "rule": "true" } ] },
        "persistence": { "ids": { "types": { "Spectre": { "strategy": "derived", "expression": "'x'" } } } }"#,
    );

    assert_eq!(
        vec![
            "error at /toolbox/groups/0/tools/0/creates: 'Base' is abstract, so nothing can create one (DISL §4.6).",
            "error at /toolbox/groups/0/tools/1/creates: 'Missing' is not a type of this specification (DISL §2.7).",
            "error at /toolbox/contextMenus/0/for: 'Ghost' is not a type of this specification (DISL §2.7).",
            "error at /toolbox/contextMenus/0/tools/0/operation: 'nothing' is not an operation of behavior.operations (DISL §7.3).",
            "error at /constraints/rules/0/scope: 'Phantom' is not a type of this specification (DISL §2.7).",
            "error at /persistence/ids/types/Spectre: 'Spectre' is not a type of this specification (DISL §2.7).",
        ],
        diagnostics(&json)
    );
}

// Step 8.

#[test]
fn an_expression_is_refused_a_variable_its_context_does_not_bind() {
    // newValue is a gesture:change variable; an invariant's rule is in the constraint context.
    let json = with(
        r#""constraints": { "rules": [ { "id": "r", "scope": "Thing", "rule": "newValue != ''" }, { "id": "g", "kind": "change", "scope": "Thing", "rule": "newValue != ''" } ] }"#,
    );

    let result = DislLoader::load(&json);

    assert_eq!(1, result.diagnostics.len());
    assert_eq!("/constraints/rules/0/rule", result.diagnostics[0].pointer);
    assert!(
        result.diagnostics[0]
            .message
            .starts_with("'newValue' is not a variable here; available: self, diagram, env.")
    );
}

#[test]
fn extensions_and_documentation_are_not_compiled() {
    let json = with(
        r#""constraints": { "rules": [ { "id": "r", "scope": "Thing", "rule": "true", "doc": { "summary": "when: no CEL here" }, "x-rule.extra": { "rule": "this is ((( not CEL" } } ] },
        "x-tool": { "when": "neither ((( is this" }"#,
    );

    let specification = loaded(&json);

    assert_eq!(
        vec!["/constraints/rules/0/rule"],
        specification
            .expressions()
            .iter()
            .map(|e| e.pointer.as_str())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        vec!["x-tool"],
        specification
            .extensions()
            .into_iter()
            .map(|(k, _)| k)
            .collect::<Vec<_>>()
    );
}

#[test]
fn actions_bind_their_names_for_the_expressions_after_them() {
    let json = with(
        r#""behavior": { "operations": { "copy": { "for": "Thing", "actions": [
          { "let": { "base": "self.name" } },
          { "create": { "type": "'Thing'", "attributes": { "name": "base + ' copy'" } }, "as": "made" },
          { "select": "made" } ] } } }"#,
    );

    let specification = loaded(&json);

    assert_eq!(
        contexts::OPERATION,
        specification
            .expression_at("/behavior/operations/copy/actions/2/select")
            .unwrap()
            .context
    );
    assert_eq!(4, specification.expressions().len());
}

#[test]
fn a_cel_string_where_no_context_is_known_is_warned_about_and_compiled() {
    let json = with(r#""simulationsElsewhere": { "next": { "cel": "self.name" } }"#);

    let result = DislLoader::load(&json);

    assert!(result.specification.is_some());
    assert_eq!(1, result.diagnostics.len());
    assert_eq!(
        "warning at /simulationsElsewhere/next/cel: This runtime does not know the CEL context of this position; it was compiled with every variable of any context.",
        result.diagnostics[0].to_string()
    );
}

#[test]
fn the_cost_limit_is_the_languages_or_one_million() {
    assert_eq!(1_000_000, loaded(&with("")).cost_limit());
    assert_eq!(
        500,
        loaded(&with(
            r#""language": { "id": "a", "version": "1.0.0", "limits": { "celCost": 500 } }"#
        ))
        .cost_limit()
    );
}
