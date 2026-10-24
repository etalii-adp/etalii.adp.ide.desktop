//! The recording painter of the shell's canvas (etalii.adp spec 035, T051, ATD §2.4): [`drawing_of`] records an open
//! diagram, as `atd record` does after each step: the scene its module draws by hand (`*_scene.rs`), or the one its
//! notation draws (`adp_drawing::notation_scene`), recorded by `adp_drawing::record`; no window is opened.

use std::path::Path;

use adp_diagram::{DiagramDocuments, DiagramElement, DiagramSession};
use adp_scene::Scene;
use adp_theme::Kind;
use serde_json::Value as Json;

pub use adp_drawing::record;

use crate::canvas::DiagramHost;

/// The scene a diagram's canvas draws, as a recording takes it: a module's own scene where its module draws it by hand,
/// with nothing selected, in the `kind` theme; otherwise the scene its notation draws, laid out with the estimated text
/// metrics that do not depend on the fonts of a machine, as the `insta` scene snapshots of `adp-disl` are.
pub fn scene_of(
    origin: &str,
    session: &DiagramSession,
    documents: &DiagramDocuments,
    elements: &[DiagramElement],
    kind: Kind,
) -> Result<Scene, String> {
    if session.module_view().is_some() {
        return Ok(crate::canvas::hand_drawn(origin, elements, &[], None, kind).0.scene);
    }
    adp_drawing::notation_scene(session, documents, elements)
}

/// The drawing of an open diagram: the scene [`scene_of`] takes, recorded.
pub fn drawing_of(
    origin: &str,
    session: &DiagramSession,
    documents: &DiagramDocuments,
    kind: Kind,
) -> Result<Json, String> {
    let elements = session.render(documents)?;
    let scene = scene_of(origin, session, documents, &elements, kind)?;
    Ok(record(&scene, &elements, "canvas"))
}

/// Opens the diagram at `path` (its body or its registration) in the project `root` headlessly and records its
/// drawing ([`drawing_of`]); the vendored tool types are read from `vendor`.
pub fn drawing(vendor: &Path, path: &Path, root: &Path, kind: Kind) -> Result<Json, String> {
    let mut host = DiagramHost::with_vendor(vendor);
    if !host.claims(path) {
        return Err(format!("no usable tool type claims {}", path.display()));
    }
    let tab = host.open(path, root)?;
    drawing_of(&tab.opened.origin, &tab.session, host.workspace(root).documents(), kind)
}
