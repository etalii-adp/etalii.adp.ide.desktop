//! The XML editor's analysis (etalii.adp spec 030, FR-020, task T032), after the standalone host's XML editor at
//! `8b6ccfc` (xml-editor: `WellFormedness.cs`, `SchemaAssociation.cs`, `SchemaValidation.cs`, `XmlAnalyzer.cs`).
//!
//! - **Well-formedness**: the text is read to its end or its first error, which becomes one finding where the reader
//!   stopped; the status then says the rest was not checked. A DTD or external entity a reader would have to follow
//!   but may not (outside the project, on the web, missing) stops the reading too, said as information.
//! - **The declared encoding**: a declaration of an encoding other than the one the file is stored in is a warning,
//!   unless every character is ASCII and so reads the same either way.
//! - **The schemas a document names**, from its prolog and its root's start tag only: `xsi:noNamespaceSchemaLocation`,
//!   the pairs of `xsi:schemaLocation`, and the `DOCTYPE`'s system identifier. Each is resolved by the project-only
//!   rules the JSON editor uses; one that cannot be followed is an information finding on its reference. An
//!   `xml-model` instruction is named and not supported; a root `xs:schema` makes the document a schema, compiled.
//! - **Validity**: the document is checked against every XSD it names together, else against its DTD, by libxml2
//!   (`adp-libxml2`), and each of libxml2's findings is said in the sentence and at the place the standalone host's
//!   validator (.NET's `System.Xml.Schema`) gives it: on the element's name, on the value that does not fit, or on
//!   the child that is not allowed. XSD 1.1, which neither checks, is said as not supported.
//!
//! Offsets are bytes into the text.

mod content_model;
mod sentences;

use std::ops::Range;

pub use adp_editor_json::ReadFile;
use adp_editor_json::{SchemaTarget, resolve_schema_reference};
use adp_libxml2::{Domain, XsdCheck};

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
    /// The files of the project the answer was read from, or would be once they exist: a change to one of them
    /// changes the answer.
    pub depends_on: Vec<Vec<String>>,
}

/// The sentence the standalone host's status line gives a document that is not well-formed.
pub const NOT_CHECKED: &str = "Not well-formed: the rest was not checked.";
const NO_SCHEMA: &str = "No schema";
const DTD_CHECKED: &str = "DTD: checked, no suggestions.";

/// Analyses an XML text; `document` is its path in the project, its file name last; `stored` is how it is stored
/// ("UTF-8" or "UTF-8 with BOM"); `read` reads the project's files, for the schemas.
pub fn analyse(text: &str, document: &[String], stored: &str, read: ReadFile) -> Analysis {
    let mut analysis = Analysis {
        findings: Vec::new(),
        status: String::new(),
        schemas: Vec::new(),
        depends_on: Vec::new(),
    };
    if let Some(finding) = declared_encoding(text, stored) {
        analysis.findings.push(finding);
    }
    let names_a_schema =
        text.contains("schemaLocation") || text.contains("noNamespaceSchemaLocation") || text.contains("<!DOCTYPE");
    let stopped = |mut analysis: Analysis| {
        analysis.status = if names_a_schema {
            format!("{NOT_CHECKED} Schema not checked.")
        } else {
            NOT_CHECKED.into()
        };
        analysis
    };
    if let Some(finding) = unreadable_reference(text, document, &read) {
        analysis.findings.push(finding);
        return stopped(analysis);
    }
    let options = roxmltree::ParsingOptions {
        allow_dtd: true,
        ..Default::default()
    };
    let tree = match roxmltree::Document::parse_with_options(text, options) {
        Ok(tree) => tree,
        Err(error) => {
            let position = error.pos();
            let at = offset_of(text, position.row as usize, position.col as usize);
            let end = text[at..]
                .chars()
                .next()
                .map_or(at, |c| at + c.len_utf8())
                .max(at + 1)
                .min(text.len().max(at + 1));
            analysis.findings.push(Finding {
                span: at..end,
                severity: Severity::Error,
                message: sentence(&error.to_string()),
            });
            return stopped(analysis);
        }
    };
    analysis.schemas = named_schemas(text, &tree);
    analysis.findings.extend(xml_models(&tree));
    let files = |path: &[String]| read(path);
    let root = tree.root_element();
    if root.tag_name().namespace() == Some(XS) && root.tag_name().name() == "schema" {
        schema_as_document(text, document, &files, &mut analysis);
        return analysis;
    }
    let xsd: Vec<Named> = analysis
        .schemas
        .iter()
        .filter(|n| n.kind == SchemaKind::Xsd)
        .cloned()
        .collect();
    if !xsd.is_empty() {
        against_xsd(text, &tree, document, &xsd, &files, &mut analysis);
    } else if text.contains("<!DOCTYPE") {
        against_dtd(text, &tree, document, &files, &mut analysis);
    } else {
        analysis.status = NO_SCHEMA.into();
    }
    analysis
}

