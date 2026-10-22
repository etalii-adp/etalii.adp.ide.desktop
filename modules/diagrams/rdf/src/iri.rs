//! Relative IRIs resolved against a base (etalii.adp spec 030, task T083), as the standalone host's parser and writer
//! did with .NET's `Uri.TryCreate(Uri, string, out Uri)` and `AbsoluteUri` at `8b6ccfc`: RFC 3986's reference
//! resolution, with the base's scheme and host lowercased as `Uri` writes them and a bare authority given its `/`.

/// Whether `iri` is absolute: a letter, then letters, digits, `+`, `-` or `.`, up to a `:` (`RdfParser.IsAbsolute`).
pub fn is_absolute(iri: &str) -> bool {
    let mut chars = iri.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() => {}
        _ => return false,
    }
    for character in chars {
        if character == ':' {
            return true;
        }
        if !character.is_ascii_alphanumeric() && !matches!(character, '+' | '-' | '.') {
            return false;
        }
    }
    false
}

struct Parts<'a> {
    scheme: &'a str,
    authority: Option<&'a str>,
    path: &'a str,
    query: Option<&'a str>,
    fragment: Option<&'a str>,
}

fn split(reference: &str) -> Parts<'_> {
    let (rest, fragment) = match reference.find('#') {
        Some(at) => (&reference[..at], Some(&reference[at + 1..])),
        None => (reference, None),
    };
    let (rest, query) = match rest.find('?') {
        Some(at) => (&rest[..at], Some(&rest[at + 1..])),
        None => (rest, None),
    };
    let (scheme, rest) = match rest.find(':') {
        Some(at) if is_absolute(&rest[..=at]) => (&rest[..at], &rest[at + 1..]),
        _ => ("", rest),
    };
    let (authority, path) = match rest.strip_prefix("//") {
        Some(after) => {
            let end = after.find('/').unwrap_or(after.len());
            (Some(&after[..end]), &after[end..])
        }
        None => (None, rest),
    };
    Parts {
        scheme,
        authority,
        path,
        query,
        fragment,
    }
}

/// RFC 3986, 5.2.4.
fn remove_dot_segments(path: &str) -> String {
    let mut input = path.to_owned();
    let mut output = String::new();
    while !input.is_empty() {
        if let Some(rest) = input.strip_prefix("../") {
            input = rest.to_owned();
        } else if let Some(rest) = input.strip_prefix("./") {
            input = rest.to_owned();
        } else if input.starts_with("/./") {
            input = input[2..].to_owned();
        } else if input == "/." {
            input = "/".to_owned();
        } else if input.starts_with("/../") || input == "/.." {
            input = if input == "/.." {
                "/".to_owned()
            } else {
                input[3..].to_owned()
            };
            match output.rfind('/') {
                Some(at) => output.truncate(at),
                None => output.clear(),
            }
        } else if input == "." || input == ".." {
            input.clear();
        } else {
            let start = usize::from(input.starts_with('/'));
            let end = input[start..].find('/').map_or(input.len(), |at| at + start);
            output.push_str(&input[..end]);
            input = input[end..].to_owned();
        }
    }
    output
}

fn merge(base: &Parts<'_>, path: &str) -> String {
    if base.authority.is_some() && base.path.is_empty() {
        return format!("/{path}");
    }
    match base.path.rfind('/') {
        Some(at) => format!("{}{path}", &base.path[..=at]),
        None => path.to_owned(),
    }
}

fn compose(scheme: &str, authority: Option<&str>, path: &str, query: Option<&str>, fragment: Option<&str>) -> String {
    let mut out = String::new();
    out.push_str(&scheme.to_ascii_lowercase());
    out.push(':');
    if let Some(authority) = authority {
        out.push_str("//");
        out.push_str(&authority.to_ascii_lowercase());
        if path.is_empty() {
            out.push('/');
        }
    }
    out.push_str(path);
    if let Some(query) = query {
        out.push('?');
        out.push_str(query);
    }
    if let Some(fragment) = fragment {
        out.push('#');
        out.push_str(fragment);
    }
    out
}

/// `reference` resolved against the absolute `base`, or none when the base is not absolute.
pub fn resolve(base: &str, reference: &str) -> Option<String> {
    if !is_absolute(base) {
        return None;
    }
    let base_parts = split(base);
    let reference_parts = split(reference);
    if !reference_parts.scheme.is_empty() {
        return Some(compose(
            reference_parts.scheme,
            reference_parts.authority,
            &remove_dot_segments(reference_parts.path),
            reference_parts.query,
            reference_parts.fragment,
        ));
    }
    if reference_parts.authority.is_some() {
        return Some(compose(
            base_parts.scheme,
            reference_parts.authority,
            &remove_dot_segments(reference_parts.path),
            reference_parts.query,
            reference_parts.fragment,
        ));
    }
    let (path, query) = if reference_parts.path.is_empty() {
        (base_parts.path.to_owned(), reference_parts.query.or(base_parts.query))
    } else if reference_parts.path.starts_with('/') {
        (remove_dot_segments(reference_parts.path), reference_parts.query)
    } else {
        (
            remove_dot_segments(&merge(&base_parts, reference_parts.path)),
            reference_parts.query,
        )
    };
    Some(compose(
        base_parts.scheme,
        base_parts.authority,
        &path,
        query,
        reference_parts.fragment,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn references_resolve_as_rfc_3986_says() {
        let base = "http://example.org/base/";
        assert_eq!(
            resolve(base, "relative").as_deref(),
            Some("http://example.org/base/relative")
        );
        assert_eq!(resolve(base, "").as_deref(), Some("http://example.org/base/"));
        assert_eq!(
            resolve("http://example.org/", "#x").as_deref(),
            Some("http://example.org/#x")
        );
        assert_eq!(resolve(base, "../up").as_deref(), Some("http://example.org/up"));
        assert_eq!(resolve(base, "/root").as_deref(), Some("http://example.org/root"));
        assert_eq!(
            resolve("http://example.org", "a").as_deref(),
            Some("http://example.org/a")
        );
    }

    #[test]
    fn absoluteness_is_a_scheme_and_a_colon() {
        assert!(is_absolute("http://x"));
        assert!(is_absolute("urn:x"));
        assert!(!is_absolute("thing"));
        assert!(!is_absolute("#x"));
    }
}
