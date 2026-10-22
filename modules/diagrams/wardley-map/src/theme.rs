//! The Wardley map's colours in the light and the dark theme (etalii.adp spec 030, FR-045, task T087).
//!
//! The standalone host's `wardley.css` at `8b6ccfc` paints a map with its host's surface, text and muted text, which
//! `wardley-map.dis`'s `notation.theme` stands in for, light and dark: a component's mark is filled with the surface and
//! an anchor with the text, a dependency, a stage label and a decorator line are muted. The light palette is the one
//! this host drew before it had a dark one; the dark one is the notation's dark mode. The stages' bands are drawn in
//! one grey at a faint opacity, which reads on either ground. The canvas draws in the palette of the active theme and
//! draws again when it switches.

use adp_theme::Kind;

/// The tokens a Wardley map is painted with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    /// A component's and an annotation's ground (`color.surface`).
    pub surface: &'static str,
    /// An anchor's fill (`color.text`).
    pub text: &'static str,
    /// A dependency, a stage label and a decorator line (`color.text.muted`).
    pub muted: &'static str,
    /// The stages' bands, drawn at each stage's opacity.
    pub band: &'static str,
}

pub const LIGHT: Palette = Palette {
    surface: "#ffffff",
    text: "#111827",
    muted: "#6b7280",
    band: "#9ca3af",
};

pub const DARK: Palette = Palette {
    surface: "#1f1e1c",
    text: "#f1efe8",
    muted: "#a8a69e",
    band: "#9ca3af",
};

/// The palette of the active theme, `kind`.
pub fn palette(kind: Kind) -> &'static Palette {
    match kind {
        Kind::Light => &LIGHT,
        Kind::Dark => &DARK,
    }
}
