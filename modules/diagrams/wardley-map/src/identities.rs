//! The identities the `.owm` format does not provide, and the sidecar beside the map that keeps them (etalii.adp spec
//! 030, task T070).
//!
//! Ported from the standalone host's `WardleyIdentities.cs`, `WardleyIdentityKeys.cs`, `_Model/WardleyIdentityEntry.cs`
//! and `_Model/WardleyIdentityKind.cs` at `8b6ccfc`. An `.owm` element has no id of any kind, so every element is
//! given a ShortGuid and found again by its kind and a natural key: a component's name, a link's ends and kind, a
//! pipeline's parent, a pipeline child's parent and name, a note's text, an annotation's number, an accelerator's
//! name and direction, an attitude's kind and first corner. They are kept in `{name}.identities.json` beside the body,
//! because the `.owm` belongs to another ecosystem. The sidecar is an optimisation, never a dependency: a missing,
//! unreadable or nonsense one means identities are derived again, and reconciling keeps every id whose key still
//! matches.

use std::fs;
use std::path::{Path, PathBuf};

use adp_disl::IdSource;
use serde_json::Value as Json;

use crate::model::{WardleyComponent, WardleyMap};

/// What separates the parts of a composite key: the ASCII unit separator, which no name can contain.
pub const SEPARATOR: char = '\u{1F}';

/// What sort of thing an identity names (`WardleyIdentityKind`); two elements of different kinds may share a key.
pub mod kind {
    pub const COMPONENT: &str = "component";
    pub const LINK: &str = "link";
    pub const PIPELINE: &str = "pipeline";
    pub const PIPELINE_CHILD: &str = "pipeline-child";
    pub const NOTE: &str = "note";
    pub const ANNOTATION: &str = "annotation";
    pub const ACCELERATOR: &str = "accelerator";
    pub const ATTITUDE: &str = "attitude";
}

/// One element's identity, and the handle the `.owm` gives for finding it again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WardleyIdentityEntry {
    pub id: String,
    pub kind: String,
    pub key: String,
}

impl WardleyIdentityEntry {
    pub fn new(id: impl Into<String>, kind: impl Into<String>, key: impl Into<String>) -> Self {
        WardleyIdentityEntry {
            id: id.into(),
            kind: kind.into(),
            key: key.into(),
        }
    }
}

/// The natural keys (`WardleyIdentityKeys.Of`).
pub mod keys {
    use super::SEPARATOR;
    use crate::model::{
        WardleyAccelerator, WardleyAnnotation, WardleyAttitude, WardleyComponent, WardleyLink, WardleyNote,
        WardleyPipeline, WardleyPipelineChild,
    };

    /// A component, anchor or submap: its name.
    pub fn component(component: &WardleyComponent) -> String {
        component.name.clone()
    }

    /// A link: both ends and its kind, so two links between one pair stay distinct.
    pub fn link(link: &WardleyLink) -> String {
        link_of(&link.source, &link.target, link.kind.name())
    }

    /// The key of a link from `source` to `target` of the kind .NET names `kind`.
    pub fn link_of(source: &str, target: &str, kind: &str) -> String {
        format!("{source}{SEPARATOR}{target}{SEPARATOR}{kind}")
    }

    /// A pipeline: its parent's name.
    pub fn pipeline(pipeline: &WardleyPipeline) -> String {
        pipeline.parent.clone()
    }

    /// A pipeline child: its parent's name and its own.
    pub fn pipeline_child(pipeline: &WardleyPipeline, child: &WardleyPipelineChild) -> String {
        format!("{}{SEPARATOR}{}", pipeline.parent, child.name)
    }

    /// A note: its text, the only handle it has.
    pub fn note(note: &WardleyNote) -> String {
        note.text.clone()
    }

    /// An annotation: its number.
    pub fn annotation(annotation: &WardleyAnnotation) -> String {
        annotation.number.to_string()
    }

    /// An accelerator: its name and its direction.
    pub fn accelerator(accelerator: &WardleyAccelerator) -> String {
        format!(
            "{}{SEPARATOR}{}",
            accelerator.name,
            if accelerator.is_deaccelerator { "de" } else { "ac" }
        )
    }

