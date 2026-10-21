//! Where an inline editor opens over a label (the standalone host's `canvas/label/labelPlacement.ts` at 8b6ccfc):
//! pure arithmetic over a box given by its centre, for the four shapes a label takes on a canvas.

use crate::text::{LABEL_FONT_SIZE, TextMetrics};

/// A box by its centre and size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LabelBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Where an editor opens: its top-left corner, its size and the text it opens on.
#[derive(Debug, Clone, PartialEq)]
pub struct LabelPlacement {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub text: String,
}

/// The floor for a label with no box of its own, and the height of one line of text.
const MINIMUM_ESTIMATED_WIDTH: f64 = 80.0;
const TEXT_LABEL_HEIGHT: f64 = 16.0;

fn estimated(metrics: &dyn TextMetrics, text: &str, measured: Option<f64>) -> f64 {
    measured.unwrap_or_else(|| metrics.width(text, LABEL_FONT_SIZE).max(MINIMUM_ESTIMATED_WIDTH))
}

/// A label filling its element's box.
pub fn centred_label_placement(box_: LabelBox, text: &str) -> LabelPlacement {
    LabelPlacement {
        x: box_.x - box_.width / 2.0,
        y: box_.y - box_.height / 2.0,
        width: box_.width,
        height: box_.height,
        text: text.to_owned(),
    }
}

/// One named line inside a composite box: `top` below the box's top, `height` high.
pub fn inset_label_placement(box_: LabelBox, top: f64, height: f64, text: &str) -> LabelPlacement {
    LabelPlacement {
        x: box_.x - box_.width / 2.0,
        y: box_.y - box_.height / 2.0 + top,
        width: box_.width,
        height,
        text: text.to_owned(),
    }
}

/// A bare text on a line: centred on its midpoint, offset by `dy`, sitting above that offset; its width measured
/// where the caller has it, estimated otherwise.
pub fn midpoint_label_placement(
    metrics: &dyn TextMetrics,
    from: (f64, f64),
    to: (f64, f64),
    dy: f64,
    text: &str,
    measured: Option<f64>,
) -> LabelPlacement {
    let middle = ((from.0 + to.0) / 2.0, (from.1 + to.1) / 2.0);
    let width = estimated(metrics, text, measured);
    LabelPlacement {
        x: middle.0 - width / 2.0,
        y: middle.1 + dy - TEXT_LABEL_HEIGHT,
        width,
        height: TEXT_LABEL_HEIGHT,
        text: text.to_owned(),
    }
}

/// A label beside a point marker: start-anchored `gap` to the right of its centre, vertically centred on it.
pub fn aside_label_placement(
    metrics: &dyn TextMetrics,
    at: (f64, f64),
    gap: f64,
    text: &str,
    measured: Option<f64>,
) -> LabelPlacement {
    LabelPlacement {
        x: at.0 + gap,
        y: at.1 - TEXT_LABEL_HEIGHT / 2.0,
        width: estimated(metrics, text, measured),
        height: TEXT_LABEL_HEIGHT,
        text: text.to_owned(),
    }
}

/// The mirror of [`aside_label_placement`]: end-anchored, its right edge `gap` before `at`.
pub fn before_label_placement(
    metrics: &dyn TextMetrics,
    at: (f64, f64),
    gap: f64,
    text: &str,
    measured: Option<f64>,
) -> LabelPlacement {
    let width = estimated(metrics, text, measured);
    LabelPlacement {
        x: at.0 - gap - width,
        y: at.1 - TEXT_LABEL_HEIGHT / 2.0,
        width,
        height: TEXT_LABEL_HEIGHT,
        text: text.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    //! The standalone host's `labelPlacement.test.ts`.
    use super::*;
    use crate::text::EstimatedMetrics;

    const M: EstimatedMetrics = EstimatedMetrics;

    #[test]
    fn a_centred_label_covers_its_box_and_an_inset_one_its_line() {
        let box_ = LabelBox {
            x: 100.0,
            y: 50.0,
            width: 120.0,
            height: 60.0,
        };
        let centred = centred_label_placement(box_, "Name");
        assert_eq!(
            (40.0, 20.0, 120.0, 60.0),
            (centred.x, centred.y, centred.width, centred.height)
        );
        let inset = inset_label_placement(box_, 8.0, 18.0, "Name");
        assert_eq!((40.0, 28.0, 120.0, 18.0), (inset.x, inset.y, inset.width, inset.height));
    }

    #[test]
    fn a_line_label_sits_above_the_midpoint_at_its_measured_or_estimated_width() {
        let estimated = midpoint_label_placement(&M, (0.0, 0.0), (100.0, 40.0), -6.0, "uses", None);
        assert_eq!(
            (10.0, -2.0, 80.0, 16.0),
            (estimated.x, estimated.y, estimated.width, estimated.height)
        );
        let measured = midpoint_label_placement(&M, (0.0, 0.0), (100.0, 40.0), -6.0, "uses", Some(30.0));
        assert_eq!((35.0, 30.0), (measured.x, measured.width));
        let long = "a label of thirty characters!!";
        assert!((midpoint_label_placement(&M, (0.0, 0.0), (0.0, 0.0), 0.0, long, None).width - 198.0).abs() < 1e-9);
    }

    #[test]
    fn an_aside_label_starts_past_its_marker_and_a_before_label_ends_short_of_it() {
        let aside = aside_label_placement(&M, (10.0, 20.0), 12.0, "Launch", None);
        assert_eq!((22.0, 12.0, 80.0), (aside.x, aside.y, aside.width));
        let before = before_label_placement(&M, (10.0, 20.0), 8.0, "Launch", Some(40.0));
        assert_eq!((-38.0, 12.0, 40.0), (before.x, before.y, before.width));
    }
}
