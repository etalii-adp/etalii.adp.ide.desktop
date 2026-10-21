//! The splices of one edit while it is being planned, against the body before the edit (the standalone host's
//! `Planning/Plan`), and the refusal that ends planning when a change cannot be made (FBL section 6.4).

use crate::rules::reading::BodyReading;
use crate::splice::{Operation, Splice};
use crate::text::Span;

/// Why a change cannot be made; the edit is refused with this sentence and nothing is written. `invalid` marks a
/// change that names what the reading does not have, which the standalone host throws as an argument error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Refused {
    pub reason: String,
    pub invalid: bool,
}

/// What a planning step gives: nothing, or the refusal.
pub(crate) type Planning<T = ()> = Result<T, Refused>;

/// Refuses the change being planned.
pub(crate) fn refuse<T>(reason: impl Into<String>) -> Planning<T> {
    Err(Refused {
        reason: reason.into(),
        invalid: false,
    })
}

/// Stops planning a change that names what the reading does not have.
pub(crate) fn invalid<T>(reason: impl Into<String>) -> Planning<T> {
    Err(Refused {
        reason: reason.into(),
        invalid: true,
    })
}

pub(crate) struct Plan<'a> {
    pub reading: &'a BodyReading,
    splices: Vec<Splice>,
    pub snapshot: bool,
}

impl<'a> Plan<'a> {
    pub fn new(reading: &'a BodyReading) -> Self {
        Plan {
            reading,
            splices: Vec::new(),
            snapshot: false,
        }
    }

    pub fn add(&mut self, operation: Operation, start: usize, end: usize, text: impl Into<String>) {
        let text = text.into();
        if start == end && text.is_empty() {
            return;
        }
        self.splices.push(Splice {
            operation,
            start,
            end,
            text,
        });
    }

    pub fn add_span(&mut self, operation: Operation, span: Span, text: impl Into<String>) {
        self.add(operation, span.start, span.end, text);
    }

    /// Whether a splice already replaces bytes overlapping `span`.
    pub fn touches(&self, span: Span) -> bool {
        self.splices.iter().any(|s| s.start < span.end && span.start < s.end)
    }

    /// The splices in body order: sorted by start, an insertion before a replacement that starts at its offset,
    /// those at one offset otherwise kept in the order they were planned (FBL section 6.5). Overlapping splices
    /// are a planning error.
    pub fn ordered(&self) -> Result<Vec<Splice>, String> {
        let mut ordered: Vec<(usize, &Splice)> = self.splices.iter().enumerate().collect();
        ordered.sort_by_key(|(i, s)| (s.start, usize::from(s.start != s.end), *i));
        let ordered: Vec<Splice> = ordered.into_iter().map(|(_, s)| s.clone()).collect();
        for pair in ordered.windows(2) {
            if pair[1].start < pair[0].end {
                return Err(format!(
                    "Two splices of one edit overlap: {} and {}.",
                    describe(&pair[0]),
                    describe(&pair[1])
                ));
            }
        }
        Ok(ordered)
    }
}

/// A splice as the standalone host writes one in a message: `replace-value 3-4 "x"`.
pub(crate) fn describe(splice: &Splice) -> String {
    format!(
        "{} {}-{} \"{}\"",
        splice.operation, splice.start, splice.end, splice.text
    )
}
