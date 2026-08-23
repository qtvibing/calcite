use super::Parser;
use crate::{
    ClockTime, Date, Diagnostic, DiagnosticKind, Expression, Lexeme, LexemeKind, OffsetDateTime,
    RelativeWeekdayDirection, Span, TemporalKeyword, Weekday,
};

impl Parser<'_> {
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
            && self.month_at(1).is_some()
    }

    pub(super) fn parse_named_date(&mut self, month_first: bool) -> Result<Expression, Diagnostic> {
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

    pub(super) fn month_at(&self, offset: usize) -> Option<u8> {
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

    pub(super) fn token_u8(&self, offset: usize) -> Option<u8> {
        let token = self.tokens.get(self.position + offset)?;
        (token.kind == LexemeKind::Number)
            .then(|| self.source[token.span.start..token.span.end].parse().ok())
            .flatten()
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

    pub(super) fn temporal_keyword(&self) -> Option<TemporalKeyword> {
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
