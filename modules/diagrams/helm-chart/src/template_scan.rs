//! The literal-fact line scanner for `templates/` content: what a Go-templated manifest says about itself on lines that
//! need no rendering to read (etalii.adp spec 030, task T080).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/helm-chart/backend/EtAlii.Adp.Diagram.HelmChart/
//! TemplateScan.cs`. A template is never parsed as YAML; a column-zero `kind:` or `apiVersion:` whose value carries
//! no template expression, a `{{ define "name" }}` and an `{{ include "name" }}` or `{{ template "name" }}` with a
//! literal double-quoted name are facts, everything templated is unknown by design. Lines are split on LF alone, a CR
//! before it taken off.

use std::collections::BTreeSet;
use std::sync::LazyLock;

use regex::Regex;

use crate::model::TemplateFacts;

static DEFINE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\{\{-?\s*define\s+"([^"]+)""#).expect("the define pattern compiles"));

static REFERENCE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\{\{-?\s*(?:include|template)\s+"([^"]+)""#).expect("the reference pattern compiles")
});

/// Scans one template's text. Pure: the caller owns all file access.
pub fn scan(text: &str) -> TemplateFacts {
    let mut kinds = BTreeSet::new();
    let mut api_versions = BTreeSet::new();
    let mut defines = BTreeSet::new();
    let mut references = BTreeSet::new();
    for raw in text.split('\n') {
        let line = raw.trim_end_matches('\r');
        if let Some(kind) = top_level_value(line, "kind:") {
            kinds.insert(kind);
        }
        if let Some(api_version) = top_level_value(line, "apiVersion:") {
            api_versions.insert(api_version);
        }
        for captures in DEFINE.captures_iter(line) {
            defines.insert(captures[1].to_owned());
        }
        for captures in REFERENCE.captures_iter(line) {
            references.insert(captures[1].to_owned());
        }
    }
    TemplateFacts {
        kinds: kinds.into_iter().collect(),
        api_versions: api_versions.into_iter().collect(),
        defines: defines.into_iter().collect(),
        references: references.into_iter().collect(),
    }
}

/// The literal value of a column-zero `key:` line; none when the line is not that key, its value is templated, or
/// nothing is left once the trailing comment and the quotes go.
fn top_level_value(line: &str, key: &str) -> Option<String> {
    let mut value = line.strip_prefix(key)?;
    // A YAML comment after the value is not part of it.
    if let Some(comment) = value.find(" #") {
        value = &value[..comment];
    }
    let value = value.trim().trim_matches(['"', '\'']).trim();
    (!value.is_empty() && !value.contains("{{")).then(|| value.to_owned())
}
