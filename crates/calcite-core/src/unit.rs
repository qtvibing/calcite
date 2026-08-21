use std::{cmp::Ordering, fmt};

use crate::{Number, NumericError};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Dimension {
    Length,
    Mass,
    Duration,
    Temperature,
    Angle,
    Area,
    Volume,
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
    Radian,
    Degree,
    SquareMeter,
    SquareKilometer,
    SquareCentimeter,
    SquareMillimeter,
    SquareInch,
    SquareFoot,
    SquareYard,
    SquareMile,
    Hectare,
    Acre,
    Liter,
    Milliliter,
    CubicMeter,
    CubicCentimeter,
    UsGallon,
    UsQuart,
    UsPint,
    UsCup,
    UsFluidOunce,
    UsTablespoon,
    UsTeaspoon,
    MetricCup,
    MetricTablespoon,
    MetricTeaspoon,
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
            Self::Radian | Self::Degree => Dimension::Angle,
            Self::SquareMeter
            | Self::SquareKilometer
            | Self::SquareCentimeter
            | Self::SquareMillimeter
            | Self::SquareInch
            | Self::SquareFoot
            | Self::SquareYard
            | Self::SquareMile
            | Self::Hectare
            | Self::Acre => Dimension::Area,
            Self::Liter
            | Self::Milliliter
            | Self::CubicMeter
            | Self::CubicCentimeter
            | Self::UsGallon
            | Self::UsQuart
            | Self::UsPint
            | Self::UsCup
            | Self::UsFluidOunce
            | Self::UsTablespoon
            | Self::UsTeaspoon
            | Self::MetricCup
            | Self::MetricTablespoon
            | Self::MetricTeaspoon => Dimension::Volume,
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
            "rad" | "radian" | "radians" => Some(Self::Radian),
            "deg" | "degree" | "degrees" => Some(Self::Degree),
            "m2" | "m²" | "sqm" | "squaremeter" | "squaremeters" | "squaremetre"
            | "squaremetres" => Some(Self::SquareMeter),
            "km2" | "km²" | "sqkm" | "squarekilometer" | "squarekilometers" | "squarekilometre"
            | "squarekilometres" => Some(Self::SquareKilometer),
            "cm2" | "cm²" | "sqcm" | "squarecentimeter" | "squarecentimeters"
            | "squarecentimetre" | "squarecentimetres" => Some(Self::SquareCentimeter),
            "mm2" | "mm²" | "sqmm" | "squaremillimeter" | "squaremillimeters"
            | "squaremillimetre" | "squaremillimetres" => Some(Self::SquareMillimeter),
            "in2" | "in²" | "sqin" | "squareinch" | "squareinches" => Some(Self::SquareInch),
            "ft2" | "ft²" | "sqft" | "squarefoot" | "squarefeet" => Some(Self::SquareFoot),
            "yd2" | "yd²" | "sqyd" | "squareyard" | "squareyards" => Some(Self::SquareYard),
            "mi2" | "mi²" | "sqmi" | "squaremile" | "squaremiles" => Some(Self::SquareMile),
            "ha" | "hectare" | "hectares" => Some(Self::Hectare),
            "acre" | "acres" => Some(Self::Acre),
            "l" | "liter" | "liters" | "litre" | "litres" => Some(Self::Liter),
            "ml" | "milliliter" | "milliliters" | "millilitre" | "millilitres" => {
                Some(Self::Milliliter)
            }
            "m3" | "m³" | "cubicmeter" | "cubicmeters" | "cubicmetre" | "cubicmetres" => {
                Some(Self::CubicMeter)
            }
            "cm3" | "cm³" | "cc" | "cubiccentimeter" | "cubiccentimeters" | "cubiccentimetre"
            | "cubiccentimetres" => Some(Self::CubicCentimeter),
            "usgal" | "usgallon" | "usgallons" => Some(Self::UsGallon),
            "usqt" | "usquart" | "usquarts" => Some(Self::UsQuart),
            "uspt" | "uspint" | "uspints" => Some(Self::UsPint),
            "uscup" | "uscups" => Some(Self::UsCup),
            "usfloz" | "usfluidounce" | "usfluidounces" => Some(Self::UsFluidOunce),
            "ustbsp" | "ustablespoon" | "ustablespoons" => Some(Self::UsTablespoon),
            "ustsp" | "usteaspoon" | "usteaspoons" => Some(Self::UsTeaspoon),
            "metriccup" | "metriccups" => Some(Self::MetricCup),
            "metrictbsp" | "metrictablespoon" | "metrictablespoons" => Some(Self::MetricTablespoon),
            "metrictsp" | "metricteaspoon" | "metricteaspoons" => Some(Self::MetricTeaspoon),
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
            Self::Radian => "rad",
            Self::Degree => "deg",
            Self::SquareMeter => "m²",
            Self::SquareKilometer => "km²",
            Self::SquareCentimeter => "cm²",
            Self::SquareMillimeter => "mm²",
            Self::SquareInch => "in²",
            Self::SquareFoot => "ft²",
            Self::SquareYard => "yd²",
            Self::SquareMile => "mi²",
            Self::Hectare => "ha",
            Self::Acre => "acre",
            Self::Liter => "L",
            Self::Milliliter => "mL",
            Self::CubicMeter => "m³",
            Self::CubicCentimeter => "cm³",
            Self::UsGallon => "US gal",
            Self::UsQuart => "US qt",
            Self::UsPint => "US pt",
            Self::UsCup => "US cup",
            Self::UsFluidOunce => "US fl oz",
            Self::UsTablespoon => "US tbsp",
            Self::UsTeaspoon => "US tsp",
            Self::MetricCup => "metric cup",
            Self::MetricTablespoon => "metric tbsp",
            Self::MetricTeaspoon => "metric tsp",
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
        if self == Self::Degree {
            return (
                Number::pi()
                    .divide(Number::from_i64(180))
                    .expect("one hundred eighty is nonzero"),
                exact("0"),
            );
        }
        let (scale, offset) = match self {
            Self::Meter
            | Self::Gram
            | Self::Second
            | Self::Celsius
            | Self::Radian
            | Self::SquareMeter
            | Self::Liter => ("1", "0"),
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
            Self::SquareKilometer => ("1000000", "0"),
            Self::SquareCentimeter => ("0.0001", "0"),
            Self::SquareMillimeter => ("0.000001", "0"),
            Self::SquareInch => ("0.00064516", "0"),
            Self::SquareFoot => ("0.09290304", "0"),
            Self::SquareYard => ("0.83612736", "0"),
            Self::SquareMile => ("2589988.110336", "0"),
            Self::Hectare => ("10000", "0"),
            Self::Acre => ("4046.8564224", "0"),
            Self::Milliliter | Self::CubicCentimeter => ("0.001", "0"),
            Self::CubicMeter => ("1000", "0"),
            Self::UsGallon => ("3.785411784", "0"),
            Self::UsQuart => ("0.946352946", "0"),
            Self::UsPint => ("0.473176473", "0"),
            Self::UsCup => ("0.2365882365", "0"),
            Self::UsFluidOunce => ("0.0295735295625", "0"),
            Self::UsTablespoon => ("0.01478676478125", "0"),
            Self::UsTeaspoon => ("0.00492892159375", "0"),
            Self::MetricCup => ("0.25", "0"),
            Self::MetricTablespoon => ("0.015", "0"),
            Self::MetricTeaspoon => ("0.005", "0"),
            Self::Fahrenheit | Self::Degree => unreachable!("handled above"),
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
            (Unit::Acre, true) => "acres",
            (Unit::UsCup, true) => "US cups",
            (Unit::MetricCup, true) => "metric cups",
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
