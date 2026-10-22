//! The Helm chart's colours in the light and the dark theme (etalii.adp spec 030, FR-045, task T087): the tokens of
//! `helm-chart.dis`'s `notation.theme`, light and dark, which are the standalone host's `--color-diagram-helm-*` of
//! `index.css` at `8b6ccfc`. The canvas draws in the palette of the active theme and draws again when it switches.

use adp_theme::Kind;

/// The theme's tokens (`notation.theme`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub text: &'static str,
    pub muted: &'static str,
    pub danger: &'static str,
    pub chart: &'static str,
    pub values: &'static str,
    pub template: &'static str,
    pub dependency: &'static str,
    pub subchart: &'static str,
    pub plain_fill: &'static str,
}

pub const LIGHT: Palette = Palette {
    text: "#0F172A",
    muted: "#64748B",
    danger: "#DC2626",
    chart: "#3D7BD9",
    values: "#2EA064",
    template: "#C48220",
    dependency: "#8E54C4",
    subchart: "#3498A0",
    plain_fill: "#64748B1F",
};

pub const DARK: Palette = Palette {
    text: "#F1F5F9",
    muted: "#94A3B8",
    danger: "#F87171",
    chart: "#7DB4F0",
    values: "#6ECD96",
    template: "#E8B05A",
    dependency: "#B88CE4",
    subchart: "#6ECDD4",
    plain_fill: "#94A3B81F",
};

/// The palette of the active theme, `kind`.
pub fn palette(kind: Kind) -> &'static Palette {
    match kind {
        Kind::Light => &LIGHT,
        Kind::Dark => &DARK,
    }
}
