//! The little of `System.Xml.Linq` the readers used (etalii.adp spec 030, task T082): a document loaded from a file or
//! parsed from text, its elements matched by local name whatever their namespace, and an element's `Value`, the text
//! of everything inside it.

use std::fs;
use std::path::Path;

use roxmltree::{Document, Node, ParsingOptions};

/// Why a file could not be loaded as XML, in a sentence.
pub fn load(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    let text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
    parse(&text).map(|_| ()).map_err(|e| e.to_string())?;
    Ok(text)
}

/// `text` as an XML document, a byte order mark ignored and a document type allowed, as `XDocument` reads it.
pub fn parse(text: &str) -> Result<Document<'_>, roxmltree::Error> {
    let options = ParsingOptions {
        allow_dtd: true,
        ..ParsingOptions::default()
    };
    Document::parse_with_options(text.trim_start_matches('\u{feff}'), options)
}

/// Every element of `document` with the local name `name`, in document order (`Descendants().Where(LocalName)`).
pub fn elements<'a, 'input>(document: &'a Document<'input>, name: &'a str) -> impl Iterator<Item = Node<'a, 'input>> {
    document
        .descendants()
        .filter(move |n| n.is_element() && n.tag_name().name() == name)
}

/// An element's `Value`: the text of every text node inside it, joined.
pub fn value(node: Node<'_, '_>) -> String {
    node.descendants()
        .filter(Node::is_text)
        .filter_map(|n| n.text())
        .collect()
}
