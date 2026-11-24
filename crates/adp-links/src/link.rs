//! A link as FBL section 17.1 writes it: `$<document-id>@<document-name>[#<element-id>[.<name-prop>][=<name>]]`.

/// The characters that end a part, and so are escaped in it, besides `%` (FBL section 17.1).
const ENDS_AN_ID: &[char] = &['$', '@', '#', '.', '='];
const ENDS_A_FILE_NAME: &[char] = &['#'];
const ENDS_A_NAME: &[char] = &[];

/// A link to a document, or to an element of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    /// The target document's id.
    pub document_id: String,
    /// The target document's file name, with its extension and without its folder.
    pub document_name: String,
    /// The target element; none for a link to the document as a whole.
    pub element: Option<ElementPart>,
}

/// The part of a link after `#`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementPart {
    pub id: String,
    /// The attribute of the target element that names it; none for its type's name property.
    pub name_prop: Option<String>,
    /// That attribute's value as last seen; none until load-time validation fills it in.
    pub name: Option<String>,
}

impl Link {
    /// A link to a document as a whole.
    pub fn to_document(document_id: impl Into<String>, document_name: impl Into<String>) -> Link {
        Link {
            document_id: document_id.into(),
            document_name: document_name.into(),
            element: None,
        }
    }

    /// A link to an element, named by `name_prop` (none for the type's name property) with the cached `name`.
    pub fn to_element(
        document_id: impl Into<String>,
        document_name: impl Into<String>,
        element_id: impl Into<String>,
        name_prop: Option<String>,
        name: Option<String>,
    ) -> Link {
        Link {
            document_id: document_id.into(),
            document_name: document_name.into(),
            element: Some(ElementPart {
                id: element_id.into(),
                name_prop,
                name,
            }),
        }
    }

    /// Reads a link, or none when `text` is not one (`link.malformed`). The text is split at its separators first,
    /// and each part unescaped after.
    pub fn parse(text: &str) -> Option<Link> {
        let rest = text.strip_prefix('$')?;
        let (document_id, rest) = rest.split_once('@')?;
        let (document_name, element) = match rest.split_once('#') {
            Some((name, element)) => (name, Some(element)),
            None => (rest, None),
        };
        let document_id = part(document_id, id_char, false)?;
        let document_name = part(document_name, file_char, false)?;
        let element = match element {
            None => None,
            Some(element) => {
                let (before_name, name) = match element.split_once('=') {
                    Some((before, name)) => (before, Some(name)),
                    None => (element, None),
                };
                let (id, name_prop) = match before_name.split_once('.') {
                    Some((id, prop)) => (id, Some(prop)),
                    None => (before_name, None),
                };
                let name_prop = match name_prop {
                    Some(prop) => Some(part(prop, id_char, false)?),
                    None => None,
                };
                let name = match name {
                    Some(name) => Some(part(name, text_char, true)?),
                    None => None,
                };
                Some(ElementPart {
                    id: part(id, id_char, false)?,
                    name_prop,
                    name,
                })
            }
        };
        Some(Link {
            document_id,
            document_name,
            element,
        })
    }

    /// The element's id, or none for a link to a document.
    pub fn element_id(&self) -> Option<&str> {
        self.element.as_ref().map(|e| e.id.as_str())
    }

    /// The name the link caches, or none.
    pub fn cached_name(&self) -> Option<&str> {
        self.element.as_ref().and_then(|e| e.name.as_deref())
    }

    /// The link as it is written, each part escaped where a character would end it.
    pub fn text(&self) -> String {
        let mut text = format!(
            "${}@{}",
            escape(&self.document_id, ENDS_AN_ID),
            escape(&self.document_name, ENDS_A_FILE_NAME)
        );
        if let Some(element) = &self.element {
            text.push('#');
            text.push_str(&escape(&element.id, ENDS_AN_ID));
            if let Some(prop) = &element.name_prop {
                text.push('.');
                text.push_str(&escape(prop, ENDS_AN_ID));
            }
            if let Some(name) = &element.name {
                text.push('=');
                text.push_str(&escape(name, ENDS_A_NAME));
            }
        }
        text
    }
}

impl std::fmt::Display for Link {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text())
    }
}

fn is_control(c: char) -> bool {
    (c as u32) < 0x20 || c as u32 == 0x7f
}

