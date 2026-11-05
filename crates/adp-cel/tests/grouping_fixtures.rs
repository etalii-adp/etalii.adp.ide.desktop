//! The grouping fixtures of DISL 0.9 (section 12.4; spec 075, T013), as vendored: each list grouped by its key gives
//! the keys and groups it states, or fails. The files are included at compile time, so that the same test runs
//! natively and, under `wasm-bindgen-test`, in WebAssembly (T019).

use std::rc::Rc;

use adp_cel::{Environment, Grouping, Map, Value};
use serde_json::Value as Json;

macro_rules! fixture {
    ($name:literal) => {
        (
            $name,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../vendor/etalii.adp/specifications/disl/fixtures/grouping/",
                $name,
                ".json"
            )),
        )
    };
}

/// Every fixture of `specifications/disl/fixtures/grouping/`, by name.
const FIXTURES: &[(&str, &str)] = &[
    fixture!("absent-key"),
    fixture!("absent-kind"),
    fixture!("empty"),
    fixture!("failing-absorbed-and"),
    fixture!("failing-absorbed-or"),
    fixture!("failing-missing-field"),
    fixture!("failing-one-item"),
    fixture!("key-bool"),
    fixture!("key-int-double"),
    fixture!("key-list"),
    fixture!("key-map"),
    fixture!("key-nan"),
    fixture!("key-null"),
    fixture!("key-number"),
    fixture!("key-zero"),
    fixture!("order-concatenated"),
    fixture!("order-distinct"),
    fixture!("order-maps"),
    fixture!("order-strings"),
];

/// A JSON value as CEL reads it: a number with a fraction or an exponent is a double, any other an int.
fn value(json: &Json) -> Value {
    match json {
        Json::Null => Value::Null,
        Json::Bool(b) => Value::Bool(*b),
        Json::Number(n) => match n.as_i64() {
            Some(i) => Value::Int(i),
            None => Value::Double(n.as_f64().expect("a number")),
        },
        Json::String(s) => Value::String(s.clone()),
        Json::Array(items) => Value::list(items.iter().map(value).collect()),
        Json::Object(entries) => {
            let mut map = Map::default();
            for (key, entry) in entries {
                map.insert(key.clone(), value(entry));
            }
            Value::Map(Rc::new(map))
        }
    }
}

/// Equal and of one kind: an int is not a double, and -0.0 is not 0.0, so that a key is the one first met.
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Double(x), Value::Double(y)) => x.to_bits() == y.to_bits(),
        (Value::String(x), Value::String(y)) => x == y,
        (Value::List(x), Value::List(y)) => x.len() == y.len() && x.iter().zip(y.iter()).all(|(a, b)| same(a, b)),
        (Value::Map(x), Value::Map(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w)))
        }
        _ => false,
    }
}

/// What the fixture's list grouped by its key gives: the grouping, or the sentence of its failure.
fn grouped(fixture: &Json) -> Result<Rc<Grouping>, String> {
    let mut environment = Environment::standard(&["list"]);
    environment.add_grouping();
    let variable = fixture["variable"].as_str().unwrap();
    let key = fixture["key"].as_str().unwrap();
    let program = environment.compile(&format!("list.groupBy({variable}, {key})"))?;
    let mut variables = Map::default();
    variables.insert("list", value(&fixture["list"]));
    let result = program.evaluate(&variables)?;
    Ok(result.downcast::<Grouping>().expect("groupBy gives a grouping"))
}

/// The problems of one fixture, none when the runtime gives what it states.
fn problems(name: &str, text: &str) -> Vec<String> {
    let fixture: Json = serde_json::from_str(text).unwrap();
    let mut problems = Vec::new();
    let grouping = match grouped(&fixture) {
        Ok(grouping) if fixture.get("fails").is_some() => {
            problems.push(format!("{name}: grouped into {:?} where it fails", grouping.keys()));
            return problems;
        }
        Ok(grouping) => grouping,
        Err(e) if fixture.get("fails").is_some() => {
            assert!(!e.is_empty());
            return problems;
        }
        Err(e) => {
            problems.push(format!("{name}: failed where it groups: {e}"));
            return problems;
        }
    };
    let keys = grouping.keys();
    if !same(&keys, &value(&fixture["keys"])) {
        problems.push(format!("{name}: keys {keys:?}, not {}", fixture["keys"]));
    }
    let keys = keys.as_list().unwrap().to_vec();
    for (key, group) in keys.iter().zip(fixture["groups"].as_array().unwrap()) {
        let got = grouping.get(key).unwrap();
        if !same(&got, &value(group)) {
            problems.push(format!("{name}: the group of {key:?} is {got:?}, not {group}"));
        }
    }
    for read in fixture.get("reads").and_then(Json::as_array).into_iter().flatten() {
        let got = grouping.get(&value(&read["key"])).unwrap();
        if !same(&got, &value(&read["group"])) {
            problems.push(format!(
                "{name}: get({}) is {got:?}, not {}",
                read["key"], read["group"]
            ));
        }
    }
    problems
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn every_grouping_fixture_gives_what_it_states() {
    let problems: Vec<String> = FIXTURES.iter().flat_map(|(name, text)| problems(name, text)).collect();
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// The list above names every fixture vendored, so that a new one is never skipped.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn every_vendored_fixture_is_run() {
    let folder = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vendor/etalii.adp/specifications/disl/fixtures/grouping"
    );
    let mut vendored: Vec<String> = std::fs::read_dir(folder)
        .unwrap()
        .map(|e| {
            e.unwrap()
                .file_name()
                .to_string_lossy()
                .trim_end_matches(".json")
                .to_owned()
        })
        .collect();
    vendored.sort();
    let mut run: Vec<String> = FIXTURES.iter().map(|(name, _)| (*name).to_owned()).collect();
    run.sort();
    assert_eq!(vendored, run);
}
