//! The SPARQL query diagram's colours in the light and the dark theme (etalii.adp spec 030, FR-045, task T087): the
//! tokens of `sparql-query.dis`'s `notation.theme`, light and dark. The canvas draws in the palette of the active theme
//! and draws again when it switches.

use adp_theme::Kind;

/// The theme's tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub surface: &'static str,
    pub text: &'static str,
    pub muted: &'static str,
    pub border: &'static str,
    pub primary: &'static str,
    pub warning: &'static str,
}

pub const LIGHT: Palette = Palette {
    surface: "#ffffff",
    text: "#1f1e1c",
    muted: "#6b6a65",
    border: "#8a8880",
    primary: "#2f6fb2",
    warning: "#b7791f",
};

pub const DARK: Palette = Palette {
    surface: "#1f1e1c",
    text: "#f1efe8",
    muted: "#a9a79f",
    border: "#b4b2a9",
    primary: "#2f6fb2",
    warning: "#b7791f",
};

/// The palette of the active theme, `kind`.
pub fn palette(kind: Kind) -> &'static Palette {
    match kind {
        Kind::Light => &LIGHT,
        Kind::Dark => &DARK,
    }
}
