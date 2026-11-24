//! Links between documents (FBL section 17, etalii.adp spec 089): a link written and read ([`Link`], section 17.1),
//! the workspace's index of documents and the resolution of a link against it ([`Workspace`], section 17.3), the
//! text a link is rewritten to by load-time validation (section 17.4), the backlinks of an element (section 17.5) and
//! the findings of a link that does not resolve (section 17.6).
//!
//! Nothing here knows a tool type. A tool type's module reads its own bodies and gives the index what it holds: the
//! document id, the elements that can be linked to with their names, and the links it holds ([`IndexedDocument`]).

mod ids;
mod link;
mod workspace;

pub use ids::{BASE36, nanoid};
pub use link::{ElementPart, Link};
pub use workspace::{
    Backlink, HeldLink, IndexedDocument, IndexedElement, LinkFinding, Resolution, Resolved, Workspace, codes,
};
