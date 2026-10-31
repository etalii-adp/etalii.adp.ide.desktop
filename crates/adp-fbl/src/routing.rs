//! Routing a file to its bindings and making new bodies (FBL sections 9.4, 10, 12 and 13; the standalone host's
//! `Routing`): the globs of file rules and claims, folder subjects and the files their rules select, markers,
//! the router, which returns every candidate and never chooses between several on the caller's behalf, and the
//! template writer, which replaces the four placeholders of a template and nothing else.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::{Regex, RegexBuilder};

use crate::documents::loader::scalar_text;
use crate::documents::types::{FblBinding, FileRule, Marker};
use crate::families::tree::ValueKind;
use crate::families::yaml::parser;
use crate::text::BodyText;

/// Whether the file system ignores case by the platform's convention: Windows and macOS do.
pub fn platform_ignores_case() -> bool {
    cfg!(any(target_os = "windows", target_os = "macos"))
}

/// The globs of FBL sections 10.1 and 12.1: relative, `/`-separated; `*` within one path segment, `**` any number
/// of segments, `?` one character, `[…]` a character class.
pub fn glob_matches(glob: &str, path: &str, ignore_case: bool) -> bool {
    RegexBuilder::new(&glob_to_regex(glob))
        .case_insensitive(ignore_case)
        .build()
        .is_ok_and(|r| r.is_match(&path.replace('\\', "/")))
}

fn glob_to_regex(glob: &str) -> String {
    let chars: Vec<char> = glob.chars().collect();
    let mut out = String::from("^");
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '*' if chars.get(i + 1) == Some(&'*') => {
                let slash = chars.get(i + 2) == Some(&'/');
                out.push_str(if slash { "(?:.*/)?" } else { ".*" });
                i += if slash { 2 } else { 1 };
            }
            '*' => out.push_str("[^/]*"),
            '?' => out.push_str("[^/]"),
            '[' => match chars.iter().skip(i + 2).position(|ch| *ch == ']').map(|p| p + i + 2) {
                None => out.push_str("\\["),
                Some(close) => {
                    let mut set: String = chars[i + 1..close].iter().collect();
                    if let Some(rest) = set.strip_prefix('!') {
                        set = format!("^{rest}");
                    }
                    out.push('[');
                    out.push_str(&set.replace('\\', "\\\\"));
                    out.push(']');
                    i = close;
                }
            },
            other => out.push_str(&regex::escape(&other.to_string())),
        }
        i += 1;
    }
    out.push('$');
    out
}

/// The number of lines a pattern marker looks at when it names none (FBL section 12.2).
const DEFAULT_MARKER_LINES: usize = 20;

/// Evaluates a marker (FBL section 12.2) on a body's bytes without reading it through a binding: a root key of a
/// yaml or json body, a prefix of the first line after a byte-order mark, or an expression one of the first lines
/// matches.
pub fn marker_matches(marker: &Marker, bytes: &[u8]) -> bool {
    let text = BodyText::new(bytes.to_vec());
    if !text.is_valid_utf8() {
        return false;
    }
    if let Some(key) = &marker.root_key {
        return root_key(&text, key, marker.root_value.as_ref().map(scalar_text).as_deref());
    }
    let first = text.lines()[0];
    if let Some(prefix) = &marker.first_line {
        return text
            .text(first.start.max(text.bom_length()), first.content_end)
            .starts_with(prefix.as_str());
    }
    if let Some(pattern) = &marker.pattern {
        let regex = adp_cel::BoundedRegex::new(pattern, false);
        let lines = if marker.lines > 0 {
            marker.lines as usize
        } else {
            DEFAULT_MARKER_LINES
        };
        return text
            .lines()
            .iter()
            .take(lines)
            .any(|line| regex.is_match(text.text(line.start.max(text.bom_length()), line.content_end)));
    }
    false
}

/// A root key of a yaml or json body (json is read as the yaml it also is), with its scalar value when one is
/// asked. The standalone host reads it with YamlDotNet; this host with its own yaml reader.
fn root_key(text: &BodyText, key: &str, value: Option<&str>) -> bool {
    let Ok(parsed) = parser::parse(text) else {
        return false;
    };
    let root = &parsed.values[parsed.root];
    if root.style == crate::families::tree::ValueStyle::FlowMapping {
        // A json body, or a yaml one written as a flow mapping: its members as written.
        let Some(adp_cel::Value::Map(members)) = &root.flow_text else {
            return false;
        };
        return match members.get(key) {
            None => false,
            Some(found) => value.is_none_or(|v| found.as_str() == Some(v)),
        };
    }
    if root.kind != ValueKind::Mapping {
        return false;
    }
    for &member in &root.entries {
        let entry = &parsed.entries[member];
        if entry.name.as_deref() != Some(key) {
            continue;
        }
        let found = &parsed.values[entry.tree().value];
        return value.is_none_or(|v| found.kind == ValueKind::Scalar && found.text == v);
    }
    false
}

