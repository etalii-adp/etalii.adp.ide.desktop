//! The declared metric the causal loop layouts size variables with (the standalone host's `CausalLoopMetrics.cs` at
//! `8b6ccfc`, finding A11): the text's width is the shared estimate (`adp_layout::text_metric`), and the padding,
//! minimum and separation around it are this module's. The ring and the self-organizing arrangement place boxes of
//! these sizes, so the positions they give depend on it.

use adp_layout::text_metric;

/// How a variable is sized, in canvas units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    /// Nominal font size.
    pub font_size: f64,
    /// Space either side of the text.
    pub horizontal_padding: f64,
    /// Space above and below it.
    pub vertical_padding: f64,
    /// The narrowest a variable is drawn, so a one-letter name is still a target.
    pub minimum_width: f64,
    /// The clear space kept between neighbours.
    pub separation: f64,
}

impl Default for Metrics {
    fn default() -> Self {
        Metrics {
            font_size: 14.0,
            horizontal_padding: 14.0,
            vertical_padding: 10.0,
            minimum_width: 90.0,
            separation: 40.0,
        }
    }
}

impl Metrics {
    /// How wide a variable showing `text` is drawn.
    pub fn width_of(&self, text: &str) -> f64 {
        self.minimum_width
            .max(text_metric::width_of(text, self.font_size) + self.horizontal_padding * 2.0)
    }

    /// How tall every variable is drawn: one line of text.
    pub fn height(&self) -> f64 {
        (self.font_size * 1.4) + (self.vertical_padding * 2.0)
    }
}
