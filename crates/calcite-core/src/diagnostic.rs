use std::{error::Error, fmt};

use crate::Span;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticKind {
    UnexpectedCharacter,
    UnexpectedToken,
    UnexpectedEnd,
    InvalidNumber,
    InvalidDate,
    InvalidTime,
    InvalidCalendarDuration,
    MissingTemporalContext,
    UnknownVariable,
    InvalidAssignment,
    InvalidFunctionArguments,
    NumberTooLarge,
    DivisionByZero,
    ExponentTooLarge,
    PrecisionTooLarge,
    RootDegreeTooLarge,
    InvalidLogarithm,
    InvalidTrigonometry,
    InvalidHyperbolic,
    IntegerRequired,
    InvalidCombinatorics,
    CombinatoricsTooLarge,
    NonRealResult,
    UnsupportedOperation,
    UnitMismatch,
    CurrencyMismatch,
    MissingExchangeRate,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    kind: DiagnosticKind,
    span: Span,
    detail: Option<String>,
}

impl Diagnostic {
    pub(crate) fn new(kind: DiagnosticKind, span: Span) -> Self {
        Self {
            kind,
            span,
            detail: None,
        }
    }

    pub(crate) fn with_detail(kind: DiagnosticKind, span: Span, detail: impl Into<String>) -> Self {
        Self {
            kind,
            span,
            detail: Some(detail.into()),
        }
    }

    pub const fn kind(&self) -> DiagnosticKind {
        self.kind
    }

    pub const fn span(&self) -> Span {
        self.span
    }

    pub fn detail(&self) -> Option<&str> {
        self.detail.as_deref()
    }

    pub(crate) fn shifted(mut self, offset: usize) -> Self {
        self.span = self.span.shifted(offset);
        self
    }

    pub fn message(&self) -> &'static str {
        match self.kind {
            DiagnosticKind::UnexpectedCharacter => "unexpected character",
            DiagnosticKind::UnexpectedToken => "unexpected input",
            DiagnosticKind::UnexpectedEnd => "expected more input",
            DiagnosticKind::InvalidNumber => "invalid number",
            DiagnosticKind::InvalidDate => "invalid date",
            DiagnosticKind::InvalidTime => "invalid time",
            DiagnosticKind::InvalidCalendarDuration => "invalid calendar duration",
            DiagnosticKind::MissingTemporalContext => "current date or time was not provided",
            DiagnosticKind::UnknownVariable => "unknown variable",
            DiagnosticKind::InvalidAssignment => "invalid variable assignment",
            DiagnosticKind::InvalidFunctionArguments => "invalid function arguments",
            DiagnosticKind::NumberTooLarge => "number is too large",
            DiagnosticKind::DivisionByZero => "cannot divide by zero",
            DiagnosticKind::ExponentTooLarge => "exponent is too large",
            DiagnosticKind::PrecisionTooLarge => "requested precision is too large",
            DiagnosticKind::RootDegreeTooLarge => "root degree is too large",
            DiagnosticKind::InvalidLogarithm => {
                "logarithm input and base must be positive, and the base cannot be one"
            }
            DiagnosticKind::InvalidTrigonometry => "value is outside the trigonometric domain",
            DiagnosticKind::InvalidHyperbolic => "value is outside the hyperbolic domain",
            DiagnosticKind::IntegerRequired => "operation requires exact integers",
            DiagnosticKind::InvalidCombinatorics => {
                "factorial and combinatorics require non-negative integers, with selection no greater than population"
            }
            DiagnosticKind::CombinatoricsTooLarge => "combinatorics input is too large",
            DiagnosticKind::NonRealResult => "result is not a real number",
            DiagnosticKind::UnsupportedOperation => "operation is not supported for these values",
            DiagnosticKind::UnitMismatch => "units have incompatible dimensions",
            DiagnosticKind::CurrencyMismatch => "currencies differ; convert one explicitly",
            DiagnosticKind::MissingExchangeRate => "no exchange rate was provided",
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message())?;
        if let Some(detail) = &self.detail {
            write!(formatter, ": {detail}")?;
        }
        Ok(())
    }
}

impl Error for Diagnostic {}
