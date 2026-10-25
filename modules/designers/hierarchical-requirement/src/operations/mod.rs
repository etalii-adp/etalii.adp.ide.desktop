//! The operations of `hierarchical-requirement.des` (the standalone host's `Operations/` at a7fdf03): [`text`]
//! changes an entry's title or description, [`structure`] shapes the hierarchy, [`assessments`] adds, changes,
//! resolves and removes the questions, remarks, warnings and ideas about entries; these write the body through one
//! [`body_edit::BodyEdit`], which adds the numbering every edit writes. [`tags`] and [`classification`] change the
//! tags, categories and states an entry carries and the lists the registration keeps of them, and [`filters`] the
//! stored filters: each a [`lists::ListEdit`] of the body, the registration or both, the registration through
//! [`registration`].

pub mod assessments;
pub mod body_edit;
pub mod classification;
pub mod filters;
pub mod lists;
pub mod registration;
pub mod structure;
pub mod tags;
pub mod text;
