//! The canvas's one gesture arbiter (etalii.adp spec 030, FR-018, Story 2 scenario 3, task T056): every press on the
//! canvas, of either button, is one gesture, decided once, at its end, as a click or a drag, and Escape dissolves
//! whatever is in flight without writing anything.
//!
//! Ported from the standalone host's `src/client/src/canvas/gesture/usePointerGesture.ts` at `8b6ccfc`: one gesture
//! at a time, the same movement threshold (more than 4 pixels by hypotenuse turns a press into a drag), the verdict
//! taken from what the gesture recorded rather than from a trailing click, and an abandon that reverts the preview.
//! What each verdict means is `DiagramCanvas.tsx`'s: a press on an element selects it (Ctrl toggles it), a press on
//! the empty canvas clears the selection, a drag of an element moves it with the rest of the selection as one edit,
//! a left drag on the empty canvas pans the view (`DiagramCanvas.tsx`, the `background` target), a drag from an
//! anchor draws a connection, and a release of one on empty canvas is handed on with its point, and a drag of a
//! selected element's resize handle carries that one edge, the far edge staying put, and is one resize at its release
//! (`DiagramCanvas.tsx`, the `resize` gesture). There is no selection box: the standalone host draws none.
//!
//! The right button is the context menu's (`usePointerGesture.ts`: "the right button is the menu's gesture"): a right
//! press that does not move opens the menu of what it was pressed on. A right drag draws a connection from the
//! element it began on only where the specification asks for one (`connectOnRightDrag`, DISL section 6.10's
//! pointer binding `{ "button": "secondary", "start": "body" }`, [`connects_on_right_drag`]); anywhere else it does nothing.
//!
//! The arbiter knows no toolkit and no diagram: the canvas hit-tests the press and hands it in, and turns the
//! [`Intent`]s it gives back into edits, so every rule here is tested without a window.

use adp_context::SelectMode;
use adp_diagram::ResizedEdge;
use adp_scene::Point;
use serde_json::Value as Json;

/// The movement, in screen points, a press may make and still be a click (`GESTURE_MOVEMENT_THRESHOLD_PX`).
pub const MOVEMENT_THRESHOLD: f32 = 4.0;

/// Which button a gesture was begun with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Primary,
    Secondary,
}

/// The modifiers held when a gesture began.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers {
    /// Ctrl, or Cmd on macOS.
    pub command: bool,
    pub shift: bool,
}

/// What lies under the pointer.
#[derive(Debug, Clone, PartialEq)]
pub enum Hit {
    Empty,
    Element(String),
    Relation(String),
    /// A connection's starting point on an element.
    Anchor {
        element: String,
        anchor: String,
    },
    /// One edge of a selected element that may be resized.
    Resize {
        element: String,
        edge: ResizedEdge,
    },
}

impl Hit {
    /// The element or relation the hit names, an anchor's element included.
    pub fn id(&self) -> Option<&str> {
        match self {
            Hit::Empty => None,
            Hit::Element(id) | Hit::Relation(id) => Some(id),
            Hit::Anchor { element, .. } | Hit::Resize { element, .. } => Some(element),
        }
    }
}

/// A gesture in flight.
#[derive(Debug, Clone, PartialEq)]
pub enum Gesture {
    Idle,
    /// Pressed and not yet moved past the threshold: still a click.
    Pressed {
        button: Button,
        hit: Hit,
        modifiers: Modifiers,
        screen: (f32, f32),
        canvas: Point,
        /// What a drag of this press moves.
        moves: Vec<String>,
        /// The view's origin when the press began, for a pan.
        origin: Point,
    },
    /// Elements dragged by `delta` canvas units.
    Moving {
        ids: Vec<String>,
        from: Point,
        delta: (f64, f64),
    },
    /// One edge of an element dragged by `delta` canvas units along its own axis.
    Resizing {
        element: String,
        edge: ResizedEdge,
        from: Point,
        delta: f64,
    },
    /// A connection drawn from an element towards a point.
    Connecting {
        source: String,
        from: Point,
        to: Point,
    },
    /// The view dragged: the origin it had and the screen point the drag began at.
    Panning {
        origin: Point,
        screen: (f32, f32),
    },
}

