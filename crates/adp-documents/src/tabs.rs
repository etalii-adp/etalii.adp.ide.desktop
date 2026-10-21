//! The open tabs of a window (the standalone host's diagram-workspace-tabs at `8b6ccfc`, FR-014): opening a document
//! opens it in a tab or focuses the tab it already has; a tab shows unsaved changes; closing a tab with unsaved changes
//! asks first.

use adp_context::Prompt;

/// One open tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tab {
    pub id: u64,
    /// The document's path relative to the workspace, with `/`.
    pub path: String,
    /// The tool it is open with: an origin such as `freeplane/mindmap`, or `editor/plain` for text.
    pub tool: String,
    pub dirty: bool,
}

impl Tab {
    /// The tab's title: the file name, with `●` in front while there are unsaved changes, as VS Code shows it.
    pub fn title(&self) -> String {
        let name = self.path.rsplit('/').next().unwrap_or(&self.path);
        if self.dirty {
            format!("● {name}")
        } else {
            name.to_owned()
        }
    }
}

/// What closing a tab did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Closing {
    Closed,
    /// The tab has unsaved changes; the person is asked, and [`Tabs::close`] with `discard` follows the answer.
    Ask(Prompt),
    NotOpen,
}

/// A window's tabs, in the order they were opened, with one of them active.
#[derive(Debug, Default)]
pub struct Tabs {
    tabs: Vec<Tab>,
    active: Option<u64>,
    next: u64,
}

impl Tabs {
    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    pub fn active(&self) -> Option<&Tab> {
        self.active.and_then(|id| self.tabs.iter().find(|t| t.id == id))
    }

    /// Opens a document with a tool, or focuses the tab it already has with that tool. Returns the tab's id.
    pub fn open(&mut self, path: &str, tool: &str) -> u64 {
        if let Some(tab) = self.tabs.iter().find(|t| t.path == path && t.tool == tool) {
            self.active = Some(tab.id);
            return tab.id;
        }
        self.next += 1;
        let id = self.next;
        self.tabs.push(Tab {
            id,
            path: path.into(),
            tool: tool.into(),
            dirty: false,
        });
        self.active = Some(id);
        id
    }

    pub fn focus(&mut self, id: u64) -> bool {
        let found = self.tabs.iter().any(|t| t.id == id);
        if found {
            self.active = Some(id);
        }
        found
    }

    pub fn set_dirty(&mut self, id: u64, dirty: bool) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            tab.dirty = dirty;
        }
    }

    /// Closes a tab. With unsaved changes and `discard` false, it asks instead; the tab to the right, else the left,
    /// becomes active.
    pub fn close(&mut self, id: u64, discard: bool) -> Closing {
        let Some(index) = self.tabs.iter().position(|t| t.id == id) else {
            return Closing::NotOpen;
        };
        if self.tabs[index].dirty && !discard {
            let name = self.tabs[index].path.rsplit('/').next().unwrap_or_default().to_owned();
            return Closing::Ask(Prompt::Confirm {
                title: "Unsaved changes".into(),
                icon: String::new(),
                message: format!("'{name}' has changes that are not saved. Close it and lose them?"),
                confirm_label: "Close without saving".into(),
                danger: true,
            });
        }
        self.tabs.remove(index);
        if self.active == Some(id) {
            self.active = self
                .tabs
                .get(index)
                .or_else(|| index.checked_sub(1).and_then(|i| self.tabs.get(i)))
                .map(|t| t.id);
        }
        Closing::Closed
    }

    /// Follows a rename on disk: every tab of the old path, or under the old folder, takes the new one.
    pub fn renamed(&mut self, from: &str, to: &str) {
        for tab in &mut self.tabs {
            if tab.path == from {
                tab.path = to.into();
            } else if let Some(rest) = tab.path.strip_prefix(&format!("{from}/")) {
                tab.path = format!("{to}/{rest}");
            }
        }
    }
}
