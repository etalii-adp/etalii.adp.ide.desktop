//! The XML editor's analysis (etalii.adp spec 030, FR-020, task T032), after the standalone host's XML editor at
//! `8b6ccfc` (xml-editor: `WellFormedness.cs`, `SchemaAssociation.cs`, `XmlAnalyzer.cs`).
//!
//! - **Well-formedness**: the text is read to its end or its first error, which becomes one finding where the reader
//!   stopped; the status then says the rest was not checked.
//! - **The declared encoding**: a declaration of an encoding other than the one the file is stored in is a warning,
//!   unless every character is ASCII and so reads the same either way.
//! - **The schemas a document names**, from its prolog and its root's start tag only: `xsi:noNamespaceSchemaLocation`,
//!   the pairs of `xsi:schemaLocation`, and the `DOCTYPE`'s system identifier. Each is resolved by the project-only
//!   rules the JSON editor uses; one that cannot be followed is an information finding on its reference. An
//!   `xml-model` instruction is named and not supported; a root `xs:schema` makes the document a schema.
//!
//! **Not yet**: checking a document against its XSD or DTD. The standalone host compiles and checks both; the desktop
//! host names the schema in the status line as not checked. Offsets are bytes into the text.

use std::ops::Range;

use adp_editor_json::{SchemaTarget, resolve_schema_reference};

const XSI: &str = "http://www.w3.org/2001/XMLSchema-instance";
const XS: &str = "http://www.w3.org/2001/XMLSchema";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub span: Range<usize>,
    pub severity: Severity,
    pub message: String,
}