/// What a gesture comes to, for the canvas to carry out.
#[derive(Debug, Clone, PartialEq)]
pub enum Intent {
    /// Change the selection; an empty replace clears it.
    Select { ids: Vec<String>, mode: SelectMode },
    /// Move these elements by `delta` canvas units, as one edit.
    Move { ids: Vec<String>, delta: (f64, f64) },
    /// Carry one edge of an element by `delta` canvas units along its axis, as one edit.
    Resize {
        element: String,
        edge: ResizedEdge,
        delta: f64,
    },
    /// Connect `source` to `target`, or to something made at `at` when the release is on empty canvas.
    Connect {
        source: String,
        target: Option<String>,
        at: Point,
    },
    /// Open the context menu of what was pressed, at the screen point.
    ContextMenu {
        hit: Hit,
        screen: (f32, f32),
        canvas: Point,
    },
    /// Look at the canvas from this origin.
    Pan { origin: Point },
}

/// The one arbiter of a canvas.
#[derive(Debug, Clone, PartialEq)]
pub struct Arbiter {
    pub gesture: Gesture,
    /// Whether a right drag from an element draws a connection (`connectOnRightDrag`); without it a right drag does
    /// nothing. The canvas sets it from the diagram it shows.
    pub connects_on_right_drag: bool,
}

impl Default for Arbiter {
    fn default() -> Self {
        Arbiter {
            gesture: Gesture::Idle,
            connects_on_right_drag: false,
        }
    }
}

impl Arbiter {
    pub fn is_idle(&self) -> bool {
        self.gesture == Gesture::Idle
    }

    /// Begins a gesture; a second press while one is in flight starts nothing. `moves` is what a drag of the press
    /// moves (the selection when the press is on a selected element, else the element itself, its movable nodes
    /// only), and `origin` the view's origin now.
    #[allow(clippy::too_many_arguments)]
    pub fn press(
        &mut self,
        button: Button,
        hit: Hit,
        modifiers: Modifiers,
        screen: (f32, f32),
        canvas: Point,
        moves: Vec<String>,
        origin: Point,
    ) {
        if !self.is_idle() {
            return;
        }
        self.gesture = Gesture::Pressed {
            button,
            hit,
            modifiers,
            screen,
            canvas,
            moves,
            origin,
        };
    }

    /// The pointer moved to `screen`, `canvas` on the canvas, with the view at `zoom`; a pan in flight says where
    /// the view goes.
    pub fn motion(&mut self, screen: (f32, f32), canvas: Point, zoom: f64) -> Option<Intent> {
        if let Gesture::Pressed {
            button,
            hit,
            screen: start,
            canvas: from,
            moves,
            origin,
            ..
        } = &self.gesture
        {
            let right_connects = self.connects_on_right_drag;
            let moved = ((screen.0 - start.0).powi(2) + (screen.1 - start.1).powi(2)).sqrt();
            if moved <= MOVEMENT_THRESHOLD {
                return None;
            }
            self.gesture = match (button, hit) {
                (Button::Primary, Hit::Element(_)) if !moves.is_empty() => Gesture::Moving {
                    ids: moves.clone(),
                    from: *from,
                    delta: (0.0, 0.0),
                },
                (Button::Primary, Hit::Resize { element, edge }) => Gesture::Resizing {
                    element: element.clone(),
                    edge: *edge,
                    from: *from,
                    delta: 0.0,
                },
                (Button::Primary, Hit::Empty) => Gesture::Panning {
                    origin: *origin,
                    screen: *start,
                },
                (Button::Primary, Hit::Anchor { element, .. }) => Gesture::Connecting {
                    source: element.clone(),
                    from: *from,
                    to: *from,
                },
                (Button::Secondary, Hit::Element(element) | Hit::Anchor { element, .. }) if right_connects => {
                    Gesture::Connecting {
                        source: element.clone(),
                        from: *from,
                        to: *from,
                    }
                }
                // A relation, an element that does not move, or a right drag nothing claims: dragging it is just not
                // a click, and does nothing.
                _ => Gesture::Moving {
                    ids: Vec::new(),
                    from: *from,
                    delta: (0.0, 0.0),
                },
            };
        }
        match &mut self.gesture {
            Gesture::Moving { from, delta, .. } => {
                *delta = (canvas.x - from.x, canvas.y - from.y);
                None
            }
            Gesture::Resizing { edge, from, delta, .. } => {
                *delta = along(*edge, *from, canvas);
                None
            }
            Gesture::Connecting { to, .. } => {
                *to = canvas;
                None
            }
            Gesture::Panning { origin, screen: start } => Some(Intent::Pan {
                origin: Point::new(
                    origin.x - (screen.0 - start.0) as f64 / zoom,
                    origin.y - (screen.1 - start.1) as f64 / zoom,
                ),
            }),
            _ => None,
        }
    }

