//! How a value is written on the canvas, and the colours a node or a flow may be (etalii.adp spec 030, task T077).
//!
//! Ported from the standalone host's `SankeyFormat.cs` and `SankeyColors.cs` at `8b6ccfc`. A format is any text with
//! `{value}` in it (`€{value}M`, `{value} TWh`), so money, energy and people read in their own units without the
//! module knowing what any of them are; a format without the placeholder is shown with the number before it. A colour
//! is a palette word, which the theme paints in both modes, or a hex colour, painted as written.

use crate::lines::grouped_number;

/// The placeholder a format writes the number in.
const PLACEHOLDER: &str = "{value}";

/// The number alone: what a document that states no format draws.
pub const PLAIN: &str = PLACEHOLDER;

/// `value` written through `format`.
pub fn write(format: &str, value: f64) -> String {
    let number = grouped_number(value);
    if format.is_empty() {
        return number;
    }
    if format.contains(PLACEHOLDER) {
        format.replace(PLACEHOLDER, &number)
    } else {
        format!("{number} {format}")
    }
}

/// What a node that states no colour is drawn in.
const DEFAULT_COLOR: &str = "grey";

/// The palette, in the order the property grid offers it.
pub const PALETTE: [&str; 12] = [
    "grey", "slate", "purple", "blue", "teal", "green", "lime", "yellow", "orange", "red", "pink", "brown",
];

fn is_palette_word(color: &str) -> bool {
    PALETTE.contains(&color)
}

/// Whether `color` is a colour written outright: `#rgb` or `#rrggbb`.
pub fn is_hex(color: &str) -> bool {
    color
        .strip_prefix('#')
        .is_some_and(|hex| (hex.len() == 3 || hex.len() == 6) && hex.bytes().all(|b| b.is_ascii_hexdigit()))
}

/// Whether `color` is a colour this module can paint, the empty default included.
pub fn is_known(color: &str) -> bool {
    color.is_empty() || is_palette_word(color) || is_hex(color)
}

/// The colour to paint, as the element carries it: the palette word, or the hex in lower case, and the other empty.
/// Anything else paints `fallback`, and the rules report it.
pub fn resolve(color: &str, fallback: &str) -> (String, String) {
    if is_palette_word(color) {
        return (color.to_owned(), String::new());
    }
    if is_hex(color) {
        return (String::new(), color.to_lowercase());
    }
    if is_palette_word(fallback) {
        (fallback.to_owned(), String::new())
    } else if is_hex(fallback) {
        (String::new(), fallback.to_lowercase())
    } else {
        (DEFAULT_COLOR.to_owned(), String::new())
    }
}

/// [`resolve`] with the default as the fallback.
pub fn resolve_or_default(color: &str) -> (String, String) {
    resolve(color, DEFAULT_COLOR)
}
