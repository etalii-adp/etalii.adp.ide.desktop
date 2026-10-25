//! What a new requirement document starts as (the standalone host's `HierarchicalRequirementDocumentTemplate` at
//! a7fdf03; FR-005): the binding's template, with the file's base name as its title, written by the FBL library's
//! `produce` and so never written out a second time here. The binding creates its registration with the first setting
//! the person changes, so a new document is its Markdown file alone.

use adp_desl::designer::DesignerFormat;

use crate::definition::{self, ORIGIN};

/// The text of a new document named `file_name` in `format`; none for a format other than Markdown.
pub fn create(format: &DesignerFormat, file_name: &str) -> Option<String> {
    if !format.extension.eq_ignore_ascii_case(".md") {
        return None;
    }
    // The template's numbers are written in it; it asks for no id of its own.
    let bytes = adp_fbl::routing::produce(&definition::binding(), Some(ORIGIN), file_name, &|rule| {
        panic!("The template asks for an id of '{rule}', which a requirement document gives by its counters.")
    })?;
    String::from_utf8(bytes).ok()
}
