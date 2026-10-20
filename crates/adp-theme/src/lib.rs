//! VS Code's default light and dark themes, value for value (etalii.adp spec 030, FR-044, FR-045, research R9,
//! contracts/vscode-theme.md, tasks T019 and T020).
//!
//! The colours are the ones `tools/extract-vscode-theme.py` extracted into `vscode/dark.json` and `vscode/light.json`
//! from the VS Code release in `vscode/SOURCE.md`; the sizes are those of `vscode/sizes.md`. The shell reads every
//! colour, stroke width and font size from a [`Theme`]; this crate knows no UI toolkit.

use std::collections::BTreeMap;
use std::fmt;

/// A colour with alpha, as VS Code writes it: `#RGB`, `#RGBA`, `#RRGGBB` or `#RRGGBBAA`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub fn parse(text: &str) -> Option<Self> {
        let hex = text.strip_prefix('#')?;
        let digit = |i: usize| u8::from_str_radix(&hex[i..i + 1], 16).ok().map(|d| d * 17);
        let pair = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
        if !hex.is_ascii() {
            return None;
        }
        match hex.len() {
            3 => Some(Self {
                r: digit(0)?,
                g: digit(1)?,
                b: digit(2)?,
                a: 255,
            }),
            4 => Some(Self {
                r: digit(0)?,
                g: digit(1)?,
                b: digit(2)?,
                a: digit(3)?,
            }),
            6 => Some(Self {
                r: pair(0)?,
                g: pair(2)?,
                b: pair(4)?,
                a: 255,
            }),
            8 => Some(Self {
                r: pair(0)?,
                g: pair(2)?,
                b: pair(4)?,
                a: pair(6)?,
            }),
            _ => None,
        }
    }
}

impl fmt::Display for Rgba {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.a == 255 {
            write!(f, "#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
        } else {
            write!(f, "#{:02X}{:02X}{:02X}{:02X}", self.r, self.g, self.b, self.a)
        }
    }
}

/// Light or dark, following the system setting (FR-042).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Light,
    Dark,
}

/// The workbench sizes in logical pixels, from `vscode/sizes.md`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sizes {
    pub workbench_font: f32,
    /// The editor's font size on this platform: 12 on macOS, 14 elsewhere, as VS Code's `EDITOR_FONT_DEFAULTS`.
    pub editor_font: f32,
    pub row_height: f32,
    pub tab_height: f32,
    pub title_bar_height: f32,
    pub status_bar_height: f32,
    pub activity_bar_width: f32,
    pub focus_outline_width: f32,
    pub border_width: f32,
}

impl Sizes {
    pub const VSCODE: Sizes = Sizes {
        workbench_font: 13.0,
        editor_font: if cfg!(target_os = "macos") { 12.0 } else { 14.0 },
        row_height: 22.0,
        tab_height: 35.0,
        title_bar_height: 35.0,
        status_bar_height: 22.0,
        activity_bar_width: 48.0,
        focus_outline_width: 1.0,
        border_width: 1.0,
    };
}

/// One token colour rule of the editor's syntax highlighting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenColor {
    pub scopes: Vec<String>,
    pub foreground: Option<Rgba>,
    pub font_style: String,
}

/// One of VS Code's default themes.
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub name: String,
    pub kind: Kind,
    /// The VS Code release the values come from.
    pub vscode: String,
    colors: BTreeMap<String, Rgba>,
    pub token_colors: Vec<TokenColor>,
    pub sizes: Sizes,
}

const DARK: &str = include_str!("../vscode/dark.json");
const LIGHT: &str = include_str!("../vscode/light.json");

impl Theme {
    pub fn dark() -> Self {
        Self::from_json(DARK, Kind::Dark)
    }

    pub fn light() -> Self {
        Self::from_json(LIGHT, Kind::Light)
    }

    pub fn for_kind(kind: Kind) -> Self {
        match kind {
            Kind::Dark => Self::dark(),
            Kind::Light => Self::light(),
        }
    }

    /// The colour VS Code gives this id, such as `sideBar.background`; `None` when the theme leaves it to VS Code's
    /// built-in default, which the caller then names explicitly.
    pub fn color(&self, id: &str) -> Option<Rgba> {
        self.colors.get(id).copied()
    }

    /// The colour of the first of `ids` the theme defines, so a role can fall back the way VS Code's registry does.
    pub fn color_of(&self, ids: &[&str]) -> Option<Rgba> {
        ids.iter().find_map(|id| self.color(id))
    }

    pub fn colors(&self) -> impl Iterator<Item = (&str, Rgba)> {
        self.colors.iter().map(|(id, c)| (id.as_str(), *c))
    }

    /// The raw extracted document of a kind, for the checks against VS Code's files.
    pub fn source(kind: Kind) -> &'static str {
        match kind {
            Kind::Dark => DARK,
            Kind::Light => LIGHT,
        }
    }

    fn from_json(json: &str, kind: Kind) -> Self {
        let document: serde_json::Value = serde_json::from_str(json).expect("the extracted theme is JSON");
        let text = |key: &str| document[key].as_str().unwrap_or_default().to_owned();
        let colors = document["colors"]
            .as_object()
            .map(|map| {
                map.iter()
                    .filter_map(|(id, value)| Some((id.clone(), Rgba::parse(value.as_str()?)?)))
                    .collect()
            })
            .unwrap_or_default();
        let token_colors = document["tokenColors"]
            .as_array()
            .map(|rules| rules.iter().map(token_color).collect())
            .unwrap_or_default();
        Self {
            name: text("name"),
            kind,
            vscode: text("vscode"),
            colors,
            token_colors,
            sizes: Sizes::VSCODE,
        }
    }
}

fn token_color(rule: &serde_json::Value) -> TokenColor {
    let scopes = match &rule["scope"] {
        serde_json::Value::String(s) => s.split(',').map(|s| s.trim().to_owned()).collect(),
        serde_json::Value::Array(a) => a.iter().filter_map(|s| s.as_str().map(str::to_owned)).collect(),
        _ => Vec::new(),
    };
    let settings = &rule["settings"];
    TokenColor {
        scopes,
        foreground: settings["foreground"].as_str().and_then(Rgba::parse),
        font_style: settings["fontStyle"].as_str().unwrap_or_default().to_owned(),
    }
}
