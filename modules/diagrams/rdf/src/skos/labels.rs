//! The one function that names every node of the scheme reading (etalii.adp spec 030, task T083), ported from the
//! standalone host's `Skos/SkosLabels.cs` and `Skos/_Model/SkosLabel.cs`, `SkosChosenLabel.cs` at `8b6ccfc`.
//!
//! Deterministic all the way down: the preference order over languages, and within a tag the lexicographically
//! smallest literal, so a file with duplicate preferred labels (SKOS S14) still renders the same on every open while
//! the validator reports it.

use crate::model::RdfTriple;

/// The language that heads the default preference order.
pub const DEFAULT_LANGUAGE: &str = "en";

/// Which SKOS labeling property stated a label (`SkosLabelSource`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SkosLabelSource {
    Preferred,
    Alternate,
    Hidden,
}

/// One label literal with the triple that stated it (`SkosLabel`). The language is lowercased; none for an untagged
/// literal. The triple is none only for a label made up by a test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkosLabel {
    pub source: SkosLabelSource,
    pub text: String,
    pub language: Option<String>,
    pub triple: Option<RdfTriple>,
}

impl SkosLabel {
    /// A label stated by no triple, for tests.
    pub fn new(source: SkosLabelSource, text: &str, language: Option<&str>) -> Self {
        SkosLabel {
            source,
            text: text.to_owned(),
            language: language.map(str::to_owned),
            triple: None,
        }
    }
}

/// What kind of name the chooser ended up with (`SkosLabelKind`); the payload carries it as its number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SkosLabelKind {
    Preferred = 0,
    Alternate = 1,
    IriFallback = 2,
}

/// The one name a node wears (`SkosChosenLabel`): its text, the chosen label's lowercased tag (empty for untagged and
/// the IRI fallback), and its kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkosChosenLabel {
    pub text: String,
    pub language_tag: String,
    pub kind: SkosLabelKind,
}

impl SkosChosenLabel {
    pub fn new(text: &str, language_tag: &str, kind: SkosLabelKind) -> Self {
        SkosChosenLabel {
            text: text.to_owned(),
            language_tag: language_tag.to_owned(),
            kind,
        }
    }
}

/// The name to display (`SkosLabels.Choose`): preferred labels in the display language, then the default language,
/// then untagged, then the smallest remaining tag; with no preferred label, an alternate by the same order; with none
/// at all, `iri_display`. Hidden labels never display.
pub fn choose(labels: &[SkosLabel], display_language: &str, iri_display: &str) -> SkosChosenLabel {
    let language = display_language.to_lowercase();
    pick(labels, SkosLabelSource::Preferred, &language, SkosLabelKind::Preferred)
        .or_else(|| pick(labels, SkosLabelSource::Alternate, &language, SkosLabelKind::Alternate))
        .unwrap_or_else(|| SkosChosenLabel::new(iri_display, "", SkosLabelKind::IriFallback))
}

fn pick(labels: &[SkosLabel], source: SkosLabelSource, language: &str, kind: SkosLabelKind) -> Option<SkosChosenLabel> {
    let candidates: Vec<&SkosLabel> = labels.iter().filter(|l| l.source == source).collect();
    if candidates.is_empty() {
        return None;
    }
    within(&candidates, Some(language), language, kind)
        .or_else(|| within(&candidates, Some(DEFAULT_LANGUAGE), DEFAULT_LANGUAGE, kind))
        .or_else(|| within(&candidates, None, "", kind))
        .or_else(|| {
            let tag = candidates.iter().filter_map(|l| l.language.as_deref()).min()?;
            within(&candidates, Some(tag), tag, kind)
        })
}

/// Within one tag, the lexicographically smallest literal.
fn within(
    candidates: &[&SkosLabel],
    language: Option<&str>,
    tag: &str,
    kind: SkosLabelKind,
) -> Option<SkosChosenLabel> {
    candidates
        .iter()
        .filter(|l| l.language.as_deref() == language)
        .map(|l| l.text.as_str())
        .min()
        .map(|text| SkosChosenLabel::new(text, tag, kind))
}
