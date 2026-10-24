//! `textWidth(text, fontSize)` and `textHeight(lines, fontSize)` (DISL sections 6.5 and 12.4): text measured under the
//! specification's declared `notation.textMetric`, so that every host sizes and lays out alike. An `average` metric
//! advances each counted character by `advance` times the font size and makes a line `lineHeight` times the font size
//! high. The `host` metric, the default, is this host's estimate (`adp_layout::text_metric`): 0.55 per UTF-16 code
//! unit, and lines 1.4 high as the measuring modules lay them out.

use adp_cel::{Environment, Function, Value};

use crate::json::{self, Json};

const HOST_ADVANCE: f64 = 0.55;
const HOST_LINE_HEIGHT: f64 = 1.4;

/// How characters are counted.
#[derive(Clone, Copy)]
enum Count {
    Utf16,
    CodePoint,
}

#[derive(Clone, Copy)]
struct Metric {
    advance: f64,
    count: Count,
    line_height: f64,
}

impl Metric {
    fn of(root: &Json) -> Metric {
        let declared = json::member(json::member(root, "notation"), "textMetric");
        let number = |key: &str, default: f64| json::get(declared, key).and_then(Json::as_f64).unwrap_or(default);
        let count = match json::string_of(declared, "count") {
            // A grapheme is counted as a code point: the desktop host has no segmentation, and the two agree on the
            // texts the measuring descriptions hold.
            Some("codepoint" | "grapheme") => Count::CodePoint,
            _ => Count::Utf16,
        };
        Metric {
            advance: number("advance", HOST_ADVANCE),
            count,
            line_height: number("lineHeight", HOST_LINE_HEIGHT),
        }
    }

    fn characters(&self, text: &str) -> usize {
        match self.count {
            Count::Utf16 => text.encode_utf16().count(),
            Count::CodePoint => text.chars().count(),
        }
    }
}

fn number(value: &Value, what: &str) -> Result<f64, String> {
    value.as_number().ok_or_else(|| format!("{what} is a number."))
}

pub(crate) fn register(environment: &mut Environment, root: &Json) {
    let metric = Metric::of(root);
    environment.add_function(Function::global("textWidth", 2, move |a| {
        let text = a[0].as_str().ok_or("textWidth's text is a string.")?;
        let size = number(&a[1], "textWidth's font size")?;
        Ok(Value::Double(metric.characters(text) as f64 * size * metric.advance))
    }));
    environment.add_function(Function::global("textHeight", 2, move |a| {
        let lines = number(&a[0], "textHeight's line count")?;
        let size = number(&a[1], "textHeight's font size")?;
        Ok(Value::Double(lines * size * metric.line_height))
    }));
}
