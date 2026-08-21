use std::{cmp::Ordering, error::Error, fmt};

use dashu_float::{
    FBig,
    round::mode::{Down, HalfAway, Up},
};
use dashu_ratio::RBig;

const WORKING_PRECISION: usize = 34;
const DISPLAY_DECIMAL_PLACES: usize = 10;
const MAX_LITERAL_DIGITS: usize = 10_000;
const MAX_LITERAL_EXPONENT: u32 = 100_000;
const MAX_INTEGER_EXPONENT: isize = 100_000;
const MAX_ROUNDING_PLACES: u64 = 10_000;
const MAX_ROOT_DEGREE: usize = 1_000;

type Real = FBig<HalfAway, 10>;

#[derive(Clone, Debug, PartialEq)]
enum Representation {
    Exact(RBig),
    Approximate(Real),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Number(Representation);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumberParseError {
    Invalid,
    TooLarge,
}

impl fmt::Display for NumberParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid => formatter.write_str("invalid decimal number"),
            Self::TooLarge => formatter.write_str("decimal number exceeds Calcite's limits"),
        }
    }
}

impl Error for NumberParseError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NumericError {
    DivisionByZero,
    ExponentTooLarge,
    PrecisionTooLarge,
    RootDegreeTooLarge,
    InvalidLogarithm,
    InvalidTrigonometry,
    NonRealResult,
}

impl Number {
    pub(crate) fn from_i64(value: i64) -> Self {
        Self(Representation::Exact(RBig::from(value)))
    }

    pub fn parse_decimal(source: &str) -> Result<Self, NumberParseError> {
        validate_decimal_size(source)?;
        RBig::from_str_decimal(source)
            .map(|value| Self(Representation::Exact(value)))
            .map_err(|_| NumberParseError::Invalid)
    }

    pub const fn is_exact(&self) -> bool {
        matches!(self.0, Representation::Exact(_))
    }

    pub fn exact_fraction(&self) -> Option<String> {
        match &self.0 {
            Representation::Exact(value) => Some(value.to_string()),
            Representation::Approximate(_) => None,
        }
    }

    pub(crate) fn exact_i64(&self) -> Option<i64> {
        let value = self.exact_fraction()?;
        if value.contains('/') {
            None
        } else {
            value.parse().ok()
        }
    }

    pub(crate) fn negated(self) -> Self {
        match self.0 {
            Representation::Exact(value) => Self(Representation::Exact(-value)),
            Representation::Approximate(value) => Self(Representation::Approximate(-value)),
        }
    }

    pub(crate) fn add(self, other: Self) -> Self {
        match (self.0, other.0) {
            (Representation::Exact(left), Representation::Exact(right)) => {
                Self(Representation::Exact(left + right))
            }
            (left, right) => Self(Representation::Approximate(
                into_real(left) + into_real(right),
            )),
        }
    }

    pub(crate) fn subtract(self, other: Self) -> Self {
        match (self.0, other.0) {
            (Representation::Exact(left), Representation::Exact(right)) => {
                Self(Representation::Exact(left - right))
            }
            (left, right) => Self(Representation::Approximate(
                into_real(left) - into_real(right),
            )),
        }
    }

    pub(crate) fn multiply(self, other: Self) -> Self {
        match (self.0, other.0) {
            (Representation::Exact(left), Representation::Exact(right)) => {
                Self(Representation::Exact(left * right))
            }
            (left, right) => Self(Representation::Approximate(
                into_real(left) * into_real(right),
            )),
        }
    }

    pub(crate) fn divide(self, other: Self) -> Result<Self, NumericError> {
        if other.is_zero() {
            return Err(NumericError::DivisionByZero);
        }

        Ok(match (self.0, other.0) {
            (Representation::Exact(left), Representation::Exact(right)) => {
                Self(Representation::Exact(left / right))
            }
            (left, right) => Self(Representation::Approximate(
                into_real(left) / into_real(right),
            )),
        })
    }

    pub(crate) fn compare(&self, other: &Self) -> Option<Ordering> {
        match (&self.0, &other.0) {
            (Representation::Exact(left), Representation::Exact(right)) => Some(left.cmp(right)),
            (left, right) => into_real(left.clone()).partial_cmp(&into_real(right.clone())),
        }
    }

    pub(crate) fn pi() -> Self {
        Self(Representation::Approximate(Real::pi(WORKING_PRECISION)))
    }

    pub(crate) fn sine(self) -> Self {
        Self(Representation::Approximate(into_real(self.0).sin()))
    }

    pub(crate) fn cosine(self) -> Self {
        Self(Representation::Approximate(into_real(self.0).cos()))
    }

    pub(crate) fn tangent(self) -> Self {
        Self(Representation::Approximate(into_real(self.0).tan()))
    }

