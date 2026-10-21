//! Text measured with the window's own font (etalii.adp spec 030, task T055): the [`TextMetrics`] a scene fits,
//! wraps and trims its labels with, implemented with egui's glyph advances.
//!
//! The standalone host measured with a fixed advance of 0.55 times the font size (`label/textMetrics.ts` at
//! `8b6ccfc`), because its backend sized boxes with the same estimate. The desktop host draws with a real font, so
//! a label is fitted to the width it is drawn at; a width is the sum of the glyphs' advances at the font size, in
//! diagram units, which the zoom scales like everything else. Widths are remembered per text and size, since a
//! scene asks for the same label many times.

use std::cell::RefCell;
use std::collections::HashMap;

use adp_scene::text::TextMetrics;
use eframe::egui::{self, FontFamily, FontId};

/// The font family every canvas text is set in.
pub const FAMILY: FontFamily = FontFamily::Proportional;

/// Widths measured with a context's fonts.
pub struct EguiMetrics {
    ctx: egui::Context,
    widths: RefCell<HashMap<(String, u64), f64>>,
}

impl EguiMetrics {
    /// Measures with `ctx`'s fonts, which exist once the context has begun its first frame.
    pub fn new(ctx: &egui::Context) -> Self {
        EguiMetrics {
            ctx: ctx.clone(),
            widths: RefCell::new(HashMap::new()),
        }
    }
}

impl TextMetrics for EguiMetrics {
    fn width(&self, text: &str, font_size: f64) -> f64 {
        let key = (text.to_owned(), font_size.to_bits());
        if let Some(width) = self.widths.borrow().get(&key) {
            return *width;
        }
        let font = FontId::new(font_size as f32, FAMILY);
        let width: f64 = self
            .ctx
            .fonts_mut(|fonts| text.chars().map(|c| fonts.glyph_width(&font, c) as f64).sum());
        self.widths.borrow_mut().insert(key, width);
        width
    }
}
