mod context;
mod diagnostic;
mod document;
mod evaluator;
mod formatter;
mod fraction;
mod lexer;
mod money;
mod number;
mod parser;
mod percentage;
mod radix;
mod rate;
mod span;
mod temporal;
mod unit;

pub use context::{CalculationContext, NumberFormat};
pub use diagnostic::{Diagnostic, DiagnosticKind};
pub use document::{
    DocumentCalculation, DocumentDependency, DocumentEntry, DocumentEntryKind, DocumentLine,
    calculate_document, calculate_document_with_context,
};
pub use evaluator::{Value, evaluate, evaluate_with_context};
pub use formatter::{format, format_with_context};
pub use fraction::Fraction;
pub use lexer::{Lexeme, LexemeKind, lex, lex_with_number_format};
pub use money::{Currency, ExchangeRates, InvalidExchangeRate, MonetaryContext, Money};
pub use number::{Number, NumberParseError};
pub use parser::{
    BinaryOperator, Constant, ConversionTarget, Expression, Function, RelativeWeekdayDirection,
    TemporalKeyword, UnaryOperator, parse, parse_with_context,
};
pub use percentage::Percentage;
pub use radix::{Radix, RadixInteger};
pub use rate::Rate;
pub use span::Span;
pub use temporal::{
    CalendarDuration, ClockTime, Date, OffsetDateTime, TemporalContext, UtcOffset, Weekday,
    WorkdayCalendar, WorkdayDuration,
};
pub use unit::{Dimension, Unit, UnitValue};

pub(crate) use number::NumericError;

#[derive(Clone, Debug, PartialEq)]
pub struct Calculation {
    pub value: Value,
    pub output: String,
}

pub fn calculate(source: &str) -> Result<Calculation, Diagnostic> {
    calculate_with_context(source, &CalculationContext::default())
}

pub fn calculate_with_context(
    source: &str,
    context: &CalculationContext,
) -> Result<Calculation, Diagnostic> {
    let value = evaluate_with_context(source, context)?;
    let output = format_with_context(&value, context);
    Ok(Calculation { value, output })
}
