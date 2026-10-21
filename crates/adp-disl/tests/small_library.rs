//! The smaller parts of DISL's CEL library and model, one test module per standalone host test class:
//! `TemplateFragmentsTests` (section 7.4), `ThemeTokenTests`, `ColorMixTests` and `ListAverageTests` (section 12.4),
//! `BoundsCenterTests` and `ViewAndFileMembersTests` (sections 12.2 and 8.2), and `DislAttributeTypesTests`.

mod common;

use std::collections::HashMap;

use adp_cel::{Map, Value};
use adp_disl::{DislDiagram, DislElement, DislSpecification, contexts};
use common::{attrs, diagnostics, loaded, vars, with};

fn evaluate(specification: &DislSpecification, context: &str, expression: &str) -> Result<Value, String> {
    specification
        .environment(context, &[])
        .compile(expression)
        .unwrap()
        .evaluate(&HashMap::new())
}

/// A toolbox template's fragment (section 7.4): its attribute values are Expressions, strings included - `members`
/// too - compiled in the `template` context, which binds `p`, `diagram`, `env` and `refs`.
mod template_fragments {
    use super::*;

    fn template(attributes: &str) -> String {
        with(&format!(
            r#""metamodel": {{
              "types": {{ "Spot": {{ "attributes": {{ "name": {{ "type": "string" }} }} }},
                         "Ring": {{ "attributes": {{ "title": {{ "type": "string" }}, "members": {{ "type": "Spot", "many": true }} }} }} }},
              "relations": {{ "Path": {{ "source": "Spot", "target": "Spot", "attributes": {{ "note": {{ "type": "string" }} }} }} }} }},
            "toolbox": {{ "templates": {{ "pair": {{
              "label": "Pair",
              "params": {{ "first": {{ "type": "string", "default": "one" }} }},
              "fragment": {{
                "nodes": [
                  {{ "ref": "one", "type": "Spot", "attributes": {{ "name": "p.first" }}, "at": [0, 0] }},
                  {{ "ref": "two", "type": "Spot", "attributes": {{ "name": "'two'" }}, "at": [100, 0] }},
                  {{ "ref": "ring", "type": "Ring", "attributes": {attributes} }}
                ],
                "relations": [ {{ "type": "Path", "source": "one", "target": "two", "attributes": {{ "note": "'from ' + p.first" }} }} ] }} }} }} }}"#
        ))
    }

    const MEMBERS: &str = r#"{ "title": "'Ring of ' + p.first", "members": "[refs.one, refs.two]" }"#;

    #[test]
    fn a_fragment_reading_its_refs_and_parameters_loads() {
        assert!(diagnostics(&template(MEMBERS)).is_empty());
    }

    #[test]
    fn every_attribute_value_of_a_fragment_is_compiled_in_the_template_context() {
        let specification = loaded(&template(MEMBERS));
        for pointer in [
            "/toolbox/templates/pair/fragment/nodes/0/attributes/name",
            "/toolbox/templates/pair/fragment/nodes/1/attributes/name",
            "/toolbox/templates/pair/fragment/nodes/2/attributes/title",
            "/toolbox/templates/pair/fragment/nodes/2/attributes/members",
            "/toolbox/templates/pair/fragment/relations/0/attributes/note",
        ] {
            assert_eq!(
                Some("template"),
                specification.expression_at(pointer).map(|e| e.context.as_str()),
                "{pointer}"
            );
        }
    }

    #[test]
    fn a_fragment_reading_self_is_refused() {
        // A fragment is inserted, not drawn, so there is no self.
        assert_eq!(
            vec!["error at /toolbox/templates/pair/fragment/nodes/2/attributes/title: 'self' is not a variable here; available: p, diagram, env, refs. (context template)".to_owned()],
            diagnostics(&template(r#"{ "title": "self.name" }"#))
        );
    }
}

/// `token(name)` (section 12.4): a resolved theme token, in the element and shape contexts. A headless runtime has
/// no theme, so it declares the function and reports the token it cannot resolve when one is evaluated.
mod theme_token {
    use super::*;

    #[test]
    fn a_token_compiles_in_the_contexts_that_draw_it() {
        let specification = loaded(&with(""));
        for context in [contexts::ELEMENT, contexts::SHAPE] {
            assert!(
                specification
                    .environment(context, &[])
                    .compile("token('color.text')")
                    .is_ok()
            );
        }
    }

    #[test]
    fn a_token_is_not_resolved_without_a_theme() {
        assert_eq!(
            Err(
                "The theme token 'color.text' is resolved where the diagram is drawn; this runtime has no theme."
                    .to_owned()
            ),
            evaluate(&loaded(&with("")), contexts::ELEMENT, "token('color.text')")
        );
    }

    #[test]
    fn a_definition_colouring_its_notation_by_token_loads() {
        let json = with(
            r#""metamodel": { "types": { "Thing": {} } },
            "notation": { "nodes": { "Thing": { "shape": "rect", "icon": { "icon": "mdi-sync", "color": { "cel": "token('color.text')" } } } } }"#,
        );
        assert!(diagnostics(&json).is_empty());
    }
}

/// `color(c).mix(c2, f)` (section 12.4): CSS Color 5 `color-mix()` in sRGB with premultiplied alpha, written as
/// `#rrggbbaa`.
mod color_mix {
    use super::*;

    fn mix(expression: &str) -> Result<Value, String> {
        evaluate(&loaded(&with("")), contexts::ELEMENT, expression)
    }

    #[test]
    fn a_mix_is_interpolated_on_srgb() {
        for (expression, expected) in [
            ("color('#ff0000').mix('#0000ff', 0.25)", "#bf0040ff"),
            ("color('#ff0000').mix('#0000ff', 0)", "#ff0000ff"),
            ("color('#ff0000').mix('#0000ff', 1)", "#0000ffff"),
            ("color('#fff').mix('#000000', 0.5)", "#808080ff"),
        ] {
            assert_eq!(Ok(Value::from(expected)), mix(expression), "{expression}");
        }
    }

    #[test]
    fn a_mix_premultiplies_alpha() {
        // An opaque red mixed half with transparent blue keeps red's channels and halves the alpha.
        assert_eq!(
            Ok(Value::from("#ff000080")),
            mix("color('#ff0000ff').mix('#0000ff00', 0.5)")
        );
    }

    #[test]
    fn what_is_not_a_colour_or_a_fraction_is_an_error() {
        for (expression, message) in [
            (
                "color('red').mix('#000', 0.5)",
                "color() takes a hex colour, not 'red'.",
            ),
            (
                "color('#ff0000').mix('#000', 1.5)",
                "mix() takes a fraction from 0 to 1, not 1.5.",
            ),
        ] {
            assert_eq!(Err(message.to_owned()), mix(expression));
        }
    }

    #[test]
    fn a_mix_of_theme_tokens_compiles_in_the_contexts_that_draw_it() {
        assert!(
            loaded(&with(""))
                .environment(contexts::SHAPE, &[])
                .compile("color(token('color.play')).mix(token('color.surface.raised'), 0.82)")
                .is_ok()
        );
    }
}

/// `l.avg()` (section 12.4): the mean of a list of numbers, as a double.
mod list_average {
    use super::*;

    #[test]
    fn the_mean_is_a_double() {
        let specification = loaded(&with(""));
        for (expression, expected) in [
            ("[1, 2, 3, 4].avg()", 2.5),
            ("[2.5].avg()", 2.5),
            ("[1, 2.0].avg()", 1.5),
            ("[-3, 3].avg()", 0.0),
            ("[10, 20].map(n, n * 2).avg()", 30.0),
        ] {
            assert!(
                matches!(evaluate(&specification, contexts::ELEMENT, expression), Ok(Value::Double(v)) if v == expected),
                "{expression}"
            );
        }
    }

    #[test]
    fn the_mean_of_nothing_or_of_text_is_an_error() {
        let specification = loaded(&with(""));
        for (expression, message) in [
            ("[].avg()", "avg() of an empty list has no value."),
            ("['a'].avg()", "avg() takes a list of numbers."),
        ] {
            assert_eq!(
                Err(message.to_owned()),
                evaluate(&specification, contexts::ELEMENT, expression)
            );
        }
    }
}

fn evaluate_on(
    specification: &DislSpecification,
    expression: &str,
    element: Value,
    diagram: &DislDiagram,
) -> Result<Value, String> {
    specification
        .environment(contexts::ELEMENT, &[])
        .compile(expression)
        .unwrap()
        .evaluate(&vars([
            ("self", element),
            ("diagram", diagram.to_value()),
            ("env", Value::map(Map::default())),
        ]))
}

/// `b.center()` on a Bounds (section 12.2): the point halfway across and down, as `[x, y]` doubles, for the bounds a
/// host binds in an element's `view`.
mod bounds_center {
    use super::*;

    fn specification() -> DislSpecification {
        loaded(&with(
            r#""metamodel": { "types": { "Spot": {}, "Ring": { "attributes": { "around": { "type": "Spot", "many": true } } } } }"#,
        ))
    }

    fn bounds(x: Value, y: Value, width: Value, height: Value) -> Option<Map> {
        Some(attrs([(
            "bounds",
            Value::map(attrs([("x", x), ("y", y), ("width", width), ("height", height)])),
        )]))
    }

    #[test]
    fn the_center_is_halfway_across_and_down() {
        let specification = specification();
        let diagram = DislDiagram::new(&specification);
        let spot = diagram.add_node("Spot", "s", Map::default()).unwrap();
        spot.set_view(bounds(Value::Int(10), Value::Int(20), Value::Int(100), Value::Int(40)));

        let center = evaluate_on(&specification, "self.view.bounds.center()", spot.to_value(), &diagram);

        assert!(
            matches!(center, Ok(Value::List(l)) if matches!(l.as_slice(), [Value::Double(x), Value::Double(y)] if *x == 60.0 && *y == 40.0))
        );
    }

    #[test]
    fn the_centers_of_several_elements_average() {
        let specification = specification();
        let diagram = DislDiagram::new(&specification);
        let left = diagram.add_node("Spot", "l", Map::default()).unwrap();
        let right = diagram.add_node("Spot", "r", Map::default()).unwrap();
        let d = Value::Double;
        left.set_view(bounds(d(0.0), d(0.0), d(20.0), d(10.0)));
        right.set_view(bounds(d(100.0), d(50.0), d(20.0), d(10.0)));
        let ring = diagram
            .add_node(
                "Ring",
                "ring",
                attrs([("around", Value::list(vec!["l".into(), "r".into()]))]),
            )
            .unwrap();

        let x = evaluate_on(
            &specification,
            "self.around.map(m, m.view.bounds.center()[0]).avg()",
            ring.to_value(),
            &diagram,
        );
        let y = evaluate_on(
            &specification,
            "self.around.map(m, m.view.bounds.center()[1]).avg()",
            ring.to_value(),
            &diagram,
        );

        assert_eq!((Ok(Value::Double(60.0)), Ok(Value::Double(30.0))), (x, y));
    }

    #[test]
    fn bounds_without_a_size_have_no_center() {
        let specification = specification();
        let diagram = DislDiagram::new(&specification);
        let spot = diagram.add_node("Spot", "s", Map::default()).unwrap();
        spot.set_view(Some(attrs([(
            "bounds",
            Value::map(attrs([("x", Value::Int(1)), ("y", Value::Int(2))])),
        )])));

        assert_eq!(
            Err("center() needs bounds with x, y, width and height.".to_owned()),
            evaluate_on(&specification, "self.view.bounds.center()", spot.to_value(), &diagram)
        );
    }
}

/// The two members a host binds for one reading rather than storing (sections 12.2 and 8.2): an element's `view`,
/// the viewer's state of it, and the diagram's `file`, the file a finding names.
mod view_and_file_members {
    use super::*;

    fn specification() -> DislSpecification {
        loaded(&with(
            r#""metamodel": { "types": { "Topic": { "attributes": { "text": { "type": "string" } }, "children": { "allowed": ["Topic"] } } } }"#,
        ))
    }

    fn shadowed() -> DislSpecification {
        loaded(&with(
            r#""metamodel": { "diagram": { "attributes": { "file": { "type": "string", "default": "stored" } } }, "types": { "Topic": {} } }"#,
        ))
    }

    #[test]
    fn an_elements_view_is_what_the_host_bound() {
        for (expression, expected) in [
            ("self.view.collapsed", true),
            ("has(self.view)", true),
            ("self.view.collapsed && self.children.size() > 0", true),
        ] {
            let specification = specification();
            let diagram = DislDiagram::new(&specification);
            let topic = diagram.add_node("Topic", "t", Map::default()).unwrap();
            diagram.add_child("Topic", "c", Map::default(), &topic, None).unwrap();
            topic.set_view(Some(attrs([("collapsed", Value::Bool(true))])));

            assert_eq!(
                Ok(Value::Bool(expected)),
                evaluate_on(&specification, expression, topic.to_value(), &diagram),
                "{expression}"
            );
        }
    }

    #[test]
    fn an_element_without_a_view_has_no_view_member() {
        let specification = specification();
        let diagram = DislDiagram::new(&specification);
        let topic: DislElement = diagram.add_node("Topic", "t", Map::default()).unwrap();

        assert_eq!(
            Ok(Value::Bool(false)),
            evaluate_on(&specification, "has(self.view)", topic.to_value(), &diagram)
        );
        assert!(evaluate_on(&specification, "self.view.collapsed", topic.to_value(), &diagram).is_err());
    }

    #[test]
    fn the_diagrams_file_is_what_the_host_bound() {
        let specification = specification();
        let diagram = DislDiagram::new(&specification);
        diagram.set_file(Some("ideas.adp"));

        assert_eq!(
            Ok(Value::from("ideas.adp!")),
            evaluate_on(&specification, "diagram.file + '!'", diagram.to_value(), &diagram)
        );
    }

    #[test]
    fn a_diagram_without_a_file_has_no_file_member() {
        let specification = specification();
        let diagram = DislDiagram::new(&specification);

        assert_eq!(
            Ok(Value::Bool(false)),
            evaluate_on(&specification, "has(diagram.file)", diagram.to_value(), &diagram)
        );
    }

    #[test]
    fn a_diagram_attribute_named_file_shadows_the_bound_file() {
        let specification = shadowed();
        let diagram = DislDiagram::new(&specification);
        diagram.set_file(Some("ideas.adp"));

        assert_eq!(
            Ok(Value::from("stored")),
            evaluate_on(&specification, "diagram.file", diagram.to_value(), &diagram)
        );
    }
}

/// The attribute types a specification gives a binding's types, resolved through `persistence.typeMap`: a mapped
/// type, a renamed attribute, an inherited one and a host attribute, and the types that give none.
mod disl_attribute_types {
    use super::*;
    use adp_disl::model::attribute_types;

    fn specification() -> DislSpecification {
        loaded(&with(
            r#""metamodel": {
              "diagram": { "attributes": { "scale": { "type": "int" } } },
              "types": {
                "Base": { "abstract": true, "attributes": { "title": { "type": "string" }, "start": { "type": "datetime" }, "lane": { "type": "int" } } },
                "Span": { "extends": "Base", "attributes": { "until": { "type": "date" }, "done": { "type": "bool" } } },
                "Point": { "extends": "Base" } } },
            "persistence": { "typeMap": {
              "Bar": { "as": "Span", "attributes": { "caption": "title" }, "hostAttributes": ["raw"] },
              "Top": { "as": "header" },
              "Settings": { "as": "diagram", "attributes": { "zoom": "scale" } },
              "Junk": { "as": "unreadable" } } }"#,
        ))
    }

    #[test]
    fn a_binding_attribute_has_the_type_of_the_attribute_it_maps_to() {
        let type_of = attribute_types::of(&specification());
        for (type_name, attribute, expected) in [
            ("Bar", "done", "bool"),
            ("Bar", "until", "date"),
            ("Bar", "caption", "string"),
            ("Bar", "start", "datetime"),
            ("Bar", "lane", "int"),
            ("Bar", "raw", "string"),
            ("Point", "lane", "int"),
            ("Settings", "zoom", "int"),
        ] {
            assert_eq!(
                Some(expected.to_owned()),
                type_of(type_name, attribute),
                "{type_name}.{attribute}"
            );
        }
    }

    #[test]
    fn an_attribute_the_specification_does_not_type_has_no_type() {
        let type_of = attribute_types::of(&specification());
        for (type_name, attribute) in [
            ("Top", "title"),
            ("Junk", "title"),
            ("Bar", "nothing"),
            ("Nowhere", "title"),
        ] {
            assert_eq!(None, type_of(type_name, attribute), "{type_name}.{attribute}");
        }
    }

    #[test]
    fn the_time_attributes_are_the_bindings_date_and_date_time_attributes_by_rule() {
        let (problems, document) = adp_fbl::documents::load(
            br#"{ "fbl": "0.5", "bindings": { "t": { "claims": { "extensions": [".t"] }, "body": { "kind": "file", "family": "yaml" }, "reader": "declared",
              "elements": [
                { "name": "bar", "type": "Bar", "at": "/bars/*", "attributes": { "caption": { "key": "caption" }, "start": { "key": "start" }, "until": { "key": "until" }, "lane": { "key": "lane" } } },
                { "name": "point", "type": "Point", "at": "/points/*", "attributes": { "start": { "key": "at" } } } ] } } }"#,
        );
        assert!(problems.is_empty(), "{problems:?}");
        let document = document.unwrap();

        let mut observed: Vec<String> =
            attribute_types::time_attributes(&specification(), document.binding("t").unwrap())
                .into_iter()
                .collect();
        observed.sort();

        assert_eq!(vec!["bar.start", "bar.until", "point.start"], observed);
    }
}
