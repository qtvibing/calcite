use std::cmp::Ordering;

use crate::{CalculationContext, ComplexNumber, Number, Value};

pub fn format(value: &Value) -> String {
    format_with_context(value, &CalculationContext::default())
}

pub fn format_with_context(value: &Value, context: &CalculationContext) -> String {
    let decimal_separator = context.number_format.decimal_separator();
    let render = |number: &crate::Number| {
        number.formatted_with_mode(
            context.output_format.number_mode,
            decimal_separator,
            context.output_format.mark_approximate,
        )
    };
    match value {
        Value::Number(number) => render(number),
        Value::RadixInteger(value) => value.to_string(),
        Value::RomanInteger(value) => value.to_string(),
        Value::Complex(value) => format_complex(value, &render),
        Value::Percentage(percentage) => {
            let value = percentage
                .ratio()
                .clone()
                .multiply(crate::Number::parse_decimal("100").expect("100 is valid"));
            format!("{}%", render(&value))
        }
        Value::Fraction(fraction)
            if matches!(
                context.output_format.number_mode,
                crate::NumberOutputMode::Auto
                    | crate::NumberOutputMode::Exact
                    | crate::NumberOutputMode::Fraction
            ) =>
        {
            fraction.to_string()
        }
        Value::Fraction(fraction) => render(fraction.number()),
        Value::Unit(value) => format!("{} {}", render(value.magnitude()), value.display_unit()),
        Value::Money(money) => format!("{} {}", render(money.amount()), money.currency()),
        Value::MoneyRate(rate) => rate.formatted_with(&render(rate.amount())),
        Value::Rate(rate) => rate.formatted_with(&render(rate.magnitude())),
        Value::Dice(dice) => dice.to_string(),
        Value::Date(date) => date.to_string(),
        Value::Clock(time) => time.to_string(),
        Value::OffsetClock(time) => time.to_string(),
        Value::ZonedClock(time) => time.to_string(),
        Value::OffsetDateTime(value) => value.to_string(),
        Value::ZonedDateTime(value) => value.to_string(),
        Value::TimeSpan(duration) => duration.to_string(),
        Value::CalendarDuration(duration) => duration.to_string(),
        Value::CalendarSpan(duration) => duration.to_string(),
        Value::WorkdayDuration(duration) => duration.to_string(),
        Value::Weekday(weekday) => weekday.to_string(),
        Value::Boolean(value) => value.to_string(),
    }
}

fn format_complex(value: &ComplexNumber, render: &impl Fn(&Number) -> String) -> String {
    if value.is_real() {
        return render(value.real());
    }
    let imaginary = value.imaginary_magnitude();
    let rendered_imaginary = render(&imaginary);
    let imaginary = if rendered_imaginary == "1" {
        "i".to_owned()
    } else if rendered_imaginary == "≈1" {
        "≈i".to_owned()
    } else {
        format!("{rendered_imaginary}i")
    };
    if value.real().is_zero() {
        return if value.imaginary_sign() == Some(Ordering::Less) {
            format!("-{imaginary}")
        } else {
            imaginary
        };
    }
    let operator = if value.imaginary_sign() == Some(Ordering::Less) {
        '-'
    } else {
        '+'
    };
    format!("{} {operator} {imaginary}", render(value.real()))
}
