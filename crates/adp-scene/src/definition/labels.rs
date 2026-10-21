//! What an element says, declared rather than drawn, and its layout (the standalone host's
//! `canvas/library/definition/labels.ts` at 8b6ccfc): declarations and an element in, positioned lines out.
//!
//! The layout is the standalone's, measured through [`TextMetrics`] where it counts characters. A label's text that
//! evaluates to a list is one line per entry, as the standalone's collection binding is; DISL's labels bind one value,
//! so that is the CEL's choice. Not ported, because nothing in DISL states them: a label's `stack` (its lines'
//! spacing and bound start), its `columns`, its named `slot` other than the body, and the `scaleWithView` of its
//! typography.

use adp_cel::Value;
use adp_disl::expressions::contexts;

use super::Condition;
use super::binding::{Binding, holds, text_of};
use crate::evaluation::{Evaluator, Vars};
use crate::geometry::{Point, Rect};
use crate::shapes::BuiltInShape;
use crate::shapes::outline::{outline_of, text_region_of};
use crate::style::Font;
use crate::text::{TextMetrics, continued_within, fit_to_width, fit_within, wrap};

/// How far a `before` label's right edge sits from the element's left edge.
pub const BEFORE_GAP: f64 = 8.0;

/// How far from an edge an aligned label sits, when it does not say.
const LABEL_INSET: f64 = 8.0;

/// Where a label sits relative to its element's shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LabelPlacement {
    #[default]
    Inside,
    Above,
    Below,
    /// Start-anchored, 6 to the right of the element.
    Beside,
    /// End-anchored, [`BEFORE_GAP`] to the left of the element.
    Before,
    Inset,
}

/// What a label's offset is measured from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AnchorTo {
    #[default]
    Centre,
    Top,
    Bottom,
}

/// Which end of a text sits at its position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAnchor {
    Start,
    Middle,
    End,
}

/// One declared label.
#[derive(Debug, Clone, PartialEq)]
pub struct LabelDeclaration {
    /// The DISL label's id.
    pub id: String,
    pub text: Binding,
    pub placement: LabelPlacement,
    /// An explicit position relative to [`Self::anchor_to`].
    pub offset: Option<Point>,
    pub anchor_to: AnchorTo,
    /// Sets the line's x: at the inset from the aligned edge.
    pub align: Option<TextAnchor>,
    pub inset_x: Option<f64>,
    pub editable: bool,
    /// Laid out as wrapped text inside the shape's text region.
    pub wrap: bool,
    /// Trimmed to the box with an ellipsis.
    pub truncate: bool,
    pub tooltip: Option<Binding>,
    pub when: Option<Condition>,
    /// The label's own style (DISL section 6.12 `style`), as written; its font is drawn over the element's.
    pub style: Option<serde_json::Value>,
}

impl LabelDeclaration {
    pub fn new(id: &str, text: Binding) -> Self {
        LabelDeclaration {
            id: id.to_owned(),
            text,
            placement: LabelPlacement::Inside,
            offset: None,
            anchor_to: AnchorTo::Centre,
            align: None,
            inset_x: None,
            editable: false,
            wrap: false,
            truncate: false,
            tooltip: None,
            when: None,
            style: None,
        }
    }
}

/// One line, positioned and ready to draw: `y` is its baseline.
#[derive(Debug, Clone, PartialEq)]
pub struct LaidOutLabel {
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub anchor: TextAnchor,
    pub font: Font,
    pub editable: bool,
    pub tooltip: Option<String>,
    /// For a wrapped label, the region its block was laid out in.
    pub region: Option<Rect>,
    pub declaration_index: usize,
    pub line_index: usize,
}

