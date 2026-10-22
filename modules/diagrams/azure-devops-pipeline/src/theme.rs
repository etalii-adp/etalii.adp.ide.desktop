//! The Azure DevOps pipeline's colours in the light and the dark theme (etalii.adp spec 030, FR-045, task T087).
//!
//! The standalone host's `azure-pipeline.css` at `8b6ccfc` paints a pipeline with its host's shared tokens, which
//! `index.css` gives a light and a dark value; `azure-devops-pipeline.dis` declares only the light ones, its fallbacks.
//! The light palette is those; the dark one is `index.css`'s dark block for the same tokens: `--color-bg` for a stage,
//! `--color-surface` for a job, `--color-accent-subtle` for a deployment, `--color-border-strong` for an arrow,
//! `--color-danger` for a broken one. The canvas draws in the palette of the active theme and draws again when it
//! switches.

use adp_theme::Kind;

/// The shared tokens a pipeline is painted with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    /// A stage's ground (`--color-bg`).
    pub stage_fill: &'static str,
    /// A job's, a step's and a template's ground (`--color-surface`).
    pub surface: &'static str,
    pub border: &'static str,
    pub text: &'static str,
    pub muted: &'static str,
    /// A deployment job's ground, tinted towards the accent (`--color-accent-subtle`).
    pub deployment_fill: &'static str,
    /// A deployment job's edge (`--color-primary`).
    pub deployment_edge: &'static str,
    /// An arrow of what waits for what (`--color-border-strong`).
    pub edge: &'static str,
    /// An arrow to what is not there (`--color-danger`).
    pub broken: &'static str,
}

pub const LIGHT: Palette = Palette {
    stage_fill: "#f8fafc",
    surface: "#ffffff",
    border: "#e2e8f0",
    text: "#0f172a",
    muted: "#64748b",
    deployment_fill: "#f0fdf4",
    deployment_edge: "#32cd32",
    edge: "#64748b",
    broken: "#dc2626",
};

pub const DARK: Palette = Palette {
    stage_fill: "#0f172a",
    surface: "#1e293b",
    border: "#334155",
    text: "#f1f5f9",
    muted: "#94a3b8",
    deployment_fill: "#16351f",
    deployment_edge: "#32cd32",
    edge: "#94a3b8",
    broken: "#f87171",
};

/// The palette of the active theme, `kind`.
pub fn palette(kind: Kind) -> &'static Palette {
    match kind {
        Kind::Light => &LIGHT,
        Kind::Dark => &DARK,
    }
}
