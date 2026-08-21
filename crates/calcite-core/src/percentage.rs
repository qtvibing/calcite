use std::fmt;

use crate::Number;

#[derive(Clone, Debug, PartialEq)]
pub struct Percentage(Number);

impl Percentage {
    pub fn from_ratio(ratio: Number) -> Self {
        Self(ratio)
    }

    pub const fn ratio(&self) -> &Number {
        &self.0
    }

    pub(crate) fn into_ratio(self) -> Number {
        self.0
    }

    pub(crate) fn negated(self) -> Self {
        Self(self.0.negated())
    }
}

impl fmt::Display for Percentage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = self.0.clone().multiply(Number::from_i64(100));
        write!(formatter, "{value}%")
    }
}
