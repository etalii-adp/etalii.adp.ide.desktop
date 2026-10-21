//! `formatNumber(n, pattern)` (DISL section 12.4): a number written through an ICU decimal pattern.
//!
//! The pattern is ICU's (`DecimalFormat`, after LDML's number patterns): a positive subpattern and an optional
//! negative one after `;`, each a prefix, a numeric part of `#`, `0`, `,` and `.`, and a suffix. `0` is a digit
//! always written, `#` one written when it is not a leading or trailing zero, `,` sets the grouping size (the digits
//! between the last `,` and the end of the integer part, and between the last two for the groups further left) and `.` starts the fraction. Literal text in the prefix or
//! suffix may be quoted with `'` (`''` is a quote), `%` multiplies by 100 and `‰` by 1000. The value is rounded half
//! to even, as ICU rounds, on its shortest decimal form, so `2.675` with two decimals is `2.68` as ICU writes it, not
//! the `2.67` its binary value would give.
//!
//! Section 12.4 formats "in `env.locale`"; a library function sees no environment, so this runtime writes every number
//! with the root locale's symbols (`,` between groups, `.` before the fraction, `-` for a negative number), which is
//! the locale every specification vendored here formats in.

/// One subpattern, read: what is written before and after the number, and how the number is written.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Subpattern {
    prefix: String,
    suffix: String,
    /// How many integer digits are always written.
    minimum_integer: usize,
    /// How many digits make the group nearest the point, or 0 for none.
    grouping: usize,
    /// How many make every group further left (`#,##,##0` groups 3 then 2); the first size when the pattern sets one.
    secondary_grouping: usize,
    minimum_fraction: usize,
    maximum_fraction: usize,
    /// What the value is multiplied by before it is written: 100 for `%`, 1000 for `‰`.
    multiplier: u32,
}

/// `value` written through `pattern`, or why the pattern cannot be read.
pub(crate) fn format_number(value: f64, pattern: &str) -> Result<String, String> {
    let (positive, negative) = split_subpatterns(pattern);
    let positive = read_subpattern(positive, pattern)?;
    let negative = match negative {
        Some(text) => Some(read_subpattern(text, pattern)?),
        None => None,
    };
    if value.is_nan() {
        return Ok("NaN".to_owned());
    }
    let scaled = value * f64::from(positive.multiplier);
    let digits = if scaled.is_infinite() {
        "∞".to_owned()
    } else {
        write_digits(scaled.abs(), &positive)
    };
    // A value that rounds to zero is written without its sign, as ICU writes it.
    let is_negative =
        (scaled < 0.0 && digits.chars().any(|c| c.is_ascii_digit() && c != '0')) || scaled == f64::NEG_INFINITY;
    Ok(match (is_negative, negative) {
        (false, _) => format!("{}{digits}{}", positive.prefix, positive.suffix),
        (true, Some(negative)) => format!("{}{digits}{}", negative.prefix, negative.suffix),
        (true, None) => format!("-{}{digits}{}", positive.prefix, positive.suffix),
    })
}

/// The pattern's positive subpattern and its negative one, split at the first `;` outside quotes.
fn split_subpatterns(pattern: &str) -> (&str, Option<&str>) {
    let mut quoted = false;
    for (at, c) in pattern.char_indices() {
        match c {
            '\'' => quoted = !quoted,
            ';' if !quoted => return (&pattern[..at], Some(&pattern[at + 1..])),
            _ => {}
        }
    }
    (pattern, None)
}

fn read_subpattern(text: &str, pattern: &str) -> Result<Subpattern, String> {
    let refused = || format!("formatNumber() takes an ICU number pattern such as '#,##0.##', not '{pattern}'.");
    let mut prefix = String::new();
    let mut suffix = String::new();
    let mut multiplier = 1;
    // 0 before the number, 1 in it, 2 after it.
    let mut part = 0;
    let mut integer = String::new();
    let mut fraction: Option<String> = None;
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\'' {
            if chars.peek() == Some(&'\'') {
                chars.next();
                push_literal(part, &mut prefix, &mut suffix, '\'');
            } else {
                quoted = !quoted;
            }
            continue;
        }
        let numeric = !quoted && matches!(c, '#' | '0'..='9' | ',' | '.');
        if numeric {
            if part == 2 {
                return Err(refused());
            }
            part = 1;
            match (c, &mut fraction) {
                ('.', None) => fraction = Some(String::new()),
                ('.', Some(_)) => return Err(refused()),
                (',', Some(_)) => return Err(refused()),
                (_, Some(digits)) => digits.push(c),
                (_, None) => integer.push(c),
            }
            continue;
        }
        if part == 1 {
            part = 2;
        }
        if !quoted {
            match c {
                '%' => multiplier = 100,
                '‰' => multiplier = 1000,
                _ => {}
            }
        }
        push_literal(part, &mut prefix, &mut suffix, c);
    }
    if quoted || part == 0 {
        return Err(refused());
    }
    let integer_digits: String = integer.chars().filter(|c| *c != ',').collect();
    let fraction = fraction.unwrap_or_default();
    if integer_digits.is_empty() && fraction.is_empty() {
        return Err(refused());
    }
    let commas: Vec<usize> = integer.match_indices(',').map(|(at, _)| at).collect();
    let grouping = commas.last().map_or(0, |at| integer.len() - at - 1);
    let secondary_grouping = match commas.as_slice() {
        [.., before, last] => last - before - 1,
        _ => grouping,
    };
    Ok(Subpattern {
        prefix,
        suffix,
        minimum_integer: integer_digits.chars().skip_while(|c| *c == '#').count(),
        grouping,
        secondary_grouping,
        minimum_fraction: fraction.chars().take_while(|c| *c != '#').count(),
        maximum_fraction: fraction.len(),
        multiplier,
    })
}