    /// The gesture ends where the pointer was released, over `hit`: its verdict.
    pub fn release(&mut self, canvas: Point, hit: &Hit) -> Vec<Intent> {
        let gesture = std::mem::replace(&mut self.gesture, Gesture::Idle);
        match gesture {
            Gesture::Idle => Vec::new(),
            Gesture::Pressed {
                button: Button::Primary,
                hit,
                modifiers,
                ..
            } => match hit.id() {
                Some(id) => vec![Intent::Select {
                    ids: vec![id.to_owned()],
                    mode: if modifiers.command {
                        SelectMode::Toggle
                    } else {
                        SelectMode::Replace
                    },
                }],
                None if modifiers.command => Vec::new(),
                None => vec![Intent::Select {
                    ids: Vec::new(),
                    mode: SelectMode::Replace,
                }],
            },
            Gesture::Pressed {
                button: Button::Secondary,
                hit,
                screen,
                canvas,
                ..
            } => {
                let mut intents = Vec::new();
                if let Some(id) = hit.id() {
                    intents.push(Intent::Select {
                        ids: vec![id.to_owned()],
                        mode: SelectMode::Replace,
                    });
                }
                intents.push(Intent::ContextMenu { hit, screen, canvas });
                intents
            }
            Gesture::Moving { ids, from, .. } => {
                if ids.is_empty() {
                    return Vec::new();
                }
                vec![Intent::Move {
                    ids,
                    delta: (canvas.x - from.x, canvas.y - from.y),
                }]
            }
            Gesture::Resizing {
                element, edge, from, ..
            } => {
                let delta = along(edge, from, canvas);
                if delta == 0.0 {
                    return Vec::new();
                }
                vec![Intent::Resize { element, edge, delta }]
            }
            Gesture::Connecting { source, .. } => {
                let target = match hit {
                    // The source itself is a target too: a self loop, where the relation allows one.
                    Hit::Element(id) | Hit::Anchor { element: id, .. } => Some(id.clone()),
                    _ => None,
                };
                vec![Intent::Connect {
                    source,
                    target,
                    at: canvas,
                }]
            }
            Gesture::Panning { .. } => Vec::new(),
        }
    }

    /// Escape: the gesture in flight dissolves and nothing is written; whether there was one.
    pub fn cancel(&mut self) -> bool {
        let busy = !self.is_idle();
        self.gesture = Gesture::Idle;
        busy
    }

    /// The elements being dragged and by how much, for the preview.
    pub fn moving(&self) -> Option<(&[String], (f64, f64))> {
        match &self.gesture {
            Gesture::Moving { ids, delta, .. } if !ids.is_empty() => Some((ids, *delta)),
            _ => None,
        }
    }

    /// The edge being dragged and by how much, for the preview.
    pub fn resizing(&self) -> Option<(&str, ResizedEdge, f64)> {
        match &self.gesture {
            Gesture::Resizing {
                element, edge, delta, ..
            } => Some((element, *edge, *delta)),
            _ => None,
        }
    }

    /// The pan in flight, so a cancel can put the view back where it was.
    pub fn pan_origin(&self) -> Option<Point> {
        match &self.gesture {
            Gesture::Panning { origin, .. } => Some(*origin),
            _ => None,
        }
    }
}

