use crate::{CalculationContext, Value};

pub fn format(value: &Value) -> String {
    format_with_context(value, &CalculationContext::default())
}

pub fn format_with_context(value: &Value, context: &CalculationContext) -> String {
    let decimal_separator = context.number_format.decimal_separator();
    match value {
        Value::Number(number) => number.formatted_with(decimal_separator),
        Value::Percentage(percentage) => percentage.formatted_with(decimal_separator),
        Value::Fraction(fraction) => fraction.to_string(),
        Value::Unit(value) => format!(
            "{} {}",
            value.magnitude().formatted_with(decimal_separator),
            value.unit()
        ),
        Value::Boolean(value) => value.to_string(),
    }
}
