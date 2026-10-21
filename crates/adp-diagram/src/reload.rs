//! Which open diagrams a change on disk reaches (etalii.adp spec 030, FR-023, SC-008); ported from the standalone
//! host's `EtAlii.Adp.Workspace/DiagramDocumentReloadBridge.cs` and `EtAlii.Adp.Diagram/IDiagramDocumentReloader.cs`
//! at `8b6ccfc`.
//!
//! Every opened diagram is tracked by its body, its registration and the project it is in. A change to a tracked body
//! reloads it; a change to a registration reloads the body it registers, since the body is what the views show; a
//! body deleted, or renamed away, is reported as gone rather than as a reload, so a document that keeps its last good
//! text through a read that fails can still tell the two apart; when the watcher lost events, every tracked body of
//! the project is read again, or reported gone when it is no longer there. A tool type without a reloader is left
//! alone. Tracking outlives the views, as the documents do. The standalone host's watcher normalised a save that
//! replaces a file into one change of it; the events here are taken as already normalised that way. The bridge does
//! not reload anything itself: it says what to reload, and the caller hands each to the workspace's `reconcile`.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// One file event, as the project's watcher reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileEvent {
    Created(PathBuf),
    Changed(PathBuf),
    Deleted(PathBuf),
    Renamed {
        from: PathBuf,
        to: PathBuf,
    },
    /// The watcher lost events: what it allowed to go stale must be read again.
    Lost,
}

/// What the bridge asks of a tool type's documents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reload {
    /// Read the body again.
    Reload {
        root: PathBuf,
        body: PathBuf,
        origin: String,
    },
    /// The body is gone.
    BodyDeleted {
        root: PathBuf,
        body: PathBuf,
        origin: String,
    },
}

#[derive(Debug, Clone)]
struct Tracked {
    root: PathBuf,
    body: PathBuf,
    origin: String,
}

fn key(path: &Path) -> String {
    path.to_string_lossy().to_lowercase()
}

/// The tracked diagrams and the reloads a file event means for them.
#[derive(Debug, Default)]
pub struct DiagramDocumentReloadBridge {
    reloaders: HashSet<String>,
    documents: HashMap<String, Tracked>,
    registrations: HashMap<String, String>,
}

impl DiagramDocumentReloadBridge {
    /// A bridge for the tool types, by origin, that have a reloader.
    pub fn new(origins: impl IntoIterator<Item = String>) -> Self {
        DiagramDocumentReloadBridge {
            reloaders: origins.into_iter().collect(),
            ..DiagramDocumentReloadBridge::default()
        }
    }

    /// Starts caring about one opened diagram: its body, the registration it was opened through, and its project.
    pub fn track(&mut self, root: &Path, body: &Path, registration: Option<&Path>, origin: &str) {
        if !self.reloaders.contains(origin) {
            return;
        }
        self.documents.insert(
            key(body),
            Tracked {
                root: root.to_path_buf(),
                body: body.to_path_buf(),
                origin: origin.to_owned(),
            },
        );
        if let Some(registration) = registration {
            self.registrations.insert(key(registration), key(body));
        }
    }

    /// What one file event under the project at `root` means; `exists` tells whether a body is still there when the
    /// watcher lost events and has no event to say so.
    pub fn on_change(&self, root: &Path, event: &FileEvent, exists: impl Fn(&Path) -> bool) -> Vec<Reload> {
        let mut reloads = Vec::new();
        let (old, new) = match event {
            FileEvent::Lost => {
                for tracked in self.documents.values().filter(|t| key(&t.root) == key(root)) {
                    reloads.push(if exists(&tracked.body) {
                        reload(tracked)
                    } else {
                        deleted(tracked)
                    });
                }
                reloads.sort_by(|a, b| body_of(a).cmp(body_of(b)));
                return reloads;
            }
            FileEvent::Deleted(path) => (Some(path), None),
            FileEvent::Renamed { from, to } => (Some(from), Some(to)),
            FileEvent::Created(path) | FileEvent::Changed(path) => (None, Some(path)),
        };
        if let Some(old) = old {
            match self.documents.get(&key(old)) {
                Some(tracked) => reloads.push(deleted(tracked)),
                None => reloads.extend(self.reload_if_tracked(old)),
            }
        }
        if let Some(new) = new
            && old.is_none_or(|old| key(old) != key(new))
        {
            reloads.extend(self.reload_if_tracked(new));
        }
        reloads
    }

    fn reload_if_tracked(&self, path: &Path) -> Option<Reload> {
        let path = key(path);
        let tracked = self
            .documents
            .get(&path)
            .or_else(|| self.registrations.get(&path).and_then(|body| self.documents.get(body)))?;
        Some(reload(tracked))
    }
}

