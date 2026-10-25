//! An entry of a requirement document as something selected and acted on (the standalone host's
//! `HierarchicalRequirementEntrySourceResolver` and `HierarchicalRequirementEntries` at a7fdf03; spec 029 FR-012;
//! etalii.adp spec 036, T057): the page names an entry by the id its box carries, under the file it is in, and this
//! finds it there, so that what is selected on the page is described, and offers its actions, as the file now is.

use std::path::Path;

use crate::body::RequirementBody;
use crate::designer;
use crate::mapper;
use crate::model::{Document, Entry};

/// An entry found in its file: the entry, the bytes it was read from and the document they make.
#[derive(Debug, Clone)]
pub struct Found {
    pub entry: Entry,
    pub bytes: Vec<u8>,
    pub document: Document,
}

/// The entry `element_id` names in the file at `body_path`; none when the file is not a requirement document, cannot
/// be read, or holds no such entry.
pub fn find(body_path: &Path, element_id: &str) -> Option<Found> {
    if element_id.is_empty() || !body_path.extension().is_some_and(|e| e.eq_ignore_ascii_case("md")) {
        return None;
    }
    let bytes = std::fs::read(body_path).ok()?;
    let name = body_path.file_name()?.to_string_lossy().into_owned();
    if !designer::claims(&name, &bytes) {
        return None;
    }
    let document = RequirementBody::read(bytes.clone(), &name, None).document;
    let ids = mapper::node_ids(&document.entries);
    let at = ids.iter().position(|id| id == element_id)?;
    Some(Found {
        entry: document.entries[at].clone(),
        bytes,
        document,
    })
}

/// How a selected entry is named where the selection is shown: its number and its title, or its number alone.
pub fn describe(number: &str, title: &str) -> String {
    if title.is_empty() {
        number.to_owned()
    } else {
        format!("{number} {title}")
    }
}
