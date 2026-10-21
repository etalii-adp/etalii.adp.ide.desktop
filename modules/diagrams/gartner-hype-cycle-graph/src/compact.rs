//! The Compact viewpoint (`gartner-hype-cycle-graph.dis`, `viewpoints.compact` and the layout `rowPacked`; the
//! standalone host's `GhgCanvas.tsx`, `compactWidthOf`, and the library's `rowPackedLayout.ts` as `compileNotation`
//! configured it, at `8b6ccfc`): every trend shortened to 24 canvas units per visible phase, packed left in the row it
//! keeps with causes before effects, on an axis that is no longer a date. It is the viewer's: nothing of it is saved,
//! and nothing moves or resizes in it.
//!
//! The packing is `adp-layout`'s row-packed mode, fed as the standalone canvas fed it: every trend, trigger and note at
//! its true-time centre and size, the trends at their compact width, each with the room its label takes before it,
//! 4 units apart on every row an element covers (rows 56 units apart), an influence's target after the middle of its
//! source. A drop on Compact means the true-time point the packing's inverse gives.

use std::collections::HashMap;

use adp_disl::DislDiagram;
use adp_layout::text_metric;
use adp_layout::{
    LayoutAlgorithm, LayoutConnection, LayoutDefinition, LayoutElement, LayoutInput, LayoutMode, Point,
    RowPackedDeclaration, RowPackedLayout, RowPackedWidth,
};

use crate::placement;
use crate::reading::{self, Note, Trend, Trigger};
use crate::scale;

/// A compact trend's width per visible phase, in canvas units (`compactWidthOf`: 24 steps for all four).
pub const WIDTH_PER_PHASE: f64 = 24.0;

/// The least space between two elements on one row (`rowPacked.gap`).
pub const GAP: f64 = 4.0;

/// The canvas's label size, and the space between a label drawn before an element and the element.
const LABEL_FONT_SIZE: f64 = 12.0;
const BEFORE_GAP: f64 = 8.0;

/// Where Compact draws one element: its centre and its width, in canvas units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Packed {
    pub x: f64,
    pub y: f64,
    pub width: f64,
}

/// A trend's compact width: its visible phases, 24 units each.
pub fn width_of(trend: &Trend) -> f64 {
    WIDTH_PER_PHASE * trend.visible_phases() as f64
}

/// What the packing reads of `diagram`: every trend, trigger and note true time draws, at its true-time centre and
/// size, and the influences.
fn input(diagram: &DislDiagram) -> LayoutInput {
    let unit = reading::unit_of(diagram);
    let mut elements = Vec::new();
    for node in diagram.nodes() {
        let Some((x, y)) = placement::position(&node) else {
            continue;
        };
        let element = if node.is_a("Trend") {
            let trend = Trend::of(&node);
            if trend.span().is_none() {
                continue;
            }
            let width = scale::width_of(trend.months(), unit);
            LayoutElement {
                element_type: Some("Trend".to_owned()),
                packed_width: Some(width_of(&trend)),
                leading: Some(BEFORE_GAP + text_metric::width_of(&trend.name, LABEL_FONT_SIZE)),
                ..LayoutElement::new(
                    node.id(),
                    x + width / 2.0,
                    y + scale::TREND_HEIGHT / 2.0,
                    width,
                    scale::TREND_HEIGHT,
                )
            }
        } else if node.is_a("Trigger") {
            let trigger = Trigger::of(&node);
            let Some(date) = trigger.date else { continue };
            let label = format!("{} \u{b7} {}", trigger.name, scale::format_when(date, unit));
            LayoutElement {
                element_type: Some("Trigger".to_owned()),
                leading: Some(BEFORE_GAP + text_metric::width_of(&label, LABEL_FONT_SIZE)),
                ..LayoutElement::new(node.id(), x, y, scale::TRIGGER_SIZE, scale::TRIGGER_SIZE)
            }
        } else {
            let note = Note::of(&node);
            let Some((_, width, height)) = note.placed() else {
                continue;
            };
            LayoutElement {
                element_type: Some("Note".to_owned()),
                ..LayoutElement::new(node.id(), x + width / 2.0, y + height / 2.0, width, height)
            }
        };
        elements.push(element);
    }
    let connections = diagram
        .relations_of_type("Influence")
        .iter()
        .filter_map(|i| Some(LayoutConnection::new(i.source_id()?, i.target_id()?)))
        .collect();
    LayoutInput { elements, connections }
}

/// The row-packed layout as the specification's `rowPacked` declares it for Compact.
fn definition() -> LayoutDefinition {
    LayoutDefinition {
        row_packed: Some(RowPackedDeclaration {
            width: RowPackedWidth::Bound,
            gap: GAP,
            types: Some(vec!["Trend".to_owned()]),
            row_step: Some(scale::ROW_STEP),
            follow_connections: true,
        }),
        ..LayoutDefinition::with_modes(&[LayoutMode::RowPacked])
    }
}

/// Where Compact draws every trend, trigger and note of `diagram` that true time draws, by id.
pub fn positions(diagram: &DislDiagram) -> HashMap<String, Packed> {
    let input = input(diagram);
    let placements = RowPackedLayout.place(&input, &definition(), None).unwrap_or_default();
    input
        .elements
        .iter()
        .filter_map(|element| {
            let placed = placements.get(&element.id)?;
            Some((
                element.id.clone(),
                Packed {
                    x: placed.x,
                    y: placed.y,
                    width: placed.width.unwrap_or(element.width),
                },
            ))
        })
        .collect()
}

/// The true-time canvas point a point of Compact stands for, as a drop there means it: its x read as a left edge
/// between the two packed neighbours it falls between, in proportion; its y kept.
pub fn inverse(diagram: &DislDiagram, x: f64, y: f64) -> (f64, f64) {
    let point = RowPackedLayout
        .inverse(Point { x, y }, &input(diagram), &definition())
        .unwrap_or(Point { x, y });
    (point.x, point.y)
}
