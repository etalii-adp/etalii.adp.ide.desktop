//! libxml2's findings in the sentences, and at the places, the standalone host's validator (.NET's
//! `System.Xml.Schema`, through `SchemaValidation.cs`) gives them: a finding about an element sits on its name in the
//! start tag, one about a value on the value, one about a child that is not allowed on the child. A finding this
//! module has no sentence for keeps libxml2's own, which names the element it is about.

use std::ops::Range;

use adp_libxml2::Issue;
use roxmltree::{Document, Node};

use crate::content_model::{Departure, Item, Model};
use crate::{Finding, Severity};

const XS: &str = "http://www.w3.org/2001/XMLSchema";

/// The element libxml2's node path names (`/library/book[2]`, `/*/p:party/@x`), and the attribute it ends in.
fn element_at<'a, 'input>(
    tree: &'a Document<'input>,
    text: &str,
    path: &str,
) -> Option<(Node<'a, 'input>, Option<String>)> {
    let mut current = tree.root();
    let mut attribute = None;
    for step in path.split('/').filter(|s| !s.is_empty()) {
        if let Some(name) = step.strip_prefix('@') {
            attribute = Some(name.to_owned());
            break;
        }
        let (name, index) = match step.split_once('[') {
            Some((name, rest)) => (name, rest.trim_end_matches(']').parse::<usize>().ok()?),
            None => (step, 1),
        };
        if name.contains('(') {
            // text(), comment(): the element above is the one meant.
            break;
        }
        current = current
            .children()
            .filter(|c| c.is_element() && (name == "*" || written_name(text, *c) == name))
            .nth(index.checked_sub(1)?)?;
    }
    current.is_element().then_some((current, attribute))
}

/// An element's name as written in its start tag, with its prefix.
fn written_name<'t>(text: &'t str, element: Node) -> &'t str {
    &text[name_span(text, element)]
}

/// Where an element's name stands in its start tag.
fn name_span(text: &str, element: Node) -> Range<usize> {
    let start = element.range().start + 1;
    let length = text[start..]
        .find(|c: char| c.is_whitespace() || c == '/' || c == '>')
        .unwrap_or(text.len() - start);
    start..start + length
}

/// The element's text, without the white space around it; its name when it has none.
fn text_span(text: &str, element: Node) -> Range<usize> {
    let texts: Vec<Node> = element.children().filter(|c| c.is_text()).collect();
    if let (Some(first), Some(last)) = (texts.first(), texts.last()) {
        let (mut from, mut to) = (first.range().start, last.range().end);
        while from < to && text[from..].starts_with(char::is_whitespace) {
            from += text[from..].chars().next().map_or(1, char::len_utf8);
        }
        while to > from && text[..to].ends_with(char::is_whitespace) {
            to -= text[..to].chars().next_back().map_or(1, char::len_utf8);
        }
        if from < to {
            return from..to;
        }
    }
    name_span(text, element)
}

/// The first text of an element that is not only white space.
fn first_text<'a, 'input>(element: Node<'a, 'input>) -> Option<Node<'a, 'input>> {
    element
        .children()
        .find(|c| c.is_text() && !c.text().unwrap_or_default().trim().is_empty())
}

fn trimmed(text: &str, node: Node) -> Range<usize> {
    let range = node.range();
    let slice = &text[range.clone()];
    let start = range.start + (slice.len() - slice.trim_start().len());
    let end = range.end - (slice.len() - slice.trim_end().len());
    if start < end { start..end } else { range }
}

/// An attribute of `element` by the name libxml2 gives it (`format`, `{ns}local`, `p:local`).
fn attribute_of<'a, 'input>(element: Node<'a, 'input>, name: &str) -> Option<roxmltree::Attribute<'a, 'input>> {
    let (namespace, local) = expanded(name);
    let local = local.rsplit(':').next().unwrap_or(local);
    element
        .attributes()
        .find(|a| a.name() == local && (namespace.is_none() || a.namespace() == namespace))
}

/// `{ns}local` as its namespace and local name.
fn expanded(name: &str) -> (Option<&str>, &str) {
    match name.strip_prefix('{').and_then(|rest| rest.split_once('}')) {
        Some((namespace, local)) => (Some(namespace), local),
        None => (None, name),
    }
}

/// A name as .NET says it: `'local'`, and ` in namespace 'ns'` when it has one.
fn said(local: &str, namespace: Option<&str>) -> String {
    match namespace {
        Some(namespace) if !namespace.is_empty() => format!("'{local}' in namespace '{namespace}'"),
        _ => format!("'{local}'"),
    }
}

