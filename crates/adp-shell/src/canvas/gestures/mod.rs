//! The canvas's one gesture arbiter (etalii.adp spec 030, FR-018, Story 2 scenario 3, task T056): every press on the
//! canvas, of either button, is one gesture, decided once, at its end, as a click or a drag, and Escape dissolves
//! whatever is in flight without writing anything.
//!
//! Ported from the standalone host's `src/client/src/canvas/gesture/usePointerGesture.ts` at `8b6ccfc`: one gesture
//! at a time, the same movement threshold (more than 4 pixels by hypotenuse turns a press into a drag), the verdict
//! taken from what the gesture recorded rather than from a trailing click, and an abandon that reverts the preview.
//! What each verdict means is `DiagramCanvas.tsx`'s: a press on an element selects it (Ctrl toggles it), a press on
//! the empty canvas clears the selection, a drag of an element moves it with the rest of the selection as one edit,
//! a drag from an anchor, or a right-drag from an element, draws a connection, and a release of one on empty canvas
//! is handed on with its point. Two gestures follow this host's task list rather than the standalone's left-drag pan:
//! a left drag on empty canvas draws a selection box, and a right drag on it pans.
//!
//! The arbiter knows no toolkit and no diagram: the canvas hit-tests the press and hands it in, and turns the
//! [`Intent`]s it gives back into edits, so every rule here is tested without a window.

use adp_context::SelectMode;
use adp_scene::{Point, Rect};

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
}

impl Hit {
    /// The element or relation the hit names, an anchor's element included.
    pub fn id(&self) -> Option<&str> {
        match self {
            Hit::Empty => None,
            Hit::Element(id) | Hit::Relation(id) => Some(id),
            Hit::Anchor { element, .. } => Some(element),
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
    /// A selection box from one corner to the other.
    Boxing {
        from: Point,
        to: Point,
        additive: bool,
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
    /// Select what lies wholly inside `rect`, added to the selection when `additive`.
    BoxSelect { rect: Rect, additive: bool },
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
}

impl Default for Arbiter {
    fn default() -> Self {
        Arbiter { gesture: Gesture::Idle }
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
            modifiers,
            screen: start,
            canvas: from,
            moves,
            origin,
        } = &self.gesture
        {
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
                (Button::Primary, Hit::Empty) => Gesture::Boxing {
                    from: *from,
                    to: *from,
                    additive: modifiers.command || modifiers.shift,
                },
                (Button::Primary, Hit::Anchor { element, .. }) | (Button::Secondary, Hit::Element(element)) => {
                    Gesture::Connecting {
                        source: element.clone(),
                        from: *from,
                        to: *from,
                    }
                }
                (Button::Secondary, _) => Gesture::Panning {
                    origin: *origin,
                    screen: *start,
                },
                // A relation, or an element that does not move: dragging it is just not a click.
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
            Gesture::Boxing { to, .. } | Gesture::Connecting { to, .. } => {
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
            Gesture::Boxing { from, additive, .. } => vec![Intent::BoxSelect {
                rect: Rect::around([from, canvas]).unwrap_or_default(),
                additive,
            }],
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

    /// The pan in flight, so a cancel can put the view back where it was.
    pub fn pan_origin(&self) -> Option<Point> {
        match &self.gesture {
            Gesture::Panning { origin, .. } => Some(*origin),
            _ => None,
        }
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
    fn a_drag_on_empty_canvas_boxes_a_right_drag_pans_and_a_right_click_opens_the_menu() {
        let mut arbiter = Arbiter::default();
        pressed(&mut arbiter, Button::Primary, Hit::Empty, Modifiers::default(), &[]);
        arbiter.motion((200.0, 200.0), p(20.0, 20.0), 10.0);
        assert_eq!(
            arbiter.release(p(20.0, 30.0), &Hit::Empty),
            vec![Intent::BoxSelect {
                rect: Rect::new(10.0, 10.0, 10.0, 20.0),
                additive: false
            }]
        );

        pressed(&mut arbiter, Button::Secondary, Hit::Empty, Modifiers::default(), &[]);
        assert_eq!(
            arbiter.motion((150.0, 80.0), p(15.0, 8.0), 2.0),
            Some(Intent::Pan { origin: p(-25.0, 10.0) })
        );
        assert!(arbiter.release(p(15.0, 8.0), &Hit::Empty).is_empty());

        pressed(
            &mut arbiter,
            Button::Secondary,
            Hit::Relation("r".into()),
            Modifiers::default(),
            &[],
        );
        let intents = arbiter.release(p(10.0, 10.0), &Hit::Empty);
        assert!(matches!(
            intents.last(),
            Some(Intent::ContextMenu {
                hit: Hit::Relation(_),
                ..
            })
        ));
    }

    #[test]
    fn a_drag_from_an_anchor_or_a_right_drag_from_an_element_connects_to_what_it_is_released_on() {
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
    }
}
