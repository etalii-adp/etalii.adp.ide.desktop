//! Which open view of a diagram a viewport report or a move belongs to (etalii.adp spec 030, contracts/core-shell.md,
//! `set_viewport` and `move_elements`); ported from the standalone host's `EtAlii.Adp.Diagram/DiagramViewportRegistry.cs`
//! and `IDiagramViewportRegistry.cs` at `8b6ccfc`.
//!
//! More than one view may hold a key at once: a window may open a second view of a diagram before the first has
//! closed, and either may close first. So each key keeps every open view's registration, in the order they opened,
//! and answers with the latest one still open; a view that closes removes its own registration and no other. The key
//! is the window and the body's path, compared without regard to case.

use std::collections::HashMap;
use std::path::Path;

/// The open views of one diagram in one window, in the order they opened, each with what its reports reach.
type Registrations<S, V> = Vec<(S, Box<dyn FnMut(V)>)>;

/// The open views of diagrams, by window and body; `S` is what a view is known by (a session id, a handle).
pub struct DiagramViewportRegistry<S, V> {
    by_view: HashMap<(String, String), Registrations<S, V>>,
}

impl<S, V> Default for DiagramViewportRegistry<S, V> {
    fn default() -> Self {
        DiagramViewportRegistry {
            by_view: HashMap::new(),
        }
    }
}

fn key(window: &str, body_path: &Path) -> (String, String) {
    (window.to_owned(), body_path.to_string_lossy().to_uppercase())
}

impl<S: PartialEq + Clone, V> DiagramViewportRegistry<S, V> {
    /// An open view registers its session and what its viewport reports should reach.
    pub fn register(&mut self, window: &str, body_path: &Path, session: S, on_reported: impl FnMut(V) + 'static) {
        self.by_view
            .entry(key(window, body_path))
            .or_default()
            .push((session, Box::new(on_reported)));
    }

    /// Delivers a reported viewport to the latest open view of the diagram; false when there is none.
    pub fn report(&mut self, window: &str, body_path: &Path, viewport: V) -> bool {
        match self.by_view.get_mut(&key(window, body_path)).and_then(|r| r.last_mut()) {
            Some((_, on_reported)) => {
                on_reported(viewport);
                true
            }
            None => false,
        }
    }

    /// The latest open session behind a call that names the diagram, or none when the window has no view of it.
    pub fn find(&self, window: &str, body_path: &Path) -> Option<S> {
        self.by_view
            .get(&key(window, body_path))
            .and_then(|r| r.last())
            .map(|(session, _)| session.clone())
    }

    /// The view is closing: drops its own registration, the last it made, and no other.
    pub fn remove(&mut self, window: &str, body_path: &Path, session: &S) {
        let key = key(window, body_path);
        let Some(registrations) = self.by_view.get_mut(&key) else {
            return;
        };
        if let Some(index) = registrations.iter().rposition(|(s, _)| s == session) {
            let _ = registrations.remove(index);
        }
        if registrations.is_empty() {
            self.by_view.remove(&key);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;
    use crate::element::DiagramViewport;

    const WINDOW: &str = "window-1";
    const BODY: &str = r"C:\project\diagram.ghg";

    #[test]
    fn a_closing_view_leaves_the_registration_of_a_view_that_replaced_it() {
        let mut registry = DiagramViewportRegistry::<u32, DiagramViewport>::default();
        registry.register(WINDOW, Path::new(BODY), 1, |_| {});
        registry.register(WINDOW, Path::new(BODY), 2, |_| {});
        registry.remove(WINDOW, Path::new(BODY), &1);
        assert_eq!(registry.find(WINDOW, Path::new(BODY)), Some(2));
    }

    #[test]
    fn a_later_view_closing_first_leaves_the_earlier_one_findable_and_reached_by_view_reports() {
        let mut registry = DiagramViewportRegistry::<u32, DiagramViewport>::default();
        let reported = Rc::new(Cell::new(0));
        let counter = reported.clone();
        registry.register(WINDOW, Path::new(BODY), 1, move |_| counter.set(counter.get() + 1));
        registry.register(WINDOW, Path::new(BODY), 2, |_| {});
        registry.remove(WINDOW, Path::new(BODY), &2);
        assert_eq!(registry.find(WINDOW, Path::new(BODY)), Some(1));
        assert!(registry.report(
            WINDOW,
            Path::new(&BODY.to_lowercase()),
            DiagramViewport::new(0.0, 0.0, 1.0, 1.0)
        ));
        assert_eq!(reported.get(), 1);
    }

    #[test]
    fn a_closing_view_removes_its_own_registration() {
        let mut registry = DiagramViewportRegistry::<u32, DiagramViewport>::default();
        registry.register(WINDOW, Path::new(BODY), 1, |_| {});
        registry.remove(WINDOW, Path::new(BODY), &1);
        assert_eq!(registry.find(WINDOW, Path::new(BODY)), None);
        assert!(!registry.report(WINDOW, Path::new(BODY), DiagramViewport::UNBOUNDED));
    }
}
