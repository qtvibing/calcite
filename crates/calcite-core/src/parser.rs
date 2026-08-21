use crate::{
    CalculationContext, CalendarDuration, ClockTime, Currency, Date, Diagnostic, DiagnosticKind,
    Lexeme, LexemeKind, Number, NumberFormat, NumberParseError, OffsetDateTime, Span, Unit,
    UtcOffset, lex_with_number_format,
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
    At,
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
            Self::Multiply | Self::Divide | Self::Of | Self::On | Self::Off | Self::At => (9, 10),
            Self::Power => (11, 11),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Function {
    SquareRoot,
    CubicRoot,
    NthRoot,
    NaturalLogarithm,
    CommonLogarithm,
    BinaryLogarithm,
    Sine,
    Cosine,
    Tangent,
    ArcSine,
    ArcCosine,
    ArcTangent,
    ArcTangent2,
    Exponential,
    HyperbolicSine,
    HyperbolicCosine,
    HyperbolicTangent,
    InverseHyperbolicSine,
    InverseHyperbolicCosine,
    InverseHyperbolicTangent,
    AbsoluteValue,
    Sum,
    Average,
    Minimum,
    Maximum,
    Median,
    Count,
    Product,
    Round,
    Floor,
    Ceiling,
}

impl Function {
    fn from_name(name: &str) -> Option<Self> {
        if name.eq_ignore_ascii_case("sqrt") {
            Some(Self::SquareRoot)
        } else if name.eq_ignore_ascii_case("cbrt") {
            Some(Self::CubicRoot)
        } else if name.eq_ignore_ascii_case("root") {
            Some(Self::NthRoot)
        } else if name.eq_ignore_ascii_case("ln") {
            Some(Self::NaturalLogarithm)
        } else if name.eq_ignore_ascii_case("log")
            || name.eq_ignore_ascii_case("log10")
            || name.eq_ignore_ascii_case("logarithm")
        {
            Some(Self::CommonLogarithm)
        } else if name.eq_ignore_ascii_case("log2") {
            Some(Self::BinaryLogarithm)
        } else if name.eq_ignore_ascii_case("sin") || name.eq_ignore_ascii_case("sine") {
            Some(Self::Sine)
        } else if name.eq_ignore_ascii_case("cos") || name.eq_ignore_ascii_case("cosine") {
            Some(Self::Cosine)
        } else if name.eq_ignore_ascii_case("tan") || name.eq_ignore_ascii_case("tangent") {
            Some(Self::Tangent)
        } else if name.eq_ignore_ascii_case("asin") || name.eq_ignore_ascii_case("arcsine") {
            Some(Self::ArcSine)
        } else if name.eq_ignore_ascii_case("acos") || name.eq_ignore_ascii_case("arccosine") {
            Some(Self::ArcCosine)
        } else if name.eq_ignore_ascii_case("atan") || name.eq_ignore_ascii_case("arctangent") {
            Some(Self::ArcTangent)
        } else if name.eq_ignore_ascii_case("atan2") {
            Some(Self::ArcTangent2)
        } else if name.eq_ignore_ascii_case("exp") || name.eq_ignore_ascii_case("exponential") {
            Some(Self::Exponential)
        } else if name.eq_ignore_ascii_case("sinh") {
            Some(Self::HyperbolicSine)
        } else if name.eq_ignore_ascii_case("cosh") {
            Some(Self::HyperbolicCosine)
        } else if name.eq_ignore_ascii_case("tanh") {
            Some(Self::HyperbolicTangent)
        } else if name.eq_ignore_ascii_case("asinh") {
            Some(Self::InverseHyperbolicSine)
        } else if name.eq_ignore_ascii_case("acosh") {
            Some(Self::InverseHyperbolicCosine)
        } else if name.eq_ignore_ascii_case("atanh") {
            Some(Self::InverseHyperbolicTangent)
        } else if name.eq_ignore_ascii_case("abs") {
            Some(Self::AbsoluteValue)
        } else if name.eq_ignore_ascii_case("sum") {
            Some(Self::Sum)
        } else if name.eq_ignore_ascii_case("average") || name.eq_ignore_ascii_case("avg") {
            Some(Self::Average)
        } else if name.eq_ignore_ascii_case("minimum") || name.eq_ignore_ascii_case("min") {
            Some(Self::Minimum)
        } else if name.eq_ignore_ascii_case("maximum") || name.eq_ignore_ascii_case("max") {
            Some(Self::Maximum)
        } else if name.eq_ignore_ascii_case("median") {
            Some(Self::Median)
        } else if name.eq_ignore_ascii_case("count") {
            Some(Self::Count)
        } else if name.eq_ignore_ascii_case("product") {
            Some(Self::Product)
        } else if name.eq_ignore_ascii_case("round") {
            Some(Self::Round)
        } else if name.eq_ignore_ascii_case("floor") {
            Some(Self::Floor)
        } else if name.eq_ignore_ascii_case("ceiling") || name.eq_ignore_ascii_case("ceil") {
            Some(Self::Ceiling)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Constant {
    Pi,
    Tau,
    E,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TemporalKeyword {
    Today,
    Tomorrow,
    Yesterday,
    Now,
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
    UtcOffset(UtcOffset),
    Currency(Currency),
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
    Constant {
        constant: Constant,
        span: Span,
    },
    Unit {
        value: Number,
        unit: Unit,
        span: Span,
    },
    Money {
        amount: Number,
        currency: Currency,
        span: Span,
    },
    Date {
        value: Date,
        span: Span,
    },
    Clock {
        value: ClockTime,
        span: Span,
    },
    OffsetDateTime {
        value: OffsetDateTime,
        span: Span,
    },
    CalendarDuration {
        value: CalendarDuration,
        span: Span,
    },
    TemporalKeyword {
        keyword: TemporalKeyword,
        span: Span,
    },
    Variable {
        name: String,
        span: Span,
    },
    Call {
        function: Function,
        function_span: Span,
        arguments: Vec<Expression>,
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
            | Self::Constant { span, .. }
            | Self::Unit { span, .. }
            | Self::Money { span, .. }
            | Self::Date { span, .. }
            | Self::Clock { span, .. }
            | Self::OffsetDateTime { span, .. }
            | Self::CalendarDuration { span, .. }
            | Self::TemporalKeyword { span, .. }
            | Self::Variable { span, .. }
            | Self::Call { span, .. }
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
        default_currency: context.monetary.default_currency,
    };

    let expression = parser.parse_expression(0)?;
    if let Some(token) = parser.current() {
        if token.kind == LexemeKind::Comma {
            return Err(Diagnostic::new(DiagnosticKind::InvalidNumber, token.span));
        }
        return Err(parser.unexpected(token));
    }
    Ok(expression)
}

struct Parser<'a> {
    source: &'a str,
    tokens: Vec<Lexeme>,
    position: usize,
    number_format: NumberFormat,
    default_currency: Currency,
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
                if let Some(expression) = self.parse_iso_offset_datetime() {
                    return Ok(expression);
                }
                if self.starts_iso_datetime() {
                    let start = token.span.start;
                    let length = self.source[start..]
                        .find(char::is_whitespace)
                        .unwrap_or(self.source.len() - start);
                    return Err(Diagnostic::new(
                        DiagnosticKind::InvalidTime,
                        Span::new(start, start + length),
                    ));
                }
                if self.is_iso_date() {
                    return self.parse_iso_date();
                }
                if self.is_day_first_named_date() {
                    return self.parse_named_date(false);
                }
                if self.is_clock_time() {
                    return self.parse_clock_time();
                }
                self.position += 1;
                let value = self.parse_number_value(token)?;
                let (value, span) = if let Some((factor, scalar)) = self.scalar() {
                    self.position += 1;
                    (
                        value.multiply(Number::from_i64(factor)),
                        token.span.join(scalar.span),
                    )
                } else {
                    (value, token.span)
                };
                if let Some((currency, currency_token)) = self.currency_at(0) {
                    self.position += 1;
                    Ok(Expression::Money {
                        amount: value,
                        currency,
                        span: span.join(currency_token.span),
                    })
                } else if let Some((months_per_unit, calendar_token)) = self.calendar_unit() {
                    let count = value.exact_i64().ok_or_else(|| {
                        Diagnostic::new(DiagnosticKind::InvalidCalendarDuration, span)
                    })?;
                    let months = count.checked_mul(months_per_unit).ok_or_else(|| {
                        Diagnostic::new(DiagnosticKind::InvalidCalendarDuration, span)
                    })?;
                    self.position += 1;
                    Ok(Expression::CalendarDuration {
                        value: CalendarDuration::months(months),
                        span: span.join(calendar_token.span),
                    })
                } else if let Some((unit, unit_token)) = self.unit() {
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
            LexemeKind::CurrencySymbol => self.parse_prefixed_money(),
            LexemeKind::Word if self.currency_at(0).is_some() => self.parse_prefixed_money(),
            LexemeKind::Word if self.constant_at().is_some() => {
                let constant = self.constant_at().expect("guard checked the constant");
                self.position += 1;
                Ok(Expression::Constant {
                    constant,
                    span: token.span,
                })
            }
            LexemeKind::Word if self.function_call_at().is_some() => self.parse_function_call(),
            LexemeKind::Word if self.natural_function_at().is_some() => {
                self.parse_natural_function()
            }
            LexemeKind::Word if self.month_at(0).is_some() => self.parse_named_date(true),
            LexemeKind::Word if self.temporal_keyword().is_some() => {
                let keyword = self.temporal_keyword().expect("guard checked the keyword");
                self.position += 1;
                Ok(Expression::TemporalKeyword {
                    keyword,
                    span: token.span,
                })
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
            LexemeKind::Word => {
                self.position += 1;
                Ok(Expression::Variable {
                    name: self.source[token.span.start..token.span.end].to_owned(),
                    span: token.span,
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

    fn constant_at(&self) -> Option<Constant> {
        let token = self.current()?;
        if token.kind != LexemeKind::Word {
            return None;
        }
        let name = &self.source[token.span.start..token.span.end];
        if name.eq_ignore_ascii_case("pi") {
            Some(Constant::Pi)
        } else if name.eq_ignore_ascii_case("tau") {
            Some(Constant::Tau)
        } else if name == "e" {
            Some(Constant::E)
        } else {
            None
        }
    }

    fn function_call_at(&self) -> Option<Function> {
        let token = self.current()?;
        if token.kind != LexemeKind::Word
            || self.tokens.get(self.position + 1)?.kind != LexemeKind::LeftParenthesis
        {
            return None;
        }
        Function::from_name(&self.source[token.span.start..token.span.end])
    }

    fn natural_function_at(&self) -> Option<(Function, usize)> {
        if self.word_is(0, "square") && self.word_is(1, "root") && self.word_is(2, "of") {
            Some((Function::SquareRoot, 3))
        } else if self.word_is(0, "cube") && self.word_is(1, "root") && self.word_is(2, "of") {
            Some((Function::CubicRoot, 3))
        } else if self.word_is(0, "natural")
            && (self.word_is(1, "log") || self.word_is(1, "logarithm"))
            && self.word_is(2, "of")
        {
            Some((Function::NaturalLogarithm, 3))
        } else if self.word_is(0, "common")
            && (self.word_is(1, "log") || self.word_is(1, "logarithm"))
            && self.word_is(2, "of")
        {
            Some((Function::CommonLogarithm, 3))
        } else if self.word_is(0, "binary")
            && (self.word_is(1, "log") || self.word_is(1, "logarithm"))
            && self.word_is(2, "of")
        {
            Some((Function::BinaryLogarithm, 3))
        } else if self.word_is(0, "exponential") && self.word_is(1, "of") {
            Some((Function::Exponential, 2))
        } else if self.word_is(0, "hyperbolic")
            && self.word_is(2, "of")
            && (self.word_is(1, "sine") || self.word_is(1, "cosine") || self.word_is(1, "tangent"))
        {
            let function = if self.word_is(1, "sine") {
                Function::HyperbolicSine
            } else if self.word_is(1, "cosine") {
                Function::HyperbolicCosine
            } else {
                Function::HyperbolicTangent
            };
            Some((function, 3))
        } else if self.word_is(0, "absolute") && self.word_is(1, "value") && self.word_is(2, "of") {
            Some((Function::AbsoluteValue, 3))
        } else if self.word_is(1, "of") {
            let token = self.current()?;
            Function::from_name(&self.source[token.span.start..token.span.end])
                .map(|function| (function, 2))
        } else if self.word_is(0, "round") {
            Some((Function::Round, 1))
        } else {
            None
        }
    }

    fn parse_function_call(&mut self) -> Result<Expression, Diagnostic> {
        let function_token = self.current().expect("function name is present");
        let function = self
            .function_call_at()
            .expect("guard checked the function call");
        self.position += 2;
        let mut arguments = Vec::new();
        if !self
            .current()
            .is_some_and(|token| token.kind == LexemeKind::RightParenthesis)
        {
            loop {
                arguments.push(self.parse_expression(0)?);
                let Some(separator) = self.current() else {
                    return Err(Diagnostic::with_detail(
                        DiagnosticKind::UnexpectedEnd,
                        Span::empty(self.source.len()),
                        "expected ')'",
                    ));
                };
                if separator.kind == LexemeKind::Comma {
                    self.position += 1;
                    continue;
                }
                if separator.kind != LexemeKind::RightParenthesis {
                    return Err(Diagnostic::with_detail(
                        DiagnosticKind::UnexpectedToken,
                        separator.span,
                        "expected ',' or ')'",
                    ));
                }
                break;
            }
        }
        let closing = self.current().expect("closing parenthesis was checked");
        self.position += 1;
        Ok(Expression::Call {
            function,
            function_span: function_token.span,
            arguments,
            span: function_token.span.join(closing.span),
        })
    }

    fn parse_natural_function(&mut self) -> Result<Expression, Diagnostic> {
        let (function, consumed) = self
            .natural_function_at()
            .expect("guard checked the natural function");
        let function_span = self.phrase_span(consumed);
        self.position += consumed;
        let mut arguments = vec![self.parse_expression(0)?];
        let accepts_list = matches!(
            function,
            Function::Sum
                | Function::Average
                | Function::Minimum
                | Function::Maximum
                | Function::Median
                | Function::Count
                | Function::Product
        );
        while accepts_list
            && self
                .current()
                .is_some_and(|token| token.kind == LexemeKind::Comma)
        {
            self.position += 1;
            arguments.push(self.parse_expression(0)?);
        }
        let span = function_span.join(
            arguments
                .last()
                .expect("natural functions parse at least one argument")
                .span(),
        );
        Ok(Expression::Call {
            function,
            function_span,
            arguments,
            span,
        })
    }

    fn parse_number_value(&self, token: Lexeme) -> Result<Number, Diagnostic> {
        let source = &self.source[token.span.start..token.span.end];
        let normalized = self.number_format.normalize(source).ok_or_else(|| {
            Diagnostic::with_detail(DiagnosticKind::InvalidNumber, token.span, source)
        })?;
        Number::parse_decimal(&normalized).map_err(|error| match error {
            NumberParseError::Invalid => {
                Diagnostic::with_detail(DiagnosticKind::InvalidNumber, token.span, source)
            }
            NumberParseError::TooLarge => {
                Diagnostic::new(DiagnosticKind::NumberTooLarge, token.span)
            }
        })
    }

    fn parse_prefixed_money(&mut self) -> Result<Expression, Diagnostic> {
        let (currency, currency_token) = self
            .currency_at(0)
            .expect("money prefix is a recognized currency");
        self.position += 1;
        let Some(number_token) = self.current() else {
            return Err(Diagnostic::with_detail(
                DiagnosticKind::UnexpectedEnd,
                Span::empty(self.source.len()),
                "expected an amount after the currency",
            ));
        };
        if number_token.kind != LexemeKind::Number {
            return Err(Diagnostic::with_detail(
                DiagnosticKind::UnexpectedToken,
                number_token.span,
                "expected an amount after the currency",
            ));
        }
        let mut amount = self.parse_number_value(number_token)?;
        self.position += 1;
        let mut span = currency_token.span.join(number_token.span);
        if let Some((factor, scalar)) = self.scalar() {
            amount = amount.multiply(Number::from_i64(factor));
            span = span.join(scalar.span);
            self.position += 1;
        }
        Ok(Expression::Money {
            amount,
            currency,
            span,
        })
    }

    fn currency_at(&self, offset: usize) -> Option<(Currency, Lexeme)> {
        let token = *self.tokens.get(self.position + offset)?;
        let source = &self.source[token.span.start..token.span.end];
        let currency = match token.kind {
            LexemeKind::CurrencySymbol => match source {
                "$" => self.default_currency,
                "€" => Currency::Eur,
                "£" => Currency::Gbp,
                "¥" => Currency::Jpy,
                "₹" => Currency::Inr,
                "₩" => Currency::Krw,
                _ => return None,
            },
            LexemeKind::Word => Currency::from_code(source)?,
            _ => return None,
        };
        Some((currency, token))
    }

    fn starts_iso_datetime(&self) -> bool {
        self.is_iso_date()
            && self.source[self.tokens[self.position + 4].span.end..]
                .chars()
                .next()
                .is_some_and(|character| matches!(character, 'T' | 't'))
    }

    fn parse_iso_offset_datetime(&mut self) -> Option<Expression> {
        let start = self.current()?.span.start;
        let (value, length) = OffsetDateTime::parse_iso_prefix(&self.source[start..])?;
        let end = start + length;
        let mut position = self.position;
        while self
            .tokens
            .get(position)
            .is_some_and(|token| token.span.end <= end)
        {
            position += 1;
        }
        if position == self.position || self.tokens[position - 1].span.end != end {
            return None;
        }
        self.position = position;
        Some(Expression::OffsetDateTime {
            value,
            span: Span::new(start, end),
        })
    }

    fn is_day_first_named_date(&self) -> bool {
        self.current()
            .is_some_and(|token| token.kind == LexemeKind::Number)
            && self.month_at(1).is_some()
    }

    fn parse_named_date(&mut self, month_first: bool) -> Result<Expression, Diagnostic> {
        let start = self.current().expect("date start is present");
        let (month, day_offset, after_month_and_day) = if month_first {
            (self.month_at(0).expect("guard checked month"), 1, 2)
        } else {
            (self.month_at(1).expect("guard checked month"), 0, 2)
        };
        let year_offset = if self
            .tokens
            .get(self.position + after_month_and_day)
            .is_some_and(|token| token.kind == LexemeKind::Comma)
        {
            after_month_and_day + 1
        } else {
            after_month_and_day
        };
        let end = self.tokens.get(self.position + year_offset).copied();
        let span = start.span.join(end.map_or(start.span, |token| token.span));
        let day = self.token_u8(day_offset);
        let year = end
            .filter(|token| token.kind == LexemeKind::Number)
            .and_then(|token| {
                self.source[token.span.start..token.span.end]
                    .parse::<i32>()
                    .ok()
            });
        let value = day
            .zip(year)
            .and_then(|(day, year)| Date::new(year, month, day))
            .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidDate, span))?;
        self.position += year_offset + 1;
        Ok(Expression::Date { value, span })
    }

    fn month_at(&self, offset: usize) -> Option<u8> {
        let token = self.tokens.get(self.position + offset)?;
        if token.kind != LexemeKind::Word {
            return None;
        }
        let word = &self.source[token.span.start..token.span.end];
        [
            "january",
            "february",
            "march",
            "april",
            "may",
            "june",
            "july",
            "august",
            "september",
            "october",
            "november",
            "december",
        ]
        .iter()
        .position(|month| word.eq_ignore_ascii_case(month))
        .and_then(|month| u8::try_from(month + 1).ok())
    }

    fn is_iso_date(&self) -> bool {
        matches!(
            self.tokens.get(self.position..self.position + 5),
            Some([year, first_dash, month, second_dash, day])
                if year.kind == LexemeKind::Number
                    && first_dash.kind == LexemeKind::Minus
                    && month.kind == LexemeKind::Number
                    && second_dash.kind == LexemeKind::Minus
                    && day.kind == LexemeKind::Number
                    && year.span.end == first_dash.span.start
                    && first_dash.span.end == month.span.start
                    && month.span.end == second_dash.span.start
                    && second_dash.span.end == day.span.start
        )
    }

    fn parse_iso_date(&mut self) -> Result<Expression, Diagnostic> {
        let tokens = &self.tokens[self.position..self.position + 5];
        let span = tokens[0].span.join(tokens[4].span);
        let component = |token: Lexeme| {
            self.source[token.span.start..token.span.end]
                .parse::<i32>()
                .ok()
        };
        let year = component(tokens[0]);
        let month = component(tokens[2]).and_then(|value| u8::try_from(value).ok());
        let day = component(tokens[4]).and_then(|value| u8::try_from(value).ok());
        let value = year
            .zip(month)
            .zip(day)
            .and_then(|((year, month), day)| Date::new(year, month, day))
            .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidDate, span))?;
        self.position += 5;
        Ok(Expression::Date { value, span })
    }

    fn is_clock_time(&self) -> bool {
        matches!(
            self.tokens.get(self.position..self.position + 3),
            Some([hour, colon, minute])
                if hour.kind == LexemeKind::Number
                    && colon.kind == LexemeKind::Colon
                    && minute.kind == LexemeKind::Number
                    && hour.span.end == colon.span.start
                    && colon.span.end == minute.span.start
        )
    }

    fn parse_clock_time(&mut self) -> Result<Expression, Diagnostic> {
        let start = self.tokens[self.position];
        let hour = self.token_u8(0);
        let minute = self.token_u8(2);
        let mut consumed = 3;
        let mut second = Some(0);
        if self
            .tokens
            .get(self.position + 3)
            .is_some_and(|token| token.kind == LexemeKind::Colon)
        {
            second = self.token_u8(4);
            consumed = 5;
        }

        let suffix = self.tokens.get(self.position + consumed).and_then(|token| {
            (token.kind == LexemeKind::Word)
                .then(|| &self.source[token.span.start..token.span.end])
                .filter(|word| word.eq_ignore_ascii_case("am") || word.eq_ignore_ascii_case("pm"))
        });
        if suffix.is_some() {
            consumed += 1;
        }

        let hour = hour.and_then(|hour| match suffix {
            Some(_) if !(1..=12).contains(&hour) => None,
            Some(suffix) if suffix.eq_ignore_ascii_case("am") => Some(hour % 12),
            Some(_) => Some(hour % 12 + 12),
            None => Some(hour),
        });
        let end = self.tokens[self.position + consumed - 1];
        let span = start.span.join(end.span);
        let value = hour
            .zip(minute)
            .zip(second)
            .and_then(|((hour, minute), second)| ClockTime::new(hour, minute, second))
            .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidTime, span))?;
        self.position += consumed;
        Ok(Expression::Clock { value, span })
    }

    fn token_u8(&self, offset: usize) -> Option<u8> {
        let token = self.tokens.get(self.position + offset)?;
        (token.kind == LexemeKind::Number)
            .then(|| self.source[token.span.start..token.span.end].parse().ok())
            .flatten()
    }

    fn temporal_keyword(&self) -> Option<TemporalKeyword> {
        if self.word_is(0, "today") {
            Some(TemporalKeyword::Today)
        } else if self.word_is(0, "tomorrow") {
            Some(TemporalKeyword::Tomorrow)
        } else if self.word_is(0, "yesterday") {
            Some(TemporalKeyword::Yesterday)
        } else if self.word_is(0, "now") {
            Some(TemporalKeyword::Now)
        } else {
            None
        }
    }

    fn calendar_unit(&self) -> Option<(i64, Lexeme)> {
        let token = self.current()?;
        if token.kind != LexemeKind::Word {
            return None;
        }
        if self.word_is(0, "month") || self.word_is(0, "months") {
            Some((1, token))
        } else if self.word_is(0, "year") || self.word_is(0, "years") {
            Some((12, token))
        } else {
            None
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
        let target = if let Some((currency, token)) = self.currency_at(1) {
            target_span = token.span;
            ConversionTarget::Currency(currency)
        } else if let Some((offset, offset_tokens, span)) = self.utc_offset_at(1) {
            consumed = 1 + offset_tokens;
            target_span = span;
            ConversionTarget::UtcOffset(offset)
        } else if target_token.kind == LexemeKind::Slash {
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

    fn utc_offset_at(&self, offset: usize) -> Option<(UtcOffset, usize, Span)> {
        let token = *self.tokens.get(self.position + offset)?;
        if token.kind == LexemeKind::Word {
            let word = &self.source[token.span.start..token.span.end];
            if word.eq_ignore_ascii_case("utc") || word.eq_ignore_ascii_case("z") {
                return Some((UtcOffset::UTC, 1, token.span));
            }
        }

        let direction = match token.kind {
            LexemeKind::Plus => 1,
            LexemeKind::Minus => -1,
            _ => return None,
        };
        let hours = self.token_u8(offset + 1)?;
        if self.tokens.get(self.position + offset + 2)?.kind != LexemeKind::Colon {
            return None;
        }
        let minutes = self.token_u8(offset + 3)?;
        if hours > 23 || minutes > 59 {
            return None;
        }
        let end = self.tokens[self.position + offset + 3];
        let seconds = direction * (i32::from(hours) * 3600 + i32::from(minutes) * 60);
        Some((
            UtcOffset::from_seconds(seconds)?,
            4,
            token.span.join(end.span),
        ))
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
            LexemeKind::Word
                if self.word_is(0, "at")
                    && !self.word_is(1, "least")
                    && !self.word_is(1, "most") =>
            {
                (BinaryOperator::At, current.span, 1)
            }
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
