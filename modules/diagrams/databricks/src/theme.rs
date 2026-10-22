//! The Databricks diagrams' colours in the light and the dark theme (etalii.adp spec 030, FR-045, task T087).
//!
//! The standalone host's `databricks.css` at `8b6ccfc` paints a bundle, a job and a pipeline with its host's shared
//! tokens, which `index.css` gives a light and a dark value; the three specifications declare none. The light palette
//! is `databricks.css`'s fallbacks; the dark one is `index.css`'s dark block for the same tokens (`--color-surface`,
//! `--color-text`, `--color-text-muted`, `--color-border-strong` for an arrow, `--color-success`, `--color-danger`), with
//! the border of a box the dark border the other notations give a node's outline (`#64748b`), since the dark
//! `--color-border` (`#334155`) all but vanishes on the dark surface. The canvas draws in the palette of the active
//! theme and draws again when it switches.

use adp_theme::Kind;

/// The shared tokens a Databricks diagram is painted with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub surface: &'static str,
    pub border: &'static str,
    pub text: &'static str,
    pub muted: &'static str,
    /// A plain arrow.
    pub edge: &'static str,
    /// An arrow taken when a condition holds.
    pub success: &'static str,
    /// An arrow taken when it does not.
    pub danger: &'static str,
}

pub const LIGHT: Palette = Palette {
    surface: "#ffffff",
    border: "#8892a6",
    text: "#1c2333",
    muted: "#5a6376",
    edge: "#64748b",
    success: "#15803d",
    danger: "#dc2626",
};

pub const DARK: Palette = Palette {
    surface: "#1e293b",
    border: "#64748b",
    text: "#f1f5f9",
    muted: "#94a3b8",
    edge: "#94a3b8",
    success: "#4ade80",
    danger: "#f87171",
};

/// The palette of the active theme, `kind`.
pub fn palette(kind: Kind) -> &'static Palette {
    match kind {
        Kind::Light => &LIGHT,
        Kind::Dark => &DARK,
    }
}