/// The names libxml2 lists after "Expected is ( … )" or "Expected is one of ( … )", as .NET lists them: grouped by
/// namespace, `'a, b' in namespace 'ns'`.
fn expected_list(rest: &str) -> Option<String> {
    let at = rest.find("Expected is")?;
    let open = rest[at..].find('(')? + at + 1;
    let close = rest[open..].rfind(')')? + open;
    let mut groups: Vec<(Option<&str>, Vec<&str>)> = Vec::new();
    for name in rest[open..close].split(',').map(str::trim).filter(|n| !n.is_empty()) {
        let (namespace, local) = expanded(name);
        match groups.last_mut() {
            Some((ns, names)) if *ns == namespace => names.push(local),
            _ => groups.push((namespace, vec![local])),
        }
    }
    let groups: Vec<String> = groups
        .iter()
        .map(|(namespace, names)| said(&names.join(", "), *namespace))
        .collect();
    (!groups.is_empty()).then(|| groups.join(" as well as "))
}

/// .NET's name for a built-in type's values, as its sentence "The string 'x' is not a valid Int32 value." says it.
fn value_kind(local: &str) -> String {
    match local {
        "int" => "Int32".into(),
        "long" => "Int64".into(),
        "short" => "Int16".into(),
        "byte" => "SByte".into(),
        "unsignedInt" => "UInt32".into(),
        "unsignedLong" => "UInt64".into(),
        "unsignedShort" => "UInt16".into(),
        "unsignedByte" => "Byte".into(),
        "float" => "Single".into(),
        "integer" | "positiveInteger" | "negativeInteger" | "nonNegativeInteger" | "nonPositiveInteger" => {
            "Integer".into()
        }
        other => {
            let mut chars = other.chars();
            chars
                .next()
                .map(|c| c.to_uppercase().chain(chars).collect())
                .unwrap_or_default()
        }
    }
}

/// A type libxml2 names (`xs:gYear`, `{ns}local`, `local`) as .NET names a datatype, and its local name.
fn datatype(name: &str) -> (String, String, bool) {
    if let Some(local) = name.strip_prefix("xs:") {
        return (format!("{XS}:{local}"), local.to_owned(), true);
    }
    match expanded(name) {
        (Some(namespace), local) => (format!("{namespace}:{local}"), local.to_owned(), namespace == XS),
        (None, local) => (local.to_owned(), local.to_owned(), false),
    }
}

/// .NET's sentence for a facet that failed.
fn facet_failed(facet: &str) -> String {
    match facet {
        "maxLength" => "The actual length is greater than the MaxLength value.".into(),
        "minLength" => "The actual length is less than the MinLength value.".into(),
        "length" => "The actual length is not equal to the specified length.".into(),
        other => {
            let mut chars = other.chars();
            let name: String = chars
                .next()
                .map(|c| c.to_uppercase().chain(chars).collect())
                .unwrap_or_default();
            format!("The {name} constraint failed.")
        }
    }
}

/// libxml2's "Element 'x', attribute 'y': rest" as its parts.
fn parts(message: &str) -> (Option<&str>, Option<&str>, &str) {
    let Some(rest) = message.strip_prefix("Element '") else {
        return (None, None, message);
    };
    let Some(end) = rest.find('\'') else {
        return (None, None, message);
    };
    let element = &rest[..end];
    let mut rest = &rest[end + 1..];
    let mut attribute = None;
    if let Some(after) = rest.strip_prefix(", attribute '")
        && let Some(end) = after.find('\'')
    {
        attribute = Some(&after[..end]);
        rest = &after[end + 1..];
    }
    (Some(element), attribute, rest.strip_prefix(": ").unwrap_or(rest))
}

