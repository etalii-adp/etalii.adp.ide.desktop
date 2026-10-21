//! The common subset of RE2, .NET, Java and ECMAScript regular expressions FBL section 2.5 allows (the standalone
//! host's `RegexSubset` and `BoundedRegex`). A validator rejects exactly what that section excludes: backreferences,
//! lookaround, atomic groups, possessive quantifiers, inline flags and Unicode property classes. The `regex` crate
//! matches in linear time, so the time bound of FBL section 16 is never reached here.

/// None when `expression` is in the subset; otherwise a sentence naming the construct.
pub fn check(expression: &str) -> Option<String> {
    let chars: Vec<char> = expression.chars().collect();
    let mut in_class = false;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' {
            if i + 1 >= chars.len() {
                return Some("The expression ends with a lone backslash.".into());
            }
            let next = chars[i + 1];
            if next.is_ascii_digit() && next != '0' {
                return Some(format!(
                    "'\\{next}' is a backreference, which FBL's regular expressions do not allow."
                ));
            }
            if next == 'k' {
                return Some("'\\k' is a named backreference, which FBL's regular expressions do not allow.".into());
            }
            if next == 'p' || next == 'P' {
                return Some(format!(
                    "'\\{next}' is a Unicode property class, which FBL's regular expressions do not allow."
                ));
            }
            i += 2;
            continue;
        }
        if in_class {
            if c == ']' {
                in_class = false;
            }
            i += 1;
            continue;
        }
        match c {
            '[' => {
                in_class = true;
                if chars.get(i + 1) == Some(&'^') {
                    i += 1;
                }
                if chars.get(i + 1) == Some(&']') {
                    i += 1;
                }
            }
            '(' if chars.get(i + 1) == Some(&'?') => {
                let rest: String = chars[i + 2..].iter().collect();
                if rest.starts_with(':') {
                } else if rest.starts_with("<=") || rest.starts_with("<!") {
                    return Some("Lookbehind is not allowed in FBL's regular expressions.".into());
                } else if rest.starts_with("P<") {
                    return Some("'(?P<name>' is not in the common subset; write '(?<name>'.".into());
                } else if rest.starts_with('<') {
                } else if rest.starts_with('=') || rest.starts_with('!') {
                    return Some("Lookahead is not allowed in FBL's regular expressions.".into());
                } else if rest.starts_with('>') {
                    return Some("Atomic groups are not allowed in FBL's regular expressions.".into());
                } else {
                    return Some(
                        "Inline flags are not allowed in FBL's regular expressions; use the rule's caseInsensitive."
                            .into(),
                    );
                }
            }
            '*' | '+' | '?' | '}' if chars.get(i + 1) == Some(&'+') => {
                return Some("Possessive quantifiers are not allowed in FBL's regular expressions.".into());
            }
            _ => {}
        }
        i += 1;
    }
    if in_class {
        return Some("A character class is not closed.".into());
    }
    if let Err(e) = regex::Regex::new(&to_rust(expression, false)) {
        return Some(format!("The expression does not compile: {e}"));
    }
    None
}

/// The Rust form of a subset expression: `\d` and `\w` mean their ASCII classes, as in RE2 and ECMAScript, and
/// case-insensitivity covers ASCII letters only.
pub fn to_rust(expression: &str, case_insensitive: bool) -> String {
    let chars: Vec<char> = expression.chars().collect();
    let mut out = String::with_capacity(expression.len() + 16);
    let mut in_class = false;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' && i + 1 < chars.len() {
            let next = chars[i + 1];
            i += 2;
            match next {
                'd' => out.push_str(if in_class { "0-9" } else { "[0-9]" }),
                'D' => out.push_str(if in_class { "\\D" } else { "[^0-9]" }),
                'w' => out.push_str(if in_class { "A-Za-z0-9_" } else { "[A-Za-z0-9_]" }),
                'W' => out.push_str(if in_class { "\\W" } else { "[^A-Za-z0-9_]" }),
                _ => {
                    out.push('\\');
                    out.push(next);
                }
            }
            continue;
        }
        if in_class {
            if c == ']' {
                in_class = false;
            }
            ascii_case(&mut out, c, case_insensitive, true);
            i += 1;
            continue;
        }
        if c == '[' {
            in_class = true;
            out.push(c);
            if chars.get(i + 1) == Some(&'^') {
                out.push('^');
                i += 1;
            }
            if chars.get(i + 1) == Some(&']') {
                out.push(']');
                i += 1;
            }
            i += 1;
            continue;
        }
        if c == '(' && chars.get(i + 1) == Some(&'?') && chars.get(i + 2) == Some(&'<') {
            let close = chars[i..]
                .iter()
                .position(|ch| *ch == '>')
                .map(|p| p + i)
                .unwrap_or(chars.len() - 1);
            out.extend(&chars[i..=close]);
            i = close + 1;
            continue;
        }
        ascii_case(&mut out, c, case_insensitive, false);
        i += 1;
    }
    out
}

fn ascii_case(out: &mut String, c: char, case_insensitive: bool, in_class: bool) {
    if !case_insensitive || !c.is_ascii_alphabetic() {
        out.push(c);
        return;
    }
    let (lower, upper) = (c.to_ascii_lowercase(), c.to_ascii_uppercase());
    if in_class {
        out.push(lower);
        out.push(upper);
    } else {
        out.push('[');
        out.push(lower);
        out.push(upper);
        out.push(']');
    }
}

/// A compiled subset expression (FBL section 2.5).
#[derive(Debug, Clone)]
pub struct BoundedRegex {
    pub expression: String,
    pub regex: regex::Regex,
}

impl BoundedRegex {
    /// Compiles `expression`; one that does not compile matches nothing, the loader having refused it already.
    pub fn new(expression: &str, case_insensitive: bool) -> Self {
        let regex = regex::Regex::new(&to_rust(expression, case_insensitive))
            .unwrap_or_else(|_| regex::Regex::new("[^\\s\\S]").expect("the empty class compiles"));
        BoundedRegex {
            expression: expression.to_owned(),
            regex,
        }
    }

    pub fn is_match(&self, input: &str) -> bool {
        self.regex.is_match(input)
    }

    pub fn captures<'h>(&self, input: &'h str) -> Option<regex::Captures<'h>> {
        self.regex.captures(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_excluded_constructs_are_refused() {
        assert!(check("(?=x)").unwrap().contains("Lookahead"));
        assert!(check("(a)\\1").unwrap().contains("backreference"));
        assert!(check("(?i)x").unwrap().contains("Inline flags"));
        assert!(check("^\\s*variable\\s+(?<name>\"[^\"]+\"|[^\\s\"]+)$").is_none());
    }

    #[test]
    fn digits_are_ascii_and_case_insensitivity_covers_ascii_letters() {
        assert_eq!(to_rust("\\d+[\\w]", false), "[0-9]+[A-Za-z0-9_]");
        assert!(BoundedRegex::new("^container", true).is_match("Container"));
        assert!(!BoundedRegex::new("^\\d$", false).is_match("٣"));
    }
}
