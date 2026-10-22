//! A relation type inherits `directed`, `allowSelfLoops` and `allowParallel` along its C3 linearisation, else the
//! section 4.9 default, and may only narrow them; supertypes that differ in a flag the type does not state are a
//! specification error (DISL 0.6, section 4.7; the standalone host's `RelationFlagInheritanceTests`). Checked against
//! `std.endpoints` (section 8.7) and against etalii.adp's example and refused specifications, as vendored.

mod common;

use adp_disl::derivation::constraints;
use adp_disl::{DislConstraintOptions, DislDiagram, DislElement, DislLoader, DislSpecification, NewElement};
use common::{attrs, loaded, with};

const VENDORED: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../vendor/etalii.adp/specifications/disl"
);

fn vendored(path: &str) -> String {
    std::fs::read_to_string(format!("{VENDORED}/{path}")).expect("the vendored document is there")
}

fn specification() -> DislSpecification {
    loaded(&with(
        r#""metamodel": {
          "types": { "Box": {} },
          "relations": {
            "Once": { "abstract": true, "source": "Box", "target": "Box", "allowParallel": false },
            "OnceChild": { "extends": "Once" },
            "Loops": { "abstract": true, "source": "Box", "target": "Box", "allowSelfLoops": true },
            "LoopsChild": { "extends": "Loops" },
            "Mutual": { "abstract": true, "source": "Box", "target": "Box", "directed": false, "allowParallel": false },
            "MutualChild": { "extends": "Mutual" },
            "Unstated": { "source": "Box", "target": "Box" }
          }
        },
        "constraints": { "builtIn": { "std.endpoints": { "message": { "cel": "detail.violation" } } } }"#,
    ))
}

fn node(diagram: &DislDiagram, type_name: &str, id: &str) -> DislElement {
    diagram.add(NewElement::new(type_name, id)).unwrap()
}

fn relation(diagram: &DislDiagram, type_name: &str, id: &str, source: &DislElement, target: &DislElement) {
    let mut new = NewElement::new(type_name, id);
    new.source = Some(source.clone());
    new.target = Some(target.clone());
    new.source_id = Some(source.id());
    new.target_id = Some(target.id());
    diagram.add(new).unwrap();
}

/// The `std.endpoints` findings of two relations of `relation_type` (one loop when `pair` is none), as
/// `violation@element`.
fn violations(relation_type: &str, pair: Option<bool>) -> Vec<String> {
    let specification = specification();
    let diagram = DislDiagram::new(&specification);
    let a = node(&diagram, "Box", "a");
    match pair {
        None => relation(&diagram, relation_type, "first", &a, &a),
        Some(opposite) => {
            let b = node(&diagram, "Box", "b");
            relation(&diagram, relation_type, "first", &a, &b);
            if opposite {
                relation(&diagram, relation_type, "second", &b, &a);
            } else {
                relation(&diagram, relation_type, "second", &a, &b);
            }
        }
    }
    constraints::evaluate(&specification, &diagram, &DislConstraintOptions::default())
        .iter()
        .filter(|f| f.constraint_id == "std.endpoints")
        .map(|f| format!("{}@{}", f.message, f.element_ids.join("|")))
        .collect()
}

#[test]
fn a_second_relation_on_one_pair_of_a_subtype_that_inherits_allow_parallel_false_is_a_parallel() {
    assert_eq!(vec!["parallel@second"], violations("OnceChild", Some(false)));
}

#[test]
fn a_loop_of_a_subtype_that_inherits_allow_self_loops_true_is_not_reported() {
    assert!(violations("LoopsChild", None).is_empty());
}

#[test]
fn an_opposite_pair_of_a_subtype_that_inherits_directed_false_and_allow_parallel_false_is_a_parallel() {
    assert_eq!(vec!["parallel@second"], violations("MutualChild", Some(true)));
}

#[test]
fn a_loop_of_a_relation_type_that_states_no_allow_self_loops_anywhere_is_a_self_loop() {
    assert_eq!(vec!["selfLoop@first"], violations("Unstated", None));
}

#[test]
fn the_service_calls_example_has_the_effective_flags_of_the_data_model() {
    let specification = loaded(&vendored("service-calls.dis"));
    let flags = |name: &str| {
        let t = &specification.metamodel().relations[name];
        (t.directed, t.allow_self_loops, t.allow_parallel)
    };
    assert_eq!(
        vec![
            (true, true, false),
            (true, true, false),
            (true, true, false),
            (true, false, true),
            (true, false, false)
        ],
        ["Call", "SyncCall", "AsyncCall", "Observation", "TracedCall"]
            .map(flags)
            .to_vec()
    );
}

/// The definition of etalii.adp's `service-calls.did`, built on the vendored `service-calls.dis`, gives exactly the
/// two findings of `service-calls.findings.json`, in its order.
#[test]
fn the_service_calls_example_gives_the_two_findings_of_its_findings_file() {
    let specification = loaded(&vendored("service-calls.dis"));
    let diagram = DislDiagram::new(&specification);
    let service = |id: &str, name: &str| {
        let mut new = NewElement::new("Service", id);
        new.attributes = attrs([("name", name.into())]);
        diagram.add(new).unwrap()
    };
    let (checkout, payments, ledger) = (
        service("checkout", "Checkout"),
        service("payments", "Payments"),
        service("ledger", "Ledger"),
    );
    relation(&diagram, "SyncCall", "call-1", &checkout, &payments);
    relation(&diagram, "SyncCall", "call-2", &checkout, &payments);
    relation(&diagram, "AsyncCall", "call-3", &checkout, &payments);
    relation(&diagram, "SyncCall", "call-4", &ledger, &ledger);
    relation(&diagram, "TracedCall", "call-5", &payments, &payments);

    let findings: Vec<String> = constraints::evaluate(&specification, &diagram, &DislConstraintOptions::default())
        .iter()
        .map(|f| format!("{} {} {}: {}", f.constraint_id, f.severity, f.element_ids[0], f.message))
        .collect();

    assert_eq!(
        vec![
            "std.endpoints error call-2: A SyncCall already connects these two elements.",
            "std.endpoints error call-5: A TracedCall cannot connect an element to itself.",
        ],
        findings
    );
}

/// Each specification etalii.adp keeps in `specifications/disl/refused/` fails to load, with one error naming its
/// relation type and flag.
#[test]
fn a_refused_specification_fails_to_load_with_one_error_naming_its_type_and_flag() {
    for (file, pointer, type_name, flag) in [
        (
            "relation-flag-widened.dis",
            "/metamodel/relations/Strong/allowParallel",
            "'Strong'",
            "allowParallel",
        ),
        (
            "relation-flag-directed.dis",
            "/metamodel/relations/Mutual/directed",
            "'Mutual'",
            "directed",
        ),
        (
            "relation-flag-conflict.dis",
            "/metamodel/relations/Both",
            "'Both'",
            "allowSelfLoops",
        ),
    ] {
        let result = DislLoader::load(&vendored(&format!("refused/{file}")));
        assert!(result.specification.is_none(), "{file} loads");
        let errors: Vec<_> = result.errors().collect();
        assert_eq!(1, errors.len(), "{file}: {errors:?}");
        assert_eq!(pointer, errors[0].pointer, "{file}");
        for word in [type_name, flag, "DISL §4.7"] {
            assert!(errors[0].message.contains(word), "{file}: {}", errors[0].message);
        }
    }
}
