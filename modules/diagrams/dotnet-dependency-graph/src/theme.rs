//! The .NET dependency graph's colours in the light and the dark theme (etalii.adp spec 030, FR-045, task T087): the
//! tokens of `dotnet-dependency-graph.dis`'s `notation.theme`, light and dark. The canvas draws in the palette of the
//! active theme and draws again when it switches.

use adp_theme::Kind;

/// The theme's tokens (`notation.theme`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub project_fill: &'static str,
    pub project_stroke: &'static str,
    pub package_fill: &'static str,
    pub package_stroke: &'static str,
    pub label: &'static str,
    pub sublabel: &'static str,
    pub edge_project: &'static str,
    pub edge_package: &'static str,
    pub warning: &'static str,
}

pub const LIGHT: Palette = Palette {
    project_fill: "#3b6ea5",
    project_stroke: "#2b537c",
    package_fill: "#cbd5e1",
    package_stroke: "#94a3b8",
    label: "#ffffff",
    sublabel: "#dbe6f2",
    edge_project: "#64748b",
    edge_package: "#94a3b8",
    warning: "#b45309",
};

pub const DARK: Palette = Palette {
    project_fill: "#3b6ea5",
    project_stroke: "#7fb0e0",
    package_fill: "#4a5568",
    package_stroke: "#6b7280",
    label: "#ffffff",
    sublabel: "#cbd5e1",
    edge_project: "#94a3b8",
    edge_package: "#64748b",
    warning: "#fbbf24",
};

/// The palette of the active theme, `kind`.
pub fn palette(kind: Kind) -> &'static Palette {
    match kind {
        Kind::Light => &LIGHT,
        Kind::Dark => &DARK,
    }
}
