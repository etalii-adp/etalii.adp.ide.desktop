//! Hover (FR-051): a keyword's entry of Appendix B, or a declaration's kind, type and `///` documentation.

use serde_json::{Value as Json, json};

use crate::{Request, Server, file_name, index, keywords, positions};

pub fn hover(server: &Server, request: &Request) -> Json {
    let checked = server.check(&request.path);
    let index = index::build(&checked);
    let name = file_name(&checked, &request.path);
    if let Some(occurrence) = index.at(&name, request.offset)
        && let Some(declaration) = index.declarations.get(&occurrence.symbol)
    {
        let mut value = format!("```atd\n{}\n```", declaration.detail);
        if let Some(doc) = &declaration.doc {
            value.push_str("\n\n");
            value.push_str(doc);
        }
        let range = positions::range(&request.text, occurrence.start, occurrence.end, server.encoding());
        return json!({"contents": {"kind": "markdown", "value": value}, "range": range});
    }
    // A keyword under the cursor.
    let parsed = adp_atd_syntax::parse(&request.text);
    for token in &parsed.tokens {
        if token.start <= request.offset && request.offset <= token.end && token.is_keyword() {
            let word = token.text(&request.text);
            if let Some(doc) = keywords::doc(word) {
                let range = positions::range(&request.text, token.start, token.end, server.encoding());
                return json!({"contents": {"kind": "markdown", "value": format!("**{word}** — {doc}")}, "range": range});
            }
        }
    }
    Json::Null
}