fn push_literal(part: u8, prefix: &mut String, suffix: &mut String, c: char) {
    if part == 0 {
        prefix.push(c);
    } else {
        suffix.push(c);
    }
}

/// A non-negative finite `value` as the subpattern writes its digits: rounded, padded, trimmed and grouped.
fn write_digits(value: f64, subpattern: &Subpattern) -> String {
    let (mut integer, mut fraction) = rounded(value, subpattern.maximum_fraction);
    while fraction.len() > subpattern.minimum_fraction && fraction.ends_with('0') {
        fraction.pop();
    }
    let integer_trimmed = integer.trim_start_matches('0');
    integer = if integer_trimmed.len() < subpattern.minimum_integer {
        format!("{integer_trimmed:0>width$}", width = subpattern.minimum_integer)
    } else {
        integer_trimmed.to_owned()
    };
    if integer.is_empty() && fraction.is_empty() {
        integer.push('0');
    }
    let integer = grouped(&integer, subpattern.grouping, subpattern.secondary_grouping);
    if fraction.is_empty() {
        integer
    } else {
        format!("{integer}.{fraction}")
    }
}

/// `value`'s shortest decimal form rounded half to even to `decimals` fraction digits: its integer digits and its
/// `decimals` fraction digits.
fn rounded(value: f64, decimals: usize) -> (String, String) {
    let shortest = format!("{value}");
    let (integer, fraction) = shortest.split_once('.').unwrap_or((&shortest, ""));
    let mut digits: Vec<u8> = integer.bytes().chain(fraction.bytes()).map(|b| b - b'0').collect();
    let point = integer.len();
    let keep = point + decimals;
    if digits.len() > keep {
        let rest = &digits[keep..];
        let first = rest[0];
        let beyond_half = rest[1..].iter().any(|d| *d != 0);
        let previous_is_odd = keep > 0 && digits[keep - 1] % 2 == 1;
        let up = first > 5 || (first == 5 && (beyond_half || previous_is_odd));
        digits.truncate(keep);
        if up {
            let mut at = keep;
            loop {
                if at == 0 {
                    digits.insert(0, 1);
                    break;
                }
                at -= 1;
                if digits[at] == 9 {
                    digits[at] = 0;
                } else {
                    digits[at] += 1;
                    break;
                }
            }
        }
    }
    digits.resize(digits.len().max(keep), 0);
    let point = digits.len() - decimals;
    let text = |range: &[u8]| range.iter().map(|d| char::from(b'0' + d)).collect::<String>();
    (text(&digits[..point]), text(&digits[point..]))
}

/// `integer` with a `,` after the first `primary` digits from the right and then after every `secondary`; as it is
/// for a primary size of 0.
fn grouped(integer: &str, primary: usize, secondary: usize) -> String {
    if primary == 0 || integer.len() <= primary {
        return integer.to_owned();
    }
    let secondary = if secondary == 0 { primary } else { secondary };
    let mut groups = Vec::new();
    let mut end = integer.len() - primary;
    groups.push(&integer[end..]);
    while end > secondary {
        groups.push(&integer[end - secondary..end]);
        end -= secondary;
    }
    groups.push(&integer[..end]);
    groups.reverse();
    groups.join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halves_round_to_even_on_the_shortest_decimal_form() {
        assert_eq!(rounded(2.675, 2), ("2".into(), "68".into()));
        assert_eq!(rounded(0.125, 2), ("0".into(), "12".into()));
        assert_eq!(rounded(0.135, 2), ("0".into(), "14".into()));
        assert_eq!(rounded(9.995, 2), ("10".into(), "00".into()));
        assert_eq!(rounded(2.5, 0), ("2".into(), "".into()));
        assert_eq!(rounded(3.5, 0), ("4".into(), "".into()));
    }

    #[test]
    fn groups_are_counted_from_the_right() {
        assert_eq!(grouped("1234567", 3, 3), "1,234,567");
        assert_eq!(grouped("123", 3, 3), "123");
        assert_eq!(grouped("12345", 0, 0), "12345");
        assert_eq!(grouped("1234567", 3, 2), "12,34,567");
    }
}
