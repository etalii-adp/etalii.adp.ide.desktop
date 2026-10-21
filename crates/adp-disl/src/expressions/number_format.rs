//! `formatNumber(n, pattern)` of the DISL function library (DISL section 12.4: "ICU number formatting in
//! `env.locale`"), for the decimal patterns a specification writes: `#,##0`, `0.00`, `#,##0.###`, a literal prefix or
//! suffix (quoted with `'` where it holds a pattern character), `%` and `‰`, and a negative subpattern after `;`.
//!
//! The value is taken at its shortest round-trip decimal digits, as ICU's `DecimalQuantity` takes a double, and
//! rounded half to even, ICU's default rounding mode. This runtime has no locale of its own, so the symbols are
//! those of `en`: `,` groups thousands, `.` separates the fraction, `-` marks a negative number.

/// What a pattern says about the number part and the text around it.
#[derive(Debug, Clone, Default, PartialEq)]
struct Pattern {
    prefix: String,
    suffix: String,
    min_integer: usize,
    min_fraction: usize,
    max_fraction: usize,
    /// The size of the group nearest the decimal separator, or none when the pattern does not group.
    grouping: Option<usize>,
    /// 1, or 100 for `%` and 1000 for `‰`.
    multiplier: u32,
}

/// One subpattern parsed: the pattern and where it stopped.
fn subpattern(text: &[char]) -> Result<Pattern, String> {
    let mut pattern = Pattern {
        multiplier: 1,
        ..Pattern::default()
    };
    let mut at = 0;
    let mut number_seen = false;
    let mut in_number = false;
    let mut integer_part = String::new();
    let mut fraction_part = String::new();
    let mut in_fraction = false;
    while at < text.len() {
        let c = text[at];
        let is_number_char = matches!(c, '#' | '0'..='9' | ',' | '.');
        if is_number_char && (!number_seen || in_number) {
            number_seen = true;
            in_number = true;
            match c {
                '.' if in_fraction => return Err("A pattern has one decimal separator.".to_owned()),
                '.' => in_fraction = true,
                ',' if in_fraction => return Err("A pattern does not group the fraction.".to_owned()),
                _ if in_fraction => fraction_part.push(c),
                _ => integer_part.push(c),
            }
            at += 1;
            continue;
        }
        in_number = false;
        let target = if number_seen {
            &mut pattern.suffix
        } else {
            &mut pattern.prefix
        };
        match c {
            '\'' => {
                // A quoted run is literal; two quotes are one quote.
                if text.get(at + 1) == Some(&'\'') {
                    target.push('\'');
                    at += 2;
                    continue;
                }
                at += 1;
                loop {
                    match text.get(at) {
                        None => return Err("A quote in the pattern is not closed.".to_owned()),
                        Some('\'') if text.get(at + 1) == Some(&'\'') => {
                            target.push('\'');
                            at += 2;
                        }
                        Some('\'') => {
                            at += 1;
                            break;
                        }
                        Some(other) => {
                            target.push(*other);
                            at += 1;
                        }
                    }
                }
                continue;
            }
            '%' => {
                pattern.multiplier = 100;
                target.push('%');
            }
            '‰' => {
                pattern.multiplier = 1000;
                target.push('‰');
            }
            '#' | '0'..='9' | ',' | '.' => {
                return Err("A pattern has one number part.".to_owned());
            }
            other => target.push(other),
        }
        at += 1;
    }
    if !number_seen {
        return Err("A pattern needs a number part, such as #,##0.##.".to_owned());
    }
    pattern.min_integer = integer_part.chars().filter(char::is_ascii_digit).count();
    pattern.min_fraction = fraction_part.chars().filter(char::is_ascii_digit).count();
    pattern.max_fraction = fraction_part.len();
    if fraction_part.contains('0') && fraction_part.trim_start_matches('0').contains('0') {
        // `0#0` after the separator: ICU wants the zeros first.
        return Err("In the fraction, a pattern's 0 digits come before its # digits.".to_owned());
    }
    if let Some(last) = integer_part.rfind(',') {
        let size = integer_part.len() - last - 1;
        if size == 0 {
            return Err("A pattern does not end its integer part with a grouping separator.".to_owned());
        }
        pattern.grouping = Some(size);
    }
    Ok(pattern)
}

/// `value` written by `pattern`, or why the pattern is not one this runtime reads.
///
/// # Errors
///
/// When the pattern has no number part, more than one, an unclosed quote, or a fraction with a `0` after a `#`.
pub fn format_number(value: f64, pattern: &str) -> Result<String, String> {
    let characters: Vec<char> = pattern.chars().collect();
    let split = split_subpatterns(&characters);
    let positive = subpattern(&characters[..split.unwrap_or(characters.len())])?;
    let negative = match split {
        Some(at) => Some(subpattern(&characters[at + 1..])?),
        None => None,
    };
    if value.is_nan() {
        return Ok(format!("{}NaN{}", positive.prefix, positive.suffix));
    }
    let negative_value = value.is_sign_negative() && value != 0.0;
    if value.is_infinite() {
        return Ok(match (negative_value, &negative) {
            (false, _) => format!("{}∞{}", positive.prefix, positive.suffix),
            (true, Some(n)) => format!("{}∞{}", n.prefix, n.suffix),
            (true, None) => format!("-{}∞{}", positive.prefix, positive.suffix),
        });
    }
    let scaled = value.abs() * f64::from(positive.multiplier);
    let (integer, fraction) = rounded_digits(scaled, positive.max_fraction);
    let mut fraction = fraction;
    while fraction.len() > positive.min_fraction && fraction.ends_with('0') {
        fraction.pop();
    }
    while fraction.len() < positive.min_fraction {
        fraction.push('0');
    }
    let mut integer = integer.trim_start_matches('0').to_owned();
    while integer.len() < positive.min_integer {
        integer.insert(0, '0');
    }
    if integer.is_empty() && fraction.is_empty() {
        // `#.##` of zero still writes a digit.
        integer.push('0');
    }
    let grouped = match positive.grouping {
        Some(size) => group(&integer, size),
        None => integer,
    };
    let body = if fraction.is_empty() {
        grouped
    } else {
        format!("{grouped}.{fraction}")
    };
    // A value that rounds to zero is written without its sign, as ICU writes it.
    let is_zero = body.chars().all(|c| matches!(c, '0' | '.' | ','));
    Ok(match (negative_value && !is_zero, &negative) {
        (false, _) => format!("{}{body}{}", positive.prefix, positive.suffix),
        (true, Some(n)) => format!("{}{body}{}", n.prefix, n.suffix),
        (true, None) => format!("-{}{body}{}", positive.prefix, positive.suffix),
    })
}