    pub(crate) fn arc_sine(self) -> Result<Self, NumericError> {
        let value = into_real(self.0);
        if value < -Real::ONE || value > Real::ONE {
            return Err(NumericError::InvalidTrigonometry);
        }
        Ok(Self(Representation::Approximate(value.asin())))
    }

    pub(crate) fn arc_cosine(self) -> Result<Self, NumericError> {
        let value = into_real(self.0);
        if value < -Real::ONE || value > Real::ONE {
            return Err(NumericError::InvalidTrigonometry);
        }
        Ok(Self(Representation::Approximate(value.acos())))
    }

    pub(crate) fn arc_tangent(self) -> Self {
        Self(Representation::Approximate(into_real(self.0).atan()))
    }

    pub(crate) fn arc_tangent2(self, x: Self) -> Result<Self, NumericError> {
        if self.is_zero() && x.is_zero() {
            return Err(NumericError::InvalidTrigonometry);
        }
        Ok(Self(Representation::Approximate(
            into_real(self.0).atan2(&into_real(x.0)),
        )))
    }

    pub(crate) fn nth_root(self, degree: usize) -> Result<Self, NumericError> {
        if degree == 0 {
            return Err(NumericError::NonRealResult);
        }
        if degree > MAX_ROOT_DEGREE {
            return Err(NumericError::RootDegreeTooLarge);
        }
        let value = into_real(self.0);
        if value < Real::ZERO && degree.is_multiple_of(2) {
            return Err(NumericError::NonRealResult);
        }
        Ok(Self(Representation::Approximate(value.nth_root(degree))))
    }

    pub(crate) fn natural_log(self) -> Result<Self, NumericError> {
        let value = into_real(self.0);
        if value <= Real::ZERO {
            return Err(NumericError::InvalidLogarithm);
        }
        Ok(Self(Representation::Approximate(value.ln())))
    }

    pub(crate) fn common_log(self) -> Result<Self, NumericError> {
        let value = into_real(self.0);
        if value <= Real::ZERO {
            return Err(NumericError::InvalidLogarithm);
        }
        Ok(Self(Representation::Approximate(value.log10())))
    }

    pub(crate) fn binary_log(self) -> Result<Self, NumericError> {
        let value = into_real(self.0);
        if value <= Real::ZERO {
            return Err(NumericError::InvalidLogarithm);
        }
        Ok(Self(Representation::Approximate(value.log2())))
    }

    pub(crate) fn logarithm(self, base: Self) -> Result<Self, NumericError> {
        let value = into_real(self.0);
        let base = into_real(base.0);
        if value <= Real::ZERO || base <= Real::ZERO || base == Real::ONE {
            return Err(NumericError::InvalidLogarithm);
        }
        Ok(Self(Representation::Approximate(value.ln() / base.ln())))
    }

    pub(crate) fn rounded(self, decimal_places: i64) -> Result<Self, NumericError> {
        self.round_with(decimal_places, Rounding::Nearest)
    }

    pub(crate) fn floor(self) -> Self {
        self.round_with(0, Rounding::Floor)
            .expect("zero decimal places are within the rounding limit")
    }

    pub(crate) fn ceiling(self) -> Self {
        self.round_with(0, Rounding::Ceiling)
            .expect("zero decimal places are within the rounding limit")
    }

    fn round_with(self, decimal_places: i64, rounding: Rounding) -> Result<Self, NumericError> {
        if decimal_places.unsigned_abs() > MAX_ROUNDING_PLACES {
            return Err(NumericError::PrecisionTooLarge);
        }
        let power = isize::try_from(decimal_places.unsigned_abs())
            .map_err(|_| NumericError::PrecisionTooLarge)?;
        let factor = Self(Representation::Exact(RBig::from(10).pow(power)));
        let scaled = if decimal_places >= 0 {
            self.multiply(factor.clone())
        } else {
            self.divide(factor.clone())?
        };
        let integer = match scaled.0 {
            Representation::Exact(value) => match rounding {
                Rounding::Nearest => value.round(),
                Rounding::Floor => value.floor(),
                Rounding::Ceiling => value.ceil(),
            },
            Representation::Approximate(value) => match rounding {
                Rounding::Nearest => value.to_int().value(),
                Rounding::Floor => value.with_rounding::<Down>().to_int().value(),
                Rounding::Ceiling => value.with_rounding::<Up>().to_int().value(),
            },
        };
        let integer = Self(Representation::Exact(RBig::from(integer)));
        if decimal_places >= 0 {
            integer.divide(factor)
        } else {
            Ok(integer.multiply(factor))
        }
    }