/// The wrapped lines of one declaration, fitted to the shape rather than to its bounding box: the band decides the
/// region and the region the line count, solved together until the count stops changing.
fn fitted_wrap(
    metrics: &dyn TextMetrics,
    text: &str,
    shape: BuiltInShape,
    bounds: Rect,
    line_height: f64,
    font_size: f64,
) -> (Vec<String>, Rect, bool) {
    let mut line_count = 1;
    let mut region = text_region_of(shape, bounds, line_height);
    let mut lines = wrap(metrics, text, region.width, font_size);
    let mut round = 0;
    while round < 6 && lines.len() != line_count {
        line_count = lines.len();
        let wanted = text_region_of(shape, bounds, line_count as f64 * line_height);
        if wanted.width <= 0.0 || wanted.height <= 0.0 {
            break;
        }
        region = wanted;
        lines = wrap(metrics, text, region.width, font_size);
        round += 1;
    }
    let room = ((region.height / line_height).floor() as usize).max(1);
    if lines.len() <= room {
        return (lines, region, false);
    }
    let mut visible: Vec<String> = lines[..room].to_vec();
    let last = visible[room - 1].clone();
    visible[room - 1] = continued_within(metrics, &last, region.width, font_size);
    (visible, region, true)
}

/// The text region a truncated single line is fitted to, or none to keep trimming to the box: only a centred,
/// truncated line inside a shape with an outline.
fn fitted_region_of(
    declaration: &LabelDeclaration,
    bounds: Rect,
    shape: BuiltInShape,
    line_height: f64,
) -> Option<Rect> {
    let inside = matches!(declaration.placement, LabelPlacement::Inside | LabelPlacement::Inset);
    let fits = declaration.truncate
        && inside
        && declaration.offset.is_none()
        && matches!(declaration.align, None | Some(TextAnchor::Middle))
        && !outline_of(shape, bounds).is_empty();
    if !fits {
        return None;
    }
    let region = text_region_of(shape, bounds, line_height);
    (region.width > 0.0).then_some(region)
}

fn aligned_x(align: TextAnchor, bounds: Rect, inset_x: Option<f64>) -> f64 {
    let inset = inset_x.unwrap_or(LABEL_INSET);
    match align {
        TextAnchor::Start => bounds.x + inset,
        TextAnchor::End => bounds.right() - inset,
        TextAnchor::Middle => bounds.x + bounds.width / 2.0,
    }
}

/// The width a truncated line may fill: the whole box when centred, from its inset to the far edge when aligned.
fn room_of(align: Option<TextAnchor>, bounds: Rect, inset_x: Option<f64>) -> f64 {
    match align {
        Some(TextAnchor::Start) | Some(TextAnchor::End) => bounds.width - inset_x.unwrap_or(LABEL_INSET),
        _ => bounds.width,
    }
}

fn placement_of(declaration: &LabelDeclaration, bounds: Rect) -> (f64, f64, TextAnchor) {
    let centre_x = bounds.x + bounds.width / 2.0;
    let centre_y = bounds.y + bounds.height / 2.0;
    if let Some(offset) = declaration.offset {
        let origin_y = match declaration.anchor_to {
            AnchorTo::Top => bounds.y,
            AnchorTo::Bottom => bounds.bottom(),
            AnchorTo::Centre => centre_y,
        };
        return (centre_x + offset.x, origin_y + offset.y, TextAnchor::Middle);
    }
    match declaration.placement {
        LabelPlacement::Above => (centre_x, bounds.y - 6.0, TextAnchor::Middle),
        LabelPlacement::Below => (centre_x, bounds.bottom() + 14.0, TextAnchor::Middle),
        LabelPlacement::Beside => (bounds.right() + 6.0, centre_y + 4.0, TextAnchor::Start),
        LabelPlacement::Before => (bounds.x - BEFORE_GAP, centre_y + 4.0, TextAnchor::End),
        LabelPlacement::Inside | LabelPlacement::Inset => {
            (centre_x, bounds.y + bounds.height * 0.5 + 4.0, TextAnchor::Middle)
        }
    }
}

/// Alignment sets x and an offset sets y (`baselineOf`).
fn baseline_of(declaration: &LabelDeclaration, bounds: Rect) -> (f64, f64, TextAnchor) {
    let placed = placement_of(declaration, bounds);
    match declaration.align {
        None => placed,
        Some(align) => (aligned_x(align, bounds, declaration.inset_x), placed.1, align),
    }
}

/// The line height of a font: `round(size * 1.4)`, as the canvas library takes it.
pub fn line_height_of(font_size: f64) -> f64 {
    (font_size * 1.4 + 0.5).floor()
}

/// The lines a label's text gives: one per entry of a list, one for anything else, none for nothing.
fn entries_of(evaluator: &Evaluator, binding: &Binding, vars: &Vars) -> Vec<String> {
    match binding.value(evaluator, contexts::ELEMENT, vars) {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::List(items)) => items.iter().filter(|v| !v.is_null()).map(text_of).collect(),
        Some(value) => vec![text_of(&value)],
    }
}