/// The part of a body `suggest` looks at (FBL section 12.1).
const SUGGEST_BYTES: usize = 64 * 1024;

/// The bindings a bare file routes to (FBL section 12.3): those whose `names` match the file name or whose
/// `extensions` include its extension, ignoring case; never a `registrationOnly` binding; and a binding with a
/// marker only when the marker matches.
pub fn candidates<'a>(
    file_name: &str,
    bytes: &[u8],
    bindings: impl IntoIterator<Item = &'a FblBinding>,
) -> Vec<&'a FblBinding> {
    let name = Path::new(file_name)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let extension = Path::new(&name)
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    let mut found = Vec::new();
    for binding in bindings {
        let claims = &binding.claims;
        if claims.registration_only {
            continue;
        }
        let named = claims
            .names
            .iter()
            .any(|glob| glob_matches(glob, &name, platform_ignores_case()));
        let extended = !extension.is_empty() && claims.extensions.iter().any(|e| e.eq_ignore_ascii_case(&extension));
        if !named && !extended {
            continue;
        }
        if let Some(marker) = &claims.marker
            && !marker_matches(marker, bytes)
        {
            continue;
        }
        if claims.shared && claims.marker.is_none() {
            continue;
        }
        found.push(binding);
    }
    found
}

fn contains(bytes: &[u8], substrings: &[String]) -> bool {
    if substrings.is_empty() {
        return false;
    }
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(SUGGEST_BYTES)]);
    substrings.iter().any(|s| head.contains(s.as_str()))
}

/// The readings a binding offers for a body (FBL section 9.4): each origin in `claims.origins` order, those whose
/// reading's `suggest` matches the body first.
pub fn readings(binding: &FblBinding, bytes: &[u8]) -> Vec<String> {
    let origins = &binding.claims.origins;
    let suggested: Vec<String> = origins
        .iter()
        .filter(|o| suggests_reading(binding, o, bytes))
        .cloned()
        .collect();
    let mut all = suggested.clone();
    all.extend(origins.iter().filter(|o| !suggested.contains(o)).cloned());
    all
}

/// The reading a bare file opens as (FBL section 12.3): the one marked `bare`, else the binding's only origin.
pub fn bare(binding: &FblBinding) -> Option<String> {
    if let Some((origin, _)) = binding.claims.readings.iter().find(|(_, r)| r.bare) {
        return Some(origin.clone());
    }
    (binding.claims.origins.len() == 1).then(|| binding.claims.origins[0].clone())
}

/// Whether a reading's `suggest` matches the body (FBL section 9.4).
pub fn suggests_reading(binding: &FblBinding, origin: &str, bytes: &[u8]) -> bool {
    binding
        .claims
        .readings
        .iter()
        .find(|(o, _)| o == origin)
        .is_some_and(|(_, r)| contains(bytes, &r.suggest))
}

/// A file of a folder subject (FBL section 10.2): its `/`-separated path relative to the folder and the file rule
/// that selected it.
#[derive(Debug, Clone)]
pub struct FolderFile {
    pub relative_path: String,
    pub full_path: PathBuf,
    pub rule: FileRule,
}

