//! The Knowledge designer's table colours in the light and the dark theme (etalii.adp spec 030, FR-045, task T087).
//!
//! The table draws on VS Code's colours, by role, which the active theme gives. An option's tag is the one thing VS Code
//! has no colour for: these are the standalone host's ten `--color-table-option-*` of `index.css` at `8b6ccfc`, pale
//! fills behind the text in the light theme and deep ones in the dark, where the text over them is light. A file names
//! an option's colour by its word, never by a value, so these twenty are the whole palette.

use adp_theme::Kind;

/// The words an option's colour is named by, besides `default`, which any other word is drawn as.
pub const OPTION_WORDS: [&str; 9] = [
    "gray", "brown", "orange", "yellow", "green", "blue", "purple", "pink", "red",
];

/// The fill of an option tag named `word` in the active theme, `theme`, light then dark on each line. Four dark fills
/// keep the standalone host's hue and saturation with the lightness lowered until VS Code's dark `foreground`
/// (`#BFBFBF`), the text the table writes on them, reaches 5.0:1: the standalone host's gray `#475569`, orange
/// `#7a4317`, yellow `#6b4d14` and green `#2b593f` were chosen against its own `#f1f5f9` and reach only 4.1:1 to 4.4:1
/// there (task T087, `adp-shell`'s `table::view` theme test).
pub fn option(word: &str, theme: Kind) -> &'static str {
    let (light, dark) = match word {
        "gray" => ("#e2e8f0", "#3c4859"),
        "brown" => ("#eedfd3", "#603b2c"),
        "orange" => ("#fde3c8", "#6c3b14"),
        "yellow" => ("#fdecc8", "#5d4311"),
        "green" => ("#dbeddb", "#264f38"),
        "blue" => ("#d3e5ef", "#28456c"),
        "purple" => ("#e8deee", "#492f64"),
        "pink" => ("#f5e0e9", "#69314c"),
        "red" => ("#ffe2dd", "#6e3630"),
        _ => ("#f1f5f9", "#334155"),
    };
    match theme {
        Kind::Light => light,
        Kind::Dark => dark,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use adp_theme::Rgba;

    #[test]
    fn every_option_word_has_a_fill_of_its_own_in_both_themes() {
        for word in OPTION_WORDS.iter().chain(&["default"]) {
            let (light, dark) = (option(word, Kind::Light), option(word, Kind::Dark));
            assert!(Rgba::parse(light).is_some() && Rgba::parse(dark).is_some(), "{word}");
            assert_ne!(light, dark, "{word} has a fill of its own in the dark theme");
        }
    }
}