/// A schema a document names: its kind, where it is, and where the name stands in the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Named {
    pub kind: SchemaKind,
    /// The namespace it is for; empty for none and for a DTD.
    pub namespace: String,
    pub location: String,
    pub span: Range<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaKind {
    Xsd,
    Dtd,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Analysis {
    pub findings: Vec<Finding>,
    pub status: String,
    pub schemas: Vec<Named>,
}

/// The sentence the standalone host's status line gives a document that is not well-formed.
pub const NOT_CHECKED: &str = "Not well-formed: the rest was not checked.";

/// Analyses an XML text; `document` is its path in the project, its file name last; `stored` is how it is stored
/// ("UTF-8" or "UTF-8 with BOM").
pub fn analyse(text: &str, document: &[String], stored: &str) -> Analysis {
    let mut findings = Vec::new();
    if let Some(finding) = declared_encoding(text, stored) {
        findings.push(finding);
    }
    let options = roxmltree::ParsingOptions {
        allow_dtd: true,
        ..Default::default()
    };
    let parsed = roxmltree::Document::parse_with_options(text, options);
    let schemas = match &parsed {
        Ok(tree) => named_schemas(text, tree),
        Err(_) => Vec::new(),
    };
    if let Err(error) = &parsed {
        let position = error.pos();
        let at = offset_of(text, position.row as usize, position.col as usize);
        let end = text[at..]
            .chars()
            .next()
            .map_or(at, |c| at + c.len_utf8())
            .max(at + 1)
            .min(text.len().max(at + 1));
        findings.push(Finding {
            span: at..end,
            severity: Severity::Error,
            message: sentence(&error.to_string()),
        });
        let names_a_schema = text.contains("schemaLocation") || text.contains("<!DOCTYPE");
        let status = if names_a_schema {
            format!("{NOT_CHECKED} Schema not checked.")
        } else {
            NOT_CHECKED.into()
        };
        return Analysis {
            findings,
            status,
            schemas,
        };
    }
    let tree = parsed.as_ref().ok();
    if let Some(root) = tree.map(roxmltree::Document::root_element)
        && root.tag_name().namespace() == Some(XS)
        && root.tag_name().name() == "schema"
    {
        return Analysis {
            findings,
            status: "XSD: not checked yet.".into(),
            schemas,
        };
    }
    for pi in tree.into_iter().flat_map(|t| t.root().children()).filter(|n| n.is_pi()) {
        if let Some(instruction) = pi.pi()
            && instruction.target == "xml-model"
        {
            let language = if instruction.value.unwrap_or_default().contains(XS) {
                "XSD"
            } else {
                "a schema language"
            };
            findings.push(Finding {
                span: pi.range(),
                severity: Severity::Info,
                message: format!(
                    "An xml-model instruction names {language}, which this editor does not check: it checks XSD 1.0 and DTD named by the document."
                ),
            });
        }
    }
    if schemas.is_empty() {
        return Analysis {
            findings,
            status: "No schema".into(),
            schemas,
        };
    }
    let mut names = Vec::new();
    for named in &schemas {
        match resolve_schema_reference(&named.location, document) {
            SchemaTarget::Project(path) => names.push(path.last().cloned().unwrap_or_default()),
            SchemaTarget::None(reason)
                if reason.starts_with("is a web address") || reason == "is not a file of the project" =>
            {
                findings.push(Finding {
                    span: named.span.clone(),
                    severity: Severity::Info,
                    message: format!(
                        "'{}' is not in the project, so no schema is checked from it.",
                        named.location
                    ),
                });
            }
            SchemaTarget::None(reason) => findings.push(Finding {
                span: named.span.clone(),
                severity: Severity::Info,
                message: format!("'{}' {reason}.", named.location),
            }),
            SchemaTarget::Bundled(address) => names.push(address),
        }
    }
    let status = if names.is_empty() {
        "No schema".into()
    } else {
        format!("Schema: {} (not checked yet)", names.join(", "))
    };
    Analysis {
        findings,
        status,
        schemas,
    }
}

/// The schemas the prolog and the root's start tag name.
fn named_schemas(text: &str, tree: &roxmltree::Document) -> Vec<Named> {
    let mut named = Vec::new();
    // The DOCTYPE's system identifier.
    if let Some(start) = text.find("<!DOCTYPE") {
        let declaration = &text[start..text[start..].find(['[', '>']).map_or(text.len(), |e| start + e)];
        if let Some(system) = declaration.find("SYSTEM").or_else(|| declaration.find("PUBLIC")) {
            let quotes: Vec<(usize, char)> = declaration[system..]
                .char_indices()
                .filter(|(_, c)| *c == '"' || *c == '\'')
                .collect();
            // SYSTEM "x": the first quoted string; PUBLIC "id" "x": the second.
            let pair = if declaration[system..].starts_with("PUBLIC") {
                quotes.get(2..4)
            } else {
                quotes.get(0..2)
            };
            if let Some([(open, _), (close, _)]) = pair {
                let from = start + system + open + 1;
                let to = start + system + close;
                named.push(Named {
                    kind: SchemaKind::Dtd,
                    namespace: String::new(),
                    location: text[from..to].to_owned(),
                    span: from..to,
                });
            }
        }
    }
    let root = tree.root_element();
    for attribute in root.attributes() {
        if attribute.namespace() != Some(XSI) {
            continue;
        }
        let value_range = attribute.range_value();
        match attribute.name() {
            "noNamespaceSchemaLocation" => named.push(Named {
                kind: SchemaKind::Xsd,
                namespace: String::new(),
                location: attribute.value().to_owned(),
                span: value_range,
            }),
            "schemaLocation" => {
                let raw = &text[value_range.clone()];
                let mut tokens = Vec::new();
                let mut at = 0;
                for token in raw.split_whitespace() {
                    let from = raw[at..].find(token).map_or(at, |i| at + i);
                    tokens.push((token, value_range.start + from..value_range.start + from + token.len()));
                    at = from + token.len();
                }
                for pair in tokens.chunks(2) {
                    if let [(namespace, _), (location, span)] = pair {
                        named.push(Named {
                            kind: SchemaKind::Xsd,
                            namespace: (*namespace).to_owned(),
                            location: (*location).to_owned(),
                            span: span.clone(),
                        });
                    }
                }
            }
            _ => {}
        }
    }
    named
}

/// The warning when the declaration names an encoding other than the stored one and the text is not plain ASCII.
fn declared_encoding(text: &str, stored: &str) -> Option<Finding> {
    let prolog = text.strip_prefix('\u{feff}').unwrap_or(text);
    if !prolog.starts_with("<?xml") {
        return None;
    }
    let end = prolog.find("?>")?;
    let declaration = &prolog[..end];
    let at = declaration.find("encoding")?;
    let rest = &declaration[at..];
    let open = rest.find(['"', '\''])?;
    let quote = rest[open..].chars().next()?;
    let close = rest[open + 1..].find(quote)?;
    let declared = &rest[open + 1..open + 1 + close];
    let offset = text.len() - prolog.len() + at + open + 1;
    let agrees = declared.eq_ignore_ascii_case("utf-8") || declared.eq_ignore_ascii_case("utf8") || text.is_ascii();
    (!agrees).then(|| Finding {
        span: offset..offset + declared.len(),
        severity: Severity::Warning,
        message: format!("The declaration says the file is {declared}, but it is stored as {stored}."),
    })
}

/// The byte offset of a 1-based row and column.
fn offset_of(text: &str, row: usize, column: usize) -> usize {
    let line_start: usize = text
        .split_inclusive('\n')
        .take(row.saturating_sub(1))
        .map(str::len)
        .sum();
    let line = text[line_start..].split('\n').next().unwrap_or_default();
    line_start
        + line
            .char_indices()
            .nth(column.saturating_sub(1))
            .map_or(line.len(), |(b, _)| b)
}

/// roxmltree's message without its trailing position, which the underline already shows, as a sentence.
fn sentence(message: &str) -> String {
    let text = message.split(" at ").next().unwrap_or(message).trim();
    let mut text = text.to_owned();
    if let Some(first) = text.get(..1) {
        let upper = first.to_uppercase();
        text.replace_range(..1, &upper);
    }
    if !text.ends_with('.') {
        text.push('.');
    }
    text
}
