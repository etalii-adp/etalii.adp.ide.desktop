//! The operations that change an entry's text, `setTitle` and `setDescription` of `hierarchical-requirement.des` (the
//! standalone host's `TextOperations` at a7fdf03; FR-026): a replace of the one value, with the numbering every edit
//! writes (spec 029 R17). What they write is what the conformance fixtures state, byte for byte.

use adp_cel::Value;

use super::body_edit::BodyEdit;
use crate::body::RequirementBody;
use crate::gestures::DRIFTED;
use crate::mapper;

pub const SET_TITLE: &str = "setTitle";
pub const SET_DESCRIPTION: &str = "setDescription";

/// Whether `operation` is one of these.
pub fn handles(operation: &str) -> bool {
    operation == SET_TITLE || operation == SET_DESCRIPTION
}

/// The attribute an operation sets, which is also the name of its one param.
pub fn attribute_of(operation: &str) -> &'static str {
    if operation == SET_TITLE { "title" } else { "description" }
}

/// Whether `value` can be written as the attribute `operation` sets: the sentence saying why not, or empty. A title is
/// the rest of its bullet's line, so it holds no line break.
pub fn refusal(operation: &str, value: &str) -> &'static str {
    if operation == SET_TITLE && value.contains(['\r', '\n']) {
        "A title is one line; the description holds the lines below it."
    } else {
        ""
    }
}

/// The bytes of the body `body`, named `file_name`, after `operation` set the entry the surface names `target_id` to
/// `value`; or the sentence saying why it cannot be, in which case nothing changed. A target the file no longer has is
/// refused as a change made against an older file.
pub fn apply(body: &[u8], file_name: &str, operation: &str, target_id: &str, value: &str) -> Result<Vec<u8>, String> {
    let refused = refusal(operation, value);
    if !refused.is_empty() {
        return Err(refused.into());
    }
    let mut read = RequirementBody::read(body.to_vec(), file_name, None);
    if !read.document.read_only_reason.is_empty() {
        return Err(read.document.read_only_reason.clone());
    }
    let ids = mapper::node_ids(&read.document.entries);
    let Some(at) = ids.iter().position(|id| id == target_id) else {
        return Err(DRIFTED.into());
    };
    let target = read.document.entries[at].clone();
    let mut edit = BodyEdit::new(&mut read);
    let element = edit.element_of(&target);
    edit.set(&element, attribute_of(operation), Value::String(value.to_owned()));
    edit.write()
}
