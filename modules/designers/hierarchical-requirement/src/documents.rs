//! A requirement document's two files on disk (the standalone host's `HierarchicalRequirementSession.ReadFiles`,
//! `IsThisDesigners` and `WriteSettingAsync` at a7fdf03): the Markdown file and the registration beside it, read
//! together, and a setting written to the registration, which is created with the first one (FBL 0.7 section 8.4).
//! **Opening writes nothing**: the bytes are kept exactly as they were.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use adp_fbl::registration::create_registration;
use adp_fbl::{ModelChange, OpenRegistration, PlanResult, Setting};

use crate::definition::{self, ORIGIN};

/// How a file is written: the path and the bytes it is to hold, or why it could not be.
pub type Writer = Box<dyn FnMut(&Path, &[u8]) -> Result<(), String>>;

/// The writer that writes to disk.
pub fn disk_writer() -> Writer {
    Box::new(|path: &Path, bytes: &[u8]| std::fs::write(path, bytes).map_err(|e| e.to_string()))
}

/// The bytes of the body and of the registration, as they were read; the body is none when it could not be, with
/// the reason.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Files {
    pub body: Option<Vec<u8>>,
    pub registration: Option<Vec<u8>>,
    pub refusal: String,
}

/// The registration beside a body when it has none named: the body's name with `.adp`.
pub fn registration_beside(body: &Path) -> PathBuf {
    body.with_extension("adp")
}

/// The requirement document a path opens: a body whose first line is the envelope, with no registration named, or a
/// registration whose first line is this designer type's, with the body its `body:` line names or, without one, the
/// `.md` of its base name beside it (FBL 0.7 section 8.1). None for any other path.
pub fn document_at(path: &Path) -> Option<(PathBuf, Option<PathBuf>)> {
    if !path.is_file() {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    let name = path.file_name()?.to_string_lossy().into_owned();
    if !path.extension().is_some_and(|e| e.eq_ignore_ascii_case("adp")) {
        return crate::designer::claims(&name, &bytes).then(|| (path.to_path_buf(), None));
    }
    if !is_this_designers(&bytes) {
        return None;
    }
    let text = String::from_utf8_lossy(&bytes);
    let named = text
        .split(['\r', '\n'])
        .skip(1)
        .find_map(|line| line.trim().strip_prefix("body:"))
        .map(str::trim)
        .filter(|n| !n.is_empty());
    let folder = path.parent().unwrap_or(Path::new(""));
    let body = match named {
        Some(named) => folder.join(named),
        None => path.with_extension("md"),
    };
    body.is_file().then(|| (body, Some(path.to_path_buf())))
}

/// Reads both files. A registration whose first line names another designer type is not this document's, and is
/// not read.
pub fn read(body: &Path, registration: &Path) -> Files {
    match std::fs::read(body) {
        Ok(bytes) => Files {
            body: Some(bytes),
            registration: std::fs::read(registration).ok().filter(|r| is_this_designers(r)),
            refusal: String::new(),
        },
        Err(e) => Files {
            body: None,
            registration: None,
            refusal: e.to_string(),
        },
    }
}

/// Whether a registration's first line names this designer type.
fn is_this_designers(registration: &[u8]) -> bool {
    let text = String::from_utf8_lossy(registration);
    let first = text.split(['\r', '\n']).next().unwrap_or_default();
    first.trim_start_matches('\u{feff}').trim() == ORIGIN
}

/// Writes one setting to the registration, creating it first when there is none, with `known_ids` the numbers the
/// body holds. Refused with the reason when it cannot be kept.
pub fn write_setting(
    body_path: &Path,
    registration_path: &Path,
    setting: Setting,
    known_ids: HashSet<String>,
    write: &mut Writer,
) -> Result<(), String> {
    let name = |path: &Path| {
        path.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    };
    let body = std::fs::read(body_path).map_err(|e| format!("'{}' could not be read: {e}", name(body_path)))?;
    let existing = std::fs::read(registration_path).ok();
    let binding = definition::binding();
    let created = create_registration(
        &binding,
        ORIGIN,
        &name(registration_path),
        &name(body_path),
        &body,
        existing.as_deref(),
    )?;
    let mut registration = OpenRegistration::open_with(created, &binding);
    registration.known_ids = Some(known_ids);
    match registration.change(&ModelChange::Setting(setting)) {
        PlanResult::Planned(_) => {}
        PlanResult::Refused(reason) | PlanResult::Invalid(reason) => return Err(reason),
    }
    write(registration_path, registration.bytes())
        .map_err(|e| format!("'{}' could not be written: {e}", name(registration_path)))
}
