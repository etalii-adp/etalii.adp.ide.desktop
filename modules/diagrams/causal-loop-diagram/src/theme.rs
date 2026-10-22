//! The causal loop diagram's colours in the light and the dark theme (etalii.adp spec 030, FR-045, task T087).
//!
//! The canvas draws a causal loop diagram from `causal-loop-diagram.dis`'s notation, in its theme's mode for the active
//! theme. The notation's dark mode gives the text, the surface, the muted text and the border, but not the two loop
//! colours, so a loop's caption would keep its light blue or orange on the dark canvas. These are the tokens this
//! module adds over the notation's mode: in the dark theme the standalone host's dark `--color-loop-reinforcing` and
//! `--color-loop-balancing` of `index.css` at `8b6ccfc`, the original hues, which read on a dark ground; in the light
//! theme nothing, since the notation's own values are the light ones.

use adp_theme::Kind;

/// The notation's tokens this module gives a value of its own in the active theme, `kind`.
pub fn tokens(kind: Kind) -> &'static [(&'static str, &'static str)] {
    match kind {
        Kind::Light => &[],
        Kind::Dark => &[
            ("color.loop.reinforcing", "#4daafc"),
            ("color.loop.balancing", "#fb923c"),
        ],
    }
}
