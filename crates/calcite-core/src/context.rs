#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NumberFormat {
    decimal_separator: char,
    grouping_separator: Option<char>,
}

impl NumberFormat {
    pub const fn new(decimal_separator: char, grouping_separator: Option<char>) -> Self {
        assert!(
            !matches!(grouping_separator, Some(grouping) if grouping == decimal_separator),
            "decimal and grouping separators must differ"
        );
        Self {
            decimal_separator,
            grouping_separator,
        }
    }

    pub const fn decimal_separator(self) -> char {
        self.decimal_separator
    }

    pub const fn grouping_separator(self) -> Option<char> {
        self.grouping_separator
    }

    pub const fn decimal_point() -> Self {
        Self::new('.', Some(','))
    }

    pub const fn decimal_comma() -> Self {
        Self::new(',', Some('.'))
    }

    pub(crate) fn normalize(self, source: &str) -> Option<String> {
        let exponent_start = source.find(['e', 'E']);
        let (coefficient, exponent) = exponent_start
            .map(|at| (&source[..at], &source[at..]))
            .unwrap_or((source, ""));

        let mut coefficient_parts = coefficient.split(self.decimal_separator);
        let integer = coefficient_parts.next()?;
        let fraction = coefficient_parts.next();
        if coefficient_parts.next().is_some() {
            return None;
        }

        let integer = match self.grouping_separator {
            Some(grouping) if integer.contains(grouping) => {
                let groups: Vec<_> = integer.split(grouping).collect();
                if groups.is_empty()
                    || !(1..=3).contains(&digit_count(groups[0]))
                    || groups[1..].iter().any(|group| digit_count(group) != 3)
                {
                    return None;
                }
                groups.concat()
            }
            _ => integer.to_owned(),
        };

        if self
            .grouping_separator
            .is_some_and(|grouping| fraction.is_some_and(|part| part.contains(grouping)))
        {
            return None;
        }

        let mut normalized = integer;
        if let Some(fraction) = fraction {
            normalized.push('.');
            normalized.push_str(fraction);
        }
        normalized.push_str(exponent);
        Some(normalized)
    }
}

impl Default for NumberFormat {
    fn default() -> Self {
        Self::decimal_point()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumberOutputMode {
    Auto,
    Exact,
    Fraction,
    Mixed,
    Decimal { places: u8 },
    Significant { digits: u8 },
    Scientific { digits: u8 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutputFormat {
    pub number_mode: NumberOutputMode,
    pub mark_approximate: bool,
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self {
            number_mode: NumberOutputMode::Auto,
            mark_approximate: false,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CalculationContext {
    pub number_format: NumberFormat,
    pub output_format: OutputFormat,
    pub temporal: crate::TemporalContext,
    pub monetary: crate::MonetaryContext,
    pub custom_units: crate::CustomUnitRegistry,
}

fn digit_count(value: &str) -> usize {
    value.bytes().filter(u8::is_ascii_digit).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_valid_grouping_without_changing_the_decimal_value() {
        assert_eq!(
            NumberFormat::decimal_point().normalize("1,234,567.89"),
            Some("1234567.89".to_owned())
        );
        assert_eq!(
            NumberFormat::decimal_comma().normalize("1.234.567,89"),
            Some("1234567.89".to_owned())
        );
    }

    #[test]
    fn rejects_malformed_grouping() {
        assert_eq!(NumberFormat::decimal_point().normalize("12,34"), None);
        assert_eq!(NumberFormat::decimal_point().normalize("1.23,4"), None);
    }
}
