//! User functions (DISL section 3.4): compiled in declaration order, recursion only when bounded, context variables
//! only through `uses` (the standalone host's `UserFunctionsTests`).

mod common;

use std::collections::HashMap;

use adp_cel::{Map, Value};
use adp_disl::{DislSpecification, contexts};
use common::{diagnostics, loaded, vars, with};

fn evaluate(
    specification: &DislSpecification,
    expression: &str,
    variables: HashMap<String, Value>,
) -> Result<Value, String> {
    specification
        .environment(contexts::ELEMENT, &[])
        .compile(expression)
        .unwrap()
        .evaluate(&variables)
}

#[test]
fn a_function_calls_the_functions_declared_before_it() {
    let specification = loaded(&with(
        r#""functions": {
          "twice": { "params": [ { "name": "n", "type": "int" } ], "returns": "int", "cel": "n * 2" },
          "quadruple": { "params": [ { "name": "n", "type": "int" } ], "returns": "int", "cel": "twice(twice(n))" } }"#,
    ));

    assert_eq!(
        Ok(Value::Int(20)),
        evaluate(&specification, "quadruple(5)", HashMap::new())
    );
    assert_eq!(
        vec!["twice", "quadruple"],
        specification
            .functions()
            .iter()
            .map(|f| f.name.as_str())
            .collect::<Vec<_>>()
    );
}

#[test]
fn a_function_calling_one_declared_after_it_is_refused() {
    let json = with(
        r#""functions": {
          "quadruple": { "params": [ { "name": "n", "type": "int" } ], "returns": "int", "cel": "twice(twice(n))" },
          "twice": { "params": [ { "name": "n", "type": "int" } ], "returns": "int", "cel": "n * 2" } }"#,
    );

    assert_eq!(
        vec!["error at /functions/quadruple/cel: 'quadruple' calls 'twice', which is declared after it or does not compile; a function may call only the functions declared before it (DISL §3.4).".to_owned()],
        diagnostics(&json)
    );
}

#[test]
fn a_function_calling_itself_without_recursion_is_refused() {
    let json = with(
        r#""functions": { "loop": { "params": [ { "name": "n", "type": "int" } ], "returns": "int", "cel": "n == 0 ? 0 : loop(n - 1)" } }"#,
    );

    assert_eq!(
        vec!["error at /functions/loop/cel: 'loop' calls itself, which a function may do only when it declares recursion (DISL §3.4).".to_owned()],
        diagnostics(&json)
    );
}

/// sum(n) = n + sum(n - 1), four calls deep at most: sum(3) reaches sum(0) at depth 4 and is exact; sum(4) would
/// reach sum(0) at depth 5, and sum(5) sum(1), which give atMaxDepth over their own n.
#[test]
fn a_bounded_recursive_function_gives_at_max_depth_beyond_its_depth() {
    let specification = loaded(&with(
        r#""functions": { "sum": { "params": [ { "name": "n", "type": "int" } ], "returns": "int",
          "recursion": { "maxDepth": 4, "atMaxDepth": "-1000 - n" },
          "cel": "n == 0 ? 0 : n + sum(n - 1)" } }"#,
    ));

    for (n, expected) in [(1, 1), (3, 6), (4, 4 + 3 + 2 + 1 - 1000), (5, 5 + 4 + 3 + 2 - 1001)] {
        assert_eq!(
            Ok(Value::Int(expected)),
            evaluate(&specification, &format!("sum({n})"), HashMap::new())
        );
        assert_eq!(
            Ok(Value::Int(expected)),
            evaluate(&specification, &format!("sum({n})"), HashMap::new())
        );
    }
}

#[test]
fn a_function_reads_diagram_only_through_uses() {
    let refused = with(r#""functions": { "count": { "params": [], "returns": "int", "cel": "size(diagram)" } }"#);
    let allowed = loaded(&with(
        r#""functions": { "count": { "params": [], "returns": "int", "uses": ["diagram"], "cel": "size(diagram)" } }"#,
    ));

    let found = diagnostics(&refused);
    assert_eq!(1, found.len());
    assert!(found[0].starts_with("error at /functions/count/cel: 'diagram' is not a variable here"));
    let mut map = Map::default();
    map.insert("a", Value::Int(1));
    map.insert("b", Value::Int(2));
    assert_eq!(
        Ok(Value::Int(2)),
        evaluate(
            &allowed,
            "count()",
            vars([
                ("diagram", Value::map(map)),
                ("self", Value::Null),
                ("env", Value::Null)
            ])
        )
    );
}

#[test]
fn a_function_does_not_see_the_variables_of_its_caller() {
    let json = with(r#""functions": { "name": { "params": [], "returns": "string", "cel": "self.name" } }"#);

    let found = diagnostics(&json);
    assert_eq!(1, found.len());
    assert!(found[0].starts_with("error at /functions/name/cel: 'self' is not a variable here"));
}

#[test]
fn a_failing_function_fails_the_expression_that_calls_it() {
    let specification = loaded(&with(
        r#""functions": { "half": { "params": [ { "name": "n", "type": "int" } ], "returns": "int", "cel": "10 / n" } }"#,
    ));

    assert_eq!(
        Err("Division by zero.".to_owned()),
        evaluate(&specification, "half(0) + 1", HashMap::new())
    );
}
