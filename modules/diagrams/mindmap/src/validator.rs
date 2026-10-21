//! The mind map's findings (etalii.adp spec 030, FR-018), ported from the standalone host's `MindmapValidator.cs` and
//! `MindmapDefinition.Findings` at `8b6ccfc`.
//!
//! The findings are the specification's: its built-in `std.unparseable` (`mindmap.not-a-map`) and its rules over the
//! map's DISL model, read through the binding exactly as the file has it, no id assigned. A finding names the map by
//! the file the problems panel attributes it to, the registration when there is one, and a node by the `ID` the file
//! writes, located only when it has one. An ordinary node without text is not judged: Freeplane keeps such nodes.

use std::rc::Rc;

use adp_diagram::{DiagramProblem, DiagramProblemLocation, DiagramTool, DislDiagramDefinition};
use adp_disl::derivation::constraints;
use adp_disl::model::builder;
use adp_disl::{DislConstraintOptions, DislEnv, DislReaderFinding};
use adp_fbl::model::codes;
use adp_fbl::plugins::PluginBody;

use crate::plugin::{MindmapFreeplanePlugin, STORED_ID_ATTRIBUTE};

/// The findings of the map `text`, attributed to the file `file` (a registration's name, else the body's).
pub fn validate(tool: &DiagramTool, text: &str, file: &str) -> Vec<DiagramProblem> {
    let specification = tool.definition.specification();
    let body = match PluginBody::open(
        text.as_bytes().to_vec(),
        tool.binding.clone(),
        Some(Rc::new(MindmapFreeplanePlugin)),
        file,
    ) {
        Ok(body) => body,
        Err(reason) => {
            return vec![DiagramProblem {
                severity: adp_diagram::DiagramProblemSeverity::Error,
                message: reason,
                rule_id: codes::PLUGIN_MISSING.into(),
                location: None,
            }];
        }
    };
    let fbl = body.model();
    let model = builder::from(&fbl, specification);
    model.diagram.set_file(Some(file));
    let reader_findings = fbl
        .findings
        .iter()
        .filter(|f| f.code == codes::UNPARSEABLE)
        .map(|f| DislReaderFinding::not_parsed(&f.message, f.location.as_ref().map(|l| l.line as i32), None))
        .collect();
    let options = DislConstraintOptions {
        env: Some(DislEnv::default()),
        reader_findings,
        written_id: Some(Rc::new(|element| {
            element
                .host_attributes()
                .get(STORED_ID_ATTRIBUTE)
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default()
        })),
    };
    constraints::evaluate(specification, &model.diagram, &options)
        .into_iter()
        .map(|finding| DiagramProblem {
            severity: DislDiagramDefinition::severity_of(&finding),
            location: finding
                .element_ids
                .first()
                .filter(|id| !id.is_empty())
                .map(|id| DiagramProblemLocation::Element(id.clone())),
            message: finding.message,
            rule_id: finding.code,
        })
        .collect()
}
