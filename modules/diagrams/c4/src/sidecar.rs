//! Where the positions a person arranged by hand are kept: the layout sidecar beside the `.dsl` (etalii.adp spec 030,
//! task T061).
//!
//! Ported from the standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4/C4LayoutSidecar.cs` and
//! `C4SidecarPosition.cs` at `8b6ccfc`. A sidecar rather than the `.dsl`, because the DSL is a format another
//! ecosystem owns. One file per document, `<base>.layout.json`, keyed by view and then by element, both compared
//! without regard to case. On opening it is an optimisation, never a dependency: anything that cannot be read yields
//! no positions. On a change the whole file is read, changed and written again, so a file that is there and cannot be
//! read refuses the change rather than writing an empty arrangement over every other view's. It is written as
//! `System.Text.Json` wrote it: indented by two, the platform's line ending, each number as .NET writes a double, and
//! each name escaped as its default encoder escapes it.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value as Json;

use crate::model::same;

/// One view's authored positions, by element id compared without regard to case, in the order the file lists them.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Positions {
    entries: Vec<(String, f64, f64)>,
}

impl Positions {
    pub fn get(&self, id: &str) -> Option<(f64, f64)> {
        self.entries
            .iter()
            .find(|(key, _, _)| same(key, id))
            .map(|(_, x, y)| (*x, *y))
    }

    /// Records `id` at (`x`, `y`): an id already there keeps its place and its spelling.
    pub fn set(&mut self, id: &str, x: f64, y: f64) {
        match self.entries.iter_mut().find(|(key, _, _)| same(key, id)) {
            Some(entry) => {
                entry.1 = x;
                entry.2 = y;
            }
            None => self.entries.push((id.to_owned(), x, y)),
        }
    }

    pub fn remove(&mut self, id: &str) {
        self.entries.retain(|(key, _, _)| !same(key, id));
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|(key, _, _)| key.as_str())
    }

    /// Every position, `(id, x, y)`, in the file's order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, f64, f64)> {
        self.entries.iter().map(|(key, x, y)| (key.as_str(), *x, *y))
    }
}

/// The whole sidecar: every view's positions, by view key compared without regard to case, in the file's order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Sidecar {
    views: Vec<(String, Positions)>,
}

impl Sidecar {
    pub fn view(&self, key: &str) -> Option<&Positions> {
        self.views.iter().find(|(k, _)| same(k, key)).map(|(_, p)| p)
    }

    fn view_mut(&mut self, key: &str) -> &mut Positions {
        let at = match self.views.iter().position(|(k, _)| same(k, key)) {
            Some(at) => at,
            None => {
                self.views.push((key.to_owned(), Positions::default()));
                self.views.len() - 1
            }
        };
        &mut self.views[at].1
    }

    /// The sidecar as `System.Text.Json` writes it indented, lines ending in `newline`.
    pub fn to_json(&self, newline: &str) -> String {
        if self.views.is_empty() {
            return "{}".to_owned();
        }
        let mut out = String::from("{");
        for (index, (key, positions)) in self.views.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            out.push_str(newline);
            out.push_str("  ");
            out.push_str(&json_string(key));
            out.push_str(": ");
            if positions.is_empty() {
                out.push_str("{}");
                continue;
            }
            out.push('{');
            for (inner, (id, x, y)) in positions.iter().enumerate() {
                if inner > 0 {
                    out.push(',');
                }
                out.push_str(newline);
                out.push_str("    ");
                out.push_str(&json_string(id));
                out.push_str(": {");
                out.push_str(newline);
                out.push_str("      \"x\": ");
                out.push_str(&adp_fbl::planning::dotnet_double(x));
                out.push(',');
                out.push_str(newline);
                out.push_str("      \"y\": ");
                out.push_str(&adp_fbl::planning::dotnet_double(y));
                out.push_str(newline);
                out.push_str("    }");
            }
            out.push_str(newline);
            out.push_str("  }");
        }
        out.push_str(newline);
        out.push('}');
        out
    }

    /// Reads a sidecar's text, or why it is not one.
    pub fn parse(text: &str) -> Result<Sidecar, String> {
        let json: Json = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let Json::Object(views) = json else {
            return Err("A layout sidecar is an object of views.".to_owned());
        };
        let mut sidecar = Sidecar::default();
        for (key, positions) in views {
            let Json::Object(positions) = positions else {
                return Err(format!("The view '{key}' is not an object of positions."));
            };
            let view = sidecar.view_mut(&key);
            for (id, position) in positions {
                let Json::Object(position) = position else {
                    return Err(format!("The position of '{id}' is not an object."));
                };
                let number = |name: &str| -> Result<f64, String> {
                    match position.get(name) {
                        None => Ok(0.0),
                        Some(value) => value
                            .as_f64()
                            .ok_or_else(|| format!("The {name} of '{id}' is not a number.")),
                    }
                };
                view.set(&id, number("x")?, number("y")?);
            }
        }
        Ok(sidecar)
    }
}

