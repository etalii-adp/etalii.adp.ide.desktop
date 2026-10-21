//! The tool's sentence for a statement the binding does not read (the standalone host's `CausalLoopUnreadable.cs` at
//! `8b6ccfc`; causal-loop-disl-fbl finding B2, `causal-loop-diagram.md` gap 1): one of four, chosen by what the
//! statement starts like. Whether a line is read is the binding's alone; this only words a line it left unread, which
//! the specification reports once on its line under `causal-loop.unreadable-line`.

/// What a link reads like, for a statement that starts like one and is not one.
pub const LINK_SHAPE: &str = "A link reads 'link <from> -> <to>', optionally followed by a polarity, 'delayed', 'flipped', a weight and a label.";

/// The sentence for `statement`, a line the binding does not read.
pub fn sentence_for(statement: &str) -> String {
    let words = words(statement);
    let first = words.first().cloned().unwrap_or_else(|| statement.trim().to_owned());
    match first.as_str() {
        "variable" if words.get(1).is_some_and(String::is_empty) => {
            "A variable needs an id of at least one character.".to_owned()
        }
        "link" if words.len() >= 4 && words[2] == "->" => {
            for word in &words[4..] {
                if let Some(weight) = word.strip_prefix("weight=")
                    && !reads_as_number(weight)
                {
                    return format!("'{word}' does not read as a weight.");
                }
            }
            LINK_SHAPE.to_owned()
        }
        "link" => LINK_SHAPE.to_owned(),
        _ => format!("'{first}' is not a statement this reading knows."),
    }
}

/// Whether `text` reads as a number as .NET's `double.TryParse` with `NumberStyles.Float` and the invariant culture
/// reads one: surrounding whitespace, a sign, digits with one decimal point, an exponent, and the invariant names of
/// the infinities and of NaN.
fn reads_as_number(text: &str) -> bool {
    let text = text.trim();
    if matches!(
        text,
        "NaN" | "Infinity" | "-Infinity" | "+Infinity" | "\u{221E}" | "-\u{221E}" | "+\u{221E}"
    ) {
        return true;
    }
    let bytes = text.as_bytes();
    let mut i = 0;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }
    let mut digits = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
        digits += 1;
    }
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return false;
    }
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        i += 1;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i == start {
            return false;
        }
    }
    i == bytes.len()
}

/// The statement's words: split at whitespace, a double-quoted run kept together as one word without its quotes,
/// and an unclosed quote running to the end of the line.
fn words(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut words = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        if chars[index].is_whitespace() {
            index += 1;
            continue;
        }
        if chars[index] == '"' {
            match chars[index + 1..].iter().position(|c| *c == '"') {
                None => {
                    words.push(chars[index + 1..].iter().collect());
                    break;
                }
                Some(offset) => {
                    let close = index + 1 + offset;
                    words.push(chars[index + 1..close].iter().collect());
                    index = close + 1;
                    continue;
                }
            }
        }
        let start = index;
        while index < chars.len() && !chars[index].is_whitespace() {
            index += 1;
        }
        words.push(chars[start..index].iter().collect());
    }
    words
}
