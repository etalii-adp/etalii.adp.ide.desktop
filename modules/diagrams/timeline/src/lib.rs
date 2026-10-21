//! The timeline diagram's residue (etalii.adp spec 030, task T060, after the standalone host's
//! `src/diagrams/timeline` at `8b6ccfc`): what the timeline's specification (`definitions/diagrams/timeline.dis`)
//! cannot state, registered with `adp-diagram` as the [`DiagramModule`] of the origin [`ORIGIN`].
//!
//! The standalone module read and wrote `.tml` bodies through its own commands; the desktop host reads and writes
//! them through the specification and the binding `timeline.fbl`, like every other DISL and FBL diagram, and keeps
//! here only what the module added:
//!
//! - [`times`]: the CEL functions of the plugin `net.etalii.adp.generic.timelineTimes` (`readableTime`,
//!   `compareTimes`, `dateAfter`, `today`, `writtenEnd`), over the reading of a time in [`instants`];
//! - [`scale`] and [`placement`]: the canvas coordinates, seconds since 1970 across and 60 units per row down, and
//!   what a gesture to a point writes (a begin in the element's own precision, a period's end that keeps its
//!   duration, the nearest row); a period as long as it lasts, in seconds, and what a drag of its edge writes (its
//!   begin or its end);
//! - [`ruler`]: the labels of the time ruler under the canvas (`timelineTicks.ts`);
//! - [`arrangement`]: the plugin layout `net.etalii.adp.generic.timelineRowArrange` behind Arrange diagram;
//! - an element's computed label renamed through its `label` attribute, as the module's own rename wrote it;
//! - giving a moment an end and removing a period's end written as the change of `end` alone: the binding tells a
//!   period from a moment by its `end` key, so the operations' `retype` is that change and is not written apart;
//! - no edit, whichever gesture, form or operation makes it, writes an element that ends before it begins.

pub mod arrangement;
pub mod instants;
pub mod placement;
pub mod reading;
pub mod ruler;
pub mod scale;
pub mod times;

use std::rc::Rc;

use adp_cel::{Map, Value};
use adp_diagram::{DiagramModule, Extent, Length, ModulePlacement, Resize, ResizedEdge};
use adp_disl::{DislChange, DislDiagram, DislElement, DislPluginFunctions, DislSpecification};

use crate::instants::Precision;
use crate::reading::TimelineElement;

/// The tool type this module serves.
pub const ORIGIN: &str = "generic/timeline";

/// The timeline's own code beside its specification.
#[derive(Debug, Default, Clone, Copy)]
pub struct TimelineModule;

/// The module, to register with `adp_diagram::DiagramTools::with_module` under [`ORIGIN`].
pub fn module() -> Rc<dyn DiagramModule> {
    Rc::new(TimelineModule)
}

impl DiagramModule for TimelineModule {
    fn plugin_functions(&self) -> DislPluginFunctions {
        times::plugin_functions()
    }

    fn position(&self, element: &DislElement) -> Option<(f64, f64)> {
        (!element.is_relation()).then(|| placement::position(&TimelineElement::of(element)))
    }

    fn placement(&self, element: &DislElement, x: f64, y: f64) -> Option<Result<ModulePlacement, String>> {
        (!element.is_relation()).then(|| placement::placement(&TimelineElement::of(element), x, y))
    }

    /// A period is as wide as it lasts, in seconds on the time axis; a moment is the notation's size.
    fn extent(&self, element: &DislElement) -> Option<Extent> {
        let width = placement::duration(&TimelineElement::of(element))?;
        Some(Extent {
            width: Some(Length::Domain(width)),
            height: None,
        })
    }

    /// A drag of a period's left edge writes its begin, of its right edge its end.
    fn resize(&self, element: &DislElement, resize: &Resize) -> Option<Result<ModulePlacement, String>> {
        if element.is_relation() {
            return None;
        }
        let read = TimelineElement::of(element);
        Some(match resize.edge {
            ResizedEdge::Left => placement::resized(&read, true, resize.x),
            ResizedEdge::Right => placement::resized(&read, false, resize.right),
            ResizedEdge::Top | ResizedEdge::Bottom => Err(placement::NOT_RESIZED.to_owned()),
        })
    }

    /// A point on the canvas as an addition there reads it: the date under it, its begin at the start of the day,
    /// and the nearest row.
    fn position_at(&self, x: f64, y: f64) -> Option<Map> {
        let begin = scale::to_text(scale::to_time(x, Precision::Date), Precision::Date);
        Some(
            [("x", Value::String(begin)), ("y", Value::Int(scale::to_nearest_row(y)))]
                .into_iter()
                .collect(),
        )
    }

    fn layout(
        &self,
        _specification: &DislSpecification,
        plugin: &str,
        diagram: &DislDiagram,
    ) -> Option<Result<Vec<DislChange>, String>> {
        (plugin == arrangement::PLUGIN).then(|| arrangement::arrange(diagram))
    }

    /// An element's label is computed (`shownLabel(self)`) and binds no attribute; its rename writes `label`.
    fn label_attribute(&self, element: &DislElement, label: &str) -> Option<String> {
        (label == "label" && element.is_a("Element")).then(|| "label".to_owned())
    }

    /// A retype left out, and any change that would make an element end before it begins refused.
    fn written(&self, diagram: &DislDiagram, changes: Vec<DislChange>) -> Result<Vec<DislChange>, String> {
        placement::refuse_inversions(diagram, &changes)?;
        Ok(changes
            .into_iter()
            .filter(|change| !matches!(change, DislChange::Retype { .. }))
            .collect())
    }
}
