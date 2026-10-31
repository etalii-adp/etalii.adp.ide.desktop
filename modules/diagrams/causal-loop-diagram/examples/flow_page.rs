//! Writes the flow page of the `.cld` named first on the command line to the path named second, as the export does, so
//! that `tools/flow-page-check` can run a real page (etalii.adp spec 074, R6). A registration beside the file is used for
//! its positions.

use std::fs;
use std::path::Path;

use adp_core::Catalogue;
use adp_diagram::{DiagramTools, DiagramWorkspace};
use adp_diagram_causal_loop::flow_page::generate;
use adp_diagram_causal_loop::metrics::Metrics;
use adp_disl::DislPluginFunctions;

fn main() {
    let mut arguments = std::env::args().skip(1);
    let (body, out) = (arguments.next().expect("a .cld"), arguments.next().expect("a page"));
    let body = Path::new(&body).canonicalize().expect("the .cld exists");
    let folder = body.parent().expect("a folder");
    let stem = body.file_stem().expect("a name").to_string_lossy().into_owned();
    let registration = folder.join(format!("{stem}.adp"));
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
    let mut tools = DiagramTools::new(Catalogue::load(&vendor), |_| DislPluginFunctions::none())
        .with_module(adp_diagram_causal_loop::ORIGIN, adp_diagram_causal_loop::module());
    let mut workspace = DiagramWorkspace::new(folder);
    let scratch = tempfile::tempdir().expect("a folder of its own");
    let opened = if registration.exists() {
        Path::new(registration.file_name().expect("a name")).to_owned()
    } else {
        // A body on its own is opened through a registration in a folder of its own.
        let copy = scratch.path().join(body.file_name().expect("a name"));
        fs::copy(&body, &copy).expect("a copy");
        fs::write(
            scratch.path().join(format!("{stem}.adp")),
            "systems/causal-loop-diagram\r\n",
        )
        .expect("a registration");
        workspace = DiagramWorkspace::new(scratch.path());
        Path::new(&format!("{stem}.adp")).to_owned()
    };
    workspace.open(&mut tools, &opened).expect("the diagram opens");
    let path = if registration.exists() {
        body.clone()
    } else {
        scratch.path().join(body.file_name().expect("a name"))
    };
    let document = workspace.documents().get(&path).expect("open");
    fs::write(out, generate(document, &Metrics::default(), &stem)).expect("the page is written");
}
