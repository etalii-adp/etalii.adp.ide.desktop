//! What an operation on a list the registration keeps writes - the tags, the categories, the states or the filters
//! (the standalone host's `ListEdit` at a7fdf03): the body, and the registration with what it held before; none where
//! it writes nothing (spec 029 R17).

/// The files after an operation on a list, each none when the operation does not write it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ListEdit {
    /// The body's bytes after the operation, or none when they do not change.
    pub body: Option<Vec<u8>>,
    /// The registration as the step starts from it: as it is, or as it is created first, outside the step.
    pub registration_before: Option<Vec<u8>>,
    /// The registration's bytes after the operation, or none when it is not written.
    pub registration_after: Option<Vec<u8>>,
}

/// The two files an operation on a list reads, with their names.
#[derive(Debug, Clone, Copy)]
pub struct ListFiles<'a> {
    pub body: &'a [u8],
    pub body_name: &'a str,
    /// The registration's bytes, or none when the body has none yet.
    pub registration: Option<&'a [u8]>,
    pub registration_name: &'a str,
}

/// A typed name as the operations write it: `p.name.trim().replace(' ', '-')`.
pub fn normalised(name: &str) -> String {
    name.trim().replace(' ', "-")
}

/// Whether two names are the same in CEL's `lowerAscii`, which the specification compares names with.
pub fn same(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}