/// A finding of an XSD's validator, in the standalone host's words and place.
pub fn xsd_finding(issue: &Issue, text: &str, tree: &Document) -> Finding {
    let severity = if issue.error {
        Severity::Error
    } else {
        Severity::Warning
    };
    let place = issue.node.as_deref().and_then(|path| element_at(tree, text, path));
    let (_, attribute, rest) = parts(&issue.message);
    let Some((element, _)) = place else {
        return Finding {
            span: 0..0,
            severity,
            message: issue.message.clone(),
        };
    };
    let name = name_span(text, element);
    let tag = element.tag_name();
    let element_said = said(tag.name(), tag.namespace());
    let found = |span: Range<usize>, message: String| Finding {
        span,
        severity,
        message,
    };
    match issue.code {
        // cvc-complex-type.4: a required attribute missing.
        1868 => {
            let missing = issue.str1.as_deref().map_or("", |a| expanded(a).1);
            return found(name, format!("The required attribute '{missing}' is missing."));
        }
        // cvc-complex-type.3.2: an attribute the type does not declare.
        1866 | 1867 => {
            let local = issue.str1.as_deref().or(attribute).map_or("", |a| expanded(a).1);
            let span = attribute_of(element, issue.str1.as_deref().or(attribute).unwrap_or_default())
                .map_or(name, |a| a.range_qname());
            return found(span, format!("The '{local}' attribute is not declared."));
        }
        // The content of an element: a child not expected, or children missing.
        1871 => {
            let list = expected_list(rest)
                .map(|l| format!(" List of possible elements expected: {l}."))
                .unwrap_or_default();
            if rest.starts_with("This element is not expected.") {
                let parent = element.parent_element().map_or_else(String::new, |p| {
                    let tag = p.tag_name();
                    said(tag.name(), tag.namespace())
                });
                return found(
                    name,
                    format!("The element {parent} has invalid child element {element_said}.{list}"),
                );
            }
            if rest.starts_with("Missing child element(s).") {
                return found(
                    name,
                    format!("The element {element_said} has incomplete content.{list}"),
                );
            }
        }
        // Text in an element whose content is empty, or only elements.
        1841 => {
            let span = first_text(element).map_or(name, |t| trimmed(text, t));
            return found(span, "The element cannot contain text. Content model is empty.".into());
        }
        1843 => {
            let span = first_text(element).map_or(name, |t| trimmed(text, t));
            return found(span, format!("The element {element_said} cannot contain text."));
        }
        // cvc-elt.1: the root is not declared.
        1845 => return found(name, format!("The {} element is not declared.", said(tag.name(), None))),
        _ => {}
    }
    // A value that does not fit its type, or a facet of it.
    let facet = rest
        .strip_prefix("[facet '")
        .and_then(|r| r.split_once('\''))
        .map(|(f, _)| f);
    let atomic = rest.find(" is not a valid value of the ").and_then(|at| {
        let after = &rest[at..];
        let open = after.find("type '")? + "type '".len();
        let close = after[open..].find('\'')? + open;
        Some(&after[open..close])
    });
    if facet.is_some() || atomic.is_some() {
        let (subject, span) = match attribute.and_then(|a| attribute_of(element, a)) {
            Some(a) => (format!("The '{}' attribute", a.name()), a.range_value()),
            None => (format!("The '{}' element", tag.name()), text_span(text, element)),
        };
        let value = match (facet, &issue.str1) {
            (Some("length" | "minLength" | "maxLength"), _) | (_, None) => text[span.clone()].to_owned(),
            (_, Some(value)) => value.clone(),
        };
        let type_name = if facet.is_some() { issue.str3.as_deref() } else { atomic };
        let (datatype, local, built_in) = type_name.map(datatype).unwrap_or_default();
        let why = match facet {
            Some(facet) => facet_failed(facet),
            None if built_in || !local.is_empty() => {
                format!("The string '{value}' is not a valid {} value.", value_kind(&local))
            }
            None => format!("The value '{value}' is not valid."),
        };
        let according = if datatype.is_empty() {
            String::new()
        } else {
            format!(" according to its datatype '{datatype}'")
        };
        return found(
            span,
            format!("{subject} is invalid - The value '{value}' is invalid{according} - {why}"),
        );
    }
    found(name, issue.message.clone())
}

/// A finding of a DTD's validator, in the standalone host's words and place; `declarations` holds the internal subset
/// and the DTDs read, for an element's content model.
pub fn dtd_finding(issue: &Issue, text: &str, tree: &Document, declarations: &str) -> Finding {
    let severity = if issue.error {
        Severity::Error
    } else {
        Severity::Warning
    };
    let found = |span: Range<usize>, message: String| Finding {
        span,
        severity,
        message,
    };
    let Some((element, _)) = issue.node.as_deref().and_then(|path| element_at(tree, text, path)) else {
        return found(0..0, issue.message.clone());
    };
    let name = name_span(text, element);
    let element_name = written_name(text, element).to_owned();
    let str1 = issue.str1.clone().unwrap_or_default();
    let str2 = issue.str2.clone().unwrap_or_default();
    match issue.code {
        // XML_DTD_UNKNOWN_ATTRIBUTE
        533 => {
            let span = element
                .attributes()
                .find(|a| text[a.range_qname()] == str1)
                .map_or(name, |a| a.range_qname());
            found(span, format!("The '{str1}' attribute is not declared."))
        }
        // XML_DTD_MISSING_ATTRIBUTE
        518 => found(name, format!("The required attribute '{str2}' is missing.")),
        // XML_DTD_UNKNOWN_ELEM
        534 => found(name, format!("The '{str1}' element is not declared.")),
        // XML_DTD_NOT_EMPTY
        528 => match element.children().find(Node::is_element) {
            Some(child) => found(
                name_span(text, child),
                format!(
                    "The element '{element_name}' cannot contain child element '{}' because the parent element's content model is empty.",
                    written_name(text, child)
                ),
            ),
            None => found(
                first_text(element).map_or(name, |t| trimmed(text, t)),
                "The element cannot contain text. Content model is empty.".into(),
            ),
        },
        // XML_DTD_CONTENT_MODEL, with the model; XML_DTD_NOT_PCDATA's kin, a child not in a mixed list, without.
        504 | 515 => {
            let model = if issue.code == 504 {
                Some(str2.clone())
            } else {
                declared_model(declarations, &element_name)
            };
            content(issue, text, element, model.as_deref().and_then(Model::parse))
        }
        _ => found(name, issue.message.clone()),
    }
}

