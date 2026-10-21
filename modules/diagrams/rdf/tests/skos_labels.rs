//! The label rule as a table (etalii.adp spec 030, task T083), ported from the standalone host's
//! `SkosLabels.Tests.cs` at `8b6ccfc`.

use adp_diagram_rdf::skos::labels::{SkosChosenLabel, SkosLabel, SkosLabelKind, SkosLabelSource, choose};

fn label(source: SkosLabelSource, text: &str, language: Option<&str>) -> SkosLabel {
    SkosLabel::new(source, text, language)
}

use SkosLabelSource::{Alternate, Hidden, Preferred};

#[test]
fn choose_follows_the_approved_preference_order() {
    let table: Vec<(&str, Vec<SkosLabel>, &str, &str, SkosLabelKind)> = vec![
        (
            "nl",
            vec![
                label(Preferred, "Tea", Some("en")),
                label(Preferred, "Thee", Some("nl")),
            ],
            "Thee",
            "nl",
            SkosLabelKind::Preferred,
        ),
        (
            "nl",
            vec![label(Preferred, "Tee", Some("de")), label(Preferred, "Tea", Some("en"))],
            "Tea",
            "en",
            SkosLabelKind::Preferred,
        ),
        (
            "nl",
            vec![label(Preferred, "Tee", Some("de")), label(Preferred, "Tea", None)],
            "Tea",
            "",
            SkosLabelKind::Preferred,
        ),
        (
            "nl",
            vec![label(Preferred, "Te", Some("fi")), label(Preferred, "Tee", Some("de"))],
            "Tee",
            "de",
            SkosLabelKind::Preferred,
        ),
        (
            "en",
            vec![
                label(Preferred, "Zebra", Some("en")),
                label(Preferred, "Aardvark", Some("en")),
            ],
            "Aardvark",
            "en",
            SkosLabelKind::Preferred,
        ),
        (
            "EN",
            vec![label(Preferred, "Tea", Some("en"))],
            "Tea",
            "en",
            SkosLabelKind::Preferred,
        ),
        (
            "nl",
            vec![
                label(Alternate, "Cuppa", Some("en")),
                label(Alternate, "Bakkie", Some("nl")),
            ],
            "Bakkie",
            "nl",
            SkosLabelKind::Alternate,
        ),
        (
            "nl",
            vec![
                label(Alternate, "Bakkie", Some("nl")),
                label(Preferred, "Chai", Some("hi")),
            ],
            "Chai",
            "hi",
            SkosLabelKind::Preferred,
        ),
        (
            "en",
            vec![label(Hidden, "misspellling", Some("en"))],
            "ex:tea",
            "",
            SkosLabelKind::IriFallback,
        ),
        ("en", vec![], "ex:tea", "", SkosLabelKind::IriFallback),
    ];
    for (display_language, labels, text, tag, kind) in table {
        assert_eq!(
            SkosChosenLabel::new(text, tag, kind),
            choose(&labels, display_language, "ex:tea"),
            "{display_language} {labels:?}"
        );
    }
}

#[test]
fn choose_is_deterministic_whatever_the_input_order() {
    let forward = vec![
        label(Preferred, "Zebra", Some("en")),
        label(Preferred, "Aardvark", Some("en")),
        label(Preferred, "Thee", Some("nl")),
    ];
    let backward: Vec<SkosLabel> = forward.iter().rev().cloned().collect();
    assert_eq!(choose(&forward, "en", "x"), choose(&backward, "en", "x"));
}

#[test]
fn the_chip_rule_is_data_not_client_logic() {
    let gap = choose(&[label(Preferred, "Tea", Some("en"))], "nl", "x");
    let untagged = choose(&[label(Preferred, "Tea", None)], "nl", "x");
    assert_eq!("en", gap.language_tag);
    assert_eq!("", untagged.language_tag);
}
