//! What a session gives the shell to draw, and the changes between two renderings (etalii.adp spec 030, FR-018;
//! contracts/core-shell.md, `SceneChanged`).
//!
//! Ported from the standalone host's `EtAlii.Adp.Diagram/_Model` (`DiagramElement`, `DiagramDelta` and its four
//! records, `DiagramViewport`), `DiagramDiff.cs` and `EtAlii.Adp.Diagram.Modules/DiagramViewportExtensions.cs` at
//! `8b6ccfc`. The gRPC contract carried an element's payload as a module's serialised protobuf message; in one
//! process it is plain data: the element's DISL type, its kind, its ends or parent, and its attributes as text.

/// Whether an element is drawn as a node or as a relation between two nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ElementKind {
    Node,
    Relation,
}

/// A diagram element as a session hands it over: its identity, where it is placed, its type, and what it holds.
#[derive(Debug, Clone, PartialEq)]
pub struct DiagramElement {
    /// The element's id, the same one a selection names.
    pub id: String,
    /// The authored position, in the diagram's own units; `0` when the element has none.
    pub x: f64,
    pub y: f64,
    /// Whether the position was authored (by the attributes the notation places the node by, or in the registration's
    /// `layout:`), rather than left to a layout.
    pub placed: bool,
    /// The element's type in the specification's metamodel (`Variable`, `CausalLink`).
    pub element_type: String,
    pub kind: ElementKind,
    /// The containing element, for a node of a tree.
    pub parent: Option<String>,
    /// A relation's ends, by the ids of the nodes they name, as read.
    pub source: Option<String>,
    pub target: Option<String>,
    /// The element's attributes as text, in the metamodel's order.
    pub attributes: Vec<(String, String)>,
}

impl DiagramElement {
    /// A node of `element_type` with `id` and nothing else.
    pub fn node(id: impl Into<String>, element_type: impl Into<String>) -> Self {
        DiagramElement {
            id: id.into(),
            x: 0.0,
            y: 0.0,
            placed: false,
            element_type: element_type.into(),
            kind: ElementKind::Node,
            parent: None,
            source: None,
            target: None,
            attributes: Vec::new(),
        }
    }

    /// The text of attribute `name`, empty when the element has none.
    pub fn attribute(&self, name: &str) -> &str {
        self.attributes
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
            .unwrap_or_default()
    }
}

/// One change to what a connection holds: the standalone host's add, remove, group and ungroup vocabulary.
#[derive(Debug, Clone, PartialEq)]
pub enum DiagramDelta {
    /// Upsert these elements: a new id is inserted, a known one replaced.
    Add(Vec<DiagramElement>),
    /// Remove the elements with these ids.
    Remove(Vec<String>),
    /// Fold: the group element stands for the branch whose element ids are hidden.
    Group {
        source_element_ids: Vec<String>,
        group_element: DiagramElement,
    },
    /// Unfold: the branch's elements come back.
    Ungroup {
        group_element_id: String,
        elements: Vec<DiagramElement>,
    },
}

/// The visible rectangle a connection reported, in the diagram's own units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiagramViewport {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

/// A box in the diagram's own units: its top left corner and its size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Rect { x, y, width, height }
    }

    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }
}

impl DiagramViewport {
    /// The "everything is in view" viewport a connection has before it reports one.
    pub const UNBOUNDED: DiagramViewport = DiagramViewport {
        min_x: f64::NEG_INFINITY,
        min_y: f64::NEG_INFINITY,
        max_x: f64::INFINITY,
        max_y: f64::INFINITY,
    };

