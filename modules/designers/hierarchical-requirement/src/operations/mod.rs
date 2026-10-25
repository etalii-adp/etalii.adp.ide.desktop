//! The operations of `hierarchical-requirement.des` that change the body (the standalone host's `Operations/` at
//! a7fdf03): [`text`] changes an entry's title or description, [`structure`] shapes the hierarchy, [`assessments`]
//! adds, changes, resolves and removes the questions, remarks, warnings and ideas about entries, and all write through
//! one [`body_edit::BodyEdit`], which adds the numbering every edit writes.

pub mod assessments;
pub mod body_edit;
pub mod structure;
pub mod text;
