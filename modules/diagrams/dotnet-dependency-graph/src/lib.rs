//! The .NET dependency graph's residue (etalii.adp spec 030, FR-018, FR-031 and FR-032, task T082): the tool type
//! `dotnet/dependency-graph`, read-only, after the standalone host's
//! `src/diagrams/dotnet-dependency-graph/backend/EtAlii.Adp.Diagram.DotNetDependencyGraph` at `8b6ccfc`.
//!
//! The diagram is a projection of files the build owns: a `.sln` or `.slnx` solution named by the registration's
//! `body:` header (else the registration's sibling `.sln`), the project files it names, the `Directory.Packages.props`
//! files above them and the local NuGet cache. They are parsed here, in Rust: nothing of .NET runs. The one edit is a
//! position, written into the registration's `layout:` block; no solution or project file is ever written. The generic
//! path reads documents through a binding, which this tool type has none of at the reference commit, so
//! [`DotNetModule`] [opens its documents itself](adp_diagram::DiagramModule::opens_documents) and each open view is a
//! [`view::DotNetView`].
//!
//! - [`solution`], [`project`] and [`descriptions`]: the three readers, with [`paths`] and [`xml`] beneath them.
//! - [`graph`] and [`layout`]: the derived graph and where its boxes go.
//! - [`definition`] and [`mapper`]: the rows, and what the shell draws.
//! - [`registration_layout`] and [`commands`]: the stored positions and the command that stores one.
//! - [`store`] and [`view`]: the open solutions, and one open view.

pub mod commands;
pub mod definition;
pub mod descriptions;
pub mod graph;
pub mod layout;
pub mod mapper;
pub mod paths;
pub mod project;
pub mod registration_layout;
pub mod solution;
pub mod store;
pub mod view;
pub mod xml;

use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_diagram::workspace::{CANNOT_OPEN, OpenedDiagram};
use adp_diagram::{DiagramDocuments, DiagramModule, DiagramTools, ModuleView};

use crate::view::DotNetView;

/// The tool type this module serves.
pub const ORIGIN: &str = "dotnet/dependency-graph";

/// The classic solution serialization: the extension a registration without a `body:` header derives its body by.
pub const DOCUMENT_EXTENSION: &str = ".sln";

/// The XML serialization of the same solution.
pub const ALTERNATE_DOCUMENT_EXTENSION: &str = ".slnx";

/// The body of a new diagram: an empty solution in the newer serialization (`AddEmptyDocument`).
pub const EMPTY_DOCUMENT: &str = "<Solution>\r\n</Solution>\r\n";

/// How many lines after the first a registration's headers are looked for in (`DiagramFilePair.HeaderScanLimit`).
const HEADER_SCAN_LIMIT: usize = 16;

/// The refusal of a `body:` header naming a file outside the project.
pub const OUTSIDE: &str =
    "Its body: header names a document outside the project, which is refused rather than followed.";

/// The .NET dependency graph module.
#[derive(Debug, Clone, Copy, Default)]
pub struct DotNetModule;

/// `tools` with the .NET dependency graph module added to its tool type.
pub fn register(tools: DiagramTools) -> DiagramTools {
    tools.with_module(ORIGIN, Rc::new(DotNetModule))
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The `body:` header of a registration's text, read from the lines after its first, blank lines skipped, up to the
/// first line that is neither a header nor blank (`DiagramFilePair.ReadBodyAndViewHeaders`).
fn body_header(text: &str) -> Option<String> {
    let mut body = None;
    for line in text
        .trim_start_matches('\u{feff}')
        .lines()
        .skip(1)
        .take(HEADER_SCAN_LIMIT)
    {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let starts = |header: &str| {
            trimmed
                .get(..header.len())
                .is_some_and(|h| h.eq_ignore_ascii_case(header))
        };
        if starts("body:") {
            let value = trimmed["body:".len()..].trim();
            body = (!value.is_empty()).then(|| value.to_owned());
        } else if !starts("view:") {
            break;
        }
    }
    body
}

/// `relative` resolved against `folder`, or none when it is rooted or leaves `root` (`DiagramFilePair.ResolveWithin`).
fn resolve_within(root: &Path, folder: &Path, relative: &str) -> Option<PathBuf> {
    if Path::new(relative).has_root() || Path::new(relative).is_absolute() {
        return None;
    }
    let full = paths::normalise(&folder.join(relative));
    full.starts_with(paths::normalise(root)).then_some(full)
}

/// The solution the registration at `registration` names: its `body:` header, else its sibling `.sln`.
fn route(root: &Path, registration: &Path) -> Result<PathBuf, String> {
    let text = fs::read(registration)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .map_err(|_| "The registration file could not be read.".to_owned())?;
    let folder = registration.parent().unwrap_or(root);
    match body_header(&text) {
        Some(body) => resolve_within(root, folder, &body).ok_or_else(|| OUTSIDE.to_owned()),
        None => {
            let stem = registration
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .filter(|s| !s.is_empty() && s != ".adp")
                .ok_or_else(|| CANNOT_OPEN.to_owned())?;
            Ok(folder.join(format!("{stem}{DOCUMENT_EXTENSION}")))
        }
    }
}

impl DiagramModule for DotNetModule {
    fn opens_documents(&self) -> bool {
        true
    }

    /// Opens the registration at `path` against the solution it names (`DotNetDependencyGraphSessionFactory.Open`); a
    /// solution opened on its own has no registration to store a position in.
    fn open(
        &self,
        root: &Path,
        path: &Path,
        origin: &str,
        documents: &mut DiagramDocuments,
    ) -> Result<(OpenedDiagram, Rc<dyn ModuleView>), String> {
        let is_registration = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("adp"));
        let (solution, registration) = if is_registration {
            (route(root, path)?, Some(path.to_path_buf()))
        } else {
            (path.to_path_buf(), None)
        };
        if !solution.is_file() {
            return Err(format!(
                "The body of '{}' is missing.",
                file_name(registration.as_deref().unwrap_or(&solution))
            ));
        }
        store::open(documents, &solution, registration.as_deref());
        let view: Rc<dyn ModuleView> = Rc::new(DotNetView::new(&solution, registration.as_deref()));
        Ok((
            OpenedDiagram {
                origin: origin.to_owned(),
                body_path: solution,
                registration_path: registration,
            },
            view,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_body_header_is_read_after_the_first_line_and_blank_lines() {
        assert_eq!(
            body_header("dotnet/dependency-graph\r\n\r\nBODY: S.slnx \r\n").as_deref(),
            Some("S.slnx")
        );
        assert_eq!(
            body_header("dotnet/dependency-graph\r\nlayout:\r\nbody: S.slnx\r\n"),
            None
        );
        assert_eq!(body_header("dotnet/dependency-graph\r\n"), None);
    }
}
