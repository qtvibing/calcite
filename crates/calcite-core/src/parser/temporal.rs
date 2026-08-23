use super::Parser;
use crate::{
    CalendarDuration, ClockTime, CompoundUnit, Date, DateOrder, Diagnostic, DiagnosticKind,
    Dimension, Expression, Function, Lexeme, LexemeKind, Number, OffsetClockTime, OffsetDateTime,
    RelativeWeekdayDirection, Span, TemporalKeyword, Unit, Weekday, ZonedClockTime,
};

#[derive(Clone, Copy)]
enum TimeDifferenceTarget {
    Fixed(Unit),
    Months,
    Years,
    Calendar,
}

impl Parser<'_> {
    pub(super) fn parse_configured_numeric_date(
        &mut self,
    ) -> Option<Result<Expression, Diagnostic>> {
        let order = self.date_order?;
        let start = self.current()?.span.start;
        let length = self.source[start..]
            .bytes()
            .take_while(|byte| byte.is_ascii_digit() || matches!(*byte, b'/' | b'.'))
            .count();
        let end = start + length;
        let source = &self.source[start..end];
        let separator = if source.bytes().filter(|byte| *byte == b'/').count() == 2 {
            '/'
        } else if source.bytes().filter(|byte| *byte == b'.').count() == 2 {
            '.'
        } else {
            return None;
        };
        if source.contains(if separator == '/' { '.' } else { '/' }) {
            return None;
        }
        let parts = source.split(separator).collect::<Vec<_>>();
        if parts.len() != 3 || parts.iter().any(|part| part.is_empty()) {
            return None;
        }
        let (year, month, day) = match order {
            DateOrder::DayMonthYear => (parts[2], parts[1], parts[0]),
            DateOrder::MonthDayYear => (parts[2], parts[0], parts[1]),
            DateOrder::YearMonthDay => (parts[0], parts[1], parts[2]),
        };
        if year.len() != 4 {
            return Some(Err(Diagnostic::with_detail(
                DiagnosticKind::InvalidDate,
                Span::new(start, end),
                "numeric dates require a four-digit year",
            )));
        }
        let date = year
            .parse::<i32>()
            .ok()
            .zip(month.parse::<u8>().ok())
            .zip(day.parse::<u8>().ok())
            .and_then(|((year, month), day)| Date::new(year, month, day));
        let Some(date) = date else {
            return Some(Err(Diagnostic::new(
                DiagnosticKind::InvalidDate,
                Span::new(start, end),
            )));
        };
        while self.current().is_some_and(|token| token.span.start < end) {
            self.position += 1;
        }
        Some(Ok(Expression::Date {
            value: date,
            span: Span::new(start, end),
        }))
    }

    pub(super) fn starts_iso_datetime(&self) -> bool {
        self.is_iso_date()
            && self.source[self.tokens[self.position + 4].span.end..]
                .chars()
                .next()
                .is_some_and(|character| matches!(character, 'T' | 't'))
    }

    pub(super) fn parse_iso_offset_datetime(&mut self) -> Option<Expression> {
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

    pub(super) fn is_day_first_named_date(&self) -> bool {
        self.current()
            .is_some_and(|token| token.kind == LexemeKind::Number)
            && (self.month_at(1).is_some()
                || (self.ordinal_suffix_at(1).is_some() && self.month_at(2).is_some()))
    }

    pub(super) fn parse_named_date(&mut self, month_first: bool) -> Result<Expression, Diagnostic> {
        let start = self.current().expect("date start is present");
        let day_offset = usize::from(month_first);
        let suffix_offset = day_offset + 1;
        let has_ordinal_suffix = self.ordinal_suffix_at(suffix_offset).is_some();
        let month_offset = if month_first {
            0
        } else {
            1 + usize::from(has_ordinal_suffix)
        };
        let after_month_and_day = 2 + usize::from(has_ordinal_suffix);
        let month = self
            .month_at(month_offset)
            .expect("guard checked the month");
        let comma = self
            .tokens
            .get(self.position + after_month_and_day)
            .filter(|token| token.kind == LexemeKind::Comma)
            .copied();
        let year_offset = after_month_and_day + usize::from(comma.is_some());
        let year_token = self
            .tokens
            .get(self.position + year_offset)
            .filter(|token| token.kind == LexemeKind::Number)
            .copied();
        if let Some(comma) = comma.filter(|_| year_token.is_none()) {
            return Err(Diagnostic::new(
                DiagnosticKind::InvalidDate,
                start.span.join(comma.span),
            ));
        }

        let day = self
            .token_u8(day_offset)
            .filter(|day| Date::new(2000, month, *day).is_some())
            .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidDate, start.span))?;
        if let Some(year_token) = year_token {
            let span = start.span.join(year_token.span);
            let year = self.source[year_token.span.start..year_token.span.end]
                .parse::<i32>()
                .ok();
            let value = year
                .and_then(|year| Date::new(year, month, day))
                .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidDate, span))?;
            self.position += year_offset + 1;
            Ok(Expression::Date { value, span })
        } else {
            let end_offset = after_month_and_day - 1;
            let span = start
                .span
                .join(self.tokens[self.position + end_offset].span);
            self.position += after_month_and_day;
            Ok(Expression::PartialDate { month, day, span })
        }
    }

    pub(super) fn month_at(&self, offset: usize) -> Option<u8> {
        let token = self.tokens.get(self.position + offset)?;
        if token.kind != LexemeKind::Word {
            return None;
        }
        let word = &self.source[token.span.start..token.span.end];
        [
            &["january", "jan"][..],
            &["february", "feb"][..],
            &["march", "mar"][..],
            &["april", "apr"][..],
            &["may"][..],
            &["june", "jun"][..],
            &["july", "jul"][..],
            &["august", "aug"][..],
            &["september", "sep", "sept"][..],
            &["october", "oct"][..],
            &["november", "nov"][..],
            &["december", "dec"][..],
        ]
        .iter()
        .position(|aliases| aliases.iter().any(|alias| word.eq_ignore_ascii_case(alias)))
        .and_then(|month| u8::try_from(month + 1).ok())
    }

    pub(super) fn ordinal_suffix_at(&self, offset: usize) -> Option<Lexeme> {
        let number = *self.tokens.get(self.position + offset - 1)?;
        let suffix = *self.tokens.get(self.position + offset)?;
        if number.kind != LexemeKind::Number
            || suffix.kind != LexemeKind::Word
            || number.span.end != suffix.span.start
        {
            return None;
        }
        let day = self.source[number.span.start..number.span.end]
            .parse::<u8>()
            .ok()?;
        let expected = match day % 100 {
            11..=13 => "th",
            _ => match day % 10 {
                1 => "st",
                2 => "nd",
                3 => "rd",
                _ => "th",
            },
        };
        self.source[suffix.span.start..suffix.span.end]
            .eq_ignore_ascii_case(expected)
            .then_some(suffix)
    }

    pub(super) fn is_iso_date(&self) -> bool {
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

    pub(super) fn parse_iso_date(&mut self) -> Result<Expression, Diagnostic> {
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

    pub(super) fn is_clock_time(&self) -> bool {
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

    pub(super) fn parse_clock_time(&mut self) -> Result<Expression, Diagnostic> {
        let start = self.tokens[self.position];
        let hour = self.token_u8(0);
        let minute = self.token_u8(2);
        let mut consumed = 3;
        let mut second = Some((0, 0));
        if self
            .tokens
            .get(self.position + 3)
            .is_some_and(|token| token.kind == LexemeKind::Colon)
        {
            second = self.clock_second_at(4);
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
        let value = hour
            .zip(minute)
            .zip(second)
            .and_then(|((hour, minute), (second, nanosecond))| {
                ClockTime::with_nanosecond(hour, minute, second, nanosecond)
            })
            .ok_or_else(|| Diagnostic::new(DiagnosticKind::InvalidTime, start.span))?;
        let offset = self
            .tokens
            .get(self.position + consumed)
            .filter(|token| token.kind == LexemeKind::Word)
            .and_then(|_| self.utc_offset_at(consumed));
        if let Some((offset, offset_tokens, offset_span)) = offset {
            consumed += offset_tokens;
            let span = start.span.join(offset_span);
            self.position += consumed;
            Ok(Expression::OffsetClock {
                value: OffsetClockTime::new(value, offset),
                span,
            })
        } else if let Some((zone, zone_tokens, zone_span)) = self.iana_zone_at(consumed) {
            consumed += zone_tokens;
            let span = start.span.join(zone_span);
            self.position += consumed;
            Ok(Expression::ZonedClock {
                value: ZonedClockTime::new(value, zone),
                span,
            })
        } else {
            let end = self.tokens[self.position + consumed - 1];
            let span = start.span.join(end.span);
            self.position += consumed;
            Ok(Expression::Clock { value, span })
        }
    }

    pub(super) fn iana_zone_at(&self, offset: usize) -> Option<(String, usize, Span)> {
        let first = *self.tokens.get(self.position + offset)?;
        if first.kind != LexemeKind::Word {
            return None;
        }
        let region = &self.source[first.span.start..first.span.end];
        if ![
            "africa",
            "america",
            "antarctica",
            "arctic",
            "asia",
            "atlantic",
            "australia",
            "europe",
            "indian",
            "pacific",
            "etc",
        ]
        .iter()
        .any(|candidate| region.eq_ignore_ascii_case(candidate))
        {
            return None;
        }

        let mut consumed = 1;
        let mut end = first.span;
        loop {
            let slash = self.tokens.get(self.position + offset + consumed);
            let segment = self.tokens.get(self.position + offset + consumed + 1);
            let Some((slash, segment)) = slash.zip(segment) else {
                break;
            };
            if slash.kind != LexemeKind::Slash
                || segment.kind != LexemeKind::Word
                || end.end != slash.span.start
                || slash.span.end != segment.span.start
            {
                break;
            }
            consumed += 2;
            end = segment.span;
        }
        if consumed == 1 {
            return None;
        }
        let span = first.span.join(end);
        Some((self.source[span.start..span.end].to_owned(), consumed, span))
    }

    pub(super) fn token_u8(&self, offset: usize) -> Option<u8> {
        let token = self.tokens.get(self.position + offset)?;
        (token.kind == LexemeKind::Number)
            .then(|| self.source[token.span.start..token.span.end].parse().ok())
            .flatten()
    }

    fn clock_second_at(&self, offset: usize) -> Option<(u8, u32)> {
        let token = *self.tokens.get(self.position + offset)?;
        if token.kind != LexemeKind::Number {
            return None;
        }
        let nanoseconds = self
            .parse_number_value(token)
            .ok()?
            .multiply(Number::from_i64(1_000_000_000))
            .exact_i64()?;
        if !(0..60_000_000_000).contains(&nanoseconds) {
            return None;
        }
        Some((
            (nanoseconds / 1_000_000_000) as u8,
            (nanoseconds % 1_000_000_000) as u32,
        ))
    }

    pub(super) fn relative_weekday_at(&self) -> Option<(RelativeWeekdayDirection, Weekday, Span)> {
        let first = self.current()?;
        let direction = if self.word_is(0, "next") {
            RelativeWeekdayDirection::Next
        } else if self.word_is(0, "last") || self.word_is(0, "previous") {
            RelativeWeekdayDirection::Previous
        } else if self.word_is(0, "this") {
            RelativeWeekdayDirection::This
        } else {
            return None;
        };
        let second = self.tokens.get(self.position + 1)?;
        let weekday = self.weekday_at(1)?;
        Some((direction, weekday, first.span.join(second.span)))
    }

    pub(super) fn weekday_at(&self, offset: usize) -> Option<Weekday> {
        let weekday = self.word_text_at(offset)?;
        if weekday.eq_ignore_ascii_case("monday") || weekday.eq_ignore_ascii_case("mon") {
            Some(Weekday::Monday)
        } else if weekday.eq_ignore_ascii_case("tuesday") || weekday.eq_ignore_ascii_case("tue") {
            Some(Weekday::Tuesday)
        } else if weekday.eq_ignore_ascii_case("wednesday") || weekday.eq_ignore_ascii_case("wed") {
            Some(Weekday::Wednesday)
        } else if weekday.eq_ignore_ascii_case("thursday") || weekday.eq_ignore_ascii_case("thu") {
            Some(Weekday::Thursday)
        } else if weekday.eq_ignore_ascii_case("friday") || weekday.eq_ignore_ascii_case("fri") {
            Some(Weekday::Friday)
        } else if weekday.eq_ignore_ascii_case("saturday") || weekday.eq_ignore_ascii_case("sat") {
            Some(Weekday::Saturday)
        } else if weekday.eq_ignore_ascii_case("sunday") || weekday.eq_ignore_ascii_case("sun") {
            Some(Weekday::Sunday)
        } else {
            None
        }
    }

    pub(super) fn starts_natural_time_difference(&self) -> bool {
        self.time_difference_prefix()
            .is_some_and(|(_, relation_offset)| {
                self.word_is(relation_offset, "between")
                    || self.word_is(relation_offset, "from")
                    || self.word_is(relation_offset, "since")
                    || self.word_is(relation_offset, "until")
            })
    }

    fn time_difference_prefix(&self) -> Option<(Option<TimeDifferenceTarget>, usize)> {
        if self.word_is(0, "how") && self.word_is(1, "long") {
            return Some((None, 2));
        }
        if self.word_is(0, "calendar") && (self.word_is(1, "time") || self.word_is(1, "duration")) {
            return Some((Some(TimeDifferenceTarget::Calendar), 2));
        }
        self.time_difference_target().map(|target| (target, 1))
    }

    fn time_difference_target(&self) -> Option<Option<TimeDifferenceTarget>> {
        let word = self.word_text_at(0)?;
        if word.eq_ignore_ascii_case("time") || word.eq_ignore_ascii_case("duration") {
            return Some(None);
        }
        if word.eq_ignore_ascii_case("month") || word.eq_ignore_ascii_case("months") {
            return Some(Some(TimeDifferenceTarget::Months));
        }
        if word.eq_ignore_ascii_case("year") || word.eq_ignore_ascii_case("years") {
            return Some(Some(TimeDifferenceTarget::Years));
        }
        Unit::from_alias(word)
            .filter(|unit| unit.dimension() == Dimension::Duration)
            .map(TimeDifferenceTarget::Fixed)
            .map(Some)
    }

    pub(super) fn parse_natural_time_difference(&mut self) -> Result<Expression, Diagnostic> {
        let start = self.current().expect("time phrase is present").span;
        let (target, relation_offset) = self
            .time_difference_prefix()
            .expect("guard checked the time difference prefix");
        let calendar = matches!(target, Some(TimeDifferenceTarget::Calendar));
        let function = if self.word_is(relation_offset, "between") {
            if calendar {
                Function::CalendarBetween
            } else {
                Function::TimeBetween
            }
        } else if self.word_is(relation_offset, "from") {
            if calendar {
                Function::CalendarFrom
            } else {
                Function::TimeFrom
            }
        } else if self.word_is(relation_offset, "since") {
            if calendar {
                Function::CalendarSince
            } else {
                Function::TimeSince
            }
        } else if calendar {
            Function::CalendarUntil
        } else {
            Function::TimeUntil
        };
        self.position += relation_offset + 1;

        let has_second_argument = matches!(
            function,
            Function::TimeBetween
                | Function::TimeFrom
                | Function::CalendarBetween
                | Function::CalendarFrom
        );
        let first = self.parse_expression(if has_second_argument { 4 } else { 0 })?;
        let mut arguments = vec![first];
        if has_second_argument {
            let (separator, detail) =
                if matches!(function, Function::TimeBetween | Function::CalendarBetween) {
                    ("and", "expected 'and'")
                } else {
                    ("to", "expected 'to'")
                };
            self.consume_word(separator, detail)?;
            arguments.push(self.parse_expression(0)?);
        }
        let end = arguments
            .last()
            .expect("a time difference has an argument")
            .span();
        match target {
            Some(TimeDifferenceTarget::Fixed(unit)) => arguments.push(Expression::Unit {
                value: Number::from_i64(1),
                unit: CompoundUnit::from_unit(unit),
                span: start,
            }),
            Some(TimeDifferenceTarget::Months) => {
                arguments.push(Expression::CalendarDuration {
                    value: CalendarDuration::months(1),
                    span: start,
                });
            }
            Some(TimeDifferenceTarget::Years) => {
                arguments.push(Expression::CalendarDuration {
                    value: CalendarDuration::years(1).expect("one year is representable"),
                    span: start,
                });
            }
            Some(TimeDifferenceTarget::Calendar) | None => {}
        }
        let span = start.join(end);
        Ok(Expression::Call {
            function,
            function_span: start,
            arguments,
            span,
        })
    }

    pub(super) fn temporal_keyword(&self) -> Option<(TemporalKeyword, usize, Span)> {
        let first = self.current()?;
        let single = if self.word_is(0, "today") {
            Some(TemporalKeyword::Today)
        } else if self.word_is(0, "tomorrow") {
            Some(TemporalKeyword::Tomorrow)
        } else if self.word_is(0, "yesterday") {
            Some(TemporalKeyword::Yesterday)
        } else if self.word_is(0, "now") {
            Some(TemporalKeyword::Now)
        } else {
            None
        };
        if let Some(keyword) = single {
            return Some((keyword, 1, first.span));
        }

        let keyword = if self.word_is(0, "next") && self.word_is(1, "month") {
            TemporalKeyword::NextMonth
        } else if (self.word_is(0, "last") || self.word_is(0, "previous"))
            && self.word_is(1, "month")
        {
            TemporalKeyword::PreviousMonth
        } else if self.word_is(0, "next") && self.word_is(1, "year") {
            TemporalKeyword::NextYear
        } else if (self.word_is(0, "last") || self.word_is(0, "previous"))
            && self.word_is(1, "year")
        {
            TemporalKeyword::PreviousYear
        } else if self.word_is(0, "start") && self.word_is(1, "of") && self.word_is(2, "month") {
            TemporalKeyword::StartOfMonth
        } else if self.word_is(0, "end") && self.word_is(1, "of") && self.word_is(2, "month") {
            TemporalKeyword::EndOfMonth
        } else if self.word_is(0, "start") && self.word_is(1, "of") && self.word_is(2, "year") {
            TemporalKeyword::StartOfYear
        } else if self.word_is(0, "end") && self.word_is(1, "of") && self.word_is(2, "year") {
            TemporalKeyword::EndOfYear
        } else {
            return None;
        };
        let consumed = if matches!(
            keyword,
            TemporalKeyword::StartOfMonth
                | TemporalKeyword::EndOfMonth
                | TemporalKeyword::StartOfYear
                | TemporalKeyword::EndOfYear
        ) {
            3
        } else {
            2
        };
        Some((keyword, consumed, self.phrase_span(consumed)))
    }

    pub(super) fn workday_unit(&self) -> Option<(usize, Span)> {
        let first = self.current()?;
        let word = self.word_text_at(0)?;
        if word.eq_ignore_ascii_case("workday")
            || word.eq_ignore_ascii_case("workdays")
            || word.eq_ignore_ascii_case("weekday")
            || word.eq_ignore_ascii_case("weekdays")
            || word.eq_ignore_ascii_case("businessday")
            || word.eq_ignore_ascii_case("businessdays")
        {
            return Some((1, first.span));
        }
        if word.eq_ignore_ascii_case("business")
            && (self.word_is(1, "day") || self.word_is(1, "days"))
        {
            let second = self.tokens[self.position + 1];
            return Some((2, first.span.join(second.span)));
        }
        None
    }

    pub(super) fn calendar_unit(&self) -> Option<(i64, Lexeme)> {
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
}
