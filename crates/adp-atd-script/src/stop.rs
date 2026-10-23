//! How a call ends other than by returning (ATD §12.4): the gesture is refused and its transaction discarded.

use adp_atd_check::{Finding, Severity};
use adp_atd_syntax::Span;
use adp_history::Refusal;
use serde_json::Value as Json;

/// Why a call stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// A `fail`: the message is the tool engineer's.
    Fail,
    /// A CEL error, or an action the host refused.
    Error,
    /// A loop reached its `limit` (`ATD7101`).
    Limit,
    /// The gesture's step budget, or a value's length, was exceeded (`ATD7102`).
    Budget,
    /// A function recursed past its bound (`ATD7103`).
    Recursion,
    /// The debugger stopped the call.
    Debugger,
}

/// A call that stopped: why, in which function (or operation, or hook), where, and the sentence a person reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stop {
    pub kind: Kind,
    pub function: String,
    pub message: String,
    pub span: Option<Box<Span>>,
}

impl Stop {
    pub(crate) fn new(kind: Kind, function: &str, node: &Json, message: String) -> Self {
        Stop {
            kind,
            function: function.to_owned(),
            message,
            span: Span::of(node).map(Box::new),
        }
    }

    /// The code of ATD §15 the stop is reported with: the run-time codes for a limit, the budget and the recursion.
    pub fn code(&self) -> Option<&'static str> {
        match self.kind {
            Kind::Limit => Some("ATD7101"),
            Kind::Budget => Some("ATD7102"),
            Kind::Recursion => Some("ATD7103"),
            Kind::Fail | Kind::Error | Kind::Debugger => None,
        }
    }

    /// The finding a stop with a code is reported as, at the statement or function it names.
    pub fn finding(&self) -> Option<Finding> {
        let code = self.code()?;
        let span = self.span.as_deref().cloned().unwrap_or_default();
        Some(Finding {
            code,
            severity: Severity::Error,
            file: span.file,
            line: span.line,
            column: span.column,
            end_line: span.end_line,
            end_column: span.end_column,
            start: span.start,
            end: span.end,
            message: self.message.clone(),
            suggestions: Vec::new(),
            fixes: Vec::new(),
        })
    }

    /// The refusal of the gesture, as the history reports it.
    pub fn refusal(&self) -> Refusal {
        Refusal::new(self.message.clone())
    }
}

impl std::fmt::Display for Stop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.code() {
            Some(code) => write!(f, "{code}: {}", self.message),
            None => f.write_str(&self.message),
        }
    }
}

impl std::error::Error for Stop {}
