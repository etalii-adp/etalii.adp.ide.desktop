//! The Hierarchical Requirement Designer (etalii.adp spec 036, after the standalone host's
//! `src/designers/hierarchical-requirement` at a7fdf03): the requirements of one subject as items and requirements in
//! one Markdown file - a DED definition of the designer type `etalii/hierarchical-requirement`, read and written
//! through the vendored `hierarchical-requirement.fbl` and laid out on the `hierarchy` surface as
//! `hierarchical-requirement.des` says.

/// The designer type's origin, as the envelope on a document's line 1 and its registration name it.
pub const ORIGIN: &str = "etalii/hierarchical-requirement";
