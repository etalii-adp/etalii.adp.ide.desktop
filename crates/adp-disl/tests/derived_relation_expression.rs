//! A relation type derived in the expression form (DISL section 4.9): a list of maps, each with its `source` and
//! `target`, its `id` or the default `<Type>:<source>-><target>`, and the attributes it names.

mod common;

use adp_cel::Value;
use adp_disl::DislDiagram;
use adp_disl::model::builder;
use common::{loaded, with};

#[test]
fn each_map_the_expression_gives_is_one_relation() {
    let specification = loaded(&with(
        r#""metamodel": {
            "types": { "Stage": { "attributes": {} } },
            "relations": {
                "Next": {
                    "from": "Stage", "to": "Stage",
                    "attributes": { "implicit": { "type": "bool" } },
                    "derived": { "cel": "cel.bind(st, diagram.nodesOfType('Stage'), lists.range(st.size()).filter(i, i > 0).map(i, i == 1 ? {'source': st[0], 'target': st[1], 'implicit': true} : {'id': 'edge:' + st[i - 1].id + '->' + st[i].id, 'source': st[i - 1], 'target': st[i]}))" }
                }
            }
        },
        "notation": { "nodes": { "Stage": { "shape": "rect" } } }"#,
    ));
    let diagram = DislDiagram::new(&specification);
    for id in ["a", "b", "c"] {
        diagram.add_node("Stage", id, adp_cel::Map::default()).unwrap();
    }
    let findings = builder::complete(&diagram);
    assert!(findings.is_empty(), "{findings:?}");
    let relations: Vec<(String, String, String)> = diagram
        .relations()
        .iter()
        .map(|r| (r.id(), r.source().unwrap().id(), r.target().unwrap().id()))
        .collect();
    assert_eq!(
        relations,
        [
            ("Next:a->b".to_owned(), "a".to_owned(), "b".to_owned()),
            ("edge:b->c".to_owned(), "b".to_owned(), "c".to_owned())
        ]
    );
    assert_eq!(diagram.relations()[0].value_of("implicit"), Value::Bool(true));
    assert!(diagram.relations()[0].is_derived());
}
