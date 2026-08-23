mod functions;
mod temporal;
mod units;

use crate::{
    BinaryOperator, CalculationContext, CalendarDuration, Currency, DateOrder, Diagnostic,
    DiagnosticKind, Expression, Function, Lexeme, LexemeKind, Number, NumberFormat,
    NumberParseError, RadixInteger, Span, UnaryOperator, WorkdayDuration, lex_with_number_format,
};

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
        date_order: context.temporal.date_order,
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
    date_order: Option<DateOrder>,
}

impl Parser<'_> {
    fn parse_expression(&mut self, minimum_binding_power: u8) -> Result<Expression, Diagnostic> {
        let mut left = self.parse_prefix()?;

        loop {
            if minimum_binding_power == 0 && self.word_is(0, "is") && self.word_is(1, "to") {
                left = self.parse_proportion(left)?;
                continue;
            }
            if let Some(percent) = self
                .current()
                .filter(|token| token.kind == LexemeKind::Percent)
                .filter(|token| !self.is_spaced_remainder(*token))
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

            if let Some(factorial) = self
                .current()
                .filter(|token| token.kind == LexemeKind::LogicalNot)
            {
                self.position += 1;
                let span = left.span().join(factorial.span);
                left = Expression::Call {
                    function: Function::Factorial,
                    function_span: factorial.span,
                    arguments: vec![left],
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
                if let Some(expression) = self.parse_configured_numeric_date() {
                    return expression;
                }
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
                let ordinal_suffix = self.ordinal_suffix_at(1);
                self.position += 1 + usize::from(ordinal_suffix.is_some());
                let source = &self.source[token.span.start..token.span.end];
                let radix_value = (source.contains('#')
                    || source.get(..2).is_some_and(|prefix| {
                        matches!(prefix, "0b" | "0B" | "0o" | "0O" | "0x" | "0X")
                    }))
                .then(|| {
                    RadixInteger::parse(source).map_err(|error| {
                        let kind = match error {
                            NumberParseError::Invalid => DiagnosticKind::InvalidNumber,
                            NumberParseError::TooLarge => DiagnosticKind::NumberTooLarge,
                        };
                        Diagnostic::new(kind, token.span)
                    })
                })
                .transpose()?;
                let value = match &radix_value {
                    Some(value) => value.value().clone(),
                    None => self.parse_number_value(token)?,
                };
                let base_span =
                    ordinal_suffix.map_or(token.span, |suffix| token.span.join(suffix.span));
                let (value, span) = if let Some((factor, scalar)) = self.scalar() {
                    self.position += 1;
                    (
                        value.multiply(Number::from_i64(factor)),
                        base_span.join(scalar.span),
                    )
                } else {
                    (value, base_span)
                };
                if let Some((currency, currency_token)) =
                    self.currency_at(0).filter(|_| self.unit().is_none())
                {
                    self.position += 1;
                    Ok(Expression::Money {
                        amount: value,
                        currency,
                        span: span.join(currency_token.span),
                    })
                } else if let Some((consumed, workday_span)) = self.workday_unit() {
                    let days = value.exact_i64().ok_or_else(|| {
                        Diagnostic::new(DiagnosticKind::InvalidCalendarDuration, span)
                    })?;
                    self.position += consumed;
                    Ok(Expression::WorkdayDuration {
                        value: WorkdayDuration::new(days),
                        span: span.join(workday_span),
                    })
                } else if let Some((months_per_unit, calendar_token)) = self.calendar_unit() {
                    let count = value.exact_i64().ok_or_else(|| {
                        Diagnostic::new(DiagnosticKind::InvalidCalendarDuration, span)
                    })?;
                    let duration = if months_per_unit == 12 {
                        CalendarDuration::years(count)
                    } else {
                        count
                            .checked_mul(months_per_unit)
                            .map(CalendarDuration::months)
                    }
                    .ok_or_else(|| {
                        Diagnostic::new(DiagnosticKind::InvalidCalendarDuration, span)
                    })?;
                    self.position += 1;
                    Ok(Expression::CalendarDuration {
                        value: duration,
                        span: span.join(calendar_token.span),
                    })
                } else if let Some((unit, consumed, unit_span)) =
                    self.unit().filter(|_| !self.in_starts_conversion())
                {
                    let (unit, consumed, unit_span) =
                        self.compound_unit_factor(unit, consumed, unit_span, 0)?;
                    self.position += consumed;
                    Ok(Expression::Unit {
                        value,
                        unit,
                        span: span.join(unit_span),
                    })
                } else if let Some(value) = radix_value {
                    Ok(Expression::RadixInteger { value, span })
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
            LexemeKind::Word if self.workday_range_at().is_some() => self.parse_workday_range(),
            LexemeKind::Word if self.word_is(0, "remainder") && self.word_is(1, "of") => {
                self.parse_natural_remainder()
            }
            LexemeKind::Word if self.starts_natural_time_difference() => {
                self.parse_natural_time_difference()
            }
            LexemeKind::Word if self.word_is(0, "clamp") => self.parse_natural_clamp(),
            LexemeKind::Word if self.word_is(0, "midpoint") && self.word_is(1, "between") => {
                self.parse_natural_midpoint()
            }
            LexemeKind::Word if self.natural_function_at().is_some() => {
                self.parse_natural_function()
            }
            LexemeKind::Word if self.relative_weekday_at().is_some() => {
                let (direction, weekday, span) = self
                    .relative_weekday_at()
                    .expect("guard checked the relative weekday");
                self.position += 2;
                Ok(Expression::RelativeWeekday {
                    direction,
                    weekday,
                    span,
                })
            }
            LexemeKind::Word if self.month_at(0).is_some() => self.parse_named_date(true),
            LexemeKind::Word if self.temporal_keyword().is_some() => {
                let (keyword, consumed, span) =
                    self.temporal_keyword().expect("guard checked the keyword");
                self.position += consumed;
                Ok(Expression::TemporalKeyword { keyword, span })
            }
            LexemeKind::Word if self.unit().is_some() => {
                let (unit, consumed, span) = self.unit().expect("guard checked the unit");
                let (unit, consumed, span) = self.compound_unit_factor(unit, consumed, span, 0)?;
                self.position += consumed;
                Ok(Expression::Unit {
                    value: Number::from_i64(1),
                    unit,
                    span,
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
                    LexemeKind::Plus => (UnaryOperator::Positive, 19),
                    LexemeKind::Minus => (UnaryOperator::Negative, 19),
                    LexemeKind::LogicalNot => (UnaryOperator::Not, 11),
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
                let operand = self.parse_expression(11)?;
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

    fn parse_number_value(&self, token: Lexeme) -> Result<Number, Diagnostic> {
        let source = &self.source[token.span.start..token.span.end];
        let normalized = self.number_format.normalize(source).ok_or_else(|| {
            Diagnostic::with_detail(DiagnosticKind::InvalidNumber, token.span, source)
        })?;
        let parsed = if normalized.contains('(') {
            Number::parse_repeating_decimal(&normalized)
        } else {
            Number::parse_decimal(&normalized)
        };
        parsed.map_err(|error| match error {
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
            LexemeKind::CurrencySymbol => Currency::from_symbol(source, self.default_currency)?,
            LexemeKind::Word if source.len() == 3 => Currency::from_code(source)?,
            _ => return None,
        };
        Some((currency, token))
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

    fn word_text_at(&self, offset: usize) -> Option<&str> {
        let token = self.tokens.get(self.position + offset)?;
        (token.kind == LexemeKind::Word).then(|| &self.source[token.span.start..token.span.end])
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
            match text.to_ascii_lowercase().as_str() {
                "k" => Some(1_000),
                "b" => Some(1_000_000_000),
                "t" => Some(1_000_000_000_000),
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

    fn is_spaced_remainder(&self, token: Lexeme) -> bool {
        self.tokens.get(self.position + 1).is_some()
            && self.source[..token.span.start]
                .chars()
                .next_back()
                .is_some_and(char::is_whitespace)
            && self.source[token.span.end..]
                .chars()
                .next()
                .is_some_and(char::is_whitespace)
    }

    fn binary_operator(&self) -> Option<(BinaryOperator, Span, usize, u8, u8)> {
        let current = self.current()?;
        let (operator, span, consumed) = match current.kind {
            LexemeKind::Plus => (BinaryOperator::Add, current.span, 1),
            LexemeKind::Minus => (BinaryOperator::Subtract, current.span, 1),
            LexemeKind::Star => (BinaryOperator::Multiply, current.span, 1),
            LexemeKind::Slash => (BinaryOperator::Divide, current.span, 1),
            LexemeKind::Percent if self.is_spaced_remainder(current) => {
                (BinaryOperator::Remainder, current.span, 1)
            }
            LexemeKind::Caret => (BinaryOperator::Power, current.span, 1),
            LexemeKind::EqualEqual => (BinaryOperator::Equal, current.span, 1),
            LexemeKind::NotEqual => (BinaryOperator::NotEqual, current.span, 1),
            LexemeKind::Greater => (BinaryOperator::Greater, current.span, 1),
            LexemeKind::GreaterEqual => (BinaryOperator::GreaterEqual, current.span, 1),
            LexemeKind::Less => (BinaryOperator::Less, current.span, 1),
            LexemeKind::LessEqual => (BinaryOperator::LessEqual, current.span, 1),
            LexemeKind::LogicalAnd => (BinaryOperator::And, current.span, 1),
            LexemeKind::LogicalOr => (BinaryOperator::Or, current.span, 1),
            LexemeKind::BitwiseAnd => (BinaryOperator::BitwiseAnd, current.span, 1),
            LexemeKind::BitwiseOr => (BinaryOperator::BitwiseOr, current.span, 1),
            LexemeKind::ShiftLeft => (BinaryOperator::ShiftLeft, current.span, 1),
            LexemeKind::ShiftRight => (BinaryOperator::ShiftRight, current.span, 1),
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
            LexemeKind::Word if self.word_is(0, "mod") || self.word_is(0, "modulo") => {
                (BinaryOperator::Modulo, current.span, 1)
            }
            LexemeKind::Word if self.word_is(0, "remainder") => {
                (BinaryOperator::Remainder, current.span, 1)
            }
            LexemeKind::Word
                if (self.word_is(0, "permutation") || self.word_is(0, "permutations"))
                    && self.word_is(1, "of") =>
            {
                (BinaryOperator::PermutationsOf, self.phrase_span(2), 2)
            }
            LexemeKind::Word
                if (self.word_is(0, "combination") || self.word_is(0, "combinations"))
                    && self.word_is(1, "of") =>
            {
                (BinaryOperator::CombinationsOf, self.phrase_span(2), 2)
            }
            LexemeKind::Word
                if self.word_is(0, "permutation") || self.word_is(0, "permutations") =>
            {
                (BinaryOperator::Permutation, current.span, 1)
            }
            LexemeKind::Word
                if self.word_is(0, "combination") || self.word_is(0, "combinations") =>
            {
                (BinaryOperator::Combination, current.span, 1)
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
            LexemeKind::Word if self.word_is(0, "xor") => {
                (BinaryOperator::BitwiseXor, current.span, 1)
            }
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
