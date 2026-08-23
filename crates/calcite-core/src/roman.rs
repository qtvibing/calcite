use std::fmt;

use crate::{Number, NumberParseError};

const MAX_ROMAN_VALUE: u32 = 3_999;

#[derive(Clone, Debug, PartialEq)]
pub struct RomanInteger {
    value: Number,
}

impl RomanInteger {
    pub fn parse(source: &str) -> Result<Self, NumberParseError> {
        if source.is_empty() {
            return Err(NumberParseError::Invalid);
        }
        let upper = source.to_ascii_uppercase();
        let mut remainder = upper.as_str();
        let mut value = 0u32;
        for (symbol, amount) in ROMAN_DIGITS {
            while remainder.starts_with(symbol) {
                value = value
                    .checked_add(amount)
                    .ok_or(NumberParseError::TooLarge)?;
                remainder = &remainder[symbol.len()..];
            }
        }
        if !remainder.is_empty()
            || value == 0
            || value > MAX_ROMAN_VALUE
            || format_roman(value) != upper
        {
            return Err(NumberParseError::Invalid);
        }
        Ok(Self {
            value: Number::from_i64(i64::from(value)),
        })
    }

    pub(crate) fn new(value: Number) -> Result<Self, NumberParseError> {
        let value_u32 = value
            .exact_i64()
            .and_then(|value| u32::try_from(value).ok())
            .filter(|value| (1..=MAX_ROMAN_VALUE).contains(value))
            .ok_or(NumberParseError::Invalid)?;
        Ok(Self {
            value: Number::from_i64(i64::from(value_u32)),
        })
    }

    pub const fn value(&self) -> &Number {
        &self.value
    }

    pub(crate) fn into_number(self) -> Number {
        self.value
    }
}

impl fmt::Display for RomanInteger {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = self
            .value
            .exact_i64()
            .expect("Roman integers contain bounded exact integers") as u32;
        formatter.write_str(&format_roman(value))
    }
}

const ROMAN_DIGITS: [(&str, u32); 13] = [
    ("M", 1000),
    ("CM", 900),
    ("D", 500),
    ("CD", 400),
    ("C", 100),
    ("XC", 90),
    ("L", 50),
    ("XL", 40),
    ("X", 10),
    ("IX", 9),
    ("V", 5),
    ("IV", 4),
    ("I", 1),
];

fn format_roman(mut value: u32) -> String {
    let mut output = String::new();
    for (symbol, amount) in ROMAN_DIGITS {
        while value >= amount {
            output.push_str(symbol);
            value -= amount;
        }
    }
    output
}
