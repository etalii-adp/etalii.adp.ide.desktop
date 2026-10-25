//! The templates of a new description, one per kind (etalii.adp spec 035, FR-060; contracts/checker.md `atd new`,
//! contracts/language-server.md `atd/newDescription`): a tool type with an origin and a display name that checks
//! without findings and is already in the canonical layout, a small working example of its kind to grow from.

use std::fmt;
use std::str::FromStr;

/// The kind of a description (ATD §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Diagram,
    Designer,
    Editor,
}

impl Kind {
    pub const ALL: [Kind; 3] = [Kind::Diagram, Kind::Designer, Kind::Editor];

    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Diagram => "diagram",
            Kind::Designer => "designer",
            Kind::Editor => "editor",
        }
    }

    fn text(self) -> &'static str {
        match self {
            Kind::Diagram => include_str!("diagram.atd"),
            Kind::Designer => include_str!("designer.atd"),
            Kind::Editor => include_str!("editor.atd"),
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Kind {
    type Err = String;

    fn from_str(kind: &str) -> Result<Self, String> {
        Kind::ALL
            .into_iter()
            .find(|k| k.as_str() == kind)
            .ok_or_else(|| format!("'{kind}' is not a kind of description: diagram, designer or editor."))
    }
}

/// The text of a new description of `kind` with the origin `origin` (`<vendor>/<type>`, ATD §3 `ORIGIN`) and the
/// display name `name`; the reason when the origin or the name cannot be written.
pub fn template(kind: Kind, origin: &str, name: &str) -> Result<String, String> {
    let Some((_, type_name)) = origin.split_once('/').filter(|_| is_origin(origin)) else {
        return Err(format!("'{origin}' is not an origin of the form <vendor>/<type>."));
    };
    if name.trim().is_empty() {
        return Err("A description needs a name.".to_owned());
    }
    if name.chars().any(char::is_control) {
        return Err("A description's name is one line of text.".to_owned());
    }
    let quoted = name.replace('\\', "\\\\").replace('"', "\\\"");
    Ok(kind
        .text()
        .replace("/// __NAME__:", &format!("/// {name}:"))
        .replace("__NAME__", &quoted)
        .replace("__ORIGIN__", origin)
        .replace("__TYPE__", type_name))
}

/// The file name `atd new` writes a description of `origin` to: its type, with `.atd`.
pub fn file_name(origin: &str) -> Option<String> {
    let (_, type_name) = origin.split_once('/').filter(|_| is_origin(origin))?;
    Some(format!("{type_name}.atd"))
}

/// `[A-Za-z0-9_][A-Za-z0-9_.-]*/[A-Za-z0-9_][A-Za-z0-9_.-]*`, the terminal `ORIGIN` (ATD §3).
fn is_origin(origin: &str) -> bool {
    let segment = |s: &str| {
        let mut chars = s.chars();
        chars.next().is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
            && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
    };
    origin
        .split_once('/')
        .is_some_and(|(vendor, type_name)| segment(vendor) && segment(type_name))
}
