//! The WCAG contrast between two colours, and how far apart two colours look (etalii.adp spec 030, FR-042, research
//! R9, task T086).
//!
//! Ported from the standalone host's `src/client/src/themeContrast.ts` at `8b6ccfc`, and the one implementation in the
//! repository, deliberately: an acceptance number stated as a ratio is only meaningful if everyone computes it the same
//! way, and two implementations of [`contrast_ratio`] are two acceptance criteria wearing one name. The colours are
//! taken as opaque; alpha is not composited, so a caller measures only colours that are drawn opaque.

use crate::Rgba;

/// One sRGB channel linearised as WCAG 2.x states it, with its `0.03928` threshold.
fn wcag_linear(channel: u8) -> f64 {
    let c = f64::from(channel) / 255.0;
    if c <= 0.03928 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// WCAG 2.x relative luminance: `0.2126 R + 0.7152 G + 0.0722 B` of the linearised channels.
pub fn relative_luminance(colour: Rgba) -> f64 {
    0.2126 * wcag_linear(colour.r) + 0.7152 * wcag_linear(colour.g) + 0.0722 * wcag_linear(colour.b)
}

/// WCAG 2.x contrast ratio, 1 to 21: `(lighter + 0.05) / (darker + 0.05)`. The order of the two colours does not
/// matter.
pub fn contrast_ratio(one: Rgba, other: Rgba) -> f64 {
    let (a, b) = (relative_luminance(one), relative_luminance(other));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// CIE L\*a\*b\* under D65, from sRGB linearised with the sRGB standard's `0.04045` threshold, as the standalone host's
/// supply-chain and Sankey checks compute it.
pub fn lab(colour: Rgba) -> [f64; 3] {
    let linear = |channel: u8| {
        let c = f64::from(channel) / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let (r, g, b) = (linear(colour.r), linear(colour.g), linear(colour.b));
    let f = |t: f64| {
        if t > 216.0 / 24389.0 {
            t.cbrt()
        } else {
            (24389.0 / 27.0 * t + 16.0) / 116.0
        }
    };
    let x = f((0.4124 * r + 0.3576 * g + 0.1805 * b) / 0.95047);
    let y = f(0.2126 * r + 0.7152 * g + 0.0722 * b);
    let z = f((0.0193 * r + 0.1192 * g + 0.9505 * b) / 1.08883);
    [116.0 * y - 16.0, 500.0 * (x - y), 200.0 * (y - z)]
}

/// How far apart two colours look: the CIE76 distance between their L\*a\*b\* values.
pub fn delta_e(one: Rgba, other: Rgba) -> f64 {
    let (a, b) = (lab(one), lab(other));
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgba(text: &str) -> Rgba {
        Rgba::parse(text).unwrap()
    }

    #[test]
    fn black_on_white_is_twenty_one_to_one_either_way_round() {
        assert_eq!(contrast_ratio(rgba("#000000"), rgba("#ffffff")), 21.0);
        assert_eq!(contrast_ratio(rgba("#ffffff"), rgba("#000000")), 21.0);
        assert_eq!(contrast_ratio(rgba("#777"), rgba("#777")), 1.0);
    }

    #[test]
    fn measures_against_the_text_colour_rather_than_white_as_the_standalone_host_recorded() {
        // `theme.contrast.test.ts`: `#7c7203` reads 4.4942 against `#f1f5f9` and 4.92 against white.
        let olive = rgba("#7c7203");
        assert_eq!(format!("{:.4}", contrast_ratio(olive, rgba("#f1f5f9"))), "4.4942");
        assert_eq!(format!("{:.2}", contrast_ratio(olive, rgba("#ffffff"))), "4.92");
    }

    #[test]
    fn white_and_black_are_a_hundred_apart_and_a_colour_none_from_itself() {
        assert!((delta_e(rgba("#ffffff"), rgba("#000000")) - 100.0).abs() < 0.01);
        assert_eq!(delta_e(rgba("#7c3aed"), rgba("#7c3aed")), 0.0);
    }
}
