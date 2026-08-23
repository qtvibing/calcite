use std::fmt;

use crate::unit::UnitError;
use crate::{CompoundUnit, Currency, Money, Number, UnitValue};

#[derive(Clone, Debug, PartialEq)]
pub struct MoneyRate {
    amount: Number,
    currency: Currency,
    denominator: CompoundUnit,
}

impl MoneyRate {
    pub fn new(amount: Number, currency: Currency, denominator: CompoundUnit) -> Self {
        Self {
            amount,
            currency,
            denominator,
        }
    }

    pub const fn amount(&self) -> &Number {
        &self.amount
    }

    pub const fn currency(&self) -> Currency {
        self.currency
    }

    pub const fn denominator(&self) -> &CompoundUnit {
        &self.denominator
    }

    pub(crate) fn converted_denominator(&self, target: &CompoundUnit) -> Result<Self, UnitError> {
        let source_rate_unit = CompoundUnit::default().divide(&self.denominator);
        let target_rate_unit = CompoundUnit::default().divide(target);
        let converted = UnitValue::from_compound(self.amount.clone(), source_rate_unit)
            .converted_to_compound(&target_rate_unit)?;
        Ok(Self::new(
            converted.magnitude().clone(),
            self.currency,
            target.clone(),
        ))
    }

    pub(crate) fn with_currency(self, amount: Number, currency: Currency) -> Self {
        Self::new(amount, currency, self.denominator)
    }

    pub(crate) fn scaled(self, factor: Number) -> Self {
        Self::new(
            self.amount.multiply(factor),
            self.currency,
            self.denominator,
        )
    }

    pub(crate) fn negated(self) -> Self {
        Self::new(self.amount.negated(), self.currency, self.denominator)
    }

    pub(crate) fn apply_to(self, quantity: UnitValue) -> Result<Money, UnitError> {
        let quantity = quantity.converted_to_compound(&self.denominator)?;
        Ok(Money::new(
            self.amount.multiply(quantity.magnitude().clone()),
            self.currency,
        ))
    }

    pub(crate) fn solve_quantity(self, money: Money) -> Result<UnitValue, UnitError> {
        money
            .amount()
            .clone()
            .divide(self.amount)
            .map(|magnitude| UnitValue::from_compound(magnitude, self.denominator))
            .map_err(UnitError::Numeric)
    }

    pub(crate) fn formatted_with(&self, amount: &str) -> String {
        format!("{amount} {}/{}", self.currency, self.denominator)
    }
}

impl fmt::Display for MoneyRate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.formatted_with(&self.amount.to_string()))
    }
}