fn reload(tracked: &Tracked) -> Reload {
    Reload::Reload {
        root: tracked.root.clone(),
        body: tracked.body.clone(),
        origin: tracked.origin.clone(),
    }
}

fn deleted(tracked: &Tracked) -> Reload {
    Reload::BodyDeleted {
        root: tracked.root.clone(),
        body: tracked.body.clone(),
        origin: tracked.origin.clone(),
    }
}

fn body_of(reload: &Reload) -> &Path {
    match reload {
        Reload::Reload { body, .. } | Reload::BodyDeleted { body, .. } => body,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORIGIN: &str = "test/sample";

    fn bridge() -> (DiagramDocumentReloadBridge, PathBuf, PathBuf) {
        let root = PathBuf::from("/project");
        let body = root.join("sample.dsl");
        let mut bridge = DiagramDocumentReloadBridge::new([ORIGIN.to_owned()]);
        bridge.track(&root, &body, None, ORIGIN);
        (bridge, root, body)
    }

    fn reloaded(root: &Path, body: &Path) -> Reload {
        Reload::Reload {
            root: root.to_path_buf(),
            body: body.to_path_buf(),
            origin: ORIGIN.into(),
        }
    }

    fn gone(root: &Path, body: &Path) -> Reload {
        Reload::BodyDeleted {
            root: root.to_path_buf(),
            body: body.to_path_buf(),
            origin: ORIGIN.into(),
        }
    }

    #[test]
    fn a_tracked_body_that_is_deleted_is_reported_as_deleted_not_reloaded() {
        let (bridge, root, body) = bridge();
        assert_eq!(
            bridge.on_change(&root, &FileEvent::Deleted(body.clone()), |_| false),
            [gone(&root, &body)]
        );
    }

    #[test]
    fn a_tracked_body_renamed_away_is_reported_as_deleted() {
        let (bridge, root, body) = bridge();
        let event = FileEvent::Renamed {
            from: body.clone(),
            to: root.join("renamed.dsl"),
        };
        assert_eq!(bridge.on_change(&root, &event, |_| true), [gone(&root, &body)]);
    }

    #[test]
    fn a_save_that_replaces_the_body_reloads_it_and_is_never_reported_as_deleted() {
        let (bridge, root, body) = bridge();
        let event = FileEvent::Renamed {
            from: root.join("sample.dsl.tmp"),
            to: body.clone(),
        };
        assert_eq!(bridge.on_change(&root, &event, |_| true), [reloaded(&root, &body)]);
    }

    #[test]
    fn an_external_write_to_a_tracked_body_reaches_the_reloader() {
        let (bridge, root, body) = bridge();
        let upper = PathBuf::from(body.to_string_lossy().to_uppercase());
        assert_eq!(
            bridge.on_change(&root, &FileEvent::Changed(upper), |_| true),
            [reloaded(&root, &body)]
        );
    }

    #[test]
    fn a_write_to_the_registration_reloads_its_body() {
        let root = PathBuf::from("/project");
        let body = root.join("sample.dsl");
        let registration = root.join("sample.adp");
        let mut bridge = DiagramDocumentReloadBridge::new([ORIGIN.to_owned()]);
        bridge.track(&root, &body, Some(&registration), ORIGIN);
        assert_eq!(
            bridge.on_change(&root, &FileEvent::Changed(registration), |_| true),
            [reloaded(&root, &body)]
        );
    }

    #[test]
    fn a_write_to_an_untracked_file_reloads_nothing() {
        let (bridge, root, _) = bridge();
        assert!(
            bridge
                .on_change(&root, &FileEvent::Changed(root.join("unrelated.txt")), |_| true)
                .is_empty()
        );
    }

    #[test]
    fn an_origin_without_a_reloader_is_left_alone() {
        let root = PathBuf::from("/project");
        let body = root.join("other.yml");
        let mut bridge = DiagramDocumentReloadBridge::new([ORIGIN.to_owned()]);
        bridge.track(&root, &body, None, "test/unregistered");
        assert!(bridge.on_change(&root, &FileEvent::Changed(body), |_| true).is_empty());
    }

    #[test]
    fn lost_events_read_every_tracked_body_of_the_project_again() {
        let root = PathBuf::from("/project");
        let kept = root.join("a.dsl");
        let gone_body = root.join("b.dsl");
        let mut bridge = DiagramDocumentReloadBridge::new([ORIGIN.to_owned()]);
        bridge.track(&root, &kept, None, ORIGIN);
        bridge.track(&root, &gone_body, None, ORIGIN);
        bridge.track(Path::new("/elsewhere"), Path::new("/elsewhere/c.dsl"), None, ORIGIN);
        let reloads = bridge.on_change(&root, &FileEvent::Lost, |path| path == kept);
        assert_eq!(reloads, [reloaded(&root, &kept), gone(&root, &gone_body)]);
    }
}