/// Every entry under the folder, relative and `/`-separated, in ordinal order, never through a symbolic link.
fn folder_entries(folder: &Path, include_directories: bool) -> Vec<(String, PathBuf)> {
    fn walk(directory: &Path, prefix: &str, include_directories: bool, found: &mut Vec<(String, PathBuf)>) {
        let Ok(entries) = std::fs::read_dir(directory) else {
            return;
        };
        for entry in entries.flatten() {
            let Ok(metadata) = std::fs::symlink_metadata(entry.path()) else {
                continue;
            };
            if metadata.file_type().is_symlink() {
                continue;
            }
            let relative = format!("{prefix}{}", entry.file_name().to_string_lossy());
            if metadata.is_dir() {
                if include_directories {
                    found.push((relative.clone(), entry.path()));
                }
                walk(&entry.path(), &format!("{relative}/"), include_directories, found);
            } else {
                found.push((relative, entry.path()));
            }
        }
    }
    let mut found = Vec::new();
    walk(folder, "", include_directories, &mut found);
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

/// Whether `folder` qualifies as the binding's folder subject (FBL section 10.1): every `all` glob matches an
/// entry, at least one `any` glob does when there are any, and no `none` glob does.
pub fn recognise(binding: &FblBinding, folder: &Path, ignore_case: Option<bool>) -> bool {
    if !binding.body.is_folder || !folder.is_dir() {
        return false;
    }
    let caseless = ignore_case.unwrap_or_else(platform_ignores_case);
    let entries: Vec<String> = folder_entries(folder, true).into_iter().map(|(r, _)| r).collect();
    let any = |glob: &String| entries.iter().any(|e| glob_matches(glob, e, caseless));
    binding.body.recognise_all.iter().all(any)
        && (binding.body.recognise_any.is_empty() || binding.body.recognise_any.iter().any(any))
        && !binding.body.recognise_none.iter().any(any)
}

/// The files the binding's file rules select (FBL section 10.2), first matching rule each, `ignore` excluded.
pub fn folder_files(binding: &FblBinding, folder: &Path, ignore_case: Option<bool>) -> Vec<FolderFile> {
    let caseless = ignore_case.unwrap_or_else(platform_ignores_case);
    folder_entries(folder, false)
        .into_iter()
        .filter(|(relative, _)| !binding.body.ignore.iter().any(|g| glob_matches(g, relative, caseless)))
        .filter_map(|(relative, full)| {
            let rule = binding
                .body
                .files
                .iter()
                .find(|r| glob_matches(&r.glob, &relative, caseless))?;
            Some(FolderFile {
                relative_path: relative,
                full_path: full,
                rule: rule.clone(),
            })
        })
        .collect()
}

static PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\{(name|base|key|key:lower|newid:[A-Za-z_][A-Za-z0-9_-]*)\}")
        .expect("the template placeholder compiles")
});

/// The text of a new body named `file_name` from the binding's template (FBL section 13):
/// `template.byOrigin[origin]` before `template.text`; none when the binding has no template, as a plugin's
/// `template` operation is not run by this host. `new_id` makes an id by DISL's strategy for a rule's type.
pub fn produce(
    binding: &FblBinding,
    origin: Option<&str>,
    file_name: &str,
    new_id: &dyn Fn(&str) -> String,
) -> Option<Vec<u8>> {
    let template = binding.template.as_ref()?;
    let text = origin
        .and_then(|o| template.by_origin.iter().find(|(k, _)| k == o).map(|(_, t)| t))
        .unwrap_or(&template.text);
    Some(replace(text, file_name, new_id).into_bytes())
}

/// Replaces the placeholder forms of FBL section 13 and leaves every other brace literal.
pub fn replace(template: &str, file_name: &str, new_id: &dyn Fn(&str) -> String) -> String {
    let name = Path::new(file_name)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let base = Path::new(&name)
        .file_stem()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let key_value = key(&base);
    PLACEHOLDER
        .replace_all(template, |captures: &regex::Captures<'_>| match &captures[1] {
            "name" => name.clone(),
            "base" => base.clone(),
            "key" => key_value.clone(),
            "key:lower" => key_value.to_ascii_lowercase(),
            other => new_id(&other["newid:".len()..]),
        })
        .into_owned()
}

/// `{key}` (FBL section 13): every character outside ASCII letters, digits and `_` replaced by `_`, leading and
/// trailing `_` removed, `_` prefixed when it then starts with a digit, and `untitled` when nothing is left.
pub fn key(base_name: &str) -> String {
    let replaced: String = base_name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
        .collect();
    let key = replaced.trim_matches('_');
    if key.is_empty() {
        return "untitled".into();
    }
    if key.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{key}")
    } else {
        key.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globs_match_segments_and_any_depth() {
        assert!(glob_matches("values*.yaml", "values-prod.yaml", false));
        assert!(!glob_matches("*.yaml", "templates/a.yaml", false));
        assert!(glob_matches("**/*.yaml", "templates/a.yaml", false));
        assert!(glob_matches("**/*.yaml", "a.yaml", false));
        assert!(glob_matches("Chart.[yY]aml", "chart.yaml", true));
    }

    #[test]
    fn a_key_is_made_of_what_a_name_allows() {
        assert_eq!(key("My roadmap 2025"), "My_roadmap_2025");
        assert_eq!(key("2025-plan"), "_2025_plan");
        assert_eq!(key("---"), "untitled");
        let replaced = replace("{name} {base} {key} {newid:task} {other}", "dir/1 plan.tml", &|rule| {
            format!("{rule}-1")
        });
        assert_eq!(replaced, "1 plan.tml 1 plan _1_plan task-1 {other}");
    }
}
