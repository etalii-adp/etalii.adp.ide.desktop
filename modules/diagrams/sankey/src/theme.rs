//! The Sankey diagram's colours in the light and the dark theme (etalii.adp spec 030, FR-045, task T087): the twelve
//! palette words of `sankey.dis`'s `notation.theme`, light and dark, as the standalone host's `index.css` at `8b6ccfc`
//! declares them, and the colour a node's note is written in. A hex colour a document states is its own and is painted
//! as written in both themes. The canvas draws in the colours of the active theme and draws again when it switches.

use adp_theme::Kind;

/// A palette word's colour (`color.sankey.<word>`) in `theme`, light then dark on each line: the dark values lift each
/// word so the bar, the band and the label still read on the dark canvas, where the light ones measured 2.3:1 to 3.8:1
/// on VS Code's dark `editor.background` (task T086, `adp-theme/tests/contrast.rs`). A word the palette does not have
/// is grey.
pub fn palette(word: &str, theme: Kind) -> &'static str {
    let (light, dark) = match word {
        "slate" => ("#2f5b8a", "#8fb0d4"),
        "purple" => ("#86198f", "#e879f9"),
        "blue" => ("#1d4ed8", "#60a5fa"),
        "teal" => ("#0f766e", "#2dd4bf"),
        "green" => ("#15803d", "#4ade80"),
        "lime" => ("#4d7c0f", "#a3e635"),
        "yellow" => ("#a16207", "#facc15"),
        "orange" => ("#c2410c", "#fb923c"),
        "red" => ("#a3151a", "#f87171"),
        "pink" => ("#be185d", "#f472b6"),
        "brown" => ("#7c4a1e", "#d6a26f"),
        _ => ("#666666", "#a8a8a8"),
    };
    match theme {
        Kind::Light => light,
        Kind::Dark => dark,
    }
}

/// The colour a node's note is written in (`color.text.muted`), in `theme`: the light one, `#6b7280`, measured below
/// 4.5:1 on VS Code's dark `editor.background`, so the dark theme takes the notation's dark muted text.
pub fn note(theme: Kind) -> &'static str {
    match theme {
        Kind::Light => "#6b7280",
        Kind::Dark => "#94a3b8",
    }
}