    /// An attitude region: its kind and one corner, its numbers as .NET writes a double.
    pub fn attitude(attitude: &WardleyAttitude) -> String {
        format!(
            "{}{SEPARATOR}{}{SEPARATOR}{}",
            attitude.kind.name(),
            adp_fbl::planning::dotnet_double(attitude.from.visibility),
            adp_fbl::planning::dotnet_double(attitude.from.maturity)
        )
    }
}

/// The element type a new id of `kind` is asked for under, as the standalone host's diagram stream typed the element
/// it reported (`wardley/map+element` for a component and a pipeline child).
pub fn element_type_of(kind: &str) -> &'static str {
    match kind {
        kind::COMPONENT | kind::PIPELINE_CHILD => "element",
        kind::LINK => "link",
        kind::NOTE => "note",
        kind::ANNOTATION => "annotation",
        kind::ACCELERATOR => "accelerator",
        kind::ATTITUDE => "attitude",
        _ => "pipeline",
    }
}

/// A new ShortGuid: a version 4 GUID as 25 characters of base 36.
pub fn short_guid() -> String {
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut value = uuid::Uuid::new_v4().as_u128();
    let mut digits = [b'0'; 25];
    for digit in digits.iter_mut().rev() {
        *digit = DIGITS[(value % 36) as usize];
        value /= 36;
    }
    String::from_utf8(digits.to_vec()).expect("base 36 digits")
}

/// The source of fresh ShortGuids a running host mints.
#[derive(Debug, Default, Clone, Copy)]
pub struct ShortGuids;

impl IdSource for ShortGuids {
    fn next(&self, _type_name: &str) -> Result<String, String> {
        Ok(short_guid())
    }
}

/// Matches recorded identities to the elements of `map`: an element the record knows keeps its id, one it does not
/// gets a fresh one from `ids`, and an entry no element claims is dropped. The complete current set, in a stable
/// order: components, links, pipelines with their children, notes, annotations, accelerators, attitudes.
pub fn reconcile(map: &WardleyMap, recorded: &[WardleyIdentityEntry], ids: &dyn IdSource) -> Vec<WardleyIdentityEntry> {
    let known = |kind: &str, key: &str| {
        recorded
            .iter()
            .find(|e| e.kind == kind && e.key == key)
            .map(|e| e.id.clone())
    };
    let mut reconciled = Vec::new();
    let mut take = |kind: &str, key: String| {
        let id = known(kind, &key).unwrap_or_else(|| ids.next(element_type_of(kind)).unwrap_or_else(|_| short_guid()));
        reconciled.push(WardleyIdentityEntry::new(id, kind, key));
    };
    for component in &map.components {
        take(kind::COMPONENT, keys::component(component));
    }
    for link in &map.links {
        take(kind::LINK, keys::link(link));
    }
    for pipeline in &map.pipelines {
        take(kind::PIPELINE, keys::pipeline(pipeline));
        for child in &pipeline.children {
            take(kind::PIPELINE_CHILD, keys::pipeline_child(pipeline, child));
        }
    }
    for note in &map.notes {
        take(kind::NOTE, keys::note(note));
    }
    for annotation in &map.annotations {
        take(kind::ANNOTATION, keys::annotation(annotation));
    }
    for accelerator in &map.accelerators {
        take(kind::ACCELERATOR, keys::accelerator(accelerator));
    }
    for attitude in &map.attitudes {
        take(kind::ATTITUDE, keys::attitude(attitude));
    }
    reconciled
}

/// The id of the element of `kind` keyed `key`, or empty.
pub fn id_of(identities: &[WardleyIdentityEntry], kind: &str, key: &str) -> String {
    identities
        .iter()
        .find(|e| e.kind == kind && e.key == key)
        .map(|e| e.id.clone())
        .unwrap_or_default()
}

// ---- the sidecar ---------------------------------------------------------------------------------------------------

/// The sidecar beside `body_path`: the same base name, `.identities.json`.
pub fn path_for(body_path: &Path) -> PathBuf {
    let stem = body_path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    body_path.with_file_name(format!("{stem}.identities.json"))
}

