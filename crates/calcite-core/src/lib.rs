mod context;
mod diagnostic;
mod evaluator;
mod formatter;
mod fraction;
mod lexer;
mod number;
mod parser;
mod percentage;
mod span;
mod unit;

pub use context::{CalculationContext, NumberFormat};
pub use diagnostic::{Diagnostic, DiagnosticKind};
pub use evaluator::{Value, evaluate, evaluate_with_context};
pub use formatter::{format, format_with_context};
pub use fraction::Fraction;
pub use lexer::{Lexeme, LexemeKind, lex, lex_with_number_format};
pub use number::{Number, NumberParseError};
pub use parser::{
    BinaryOperator, ConversionTarget, Expression, UnaryOperator, parse, parse_with_context,
};
pub use percentage::Percentage;
pub use span::Span;
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
