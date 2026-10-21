//! Text measurement and fitting (the standalone host's `canvas/label/textMetrics.ts` at 8b6ccfc): the one metric
//! every width estimate, wrap and ellipsis of a scene comes from.
//!
//! The standalone host measures with a fixed average advance of 0.55 times the font size per character, the
//! backend's metric, so a box sized for a text is fitted by the same estimate. This host takes the measurement
//! from [`TextMetrics`], which the shell implements with its real font; [`EstimatedMetrics`] is the standalone
//! metric, the deterministic fallback the tests run with. Fitting is written against widths rather than the
//! standalone's character capacities, and gives exactly its answers under [`EstimatedMetrics`]: a line fits a width
//! when its measured width does, and is cut to the longest prefix that fits with its ellipsis. Characters are Unicode
//! scalar values where the standalone counts UTF-16 units; the two agree outside the astral planes.

/// The average advance of one character, as a fraction of the font size.
pub const AVERAGE_ADVANCE: f64 = 0.55;

/// The size an element's label is drawn at when nothing says otherwise (`.canvas-node-label` in the standalone
/// host's `canvas.css`). DISL's default font size is 14 (section 6.5); the standalone draws 12, and this host does.
pub const LABEL_FONT_SIZE: f64 = 12.0;

/// What a box keeps free inside its outline on one line of fitted text.
const FIT_PADDING: f64 = 8.0;

/// The tolerance for floating point alone: a width made for n characters must hold n.
const TOLERANCE: f64 = 1e-9;

/// How wide a text is drawn: what the shell measures with its font.
pub trait TextMetrics {
    /// The width of `text` on one line at `font_size`, in diagram units.
    fn width(&self, text: &str, font_size: f64) -> f64;
}

/// The standalone host's estimate: every character advances [`AVERAGE_ADVANCE`] times the font size.
#[derive(Debug, Clone, Copy, Default)]
pub struct EstimatedMetrics;

impl TextMetrics for EstimatedMetrics {
    fn width(&self, text: &str, font_size: f64) -> f64 {
        text.chars().count() as f64 * font_size * AVERAGE_ADVANCE
    }
}

/// The width a string is estimated to take at a font size (`widthOf`).
pub fn width_of(text: &str, font_size: f64) -> f64 {
    EstimatedMetrics.width(text, font_size)
}

/// Whether `text` fits `width` at `font_size`.
pub fn fits(metrics: &dyn TextMetrics, text: &str, width: f64, font_size: f64) -> bool {
    metrics.width(text, font_size) <= width + TOLERANCE * font_size.max(1.0)
}

/// The first `count` characters of `text`.
pub fn prefix(text: &str, count: usize) -> &str {
    match text.char_indices().nth(count) {
        Some((at, _)) => &text[..at],
        None => text,
    }
}

/// The text cut to fit `width`, its last character an ellipsis where anything was cut; an ellipsis alone when not
/// even one character and the ellipsis fit (`fitToCapacity`, measured).
pub fn fit_within(metrics: &dyn TextMetrics, text: &str, width: f64, font_size: f64) -> String {
    if fits(metrics, text, width, font_size) {
        return text.to_owned();
    }
    let count = text.chars().count();
    (1..count)
        .rev()
        .map(|kept| format!("{}…", prefix(text, kept)))
        .find(|candidate| fits(metrics, candidate, width, font_size))
        .unwrap_or_else(|| "…".to_owned())
}

/// The text as drawn in a box `width` wide (`fitToWidth`): whole when it fits, cut with an ellipsis when it does
/// not. The padding comes off first, so text sized by its width plus that padding is never trimmed.
pub fn fit_to_width(metrics: &dyn TextMetrics, text: &str, width: f64, font_size: f64) -> String {
    fit_within(metrics, text, (width - FIT_PADDING).max(0.0), font_size)
}

/// A line with more text after it that did not fit, marked so with an ellipsis within the same width
/// (`continuedWithin`).
pub fn continued_within(metrics: &dyn TextMetrics, line: &str, width: f64, font_size: f64) -> String {
    let marked = format!("{line}…");
    if fits(metrics, &marked, width, font_size) {
        return marked;
    }
    let count = line.chars().count();
    (0..count)
        .rev()
        .map(|kept| format!("{}…", prefix(line, kept)))
        .find(|candidate| fits(metrics, candidate, width, font_size))
        .unwrap_or_else(|| "…".to_owned())
}

