use crate::{
    CalculationContext, Diagnostic, DiagnosticKind, Lexeme, LexemeKind, Number, NumberFormat,
    NumberParseError, Span, Unit, lex_with_number_format,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOperator {
    Positive,
    Negative,
    Not,
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
    Equal,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    And,
    Or,
}

impl BinaryOperator {
    const fn binding_power(self) -> (u8, u8) {
        match self {
            Self::Or => (1, 2),
            Self::And => (3, 4),
            Self::Equal
            | Self::NotEqual
            | Self::Greater
            | Self::GreaterEqual
            | Self::Less
            | Self::LessEqual => (5, 6),
            Self::Add | Self::Subtract => (7, 8),
            Self::Multiply | Self::Divide | Self::Of | Self::On | Self::Off => (9, 10),
            Self::Power => (11, 11),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConversionTarget {
    Number,
    Percentage,
    Fraction,
    Unit(Unit),
    Rate {
        numerator: Option<Unit>,
        denominator: Unit,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expression {
    Number {
        value: Number,
        span: Span,
    },
    Boolean {
        value: bool,
        span: Span,
    },
    Unit {
        value: Number,
        unit: Unit,
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
    Conversion {
        expression: Box<Expression>,
        target: ConversionTarget,
        operator_span: Span,
        target_span: Span,
        span: Span,
    },
    Conditional {
        condition: Box<Expression>,
        then_branch: Box<Expression>,
        else_branch: Box<Expression>,
        inverted: bool,
        keyword_span: Span,
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
            | Self::Boolean { span, .. }
            | Self::Unit { span, .. }
            | Self::Unary { span, .. }
            | Self::Group { span, .. }
            | Self::Percentage { span, .. }
            | Self::Conversion { span, .. }
            | Self::Conditional { span, .. }
            | Self::Binary { span, .. } => *span,
        }
    }
}

pub fn parse(source: &str) -> Result<Expression, Diagnostic> {
    parse_with_context(source, &CalculationContext::default())
}

pub fn parse_with_context(
    source: &str,
    context: &CalculationContext,
) -> Result<Expression, Diagnostic> {
    let tokens = lex_with_number_format(source, context.number_format)?
        .into_iter()
        .filter(|token| token.kind != LexemeKind::Whitespace)
        .collect();
    let mut parser = Parser {
        source,
        tokens,
        position: 0,
        number_format: context.number_format,
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
    number_format: NumberFormat,
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

            if minimum_binding_power == 0 && self.is_conversion_operator() {
                left = self.parse_conversion(left)?;
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
                let normalized = self.number_format.normalize(source).ok_or_else(|| {
                    Diagnostic::with_detail(DiagnosticKind::InvalidNumber, token.span, source)
                })?;
                let value = Number::parse_decimal(&normalized).map_err(|error| match error {
                    NumberParseError::Invalid => {
                        Diagnostic::with_detail(DiagnosticKind::InvalidNumber, token.span, source)
                    }
                    NumberParseError::TooLarge => {
                        Diagnostic::new(DiagnosticKind::NumberTooLarge, token.span)
                    }
                })?;
                let (value, span) = if let Some((factor, scalar)) = self.scalar() {
                    self.position += 1;
                    (
                        value.multiply(Number::from_i64(factor)),
                        token.span.join(scalar.span),
                    )
                } else {
                    (value, token.span)
                };
                if let Some((unit, unit_token)) = self.unit() {
                    self.position += 1;
                    Ok(Expression::Unit {
                        value,
                        unit,
                        span: span.join(unit_token.span),
                    })
                } else {
                    Ok(Expression::Number { value, span })
                }
            }
            LexemeKind::Word if self.unit().is_some() => {
                let (unit, token) = self.unit().expect("guard checked the unit");
                self.position += 1;
                Ok(Expression::Unit {
                    value: Number::from_i64(1),
                    unit,
                    span: token.span,
                })
            }
            LexemeKind::Word if self.word_is(0, "if") || self.word_is(0, "unless") => {
                self.parse_conditional()
            }
            LexemeKind::Word if self.word_is(0, "true") || self.word_is(0, "false") => {
                self.position += 1;
                Ok(Expression::Boolean {
                    value: self.source[token.span.start..token.span.end]
                        .eq_ignore_ascii_case("true"),
                    span: token.span,
                })
            }
            LexemeKind::Plus | LexemeKind::Minus | LexemeKind::LogicalNot => {
                self.position += 1;
                let (operator, binding_power) = match token.kind {
                    LexemeKind::Plus => (UnaryOperator::Positive, 11),
                    LexemeKind::Minus => (UnaryOperator::Negative, 11),
                    LexemeKind::LogicalNot => (UnaryOperator::Not, 5),
                    _ => unreachable!(),
                };
                let operand = self.parse_expression(binding_power)?;
                let span = token.span.join(operand.span());
                Ok(Expression::Unary {
                    operator,
                    operator_span: token.span,
                    operand: Box::new(operand),
                    span,
                })
            }
            LexemeKind::Word if self.word_is(0, "not") => {
                self.position += 1;
                let operand = self.parse_expression(5)?;
                let span = token.span.join(operand.span());
                Ok(Expression::Unary {
                    operator: UnaryOperator::Not,
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

    fn parse_conditional(&mut self) -> Result<Expression, Diagnostic> {
        let keyword = self.current().expect("conditional keyword is present");
        let inverted = self.word_is(0, "unless");
        self.position += 1;

        let condition = self.parse_expression(0)?;
        self.expect_word("then")?;
        self.position += 1;

        let then_branch = self.parse_expression(0)?;
        self.expect_word("else")?;
        self.position += 1;

        let else_branch = self.parse_expression(0)?;
        let span = keyword.span.join(else_branch.span());
        Ok(Expression::Conditional {
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
            inverted,
            keyword_span: keyword.span,
            span,
        })
    }

    fn expect_word(&self, expected: &str) -> Result<Lexeme, Diagnostic> {
        let Some(token) = self.current() else {
            return Err(Diagnostic::with_detail(
                DiagnosticKind::UnexpectedEnd,
                Span::empty(self.source.len()),
                format!("expected '{expected}'"),
            ));
        };
        if !self.word_is(0, expected) {
            return Err(Diagnostic::with_detail(
                DiagnosticKind::UnexpectedToken,
                token.span,
                format!("expected '{expected}'"),
            ));
        }
        Ok(token)
    }

    fn is_conversion_operator(&self) -> bool {
        self.word_is(0, "as")
            || self.word_is(0, "in")
            || (self.word_is(0, "to") && !self.word_is(1, "the"))
    }

    fn parse_conversion(&mut self, expression: Expression) -> Result<Expression, Diagnostic> {
        let operator = self.current().expect("conversion operator is present");
        let Some(target_token) = self.tokens.get(self.position + 1).copied() else {
            return Err(Diagnostic::with_detail(
                DiagnosticKind::UnexpectedEnd,
                Span::empty(self.source.len()),
                "expected a conversion target",
            ));
        };

        let mut consumed = 2;
        let mut target_span = target_token.span;
        let target = if target_token.kind == LexemeKind::Slash {
            let (denominator, token) = self.unit_at(2).ok_or_else(|| {
                Diagnostic::with_detail(
                    DiagnosticKind::UnexpectedToken,
                    self.tokens
                        .get(self.position + 2)
                        .map_or(Span::empty(self.source.len()), |token| token.span),
                    "expected a rate denominator",
                )
            })?;
            consumed = 3;
            target_span = target_span.join(token.span);
            ConversionTarget::Rate {
                numerator: None,
                denominator,
            }
        } else if let Some((numerator, _)) = self.unit_at(1) {
            if self
                .tokens
                .get(self.position + 2)
                .is_some_and(|token| token.kind == LexemeKind::Slash)
                || self.word_is(2, "per")
            {
                let (denominator, token) = self.unit_at(3).ok_or_else(|| {
                    Diagnostic::with_detail(
                        DiagnosticKind::UnexpectedToken,
                        self.tokens
                            .get(self.position + 3)
                            .map_or(Span::empty(self.source.len()), |token| token.span),
                        "expected a rate denominator",
                    )
                })?;
                consumed = 4;
                target_span = target_span.join(token.span);
                ConversionTarget::Rate {
                    numerator: Some(numerator),
                    denominator,
                }
            } else {
                ConversionTarget::Unit(numerator)
            }
        } else {
            match target_token.kind {
                LexemeKind::Percent => ConversionTarget::Percentage,
                LexemeKind::Word => {
                    let target = &self.source[target_token.span.start..target_token.span.end];
                    if target.eq_ignore_ascii_case("number")
                        || target.eq_ignore_ascii_case("decimal")
                    {
                        ConversionTarget::Number
                    } else if target.eq_ignore_ascii_case("percent")
                        || target.eq_ignore_ascii_case("percentage")
                    {
                        ConversionTarget::Percentage
                    } else if target.eq_ignore_ascii_case("fraction") {
                        ConversionTarget::Fraction
                    } else {
                        return Err(Diagnostic::with_detail(
                            DiagnosticKind::UnexpectedToken,
                            target_token.span,
                            "expected a unit, rate, number, percentage, or fraction",
                        ));
                    }
                }
                _ => {
                    return Err(Diagnostic::with_detail(
                        DiagnosticKind::UnexpectedToken,
                        target_token.span,
                        "expected a unit, rate, number, percentage, or fraction",
                    ));
                }
            }
        };

        self.position += consumed;
        let span = expression.span().join(target_span);
        Ok(Expression::Conversion {
            expression: Box::new(expression),
            target,
            operator_span: operator.span,
            target_span,
            span,
        })
    }

    fn unit(&self) -> Option<(Unit, Lexeme)> {
        self.unit_at(0)
    }

    fn unit_at(&self, offset: usize) -> Option<(Unit, Lexeme)> {
        let token = *self.tokens.get(self.position + offset)?;
        if token.kind != LexemeKind::Word {
            return None;
        }
        let source = &self.source[token.span.start..token.span.end];
        Unit::from_alias(source).map(|unit| (unit, token))
    }

    fn scalar(&self) -> Option<(i64, Lexeme)> {
        let token = self.current()?;
        if token.kind != LexemeKind::Word {
            return None;
        }
        let text = &self.source[token.span.start..token.span.end];
        let is_adjacent =
            self.position > 0 && self.tokens[self.position - 1].span.end == token.span.start;

        let factor = if is_adjacent {
            match text {
                "k" | "K" => Some(1_000),
                "M" => Some(1_000_000),
                "B" | "G" => Some(1_000_000_000),
                "T" => Some(1_000_000_000_000),
                _ => None,
            }
        } else {
            None
        }
        .or_else(|| {
            if text.eq_ignore_ascii_case("thousand") {
                Some(1_000)
            } else if text.eq_ignore_ascii_case("million") {
                Some(1_000_000)
            } else if text.eq_ignore_ascii_case("billion") {
                Some(1_000_000_000)
            } else if text.eq_ignore_ascii_case("trillion") {
                Some(1_000_000_000_000)
            } else {
                None
            }
        })?;

        Some((factor, token))
    }

    fn binary_operator(&self) -> Option<(BinaryOperator, Span, usize, u8, u8)> {
        let current = self.current()?;
        let (operator, span, consumed) = match current.kind {
            LexemeKind::Plus => (BinaryOperator::Add, current.span, 1),
            LexemeKind::Minus => (BinaryOperator::Subtract, current.span, 1),
            LexemeKind::Star => (BinaryOperator::Multiply, current.span, 1),
            LexemeKind::Slash => (BinaryOperator::Divide, current.span, 1),
            LexemeKind::Caret => (BinaryOperator::Power, current.span, 1),
            LexemeKind::EqualEqual => (BinaryOperator::Equal, current.span, 1),
            LexemeKind::NotEqual => (BinaryOperator::NotEqual, current.span, 1),
            LexemeKind::Greater => (BinaryOperator::Greater, current.span, 1),
            LexemeKind::GreaterEqual => (BinaryOperator::GreaterEqual, current.span, 1),
            LexemeKind::Less => (BinaryOperator::Less, current.span, 1),
            LexemeKind::LessEqual => (BinaryOperator::LessEqual, current.span, 1),
            LexemeKind::LogicalAnd => (BinaryOperator::And, current.span, 1),
            LexemeKind::LogicalOr => (BinaryOperator::Or, current.span, 1),
            LexemeKind::Word if self.word_is(0, "plus") => (BinaryOperator::Add, current.span, 1),
            LexemeKind::Word if self.word_is(0, "minus") => {
                (BinaryOperator::Subtract, current.span, 1)
            }
            LexemeKind::Word if self.word_is(0, "times") => {
                (BinaryOperator::Multiply, current.span, 1)
            }
            LexemeKind::Word if self.word_is(0, "multiplied") => {
                let consumed = if self.word_is(1, "by") { 2 } else { 1 };
                (
                    BinaryOperator::Multiply,
                    self.phrase_span(consumed),
                    consumed,
                )
            }
            LexemeKind::Word if self.word_is(0, "divided") => {
                let consumed = if self.word_is(1, "by") { 2 } else { 1 };
                (BinaryOperator::Divide, self.phrase_span(consumed), consumed)
            }
            LexemeKind::Word if self.word_is(0, "over") || self.word_is(0, "per") => {
                (BinaryOperator::Divide, current.span, 1)
            }
            LexemeKind::Word if self.word_is(0, "of") => (BinaryOperator::Of, current.span, 1),
            LexemeKind::Word if self.word_is(0, "on") => (BinaryOperator::On, current.span, 1),
            LexemeKind::Word if self.word_is(0, "off") => (BinaryOperator::Off, current.span, 1),
            LexemeKind::Word if self.word_is(0, "and") => (BinaryOperator::And, current.span, 1),
            LexemeKind::Word if self.word_is(0, "or") => (BinaryOperator::Or, current.span, 1),
            LexemeKind::Word
                if self.word_is(0, "not") && self.word_is(1, "equal") && self.word_is(2, "to") =>
            {
                (BinaryOperator::NotEqual, self.phrase_span(3), 3)
            }
            LexemeKind::Word
                if self.word_is(0, "greater")
                    && self.word_is(1, "than")
                    && self.word_is(2, "or")
                    && self.word_is(3, "equal")
                    && self.word_is(4, "to") =>
            {
                (BinaryOperator::GreaterEqual, self.phrase_span(5), 5)
            }
            LexemeKind::Word
                if self.word_is(0, "less")
                    && self.word_is(1, "than")
                    && self.word_is(2, "or")
                    && self.word_is(3, "equal")
                    && self.word_is(4, "to") =>
            {
                (BinaryOperator::LessEqual, self.phrase_span(5), 5)
            }
            LexemeKind::Word if self.word_is(0, "greater") && self.word_is(1, "than") => {
                (BinaryOperator::Greater, self.phrase_span(2), 2)
            }
            LexemeKind::Word if self.word_is(0, "less") && self.word_is(1, "than") => {
                (BinaryOperator::Less, self.phrase_span(2), 2)
            }
            LexemeKind::Word if self.word_is(0, "equal") && self.word_is(1, "to") => {
                (BinaryOperator::Equal, self.phrase_span(2), 2)
            }
            LexemeKind::Word if self.word_is(0, "equals") => {
                (BinaryOperator::Equal, current.span, 1)
            }
            LexemeKind::Word if self.word_is(0, "at") && self.word_is(1, "least") => {
                (BinaryOperator::GreaterEqual, self.phrase_span(2), 2)
            }
            LexemeKind::Word if self.word_is(0, "at") && self.word_is(1, "most") => {
                (BinaryOperator::LessEqual, self.phrase_span(2), 2)
            }
            LexemeKind::Word
                if self.word_is(0, "to")
                    && self.word_is(1, "the")
                    && self.word_is(2, "power")
                    && self.word_is(3, "of") =>
            {
                (BinaryOperator::Power, self.phrase_span(4), 4)
            }
            _ => return None,
        };
        let (left_power, right_power) = operator.binding_power();
        Some((operator, span, consumed, left_power, right_power))
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
