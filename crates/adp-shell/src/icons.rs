//! The icons the shell draws as glyphs (etalii.adp spec 036, T026, research R7): the Material Design Icons the standalone
//! host's Hierarchical Requirement Designer names at a7fdf03, from `@mdi/font` 7.4.47, the version its client pins. Its
//! font file is subset to these glyphs in `assets/materialdesignicons-subset.ttf`, with the package's licence beside it
//! (Apache 2.0 for the fonts), and installed with egui as the font family [`FAMILY`]; `assets/README.md` says how the
//! subset is made.

use eframe::egui::{self, FontData, FontDefinitions, FontFamily, FontId};

/// The subset font.
const FONT: &[u8] = include_bytes!("../assets/materialdesignicons-subset.ttf");

/// The name of the icons' font family.
pub const FAMILY: &str = "icons";

/// Every glyph of the subset, by the name the standalone host's class names give it, in name order.
pub const GLYPHS: [(&str, char); 40] = [
    ("mdi-alert-circle-outline", '\u{F05D6}'),
    ("mdi-alert-outline", '\u{F002A}'),
    ("mdi-arrow-expand-horizontal", '\u{F084E}'),
    ("mdi-bookmark-plus-outline", '\u{F00C4}'),
    ("mdi-bookmark-remove-outline", '\u{F137A}'),
    ("mdi-check", '\u{F012C}'),
    ("mdi-check-circle-outline", '\u{F05E1}'),
    ("mdi-checkbox-blank-outline", '\u{F0131}'),
    ("mdi-chevron-double-right", '\u{F013E}'),
    ("mdi-chevron-down", '\u{F0140}'),
    ("mdi-chevron-right", '\u{F0142}'),
    ("mdi-chevron-up", '\u{F0143}'),
    ("mdi-close", '\u{F0156}'),
    ("mdi-comment-multiple-outline", '\u{F0181}'),
    ("mdi-comment-plus-outline", '\u{F0183}'),
    ("mdi-comment-question-outline", '\u{F0186}'),
    ("mdi-comment-text-outline", '\u{F0189}'),
    ("mdi-delete-outline", '\u{F09E7}'),
    ("mdi-drag-vertical", '\u{F01DD}'),
    ("mdi-file-tree-outline", '\u{F13D2}'),
    ("mdi-filter-outline", '\u{F0233}'),
    ("mdi-filter-variant", '\u{F0236}'),
    ("mdi-folder-outline", '\u{F0256}'),
    ("mdi-folder-plus-outline", '\u{F0B9D}'),
    ("mdi-help-circle-outline", '\u{F0625}'),
    ("mdi-information-outline", '\u{F02FD}'),
    ("mdi-lightbulb-outline", '\u{F0336}'),
    ("mdi-link-variant-off", '\u{F033A}'),
    ("mdi-lock-outline", '\u{F0341}'),
    ("mdi-note-plus-outline", '\u{F039D}'),
    ("mdi-pencil-outline", '\u{F0CB6}'),
    ("mdi-playlist-plus", '\u{F0412}'),
    ("mdi-plus", '\u{F0415}'),
    ("mdi-plus-box-outline", '\u{F0704}'),
    ("mdi-restore", '\u{F099B}'),
    ("mdi-sort", '\u{F04BA}'),
    ("mdi-tag-plus-outline", '\u{F1222}'),
    ("mdi-target", '\u{F04FE}'),
    ("mdi-text-box-plus-outline", '\u{F0EAB}'),
    ("mdi-trash-can-outline", '\u{F0A7A}'),
];

/// The glyph of the icon `name` (`mdi-plus`), when the subset has it.
pub fn glyph(name: &str) -> Option<char> {
    GLYPHS.iter().find(|(n, _)| *n == name).map(|(_, c)| *c)
}

/// The icon `name` as text to draw in [`font`], or nothing for a name the subset does not have.
pub fn text(name: &str) -> String {
    glyph(name).map(String::from).unwrap_or_default()
}

/// The icons' font at `size` points.
pub fn font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(FAMILY.into()))
}

/// egui's own fonts, with the icons' family added.
pub fn definitions() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    fonts
        .font_data
        .insert(FAMILY.to_owned(), std::sync::Arc::new(FontData::from_static(FONT)));
    fonts
        .families
        .insert(FontFamily::Name(FAMILY.into()), vec![FAMILY.to_owned()]);
    fonts
}

/// Installs the icons' family with egui's own fonts.
pub fn install(ctx: &egui::Context) {
    ctx.set_fonts(definitions());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_glyph_the_designer_names_is_in_the_subset_font() {
        let ctx = egui::Context::default();
        install(&ctx);
        let mut output = ctx.run_ui(egui::RawInput::default(), |_| {});
        output.textures_delta.clear();
        for (name, glyph) in GLYPHS {
            assert!(
                ctx.fonts_mut(|f| f.has_glyph(&font(16.0), glyph)),
                "{name} is in the subset"
            );
        }
        assert!(
            !ctx.fonts_mut(|f| f.has_glyph(&font(16.0), '\u{F1000}')),
            "a glyph left out of the subset is not drawn"
        );
        let mut output = ctx.run_ui(egui::RawInput::default(), |_| {});
        output.textures_delta.clear();
    }

    #[test]
    fn a_name_is_looked_up_and_one_the_subset_lacks_draws_nothing() {
        assert_eq!(glyph("mdi-plus"), Some('\u{F0415}'));
        assert_eq!(text("mdi-unknown"), "");
        let mut names: Vec<&str> = GLYPHS.iter().map(|(n, _)| *n).collect();
        names.sort_unstable();
        assert_eq!(names, GLYPHS.iter().map(|(n, _)| *n).collect::<Vec<_>>());
    }
}