fn id_char(c: char) -> bool {
    !is_control(c) && !matches!(c, '#' | '$' | '%' | '.' | '=' | '@')
}

fn file_char(c: char) -> bool {
    !is_control(c) && !matches!(c, '#' | '%')
}

fn text_char(c: char) -> bool {
    !is_control(c) && c != '%'
}

/// One part of a link unescaped, or none when it holds a character its part does not allow, or is empty where it
/// must not be.
fn part(raw: &str, allowed: fn(char) -> bool, may_be_empty: bool) -> Option<String> {
    if raw.is_empty() && !may_be_empty {
        return None;
    }
    let mut bytes = Vec::with_capacity(raw.len());
    let mut chars = raw.char_indices();
    while let Some((at, c)) = chars.next() {
        if c == '%' {
            let hex = raw.get(at + 1..at + 3)?;
            if !hex.chars().all(|h| h.is_ascii_hexdigit()) {
                return None;
            }
            bytes.push(u8::from_str_radix(hex, 16).ok()?);
            chars.next();
            chars.next();
        } else if allowed(c) {
            let mut buffer = [0u8; 4];
            bytes.extend_from_slice(c.encode_utf8(&mut buffer).as_bytes());
        } else {
            return None;
        }
    }
    String::from_utf8(bytes).ok()
}

fn escape(part: &str, reserved: &[char]) -> String {
    let mut text = String::with_capacity(part.len());
    for c in part.chars() {
        if c == '%' || reserved.contains(&c) {
            let mut buffer = [0u8; 4];
            for byte in c.encode_utf8(&mut buffer).as_bytes() {
                text.push_str(&format!("%{byte:02X}"));
            }
        } else {
            text.push(c);
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_four_forms_of_the_specification_read_and_write_back() {
        for text in [
            "$k3f9x2ma@provinces.yaml#nh.name=Noord-Holland",
            "$p0q7e2zd@release.tml#launch=Launch",
            "$m8c1u4wa@ideas.mm#ID_1723.text=v1.2 scope",
            "$k3f9x2ma@provinces.yaml",
        ] {
            let link = Link::parse(text).unwrap_or_else(|| panic!("{text} reads"));
            assert_eq!(text, link.text());
        }
        let link = Link::parse("$k3f9x2ma@provinces.yaml#nh.name=Noord-Holland").unwrap();
        assert_eq!(
            Link::to_element(
                "k3f9x2ma",
                "provinces.yaml",
                "nh",
                Some("name".into()),
                Some("Noord-Holland".into())
            ),
            link
        );
    }

    #[test]
    fn a_name_keeps_its_dots_and_equals_signs() {
        let link = Link::parse("$a@b.yaml#x=1.5 = 3/2").unwrap();
        assert_eq!(Some("1.5 = 3/2"), link.cached_name());
        assert_eq!(None, link.element.unwrap().name_prop);
    }

    #[test]
    fn what_would_end_a_part_is_escaped_and_read_back() {
        let link = Link::to_element("a.b", "a#b%.yaml", "x=y", Some("n@m".into()), Some("50% off".into()));
        let text = link.text();
        assert_eq!("$a%2Eb@a%23b%25.yaml#x%3Dy.n%40m=50%25 off", text);
        assert_eq!(Some(link), Link::parse(&text));
    }

    #[test]
    fn escapes_are_read_in_either_case_and_as_utf8() {
        let link = Link::parse("$a@caf%c3%A9.yaml").unwrap();
        assert_eq!("café.yaml", link.document_name);
    }

    #[test]
    fn what_is_not_a_link_is_none() {
        for text in [
            "",
            "k3f9x2ma@provinces.yaml#ut",
            "$@provinces.yaml",
            "$k3f9x2ma@",
            "$k3f9x2ma",
            "$k3f9x2ma@provinces.yaml#",
            "$k3f9x2ma@provinces.yaml#nh.",
            "$k3f%9x@a.yaml",
            "$k3$f@a.yaml",
            "not a link",
            "$a@b\u{7}.yaml",
        ] {
            assert_eq!(None, Link::parse(text), "{text:?}");
        }
    }

    #[test]
    fn an_empty_cached_name_is_a_name() {
        let link = Link::parse("$a@b.yaml#x=").unwrap();
        assert_eq!(Some(""), link.cached_name());
        assert_eq!("$a@b.yaml#x=", link.text());
    }
}
