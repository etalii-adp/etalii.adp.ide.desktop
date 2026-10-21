//! The vendored JSON Schemas a specification and an envelope are checked against: `desl.schema.json`
//! (`$defs/Specification`, DESL section 10.1, step 3), which references FBL's for a binding reference, and
//! `ded.schema.json` (`$defs/Definition`, DED section 6). The schemas are compiled once and kept for the process.

use std::sync::OnceLock;

use jsonschema::{Retrieve, Uri, Validator};
use serde_json::Value;

const DESL_SCHEMA: &str = include_str!("../../../vendor/etalii.adp/specifications/desl/desl.schema.json");
const FBL_SCHEMA: &str = include_str!("../../../vendor/etalii.adp/specifications/fbl/fbl.schema.json");
const DED_SCHEMA: &str = include_str!("../../../vendor/etalii.adp/specifications/ded/ded.schema.json");

/// The address DESL's schema references FBL's under.
const FBL_ID: &str = "https://etalii.net/adp/fbl/schema/0.1/fbl.schema.json";

/// Hands the schema compiler FBL's schema, the one schema DESL's references, and nothing from the network.
struct Vendored;

impl Retrieve for Vendored {
    fn retrieve(&self, uri: &Uri<String>) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        if uri.as_str().trim_end_matches('#') == FBL_ID {
            return Ok(serde_json::from_str(FBL_SCHEMA)?);
        }
        Err(format!("'{}' is not a vendored schema", uri.as_str()).into())
    }
}

fn compile(text: &str) -> Result<Validator, String> {
    let schema: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    jsonschema::options()
        .with_retriever(Vendored)
        .build(&schema)
        .map_err(|e| e.to_string())
}

fn desl() -> &'static Result<Validator, String> {
    static VALIDATOR: OnceLock<Result<Validator, String>> = OnceLock::new();
    VALIDATOR.get_or_init(|| compile(DESL_SCHEMA))
}

fn ded() -> &'static Result<Validator, String> {
    static VALIDATOR: OnceLock<Result<Validator, String>> = OnceLock::new();
    VALIDATOR.get_or_init(|| compile(DED_SCHEMA))
}

/// What a schema says of an instance: each break as the JSON Pointer of the value and the sentence.
fn check(validator: &Result<Validator, String>, instance: &Value) -> Vec<(String, String)> {
    match validator {
        Ok(validator) => validator
            .iter_errors(instance)
            .map(|e| (e.instance_path().to_string(), e.to_string()))
            .collect(),
        Err(problem) => vec![(String::new(), format!("The vendored schema cannot be used: {problem}"))],
    }
}

/// The breaks of `desl.schema.json` in a specification.
pub(crate) fn check_specification(instance: &Value) -> Vec<(String, String)> {
    check(desl(), instance)
}

/// The breaks of `ded.schema.json` in a definition's envelope, read into its YAML and JSON form (DED section 6).
pub(crate) fn check_definition(instance: &Value) -> Vec<(String, String)> {
    check(ded(), instance)
}