/// Whether a notation draws a relation by a right drag from an element's body (`compileNotation.ts`,
/// `connectsOnRightDrag`): one of its edges binds its connect gesture to the secondary button, started on the body
/// and with no modifier.
pub fn connects_on_right_drag(notation: &Json) -> bool {
    notation
        .get("edges")
        .and_then(Json::as_object)
        .into_iter()
        .flat_map(|edges| edges.values())
        .filter_map(|edge| edge.pointer("/connect/pointer"))
        .any(|pointer| {
            pointer.get("button").and_then(Json::as_str) == Some("secondary")
                && pointer.get("start").and_then(Json::as_str) == Some("body")
                && pointer
                    .get("modifiers")
                    .and_then(Json::as_array)
                    .is_none_or(|m| m.is_empty())
        })
}

/// How far the pointer went from `from` to `to` along the axis an edge moves on (`alongSide`).
fn along(edge: ResizedEdge, from: Point, to: Point) -> f64 {
    if edge.is_horizontal() {
        to.x - from.x
    } else {
        to.y - from.y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn pressed(arbiter: &mut Arbiter, button: Button, hit: Hit, modifiers: Modifiers, moves: &[&str]) {
        arbiter.press(
            button,
            hit,
            modifiers,
            (100.0, 100.0),
            p(10.0, 10.0),
            moves.iter().map(|s| (*s).to_owned()).collect(),
            p(0.0, 0.0),
        );
    }

    #[test]
    fn a_press_that_moves_no_more_than_the_threshold_is_a_click_and_selects() {
        let mut arbiter = Arbiter::default();
        pressed(
            &mut arbiter,
            Button::Primary,
            Hit::Element("a".into()),
            Modifiers::default(),
            &["a"],
        );
        arbiter.motion((103.0, 102.0), p(10.3, 10.2), 10.0);
        assert_eq!(
            arbiter.release(p(10.3, 10.2), &Hit::Element("a".into())),
            vec![Intent::Select {
                ids: vec!["a".into()],
                mode: SelectMode::Replace
            }]
        );
        assert!(arbiter.is_idle());
    }

    #[test]
    fn ctrl_toggles_and_a_click_on_empty_canvas_clears_unless_ctrl_is_held() {
        let ctrl = Modifiers {
            command: true,
            shift: false,
        };
        let mut arbiter = Arbiter::default();
        pressed(&mut arbiter, Button::Primary, Hit::Element("a".into()), ctrl, &["a"]);
        assert_eq!(
            arbiter.release(p(10.0, 10.0), &Hit::Empty),
            vec![Intent::Select {
                ids: vec!["a".into()],
                mode: SelectMode::Toggle
            }]
        );
        pressed(&mut arbiter, Button::Primary, Hit::Empty, Modifiers::default(), &[]);
        assert_eq!(
            arbiter.release(p(10.0, 10.0), &Hit::Empty),
            vec![Intent::Select {
                ids: vec![],
                mode: SelectMode::Replace
            }]
        );
        pressed(&mut arbiter, Button::Primary, Hit::Empty, ctrl, &[]);
        assert!(arbiter.release(p(10.0, 10.0), &Hit::Empty).is_empty());
    }

    #[test]
    fn a_drag_moves_everything_it_was_given_as_one_intent_and_escape_moves_nothing() {
        let mut arbiter = Arbiter::default();
        pressed(
            &mut arbiter,
            Button::Primary,
            Hit::Element("a".into()),
            Modifiers::default(),
            &["a", "b"],
        );
        arbiter.motion((140.0, 100.0), p(14.0, 10.0), 10.0);
        assert_eq!(
            arbiter.moving(),
            Some((&["a".to_owned(), "b".to_owned()][..], (4.0, 0.0)))
        );
        assert_eq!(
            arbiter.release(p(15.0, 12.0), &Hit::Empty),
            vec![Intent::Move {
                ids: vec!["a".into(), "b".into()],
                delta: (5.0, 2.0)
            }]
        );

        pressed(
            &mut arbiter,
            Button::Primary,
            Hit::Element("a".into()),
            Modifiers::default(),
            &["a"],
        );
        arbiter.motion((140.0, 100.0), p(14.0, 10.0), 10.0);
        assert!(arbiter.cancel());
        assert!(
            arbiter.release(p(15.0, 12.0), &Hit::Empty).is_empty(),
            "the release after Escape is nothing"
        );
    }

    #[test]
    fn a_left_drag_on_empty_canvas_pans_selects_nothing_and_escape_hands_back_where_it_began() {
        let mut arbiter = Arbiter::default();
        pressed(&mut arbiter, Button::Primary, Hit::Empty, Modifiers::default(), &[]);
        assert_eq!(
            arbiter.motion((150.0, 80.0), p(15.0, 8.0), 2.0),
            Some(Intent::Pan { origin: p(-25.0, 10.0) })
        );
        assert_eq!(arbiter.pan_origin(), Some(p(0.0, 0.0)));
        assert!(
            arbiter.release(p(15.0, 8.0), &Hit::Empty).is_empty(),
            "a pan neither selects nor clears"
        );

        let ctrl = Modifiers {
            command: true,
            shift: false,
        };
        pressed(&mut arbiter, Button::Primary, Hit::Empty, ctrl, &[]);
        assert!(
            matches!(
                arbiter.motion((200.0, 200.0), p(20.0, 20.0), 10.0),
                Some(Intent::Pan { .. })
            ),
            "Ctrl draws no box either: the drag pans"
        );
        assert_eq!(arbiter.pan_origin(), Some(p(0.0, 0.0)));
        assert!(arbiter.cancel());
        assert!(arbiter.release(p(20.0, 20.0), &Hit::Empty).is_empty());
    }

    #[test]
    fn a_right_click_opens_the_menu_and_a_right_drag_does_nothing_unless_the_diagram_connects_on_it() {
        let mut arbiter = Arbiter::default();
        pressed(
            &mut arbiter,
            Button::Secondary,
            Hit::Relation("r".into()),
            Modifiers::default(),
            &[],
        );
        assert_eq!(
            arbiter.release(p(10.0, 10.0), &Hit::Empty),
            vec![
                Intent::Select {
                    ids: vec!["r".into()],
                    mode: SelectMode::Replace
                },
                Intent::ContextMenu {
                    hit: Hit::Relation("r".into()),
                    screen: (100.0, 100.0),
                    canvas: p(10.0, 10.0)
                }
            ]
        );
        pressed(&mut arbiter, Button::Secondary, Hit::Empty, Modifiers::default(), &[]);
        assert!(matches!(
            arbiter.release(p(10.0, 10.0), &Hit::Empty).as_slice(),
            [Intent::ContextMenu { hit: Hit::Empty, .. }]
        ));

        for hit in [Hit::Empty, Hit::Element("a".into()), Hit::Relation("r".into())] {
            pressed(&mut arbiter, Button::Secondary, hit, Modifiers::default(), &["a"]);
            assert_eq!(arbiter.motion((180.0, 100.0), p(18.0, 10.0), 10.0), None, "no pan");
            assert_eq!(arbiter.moving(), None, "no move");
            assert_eq!(arbiter.pan_origin(), None);
            assert!(
                arbiter.release(p(30.0, 10.0), &Hit::Element("b".into())).is_empty(),
                "no menu, no connection, no selection"
            );
        }
    }

    #[test]
    fn a_drag_from_an_anchor_or_a_right_drag_from_an_element_where_the_diagram_asks_connects_to_what_it_is_released_on()
    {
        let mut arbiter = Arbiter::default();
        let anchor = Hit::Anchor {
            element: "a".into(),
            anchor: "right".into(),
        };
        pressed(&mut arbiter, Button::Primary, anchor, Modifiers::default(), &["a"]);
        arbiter.motion((180.0, 100.0), p(18.0, 10.0), 10.0);
        assert_eq!(
            arbiter.release(p(30.0, 10.0), &Hit::Element("b".into())),
            vec![Intent::Connect {
                source: "a".into(),
                target: Some("b".into()),
                at: p(30.0, 10.0)
            }]
        );
        arbiter.connects_on_right_drag = true;
        pressed(
            &mut arbiter,
            Button::Secondary,
            Hit::Element("a".into()),
            Modifiers::default(),
            &["a"],
        );
        arbiter.motion((180.0, 100.0), p(18.0, 10.0), 10.0);
        assert_eq!(
            arbiter.release(p(50.0, 50.0), &Hit::Empty),
            vec![Intent::Connect {
                source: "a".into(),
                target: None,
                at: p(50.0, 50.0)
            }]
        );
        pressed(&mut arbiter, Button::Secondary, Hit::Empty, Modifiers::default(), &[]);
        arbiter.motion((180.0, 100.0), p(18.0, 10.0), 10.0);
        assert!(
            arbiter.release(p(18.0, 10.0), &Hit::Empty).is_empty(),
            "a right drag on empty canvas still does nothing"
        );
        pressed(
            &mut arbiter,
            Button::Secondary,
            Hit::Element("a".into()),
            Modifiers::default(),
            &["a"],
        );
        assert!(
            matches!(
                arbiter.release(p(10.0, 10.0), &Hit::Empty).last(),
                Some(Intent::ContextMenu { .. })
            ),
            "a right click on the element still opens its menu"
        );
    }

    #[test]
    fn a_notation_connects_on_a_right_drag_only_by_a_secondary_body_pointer_without_modifiers() {
        let edge = |pointer: Json| serde_json::json!({ "edges": { "Link": { "connect": { "pointer": pointer } } } });
        assert!(connects_on_right_drag(&edge(
            serde_json::json!({ "button": "secondary", "start": "body", "doc": "…" })
        )));
        assert!(!connects_on_right_drag(&edge(
            serde_json::json!({ "button": "secondary", "start": "anchor" })
        )));
        assert!(!connects_on_right_drag(&edge(
            serde_json::json!({ "button": "primary", "start": "body" })
        )));
        assert!(!connects_on_right_drag(&edge(
            serde_json::json!({ "button": "secondary", "start": "body", "modifiers": ["Alt"] })
        )));
        assert!(!connects_on_right_drag(
            &serde_json::json!({ "edges": { "Link": { "connect": { "from": { "right": "source" } } } } })
        ));
        assert!(!connects_on_right_drag(&serde_json::json!({})));
    }

    #[test]
    fn a_drag_of_a_resize_handle_carries_its_edge_along_its_axis_and_escape_writes_nothing() {
        let mut arbiter = Arbiter::default();
        let handle = Hit::Resize {
            element: "a".into(),
            edge: ResizedEdge::Right,
        };
        pressed(
            &mut arbiter,
            Button::Primary,
            handle.clone(),
            Modifiers::default(),
            &["a"],
        );
        arbiter.motion((160.0, 130.0), p(16.0, 13.0), 10.0);
        assert_eq!(
            arbiter.resizing(),
            Some(("a", ResizedEdge::Right, 6.0)),
            "only the x of the drag counts"
        );
        assert_eq!(
            arbiter.release(p(18.0, 20.0), &Hit::Empty),
            vec![Intent::Resize {
                element: "a".into(),
                edge: ResizedEdge::Right,
                delta: 8.0
            }]
        );

        let bottom = Hit::Resize {
            element: "a".into(),
            edge: ResizedEdge::Bottom,
        };
        pressed(
            &mut arbiter,
            Button::Primary,
            bottom.clone(),
            Modifiers::default(),
            &["a"],
        );
        arbiter.motion((100.0, 160.0), p(10.0, 16.0), 10.0);
        assert_eq!(arbiter.resizing(), Some(("a", ResizedEdge::Bottom, 6.0)));
        assert!(arbiter.cancel());
        assert!(arbiter.release(p(10.0, 16.0), &Hit::Empty).is_empty());

        pressed(&mut arbiter, Button::Primary, handle, Modifiers::default(), &["a"]);
        assert_eq!(
            arbiter.release(p(10.0, 10.0), &Hit::Empty),
            vec![Intent::Select {
                ids: vec!["a".into()],
                mode: SelectMode::Replace
            }],
            "a click on a handle selects its element"
        );
    }
}
