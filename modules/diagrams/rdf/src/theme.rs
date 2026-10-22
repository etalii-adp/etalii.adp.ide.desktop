//! The RDF family's colours in the light and the dark theme (etalii.adp spec 030, FR-045, task T087): the tokens of
//! the family's specifications' `notation.theme` (`w3c-rdf.dis`, `w3c-owl.dis`, `w3c-skos.dis`, `w3c-shacl.dis`),
//! light and dark. The canvas draws in the palette of the active theme and draws again when it switches.

use adp_theme::Kind;

/// The theme's tokens (`notation.theme` of the family's specifications).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub surface: &'static str,
    pub text: &'static str,
    pub muted: &'static str,
    pub border: &'static str,
    pub outline: &'static str,
    pub primary: &'static str,
    pub warning: &'static str,
}

pub const LIGHT: Palette = Palette {
    surface: "#ffffff",
    text: "#0f172a",
    muted: "#64748b",
    border: "#e2e8f0",
    outline: "#8892a6",
    primary: "#32cd32",
    warning: "#b45309",
};

pub const DARK: Palette = Palette {
    surface: "#1e293b",
    text: "#f1f5f9",
    muted: "#94a3b8",
    border: "#334155",
    outline: "#8892a6",
    primary: "#32cd32",
    warning: "#fbbf24",
};

/// The palette of the active theme, `kind`.
pub fn palette(kind: Kind) -> &'static Palette {
    match kind {
        Kind::Light => &LIGHT,
        Kind::Dark => &DARK,
    }
}