/// Where the `;` that starts the negative subpattern is, outside quotes.
fn split_subpatterns(characters: &[char]) -> Option<usize> {
    let mut quoted = false;
    for (at, c) in characters.iter().enumerate() {
        match c {
            '\'' => quoted = !quoted,
            ';' if !quoted => return Some(at),
            _ => {}
        }
    }
    None
}

/// The integer digits and the `places` fraction digits of `value` (not negative), rounded half to even from its
/// shortest round-trip digits.
fn rounded_digits(value: f64, places: usize) -> (String, String) {
    // `{:e}` writes the shortest digits that read back as the same double.
    let scientific = format!("{value:e}");
    let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
    let exponent: i64 = exponent.parse().unwrap_or(0);
    let digits: Vec<u8> = mantissa.bytes().filter(u8::is_ascii_digit).map(|b| b - b'0').collect();
    // The digits as a whole number times ten to the power `point_shift`: value = digits × 10^(exponent - (n - 1)).
    let point = exponent + 1; // how many of the digits stand before the decimal point
    let mut integer: Vec<u8> = Vec::new();
    let mut fraction: Vec<u8> = Vec::new();
    for (index, digit) in digits.iter().enumerate() {
        if (index as i64) < point {
            integer.push(*digit);
        } else {
            fraction.push(*digit);
        }
    }
    if point > digits.len() as i64 {
        integer.extend(std::iter::repeat_n(0, (point - digits.len() as i64) as usize));
    }
    if point < 0 {
        let mut leading = vec![0u8; (-point) as usize];
        leading.extend(fraction);
        fraction = leading;
    }
    if fraction.len() > places {
        let kept = &fraction[..places];
        let rest = &fraction[places..];
        let first = rest[0];
        let beyond = rest[1..].iter().any(|d| *d != 0);
        let last_kept = kept.last().copied().or_else(|| integer.last().copied()).unwrap_or(0);
        let up = first > 5 || (first == 5 && (beyond || last_kept % 2 == 1));
        let mut whole: Vec<u8> = integer.iter().chain(kept.iter()).copied().collect();
        if up {
            let mut at = whole.len();
            loop {
                if at == 0 {
                    whole.insert(0, 1);
                    break;
                }
                at -= 1;
                if whole[at] == 9 {
                    whole[at] = 0;
                } else {
                    whole[at] += 1;
                    break;
                }
            }
        }
        let integer_len = whole.len() - places;
        integer = whole[..integer_len].to_vec();
        fraction = whole[integer_len..].to_vec();
    }
    let text = |d: &[u8]| d.iter().map(|d| char::from(b'0' + d)).collect::<String>();
    (text(&integer), text(&fraction))
}

/// `integer` with a `,` between every group of `size` digits from the right.
fn group(integer: &str, size: usize) -> String {
    let digits: Vec<char> = integer.chars().collect();
    let mut out = String::new();
    for (index, digit) in digits.iter().enumerate() {
        let remaining = digits.len() - index;
        if index > 0 && remaining.is_multiple_of(size) {
            out.push(',');
        }
        out.push(*digit);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::format_number;

    #[test]
    fn decimal_patterns_group_and_round_half_to_even() {
        for (value, pattern, expected) in [
            (1234567.891, "#,##0.##", "1,234,567.89"),
            (0.5, "#,##0", "0"),
            (1.5, "#,##0", "2"),
            (2.5, "#,##0", "2"),
            (2.675, "0.00", "2.68"),
            (0.1234, "#,##0.###", "0.123"),
            (12.0, "#,##0.##", "12"),
            (12.0, "0.00", "12.00"),
            (-3.25, "#,##0.#", "-3.2"),
            (-0.001, "#,##0.##", "0"),
            (999.996, "#,##0.##", "1,000"),
            (0.25, "0%", "25%"),
            (5.0, "'#'0", "#5"),
            (1e21, "#,##0", "1,000,000,000,000,000,000,000"),
            (-4.0, "0;(0)", "(4)"),
            (7.0, "#.##", "7"),
            (0.0, "#.##", "0"),
        ] {
            assert_eq!(
                Ok(expected.to_owned()),
                format_number(value, pattern),
                "{value} {pattern}"
            );
        }
    }

    #[test]
    fn a_pattern_without_a_number_part_is_an_error() {
        assert!(format_number(1.0, "abc").is_err());
        assert!(format_number(1.0, "'unclosed 0").is_err());
    }
}
