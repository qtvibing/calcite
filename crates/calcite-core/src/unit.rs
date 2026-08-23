use std::{cmp::Ordering, collections::BTreeMap, fmt};

use crate::{Number, NumericError};

const MAX_COMPOUND_EXPONENT: u32 = 10_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Dimension {
    Length,
    Mass,
    Duration,
    Temperature,
    Angle,
    Area,
    Volume,
    Frequency,
    Force,
    Energy,
    Power,
    Pressure,
    ElectricCurrent,
    ElectricPotential,
    Resistance,
    ElectricCharge,
    Information,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    Nanosecond,
    Microsecond,
    Millisecond,
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
    Hertz,
    Kilohertz,
    Megahertz,
    Gigahertz,
    Newton,
    Kilonewton,
    Joule,
    Kilojoule,
    WattHour,
    KilowattHour,
    Watt,
    Kilowatt,
    Megawatt,
    Pascal,
    Kilopascal,
    Megapascal,
    Bar,
    Psi,
    Ampere,
    Milliampere,
    Volt,
    Kilovolt,
    Ohm,
    Coulomb,
    Bit,
    Byte,
    Kilobit,
    Megabit,
    Gigabit,
    Kilobyte,
    Megabyte,
    Gigabyte,
    Terabyte,
    Kibibyte,
    Mebibyte,
    Gibibyte,
    Tebibyte,
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
            Self::Nanosecond
            | Self::Microsecond
            | Self::Millisecond
            | Self::Second
            | Self::Minute
            | Self::Hour
            | Self::Day
            | Self::Week => Dimension::Duration,
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
            Self::Hertz | Self::Kilohertz | Self::Megahertz | Self::Gigahertz => {
                Dimension::Frequency
            }
            Self::Newton | Self::Kilonewton => Dimension::Force,
            Self::Joule | Self::Kilojoule | Self::WattHour | Self::KilowattHour => {
                Dimension::Energy
            }
            Self::Watt | Self::Kilowatt | Self::Megawatt => Dimension::Power,
            Self::Pascal | Self::Kilopascal | Self::Megapascal | Self::Bar | Self::Psi => {
                Dimension::Pressure
            }
            Self::Ampere | Self::Milliampere => Dimension::ElectricCurrent,
            Self::Volt | Self::Kilovolt => Dimension::ElectricPotential,
            Self::Ohm => Dimension::Resistance,
            Self::Coulomb => Dimension::ElectricCharge,
            Self::Bit
            | Self::Byte
            | Self::Kilobit
            | Self::Megabit
            | Self::Gigabit
            | Self::Kilobyte
            | Self::Megabyte
            | Self::Gigabyte
            | Self::Terabyte
            | Self::Kibibyte
            | Self::Mebibyte
            | Self::Gibibyte
            | Self::Tebibyte => Dimension::Information,
        }
    }

    pub fn from_alias(alias: &str) -> Option<Self> {
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
            "ns" | "nanosecond" | "nanoseconds" => Some(Self::Nanosecond),
            "us" | "μs" | "µs" | "microsecond" | "microseconds" => Some(Self::Microsecond),
            "ms" | "millisecond" | "milliseconds" => Some(Self::Millisecond),
            "s" | "sec" | "second" | "seconds" => Some(Self::Second),
            "min" | "minute" | "minutes" => Some(Self::Minute),
            "h" | "hr" | "hour" | "hours" => Some(Self::Hour),
            "day" | "days" => Some(Self::Day),
            "wk" | "week" | "weeks" => Some(Self::Week),
            "c" | "celsius" => Some(Self::Celsius),
            "f" | "fahrenheit" => Some(Self::Fahrenheit),
            "k" | "kelvin" => Some(Self::Kelvin),
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
            "hz" | "hertz" => Some(Self::Hertz),
            "khz" | "kilohertz" => Some(Self::Kilohertz),
            "mhz" | "megahertz" => Some(Self::Megahertz),
            "ghz" | "gigahertz" => Some(Self::Gigahertz),
            "newton" | "newtons" => Some(Self::Newton),
            "kn" | "kilonewton" | "kilonewtons" => Some(Self::Kilonewton),
            "joule" | "joules" => Some(Self::Joule),
            "kj" | "kilojoule" | "kilojoules" => Some(Self::Kilojoule),
            "wh" | "watthour" | "watthours" => Some(Self::WattHour),
            "kwh" | "kilowatthour" | "kilowatthours" => Some(Self::KilowattHour),
            "watt" | "watts" => Some(Self::Watt),
            "kw" | "kilowatt" | "kilowatts" => Some(Self::Kilowatt),
            "mw" | "megawatt" | "megawatts" => Some(Self::Megawatt),
            "pa" | "pascal" | "pascals" => Some(Self::Pascal),
            "kpa" | "kilopascal" | "kilopascals" => Some(Self::Kilopascal),
            "mpa" | "megapascal" | "megapascals" => Some(Self::Megapascal),
            "bar" | "bars" => Some(Self::Bar),
            "psi" => Some(Self::Psi),
            "amp" | "amps" | "ampere" | "amperes" => Some(Self::Ampere),
            "ma" | "milliamp" | "milliamps" | "milliampere" | "milliamperes" => {
                Some(Self::Milliampere)
            }
            "volt" | "volts" => Some(Self::Volt),
            "kv" | "kilovolt" | "kilovolts" => Some(Self::Kilovolt),
            "ohm" | "ohms" | "ω" => Some(Self::Ohm),
            "coulomb" | "coulombs" => Some(Self::Coulomb),
            "bit" | "bits" => Some(Self::Bit),
            "byte" | "bytes" => Some(Self::Byte),
            "kbit" | "kilobit" | "kilobits" => Some(Self::Kilobit),
            "mbit" | "megabit" | "megabits" => Some(Self::Megabit),
            "gbit" | "gigabit" | "gigabits" => Some(Self::Gigabit),
            "kbyte" | "kilobyte" | "kilobytes" => Some(Self::Kilobyte),
            "mbyte" | "megabyte" | "megabytes" => Some(Self::Megabyte),
            "gbyte" | "gigabyte" | "gigabytes" => Some(Self::Gigabyte),
            "tbyte" | "terabyte" | "terabytes" => Some(Self::Terabyte),
            "kib" | "kibibyte" | "kibibytes" => Some(Self::Kibibyte),
            "mib" | "mebibyte" | "mebibytes" => Some(Self::Mebibyte),
            "gib" | "gibibyte" | "gibibytes" => Some(Self::Gibibyte),
            "tib" | "tebibyte" | "tebibytes" => Some(Self::Tebibyte),
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
            Self::Nanosecond => "ns",
            Self::Microsecond => "μs",
            Self::Millisecond => "ms",
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
            Self::Hertz => "Hz",
            Self::Kilohertz => "kHz",
            Self::Megahertz => "MHz",
            Self::Gigahertz => "GHz",
            Self::Newton => "newton",
            Self::Kilonewton => "kN",
            Self::Joule => "joule",
            Self::Kilojoule => "kJ",
            Self::WattHour => "Wh",
            Self::KilowattHour => "kWh",
            Self::Watt => "watt",
            Self::Kilowatt => "kW",
            Self::Megawatt => "MW",
            Self::Pascal => "Pa",
            Self::Kilopascal => "kPa",
            Self::Megapascal => "MPa",
            Self::Bar => "bar",
            Self::Psi => "psi",
            Self::Ampere => "amp",
            Self::Milliampere => "mA",
            Self::Volt => "volt",
            Self::Kilovolt => "kV",
            Self::Ohm => "ohm",
            Self::Coulomb => "coulomb",
            Self::Bit => "bit",
            Self::Byte => "byte",
            Self::Kilobit => "kbit",
            Self::Megabit => "Mbit",
            Self::Gigabit => "Gbit",
            Self::Kilobyte => "kbyte",
            Self::Megabyte => "Mbyte",
            Self::Gigabyte => "Gbyte",
            Self::Terabyte => "Tbyte",
            Self::Kibibyte => "KiB",
            Self::Mebibyte => "MiB",
            Self::Gibibyte => "GiB",
            Self::Tebibyte => "TiB",
        }
    }

    fn dimensions(self) -> Dimensions {
        match self.dimension() {
            Dimension::Length => Dimensions::new(1, 0, 0, 0, 0, 0, 0),
            Dimension::Mass => Dimensions::new(0, 1, 0, 0, 0, 0, 0),
            Dimension::Duration => Dimensions::new(0, 0, 1, 0, 0, 0, 0),
            Dimension::Temperature => Dimensions::new(0, 0, 0, 1, 0, 0, 0),
            Dimension::Angle => Dimensions::new(0, 0, 0, 0, 1, 0, 0),
            Dimension::Area => Dimensions::new(2, 0, 0, 0, 0, 0, 0),
            Dimension::Volume => Dimensions::new(3, 0, 0, 0, 0, 0, 0),
            Dimension::Frequency => Dimensions::new(0, 0, -1, 0, 0, 0, 0),
            Dimension::Force => Dimensions::new(1, 1, -2, 0, 0, 0, 0),
            Dimension::Energy => Dimensions::new(2, 1, -2, 0, 0, 0, 0),
            Dimension::Power => Dimensions::new(2, 1, -3, 0, 0, 0, 0),
            Dimension::Pressure => Dimensions::new(-1, 1, -2, 0, 0, 0, 0),
            Dimension::ElectricCurrent => Dimensions::new(0, 0, 0, 0, 0, 1, 0),
            Dimension::ElectricPotential => Dimensions::new(2, 1, -3, 0, 0, -1, 0),
            Dimension::Resistance => Dimensions::new(2, 1, -3, 0, 0, -2, 0),
            Dimension::ElectricCharge => Dimensions::new(0, 0, 1, 0, 0, 1, 0),
            Dimension::Information => Dimensions::new(0, 0, 0, 0, 0, 0, 1),
        }
    }

    pub(crate) fn scale_and_offset(self) -> (Number, Number) {
        if self == Self::Fahrenheit {
            return (ratio("5", "9"), ratio("45967", "180"));
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
            | Self::Kelvin
            | Self::Radian
            | Self::SquareMeter
            | Self::Hertz
            | Self::Ampere
            | Self::Coulomb
            | Self::Bit => ("1", "0"),
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
            Self::Nanosecond => ("0.000000001", "0"),
            Self::Microsecond => ("0.000001", "0"),
            Self::Millisecond => ("0.001", "0"),
            Self::Minute => ("60", "0"),
            Self::Hour => ("3600", "0"),
            Self::Day => ("86400", "0"),
            Self::Week => ("604800", "0"),
            Self::Celsius => ("1", "273.15"),
            Self::SquareKilometer => ("1000000", "0"),
            Self::SquareCentimeter => ("0.0001", "0"),
            Self::SquareMillimeter => ("0.000001", "0"),
            Self::SquareInch => ("0.00064516", "0"),
            Self::SquareFoot => ("0.09290304", "0"),
            Self::SquareYard => ("0.83612736", "0"),
            Self::SquareMile => ("2589988.110336", "0"),
            Self::Hectare => ("10000", "0"),
            Self::Acre => ("4046.8564224", "0"),
            Self::Liter => ("0.001", "0"),
            Self::Milliliter | Self::CubicCentimeter => ("0.000001", "0"),
            Self::CubicMeter => ("1", "0"),
            Self::UsGallon => ("0.003785411784", "0"),
            Self::UsQuart => ("0.000946352946", "0"),
            Self::UsPint => ("0.000473176473", "0"),
            Self::UsCup => ("0.0002365882365", "0"),
            Self::UsFluidOunce => ("0.0000295735295625", "0"),
            Self::UsTablespoon => ("0.00001478676478125", "0"),
            Self::UsTeaspoon => ("0.00000492892159375", "0"),
            Self::MetricCup => ("0.00025", "0"),
            Self::MetricTablespoon => ("0.000015", "0"),
            Self::MetricTeaspoon => ("0.000005", "0"),
            Self::Kilohertz => ("1000", "0"),
            Self::Megahertz => ("1000000", "0"),
            Self::Gigahertz => ("1000000000", "0"),
            Self::Newton | Self::Watt | Self::Pascal | Self::Volt | Self::Ohm => ("1000", "0"),
            Self::Kilonewton | Self::Kilojoule | Self::Kilowatt | Self::Kilopascal => {
                ("1000000", "0")
            }
            Self::Joule => ("1000", "0"),
            Self::WattHour => ("3600000", "0"),
            Self::KilowattHour => ("3600000000", "0"),
            Self::Megawatt | Self::Megapascal => ("1000000000", "0"),
            Self::Bar => ("100000000", "0"),
            Self::Psi => ("6894757.293168", "0"),
            Self::Milliampere => ("0.001", "0"),
            Self::Kilovolt => ("1000000", "0"),
            Self::Byte => ("8", "0"),
            Self::Kilobit => ("1000", "0"),
            Self::Megabit => ("1000000", "0"),
            Self::Gigabit => ("1000000000", "0"),
            Self::Kilobyte => ("8000", "0"),
            Self::Megabyte => ("8000000", "0"),
            Self::Gigabyte => ("8000000000", "0"),
            Self::Terabyte => ("8000000000000", "0"),
            Self::Kibibyte => ("8192", "0"),
            Self::Mebibyte => ("8388608", "0"),
            Self::Gibibyte => ("8589934592", "0"),
            Self::Tebibyte => ("8796093022208", "0"),
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Dimensions([i32; 7]);

impl Dimensions {
    const fn new(
        length: i32,
        mass: i32,
        duration: i32,
        temperature: i32,
        angle: i32,
        current: i32,
        information: i32,
    ) -> Self {
        Self([
            length,
            mass,
            duration,
            temperature,
            angle,
            current,
            information,
        ])
    }

    fn add_scaled(&mut self, other: Self, exponent: i32) {
        for (value, other) in self.0.iter_mut().zip(other.0) {
            *value += other * exponent;
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CompoundUnit {
    factors: BTreeMap<Unit, i32>,
}

impl CompoundUnit {
    pub fn from_unit(unit: Unit) -> Self {
        Self {
            factors: BTreeMap::from([(unit, 1)]),
        }
    }

    pub fn atomic_unit(&self) -> Option<Unit> {
        let mut factors = self.factors.iter();
        let (unit, exponent) = factors.next()?;
        (*exponent == 1 && factors.next().is_none()).then_some(*unit)
    }

    pub fn is_dimensionless(&self) -> bool {
        self.factors.is_empty()
    }

    pub fn has_denominator(&self) -> bool {
        self.factors.values().any(|exponent| *exponent < 0)
    }

    pub(crate) fn exponents_are_bounded(&self) -> bool {
        self.factors
            .values()
            .all(|exponent| exponent.unsigned_abs() <= MAX_COMPOUND_EXPONENT)
    }

    fn dimensions(&self) -> Dimensions {
        let mut dimensions = Dimensions::default();
        for (unit, exponent) in &self.factors {
            dimensions.add_scaled(unit.dimensions(), *exponent);
        }
        dimensions
    }

    fn contains_affine_unit(&self) -> bool {
        self.factors
            .keys()
            .any(|unit| matches!(unit, Unit::Celsius | Unit::Fahrenheit))
    }

    fn scale(&self) -> Result<Number, UnitError> {
        let mut scale = Number::from_i64(1);
        for (unit, exponent) in &self.factors {
            if matches!(unit, Unit::Celsius | Unit::Fahrenheit) && self.atomic_unit().is_none() {
                return Err(UnitError::AffineArithmetic);
            }
            let (factor, _) = unit.scale_and_offset();
            scale = scale.multiply(
                factor
                    .pow(Number::from_i64(i64::from(*exponent)))
                    .map_err(UnitError::Numeric)?,
            );
        }
        Ok(scale)
    }

    pub(crate) fn multiply(&self, right: &Self) -> Self {
        self.combine(right, 1)
    }

    pub(crate) fn divide(&self, right: &Self) -> Self {
        self.combine(right, -1)
    }

    pub(crate) fn pow(&self, exponent: i32) -> Self {
        let factors = self
            .factors
            .iter()
            .filter_map(|(unit, power)| {
                let power = power.saturating_mul(exponent);
                (power != 0).then_some((*unit, power))
            })
            .collect();
        Self { factors }
    }

    fn combine(&self, right: &Self, sign: i32) -> Self {
        let mut factors = self.factors.clone();
        for (unit, exponent) in &right.factors {
            let combined = factors
                .get(unit)
                .copied()
                .unwrap_or(0)
                .saturating_add(exponent.saturating_mul(sign));
            if combined == 0 {
                factors.remove(unit);
            } else {
                factors.insert(*unit, combined);
            }
        }
        Self { factors }
    }

    fn has_dimensional_cancellation(&self) -> bool {
        let mut net = Dimensions::default();
        let mut absolute_contributions = [0i32; 7];
        for (unit, exponent) in &self.factors {
            let dimensions = unit.dimensions();
            net.add_scaled(dimensions, *exponent);
            for (total, component) in absolute_contributions.iter_mut().zip(dimensions.0) {
                *total += component.saturating_mul(*exponent).saturating_abs();
            }
        }
        net.0
            .iter()
            .zip(absolute_contributions)
            .any(|(net, total)| net.saturating_abs() < total)
    }

    fn canonical(dimensions: Dimensions) -> Self {
        let units = [
            Unit::Meter,
            Unit::Gram,
            Unit::Second,
            Unit::Kelvin,
            Unit::Radian,
            Unit::Ampere,
            Unit::Bit,
        ];
        let factors = units
            .into_iter()
            .zip(dimensions.0)
            .filter_map(|(unit, exponent)| (exponent != 0).then_some((unit, exponent)))
            .collect();
        Self { factors }
    }

    pub(crate) fn formatted(&self) -> String {
        let numerator = self.format_side(1, "·");
        let denominator = self.format_side(-1, "/");
        match (numerator.is_empty(), denominator.is_empty()) {
            (false, true) => numerator,
            (true, false) => format!("1/{denominator}"),
            (false, false) => format!("{numerator}/{denominator}"),
            (true, true) => String::new(),
        }
    }

    fn format_side(&self, sign: i32, separator: &str) -> String {
        self.factors
            .iter()
            .filter(|(_, exponent)| exponent.signum() == sign)
            .map(|(unit, exponent)| format_factor(*unit, exponent.unsigned_abs()))
            .collect::<Vec<_>>()
            .join(separator)
    }
}

impl fmt::Display for CompoundUnit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.formatted())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnitValue {
    magnitude: Number,
    unit: CompoundUnit,
}

impl UnitValue {
    pub fn new(magnitude: Number, unit: Unit) -> Self {
        Self::from_compound(magnitude, CompoundUnit::from_unit(unit))
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

    pub fn atomic_unit(&self) -> Option<Unit> {
        self.unit.atomic_unit()
    }

    pub(crate) fn display_unit(&self) -> String {
        let plural = !matches!(self.magnitude.exact_fraction().as_deref(), Some("1" | "-1"));
        match (self.atomic_unit(), plural) {
            (Some(Unit::Day), true) => "days".into(),
            (Some(Unit::Week), true) => "weeks".into(),
            (Some(Unit::Acre), true) => "acres".into(),
            (Some(Unit::UsCup), true) => "US cups".into(),
            (Some(Unit::MetricCup), true) => "metric cups".into(),
            _ => self.unit.formatted(),
        }
    }

    pub(crate) fn converted_to(&self, target: Unit) -> Result<Self, UnitError> {
        self.converted_to_compound(&CompoundUnit::from_unit(target))
    }

    pub(crate) fn converted_to_compound(&self, target: &CompoundUnit) -> Result<Self, UnitError> {
        if self.unit.dimensions() != target.dimensions() {
            return Err(UnitError::IncompatibleDimensions);
        }
        let magnitude = match (self.atomic_unit(), target.atomic_unit()) {
            (Some(source), Some(target))
                if source.dimension() == Dimension::Temperature
                    && target.dimension() == Dimension::Temperature =>
            {
                let (source_scale, source_offset) = source.scale_and_offset();
                let base = self
                    .magnitude
                    .clone()
                    .multiply(source_scale)
                    .add(source_offset);
                let (target_scale, target_offset) = target.scale_and_offset();
                base.subtract(target_offset)
                    .divide(target_scale)
                    .map_err(UnitError::Numeric)?
            }
            _ => self
                .magnitude
                .clone()
                .multiply(self.unit.scale()?)
                .divide(target.scale()?)
                .map_err(UnitError::Numeric)?,
        };
        Ok(Self::from_compound(magnitude, target.clone()))
    }

    pub(crate) fn negated(self) -> Self {
        Self::from_compound(self.magnitude.negated(), self.unit)
    }

    pub(crate) fn add(self, right: Self) -> Result<Self, UnitError> {
        self.ensure_arithmetic_dimension(&right)?;
        let right = right.converted_to_compound(&self.unit)?;
        Ok(Self::from_compound(
            self.magnitude.add(right.magnitude),
            self.unit,
        ))
    }

    pub(crate) fn subtract(self, right: Self) -> Result<Self, UnitError> {
        self.ensure_arithmetic_dimension(&right)?;
        let right = right.converted_to_compound(&self.unit)?;
        Ok(Self::from_compound(
            self.magnitude.subtract(right.magnitude),
            self.unit,
        ))
    }

    pub(crate) fn multiply_number(self, number: Number) -> Self {
        Self::from_compound(self.magnitude.multiply(number), self.unit)
    }

    pub(crate) fn divide_number(self, number: Number) -> Result<Self, UnitError> {
        self.magnitude
            .divide(number)
            .map(|magnitude| Self::from_compound(magnitude, self.unit))
            .map_err(UnitError::Numeric)
    }

    pub(crate) fn multiply_unit(self, right: Self) -> Result<Self, UnitError> {
        if self.unit.contains_affine_unit() || right.unit.contains_affine_unit() {
            return Err(UnitError::AffineArithmetic);
        }
        let magnitude = self.magnitude.multiply(right.magnitude);
        let unit = self.unit.multiply(&right.unit);
        ensure_bounded_exponents(&unit)?;
        normalize_compound_value(magnitude, unit)
    }

    pub(crate) fn divide_unit(self, right: Self) -> Result<Self, UnitError> {
        if self.unit.contains_affine_unit() || right.unit.contains_affine_unit() {
            return Err(UnitError::AffineArithmetic);
        }
        let magnitude = self
            .magnitude
            .divide(right.magnitude)
            .map_err(UnitError::Numeric)?;
        let unit = self.unit.divide(&right.unit);
        ensure_bounded_exponents(&unit)?;
        normalize_compound_value(magnitude, unit)
    }

    pub(crate) fn pow(self, exponent: i32) -> Result<Self, UnitError> {
        if self.unit.contains_affine_unit() && exponent != 1 {
            return Err(UnitError::AffineArithmetic);
        }
        let magnitude = self
            .magnitude
            .pow(Number::from_i64(i64::from(exponent)))
            .map_err(UnitError::Numeric)?;
        let unit = self.unit.pow(exponent);
        ensure_bounded_exponents(&unit)?;
        normalize_compound_value(magnitude, unit)
    }

    pub(crate) fn compare(&self, right: &Self) -> Result<Option<Ordering>, UnitError> {
        let right = right.converted_to_compound(&self.unit)?;
        Ok(self.magnitude.compare(&right.magnitude))
    }

    fn ensure_arithmetic_dimension(&self, right: &Self) -> Result<(), UnitError> {
        if self.unit.dimensions() != right.unit.dimensions() {
            Err(UnitError::IncompatibleDimensions)
        } else if self.unit.contains_affine_unit() || right.unit.contains_affine_unit() {
            Err(UnitError::AffineArithmetic)
        } else {
            Ok(())
        }
    }
}

fn ensure_bounded_exponents(unit: &CompoundUnit) -> Result<(), UnitError> {
    if unit.exponents_are_bounded() {
        Ok(())
    } else {
        Err(UnitError::InvalidExponent)
    }
}

fn normalize_compound_value(
    mut magnitude: Number,
    mut unit: CompoundUnit,
) -> Result<UnitValue, UnitError> {
    if unit.is_dimensionless() || unit.has_dimensional_cancellation() {
        magnitude = magnitude.multiply(unit.scale()?);
        unit = CompoundUnit::canonical(unit.dimensions());
    }
    Ok(UnitValue::from_compound(magnitude, unit))
}

fn format_factor(unit: Unit, exponent: u32) -> String {
    match exponent {
        1 => unit.symbol().into(),
        2 => format!("{}²", unit.symbol()),
        3 => format!("{}³", unit.symbol()),
        exponent => format!("{}^{exponent}", unit.symbol()),
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CustomUnitRegistry {
    definitions: BTreeMap<String, CustomUnitDefinition>,
}

impl CustomUnitRegistry {
    pub fn define(
        &mut self,
        name: impl Into<String>,
        scale: Number,
        unit: CompoundUnit,
    ) -> Result<Option<CustomUnitDefinition>, CustomUnitError> {
        let name = name.into();
        if !valid_custom_unit_name(&name) || Unit::from_alias(&name).is_some() {
            return Err(CustomUnitError::InvalidName);
        }
        if scale.compare(&Number::from_i64(0)) != Some(Ordering::Greater)
            || unit.contains_affine_unit()
        {
            return Err(CustomUnitError::InvalidDefinition);
        }
        let definition = CustomUnitDefinition {
            name: name.clone(),
            scale,
            unit,
        };
        Ok(self.definitions.insert(name.to_lowercase(), definition))
    }

    pub fn get(&self, name: &str) -> Option<&CustomUnitDefinition> {
        self.definitions.get(&name.to_lowercase())
    }

    pub fn iter(&self) -> impl Iterator<Item = &CustomUnitDefinition> {
        self.definitions.values()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CustomUnitDefinition {
    name: String,
    scale: Number,
    unit: CompoundUnit,
}

impl CustomUnitDefinition {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn scale(&self) -> &Number {
        &self.scale
    }

    pub const fn unit(&self) -> &CompoundUnit {
        &self.unit
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CustomUnitValue {
    magnitude: Number,
    definition: CustomUnitDefinition,
}

impl CustomUnitValue {
    pub const fn new(magnitude: Number, definition: CustomUnitDefinition) -> Self {
        Self {
            magnitude,
            definition,
        }
    }

    pub const fn magnitude(&self) -> &Number {
        &self.magnitude
    }

    pub const fn definition(&self) -> &CustomUnitDefinition {
        &self.definition
    }

    pub(crate) fn into_unit(self) -> UnitValue {
        UnitValue::from_compound(
            self.magnitude.multiply(self.definition.scale),
            self.definition.unit,
        )
    }

    pub(crate) fn negated(self) -> Self {
        Self::new(self.magnitude.negated(), self.definition)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CustomUnitError {
    InvalidName,
    InvalidDefinition,
}

impl fmt::Display for CustomUnitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidName => "custom unit names must be unused identifiers",
            Self::InvalidDefinition => {
                "custom units require a positive scale and a non-affine compound unit"
            }
        })
    }
}

impl std::error::Error for CustomUnitError {}

fn valid_custom_unit_name(name: &str) -> bool {
    let mut characters = name.chars();
    characters
        .next()
        .is_some_and(|character| character.is_alphabetic() || character == '_')
        && characters.all(|character| character.is_alphanumeric() || character == '_')
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum UnitError {
    IncompatibleDimensions,
    AffineArithmetic,
    InvalidExponent,
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
