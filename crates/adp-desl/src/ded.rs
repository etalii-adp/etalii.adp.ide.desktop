//! A DED definition's envelope (DED sections 2, 4 and 6): the version of DED and the designer type's origin that
//! each document of a designer type begins with, read in each of its forms - two root keys in YAML and JSON, the root
//! element's first two attributes in XML, and line 1 in Markdown - and checked as a host checks it before reading
//! the content: against `ded.schema.json`, for a version this runtime implements, and for a designer type the host
//! has installed. The envelope is the designer type's header (FBL section 5.6); its content is read through the
//! type's bindings, about which DED says nothing (DED section 3).
//!
//! The standalone host reads the envelope only through the Knowledge designer's bindings, whose `required` header
//! makes a file of another version unreadable (`KnowledgeBody.AnotherVersion`). This is DED's own reading, for a host
//! that has to tell a designer's document from any other file before it knows which bindings to use.

use std::sync::LazyLock;

use regex::Regex;
use serde_json::Value;

use crate::schema;

/// The versions of DED this runtime implements (DED section 4, step 2).
pub const DED_VERSIONS: [&str; 2] = ["0.1", "0.2"];

/// Line 1 of a Markdown definition, as DED section 2 reads it.
static MARKDOWN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^<!--[ \t]+ded:[ \t]+(?<ded>[^ \t]+)[ \t]+designer:[ \t]+(?<designer>[^ \t]+)[ \t]+-->[ \t]*$")
        .expect("the envelope pattern compiles")
});

/// One attribute of an XML start tag.
static XML_ATTRIBUTE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"([A-Za-z_][A-Za-z0-9_.:-]*)\s*=\s*(?:"([^"]*)"|'([^']*)')"#).expect("the attribute pattern compiles")
});

/// The form a definition is written in, which says where its envelope is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DedFormat {
    Yaml,
    Json,
    Xml,
    Markdown,
}

impl DedFormat {
    /// The form a file of this name is in, by its extension: the shared extensions of DED section 4, and `.ded`, the
    /// JSON form's own (DED section 5).
    pub fn of_file(file_name: &str) -> Option<DedFormat> {
        let extension = file_name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase())?;
        Some(match extension.as_str() {
            "yaml" | "yml" => DedFormat::Yaml,
            "json" | "ded" => DedFormat::Json,
            "xml" => DedFormat::Xml,
            "md" => DedFormat::Markdown,
            _ => return None,
        })
    }
}

/// A definition's envelope, as read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DedEnvelope {
    pub format: DedFormat,
    /// The version of DED the definition is written in.
    pub ded: String,
    /// The origin of the designer type it was made with.
    pub designer: String,
}

impl DedEnvelope {
    /// The envelope of a file, or why the file is no DED definition: its form is none DED states, or it does not
    /// begin with the envelope.
    pub fn read(bytes: &[u8], file_name: &str) -> Result<DedEnvelope, String> {
        let Some(format) = DedFormat::of_file(file_name) else {
            return Err(format!("'{file_name}' is in no form a DED definition is written in."));
        };
        let text = String::from_utf8_lossy(bytes.strip_prefix(&[0xEF, 0xBB, 0xBF][..]).unwrap_or(bytes)).into_owned();
        let found = match format {
            DedFormat::Yaml => yaml(&text),
            DedFormat::Json => json(&text),
            DedFormat::Xml => xml(&text),
            DedFormat::Markdown => markdown(&text),
        };
        found
            .map(|(ded, designer)| DedEnvelope { format, ded, designer })
            .ok_or_else(|| format!("'{file_name}' does not begin with a DED envelope (DED §2)."))
    }

    /// What `ded.schema.json` says of the envelope, read into its YAML and JSON form (DED section 6): each break as
    /// a sentence. None for an envelope the schema accepts.
    pub fn findings(&self) -> Vec<String> {
        let instance = serde_json::json!({ "ded": self.ded, "designer": self.designer });
        schema::check_definition(&instance)
            .into_iter()
            .map(|(pointer, message)| {
                format!(
                    "The envelope's {}: {message} (ded.schema.json)",
                    pointer.trim_start_matches('/')
                )
            })
            .collect()
    }

