use super::Parser;
use crate::{
    CompoundUnit, ConversionTarget, Diagnostic, DiagnosticKind, Expression, LexemeKind, Radix,
    Span, Unit, UtcOffset,
};

impl Parser<'_> {
    pub(super) fn in_starts_conversion(&self) -> bool {
        if !self.word_is(0, "in") {
            return false;
        }
        if self.unit_at(1).is_some()
            || self.custom_unit_at(1).is_some()
            || self.currency_at(1).is_some()
            || self.utc_offset_at(1).is_some()
        {
            return true;
        }
        self.word_text_at(1).is_some_and(|target| {
            target.eq_ignore_ascii_case("binary")
                || target.eq_ignore_ascii_case("bin")
                || target.eq_ignore_ascii_case("octal")
                || target.eq_ignore_ascii_case("oct")
                || target.eq_ignore_ascii_case("hexadecimal")
                || target.eq_ignore_ascii_case("hex")
                || target.eq_ignore_ascii_case("decimal")
                || target.eq_ignore_ascii_case("dec")
                || target.eq_ignore_ascii_case("number")
                || target.eq_ignore_ascii_case("base")
                || target.eq_ignore_ascii_case("roman")
                || target.eq_ignore_ascii_case("fraction")
                || target.eq_ignore_ascii_case("percent")
                || target.eq_ignore_ascii_case("percentage")
        })
    }

    pub(super) fn is_conversion_operator(&self) -> bool {
        self.word_is(0, "as")
            || self.word_is(0, "in")
            || (self.word_is(0, "to") && !self.word_is(1, "the"))
    }

    pub(super) fn compound_unit_target_at(
        &self,
        offset: usize,
    ) -> Result<Option<(CompoundUnit, usize, Span)>, Diagnostic> {
        let mut cursor = offset;
        let mut unit = CompoundUnit::default();
        let mut divide = false;
        let mut span = None;

        if self
            .tokens
            .get(self.position + cursor)
            .is_some_and(|token| token.kind == LexemeKind::Slash)
        {
            span = Some(self.tokens[self.position + cursor].span);
            divide = true;
            cursor += 1;
        }

        let Some((first, consumed, first_span)) = self.unit_at(cursor) else {
            if divide {
                return Err(Diagnostic::with_detail(
                    DiagnosticKind::UnexpectedToken,
                    self.tokens
                        .get(self.position + cursor)
                        .map_or(Span::empty(self.source.len()), |token| token.span),
                    "expected a unit after '/'",
                ));
            }
            return Ok(None);
        };
        let (factor, factor_consumed, factor_span) =
            self.compound_unit_factor(first, consumed, first_span, cursor)?;
        unit = if divide {
            unit.divide(&factor)
        } else {
            unit.multiply(&factor)
        };
        cursor += factor_consumed;
        span = Some(span.map_or(factor_span, |span: Span| span.join(factor_span)));

        loop {
            let separator = self.tokens.get(self.position + cursor).copied();
            let separator_is_divide = separator
                .is_some_and(|token| token.kind == LexemeKind::Slash)
                || self.word_is(cursor, "per");
            let separator_is_multiply =
                separator.is_some_and(|token| token.kind == LexemeKind::Star);
            if !separator_is_divide && !separator_is_multiply {
                break;
            }
            let factor_offset = cursor + 1;
            let Some((next, consumed, next_span)) = self.unit_at(factor_offset) else {
                break;
            };
            let (factor, factor_consumed, factor_span) =
                self.compound_unit_factor(next, consumed, next_span, factor_offset)?;
            unit = if separator_is_divide {
                unit.divide(&factor)
            } else {
                unit.multiply(&factor)
            };
            cursor = factor_offset + factor_consumed;
            span = Some(
                span.expect("the first unit established a span")
                    .join(factor_span),
            );
        }

        if !unit.exponents_are_bounded() {
            return Err(Diagnostic::new(
                DiagnosticKind::InvalidUnitExponent,
                span.expect("a compound unit contains at least one factor"),
            ));
        }
        Ok(Some((
            unit,
            cursor - offset,
            span.expect("a compound unit contains at least one factor"),
        )))
    }

    pub(super) fn compound_unit_factor(
        &self,
        unit: Unit,
        consumed: usize,
        span: Span,
        offset: usize,
    ) -> Result<(CompoundUnit, usize, Span), Diagnostic> {
        let unit_source = &self.source[span.start..span.end];
        if Unit::from_alias(unit_source).is_none() {
            if unit_source.ends_with('²') {
                return Ok((CompoundUnit::from_unit(unit).pow(2), consumed, span));
            }
            if unit_source.ends_with('³') {
                return Ok((CompoundUnit::from_unit(unit).pow(3), consumed, span));
            }
        }

        let exponent_offset = offset + consumed;
        if !self
            .tokens
            .get(self.position + exponent_offset)
            .is_some_and(|token| token.kind == LexemeKind::Caret)
        {
            return Ok((CompoundUnit::from_unit(unit), consumed, span));
        }
        let sign_token = self
            .tokens
            .get(self.position + exponent_offset + 1)
            .copied();
        let negative = sign_token.is_some_and(|token| token.kind == LexemeKind::Minus);
        let exponent_token_offset = exponent_offset + 1 + usize::from(negative);
        let exponent_token = self
            .tokens
            .get(self.position + exponent_token_offset)
            .copied()
            .ok_or_else(|| {
                Diagnostic::with_detail(
                    DiagnosticKind::UnexpectedEnd,
                    Span::empty(self.source.len()),
                    "expected an integer unit exponent",
                )
            })?;
        if exponent_token.kind != LexemeKind::Number {
            return Err(Diagnostic::with_detail(
                DiagnosticKind::UnexpectedToken,
                exponent_token.span,
                "expected an integer unit exponent",
            ));
        }
        let exponent = self
            .parse_number_value(exponent_token)?
            .exact_i64()
            .map(|value| if negative { -value } else { value })
            .and_then(|value| i32::try_from(value).ok())
            .filter(|value| value.unsigned_abs() <= 100)
            .ok_or_else(|| {
                Diagnostic::new(DiagnosticKind::InvalidUnitExponent, exponent_token.span)
            })?;
        Ok((
            CompoundUnit::from_unit(unit).pow(exponent),
            consumed + 2 + usize::from(negative),
            span.join(exponent_token.span),
        ))
    }

    pub(super) fn parse_conversion(
        &mut self,
        expression: Expression,
    ) -> Result<Expression, Diagnostic> {
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
        } else if let Some((zone, zone_tokens, zone_span)) = self.iana_zone_at(1) {
            consumed = 1 + zone_tokens;
            target_span = zone_span;
            ConversionTarget::TimeZone(zone)
        } else if let Some((definition, unit_span)) = self.custom_unit_at(1) {
            target_span = unit_span;
            ConversionTarget::CustomUnit(definition)
        } else if let Some((unit, unit_tokens, unit_span)) = self.compound_unit_target_at(1)? {
            consumed = 1 + unit_tokens;
            target_span = unit_span;
            ConversionTarget::CompoundUnit(unit)
        } else {
            match target_token.kind {
                LexemeKind::Percent => ConversionTarget::Percentage,
                LexemeKind::Word => {
                    let target = &self.source[target_token.span.start..target_token.span.end];
                    if target.eq_ignore_ascii_case("number")
                        || target.eq_ignore_ascii_case("decimal")
                        || target.eq_ignore_ascii_case("dec")
                    {
                        ConversionTarget::Number
                    } else if target.eq_ignore_ascii_case("binary")
                        || target.eq_ignore_ascii_case("bin")
                    {
                        ConversionTarget::Radix(Radix::Binary)
                    } else if target.eq_ignore_ascii_case("octal")
                        || target.eq_ignore_ascii_case("oct")
                    {
                        ConversionTarget::Radix(Radix::Octal)
                    } else if target.eq_ignore_ascii_case("hexadecimal")
                        || target.eq_ignore_ascii_case("hex")
                    {
                        ConversionTarget::Radix(Radix::Hexadecimal)
                    } else if target.eq_ignore_ascii_case("base") {
                        let radix_token = self.tokens.get(self.position + 2).ok_or_else(|| {
                            Diagnostic::with_detail(
                                DiagnosticKind::UnexpectedEnd,
                                Span::empty(self.source.len()),
                                "expected an integer from 2 through 36 after 'base'",
                            )
                        })?;
                        let radix = &self.source[radix_token.span.start..radix_token.span.end];
                        consumed = 3;
                        target_span = target_span.join(radix_token.span);
                        match radix.parse::<u8>().ok() {
                            Some(10) => ConversionTarget::Number,
                            Some(radix) => {
                                ConversionTarget::Radix(Radix::new(radix).ok_or_else(|| {
                                    Diagnostic::with_detail(
                                        DiagnosticKind::UnexpectedToken,
                                        radix_token.span,
                                        "expected a base from 2 through 36",
                                    )
                                })?)
                            }
                            None => {
                                return Err(Diagnostic::with_detail(
                                    DiagnosticKind::UnexpectedToken,
                                    radix_token.span,
                                    "expected a base from 2 through 36",
                                ));
                            }
                        }
                    } else if target.eq_ignore_ascii_case("roman") {
                        ConversionTarget::Roman
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

    pub(super) fn utc_offset_at(&self, offset: usize) -> Option<(UtcOffset, usize, Span)> {
        let token = *self.tokens.get(self.position + offset)?;
        if token.kind == LexemeKind::Word {
            let word = &self.source[token.span.start..token.span.end];
            let fixed_seconds = if word.eq_ignore_ascii_case("utc")
                || word.eq_ignore_ascii_case("gmt")
                || word.eq_ignore_ascii_case("z")
            {
                0
            } else if word.eq_ignore_ascii_case("cet") {
                3600
            } else if word.eq_ignore_ascii_case("cest") {
                7200
            } else {
                return None;
            };

            if (word.eq_ignore_ascii_case("utc") || word.eq_ignore_ascii_case("gmt"))
                && self
                    .tokens
                    .get(self.position + offset + 1)
                    .is_some_and(|token| matches!(token.kind, LexemeKind::Plus | LexemeKind::Minus))
            {
                let (value, consumed, span) = self.signed_utc_offset_at(offset + 1)?;
                return Some((value, consumed + 1, token.span.join(span)));
            }
            return Some((UtcOffset::from_seconds(fixed_seconds)?, 1, token.span));
        }

        self.signed_utc_offset_at(offset)
    }

    fn signed_utc_offset_at(&self, offset: usize) -> Option<(UtcOffset, usize, Span)> {
        let sign = *self.tokens.get(self.position + offset)?;
        let direction = match sign.kind {
            LexemeKind::Plus => 1,
            LexemeKind::Minus => -1,
            _ => return None,
        };
        let hours = self.token_u8(offset + 1)?;
        let mut consumed = 2;
        let mut minutes = 0;
        if self
            .tokens
            .get(self.position + offset + 2)
            .is_some_and(|token| token.kind == LexemeKind::Colon)
        {
            minutes = self.token_u8(offset + 3)?;
            consumed = 4;
        }
        if hours > 23 || minutes > 59 {
            return None;
        }
        let end = self.tokens[self.position + offset + consumed - 1];
        let seconds = direction * (i32::from(hours) * 3600 + i32::from(minutes) * 60);
        Some((
            UtcOffset::from_seconds(seconds)?,
            consumed,
            sign.span.join(end.span),
        ))
    }

    pub(super) fn unit(&self) -> Option<(Unit, usize, Span)> {
        self.unit_at(0)
    }

    pub(super) fn unit_at(&self, offset: usize) -> Option<(Unit, usize, Span)> {
        let first_token = *self.tokens.get(self.position + offset)?;
        let first = self.word_text_at(offset)?;

        if first.eq_ignore_ascii_case("square") || first.eq_ignore_ascii_case("sq") {
            let second_token = *self.tokens.get(self.position + offset + 1)?;
            let unit = match Unit::from_alias(self.word_text_at(offset + 1)?)? {
                Unit::Meter => Unit::SquareMeter,
                Unit::Kilometer => Unit::SquareKilometer,
                Unit::Centimeter => Unit::SquareCentimeter,
                Unit::Millimeter => Unit::SquareMillimeter,
                Unit::Inch => Unit::SquareInch,
                Unit::Foot => Unit::SquareFoot,
                Unit::Yard => Unit::SquareYard,
                Unit::Mile => Unit::SquareMile,
                _ => return None,
            };
            return Some((unit, 2, first_token.span.join(second_token.span)));
        }

        if first.eq_ignore_ascii_case("cubic") {
            let second_token = *self.tokens.get(self.position + offset + 1)?;
            let unit = match Unit::from_alias(self.word_text_at(offset + 1)?)? {
                Unit::Meter => Unit::CubicMeter,
                Unit::Centimeter => Unit::CubicCentimeter,
                _ => return None,
            };
            return Some((unit, 2, first_token.span.join(second_token.span)));
        }

        if first.eq_ignore_ascii_case("us")
            && let (Some(second_token), Some(second)) = (
                self.tokens.get(self.position + offset + 1).copied(),
                self.word_text_at(offset + 1),
            )
        {
            if (second.eq_ignore_ascii_case("fluid") || second.eq_ignore_ascii_case("fl"))
                && let (Some(third_token), Some(third)) = (
                    self.tokens.get(self.position + offset + 2).copied(),
                    self.word_text_at(offset + 2),
                )
                && (third.eq_ignore_ascii_case("ounce")
                    || third.eq_ignore_ascii_case("ounces")
                    || third.eq_ignore_ascii_case("oz"))
            {
                return Some((
                    Unit::UsFluidOunce,
                    3,
                    first_token.span.join(third_token.span),
                ));
            }
            let unit = if second.eq_ignore_ascii_case("gallon")
                || second.eq_ignore_ascii_case("gallons")
                || second.eq_ignore_ascii_case("gal")
            {
                Some(Unit::UsGallon)
            } else if second.eq_ignore_ascii_case("quart")
                || second.eq_ignore_ascii_case("quarts")
                || second.eq_ignore_ascii_case("qt")
            {
                Some(Unit::UsQuart)
            } else if second.eq_ignore_ascii_case("pint")
                || second.eq_ignore_ascii_case("pints")
                || second.eq_ignore_ascii_case("pt")
            {
                Some(Unit::UsPint)
            } else if second.eq_ignore_ascii_case("cup") || second.eq_ignore_ascii_case("cups") {
                Some(Unit::UsCup)
            } else if second.eq_ignore_ascii_case("tablespoon")
                || second.eq_ignore_ascii_case("tablespoons")
                || second.eq_ignore_ascii_case("tbsp")
            {
                Some(Unit::UsTablespoon)
            } else if second.eq_ignore_ascii_case("teaspoon")
                || second.eq_ignore_ascii_case("teaspoons")
                || second.eq_ignore_ascii_case("tsp")
            {
                Some(Unit::UsTeaspoon)
            } else {
                None
            };
            if let Some(unit) = unit {
                return Some((unit, 2, first_token.span.join(second_token.span)));
            }
        }

        if (first.eq_ignore_ascii_case("watt") || first.eq_ignore_ascii_case("kilowatt"))
            && let (Some(second_token), Some(second)) = (
                self.tokens.get(self.position + offset + 1).copied(),
                self.word_text_at(offset + 1),
            )
            && (second.eq_ignore_ascii_case("hour") || second.eq_ignore_ascii_case("hours"))
        {
            let unit = if first.eq_ignore_ascii_case("watt") {
                Unit::WattHour
            } else {
                Unit::KilowattHour
            };
            return Some((unit, 2, first_token.span.join(second_token.span)));
        }

        if first.eq_ignore_ascii_case("metric") {
            let second_token = *self.tokens.get(self.position + offset + 1)?;
            let second = self.word_text_at(offset + 1)?;
            let unit = if second.eq_ignore_ascii_case("cup") || second.eq_ignore_ascii_case("cups")
            {
                Unit::MetricCup
            } else if second.eq_ignore_ascii_case("tablespoon")
                || second.eq_ignore_ascii_case("tablespoons")
                || second.eq_ignore_ascii_case("tbsp")
            {
                Unit::MetricTablespoon
            } else if second.eq_ignore_ascii_case("teaspoon")
                || second.eq_ignore_ascii_case("teaspoons")
                || second.eq_ignore_ascii_case("tsp")
            {
                Unit::MetricTeaspoon
            } else {
                return None;
            };
            return Some((unit, 2, first_token.span.join(second_token.span)));
        }

        Unit::from_alias(first)
            .or_else(|| {
                first
                    .strip_suffix('²')
                    .or_else(|| first.strip_suffix('³'))
                    .and_then(Unit::from_alias)
            })
            .map(|unit| (unit, 1, first_token.span))
    }
}
