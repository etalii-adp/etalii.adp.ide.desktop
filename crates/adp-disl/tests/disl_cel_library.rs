//! The DISL function library (DISL section 12.4) the two bundled definitions call, value by value (the standalone
//! host's `DislCelLibraryTests`).

mod common;

use std::collections::HashMap;
use std::sync::LazyLock;

use adp_cel::Value;
use adp_disl::{DislSpecification, contexts};
use common::{loaded, with};

static JSON: LazyLock<String> = LazyLock::new(|| {
    with(
        r#""metamodel": {
          "enums": { "Phase": { "values": { "peak": { "label": "Peak" }, "trough": {} } } },
          "types": { "Thing": { "attributes": { "name": { "type": "string" } } } } }"#,
    )
});

fn specification() -> DislSpecification {
    loaded(&JSON)
}

fn evaluate(expression: &str) -> Result<Value, String> {
    specification()
        .environment(contexts::ELEMENT, &[])
        .compile(expression)
        .unwrap()
        .evaluate(&HashMap::new())
}

#[test]
fn a_month_index_is_built_and_taken_apart_in_astronomical_years() {
    for (expression, expected) in [
        ("yearMonth(2026, 9)", 2026 * 12 + 8),
        ("yearMonth(0, 1)", 0),
        ("yearMonth(-1, 12)", -1),
        ("yearMonth(-3200, 1)", -3200 * 12),
        ("yearMonth(1900, 1).year()", 1900),
        ("(-1).year()", -1),
        ("(-1).month()", 12),
        ("(-13).year()", -2),
        ("(-12).month()", 1),
        ("0.month()", 1),
        ("yearMonth(2026, 9).month()", 9),
    ] {
        assert_eq!(Ok(Value::Int(expected)), evaluate(expression), "{expression}");
    }
}

#[test]
fn a_month_outside_one_to_twelve_is_an_error() {
    for (expression, month) in [("yearMonth(2026, 0)", 0), ("yearMonth(2026, 13)", 13)] {
        assert_eq!(
            Err(format!("yearMonth() takes a month from 1 to 12, not {month}.")),
            evaluate(expression)
        );
    }
}

#[test]
fn a_month_index_is_formatted_with_an_ldml_pattern() {
    for (expression, expected) in [
        ("formatYearMonth(yearMonth(2026, 9), 'uuuu-MM')", "2026-09"),
        ("formatYearMonth(-1, 'uuuu-MM')", "-0001-12"),
        ("formatYearMonth(yearMonth(-3200, 1), 'uuuu-MM')", "-3200-01"),
        ("formatYearMonth(yearMonth(123456, 1), 'uuuu-MM')", "123456-01"),
        ("formatYearMonth(yearMonth(7, 3), 'u-M')", "7-3"),
        ("formatYearMonth(yearMonth(2026, 9), 'MMM uuuu')", "Sep 2026"),
        ("formatYearMonth(yearMonth(2026, 9), 'MMMM')", "September"),
        (
            "formatYearMonth(yearMonth(2026, 9), 'MMMM \\'of\\' uuuu')",
            "September of 2026",
        ),
        ("formatYearMonth(yearMonth(2026, 9), '\\'\\'MM')", "'09"),
    ] {
        assert_eq!(Ok(Value::from(expected)), evaluate(expression), "{expression}");
    }
}

#[test]
fn a_pattern_letter_a_month_has_not_is_an_error() {
    for (expression, letter) in [
        ("formatYearMonth(0, 'yyyy')", 'y'),
        ("formatYearMonth(0, 'dd-MM')", 'd'),
    ] {
        assert_eq!(
            Err(format!(
                "formatYearMonth() formats a month index with the pattern letters u and M only, not '{letter}' (DISL §5.5)."
            )),
            evaluate(expression)
        );
    }
}

#[test]
fn the_json_form_is_parsed() {
    for (text, expected) in [
        ("2026-09", 2026 * 12 + 8),
        ("0000-01", 0),
        ("-0001-12", -1),
        ("-3200-01", -3200 * 12),
        ("123456-01", 123456 * 12),
        ("-123456-12", -123456 * 12 + 11),
    ] {
        assert_eq!(
            Ok(Value::some(Value::Int(expected))),
            evaluate(&format!("parseYearMonth('{text}')"))
        );
    }
}

#[test]
fn text_not_in_the_json_form_parses_to_none() {
    for text in [
        "2026-00",
        "2026-13",
        "2026-9",
        "226-09",
        "1234567-01",
        "+2026-09",
        " 2026-09",
        "2026-09 ",
        "2026/09",
        "",
    ] {
        assert_eq!(
            Ok(Value::none()),
            evaluate(&format!("parseYearMonth('{text}')")),
            "{text}"
        );
    }
}

