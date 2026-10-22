//! The sentences the shapes reading refuses with (etalii.adp spec 030, task T083), in one place because the writer and
//! the gate must say the same thing (`Shacl/ShaclRefusals.cs` at `8b6ccfc`).

/// Mutating or removing content rooted in a blank node: the blank-node identity boundary.
pub const BLANK_ROOTED: &str = "This constraint is written as a blank node, which has no identity that survives a reparse, so an edit keyed to it could not be undone reliably. Open the file as text to change it, or give the shape an IRI of its own.";

/// The shape a gesture names is not in the file, or is not IRI-named.
pub const NO_SUCH_SHAPE: &str = "No IRI-named shape of that name is in this file, so there is nothing to edit.";

/// A property row asked for without a path.
pub const PATH_REQUIRED: &str =
    "A property shape needs exactly one sh:path, so a path is required before the row can be written.";

/// The target a removal names is not declared.
pub const NO_SUCH_TARGET: &str = "That target is not declared in this file, so there is nothing to remove.";

/// An implicit class target is stated by no triple.
pub const IMPLICIT_TARGET: &str = "This shape targets its own instances because the file states it to be a class, not through a target triple, so there is no declaration to remove. Remove the class typing instead, as text.";