    pub fn new(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        DiagramViewport {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }

    /// Whether `bounds` is in view: a box the viewport merely touches counts, so an element on the edge is sent
    /// rather than popping in a pixel later (code-structure FR-045).
    pub fn intersects(&self, bounds: Rect) -> bool {
        bounds.right() >= self.min_x
            && bounds.x <= self.max_x
            && bounds.bottom() >= self.min_y
            && bounds.y <= self.max_y
    }
}

/// Whether two renderings of one element say the same thing: equal position, type and payload.
pub fn same(left: &DiagramElement, right: &DiagramElement) -> bool {
    left == right
}

/// The deltas that turn `before` into `after`: at most one remove, then at most one add, and nothing at all when the
/// two say the same thing. What is new or changed is added, what is gone is removed; an edited element keeps its id,
/// so it arrives as an add carrying its new state. Removals come first, so removing can never delete something the
/// same batch just added.
///
/// # Panics
///
/// When an id appears twice in one rendering: that is an error in what rendered it, and choosing one of the two
/// would hide it.
pub fn diff(before: &[DiagramElement], after: &[DiagramElement]) -> Vec<DiagramDelta> {
    let before_by_id = by_id(before);
    let after_by_id = by_id(after);
    let gone: Vec<String> = before
        .iter()
        .filter(|e| !after_by_id.contains_key(e.id.as_str()))
        .map(|e| e.id.clone())
        .collect();
    let arrived: Vec<DiagramElement> = after
        .iter()
        .filter(|e| before_by_id.get(e.id.as_str()).is_none_or(|was| !same(was, e)))
        .cloned()
        .collect();
    let mut deltas = Vec::with_capacity(2);
    if !gone.is_empty() {
        deltas.push(DiagramDelta::Remove(gone));
    }
    if !arrived.is_empty() {
        deltas.push(DiagramDelta::Add(arrived));
    }
    deltas
}

fn by_id(elements: &[DiagramElement]) -> std::collections::HashMap<&str, &DiagramElement> {
    let mut map = std::collections::HashMap::with_capacity(elements.len());
    for element in elements {
        assert!(
            map.insert(element.id.as_str(), element).is_none(),
            "The id '{}' appears twice in one rendering.",
            element.id
        );
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    fn element(id: &str, x: f64, element_type: &str, payload: &str) -> DiagramElement {
        DiagramElement {
            x,
            y: 20.0,
            attributes: vec![("payload".into(), payload.into())],
            ..DiagramElement::node(id, element_type)
        }
    }

    fn plain(id: &str) -> DiagramElement {
        element(id, 10.0, "Node", "1")
    }

    fn removed(deltas: &[DiagramDelta]) -> Vec<String> {
        deltas
            .iter()
            .filter_map(|d| match d {
                DiagramDelta::Remove(ids) => Some(ids.clone()),
                _ => None,
            })
            .flatten()
            .collect()
    }

    fn added(deltas: &[DiagramDelta]) -> Vec<String> {
        deltas
            .iter()
            .filter_map(|d| match d {
                DiagramDelta::Add(elements) => Some(elements.iter().map(|e| e.id.clone()).collect::<Vec<_>>()),
                _ => None,
            })
            .flatten()
            .collect()
    }

    #[test]
    fn each_kind_of_change_arrives_as_the_delta_it_is() {
        type Case = (
            &'static str,
            Vec<DiagramElement>,
            Vec<DiagramElement>,
            Vec<&'static str>,
            Vec<&'static str>,
        );
        let cases: Vec<Case> = vec![
            ("add", vec![plain("a")], vec![plain("a"), plain("b")], vec![], vec!["b"]),
            (
                "remove",
                vec![plain("a"), plain("b")],
                vec![plain("a")],
                vec!["b"],
                vec![],
            ),
            (
                "move",
                vec![plain("a")],
                vec![element("a", 40.0, "Node", "1")],
                vec![],
                vec!["a"],
            ),
            (
                "retype",
                vec![plain("a")],
                vec![element("a", 10.0, "Frame", "1")],
                vec![],
                vec!["a"],
            ),
            (
                "payload change",
                vec![element("a", 10.0, "Node", "1 2")],
                vec![element("a", 10.0, "Node", "1 3")],
                vec![],
                vec!["a"],
            ),
            (
                "nothing changed",
                vec![plain("a"), plain("b")],
                vec![plain("a"), plain("b")],
                vec![],
                vec![],
            ),
        ];
        for (name, before, after, gone, arrived) in cases {
            let deltas = diff(&before, &after);
            assert_eq!(removed(&deltas), gone, "{name}: removed");
            assert_eq!(added(&deltas), arrived, "{name}: added");
        }
    }

    #[test]
    fn an_unchanged_element_rendered_again_is_not_resent() {
        assert!(
            diff(
                &[element("a", 10.0, "Node", "7 8 9")],
                &[element("a", 10.0, "Node", "7 8 9")]
            )
            .is_empty()
        );
    }

    #[test]
    fn a_diff_that_removes_and_adds_removes_first() {
        let deltas = diff(&[plain("gone")], &[plain("new")]);
        assert_eq!(
            deltas,
            vec![
                DiagramDelta::Remove(vec!["gone".into()]),
                DiagramDelta::Add(vec![plain("new")])
            ]
        );
    }

    #[test]
    fn a_changed_element_arrives_in_its_new_state() {
        let deltas = diff(&[element("a", 1.0, "Node", "1")], &[element("a", 2.0, "Node", "1")]);
        let [DiagramDelta::Add(elements)] = deltas.as_slice() else {
            panic!("one add, not {deltas:?}");
        };
        assert_eq!(elements[0].x, 2.0);
    }

    #[test]
    #[should_panic(expected = "appears twice")]
    fn an_id_twice_in_one_rendering_is_an_error() {
        diff(&[], &[plain("a"), plain("a")]);
    }

    #[test]
    fn the_viewport_counts_a_box_it_merely_touches() {
        let viewport = DiagramViewport::new(0.0, 0.0, 100.0, 100.0);
        for (x, y, expected) in [
            (50.0, 50.0, true),
            (100.0, 50.0, true),
            (-10.0, 50.0, true),
            (100.5, 50.0, false),
            (-10.5, 50.0, false),
            (50.0, 100.5, false),
            (50.0, -10.5, false),
        ] {
            assert_eq!(viewport.intersects(Rect::new(x, y, 10.0, 10.0)), expected, "({x}, {y})");
        }
    }

    #[test]
    fn the_unbounded_viewport_has_every_box_in_view() {
        assert!(DiagramViewport::UNBOUNDED.intersects(Rect::new(-1e9, 1e9, 1.0, 1.0)));
    }
}
