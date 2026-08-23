mod assignment;
mod complex;
mod context;
mod crypto;
mod currency;
mod diagnostic;
mod dice;
mod document;
mod evaluator;
mod finance;
mod formatter;
mod fraction;
mod functions;
mod lexer;
mod money;
mod money_rate;
mod number;
mod operations;
mod parser;
mod percentage;
mod price_index;
mod radix;
mod rate;
mod roman;
mod session;
mod span;
mod syntax;
mod temporal;
mod timezone;
mod unit;
mod user_function;

pub use complex::ComplexNumber;
pub use context::{CalculationContext, NumberFormat, NumberOutputMode, OutputFormat};
pub use crypto::{
    CryptoAmount, CryptoAsset, CryptoError, CryptoQuote, CryptoQuoteFreshness, CryptoQuoteSnapshot,
};
pub use currency::Currency;
pub use diagnostic::{Diagnostic, DiagnosticKind};
pub use dice::{CoinSide, DiceDistribution, DiceError, EntropySource, flip_coin};
pub use document::{
    DocumentCalculation, DocumentDependency, DocumentEntry, DocumentEntryKind, DocumentLine,
    calculate_document, calculate_document_with_context,
};
pub use evaluator::{Value, evaluate, evaluate_with_context};
pub use finance::{AmortizationPayment, AmortizationSchedule, FinanceError, amortization_schedule};
pub use formatter::{format, format_with_context};
pub use fraction::Fraction;
pub use lexer::{Lexeme, LexemeKind, lex, lex_with_number_format};
pub use money::{
    ExchangeRateFreshness, ExchangeRateLookupError, ExchangeRateSnapshot, ExchangeRates,
    InvalidExchangeRate, MonetaryContext, Money,
};
pub use money_rate::MoneyRate;
pub use number::{Number, NumberParseError};
pub use parser::{parse, parse_with_context};
pub use percentage::Percentage;
pub use price_index::{PriceIndexError, PriceIndexSeries, PriceIndexSnapshot};
pub use radix::{Radix, RadixInteger};
pub use rate::Rate;
pub use roman::RomanInteger;
pub use session::CalculationSession;
pub use span::Span;
pub use syntax::{
    BinaryOperator, Constant, ConversionTarget, Expression, Function, RelativeWeekdayDirection,
    TemporalKeyword, UnaryOperator,
};
pub use temporal::{
    CalendarDuration, CalendarSpan, ClockTime, Date, DateOrder, HolidayCalendarError,
    LEAP_SECOND_POLICY, LeapSecondPolicy, OffsetClockTime, OffsetDateTime, TemporalContext,
    TimeSpan, UtcOffset, Weekday, WorkdayCalendar, WorkdayDuration,
};
pub use timezone::{
    LocalTimeResolution, TimeZoneDefinition, TimeZoneError, TimeZoneSnapshot, TimeZoneTransition,
    ZonedClockTime, ZonedDateTime,
};
pub use unit::{
    CompoundUnit, CustomUnitDefinition, CustomUnitError, CustomUnitRegistry, CustomUnitValue,
    Dimension, Unit, UnitValue,
};
pub use user_function::UserFunction;

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
