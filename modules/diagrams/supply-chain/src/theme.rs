//! The supply chain's colours in the light and the dark theme (etalii.adp spec 030, FR-045, task T087): the tokens of
//! `supply-chain.dis`'s `notation.theme`, light and dark, as the standalone host's `supply-chain.css` and `index.css` at
//! `8b6ccfc` paint them. The seven stage hues are one set for both themes, each a header behind white text; the text,
//! the surface, the border, the flows and a group's ground take the theme's own. The canvas draws in the palette of the
//! active theme and draws again when it switches.

use adp_theme::Kind;

/// The header colour of `stage` (`color.supply.<stage>`), the same in both themes.
pub fn stage_colour(stage: &str) -> &'static str {
    match stage {
        "Source" => "#7c4a1e",
        "Processor" => "#b91c1c",
        "Producer" => "#1d4ed8",
        "Integrator" => "#0f766e",
        "Hub" => "#475569",
        "Outlet" => "#15803d",
        _ => "#be185d",
    }
}

/// The theme's tokens (`notation.theme`) besides the stage hues.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub text: &'static str,
    pub muted: &'static str,
    pub surface: &'static str,
    pub border: &'static str,
    /// A flow's line and arrowhead (`color.supply.flow`).
    pub flow: &'static str,
    /// What is written on a stage's header (`color.supply.on-stage`).
    pub on_stage: &'static str,
    /// A group's ground (`color.supply.group-fill`).
    pub group_fill: &'static str,
}

pub const LIGHT: Palette = Palette {
    text: "#0f172a",
    muted: "#64748b",
    surface: "#ffffff",
    border: "#e2e8f0",
    flow: "#64748b",
    on_stage: "#ffffff",
    group_fill: "#0000000f",
};

pub const DARK: Palette = Palette {
    text: "#f1f5f9",
    muted: "#94a3b8",
    surface: "#1e293b",
    border: "#334155",
    flow: "#94a3b8",
    on_stage: "#ffffff",
    group_fill: "#ffffff0f",
};

/// The palette of the active theme, `kind`.
pub fn palette(kind: Kind) -> &'static Palette {
    match kind {
        Kind::Light => &LIGHT,
        Kind::Dark => &DARK,
    }
}