/// Checks a document against the XSDs it names, the ones that can be followed together.
fn against_xsd(
    text: &str,
    tree: &roxmltree::Document,
    document: &[String],
    named: &[Named],
    files: adp_libxml2::Files,
    analysis: &mut Analysis,
) {
    let mut roots: Vec<(&Named, Vec<String>, String)> = Vec::new();
    for reference in named {
        match resolve_schema_reference(&reference.location, document) {
            SchemaTarget::Project(path) => match files(&path) {
                Some(schema) => roots.push((reference, path, schema)),
                None => {
                    analysis.depends_on.push(path);
                    analysis.findings.push(info(
                        reference,
                        format!(
                            "'{}' is not in the project, so no schema is checked from it.",
                            reference.location
                        ),
                    ));
                }
            },
            SchemaTarget::None(reason) => analysis
                .findings
                .push(info(reference, format!("'{}' {reason}.", reference.location))),
            SchemaTarget::Bundled(_) => {}
        }
    }
    if roots.is_empty() {
        analysis.status = NO_SCHEMA.into();
        return;
    }
    if let Some((reference, _, _)) = roots.iter().find(|(_, _, schema)| declares_version_1_1(schema)) {
        analysis.findings.push(info(
            reference,
            format!(
                "'{}' declares XSD 1.1, which this editor does not check: it checks XSD 1.0.",
                reference.location
            ),
        ));
        analysis.status = "Schema not supported: XSD 1.1.".into();
        return;
    }
    let mut paths: Vec<(String, Vec<String>)> = Vec::new();
    for (reference, path, _) in &roots {
        if !paths.iter().any(|(_, p)| p == path) {
            paths.push((reference.namespace.clone(), path.clone()));
        }
    }
    match adp_libxml2::check_against_xsd(text, document, &paths, files) {
        XsdCheck::DoesNotCompile(outcome) => {
            depend(analysis, &outcome.read);
            let problem = outcome.issues.iter().find(|i| i.error).or(outcome.issues.first());
            let (file, message) = match problem {
                Some(issue) => (
                    issue.file.as_ref().and_then(|f| f.last().cloned()),
                    sentences::does_not_compile(issue),
                ),
                None => (None, "The schema does not compile.".into()),
            };
            let file = file.unwrap_or_else(|| roots[0].1.last().cloned().unwrap_or_default());
            analysis.findings.push(info(roots[0].0, message));
            analysis.status = format!("Schema: {file} does not compile.");
        }
        XsdCheck::Checked(outcome) => {
            depend(analysis, &outcome.read);
            let mut found: Vec<Finding> = Vec::new();
            for issue in outcome.issues.iter().filter(|i| i.domain == Domain::SchemaValidity) {
                let finding = sentences::xsd_finding(issue, text, tree);
                // libxml2 may say one value twice, by its facet and by its type; the standalone host says it once.
                if !found
                    .iter()
                    .any(|f| f.span == finding.span && f.span.start < f.span.end)
                {
                    found.push(finding);
                }
            }
            found.sort_by_key(|f| f.span.start);
            analysis.findings.extend(found);
            let mut names: Vec<String> = Vec::new();
            for (reference, path, _) in &roots {
                let name = if reference.namespace.is_empty() {
                    path.last().cloned().unwrap_or_default()
                } else {
                    reference.namespace.clone()
                };
                if !names.contains(&name) {
                    names.push(name);
                }
            }
            analysis.status = format!("Schema: {}", names.join(", "));
        }
    }
}

