//! The module's examples read as their readme says (the standalone host's `Examples.Tests.cs` at a7fdf03): what it
//! lists of them is there, and what it says they do not demonstrate is absent.

mod common;

use std::fs;

use adp_designer_hierarchical_requirement::model::{AssessmentKind, Document};
use common::{examples_folder, web_shop};
use regex::Regex;

/// How many items stand above and at `id`, counting itself.
fn depth(document: &Document, id: &str) -> usize {
    let mut depth = 0;
    let mut at = document.entries.iter().find(|e| e.id == id);
    while let Some(entry) = at {
        depth += 1;
        at = entry
            .parent_id
            .as_deref()
            .and_then(|parent| document.entries.iter().find(|e| e.id == parent));
    }
    depth
}

#[test]
fn the_web_shop_has_what_its_readme_lists() {
    let document = web_shop().document;

    // Six items at three levels with ten numbered requirements and one placeholder, and four assessments, one of
    // each kind.
    assert_eq!(6, document.entries.iter().filter(|e| e.is_item()).count());
    assert_eq!(11, document.entries.iter().filter(|e| !e.is_item()).count());
    assert_eq!(1, document.entries.iter().filter(|e| e.placeholder).count());
    assert_eq!(3, depth(&document, "ITEM-0003"));
    let mut kinds: Vec<AssessmentKind> = document.assessments.iter().map(|a| a.kind).collect();
    kinds.sort();
    assert_eq!(AssessmentKind::ALL.to_vec(), kinds);

    // The registration's lists, two collapsed entries and two stored filters.
    assert_eq!(9, document.tags.len());
    assert_eq!(2, document.entries.iter().filter(|e| e.collapsed).count());
    let filters: Vec<(&str, &str)> = document
        .filters
        .iter()
        .map(|f| (f.name.as_str(), f.query.as_str()))
        .collect();
    assert_eq!(
        vec![
            ("Open security", "#security @proposed sort:state"),
            ("Backend", "item:back* sort:number")
        ],
        filters
    );
    let colours = |members: &[adp_designer_hierarchical_requirement::model::Member]| {
        members.iter().map(|m| m.colour.clone()).collect::<Vec<_>>()
    };
    assert_eq!(vec!["blue", "purple", "brown"], colours(&document.categories));
    assert_eq!(vec!["gray", "green", "red", "blue"], colours(&document.states));
}

#[test]
fn the_web_shop_uses_the_three_markers_and_an_assessment_without_a_checkbox() {
    let text = fs::read_to_string(examples_folder().join("web-shop.md")).unwrap();
    let lines: Vec<&str> = text.lines().collect();

    for marker in ['-', '*', '+'] {
        assert!(
            lines
                .iter()
                .any(|line| line.trim_start().starts_with(&format!("{marker} REQ-"))),
            "no requirement with the marker {marker}"
        );
    }
    assert!(lines.iter().any(|line| line.starts_with("- Idea ")));
    assert!(lines.iter().any(|line| line.starts_with("- [ ] remark ")));
}

#[test]
fn the_web_shop_does_not_hold_what_its_readme_says_it_does_not() {
    let document = web_shop().document;
    let text = fs::read_to_string(examples_folder().join("web-shop.md")).unwrap();

    // Nothing wrong but the placeholder, and both headings written at level two as the binding names them.
    let codes: Vec<&str> = document.findings.iter().map(|f| f.code.as_str()).collect();
    assert_eq!(vec!["std.missingId"], codes);
    assert!(document.findings[0].message.contains("REQ-NEW"));
    let headings = Regex::new(r"(?m)^## (?:Requirements|Assessments)\r?$").unwrap();
    assert_eq!(2, headings.find_iter(&text).count());
}