/// The text broken into lines no wider than `width`, at spaces and at explicit newlines (`wrappedLines` of
/// `labels.ts`). A word longer than a line is broken where it reaches the width rather than left to overflow, and
/// at least one character goes on every line, so wrapping always advances.
pub fn wrap(metrics: &dyn TextMetrics, text: &str, width: f64, font_size: f64) -> Vec<String> {
    let fit = |candidate: &str| fits(metrics, candidate, width, font_size);
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut current = String::new();
        for word in paragraph.split(' ') {
            let mut remaining = word.to_owned();
            while !fit(&remaining) {
                // A word that cannot fit any line at all: break it where it reaches the width.
                let count = remaining.chars().count();
                let kept = (1..count)
                    .rev()
                    .find(|kept| fit(prefix(&remaining, *kept)))
                    .unwrap_or(1);
                if kept >= count {
                    break;
                }
                if !current.is_empty() {
                    lines.push(std::mem::take(&mut current));
                }
                lines.push(prefix(&remaining, kept).to_owned());
                remaining = remaining.chars().skip(kept).collect();
            }
            let candidate = if current.is_empty() {
                remaining.clone()
            } else {
                format!("{current} {remaining}")
            };
            if fit(&candidate) {
                current = candidate;
                continue;
            }
            if !current.is_empty() {
                lines.push(current);
            }
            current = remaining;
        }
        lines.push(current);
    }
    lines
}

#[cfg(test)]
mod tests {
    //! The standalone host's `textMetrics.test.ts`, with the same expectations under the estimate.
    use super::*;

    const M: EstimatedMetrics = EstimatedMetrics;

    /// How many characters of the estimate fit a width (`capacityOf`): at least one.
    fn capacity_of(width: f64, font_size: f64) -> usize {
        ((width / (font_size * AVERAGE_ADVANCE) + 1e-9).floor() as usize).max(1)
    }

    #[test]
    fn the_estimate_is_characters_times_the_size_times_the_advance() {
        assert!((width_of("abcd", 12.0) - 26.4).abs() < 1e-9);
        assert_eq!(0.0, width_of("", 12.0));
    }

    #[test]
    fn a_width_made_for_n_characters_holds_n() {
        for n in 1..40 {
            assert_eq!(n, capacity_of(width_of(&"x".repeat(n), 12.0), 12.0));
        }
        assert_eq!(1, capacity_of(0.0, 12.0));
    }

    #[test]
    fn text_sized_by_its_width_plus_the_padding_is_never_trimmed() {
        for text in ["a", "Discovery", "A rather longer label than most"] {
            let width = width_of(text, LABEL_FONT_SIZE) + 8.0;
            assert_eq!(text, fit_to_width(&M, text, width, LABEL_FONT_SIZE));
        }
    }

    #[test]
    fn a_text_one_character_too_wide_is_trimmed_inside_the_box() {
        for font_size in [10.0, 12.0, 14.0] {
            for text in ["Discovery", "A rather longer label than most"] {
                let width = width_of(text, font_size) + 8.0 - font_size * AVERAGE_ADVANCE;
                let fitted = fit_to_width(&M, text, width, font_size);
                assert!(fitted.ends_with('…'), "{text} at {font_size}");
                assert!(
                    width_of(&fitted, font_size) <= width - 8.0 + 1e-9,
                    "{text} at {font_size}"
                );
            }
        }
    }

    #[test]
    fn a_text_is_cut_to_a_capacity_with_the_ellipsis_inside_it() {
        let at = |n: usize| width_of(&"x".repeat(n), 12.0);
        assert_eq!("abcdef", fit_within(&M, "abcdef", at(6), 12.0));
        assert_eq!("abc…", fit_within(&M, "abcdef", at(4), 12.0));
        assert_eq!("…", fit_within(&M, "abcdef", at(1), 12.0));
    }

    #[test]
    fn text_that_does_not_fit_ends_in_an_ellipsis_within_the_width() {
        // 6.6 per character at 12: 40 - 8 = 32 holds four characters, the last the ellipsis.
        assert_eq!("abc…", fit_to_width(&M, "abcdefgh", 40.0, 12.0));
        assert_eq!("…", fit_to_width(&M, "abcdefgh", 8.0, 12.0));
    }

    #[test]
    fn a_continued_line_is_marked_within_its_capacity() {
        let width = width_of("abcd", 12.0);
        assert_eq!("ab…", continued_within(&M, "ab", width, 12.0));
        assert_eq!("abc…", continued_within(&M, "abcd", width, 12.0));
    }

    #[test]
    fn wrapping_breaks_at_spaces_newlines_and_inside_a_word_too_long_for_a_line() {
        let width = width_of("abcdef", 12.0);
        assert_eq!(vec!["ab cd", "efg"], wrap(&M, "ab cd efg", width, 12.0));
        assert_eq!(vec!["ab", "cd"], wrap(&M, "ab\ncd", width, 12.0));
        assert_eq!(vec!["abcdef", "ghij"], wrap(&M, "abcdefghij", width, 12.0));
        assert_eq!(vec!["x", "abcdef", "gh"], wrap(&M, "x abcdefgh", width, 12.0));
    }
}
