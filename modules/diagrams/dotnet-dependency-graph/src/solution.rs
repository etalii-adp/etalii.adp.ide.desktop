//! Reads a solution file for the project files it names (etalii.adp spec 030, task T082), ported from the standalone
//! host's `SolutionReader.cs` at `8b6ccfc`: the classic `.sln` with its `Project(...)` lines, solution folders skipped,
//! and the newer `.slnx` XML, every `<Project Path>` at any depth and nothing else.
//!
//! Nothing here is fatal and nothing here is silent: an unreadable solution yields no projects and one failure; a
//! project the solution names that is not on disk is left out and named in the failures. This reader never writes.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use crate::graph::SolutionFailure;
use crate::{paths, xml};

/// The type GUID a classic `.sln` gives a solution folder, compared ignoring case.
const SOLUTION_FOLDER_TYPE: &str = "2150E333-8FDC-42A3-9474-1A3956D46DE8";

/// `Project("{type}") = "name", "relative\path.csproj", "{id}"`, one line.
static CLASSIC_PROJECT_LINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?m)^\s*Project\("\{(?P<type>[^}]*)\}"\)\s*=\s*"(?P<name>[^"]*)"\s*,\s*"(?P<path>[^"]*)"\s*,\s*"\{(?P<id>[^}]*)\}"\s*$"#,
    )
    .expect("the classic project line pattern compiles")
});

/// One project a solution names and which is on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolutionProject {
    /// The project's file name without its extension: what the graph labels it.
    pub name: String,
    /// The path relative to the solution, forward-slashed on every platform: what its element id is built from.
    pub relative_path: String,
    /// Where to read the project file from. Never written to.
    pub absolute_path: PathBuf,
}

/// What one read of a solution produced: the projects that resolved, and everything that did not.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SolutionReading {
    pub projects: Vec<SolutionProject>,
    pub failures: Vec<SolutionFailure>,
}

impl SolutionReading {
    fn failure(path: &Path, reason: impl Into<String>) -> Self {
        SolutionReading {
            projects: Vec::new(),
            failures: vec![SolutionFailure {
                path: path.to_string_lossy().into_owned(),
                reason: reason.into(),
            }],
        }
    }
}

/// Reads the solution at `path`; never panics for a bad solution.
pub fn read(path: &Path) -> SolutionReading {
    if !path.is_file() {
        return SolutionReading::failure(path, "The solution file does not exist.");
    }
    let text = match fs::read(path) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(error) => return SolutionReading::failure(path, format!("The solution file could not be read: {error}")),
    };
    let is_slnx = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("slnx"));
    if is_slnx {
        return match read_slnx(&text) {
            Some(declared) => resolve(&declared, path),
            None => SolutionReading::failure(path, "The solution file could not be parsed."),
        };
    }
    resolve(&read_classic(&text), path)
}

/// The XML serialization's `<Project Path>`s, at any depth inside folders; none when the document does not parse.
fn read_slnx(text: &str) -> Option<Vec<String>> {
    let document = xml::parse(text).ok()?;
    Some(
        xml::elements(&document, "Project")
            .filter_map(|e| e.attribute("Path"))
            .filter(|p| !p.trim().is_empty())
            .map(str::to_owned)
            .collect(),
    )
}

/// The classic serialization's `Project(...)` lines, solution folders left out.
fn read_classic(text: &str) -> Vec<String> {
    CLASSIC_PROJECT_LINE
        .captures_iter(text)
        .filter(|c| !c["type"].eq_ignore_ascii_case(SOLUTION_FOLDER_TYPE))
        .map(|c| c["path"].to_owned())
        .collect()
}

/// The declared paths as projects on disk, relative to the solution; a path that names nothing is a failure.
fn resolve(declared: &[String], solution: &Path) -> SolutionReading {
    let full = paths::normalise(solution);
    let folder = full.parent().map(Path::to_path_buf).unwrap_or_default();
    let mut reading = SolutionReading::default();
    for relative in declared {
        let absolute = paths::normalise(&folder.join(paths::with_platform_separators(relative)));
        if !absolute.is_file() {
            reading.failures.push(SolutionFailure {
                path: relative.clone(),
                reason: "The solution names this project, but the file is not there.".to_owned(),
            });
            continue;
        }
        reading.projects.push(SolutionProject {
            name: absolute
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
            relative_path: paths::relative(&folder, &absolute),
            absolute_path: absolute,
        });
    }
    reading
}
