use std::{cmp::Ordering, fmt};

use crate::{Number, NumericError};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Dimension {
    Length,
    Mass,
    Duration,
    Temperature,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Unit {
    Meter,
    Kilometer,
    Centimeter,
    Millimeter,
    Inch,
    Foot,
    Yard,
    Mile,
    Gram,
    Kilogram,
    Ounce,
    Pound,
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Celsius,
    Fahrenheit,
    Kelvin,
}

impl Unit {
    pub const fn dimension(self) -> Dimension {
        match self {
            Self::Meter
            | Self::Kilometer
            | Self::Centimeter
            | Self::Millimeter
            | Self::Inch
            | Self::Foot
            | Self::Yard
            | Self::Mile => Dimension::Length,
            Self::Gram | Self::Kilogram | Self::Ounce | Self::Pound => Dimension::Mass,
            Self::Second | Self::Minute | Self::Hour | Self::Day | Self::Week => {
                Dimension::Duration
            }
            Self::Celsius | Self::Fahrenheit | Self::Kelvin => Dimension::Temperature,
        }
    }

    pub fn from_alias(alias: &str) -> Option<Self> {
        match alias {
            "C" => return Some(Self::Celsius),
            "F" => return Some(Self::Fahrenheit),
            "K" => return Some(Self::Kelvin),
            _ => {}
        }
        match alias.to_ascii_lowercase().as_str() {
            "m" | "meter" | "meters" | "metre" | "metres" => Some(Self::Meter),
            "km" | "kilometer" | "kilometers" | "kilometre" | "kilometres" => Some(Self::Kilometer),
            "cm" | "centimeter" | "centimeters" | "centimetre" | "centimetres" => {
                Some(Self::Centimeter)
            }
            "mm" | "millimeter" | "millimeters" | "millimetre" | "millimetres" => {
                Some(Self::Millimeter)
            }
            "in" | "inch" | "inches" => Some(Self::Inch),
            "ft" | "foot" | "feet" => Some(Self::Foot),
            "yd" | "yard" | "yards" => Some(Self::Yard),
            "mi" | "mile" | "miles" => Some(Self::Mile),
            "g" | "gram" | "grams" => Some(Self::Gram),
            "kg" | "kilogram" | "kilograms" => Some(Self::Kilogram),
            "oz" | "ounce" | "ounces" => Some(Self::Ounce),
            "lb" | "lbs" | "pound" | "pounds" => Some(Self::Pound),
            "s" | "sec" | "second" | "seconds" => Some(Self::Second),
            "min" | "minute" | "minutes" => Some(Self::Minute),
            "h" | "hr" | "hour" | "hours" => Some(Self::Hour),
            "day" | "days" => Some(Self::Day),
            "wk" | "week" | "weeks" => Some(Self::Week),
            "celsius" => Some(Self::Celsius),
            "fahrenheit" => Some(Self::Fahrenheit),
            "kelvin" => Some(Self::Kelvin),
            _ => None,
        }
    }

    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Meter => "m",
            Self::Kilometer => "km",
            Self::Centimeter => "cm",
            Self::Millimeter => "mm",
            Self::Inch => "in",
            Self::Foot => "ft",
            Self::Yard => "yd",
            Self::Mile => "mi",
            Self::Gram => "g",
            Self::Kilogram => "kg",
            Self::Ounce => "oz",
            Self::Pound => "lb",
            Self::Second => "s",
            Self::Minute => "min",
            Self::Hour => "h",
            Self::Day => "day",
            Self::Week => "week",
            Self::Celsius => "°C",
            Self::Fahrenheit => "°F",
            Self::Kelvin => "K",
        }
    }

    pub(crate) fn linear_factor_to(self, target: Self) -> Result<Number, UnitError> {
        if self.dimension() != target.dimension() {
            return Err(UnitError::IncompatibleDimensions);
        }
        if self.dimension() == Dimension::Temperature {
            return Err(UnitError::AffineArithmetic);
        }
        let (source_scale, _) = self.scale_and_offset();
        let (target_scale, _) = target.scale_and_offset();
        source_scale
            .divide(target_scale)
            .map_err(UnitError::Numeric)
    }

    fn scale_and_offset(self) -> (Number, Number) {
        if self == Self::Fahrenheit {
            return (ratio("5", "9"), ratio("-160", "9"));
        }
        let (scale, offset) = match self {
            Self::Meter | Self::Gram | Self::Second | Self::Celsius => ("1", "0"),
            Self::Kilometer => ("1000", "0"),
            Self::Centimeter => ("0.01", "0"),
            Self::Millimeter => ("0.001", "0"),
            Self::Inch => ("0.0254", "0"),
            Self::Foot => ("0.3048", "0"),
            Self::Yard => ("0.9144", "0"),
            Self::Mile => ("1609.344", "0"),
            Self::Kilogram => ("1000", "0"),
            Self::Ounce => ("28.349523125", "0"),
            Self::Pound => ("453.59237", "0"),
            Self::Minute => ("60", "0"),
            Self::Hour => ("3600", "0"),
            Self::Day => ("86400", "0"),
            Self::Week => ("604800", "0"),
            Self::Kelvin => ("1", "-273.15"),
            Self::Fahrenheit => unreachable!("handled above"),
        };
        (exact(scale), exact(offset))
    }
}

