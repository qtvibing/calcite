mod diagnostic;
mod evaluator;
mod formatter;
mod lexer;
mod number;
mod parser;
mod percentage;
mod span;

pub use diagnostic::{Diagnostic, DiagnosticKind};
pub use evaluator::{Value, evaluate};
pub use formatter::format;
pub use lexer::{Lexeme, LexemeKind, lex};
pub use number::{Number, NumberParseError};
pub use parser::{BinaryOperator, Expression, UnaryOperator, parse};
pub use percentage::Percentage;
pub use span::Span;

pub(crate) use number::NumericError;

#[derive(Clone, Debug, PartialEq)]
pub struct Calculation {
    pub value: Value,
    pub output: String,
}

pub fn calculate(source: &str) -> Result<Calculation, Diagnostic> {
    let value = evaluate(source)?;
    let output = format(&value);
    Ok(Calculation { value, output })
}
