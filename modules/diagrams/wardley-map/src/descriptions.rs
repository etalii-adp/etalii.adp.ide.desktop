//! What to call an element given only its id, and what a selection of it reads as (etalii.adp spec 030, task T070).
//!
//! Ported from the standalone host's `WardleyElementDescriptions.cs` and `WardleyContextSourceResolver.cs` at `8b6ccfc`.
//! An identity's key is a handle, not a label: a link's key runs its ends and kind together, an attitude's its kind
//! and a corner. So the element is looked up by key and then described by what it is. The evolution axis is drawn but
//! never selectable: it is the chrome the map is drawn against.

use crate::identities::{WardleyIdentityEntry, keys, kind};
use crate::model::WardleyMap;

/// What one element is called and where it sits in the chain of names a selection carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WardleyElementDescription {
    /// The element's display text: a component's name, a link's two ends.
    pub text: String,
    /// The chain of names from the top of the map: one segment for everything but a pipeline child, which is named
    /// beneath its parent.
    pub path: Vec<String>,
    /// True for a component that is a pipeline parent with children, and nothing else.
    pub has_children: bool,
}

fn flat(text: String) -> WardleyElementDescription {
    WardleyElementDescription {
        path: vec![text.clone()],
        text,
        has_children: false,
    }
}

/// The element `element_id` names, or none when the map no longer has it.
pub fn of(
    map: &WardleyMap,
    identities: &[WardleyIdentityEntry],
    element_id: &str,
) -> Option<WardleyElementDescription> {
    let entry = identities.iter().find(|e| e.id == element_id)?;
    let key = entry.key.as_str();
    match entry.kind.as_str() {
        kind::COMPONENT => {
            let component = map.components.iter().find(|c| keys::component(c) == key)?;
            let has_children = map
                .pipelines
                .iter()
                .any(|p| p.parent == component.name && !p.children.is_empty());
            Some(WardleyElementDescription {
                text: component.name.clone(),
                path: vec![component.name.clone()],
                has_children,
            })
        }
        kind::PIPELINE_CHILD => map.pipelines.iter().find_map(|pipeline| {
            pipeline
                .children
                .iter()
                .find(|child| keys::pipeline_child(pipeline, child) == key)
                .map(|child| WardleyElementDescription {
                    text: child.name.clone(),
                    path: vec![pipeline.parent.clone(), child.name.clone()],
                    has_children: false,
                })
        }),
        kind::LINK => {
            let link = map.links.iter().find(|l| keys::link(l) == key)?;
            let arrow = link.kind.arrow();
            Some(flat(if link.context.is_empty() {
                format!("{} {arrow} {}", link.source, link.target)
            } else {
                format!("{} {arrow} {}: {}", link.source, link.target, link.context)
            }))
        }
        kind::NOTE => map
            .notes
            .iter()
            .find(|n| keys::note(n) == key)
            .map(|n| flat(n.text.clone())),
        kind::ANNOTATION => map.annotations.iter().find(|a| keys::annotation(a) == key).map(|a| {
            flat(if a.text.is_empty() {
                a.number.to_string()
            } else {
                format!("{}. {}", a.number, a.text)
            })
        }),
        kind::ACCELERATOR => map.accelerators.iter().find(|a| keys::accelerator(a) == key).map(|a| {
            flat(if a.is_deaccelerator {
                format!("{} (deaccelerator)", a.name)
            } else {
                a.name.clone()
            })
        }),
        kind::ATTITUDE => map
            .attitudes
            .iter()
            .find(|a| keys::attitude(a) == key)
            .map(|a| flat(a.kind.label().to_owned())),
        _ => None,
    }
}
