use std::fmt;

use crate::{Number, NumberParseError};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Radix(u8);

#[allow(non_upper_case_globals)]
impl Radix {
    pub const Binary: Self = Self(2);
    pub const Octal: Self = Self(8);
    pub const Decimal: Self = Self(10);
    pub const Hexadecimal: Self = Self(16);

    pub const fn new(value: u8) -> Option<Self> {
        if value >= 2 && value <= 36 {
            Some(Self(value))
        } else {
            None
        }
    }

    pub const fn value(self) -> u8 {
        self.0
    }

    pub const fn prefix(self) -> Option<&'static str> {
        match self.0 {
            2 => Some("0b"),
            8 => Some("0o"),
            16 => Some("0x"),
            _ => None,
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
        } else if let Some((base, digits)) = source.split_once('#') {
            let base = base
                .parse::<u8>()
                .ok()
                .and_then(Radix::new)
                .ok_or(NumberParseError::Invalid)?;
            (base, digits)
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
