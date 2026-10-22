//! The agent activity diagram's colours in the light and the dark theme (etalii.adp spec 030, FR-045, task T087): the
//! tokens of `agent-activity-diagram.dis`'s `notation.theme`, light and dark, as the standalone host's `aad.css` at
//! `8b6ccfc` paints them. The canvas draws in the palette of the active theme and draws again when it switches.

use adp_theme::Kind;

/// The theme's tokens (`notation.theme`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub text: &'static str,
    pub border: &'static str,
    pub relation: &'static str,
    pub danger: &'static str,
}

pub const LIGHT: Palette = Palette {
    text: "#0f172a",
    border: "#94a3b8",
    relation: "#64748b",
    danger: "#dc2626",
};

pub const DARK: Palette = Palette {
    text: "#f1f5f9",
    border: "#64748b",
    relation: "#94a3b8",
    danger: "#f87171",
};

/// The palette of `kind`.
pub fn palette(kind: Kind) -> &'static Palette {
    match kind {
        Kind::Light => &LIGHT,
        Kind::Dark => &DARK,
    }
}

/// The fill of a kind of element (`aad.<kind>`) in `theme`.
pub fn fill(element: &str, theme: Kind) -> &'static str {
    match (element, theme) {
        ("project", Kind::Light) => "#dbeafe",
        ("project", Kind::Dark) => "#1e3a8a",
        ("specification", Kind::Light) => "#ffffff",
        ("specification", Kind::Dark) => "#1e293b",
        ("agent", Kind::Light) => "#ede9fe",
        ("agent", Kind::Dark) => "#4c1d95",
        ("location", Kind::Light) => "#fef3c7",
        ("location", Kind::Dark) => "#78350f",
        (_, Kind::Light) => "#dcfce7",
        (_, Kind::Dark) => "#14532d",
    }
}

/// A status's colour (`aad.<status>`) in `theme`: a specification's outline.
pub fn status_colour(status: &str, theme: Kind) -> &'static str {
    match (status, theme) {
        ("progressing", Kind::Light) => "#2563eb",
        ("progressing", Kind::Dark) => "#60a5fa",
        ("inputRequired", Kind::Light) => "#c2410c",
        ("inputRequired", Kind::Dark) => "#fb923c",
        ("finished", Kind::Light) => "#15803d",
        ("finished", Kind::Dark) => "#4ade80",
        ("archived", Kind::Light) => "#94a3b8",
        ("archived", Kind::Dark) => "#64748b",
        (_, Kind::Light) => "#64748b",
        (_, Kind::Dark) => "#94a3b8",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use adp_theme::Rgba;

    const ELEMENTS: [&str; 5] = ["project", "specification", "agent", "location", "environment"];
    const STATUSES: [&str; 5] = ["progressing", "inputRequired", "finished", "archived", "unknown"];

    fn colours(kind: Kind) -> Vec<&'static str> {
        let p = palette(kind);
        let mut all = vec![p.text, p.border, p.relation, p.danger];
        all.extend(ELEMENTS.iter().map(|e| fill(e, kind)));
        all.extend(STATUSES.iter().map(|s| status_colour(s, kind)));
        all
    }

    #[test]
    fn every_colour_has_a_value_in_both_themes_and_the_dark_ones_are_its_own() {
        let (light, dark) = (colours(Kind::Light), colours(Kind::Dark));
        for colour in light.iter().chain(&dark) {
            assert!(Rgba::parse(colour).is_some(), "{colour} is a colour");
        }
        for (at, element) in ELEMENTS.iter().enumerate() {
            assert_ne!(
                light[4 + at],
                dark[4 + at],
                "{element} has a fill of its own in the dark theme"
            );
        }
        assert_ne!(palette(Kind::Light), palette(Kind::Dark));
    }
}
