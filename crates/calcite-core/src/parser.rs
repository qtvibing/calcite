use crate::{Diagnostic, DiagnosticKind, Lexeme, LexemeKind, Number, NumberParseError, Span, lex};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOperator {
    Positive,
    Negative,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Power,
    Of,
    On,
    Off,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expression {
    Number {
        value: Number,
        span: Span,
    },
    Unary {
        operator: UnaryOperator,
        operator_span: Span,
        operand: Box<Expression>,
        span: Span,
    },
    Group {
        expression: Box<Expression>,
        span: Span,
    },
    Percentage {
        expression: Box<Expression>,
        operator_span: Span,
        span: Span,
    },
    Binary {
        operator: BinaryOperator,
        operator_span: Span,
        left: Box<Expression>,
        right: Box<Expression>,
        span: Span,
    },
}

impl Expression {
    pub const fn span(&self) -> Span {
        match self {
            Self::Number { span, .. }
            | Self::Unary { span, .. }
            | Self::Group { span, .. }
            | Self::Percentage { span, .. }
            | Self::Binary { span, .. } => *span,
        }
    }
}

pub fn parse(source: &str) -> Result<Expression, Diagnostic> {
    let tokens = lex(source)?
        .into_iter()
        .filter(|token| token.kind != LexemeKind::Whitespace)
        .collect();
    let mut parser = Parser {
        source,
        tokens,
        position: 0,
    };

    let expression = parser.parse_expression(0)?;
    if let Some(token) = parser.current() {
        return Err(parser.unexpected(token));
    }
    Ok(expression)
}

struct Parser<'a> {
    source: &'a str,
    tokens: Vec<Lexeme>,
    position: usize,
}

