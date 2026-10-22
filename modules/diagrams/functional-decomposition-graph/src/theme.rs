//! The functional decomposition graph's colours in the light and the dark theme (etalii.adp spec 030, FR-045, task
//! T087): the text, the border and the five fills of `functional-decomposition-graph.dis`'s `notation.theme`. The light
//! fills are the user's own hex; the dark ones are chosen against VS Code's dark text. The canvas draws in the palette
//! of the active theme and draws again when it switches.

use adp_theme::Kind;

/// The theme colours of the notation: the text, the border and each type's fill, light or dark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub text: &'static str,
    pub border: &'static str,
    pub fills: [(&'static str, &'static str); 5],
}

pub const LIGHT: Palette = Palette {
    text: "#0f172a",
    border: "#94a3b8",
    fills: [
        ("ui-element", "#aaed92"),
        ("data-element", "#ededed"),
        ("action", "#9edcfa"),
        ("function", "#86e6d9"),
        ("comment", "#fcf281"),
    ],
};

/// The dark fills keep the standalone host's hues and saturations with the lightness lowered to the lightest value that
/// reaches 5.0:1 against VS Code's dark `editor.foreground` (`#BBBEBF`), the text a tool is to take from the active theme (FR-045);
/// the standalone host's (`#2e7814`, `#696969`, `#086fa1`, `#187569`, `#736a03`) were chosen against `#f1f5f9` and
/// reach only 2.9:1 there (task T086, `adp-theme/tests/contrast.rs`).
pub const DARK: Palette = Palette {
    text: "#f1f5f9",
    border: "#64748b",
    fills: [
        ("ui-element", "#1f510e"),
        ("data-element", "#464646"),
        ("action", "#054b6d"),
        ("function", "#104f47"),
        ("comment", "#4e4702"),
    ],
};

impl Palette {
    /// The fill of `element_type`, a data element's for a type the notation does not name.
    pub fn fill(&self, element_type: &str) -> &'static str {
        self.fills
            .iter()
            .find(|(t, _)| *t == element_type)
            .or_else(|| self.fills.iter().find(|(t, _)| *t == "data-element"))
            .map(|(_, c)| *c)
            .unwrap_or(self.border)
    }
}

/// The palette of the active theme, `kind`.
pub fn palette(kind: Kind) -> &'static Palette {
    match kind {
        Kind::Light => &LIGHT,
        Kind::Dark => &DARK,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use adp_theme::Rgba;

    #[test]
    fn every_type_has_a_fill_of_its_own_in_both_themes() {
        for (at, (name, light)) in LIGHT.fills.iter().enumerate() {
            let (dark_name, dark) = DARK.fills[at];
            assert_eq!(*name, dark_name, "the two themes list the types in one order");
            assert_ne!(*light, dark, "{name} has a fill of its own in the dark theme");
            assert!(Rgba::parse(light).is_some() && Rgba::parse(dark).is_some());
        }
        assert_ne!(palette(Kind::Light).text, palette(Kind::Dark).text);
    }

    #[test]
    fn an_unknown_type_is_filled_as_a_data_element_in_the_active_theme() {
        assert_eq!(LIGHT.fill("unknown"), LIGHT.fill("data-element"));
        assert_eq!(DARK.fill("unknown"), DARK.fill("data-element"));
    }
}
