use crate::{CalculationContext, Value};

pub fn format(value: &Value) -> String {
    format_with_context(value, &CalculationContext::default())
}

pub fn format_with_context(value: &Value, context: &CalculationContext) -> String {
    let decimal_separator = context.number_format.decimal_separator();
    match value {
        Value::Number(number) => number.formatted_with(decimal_separator),
        Value::RadixInteger(value) => value.to_string(),
        Value::Percentage(percentage) => percentage.formatted_with(decimal_separator),
        Value::Fraction(fraction) => fraction.to_string(),
        Value::Unit(value) => format!(
            "{} {}",
            value.magnitude().formatted_with(decimal_separator),
            value.display_unit()
        ),
        Value::Money(money) => format!(
            "{} {}",
            money.amount().formatted_with(decimal_separator),
            money.currency()
        ),
        Value::Rate(rate) => match rate.numerator() {
            Some(numerator) => format!(
                "{} {numerator}/{}",
                rate.magnitude().formatted_with(decimal_separator),
                rate.denominator()
            ),
            None => format!(
                "{}/{}",
                rate.magnitude().formatted_with(decimal_separator),
                rate.denominator()
            ),
        },
        Value::Date(date) => date.to_string(),
        Value::Clock(time) => time.to_string(),
        Value::OffsetDateTime(value) => value.to_string(),
        Value::CalendarDuration(duration) => duration.to_string(),
        Value::WorkdayDuration(duration) => duration.to_string(),
        Value::Weekday(weekday) => weekday.to_string(),
        Value::Boolean(value) => value.to_string(),
    }
}
