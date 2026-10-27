//! FBL section 4.2's selectors over the entries of a tree (yaml, json) or of an xml document (the standalone host's
//! `Rules/Selector`): keys and names, `*`, `**`, `{capture}` and `name[@A='v']`, absolute from the root or relative
//! to an entry, and *(FBL 0.9)* `name[path='v']` over a grammar body's tree.

use std::collections::HashSet;
use std::rc::Rc;

use crate::families::{Entry, EntryId};
use crate::rules::Captures;

#[derive(Debug, Clone, PartialEq)]
enum Segment {
    Name(String, Option<(String, String)>),
    Any,
    Descendants,
    Capture(String),
}

fn parse(text: &str) -> Segment {
    if text == "*" {
        return Segment::Any;
    }
    if text == "**" {
        return Segment::Descendants;
    }
    if text.len() >= 2 && text.starts_with('{') && text.ends_with('}') {
        return Segment::Capture(text[1..text.len() - 1].to_owned());
    }
    // `name[@A='v']` tests an xml attribute; *(FBL 0.9)* `name[path='v']` a grammar node's descendant.
    if let Some(bracket) = text.find('[')
        && bracket > 0
        && text.ends_with("']")
    {
        let predicate = text[bracket + 1..text.len() - 1].trim_start_matches('@');
        if let Some(equals) = predicate.find("='")
            && equals > 0
        {
            return Segment::Name(
                text[..bracket].to_owned(),
                Some((
                    predicate[..equals].to_owned(),
                    predicate[equals + 2..predicate.len() - 1].to_owned(),
                )),
            );
        }
    }
    Segment::Name(text.to_owned(), None)
}

/// An xml element's attribute test of a `name[@A='v']` segment.
pub(crate) type AttributeEquals<'a> = &'a dyn Fn(EntryId, &str, &str) -> bool;

/// The entries `selector` reaches from `from`, in document order, each with its captures.
pub(crate) fn matches(
    entries: &[Entry],
    from: EntryId,
    selector: &str,
    attribute_equals: Option<AttributeEquals<'_>>,
) -> Vec<(EntryId, Captures)> {
    let segments: Vec<Segment> = selector.split('/').filter(|s| !s.is_empty()).map(parse).collect();
    let mut results: Vec<(EntryId, Captures)> = Vec::new();
    let mut seen: HashSet<EntryId> = HashSet::new();
    walk(
        entries,
        &segments,
        attribute_equals,
        from,
        0,
        &mut Vec::new(),
        &mut results,
        &mut seen,
    );
    results.sort_by(|a, b| {
        let (x, y) = (&entries[a.0].own, &entries[b.0].own);
        x.start.cmp(&y.start).then(y.end.cmp(&x.end))
    });
    results
}

#[allow(clippy::too_many_arguments)]
fn walk(
    entries: &[Entry],
    segments: &[Segment],
    attribute_equals: Option<AttributeEquals<'_>>,
    entry: EntryId,
    index: usize,
    captures: &mut Vec<(String, String)>,
    results: &mut Vec<(EntryId, Captures)>,
    seen: &mut HashSet<EntryId>,
) {
    if index == segments.len() {
        if seen.insert(entry) {
            results.push((entry, Rc::new(captures.clone())));
        }
        return;
    }
    match &segments[index] {
        Segment::Descendants => {
            walk(
                entries,
                segments,
                attribute_equals,
                entry,
                index + 1,
                captures,
                results,
                seen,
            );
            for child in entries[entry].children.clone() {
                walk(
                    entries,
                    segments,
                    attribute_equals,
                    child,
                    index,
                    captures,
                    results,
                    seen,
                );
            }
        }
        segment => {
            for &child in &entries[entry].children {
                match segment {
                    Segment::Any => walk(
                        entries,
                        segments,
                        attribute_equals,
                        child,
                        index + 1,
                        captures,
                        results,
                        seen,
                    ),
                    Segment::Capture(name) => {
                        let Some(key) = &entries[child].name else { continue };
                        let mut inner = captures.clone();
                        if let Some(existing) = inner.iter_mut().find(|(n, _)| n == name) {
                            existing.1 = key.clone();
                        } else {
                            inner.push((name.clone(), key.clone()));
                        }
                        walk(
                            entries,
                            segments,
                            attribute_equals,
                            child,
                            index + 1,
                            &mut inner,
                            results,
                            seen,
                        );
                    }
                    Segment::Name(name, attribute) => {
                        if entries[child].name.as_deref() != Some(name.as_str()) {
                            continue;
                        }
                        if let Some((a, v)) = attribute
                            && attribute_equals.is_none_or(|eq| !eq(child, a, v))
                        {
                            continue;
                        }
                        walk(
                            entries,
                            segments,
                            attribute_equals,
                            child,
                            index + 1,
                            captures,
                            results,
                            seen,
                        );
                    }
                    Segment::Descendants => {}
                }
            }
        }
    }
}

/// The root a selector starts from: the document root for an absolute selector, else `relative_to`.
pub(crate) fn start(selector: &str, root: EntryId, relative_to: Option<EntryId>) -> EntryId {
    if selector.starts_with('/') {
        return root;
    }
    relative_to.unwrap_or(root)
}