/// The platform's line ending, `Environment.NewLine`, which the standalone host wrote the sidecar with.
pub fn newline() -> &'static str {
    if cfg!(windows) { "\r\n" } else { "\n" }
}

/// The sidecar beside `body_path`: the same base name, `.layout.json`.
pub fn path_for(body_path: &Path) -> PathBuf {
    let stem = body_path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    body_path.with_file_name(format!("{stem}.layout.json"))
}

/// The authored positions of view `view_key`, or none: anything that cannot be read yields none.
pub fn read(body_path: &Path, view_key: &str) -> Positions {
    read_all(body_path).view(view_key).cloned().unwrap_or_default()
}

/// The whole sidecar for the open path: absent, locked or nonsense alike yield an empty one.
pub fn read_all(body_path: &Path) -> Sidecar {
    fs::read_to_string(path_for(body_path))
        .ok()
        .and_then(|text| Sidecar::parse(text.trim_start_matches('\u{feff}')).ok())
        .unwrap_or_default()
}

/// The whole sidecar for a change: none when the file is there and could not be read, so that nothing is written over
/// it; an empty one when it is absent or its content is nonsense, which the change then replaces.
fn read_all_for_modify(body_path: &Path) -> Option<Sidecar> {
    let path = path_for(body_path);
    if !path.exists() {
        return Some(Sidecar::default());
    }
    let text = fs::read_to_string(&path).ok()?;
    Some(Sidecar::parse(text.trim_start_matches('\u{feff}')).unwrap_or_default())
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// What a change answers when the sidecar could not be updated at all.
fn unreadable(body_path: &Path) -> String {
    format!(
        "The layout beside {} could not be updated, so positions may not be as expected when the diagram is reopened.",
        file_name(body_path)
    )
}

/// Writes the sidecar: empty when it landed, else the sentence that says it did not.
fn persist(body_path: &Path, sidecar: &Sidecar) -> String {
    let text = sidecar.to_json(newline());
    match adp_documents::text::write_atomically(&path_for(body_path), text.as_bytes()) {
        Ok(()) => String::new(),
        Err(_) => format!(
            "The new position could not be saved beside {}, so it will not be there when the diagram is reopened.",
            file_name(body_path)
        ),
    }
}

/// Records one element's position on one view, leaving every other view's alone: empty when it landed, else what
/// went wrong.
pub fn write(body_path: &Path, view_key: &str, element_id: &str, x: f64, y: f64) -> String {
    let Some(mut sidecar) = read_all_for_modify(body_path) else {
        return unreadable(body_path);
    };
    sidecar.view_mut(view_key).set(element_id, x, y);
    persist(body_path, &sidecar)
}

/// Forgets one element's authored position, on every view, handing it back to the layout.
pub fn remove(body_path: &Path, element_id: &str) -> String {
    let Some(mut sidecar) = read_all_for_modify(body_path) else {
        return unreadable(body_path);
    };
    for (_, positions) in &mut sidecar.views {
        positions.remove(element_id);
    }
    persist(body_path, &sidecar)
}

/// Forgets every authored position of one view; the file goes when no view is left.
pub fn clear(body_path: &Path, view_key: &str) -> String {
    let Some(mut sidecar) = read_all_for_modify(body_path) else {
        return unreadable(body_path);
    };
    sidecar.views.retain(|(key, _)| !same(key, view_key));
    if sidecar.views.is_empty() {
        let path = path_for(body_path);
        if path.exists() && fs::remove_file(&path).is_err() {
            return unreadable(body_path);
        }
        return String::new();
    }
    persist(body_path, &sidecar)
}

/// `value` as a JSON string, escaped as `System.Text.Json`'s default encoder escapes it: the quote, HTML-sensitive
/// characters and everything outside printable ASCII as `\uXXXX`, the usual short forms for the backslash and the
/// common control characters.
fn json_string(value: &str) -> String {
    let mut out = String::from("\"");
    for unit in value.encode_utf16() {
        match unit {
            0x5C => out.push_str("\\\\"),
            0x08 => out.push_str("\\b"),
            0x09 => out.push_str("\\t"),
            0x0A => out.push_str("\\n"),
            0x0C => out.push_str("\\f"),
            0x0D => out.push_str("\\r"),
            0x22 | 0x26 | 0x27 | 0x2B | 0x3C | 0x3E | 0x60 => out.push_str(&format!("\\u{unit:04X}")),
            0x20..=0x7E => out.push(char::from(unit as u8)),
            _ => out.push_str(&format!("\\u{unit:04X}")),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_escaped_as_the_default_encoder_escapes_it() {
        assert_eq!(json_string("web"), "\"web\"");
        assert_eq!(json_string("a\"b"), "\"a\\u0022b\"");
        assert_eq!(json_string("<&>"), "\"\\u003C\\u0026\\u003E\"");
        assert_eq!(json_string("é"), "\"\\u00E9\"");
    }
}
