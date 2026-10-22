//! The Ansible structure's colours in the light and the dark theme (etalii.adp spec 030, FR-045, task T087): the tokens
//! of `ansible-structure.dis`'s `notation.theme`, light and dark, with the six hues a play's tasks are told apart by.
//! The canvas draws in the palette of the active theme and draws again when it switches.

use adp_theme::Kind;

/// The theme's tokens (`notation.theme`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub surface: &'static str,
    pub raised: &'static str,
    pub text: &'static str,
    pub muted: &'static str,
    pub border: &'static str,
    pub primary: &'static str,
    pub warning: &'static str,
    pub hues: [&'static str; 6],
}

pub const LIGHT: Palette = Palette {
    surface: "#ffffff",
    raised: "#0000000f",
    text: "#1f1e1c",
    muted: "#6b6a65",
    border: "#8a8880",
    primary: "#2f6fb2",
    warning: "#b7791f",
    hues: ["#4f8cc9", "#57a773", "#c96f4f", "#8a6fc9", "#c9a84f", "#4fb5c9"],
};

pub const DARK: Palette = Palette {
    surface: "#1f1e1c",
    raised: "#ffffff0f",
    text: "#f1efe8",
    muted: "#a9a79f",
    border: "#b4b2a9",
    primary: "#2f6fb2",
    warning: "#b7791f",
    hues: ["#7fb0e0", "#7fc79a", "#e0947a", "#ab96e0", "#e0c87f", "#7fd0e0"],
};

/// The palette of the active theme, `kind`.
pub fn palette(kind: Kind) -> &'static Palette {
    match kind {
        Kind::Light => &LIGHT,
        Kind::Dark => &DARK,
    }
}
