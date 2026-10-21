//! A specification bundled with a module, loaded, with its provenance (the standalone host's `BundledDefinition`):
//! where it came from (`provenance.json`, written when the definition is bundled) - the repository, the file in it,
//! the full revision it was copied at, and the sha256 of the bundled bytes - checked against the bytes before they
//! are loaded, so a module never runs from a definition other than the one it records.

use sha2::{Digest, Sha256};

use crate::expressions::plugins::DislPluginFunctions;
use crate::json;
use crate::loading::diagnostic::DislDiagnostic;
use crate::loading::loader::DislLoader;
use crate::loading::specification::DislSpecification;

/// Where a bundled definition came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionProvenance {
    pub repository: String,
    pub path: String,
    pub revision: String,
    pub sha256: String,
}

/// A bundled specification, loaded, with its provenance.
#[derive(Debug, Clone)]
pub struct BundledDefinition {
    pub specification: DislSpecification,
    pub provenance: DefinitionProvenance,
    pub diagnostics: Vec<DislDiagnostic>,
}

impl BundledDefinition {
    /// Loads the definition `bytes`, bundled as `logical_name` (`gartner-hype-cycle-graph.dis`), with the provenance
    /// bundled beside it. An error when the provenance is incomplete, the bytes are not the ones it names, or the
    /// specification does not load: a module cannot run from a definition it does not have.
    pub fn load(
        logical_name: &str,
        bytes: &[u8],
        provenance: &[u8],
        plugins: &DislPluginFunctions,
    ) -> Result<BundledDefinition, String> {
        let name = logical_name.strip_suffix(".dis").unwrap_or(logical_name);
        let provenance = provenance_of(provenance, name)?;
        let actual: String = Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect();
        if actual != provenance.sha256 {
            return Err(format!(
                "{logical_name} hashes to {actual}, not to the {} its provenance records; re-bundle it with src/diagrams/tools/bundle-disl.sh.",
                provenance.sha256
            ));
        }
        let loaded = DislLoader::load_bytes(bytes, plugins);
        match loaded.specification {
            Some(specification) => Ok(BundledDefinition {
                specification,
                provenance,
                diagnostics: loaded.diagnostics,
            }),
            None => Err(format!(
                "{logical_name} does not load: {}",
                loaded.diagnostics.iter().filter(|d| d.severity == crate::DislSeverity::Error).map(ToString::to_string).collect::<Vec<_>>().join("; ")
            )),
        }
    }
}

fn provenance_of(bytes: &[u8], name: &str) -> Result<DefinitionProvenance, String> {
    let (root, _) = json::parse(bytes).map_err(|e| format!("The provenance of {name} is not JSON: {e}"))?;
    let field = |field: &str| {
        json::string_of(&root, field)
            .map(str::to_owned)
            .ok_or_else(|| format!("The provenance of {name} has no {field}."))
    };
    Ok(DefinitionProvenance {
        repository: field("repository")?,
        path: field("path")?,
        revision: field("revision")?,
        sha256: field("sha256")?,
    })
}