/// Checks a document against its DTD, external and internal.
fn against_dtd(
    text: &str,
    tree: &roxmltree::Document,
    document: &[String],
    files: adp_libxml2::Files,
    analysis: &mut Analysis,
) {
    let outcome = adp_libxml2::check_against_dtd(text, document, files);
    depend(analysis, &outcome.read);
    let doctype = doctype_system(text).map_or_else(
        || {
            let at = text.find("<!DOCTYPE").map_or(0, |i| i + 2);
            at..at + "DOCTYPE".len()
        },
        |(_, span)| span,
    );
    if let Some(refusal) = outcome.refused.first() {
        // Asked for from inside the DTD: what the standalone host's reader says where it stops.
        analysis.findings.push(Finding {
            span: doctype,
            severity: Severity::Info,
            message: format!(
                "'{}' {}, so the document could not be read past it.",
                refusal.address, refusal.reason
            ),
        });
        analysis.status = format!("{NOT_CHECKED} Schema not checked.");
        return;
    }
    // The models the DTD declares, for the findings libxml2 says without the model.
    let mut declarations = text.to_owned();
    for path in &outcome.read {
        if let Some(dtd) = files(path) {
            declarations.push_str(&dtd);
        }
    }
    let mut found: Vec<Finding> = Vec::new();
    for issue in &outcome.issues {
        let finding = match issue.domain {
            Domain::Dtd => sentences::dtd_finding(issue, text, tree, &declarations),
            Domain::Parser if issue.error => Finding {
                span: if issue.file.as_deref() == Some(document) && issue.line > 0 {
                    let at = offset_of(text, issue.line as usize, issue.column.max(1) as usize);
                    at..(at + 1).min(text.len())
                } else {
                    doctype.clone()
                },
                severity: Severity::Error,
                message: sentence(&issue.message),
            },
            _ => continue,
        };
        found.push(finding);
    }
    found.sort_by_key(|f| f.span.start);
    analysis.findings.extend(found);
    analysis.status = DTD_CHECKED.into();
}

/// A document that is itself a schema: compiled from its text as typed.
fn schema_as_document(text: &str, document: &[String], files: adp_libxml2::Files, analysis: &mut Analysis) {
    if declares_version_1_1(text) {
        analysis.status = "XSD 1.1: not checked.".into();
        return;
    }
    match adp_libxml2::compile_xsd(text, document, files) {
        None => analysis.status = "XSD: compiles.".into(),
        Some(outcome) => {
            depend(analysis, &outcome.read);
            if let Some(issue) = outcome.issues.iter().find(|i| i.error).or(outcome.issues.first()) {
                let span = if issue.file.as_deref() == Some(document) {
                    sentences::node_name_span(issue, text)
                } else {
                    0..0
                };
                analysis.findings.push(Finding {
                    span,
                    severity: Severity::Error,
                    message: sentences::does_not_compile(issue),
                });
            }
            analysis.status = "XSD: does not compile.".into();
        }
    }
}

fn depend(analysis: &mut Analysis, read: &[Vec<String>]) {
    for path in read {
        if !analysis.depends_on.contains(path) {
            analysis.depends_on.push(path.clone());
        }
    }
}

fn info(reference: &Named, message: String) -> Finding {
    Finding {
        span: reference.span.clone(),
        severity: Severity::Info,
        message,
    }
}

/// An XSD that says it needs version 1.1, through the versioning namespace's `minVersion`.
fn declares_version_1_1(schema: &str) -> bool {
    let Some(at) = schema.find("http://www.w3.org/2007/XMLSchema-versioning") else {
        return false;
    };
    let rest = &schema[at..];
    rest.match_indices(":minVersion").any(|(i, m)| {
        let value = rest[i + m.len()..].trim_start();
        value
            .strip_prefix('=')
            .map(str::trim_start)
            .is_some_and(|v| v.starts_with("\"1.1\"") || v.starts_with("'1.1'"))
    })
}

/// The first DTD or external entity a reader would follow and may not, said where it is named. The standalone host's
/// reader stops there: what the DTD declares cannot be known without it.
fn unreadable_reference(text: &str, document: &[String], read: &ReadFile) -> Option<Finding> {
    let mut references: Vec<(String, Range<usize>)> = Vec::new();
    if let Some(system) = doctype_system(text) {
        references.push(system);
    }
    references.extend(external_entities(text));
    for (reference, span) in references {
        let reason = match resolve_schema_reference(&reference, document) {
            SchemaTarget::Project(path) => match read(&path) {
                Some(_) => continue,
                None => "is not in the project".to_owned(),
            },
            SchemaTarget::None(reason) => reason,
            SchemaTarget::Bundled(_) => continue,
        };
        return Some(Finding {
            span,
            severity: Severity::Info,
            message: format!("'{reference}' {reason}, so the document could not be read past it."),
        });
    }
    None
}