    /// Why the definition opens read-only, if at all (DED section 4, steps 2 and 3): a version of DED this runtime
    /// does not implement, or a designer type `installed` does not know. None when it is read as DED says.
    pub fn read_only_reason(&self, installed: impl Fn(&str) -> bool) -> Option<String> {
        if !DED_VERSIONS.contains(&self.ded.as_str()) {
            return Some(format!(
                "This document is written in DED {}, a version this application does not know. It is left exactly as it is and cannot be changed here.",
                self.ded
            ));
        }
        if !installed(&self.designer) {
            return Some(format!(
                "This document was made with the designer type '{}', which is not installed. It is left exactly as it is and cannot be changed here.",
                self.designer
            ));
        }
        None
    }

    /// When a registration names another designer type than the envelope, the finding that says so: the envelope
    /// decides (DED section 4).
    pub fn registration_finding(&self, registered: &str) -> Option<String> {
        (registered != self.designer).then(|| {
            format!(
                "The registration names the designer type '{registered}', and the document says it was made with '{}'; it is opened with '{}'.",
                self.designer, self.designer
            )
        })
    }
}

/// A YAML scalar's text: unquoted, its comment left off.
fn scalar(raw: &str) -> String {
    let raw = raw.trim();
    for quote in ['"', '\''] {
        if let Some(rest) = raw.strip_prefix(quote)
            && let Some(end) = rest.find(quote)
        {
            return rest[..end].to_owned();
        }
    }
    match raw.find(" #") {
        Some(at) => raw[..at].trim_end().to_owned(),
        None => raw.to_owned(),
    }
}

/// The first two root keys, when they are `ded` and `designer` in this order.
fn yaml(text: &str) -> Option<(String, String)> {
    let mut keys = text
        .lines()
        .filter(|line| {
            !line.trim().is_empty() && !line.starts_with('#') && !line.starts_with(' ') && !line.starts_with('\t')
        })
        .filter(|line| !line.starts_with("---") && !line.starts_with('%'));
    let ded = keys.next()?.strip_prefix("ded:")?;
    let designer = keys.next()?.strip_prefix("designer:")?;
    Some((scalar(ded), scalar(designer)))
}

fn json(text: &str) -> Option<(String, String)> {
    let value: Value = serde_json::from_str(text).ok()?;
    let mut members = value.as_object()?.iter();
    let (first, ded) = members.next()?;
    let (second, designer) = members.next()?;
    if first != "ded" || second != "designer" {
        return None;
    }
    Some((ded.as_str()?.to_owned(), designer.as_str()?.to_owned()))
}

/// The root element's start tag, when it is `ded` and its first two attributes are `version` and `designer`.
fn xml(text: &str) -> Option<(String, String)> {
    let mut rest = text.trim_start();
    // The prolog: a declaration, comments, processing instructions and a document type, before the root.
    loop {
        if let Some(after) = rest.strip_prefix("<?") {
            rest = after.split_once("?>")?.1.trim_start();
        } else if let Some(after) = rest.strip_prefix("<!--") {
            rest = after.split_once("-->")?.1.trim_start();
        } else if let Some(after) = rest.strip_prefix("<!") {
            rest = after.split_once('>')?.1.trim_start();
        } else {
            break;
        }
    }
    let tag = rest.strip_prefix("<ded")?;
    if !tag.starts_with(|c: char| c.is_whitespace() || c == '>' || c == '/') {
        return None;
    }
    let end = tag.find('>')?;
    let mut attributes = XML_ATTRIBUTE.captures_iter(&tag[..end]).map(|c| {
        let value = c.get(2).or_else(|| c.get(3)).map(|m| m.as_str()).unwrap_or_default();
        (c[1].to_owned(), value.to_owned())
    });
    let (first, version) = attributes.next()?;
    let (second, designer) = attributes.next()?;
    (first == "version" && second == "designer").then_some((version, designer))
}

fn markdown(text: &str) -> Option<(String, String)> {
    let line = text.lines().next()?;
    let captures = MARKDOWN.captures(line)?;
    Some((captures["ded"].to_owned(), captures["designer"].to_owned()))
}
