use std::{cmp::Ordering, fmt};

use crate::Number;

#[derive(Clone, Debug, PartialEq)]
pub struct ComplexNumber {
    real: Number,
    imaginary: Number,
}

impl ComplexNumber {
    pub const fn new(real: Number, imaginary: Number) -> Self {
        Self { real, imaginary }
    }

    pub fn imaginary_unit() -> Self {
        Self::new(Number::from_i64(0), Number::from_i64(1))
    }

    pub const fn real(&self) -> &Number {
        &self.real
    }

    pub const fn imaginary(&self) -> &Number {
        &self.imaginary
    }

    pub(crate) fn from_real(real: Number) -> Self {
        Self::new(real, Number::from_i64(0))
    }

    pub(crate) fn negated(self) -> Self {
        Self::new(self.real.negated(), self.imaginary.negated())
    }

    pub(crate) fn add(self, other: Self) -> Self {
        Self::new(
            self.real.add(other.real),
            self.imaginary.add(other.imaginary),
        )
    }

    pub(crate) fn subtract(self, other: Self) -> Self {
        Self::new(
            self.real.subtract(other.real),
            self.imaginary.subtract(other.imaginary),
        )
    }

    pub(crate) fn multiply(self, other: Self) -> Self {
        let real = self
            .real
            .clone()
            .multiply(other.real.clone())
            .subtract(self.imaginary.clone().multiply(other.imaginary.clone()));
        let imaginary = self
            .real
            .multiply(other.imaginary)
            .add(self.imaginary.multiply(other.real));
        Self::new(real, imaginary)
    }

    pub(crate) fn checked_divide(self, other: Self) -> Option<Self> {
        let denominator = other
            .real
            .clone()
            .multiply(other.real.clone())
            .add(other.imaginary.clone().multiply(other.imaginary.clone()));
        if denominator.is_zero() {
            return None;
        }
        let real = self
            .real
            .clone()
            .multiply(other.real.clone())
            .add(self.imaginary.clone().multiply(other.imaginary.clone()))
            .checked_divide(denominator.clone())?;
        let imaginary = self
            .imaginary
            .multiply(other.real)
            .subtract(self.real.multiply(other.imaginary))
            .checked_divide(denominator)?;
        Some(Self::new(real, imaginary))
    }

    pub(crate) fn pow(self, exponent: i64) -> Option<Self> {
        if exponent.unsigned_abs() > 10_000 {
            return None;
        }
        let negative = exponent < 0;
        let mut exponent = exponent.unsigned_abs();
        let mut factor = self;
        let mut result = Self::from_real(Number::from_i64(1));
        while exponent > 0 {
            if exponent & 1 == 1 {
                result = result.multiply(factor.clone());
            }
            exponent >>= 1;
            if exponent > 0 {
                factor = factor.clone().multiply(factor);
            }
        }
        if negative {
            Self::from_real(Number::from_i64(1)).checked_divide(result)
        } else {
            Some(result)
        }
    }

    pub(crate) fn is_real(&self) -> bool {
        self.imaginary.is_zero()
    }

    pub(crate) fn imaginary_sign(&self) -> Option<Ordering> {
        self.imaginary.compare(&Number::from_i64(0))
    }

    pub(crate) fn imaginary_magnitude(&self) -> Number {
        if self.imaginary_sign() == Some(Ordering::Less) {
            self.imaginary.clone().negated()
        } else {
            self.imaginary.clone()
        }
    }
}

impl fmt::Display for ComplexNumber {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.imaginary.is_zero() {
            return self.real.fmt(formatter);
        }
        if self.real.is_zero() {
            if self.imaginary == Number::from_i64(1) {
                return formatter.write_str("i");
            }
            if self.imaginary == Number::from_i64(-1) {
                return formatter.write_str("-i");
            }
            return write!(formatter, "{}i", self.imaginary);
        }
        let operator = if self.imaginary_sign() == Some(Ordering::Less) {
            '-'
        } else {
            '+'
        };
        let magnitude = self.imaginary_magnitude();
        if magnitude == Number::from_i64(1) {
            write!(formatter, "{} {operator} i", self.real)
        } else {
            write!(formatter, "{} {operator} {magnitude}i", self.real)
        }
    }
}