impl Parser<'_> {
    fn parse_expression(&mut self, minimum_binding_power: u8) -> Result<Expression, Diagnostic> {
        let mut left = self.parse_prefix()?;

        loop {
            if let Some(percent) = self
                .current()
                .filter(|token| token.kind == LexemeKind::Percent)
            {
                self.position += 1;
                let span = left.span().join(percent.span);
                left = Expression::Percentage {
                    expression: Box::new(left),
                    operator_span: percent.span,
                    span,
                };
                continue;
            }

            let Some((operator, operator_span, consumed, left_power, right_power)) =
                self.binary_operator()
            else {
                break;
            };
            if left_power < minimum_binding_power {
                break;
            }
            self.position += consumed;
            let right = self.parse_expression(right_power)?;
            let span = left.span().join(right.span());
            left = Expression::Binary {
                operator,
                operator_span,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    fn parse_prefix(&mut self) -> Result<Expression, Diagnostic> {
        let Some(token) = self.current() else {
            return Err(Diagnostic::new(
                DiagnosticKind::UnexpectedEnd,
                Span::empty(self.source.len()),
            ));
        };

        match token.kind {
            LexemeKind::Number => {
                self.position += 1;
                let source = &self.source[token.span.start..token.span.end];
                let value = Number::parse_decimal(source).map_err(|error| match error {
                    NumberParseError::Invalid => {
                        Diagnostic::with_detail(DiagnosticKind::InvalidNumber, token.span, source)
                    }
                    NumberParseError::TooLarge => {
                        Diagnostic::new(DiagnosticKind::NumberTooLarge, token.span)
                    }
                })?;
                Ok(Expression::Number {
                    value,
                    span: token.span,
                })
            }
            LexemeKind::Plus | LexemeKind::Minus => {
                self.position += 1;
                let operator = if token.kind == LexemeKind::Plus {
                    UnaryOperator::Positive
                } else {
                    UnaryOperator::Negative
                };
                let operand = self.parse_expression(5)?;
                let span = token.span.join(operand.span());
                Ok(Expression::Unary {
                    operator,
                    operator_span: token.span,
                    operand: Box::new(operand),
                    span,
                })
            }
            LexemeKind::LeftParenthesis => {
                self.position += 1;
                let expression = self.parse_expression(0)?;
                let Some(closing) = self.current() else {
                    return Err(Diagnostic::with_detail(
                        DiagnosticKind::UnexpectedEnd,
                        Span::empty(self.source.len()),
                        "expected ')'",
                    ));
                };
                if closing.kind != LexemeKind::RightParenthesis {
                    return Err(Diagnostic::with_detail(
                        DiagnosticKind::UnexpectedToken,
                        closing.span,
                        "expected ')'",
                    ));
                }
                self.position += 1;
                Ok(Expression::Group {
                    span: token.span.join(closing.span),
                    expression: Box::new(expression),
                })
            }
            _ => Err(self.unexpected(token)),
        }
    }

    fn binary_operator(&self) -> Option<(BinaryOperator, Span, usize, u8, u8)> {
        let current = self.current()?;
        let result = match current.kind {
            LexemeKind::Plus => (BinaryOperator::Add, current.span, 1, 1, 2),
            LexemeKind::Minus => (BinaryOperator::Subtract, current.span, 1, 1, 2),
            LexemeKind::Star => (BinaryOperator::Multiply, current.span, 1, 3, 4),
            LexemeKind::Slash => (BinaryOperator::Divide, current.span, 1, 3, 4),
            LexemeKind::Caret => (BinaryOperator::Power, current.span, 1, 5, 5),
            LexemeKind::Word if self.word_is(0, "plus") => {
                (BinaryOperator::Add, current.span, 1, 1, 2)
            }
            LexemeKind::Word if self.word_is(0, "minus") => {
                (BinaryOperator::Subtract, current.span, 1, 1, 2)
            }
            LexemeKind::Word if self.word_is(0, "times") => {
                (BinaryOperator::Multiply, current.span, 1, 3, 4)
            }
            LexemeKind::Word if self.word_is(0, "multiplied") => {
                let consumed = if self.word_is(1, "by") { 2 } else { 1 };
                (
                    BinaryOperator::Multiply,
                    self.phrase_span(consumed),
                    consumed,
                    3,
                    4,
                )
            }
            LexemeKind::Word if self.word_is(0, "divided") => {
                let consumed = if self.word_is(1, "by") { 2 } else { 1 };
                (
                    BinaryOperator::Divide,
                    self.phrase_span(consumed),
                    consumed,
                    3,
                    4,
                )
            }
            LexemeKind::Word if self.word_is(0, "over") => {
                (BinaryOperator::Divide, current.span, 1, 3, 4)
            }
            LexemeKind::Word if self.word_is(0, "of") => {
                (BinaryOperator::Of, current.span, 1, 3, 4)
            }
            LexemeKind::Word if self.word_is(0, "on") => {
                (BinaryOperator::On, current.span, 1, 3, 4)
            }
            LexemeKind::Word if self.word_is(0, "off") => {
                (BinaryOperator::Off, current.span, 1, 3, 4)
            }
            LexemeKind::Word
                if self.word_is(0, "to")
                    && self.word_is(1, "the")
                    && self.word_is(2, "power")
                    && self.word_is(3, "of") =>
            {
                (BinaryOperator::Power, self.phrase_span(4), 4, 5, 5)
            }
            _ => return None,
        };
        Some(result)
    }

    fn current(&self) -> Option<Lexeme> {
        self.tokens.get(self.position).copied()
    }

    fn word_is(&self, offset: usize, expected: &str) -> bool {
        self.tokens
            .get(self.position + offset)
            .filter(|token| token.kind == LexemeKind::Word)
            .is_some_and(|token| {
                self.source[token.span.start..token.span.end].eq_ignore_ascii_case(expected)
            })
    }

    fn phrase_span(&self, token_count: usize) -> Span {
        self.tokens[self.position]
            .span
            .join(self.tokens[self.position + token_count - 1].span)
    }

    fn unexpected(&self, token: Lexeme) -> Diagnostic {
        Diagnostic::with_detail(
            DiagnosticKind::UnexpectedToken,
            token.span,
            &self.source[token.span.start..token.span.end],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_is_right_associative() {
        let expression = parse("2 ^ 3 ^ 2").unwrap();
        let Expression::Binary { right, .. } = expression else {
            panic!("expected binary expression");
        };
        assert!(matches!(
            *right,
            Expression::Binary {
                operator: BinaryOperator::Power,
                ..
            }
        ));
    }

    #[test]
    fn power_binds_more_tightly_than_a_leading_sign() {
        let expression = parse("-2 ^ 2").unwrap();
        let Expression::Unary { operand, .. } = expression else {
            panic!("expected unary expression");
        };
        assert!(matches!(
            *operand,
            Expression::Binary {
                operator: BinaryOperator::Power,
                ..
            }
        ));
    }
}
