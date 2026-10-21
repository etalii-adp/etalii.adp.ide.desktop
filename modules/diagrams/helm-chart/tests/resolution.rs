//! Every state of the three-state matcher, the alias and the stripped archive included (etalii.adp spec 030, task
//! T080): the standalone host's `DependencyResolution.Tests.cs` at `8b6ccfc`, with the same expectations, and the
//! archive name's version tail (`HelmChartReader.StripVersionTail`).

use adp_diagram_helm_chart::model::{ConditionState, DependencyDeclaration, VendoredEntry};
use adp_diagram_helm_chart::reader::strip_version_tail;
use adp_diagram_helm_chart::resolution::matched;

fn declared(name: &str, alias: Option<&str>) -> DependencyDeclaration {
    DependencyDeclaration {
        name: name.into(),
        alias: alias.map(str::to_owned),
        version_constraint: Some("1.0.0".into()),
        repository: Some("https://charts.example.com".into()),
        condition: None,
        state: ConditionState::None,
        line: 1,
    }
}

fn vendored(entry_name: &str, relative_path: &str, sealed: bool) -> VendoredEntry {
    VendoredEntry {
        entry_name: entry_name.into(),
        relative_path: relative_path.into(),
        sealed,
        chart_name: (!sealed).then(|| entry_name.to_owned()),
        chart_version: Some("1.0.0".into()),
        chart_type: "application".into(),
        template_count: 0,
        deeper_count: 0,
        failure: None,
    }
}

#[test]
fn a_vendored_directory_resolves_its_declaration() {
    let entry = vendored("redis", "charts/redis", false);
    let result = matched(&[declared("redis", None)], std::slice::from_ref(&entry));
    assert_eq!(result.dependencies.len(), 1);
    assert_eq!(result.dependencies[0].vendored.as_ref(), Some(&entry));
    assert!(result.undeclared.is_empty());
}

#[test]
fn an_aliased_dependency_still_matches_the_name_helm_vendors_under() {
    let result = matched(
        &[declared("redis", Some("cache"))],
        &[vendored("redis", "charts/redis", false)],
    );
    assert!(result.dependencies[0].is_resolved());
}

#[test]
fn an_entry_named_by_the_alias_also_matches() {
    let result = matched(
        &[declared("redis", Some("cache"))],
        &[vendored("cache", "charts/cache", false)],
    );
    assert!(result.dependencies[0].is_resolved());
}

#[test]
fn a_stripped_archive_name_matches() {
    let result = matched(
        &[declared("common", None)],
        &[vendored("common", "charts/common-2.31.4.tgz", true)],
    );
    assert!(result.dependencies[0].is_resolved());
}

#[test]
fn nothing_vendored_is_an_open_end_and_no_more() {
    let result = matched(&[declared("alertmanager", None)], &[]);
    assert!(!result.dependencies[0].is_resolved());
    assert!(result.undeclared.is_empty());
}

#[test]
fn a_vendored_entry_no_one_declares_is_undeclared() {
    let result = matched(
        &[declared("redis", None)],
        &[
            vendored("redis", "charts/redis", false),
            vendored("stray", "charts/stray", false),
        ],
    );
    assert_eq!(result.undeclared.len(), 1);
    assert_eq!(result.undeclared[0].entry_name, "stray");
}

#[test]
fn two_declarations_of_one_chart_both_resolve_to_the_same_entry() {
    let entry = vendored("redis", "charts/redis", false);
    let result = matched(
        &[declared("redis", Some("cache")), declared("redis", Some("queue"))],
        std::slice::from_ref(&entry),
    );
    assert_eq!(result.dependencies.len(), 2);
    assert!(result.dependencies.iter().all(|d| d.vendored.as_ref() == Some(&entry)));
    assert!(result.undeclared.is_empty());
}

#[test]
fn matching_is_ordinal_so_case_matters() {
    let result = matched(&[declared("Redis", None)], &[vendored("redis", "charts/redis", false)]);
    assert!(!result.dependencies[0].is_resolved());
    assert_eq!(result.undeclared.len(), 1);
}

#[test]
fn an_archives_version_tail_is_stripped_only_when_it_is_a_version() {
    assert_eq!(strip_version_tail("common-2.31.4.tgz"), "common");
    assert_eq!(strip_version_tail("kube-state-metrics-8.4.0.tgz"), "kube-state-metrics");
    assert_eq!(strip_version_tail("sealed-9.9.9.tgz"), "sealed");
    assert_eq!(strip_version_tail("my-chart.tgz"), "my-chart");
    assert_eq!(strip_version_tail("-1.0.0.tgz"), "-1.0.0");
}