/// The identities recorded beside `body_path`, or none: anything that cannot be read yields none, and a half-written
/// entry (no id, kind or key) is dropped.
pub fn read(body_path: &Path) -> Vec<WardleyIdentityEntry> {
    let Ok(text) = fs::read_to_string(path_for(body_path)) else {
        return Vec::new();
    };
    let Ok(Json::Array(entries)) = serde_json::from_str::<Json>(text.trim_start_matches('\u{feff}')) else {
        return Vec::new();
    };
    let mut read = Vec::with_capacity(entries.len());
    for entry in entries {
        // A value of the wrong type makes the whole file unreadable, as a JsonException does.
        let Json::Object(entry) = entry else {
            if entry.is_null() {
                continue;
            }
            return Vec::new();
        };
        let text = |name: &str| -> Result<String, ()> {
            match entry.get(name) {
                None | Some(Json::Null) => Ok(String::new()),
                Some(Json::String(s)) => Ok(s.clone()),
                Some(_) => Err(()),
            }
        };
        let (Ok(id), Ok(kind), Ok(key)) = (text("id"), text("kind"), text("key")) else {
            return Vec::new();
        };
        if !id.is_empty() && !kind.is_empty() && !key.is_empty() {
            read.push(WardleyIdentityEntry { id, kind, key });
        }
    }
    read
}

/// The platform's line ending, `Environment.NewLine`, which the standalone host wrote the sidecar with.
pub fn newline() -> &'static str {
    if cfg!(windows) { "\r\n" } else { "\n" }
}

/// `entries` as `System.Text.Json` writes the list indented and camel-cased, lines ending in `newline`.
pub fn to_json(entries: &[WardleyIdentityEntry], newline: &str) -> String {
    if entries.is_empty() {
        return "[]".to_owned();
    }
    let mut out = String::from("[");
    for (index, entry) in entries.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(newline);
        out.push_str("  {");
        for (inner, (name, value)) in [("id", &entry.id), ("kind", &entry.kind), ("key", &entry.key)]
            .into_iter()
            .enumerate()
        {
            if inner > 0 {
                out.push(',');
            }
            out.push_str(newline);
            out.push_str("    \"");
            out.push_str(name);
            out.push_str("\": ");
            out.push_str(&json_string(value));
        }
        out.push_str(newline);
        out.push_str("  }");
    }
    out.push_str(newline);
    out.push(']');
    out
}

/// Records `entries` beside `body_path`, replacing whatever was there, written atomically; an empty list removes the
/// sidecar. Empty when it landed, else the warning the save carries: the map landed and its ids may not survive.
pub fn write(body_path: &Path, entries: &[WardleyIdentityEntry]) -> String {
    let path = path_for(body_path);
    if entries.is_empty() {
        remove(body_path);
        return String::new();
    }
    let written = path
        .parent()
        .map_or(Ok(()), fs::create_dir_all)
        .and_then(|()| adp_documents::text::write_atomically(&path, to_json(entries, newline()).as_bytes()));
    match written {
        Ok(()) => String::new(),
        Err(_) => format!(
            "The element identities beside {} could not be saved, so this map's elements may be given new ids the next time it is opened.",
            body_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        ),
    }
}

/// Deletes the sidecar, if there is one.
pub fn remove(body_path: &Path) {
    let path = path_for(body_path);
    if path.exists() {
        let _ = fs::remove_file(path);
    }
}

/// `value` as a JSON string, escaped as `System.Text.Json`'s default encoder escapes it: the quote, HTML-sensitive
/// characters and everything outside printable ASCII as `\uXXXX`, the usual short forms for the backslash and the
/// common control characters.
pub fn json_string(value: &str) -> String {
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

/// The entry keyed for `component`.
pub fn component_entry<'a>(
    identities: &'a [WardleyIdentityEntry],
    component: &WardleyComponent,
) -> Option<&'a WardleyIdentityEntry> {
    identities
        .iter()
        .find(|e| e.kind == kind::COMPONENT && e.key == keys::component(component))
}
