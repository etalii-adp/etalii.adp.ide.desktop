//! The `yearMonth` primitive (DISL section 4.2; the standalone host's `YearMonth`): a month without a day in
//! astronomical year numbering, which CEL sees as the month index `year × 12 + (month − 1)` and JSON writes as
//! `±YYYY-MM` with four to six year digits, and the LDML subset of section 5.5 that formats it.

use std::sync::LazyLock;

use regex::Regex;

const SHORT_NAMES: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

const LONG_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

static JSON_FORM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(-?[0-9]{4,6})-([0-9]{2})$").expect("the yearMonth pattern compiles"));

/// `yearMonth(y, m)`: the month index of month `month` (1 to 12) of `year`.
pub fn of(year: i64, month: i64) -> Result<i64, String> {
    if (1..=12).contains(&month) {
        Ok(year * 12 + (month - 1))
    } else {
        Err(format!("yearMonth() takes a month from 1 to 12, not {month}."))
    }
}

/// `ym.year()`: floored, so month index −1 is December of year −1.
pub fn year_of(index: i64) -> i64 {
    index.div_euclid(12)
}

/// `ym.month()`, from 1 to 12.
pub fn month_of(index: i64) -> i64 {
    index - year_of(index) * 12 + 1
}

/// The month index of the JSON form, or none for text not in it or with a month outside 1 to 12.
pub fn parse(text: &str) -> Option<i64> {
    let captures = JSON_FORM.captures(text)?;
    let year: i64 = captures[1].parse().ok()?;
    let month: i64 = captures[2].parse().ok()?;
    (1..=12).contains(&month).then_some(year * 12 + (month - 1))
}

/// `formatYearMonth(i, pattern)`: LDML, where `u` is the signed astronomical year padded to the run's length, `M`
/// and `MM` the month's number, `MMM` and `MMMM` its English name, and quoted text literal (section 5.5). Every other
/// letter is refused, `y` included, whose era is ambiguous before year 1.
pub fn format(index: i64, pattern: &str) -> Result<String, String> {
    let year = year_of(index);
    let month = month_of(index);
    let chars: Vec<char> = pattern.chars().collect();
    let mut text = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\'' {
            let close = chars[i + 1..].iter().position(|x| *x == '\'').map(|p| p + i + 1);
            match close {
                Some(close) if close == i + 1 => {
                    text.push('\'');
                    i += 2;
                }
                Some(close) => {
                    text.extend(&chars[i + 1..close]);
                    i = close + 1;
                }
                None => return Err("formatYearMonth() was given a pattern with an unclosed quote.".into()),
            }
            continue;
        }
        if !c.is_ascii_alphabetic() {
            text.push(c);
            i += 1;
            continue;
        }
        let mut run = 1;
        while i + run < chars.len() && chars[i + run] == c {
            run += 1;
        }
        match (c, run) {
            ('u', _) => {
                if year < 0 {
                    text.push('-');
                }
                text.push_str(&format!("{:0>run$}", year.unsigned_abs()));
            }
            ('M', 1 | 2) => text.push_str(&format!("{month:0>run$}")),
            ('M', 3) => text.push_str(SHORT_NAMES[(month - 1) as usize]),
            ('M', 4) => text.push_str(LONG_NAMES[(month - 1) as usize]),
            _ => {
                return Err(format!(
                    "formatYearMonth() formats a month index with the pattern letters u and M only, not '{c}' (DISL §5.5)."
                ));
            }
        }
        i += run;
    }
    Ok(text)
}
