use std::fmt;

use crate::{Number, NumberParseError};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Radix {
    Binary,
    Octal,
    Hexadecimal,
}

impl Radix {
    pub const fn value(self) -> u8 {
        match self {
            Self::Binary => 2,
            Self::Octal => 8,
            Self::Hexadecimal => 16,
        }
    }

    pub const fn prefix(self) -> &'static str {
        match self {
            Self::Binary => "0b",
            Self::Octal => "0o",
            Self::Hexadecimal => "0x",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RadixInteger {
    value: Number,
    radix: Radix,
}

impl RadixInteger {
    pub fn parse(source: &str) -> Result<Self, NumberParseError> {
        let (radix, digits) = if let Some(digits) = source
            .strip_prefix("0b")
            .or_else(|| source.strip_prefix("0B"))
        {
            (Radix::Binary, digits)
        } else if let Some(digits) = source
            .strip_prefix("0o")
            .or_else(|| source.strip_prefix("0O"))
        {
            (Radix::Octal, digits)
        } else if let Some(digits) = source
            .strip_prefix("0x")
            .or_else(|| source.strip_prefix("0X"))
        {
            (Radix::Hexadecimal, digits)
        } else {
            return Err(NumberParseError::Invalid);
        };
        Number::parse_radix_integer(digits, radix.value()).map(|value| Self { value, radix })
    }

    pub(crate) fn new(value: Number, radix: Radix) -> Result<Self, NumberParseError> {
        if !value.is_integer() {
            return Err(NumberParseError::Invalid);
        }
        Ok(Self { value, radix })
    }

    pub const fn value(&self) -> &Number {
        &self.value
    }

    pub const fn radix(&self) -> Radix {
        self.radix
    }

    pub(crate) fn into_number(self) -> Number {
        self.value
    }

    pub(crate) fn negated(self) -> Self {
        Self {
            value: self.value.negated(),
            radix: self.radix,
        }
    }
}

impl fmt::Display for RadixInteger {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.value.formatted_radix(self.radix))
    }
}
