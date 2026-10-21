//! A node's link in its three forms (etalii.adp spec 030, FR-018; mindmap Requirement 12.3), ported from the standalone
//! host's `MindmapLinks.cs` at `8b6ccfc`: stored relative to the map in the node's `LINK`, because that is what
//! Freeplane defines the attribute to be; picked relative to the project, because the shell is never given a place
//! outside it; and rewritten when the map moves to another folder (Requirement 12.14).

use std::path::{Component, Path, PathBuf};

use crate::document::MindmapDocument;

/// Whether `link` names something outside the project, a URL say, which is shown but never resolved to a file.
pub fn is_external(link: &str) -> bool {
    let Some((scheme, rest)) = link.split_once(':') else {
        return false;
    };
    let is_scheme = scheme.len() > 1
        && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    // A drive letter is one character; `file:` is a file.
    is_scheme && !scheme.eq_ignore_ascii_case("file") && !rest.is_empty()
}

/// The full path a map-relative `link` in the map at `body_path` names, or none when it leaves the project at
/// `root` (Requirement 12.9) or is external.
pub fn resolve_within_project(body_path: &Path, link: &str, root: &Path) -> Option<PathBuf> {
    if is_external(link) {
        return None;
    }
    let folder = body_path.parent().unwrap_or(root);
    let candidate = normalise(&folder.join(link.replace('/', std::path::MAIN_SEPARATOR_STR)));
    candidate.starts_with(normalise(root)).then_some(candidate)
}

/// The project-relative segments of a full path inside the project.
pub fn project_relative(full_path: &Path, root: &Path) -> Vec<String> {
    relative(&normalise(root), &normalise(full_path))
}

/// The map-relative form Freeplane stores for a target picked by its project-relative segments: forward slashes,
/// whatever the platform.
pub fn to_map_relative(body_path: &Path, project_relative_target: &[&str], root: &Path) -> String {
    let folder = body_path.parent().unwrap_or(root);
    let mut target = root.to_path_buf();
    for segment in project_relative_target {
        target.push(segment);
    }
    let segments = relative(&normalise(folder), &normalise(&target));
    if segments.is_empty() {
        ".".to_owned()
    } else {
        segments.join("/")
    }
}

/// Rewrites every file link of `document` so it still resolves once the map moves from `old_body_path` to
/// `new_body_path`; how many it changed. External links are left alone, and a rename within the folder changes none.
pub fn rebase(document: &mut MindmapDocument, old_body_path: &Path, new_body_path: &Path) -> usize {
    let old_folder = old_body_path.parent().unwrap_or(Path::new(""));
    let new_folder = new_body_path.parent().unwrap_or(Path::new(""));
    if old_folder
        .to_string_lossy()
        .eq_ignore_ascii_case(&new_folder.to_string_lossy())
    {
        return 0;
    }
    let mut rewritten = 0;
    for node in document.nodes() {
        let Some(link) = document.link(node).map(str::to_owned) else {
            continue;
        };
        if is_external(&link) {
            continue;
        }
        let absolute = normalise(&old_folder.join(link.replace('/', std::path::MAIN_SEPARATOR_STR)));
        let segments = relative(&normalise(new_folder), &absolute);
        document.set_link(node, Some(&segments.join("/")));
        rewritten += 1;
    }
    rewritten
}

/// The segments that lead from `from` to `to`, `..` for every folder `from` has that `to` does not.
fn relative(from: &Path, to: &Path) -> Vec<String> {
    let from: Vec<_> = from.components().collect();
    let to: Vec<_> = to.components().collect();
    let shared = from
        .iter()
        .zip(&to)
        .take_while(|(a, b)| a.as_os_str().eq_ignore_ascii_case(b.as_os_str()))
        .count();
    let mut segments: Vec<String> = from[shared..].iter().map(|_| "..".to_owned()).collect();
    segments.extend(
        to[shared..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().into_owned()),
    );
    segments
}

/// The path with `.` and `..` resolved without touching the file system.
fn normalise(path: &Path) -> PathBuf {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let mut out = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}
