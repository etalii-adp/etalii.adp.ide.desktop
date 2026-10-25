//! The Hierarchical Requirement Designer as a module offers it (the standalone host's `Designer` at a7fdf03): its
//! definition, and the claim by which a requirement document is recognised by the envelope on its first line, with or
//! without a registration (spec 029 R3, R4), answered by the binding's own marker through the FBL library's routing.

use adp_desl::designer::DesignerClaim;
use adp_desl::{DesignerDefinition, DesignerFormat};

use crate::definition::{self, ORIGIN};
use crate::vocabulary;

/// The line a requirement document carries on its first line, for a refusal to name.
pub const ENVELOPE: &str = "<!-- ded: 0.2 designer: etalii/hierarchical-requirement -->";

/// The designer's definition: one format, Markdown, and the envelope claim of its binding.
pub fn definition() -> DesignerDefinition {
    let binding = definition::binding();
    DesignerDefinition {
        origin: ORIGIN.into(),
        title: vocabulary::NAME.into(),
        description:
            "The requirements of a subject on one page, by its structure, kept in a Markdown file an agent can write too."
                .into(),
        icon: "mdi-file-tree-outline".into(),
        formats: vec![DesignerFormat::new("Markdown", ".md")],
        claim: Some(DesignerClaim {
            extensions: binding.claims.extensions.clone(),
            matches: claims,
            envelope: ENVELOPE.into(),
            creates_registration: binding.registration.create_on_first_placement,
        }),
    }
}

/// Whether a file named `name` whose first bytes are `head` is a requirement document (FBL section 12.3).
pub fn claims(name: &str, head: &[u8]) -> bool {
    let binding = definition::binding();
    !adp_fbl::routing::candidates(name, head, [binding.as_ref()]).is_empty()
}

/// The body of a new requirement document named `file_name`: the binding's template ([`crate::template`]).
pub fn new_document(format: &DesignerFormat, file_name: &str) -> Option<String> {
    crate::template::create(format, file_name)
}