/// The `DOCTYPE`'s system identifier and where it stands.
fn doctype_system(text: &str) -> Option<(String, Range<usize>)> {
    let start = text.find("<!DOCTYPE")?;
    let declaration = &text[start..text[start..].find(['[', '>']).map_or(text.len(), |e| start + e)];
    let system = declaration.find("SYSTEM").or_else(|| declaration.find("PUBLIC"))?;
    let (from, to) = literal(&declaration[system..])?;
    Some((
        text[start + system + from..start + system + to].to_owned(),
        start + system + from..start + system + to,
    ))
}

/// Where the system literal of `SYSTEM "x"` or `PUBLIC "id" "x"` stands in `declaration`, which starts at the keyword.
fn literal(declaration: &str) -> Option<(usize, usize)> {
    let quotes: Vec<usize> = declaration
        .char_indices()
        .filter(|(_, c)| *c == '"' || *c == '\'')
        .map(|(i, _)| i)
        .collect();
    let pair = if declaration.starts_with("PUBLIC") {
        quotes.get(2..4)
    } else {
        quotes.get(0..2)
    };
    let [open, close] = pair? else {
        return None;
    };
    Some((open + 1, *close))
}

/// The system identifiers of the internal subset's external entities that the document uses.
fn external_entities(text: &str) -> Vec<(String, Range<usize>)> {
    let Some(start) = text.find("<!DOCTYPE") else {
        return Vec::new();
    };
    let head = &text[start..];
    let Some(open) = head
        .find(['[', '>'])
        .filter(|i| head[*i..].starts_with('['))
        .map(|i| start + i)
    else {
        return Vec::new();
    };
    let close = text[open..].find("]>").map_or(text.len(), |i| open + i);
    let subset = &text[open..close];
    let mut found = Vec::new();
    for (at, keyword) in subset.match_indices("<!ENTITY") {
        let body = at + keyword.len();
        let declaration = &subset[body..];
        let declaration = &declaration[..declaration.find('>').unwrap_or(declaration.len())];
        let mut words = declaration.split_whitespace();
        let mut name = words.next().unwrap_or_default();
        let parameter = name == "%";
        if parameter {
            name = words.next().unwrap_or_default();
        }
        let used = if parameter {
            subset.contains(&format!("%{name};"))
        } else {
            text[close..].contains(&format!("&{name};"))
        };
        let Some(system) = declaration.find("SYSTEM").or_else(|| declaration.find("PUBLIC")) else {
            continue;
        };
        if let (true, Some((from, to))) = (used, literal(&declaration[system..])) {
            let base = open + body + system;
            found.push((text[base + from..base + to].to_owned(), base + from..base + to));
        }
    }
    found
}

/// The `xml-model` instructions, each said once as a schema language this editor does not check.
fn xml_models(tree: &roxmltree::Document) -> Vec<Finding> {
    let mut found = Vec::new();
    for node in tree.root().children().filter(|n| n.is_pi()) {
        let Some(instruction) = node.pi() else {
            continue;
        };
        if instruction.target != "xml-model" {
            continue;
        }
        let value = instruction.value.unwrap_or_default();
        let namespace = value
            .find("schematypens")
            .and_then(|at| {
                let rest = value[at + "schematypens".len()..]
                    .trim_start()
                    .strip_prefix('=')?
                    .trim_start();
                let quote = rest.chars().next().filter(|c| *c == '"' || *c == '\'')?;
                rest[1..].split(quote).next()
            })
            .unwrap_or_default();
        let language = match namespace {
            "http://relaxng.org/ns/structure/1.0" => "RELAX NG",
            "http://purl.oclc.org/dsdl/schematron" | "http://www.ascc.net/xml/schematron" => "Schematron",
            XS => "XSD named by xml-model",
            "" => "a schema",
            other => other,
        };
        let start = node.range().start + 2;
        found.push(Finding {
            span: start..start + "xml-model".len(),
            severity: Severity::Info,
            message: format!(
                "An xml-model instruction names {language}, which this editor does not check: it checks XSD 1.0 and DTD named by the document."
            ),
        });
    }
    found
}

/// The schemas the prolog and the root's start tag name.
fn named_schemas(text: &str, tree: &roxmltree::Document) -> Vec<Named> {
    let mut named = Vec::new();
    if let Some((location, span)) = doctype_system(text) {
        named.push(Named {
            kind: SchemaKind::Dtd,
            namespace: String::new(),
            location,
            span,
        });
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
                location: attribute.value().trim().to_owned(),
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

/// A message without its trailing position, which the underline already shows, as a sentence.
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
