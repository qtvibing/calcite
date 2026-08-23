use std::{cmp::Ordering, error::Error, fmt};

use dashu_float::{
    FBig,
    round::mode::{Down, HalfAway, Up},
};
use dashu_int::{IBig, ops::UnsignedAbs};
use dashu_ratio::RBig;

use crate::{NumberOutputMode, Radix};

const WORKING_PRECISION: usize = 34;
const DISPLAY_DECIMAL_PLACES: usize = 10;
const MAX_LITERAL_DIGITS: usize = 10_000;
const MAX_LITERAL_EXPONENT: u32 = 100_000;
const MAX_INTEGER_EXPONENT: isize = 100_000;
const MAX_ROUNDING_PLACES: u64 = 10_000;
const MAX_ROOT_DEGREE: usize = 1_000;
const MAX_EXPONENTIAL_ARGUMENT: i64 = 1_000;
const MAX_FACTORIAL_INPUT: usize = 10_000;
const MAX_COMBINATORIC_POPULATION: usize = 100_000;
const MAX_COMBINATORIC_SELECTION: usize = 10_000;
const MAX_SHIFT_COUNT: usize = 100_000;

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
    InvalidHyperbolic,
    IntegerRequired,
    InvalidCombinatorics,
    CombinatoricsTooLarge,
    InvalidShift,
    ShiftTooLarge,
    PrimalityInputTooLarge,
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

    pub(crate) fn parse_repeating_decimal(source: &str) -> Result<Self, NumberParseError> {
        let normalized = source.replace('_', "");
        let (prefix, repeating) = normalized
            .split_once('(')
            .ok_or(NumberParseError::Invalid)?;
        let repeating = repeating
            .strip_suffix(')')
            .filter(|digits| !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()))
            .ok_or(NumberParseError::Invalid)?;
        let (integer, non_repeating) = prefix.split_once('.').ok_or(NumberParseError::Invalid)?;
        let integer = if integer.is_empty() { "0" } else { integer };
        if !integer.bytes().all(|byte| byte.is_ascii_digit())
            || !non_repeating.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(NumberParseError::Invalid);
        }

        let upper = Self::parse_decimal(&format!("{integer}{non_repeating}{repeating}"))?;
        let lower = Self::parse_decimal(&format!("{integer}{non_repeating}"))?;
        let non_repeating_places =
            i64::try_from(non_repeating.len()).map_err(|_| NumberParseError::TooLarge)?;
        let all_places = i64::try_from(non_repeating.len() + repeating.len())
            .map_err(|_| NumberParseError::TooLarge)?;
        let denominator = Self::from_i64(10)
            .pow(Self::from_i64(all_places))
            .map_err(|_| NumberParseError::TooLarge)?
            .subtract(
                Self::from_i64(10)
                    .pow(Self::from_i64(non_repeating_places))
                    .map_err(|_| NumberParseError::TooLarge)?,
            );
        upper
            .subtract(lower)
            .checked_divide(denominator)
            .ok_or(NumberParseError::Invalid)
    }

    pub(crate) fn parse_radix_integer(source: &str, radix: u8) -> Result<Self, NumberParseError> {
        let digits = source.chars().filter(|character| *character != '_').count();
        if digits == 0 {
            return Err(NumberParseError::Invalid);
        }
        if digits > MAX_LITERAL_DIGITS {
            return Err(NumberParseError::TooLarge);
        }
        let normalized = source.replace('_', "");
        IBig::from_str_radix(&normalized, u32::from(radix))
            .map(|value| Self(Representation::Exact(RBig::from(value))))
            .map_err(|_| NumberParseError::Invalid)
    }

    pub const fn is_exact(&self) -> bool {
        matches!(self.0, Representation::Exact(_))
    }

    pub(crate) fn is_integer(&self) -> bool {
        match &self.0 {
            Representation::Exact(value) => value.denominator() == &1u8.into(),
            Representation::Approximate(_) => false,
        }
    }

    pub(crate) fn formatted_radix(&self, radix: Radix) -> String {
        let value = integer_value(self.0.clone()).expect("radix values are exact integers");
        let digits = value.in_radix(radix.value()).to_string();
        let prefix = radix
            .prefix()
            .map(str::to_owned)
            .unwrap_or_else(|| format!("{}#", radix.value()));
        if let Some(digits) = digits.strip_prefix('-') {
            format!("-{prefix}{digits}")
        } else {
            format!("{prefix}{digits}")
        }
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

    pub fn checked_divide(self, other: Self) -> Option<Self> {
        self.divide(other).ok()
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

    pub(crate) fn remainder(self, other: Self) -> Result<Self, NumericError> {
        let (left, right) = integer_operands(self.0, other.0)?;
        if right == IBig::ZERO {
            return Err(NumericError::DivisionByZero);
        }
        Ok(Self(Representation::Exact(RBig::from(left % right))))
    }

    pub(crate) fn modulo(self, other: Self) -> Result<Self, NumericError> {
        let (left, right) = integer_operands(self.0, other.0)?;
        if right == IBig::ZERO {
            return Err(NumericError::DivisionByZero);
        }
        let modulus = if right < IBig::ZERO { -right } else { right };
        let remainder = left % &modulus;
        let result = if remainder < IBig::ZERO {
            remainder + modulus
        } else {
            remainder
        };
        Ok(Self(Representation::Exact(RBig::from(result))))
    }

    pub(crate) fn bitwise_and(self, other: Self) -> Result<Self, NumericError> {
        let (left, right) = integer_operands(self.0, other.0)?;
        Ok(Self(Representation::Exact(RBig::from(left & right))))
    }

    pub(crate) fn bitwise_or(self, other: Self) -> Result<Self, NumericError> {
        let (left, right) = integer_operands(self.0, other.0)?;
        Ok(Self(Representation::Exact(RBig::from(left | right))))
    }

    pub(crate) fn bitwise_xor(self, other: Self) -> Result<Self, NumericError> {
        let (left, right) = integer_operands(self.0, other.0)?;
        Ok(Self(Representation::Exact(RBig::from(left ^ right))))
    }

    pub(crate) fn shift_left(self, count: Self) -> Result<Self, NumericError> {
        let value = integer_value(self.0)?;
        let count = shift_count(count.0)?;
        Ok(Self(Representation::Exact(RBig::from(value << count))))
    }

    pub(crate) fn shift_right(self, count: Self) -> Result<Self, NumericError> {
        let value = integer_value(self.0)?;
        let count = shift_count(count.0)?;
        Ok(Self(Representation::Exact(RBig::from(value >> count))))
    }

    pub(crate) fn greatest_common_divisor(self, other: Self) -> Result<Self, NumericError> {
        let (left, right) = integer_operands(self.0, other.0)?;
        let divisor = unsigned_gcd(left.unsigned_abs(), right.unsigned_abs());
        Ok(Self(Representation::Exact(RBig::from(divisor))))
    }

    pub(crate) fn least_common_multiple(self, other: Self) -> Result<Self, NumericError> {
        let (left, right) = integer_operands(self.0, other.0)?;
        let left = left.unsigned_abs();
        let right = right.unsigned_abs();
        if left == dashu_int::UBig::ZERO || right == dashu_int::UBig::ZERO {
            return Ok(Self::from_i64(0));
        }
        let divisor = unsigned_gcd(left.clone(), right.clone());
        Ok(Self(Representation::Exact(RBig::from(
            (left / divisor) * right,
        ))))
    }

    pub(crate) fn is_prime(&self) -> Result<bool, NumericError> {
        let value = integer_value(self.0.clone())?;
        if value < IBig::from(2) {
            return Ok(false);
        }
        let value = u64::try_from(&value).map_err(|_| NumericError::PrimalityInputTooLarge)?;
        Ok(is_prime_u64(value))
    }

    pub(crate) fn factorial(self) -> Result<Self, NumericError> {
        let n = bounded_nonnegative_integer(self.0, MAX_FACTORIAL_INPUT)?;
        let mut result = IBig::ONE;
        for factor in 2..=n {
            result *= IBig::from(factor);
        }
        Ok(Self(Representation::Exact(RBig::from(result))))
    }

    pub(crate) fn permutation(self, selection: Self) -> Result<Self, NumericError> {
        let population = bounded_nonnegative_integer(self.0, MAX_COMBINATORIC_POPULATION)?;
        let selection = bounded_nonnegative_integer(selection.0, MAX_COMBINATORIC_SELECTION)?;
        if selection > population {
            return Err(NumericError::InvalidCombinatorics);
        }
        let mut result = IBig::ONE;
        for value in population - selection + 1..=population {
            result *= IBig::from(value);
        }
        Ok(Self(Representation::Exact(RBig::from(result))))
    }

    pub(crate) fn combination(self, selection: Self) -> Result<Self, NumericError> {
        let population = bounded_nonnegative_integer(self.0, MAX_COMBINATORIC_POPULATION)?;
        let selection = bounded_nonnegative_integer(selection.0, MAX_COMBINATORIC_POPULATION)?;
        if selection > population {
            return Err(NumericError::InvalidCombinatorics);
        }
        let selection = selection.min(population - selection);
        if selection > MAX_COMBINATORIC_SELECTION {
            return Err(NumericError::CombinatoricsTooLarge);
        }
        let mut result = IBig::ONE;
        for index in 1..=selection {
            result *= IBig::from(population - selection + index);
            result /= IBig::from(index);
        }
        Ok(Self(Representation::Exact(RBig::from(result))))
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

    pub(crate) fn e() -> Self {
        Self(Representation::Approximate(Real::e(WORKING_PRECISION)))
    }

    pub(crate) fn exponential(self) -> Result<Self, NumericError> {
        let value = into_real(self.0);
        let limit = Real::from(MAX_EXPONENTIAL_ARGUMENT);
        if value > limit || value < -limit {
            return Err(NumericError::ExponentTooLarge);
        }
        Ok(Self(Representation::Approximate(value.exp())))
    }

    pub(crate) fn hyperbolic_sine(self) -> Result<Self, NumericError> {
        self.bounded_hyperbolic(Real::sinh)
    }

    pub(crate) fn hyperbolic_cosine(self) -> Result<Self, NumericError> {
        self.bounded_hyperbolic(Real::cosh)
    }

    pub(crate) fn hyperbolic_tangent(self) -> Result<Self, NumericError> {
        self.bounded_hyperbolic(Real::tanh)
    }

    fn bounded_hyperbolic(
        self,
        operation: impl FnOnce(&Real) -> Real,
    ) -> Result<Self, NumericError> {
        let value = into_real(self.0);
        let limit = Real::from(MAX_EXPONENTIAL_ARGUMENT);
        if value > limit || value < -limit {
            return Err(NumericError::ExponentTooLarge);
        }
        Ok(Self(Representation::Approximate(operation(&value))))
    }

    pub(crate) fn inverse_hyperbolic_sine(self) -> Self {
        Self(Representation::Approximate(into_real(self.0).asinh()))
    }

    pub(crate) fn inverse_hyperbolic_cosine(self) -> Result<Self, NumericError> {
        let value = into_real(self.0);
        if value < Real::ONE {
            return Err(NumericError::InvalidHyperbolic);
        }
        Ok(Self(Representation::Approximate(value.acosh())))
    }

    pub(crate) fn inverse_hyperbolic_tangent(self) -> Result<Self, NumericError> {
        let value = into_real(self.0);
        if value <= -Real::ONE || value >= Real::ONE {
            return Err(NumericError::InvalidHyperbolic);
        }
        Ok(Self(Representation::Approximate(value.atanh())))
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

    pub(crate) fn is_zero(&self) -> bool {
        match &self.0 {
            Representation::Exact(value) => value == &RBig::ZERO,
            Representation::Approximate(value) => value == &Real::ZERO,
        }
    }

    pub(crate) fn formatted_with(&self, decimal_separator: char) -> String {
        self.formatted_with_mode(NumberOutputMode::Auto, decimal_separator, false)
    }

    pub(crate) fn formatted_with_mode(
        &self,
        mode: NumberOutputMode,
        decimal_separator: char,
        mark_approximate: bool,
    ) -> String {
        let (rendered, exact_rendering) = match mode {
            NumberOutputMode::Auto => {
                (self.format_decimal(DISPLAY_DECIMAL_PLACES), self.is_exact())
            }
            NumberOutputMode::Exact | NumberOutputMode::Fraction => match &self.0 {
                Representation::Exact(value) => (value.to_string(), true),
                Representation::Approximate(_) => {
                    (self.format_decimal(DISPLAY_DECIMAL_PLACES), false)
                }
            },
            NumberOutputMode::Mixed => match &self.0 {
                Representation::Exact(value) => (format_mixed(value), true),
                Representation::Approximate(_) => {
                    (self.format_decimal(DISPLAY_DECIMAL_PLACES), false)
                }
            },
            NumberOutputMode::Decimal { places } => {
                let places = usize::from(places).min(100);
                let rendered = self.format_decimal(places);
                let exact = Number::parse_decimal(&rendered)
                    .ok()
                    .is_some_and(|rendered| rendered == *self);
                (rendered, exact)
            }
            NumberOutputMode::Significant { digits } => {
                let digits = usize::from(digits).clamp(1, WORKING_PRECISION);
                let scientific = self.format_scientific(digits);
                let rendered = expand_scientific(&scientific);
                let exact = Number::parse_decimal(&rendered)
                    .ok()
                    .is_some_and(|rendered| rendered == *self);
                (rendered, exact)
            }
            NumberOutputMode::Scientific { digits } => {
                let digits = usize::from(digits).clamp(1, WORKING_PRECISION);
                let rendered = self.format_scientific(digits);
                let exact = Number::parse_decimal(&rendered)
                    .ok()
                    .is_some_and(|rendered| rendered == *self);
                (rendered, exact)
            }
        };
        let rendered = if decimal_separator == '.' {
            rendered
        } else {
            rendered.replace('.', &decimal_separator.to_string())
        };
        if mark_approximate && !exact_rendering {
            format!("≈{rendered}")
        } else {
            rendered
        }
    }

    fn format_decimal(&self, places: usize) -> String {
        trim_decimal_zeros(match &self.0 {
            Representation::Exact(value) if value.denominator() == &dashu_int::UBig::ONE => {
                value.numerator().to_string()
            }
            Representation::Exact(value) => format!("{:.places$}", value.in_expanded(10)),
            Representation::Approximate(value) => format!("{value:.places$}"),
        })
    }

    fn format_scientific(&self, digits: usize) -> String {
        let decimal_places = digits.saturating_sub(1);
        let value = into_real(self.0.clone());
        trim_decimal_zeros(format!("{value:.decimal_places$e}"))
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

fn integer_operands(
    left: Representation,
    right: Representation,
) -> Result<(IBig, IBig), NumericError> {
    Ok((integer_value(left)?, integer_value(right)?))
}

fn integer_value(value: Representation) -> Result<IBig, NumericError> {
    let Representation::Exact(value) = value else {
        return Err(NumericError::IntegerRequired);
    };
    let (numerator, denominator) = value.into_parts();
    if denominator != 1u8.into() {
        return Err(NumericError::IntegerRequired);
    }
    Ok(numerator)
}

fn unsigned_gcd(mut left: dashu_int::UBig, mut right: dashu_int::UBig) -> dashu_int::UBig {
    while right != dashu_int::UBig::ZERO {
        let remainder = &left % &right;
        left = right;
        right = remainder;
    }
    left
}

fn is_prime_u64(value: u64) -> bool {
    const SMALL_PRIMES: [u64; 12] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37];
    if SMALL_PRIMES.contains(&value) {
        return true;
    }
    if SMALL_PRIMES
        .iter()
        .any(|prime| value.is_multiple_of(*prime))
    {
        return false;
    }

    let mut odd_part = value - 1;
    let powers_of_two = odd_part.trailing_zeros();
    odd_part >>= powers_of_two;
    for base in [2_u64, 325, 9_375, 28_178, 450_775, 9_780_504, 1_795_265_022] {
        if base.is_multiple_of(value) {
            continue;
        }
        let mut witness = modular_power(base % value, odd_part, value);
        if witness == 1 || witness == value - 1 {
            continue;
        }
        let mut composite = true;
        for _ in 1..powers_of_two {
            witness = ((witness as u128 * witness as u128) % value as u128) as u64;
            if witness == value - 1 {
                composite = false;
                break;
            }
        }
        if composite {
            return false;
        }
    }
    true
}

fn modular_power(mut base: u64, mut exponent: u64, modulus: u64) -> u64 {
    let mut result = 1_u64;
    while exponent > 0 {
        if exponent & 1 == 1 {
            result = ((result as u128 * base as u128) % modulus as u128) as u64;
        }
        base = ((base as u128 * base as u128) % modulus as u128) as u64;
        exponent >>= 1;
    }
    result
}

fn shift_count(value: Representation) -> Result<usize, NumericError> {
    let value = integer_value(value)?;
    if value < IBig::ZERO {
        return Err(NumericError::InvalidShift);
    }
    if value > IBig::from(MAX_SHIFT_COUNT) {
        return Err(NumericError::ShiftTooLarge);
    }
    value
        .to_string()
        .parse()
        .map_err(|_| NumericError::ShiftTooLarge)
}

fn bounded_nonnegative_integer(
    value: Representation,
    maximum: usize,
) -> Result<usize, NumericError> {
    let value = integer_value(value)?;
    if value < IBig::ZERO {
        return Err(NumericError::InvalidCombinatorics);
    }
    if value > IBig::from(maximum) {
        return Err(NumericError::CombinatoricsTooLarge);
    }
    value
        .to_string()
        .parse()
        .map_err(|_| NumericError::CombinatoricsTooLarge)
}

fn into_real(value: Representation) -> Real {
    match value {
        Representation::Exact(value) => value.to_float(WORKING_PRECISION).value(),
        Representation::Approximate(value) => value,
    }
}

fn format_mixed(value: &RBig) -> String {
    if value.denominator() == &dashu_int::UBig::ONE {
        return value.numerator().to_string();
    }
    let denominator = IBig::from(value.denominator().clone());
    let whole = value.numerator() / &denominator;
    let remainder = (value.numerator() % &denominator).unsigned_abs();
    if whole == IBig::ZERO {
        if value.numerator() < &IBig::ZERO {
            format!("-{remainder}/{}", value.denominator())
        } else {
            format!("{remainder}/{}", value.denominator())
        }
    } else if remainder == dashu_int::UBig::ZERO {
        whole.to_string()
    } else {
        format!("{whole} {remainder}/{}", value.denominator())
    }
}

fn expand_scientific(value: &str) -> String {
    let Some((coefficient, exponent)) = value.split_once('e') else {
        return value.to_owned();
    };
    let Ok(exponent) = exponent.parse::<isize>() else {
        return value.to_owned();
    };
    let negative = coefficient.starts_with('-');
    let coefficient = coefficient.trim_start_matches('-');
    let decimal_position = coefficient.find('.').unwrap_or(coefficient.len()) as isize + exponent;
    let digits = coefficient.replace('.', "");
    let mut expanded = if decimal_position <= 0 {
        format!("0.{}{}", "0".repeat((-decimal_position) as usize), digits)
    } else if decimal_position as usize >= digits.len() {
        format!(
            "{}{}",
            digits,
            "0".repeat(decimal_position as usize - digits.len())
        )
    } else {
        let position = decimal_position as usize;
        format!("{}.{}", &digits[..position], &digits[position..])
    };
    if negative {
        expanded.insert(0, '-');
    }
    expanded
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
