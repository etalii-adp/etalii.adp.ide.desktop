//! The timeline's colours in the light and the dark theme (etalii.adp spec 030, FR-045, task T087).
//!
//! The canvas draws a timeline from `timeline.dis`'s notation, which declares no theme of its own and so takes DISL's
//! standard tokens (Appendix B.1), whose values are light ones; without a dark mode, the canvas would paint a white
//! surface under it in the dark theme. These are the tokens this module gives the notation in the dark theme: the
//! standard tokens' dark surface and border of DISL section 6.2, and for a moment the standalone host's
//! `--color-primary` (`timeline.css` at `8b6ccfc` fills a moment with it), which `index.css` keeps limegreen in the dark
//! theme. In the light theme the notation's values stand.

use adp_theme::Kind;

/// The notation's tokens this module gives a value of its own in the active theme, `kind`.
pub fn tokens(kind: Kind) -> &'static [(&'static str, &'static str)] {
    match kind {
        Kind::Light => &[],
        Kind::Dark => &[
            ("color.surface", "#1F1E1C"),
            ("color.border", "#B4B2A9"),
            ("color.accent", "#32CD32"),
        ],
    }
}
