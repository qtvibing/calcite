use std::fmt;

use crate::unit::UnitError;
use crate::{Number, Unit, UnitValue};

#[derive(Clone, Debug, PartialEq)]
pub struct Rate {
    magnitude: Number,
    numerator: Option<Unit>,
    denominator: Unit,
}

impl Rate {
    pub const fn new(magnitude: Number, numerator: Option<Unit>, denominator: Unit) -> Self {
        Self {
            magnitude,
            numerator,
            denominator,
        }
    }

    pub const fn magnitude(&self) -> &Number {
        &self.magnitude
    }

    pub const fn numerator(&self) -> Option<Unit> {
        self.numerator
    }

    pub const fn denominator(&self) -> Unit {
        self.denominator
    }

    pub(crate) fn converted_to(
        &self,
        numerator: Option<Unit>,
        denominator: Unit,
    ) -> Result<Self, UnitError> {
        let numerator_factor = match (self.numerator, numerator) {
            (Some(source), Some(target)) => source.linear_factor_to(target)?,
            (None, None) => Number::from_i64(1),
            _ => return Err(UnitError::IncompatibleDimensions),
        };
        let denominator_factor = self.denominator.linear_factor_to(denominator)?;
        let magnitude = self
            .magnitude
            .clone()
            .multiply(numerator_factor)
            .divide(denominator_factor)
            .map_err(UnitError::Numeric)?;
        Ok(Self::new(magnitude, numerator, denominator))
    }

    pub(crate) fn scaled(self, factor: Number) -> Self {
        Self::new(
            self.magnitude.multiply(factor),
            self.numerator,
            self.denominator,
        )
    }

    pub(crate) fn divided_by(self, divisor: Number) -> Result<Self, UnitError> {
        self.magnitude
            .divide(divisor)
            .map(|magnitude| Self::new(magnitude, self.numerator, self.denominator))
            .map_err(UnitError::Numeric)
    }

    pub(crate) fn apply(self, value: UnitValue) -> Result<RateApplication, UnitError> {
        let value = value.converted_to(self.denominator)?;
        let magnitude = self.magnitude.multiply(value.magnitude().clone());
        Ok(match self.numerator {
            Some(unit) => RateApplication::Unit(UnitValue::new(magnitude, unit)),
            None => RateApplication::Number(magnitude),
        })
    }

    pub(crate) fn negated(self) -> Self {
        Self::new(self.magnitude.negated(), self.numerator, self.denominator)
    }
}

impl fmt::Display for Rate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.numerator {
            Some(numerator) => write!(
                formatter,
                "{} {numerator}/{}",
                self.magnitude, self.denominator
            ),
            None => write!(formatter, "{}/{}", self.magnitude, self.denominator),
        }
    }
}

pub(crate) enum RateApplication {
    Number(Number),
    Unit(UnitValue),
}