impl fmt::Display for Unit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.symbol())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnitValue {
    magnitude: Number,
    unit: Unit,
}

impl UnitValue {
    pub const fn new(magnitude: Number, unit: Unit) -> Self {
        Self { magnitude, unit }
    }

    pub const fn magnitude(&self) -> &Number {
        &self.magnitude
    }

    pub const fn unit(&self) -> Unit {
        self.unit
    }

    pub(crate) fn display_unit(&self) -> &'static str {
        let plural = !matches!(self.magnitude.exact_fraction().as_deref(), Some("1" | "-1"));
        match (self.unit, plural) {
            (Unit::Day, true) => "days",
            (Unit::Week, true) => "weeks",
            _ => self.unit.symbol(),
        }
    }

    pub(crate) fn converted_to(&self, target: Unit) -> Result<Self, UnitError> {
        if self.unit.dimension() != target.dimension() {
            return Err(UnitError::IncompatibleDimensions);
        }
        let (source_scale, source_offset) = self.unit.scale_and_offset();
        let base = self
            .magnitude
            .clone()
            .multiply(source_scale)
            .add(source_offset);
        let (target_scale, target_offset) = target.scale_and_offset();
        let magnitude = base
            .subtract(target_offset)
            .divide(target_scale)
            .map_err(UnitError::Numeric)?;
        Ok(Self::new(magnitude, target))
    }

    pub(crate) fn negated(self) -> Self {
        Self::new(self.magnitude.negated(), self.unit)
    }

    pub(crate) fn add(self, right: Self) -> Result<Self, UnitError> {
        self.ensure_arithmetic_dimension(&right)?;
        let right = right.converted_to(self.unit)?;
        Ok(Self::new(self.magnitude.add(right.magnitude), self.unit))
    }

    pub(crate) fn subtract(self, right: Self) -> Result<Self, UnitError> {
        self.ensure_arithmetic_dimension(&right)?;
        let right = right.converted_to(self.unit)?;
        Ok(Self::new(
            self.magnitude.subtract(right.magnitude),
            self.unit,
        ))
    }

    pub(crate) fn multiply_number(self, number: Number) -> Self {
        Self::new(self.magnitude.multiply(number), self.unit)
    }

    pub(crate) fn divide_number(self, number: Number) -> Result<Self, UnitError> {
        self.magnitude
            .divide(number)
            .map(|magnitude| Self::new(magnitude, self.unit))
            .map_err(UnitError::Numeric)
    }

    pub(crate) fn ratio(self, right: Self) -> Result<Number, UnitError> {
        let right = right.converted_to(self.unit)?;
        self.magnitude
            .divide(right.magnitude)
            .map_err(UnitError::Numeric)
    }

    pub(crate) fn compare(&self, right: &Self) -> Result<Option<Ordering>, UnitError> {
        let right = right.converted_to(self.unit)?;
        Ok(self.magnitude.compare(&right.magnitude))
    }

    fn ensure_arithmetic_dimension(&self, right: &Self) -> Result<(), UnitError> {
        if self.unit.dimension() != right.unit.dimension() {
            Err(UnitError::IncompatibleDimensions)
        } else if self.unit.dimension() == Dimension::Temperature {
            Err(UnitError::AffineArithmetic)
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum UnitError {
    IncompatibleDimensions,
    AffineArithmetic,
    Numeric(NumericError),
}

fn exact(source: &str) -> Number {
    Number::parse_decimal(source).expect("unit conversion constants are valid")
}

fn ratio(numerator: &str, denominator: &str) -> Number {
    exact(numerator)
        .divide(exact(denominator))
        .expect("unit conversion denominator is nonzero")
}
