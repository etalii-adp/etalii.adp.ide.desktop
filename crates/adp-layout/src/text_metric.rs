//! How wide a text is estimated to be (etalii.adp spec 030, FR-018), ported from the standalone host's
//! `EtAlii.Adp.Layout/TextMetric.cs` at `8b6ccfc` (backend-centralization R10).
//!
//! A text's width is its characters times the font size times the average advance. The standalone host makes this
//! estimate on both of its tiers, because its backend sizes a box for a text and its client draws the text inside it
//! without measuring again; the values are the ones its measuring modules (ansible-structure, c4, causal-loop and
//! mindmap) share: 14 and 0.55. The desktop host keeps the same estimate so it sizes every box as the standalone host
//! does.
//!
//! This is the metric and nothing else: the padding a module puts around a text, and the minimum and maximum it clamps
//! the box to, stay that module's own declared values (R10.2).
//!
//! A character is a UTF-16 code unit, which is what .NET's `string.Length` and JavaScript's `length` count, so a
//! character outside the Basic Multilingual Plane counts as two. The golden fixture is the standalone host's
//! `src/fixtures/cross-tier/text-metric.json`, copied to `tests/fixtures/`.

/// The font size the measuring modules lay out with, in canvas units.
pub const DEFAULT_FONT_SIZE: f64 = 14.0;

/// The estimated width of one character, as a fraction of the font size.
pub const AVERAGE_ADVANCE: f64 = 0.55;

/// The estimated width of `text` at `font_size`, with no padding, minimum or maximum applied.
pub fn width_of(text: &str, font_size: f64) -> f64 {
    text.encode_utf16().count() as f64 * font_size * AVERAGE_ADVANCE
}

/// The estimated width of `text` at [`DEFAULT_FONT_SIZE`].
pub fn width_at_default_size(text: &str) -> f64 {
    width_of(text, DEFAULT_FONT_SIZE)
}