/// An element's content against its model, said where it first leaves it.
fn content(issue: &Issue, text: &str, element: Node, model: Option<Model>) -> Finding {
    let severity = if issue.error {
        Severity::Error
    } else {
        Severity::Warning
    };
    let name = name_span(text, element);
    let element_name = written_name(text, element);
    let nodes: Vec<Node> = element
        .children()
        .filter(|c| c.is_element() || (c.is_text() && !c.text().unwrap_or_default().trim().is_empty()))
        .collect();
    let items: Vec<Item> = nodes
        .iter()
        .map(|n| {
            if n.is_element() {
                Item::Element(written_name(text, *n).to_owned())
            } else {
                Item::Text
            }
        })
        .collect();
    let list = |names: &[String]| {
        if names.is_empty() {
            String::new()
        } else {
            format!(" List of possible elements expected: '{}'.", names.join(" "))
        }
    };
    let (span, message) = match model.and_then(|m| m.departure(&items)) {
        Some(Departure::Child(index, expected)) => (
            name_span(text, nodes[index]),
            format!(
                "The element '{element_name}' has invalid child element '{}'.{}",
                written_name(text, nodes[index]),
                list(&expected)
            ),
        ),
        Some(Departure::Text(index, expected)) => (
            trimmed(text, nodes[index]),
            format!("The element '{element_name}' cannot contain text.{}", list(&expected)),
        ),
        Some(Departure::Incomplete(expected)) => (
            name,
            format!(
                "The element '{element_name}' has incomplete content.{}",
                list(&expected)
            ),
        ),
        None => (name, issue.message.clone()),
    };
    Finding {
        span,
        severity,
        message,
    }
}

/// The content model `<!ELEMENT name model>` declares.
fn declared_model(declarations: &str, name: &str) -> Option<String> {
    declarations.match_indices("<!ELEMENT").find_map(|(at, keyword)| {
        let rest = declarations[at + keyword.len()..].trim_start();
        let rest = rest.strip_prefix(name)?;
        if !rest.starts_with(char::is_whitespace) {
            return None;
        }
        let end = rest.find('>')?;
        Some(rest[..end].trim().to_owned())
    })
}

/// Where the element a schema's problem is about stands in its text, by its name.
pub fn node_name_span(issue: &Issue, text: &str) -> Range<usize> {
    let options = roxmltree::ParsingOptions {
        allow_dtd: true,
        ..Default::default()
    };
    let Ok(tree) = Document::parse_with_options(text, options) else {
        return 0..0;
    };
    issue
        .node
        .as_deref()
        .and_then(|path| element_at(&tree, text, path))
        .map_or(0..0, |(element, _)| name_span(text, element))
}

/// The standalone host's sentence for a schema that does not compile: `'file' does not compile: why (line n)`.
pub fn does_not_compile(issue: &Issue) -> String {
    let file = issue
        .file
        .as_ref()
        .and_then(|f| f.last())
        .map_or_else(|| "The schema".to_owned(), |f| format!("'{f}'"));
    let line = if issue.line > 0 {
        format!(" (line {})", issue.line)
    } else {
        String::new()
    };
    format!("{file} does not compile: {}{line}", compile_sentence(&issue.message))
}

/// libxml2's sentence for a schema problem, as .NET says the ones it has words for.
fn compile_sentence(message: &str) -> String {
    let rest = message.split_once("': ").map_or(message, |(_, rest)| rest);
    let unresolved = rest.strip_prefix("The QName value '").and_then(|r| {
        let (name, after) = r.split_once('\'')?;
        let kind = after.strip_prefix(" does not resolve to a(n) ")?.trim_end_matches('.');
        Some((name, kind))
    });
    match unresolved {
        Some((name, "simple type definition")) => format!("Type '{name}' is not declared, or is not a simple type."),
        Some((name, "type definition")) => format!("Type '{name}' is not declared."),
        Some((name, "element declaration")) => format!("The '{name}' element is not declared."),
        Some((name, "attribute declaration")) => format!("The '{name}' attribute is not declared."),
        Some((name, "attribute group definition")) => format!("Reference to undeclared attribute group '{name}'."),
        Some((name, "model group definition")) => format!("Reference to undeclared model group '{name}'."),
        _ => rest.to_owned(),
    }
}
