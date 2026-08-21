use std::fmt;

use crate::Number;

#[derive(Clone, Debug, PartialEq)]
pub struct Fraction(Number);

impl Fraction {
    pub fn from_number(number: Number) -> Option<Self> {
        number.is_exact().then_some(Self(number))
    }

    pub const fn number(&self) -> &Number {
        &self.0
    }

    pub(crate) fn into_number(self) -> Number {
        self.0
    }

    pub(crate) fn negated(self) -> Self {
        Self(self.0.negated())
    }
}

impl fmt::Display for Fraction {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            self.0
                .exact_fraction()
                .as_deref()
                .expect("fractions always contain exact numbers"),
        )
    }
}
