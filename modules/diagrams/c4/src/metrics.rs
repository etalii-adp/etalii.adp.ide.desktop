//! The declared font metric the layout sizes C4 boxes with (etalii.adp spec 030, task T061).
//!
//! Ported from the standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4/_Model/C4Metrics.cs` at
//! `8b6ccfc`. The text's own width is the shared estimate (`adp_layout::text_metric`); the padding, minimum and
//! maximum around it, and the separations, are this module's. A C4 box carries three things: the name, the bracketed
//! type-and-technology line, and the description.

use adp_layout::rect::Rect;
use adp_layout::text_metric;

/// .NET's `Math.Round(value, 2)`: to two decimals, a midpoint to the even neighbour.
pub fn round2(value: f64) -> f64 {
    if value.abs() < 1e16 {
        (value * 100.0).round_ties_even() / 100.0
    } else {
        value
    }
}

/// The metrics a C4 layout is computed with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct C4Metrics {
    pub font_size: f64,
    pub line_height: f64,
    pub horizontal_padding: f64,
    pub vertical_padding: f64,
    pub minimum_width: f64,
    pub maximum_width: f64,
    pub rank_separation: f64,
    pub node_separation: f64,
    pub boundary_padding: f64,
}

impl Default for C4Metrics {
    fn default() -> Self {
        C4Metrics {
            font_size: 14.0,
            line_height: 1.4,
            horizontal_padding: 12.0,
            vertical_padding: 10.0,
            minimum_width: 160.0,
            maximum_width: 240.0,
            rank_separation: 120.0,
            node_separation: 60.0,
            boundary_padding: 32.0,
        }
    }
}

impl C4Metrics {
    /// The box an element occupies, given the three lines it shows: as wide as its widest line, clamped so one long
    /// description does not make a box the width of the diagram, and as tall as the lines the description wraps onto.
    pub fn measure(&self, name: &str, type_line: &str, description: &str) -> Rect {
        let content_width = self.maximum_width - 2.0 * self.horizontal_padding;
        let widest = self.text_width(name).max(self.text_width(type_line));
        let width = (widest.max(self.text_width(description).min(content_width)) + 2.0 * self.horizontal_padding)
            .clamp(self.minimum_width, self.maximum_width);
        let description_lines = if description.is_empty() {
            0
        } else {
            (self.text_width(description) / (width - 2.0 * self.horizontal_padding)).ceil() as i64
        };
        let lines = 1 + i64::from(!type_line.is_empty()) + description_lines.max(0);
        let height = lines as f64 * self.font_size * self.line_height + 2.0 * self.vertical_padding;
        Rect::new(0.0, 0.0, round2(width), round2(height))
    }

    /// The estimated width of `text` at the metric's font size.
    pub fn text_width(&self, text: &str) -> f64 {
        text_metric::width_of(text, self.font_size)
    }
}
