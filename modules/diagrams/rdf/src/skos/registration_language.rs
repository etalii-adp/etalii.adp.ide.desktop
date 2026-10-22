//! The scheme reading's `language:` header (etalii.adp spec 030, task T083), ported from the standalone host's
//! `Skos/SkosRegistrationLanguage.cs` at `8b6ccfc`: honoured in the band after `body:`/`view:` and before `layout:`,
//! and ignored and reported when a core header below it would be severed by core's header scan.

use std::fs;
use std::path::Path;

/// The header key this reading claims.
const KEY: &str = "language:";

/// Core's own header-region limit.
const HEADER_SCAN_LIMIT: usize = 8;

/// The lines of `text` as a .NET `StreamReader` reads them: split at `\r\n`, `\n` or `\r`.
fn lines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let bytes = text.as_bytes();
    let (mut start, mut at) = (0, 0);
    while at < bytes.len() {
        match bytes[at] {
            b'\r' => {
                lines.push(&text[start..at]);
                at += if bytes.get(at + 1) == Some(&b'\n') { 2 } else { 1 };
                start = at;
            }
            b'\n' => {
                lines.push(&text[start..at]);
                at += 1;
                start = at;
            }
            _ => at += 1,
        }
    }
    if start < bytes.len() {
        lines.push(&text[start..]);
    }
    lines
}

fn starts_with_ignoring_case(text: &str, prefix: &str) -> bool {
    text.get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
}

/// The display language the registration declares, lowercased, and the 1-based line of a `language:` header that sits
/// above a core header (`0` otherwise), in which case the language is none (`SkosRegistrationLanguage.Read`).
pub fn read(registration: Option<&Path>) -> (Option<String>, usize) {
    let Some(text) = registration.and_then(|path| fs::read(path).ok()) else {
        return (None, 0);
    };
    let text = String::from_utf8_lossy(&text);
    let text = text.trim_start_matches('\u{feff}');

    let mut language = None;
    let mut language_line = 0;
    let mut core_header_below_language = false;
    for (scanned, line) in lines(text).into_iter().skip(1).take(HEADER_SCAN_LIMIT).enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if starts_with_ignoring_case(trimmed, "layout:") {
            break;
        }
        if starts_with_ignoring_case(trimmed, "body:") || starts_with_ignoring_case(trimmed, "view:") {
            core_header_below_language |= language_line > 0;
            continue;
        }
        if language_line == 0 && starts_with_ignoring_case(trimmed, KEY) {
            language_line = scanned + 2;
            let value = trimmed[KEY.len()..].trim();
            language = (!value.is_empty()).then(|| value.to_lowercase());
        }
    }
    if core_header_below_language {
        (None, language_line)
    } else {
        (language, 0)
    }
}
