//! Collapse and expand (etalii.adp spec 030, FR-018), ported from the standalone host's
//! `src/diagrams/mindmap/backend/EtAlii.Adp.Diagram.Mindmap/MindmapConnectionView.cs` at `8b6ccfc`, with the mind
//! map's node type replaced by the element's id and a way to find its parent.
//!
//! Which branches a viewer has folded is viewer state: it is kept per viewer and per diagram, never written to the
//! diagram's file, so two viewers fold independently and a fold leaves nothing on the history (mindmap Requirements
//! 9.4 and 9.6). It is seeded from the folds the file itself states (Requirement 9.3). A folded element's descendants
//! are not laid out at all, so a hidden element occupies no space, and an element a reader selects has its folded
//! ancestors opened, outermost first, so it is visible (Requirement 10.5).

use std::collections::HashSet;

/// Which elements one viewer has folded.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FoldState {
    folded: HashSet<String>,
}

impl FoldState {
    /// Nothing folded.
    pub fn new() -> Self {
        Self::default()
    }

    /// The folds a file states, as a viewer first sees them.
    pub fn seeded_from<S: AsRef<str>>(folded: impl IntoIterator<Item = S>) -> Self {
        let mut state = Self::new();
        for id in folded {
            state.set_folded(id.as_ref(), true);
        }
        state
    }

    pub fn is_folded(&self, id: &str) -> bool {
        self.folded.contains(id)
    }

    fn set_folded(&mut self, id: &str, folded: bool) {
        if folded {
            self.folded.insert(id.to_string());
        } else {
            self.folded.remove(id);
        }
    }

    /// Folds an expanded element or expands a folded one, and says whether it is folded now.
    pub fn toggle(&mut self, id: &str) -> bool {
        let now = !self.is_folded(id);
        self.set_folded(id, now);
        now
    }

    /// Every folded ancestor of `id`, outermost first: what has to open for it to be visible. `parent_of` gives an
    /// element's parent, or `None` for a root.
    pub fn folded_ancestors_of<'a>(&self, id: &str, parent_of: impl Fn(&str) -> Option<&'a str>) -> Vec<String> {
        let mut ancestors = Vec::new();
        let mut current = parent_of(id);
        while let Some(ancestor) = current {
            if self.is_folded(ancestor) {
                ancestors.insert(0, ancestor.to_string());
            }
            current = parent_of(ancestor);
        }
        ancestors
    }

    /// Whether `id` lies inside a folded branch, so it is not laid out or drawn. A folded element itself is visible;
    /// only what it holds is hidden.
    pub fn is_hidden<'a>(&self, id: &str, parent_of: impl Fn(&str) -> Option<&'a str>) -> bool {
        let mut current = parent_of(id);
        while let Some(ancestor) = current {
            if self.is_folded(ancestor) {
                return true;
            }
            current = parent_of(ancestor);
        }
        false
    }
}