/// What a label is laid out with: the evaluator, the element's variables, its bounds and shape, the font its labels
/// take from the element's style, and the metric.
pub struct LabelContext<'a> {
    pub evaluator: &'a Evaluator,
    pub vars: &'a Vars,
    pub bounds: Rect,
    /// The shape the label sits in, for a wrapped label's text region; a custom shape reads as a box.
    pub shape: BuiltInShape,
    pub metrics: &'a dyn TextMetrics,
}

/// Every line a set of declarations draws, positioned (`layoutLabels`). A declaration whose `when` fails, whose text
/// gives nothing, or whose list is empty contributes no lines. `fonts` gives each declaration's font.
pub fn layout_labels(
    declarations: &[LabelDeclaration],
    context: &LabelContext<'_>,
    fonts: &dyn Fn(&LabelDeclaration) -> Font,
) -> Vec<LaidOutLabel> {
    let LabelContext {
        evaluator,
        vars,
        bounds,
        shape,
        metrics,
    } = *context;
    let mut laid_out = Vec::new();
    for (declaration_index, declaration) in declarations.iter().enumerate() {
        if !holds(evaluator, declaration.when.as_deref(), contexts::ELEMENT, vars) {
            continue;
        }
        let lines = entries_of(evaluator, &declaration.text, vars);
        if lines.is_empty() {
            continue;
        }
        let font = fonts(declaration);
        let tooltip = declaration
            .tooltip
            .as_ref()
            .and_then(|t| t.element_text(evaluator, vars));

        if declaration.wrap {
            let line_height = line_height_of(font.size);
            let whole = lines.join("\n");
            let (fitted, region, overflowed) = fitted_wrap(metrics, &whole, shape, bounds, line_height, font.size);
            let align = declaration.align.unwrap_or(TextAnchor::Middle);
            let x = match align {
                TextAnchor::Start => region.x,
                TextAnchor::End => region.right(),
                TextAnchor::Middle => region.x + region.width / 2.0,
            };
            let block = fitted.len() as f64 * line_height;
            let top = region.y + ((region.height - block) / 2.0).max(0.0);
            for (line_index, line) in fitted.into_iter().enumerate() {
                laid_out.push(LaidOutLabel {
                    text: line,
                    x,
                    y: top + line_index as f64 * line_height + line_height * 0.75,
                    anchor: align,
                    font: font.clone(),
                    editable: declaration.editable,
                    tooltip: if overflowed {
                        Some(whole.clone())
                    } else {
                        tooltip.clone()
                    },
                    region: Some(region),
                    declaration_index,
                    line_index,
                });
            }
            continue;
        }

        let (x, y, anchor) = baseline_of(declaration, bounds);
        let region = fitted_region_of(declaration, bounds, shape, line_height_of(font.size));
        let single = lines.len() == 1;
        for (line_index, line) in lines.into_iter().enumerate() {
            let text = match region {
                Some(region) => fit_within(metrics, &line, region.width, font.size),
                None if declaration.truncate => fit_to_width(
                    metrics,
                    &line,
                    room_of(declaration.align, bounds, declaration.inset_x),
                    font.size,
                ),
                None => line,
            };
            laid_out.push(LaidOutLabel {
                text,
                x: region.map(|r| r.x + r.width / 2.0).unwrap_or(x),
                y,
                anchor,
                font: font.clone(),
                // A line of a list has no single authored value to write back to.
                editable: declaration.editable && single,
                tooltip: tooltip.clone(),
                region: None,
                declaration_index,
                line_index,
            });
        }
    }
    laid_out
}

/// The boxes a laid-out label takes, for fitting the view to what is drawn (`labelBoxesOf`).
pub fn label_box_of(metrics: &dyn TextMetrics, line: &LaidOutLabel) -> Rect {
    let width = metrics.width(&line.text, line.font.size);
    let left = match line.anchor {
        TextAnchor::Start => line.x,
        TextAnchor::End => line.x - width,
        TextAnchor::Middle => line.x - width / 2.0,
    };
    Rect::new(left, line.y - line.font.size, width, line.font.size * 1.3)
}