    pub(crate) fn pow(self, exponent: Self) -> Result<Self, NumericError> {
        if let Representation::Exact(exact_exponent) = &exponent.0 {
            let (integer, fraction) = exact_exponent.clone().split_at_point();
            if fraction == RBig::ZERO {
                let integer =
                    isize::try_from(&integer).map_err(|_| NumericError::ExponentTooLarge)?;
                if integer.unsigned_abs() > MAX_INTEGER_EXPONENT as usize {
                    return Err(NumericError::ExponentTooLarge);
                }
                if self.is_zero() && integer < 0 {
                    return Err(NumericError::DivisionByZero);
                }

                return Ok(match self.0 {
                    Representation::Exact(value) => Self(Representation::Exact(value.pow(integer))),
                    Representation::Approximate(value) => {
                        Self(Representation::Approximate(value.powi(integer.into())))
                    }
                });
            }
        }

        let base = into_real(self.0);
        let exponent = into_real(exponent.0);
        let limit = Real::from(MAX_INTEGER_EXPONENT);
        if exponent > limit || exponent < -limit {
            return Err(NumericError::ExponentTooLarge);
        }
        if base < Real::ZERO {
            return Err(NumericError::NonRealResult);
        }
        if base == Real::ZERO && exponent < Real::ZERO {
            return Err(NumericError::DivisionByZero);
        }

        Ok(Self(Representation::Approximate(base.powf(&exponent))))
    }

    fn is_zero(&self) -> bool {
        match &self.0 {
            Representation::Exact(value) => value == &RBig::ZERO,
            Representation::Approximate(value) => value == &Real::ZERO,
        }
    }

    pub(crate) fn formatted_with(&self, decimal_separator: char) -> String {
        let rendered = match &self.0 {
            Representation::Exact(value) if value.denominator() == &dashu_int::UBig::ONE => {
                value.numerator().to_string()
            }
            Representation::Exact(value) => {
                format!("{:.DISPLAY_DECIMAL_PLACES$}", value.in_expanded(10))
            }
            Representation::Approximate(value) => {
                format!("{value:.DISPLAY_DECIMAL_PLACES$}")
            }
        };

        let rendered = trim_decimal_zeros(rendered);
        if decimal_separator == '.' {
            rendered
        } else {
            rendered.replace('.', &decimal_separator.to_string())
        }
    }
}

impl fmt::Display for Number {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.formatted_with('.'))
    }
}

#[derive(Clone, Copy)]
enum Rounding {
    Nearest,
    Floor,
    Ceiling,
}

fn validate_decimal_size(source: &str) -> Result<(), NumberParseError> {
    let exponent_start = source.find(['e', 'E']);
    let mantissa = exponent_start.map_or(source, |at| &source[..at]);
    if mantissa.bytes().filter(u8::is_ascii_digit).count() > MAX_LITERAL_DIGITS {
        return Err(NumberParseError::TooLarge);
    }

    if let Some(at) = exponent_start {
        let exponent: String = source[at + 1..]
            .chars()
            .filter(|character| *character != '_')
            .collect();
        let exponent = exponent
            .parse::<i32>()
            .map_err(|_| NumberParseError::TooLarge)?;
        if exponent.unsigned_abs() > MAX_LITERAL_EXPONENT {
            return Err(NumberParseError::TooLarge);
        }
    }

    Ok(())
}

fn into_real(value: Representation) -> Real {
    match value {
        Representation::Exact(value) => value.to_float(WORKING_PRECISION).value(),
        Representation::Approximate(value) => value,
    }
}

fn trim_decimal_zeros(mut value: String) -> String {
    let exponent = value.find(['e', 'E']);
    let suffix = exponent.map(|at| value.split_off(at));

    if value.contains('.') {
        while value.ends_with('0') {
            value.pop();
        }
        if value.ends_with('.') {
            value.pop();
        }
    }

    if value == "-0" {
        value.clear();
        value.push('0');
    }
    if let Some(suffix) = suffix {
        value.push_str(&suffix);
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    fn number(source: &str) -> Number {
        Number::parse_decimal(source).unwrap()
    }

    #[test]
    fn decimal_literals_are_exact() {
        let result = number("0.1").add(number("0.2"));
        assert_eq!(result.exact_fraction().as_deref(), Some("3/10"));
        assert_eq!(result.to_string(), "0.3");
    }

    #[test]
    fn rational_arithmetic_stays_exact() {
        let result = number("1")
            .divide(number("3"))
            .unwrap()
            .multiply(number("3"));
        assert_eq!(result.exact_fraction().as_deref(), Some("1"));
        assert_eq!(result.to_string(), "1");
    }

    #[test]
    fn fractional_power_uses_configured_real_precision() {
        let result = number("2").pow(number("0.5")).unwrap();
        assert!(!result.is_exact());
        assert_eq!(result.to_string(), "1.4142135624");
    }

    #[test]
    fn rejects_pathological_numeric_literals_before_parsing_them() {
        assert_eq!(
            Number::parse_decimal("1e100001"),
            Err(NumberParseError::TooLarge)
        );
        assert_eq!(
            Number::parse_decimal(&"1".repeat(MAX_LITERAL_DIGITS + 1)),
            Err(NumberParseError::TooLarge)
        );
    }
}
