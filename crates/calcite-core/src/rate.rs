use std::fmt;

use crate::unit::UnitError;
use crate::{CompoundUnit, Number, Unit, UnitValue};

#[derive(Clone, Debug, PartialEq)]
pub struct Rate {
    magnitude: Number,
    unit: CompoundUnit,
}

impl Rate {
    pub fn new(magnitude: Number, numerator: Option<Unit>, denominator: Unit) -> Self {
        let numerator = numerator.map_or_else(CompoundUnit::default, CompoundUnit::from_unit);
        let unit = numerator.divide(&CompoundUnit::from_unit(denominator));
        Self { magnitude, unit }
    }

    pub(crate) const fn from_compound(magnitude: Number, unit: CompoundUnit) -> Self {
        Self { magnitude, unit }
    }

    pub const fn magnitude(&self) -> &Number {
        &self.magnitude
    }

    pub const fn compound_unit(&self) -> &CompoundUnit {
        &self.unit
    }

    pub(crate) fn into_parts(self) -> (Number, CompoundUnit) {
        (self.magnitude, self.unit)
    }

    pub(crate) fn converted_to_compound(&self, target: &CompoundUnit) -> Result<Self, UnitError> {
        let converted = UnitValue::from_compound(self.magnitude.clone(), self.unit.clone())
            .converted_to_compound(target)?;
        Ok(Self::from_compound(
            converted.magnitude().clone(),
            converted.compound_unit().clone(),
        ))
    }

    pub(crate) fn scaled(self, factor: Number) -> Self {
        Self::from_compound(self.magnitude.multiply(factor), self.unit)
    }

    pub(crate) fn divided_by(self, divisor: Number) -> Result<Self, UnitError> {
        self.magnitude
            .divide(divisor)
            .map(|magnitude| Self::from_compound(magnitude, self.unit))
            .map_err(UnitError::Numeric)
    }

    pub(crate) fn negated(self) -> Self {
        Self::from_compound(self.magnitude.negated(), self.unit)
    }

    pub(crate) fn formatted_with(&self, magnitude: &str) -> String {
        let unit = self.unit.formatted();
        if let Some(denominator) = unit.strip_prefix("1/") {
            format!("{magnitude}/{denominator}")
        } else {
            format!("{magnitude} {unit}")
        }
    }
}

impl fmt::Display for Rate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.formatted_with(&self.magnitude.to_string()))
    }
}