#[test]
fn clamp_min_and_max_keep_integers() {
    for (expression, expected) in [
        ("clamp(5, 1, 3)", 3),
        ("clamp(0, 1, 3)", 1),
        ("clamp(2, 1, 3)", 2),
        ("min(3, 1, 2)", 1),
        ("max(3, 1, 2, 7)", 7),
        ("max(1, 0)", 1),
    ] {
        assert!(
            matches!(evaluate(expression), Ok(Value::Int(v)) if v == expected),
            "{expression}"
        );
    }
}

#[test]
fn clamp_min_and_max_compare_numbers_of_either_kind() {
    for (expression, expected) in [
        ("clamp(2.5, 1, 3)", 2.5),
        ("clamp(-0.5, 0.0, 1.0)", 0.0),
        ("min(max(1.25, 0.5), 1.0)", 1.0),
        ("max(1, 2.5)", 2.5),
        ("min(0.25, 1)", 0.25),
    ] {
        assert!(
            matches!(evaluate(expression), Ok(Value::Double(v)) if v == expected),
            "{expression}"
        );
    }
}

#[test]
fn min_and_max_give_the_argument_itself() {
    assert!(matches!(evaluate("max(1, 0.5)"), Ok(Value::Int(1))));
}

#[test]
fn text_is_derived_from_the_specification() {
    for (expression, expected) in [
        ("enumLabel('Phase', 'peak')", "Peak"),
        ("enumLabel('Phase', 'trough')", "trough"),
        ("enumLabel('Phase', 'other')", "other"),
        ("lower('ÀBc')", "àbc"),
    ] {
        assert_eq!(Ok(Value::from(expected)), evaluate(expression), "{expression}");
    }
}

#[test]
fn an_enumeration_the_specification_lacks_is_an_error() {
    assert_eq!(
        Err("'Colour' is not an enumeration of this specification.".to_owned()),
        evaluate("enumLabel('Colour', 'red')")
    );
}

/// `l.sum()` (section 12.4): an int while every item is an int, a double once one is a double.
#[test]
fn a_list_of_numbers_sums_to_an_int_or_a_double() {
    for (expression, expected) in [
        ("[1, 2, 3].sum()", Value::Int(6)),
        ("[].sum()", Value::Int(0)),
        ("[1, 2.5].sum()", Value::Double(3.5)),
        ("[0.5, 0.25].sum()", Value::Double(0.75)),
        ("[10, 20].map(n, n * 2).sum()", Value::Int(60)),
    ] {
        assert_eq!(Ok(expected), evaluate(expression), "{expression}");
    }
    assert_eq!(
        Err("sum() takes a list of numbers.".to_owned()),
        evaluate("['a'].sum()")
    );
    assert_eq!(
        Err("sum() overflowed an int.".to_owned()),
        evaluate("[9223372036854775807, 1].sum()")
    );
}

/// `formatNumber(n, pattern)` (section 12.4): ICU number formatting, in the root locale's symbols.
#[test]
fn a_number_is_formatted_with_an_icu_pattern() {
    for (expression, expected) in [
        ("formatNumber(1840, '#,##0.##')", "1,840"),
        ("formatNumber(1234567.891, '#,##0.##')", "1,234,567.89"),
        ("formatNumber(3.3, '#,##0.##')", "3.3"),
        ("formatNumber(0.5, '#,##0.##')", "0.5"),
        ("formatNumber(0, '#,##0.##')", "0"),
        ("formatNumber(-1500.5, '#,##0.##')", "-1,500.5"),
        ("formatNumber(2.675, '0.00')", "2.68"),
        ("formatNumber(0.125, '0.00')", "0.12"),
        ("formatNumber(7, '000')", "007"),
        ("formatNumber(0.25, '#.##')", ".25"),
        ("formatNumber(12.5, '0')", "12"),
        ("formatNumber(13.5, '0')", "14"),
        ("formatNumber(0.256, '0.#%')", "25.6%"),
        ("formatNumber(42, \"'#'0\")", "#42"),
        ("formatNumber(-3, '0;(0)')", "(3)"),
        ("formatNumber(-0.001, '0.##')", "0"),
        ("formatNumber(1000000, '#,##,##0')", "10,00,000"),
    ] {
        assert_eq!(Ok(Value::from(expected)), evaluate(expression), "{expression}");
    }
    assert_eq!(
        Err("formatNumber() takes an ICU number pattern such as '#,##0.##', not 'abc'.".to_owned()),
        evaluate("formatNumber(1, 'abc')")
    );
}

/// `l.min()` and `l.max()` (section 12.4): the smallest or largest item itself; an empty list has none.
#[test]
fn a_lists_smallest_and_largest_are_its_own_items() {
    for (expression, expected) in [
        ("[3, 1, 2].min()", Value::Int(1)),
        ("[3.5, 1, 7].max()", Value::Int(7)),
        ("[0.25, 0.5].max()", Value::Double(0.5)),
    ] {
        assert_eq!(Ok(expected), evaluate(expression), "{expression}");
    }
    assert_eq!(
        Err("max() of an empty list has no value.".to_owned()),
        evaluate("[].max()")
    );
    assert_eq!(
        Err("min() takes a list of numbers.".to_owned()),
        evaluate("['a'].min()")
    );
}
