//! The documentation of every keyword, from Appendix B of the ATD document (FR-051).

include!(concat!(env!("OUT_DIR"), "/keywords.rs"));

/// A keyword's documentation, without its Markdown code marks.
pub fn doc(keyword: &str) -> Option<&'static str> {
    KEYWORDS.iter().find(|(k, _)| *k == keyword).map(|(_, d)| *d)
}
