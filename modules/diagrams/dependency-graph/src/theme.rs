//! The dependency graph's colours in the light and the dark theme (etalii.adp spec 030, FR-045, task T087).
//!
//! The canvas draws a dependency graph from `dependency-graph.dis`'s notation, whose theme declares its light tokens
//! only; without a dark mode, the canvas would paint a white surface under it in the dark theme. These are the tokens
//! this module gives the notation in the dark theme: the standalone host's dark `--color-surface`, `--color-text` and
//! `--color-text-muted` of `index.css` at `8b6ccfc`, which its stylesheet paints the graph with, and for a node's outline
//! the dark border the other notations give one (`#64748b`), since the dark `--color-border` (`#334155`) all but
//! vanishes on the dark surface. In the light theme the notation's own values stand.

use adp_theme::Kind;

/// The notation's tokens this module gives a value of its own in the active theme, `kind`.
pub fn tokens(kind: Kind) -> &'static [(&'static str, &'static str)] {
    match kind {
        Kind::Light => &[],
        Kind::Dark => &[
            ("color.surface", "#1E293B"),
            ("color.border", "#64748B"),
            ("color.text", "#F1F5F9"),
            ("color.text.muted", "#94A3B8"),
        ],
    }
}
