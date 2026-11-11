use serde_json::json;

use crate::{CalendarDurationUnit, Number, Value};

impl Value {
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            Self::Number(value) => number_json(value),
            Self::String(value) => json!({ "type": "string", "value": value }),
            Self::RadixInteger(value) => json!({
                "type": "radix_integer",
                "value": value.value().exact_fraction(),
                "radix": value.radix().value(),
            }),
            Self::RomanInteger(value) => json!({
                "type": "roman_integer",
                "value": value.value().exact_fraction(),
                "roman": value.to_string(),
            }),
            Self::Complex(value) => json!({
                "type": "complex",
                "real": number_json(value.real()),
                "imaginary": number_json(value.imaginary()),
            }),
            Self::Function(value) => json!({
                "type": "function",
                "parameters": value.parameters(),
            }),
            Self::Percentage(value) => json!({
                "type": "percentage",
                "ratio": canonical_number(value.ratio()),
            }),
            Self::Fraction(value) => json!({
                "type": "fraction",
                "value": canonical_number(value.number()),
            }),
            Self::Unit(value) => json!({
                "type": "unit",
                "magnitude": canonical_number(value.magnitude()),
                "unit": value.compound_unit().to_string(),
            }),
            Self::CustomUnit(value) => json!({
                "type": "custom_unit",
                "magnitude": canonical_number(value.magnitude()),
                "unit": value.definition().name(),
                "scale": canonical_number(value.definition().scale()),
                "defined_as": value.definition().unit().to_string(),
            }),
            Self::Money(value) => json!({
                "type": "money",
                "amount": canonical_number(value.amount()),
                "currency": value.currency().to_string(),
            }),
            Self::MoneyRate(value) => json!({
                "type": "money_rate",
                "amount": canonical_number(value.amount()),
                "currency": value.currency().to_string(),
                "denominator": value.denominator().to_string(),
            }),
            Self::Rate(value) => json!({
                "type": "rate",
                "magnitude": canonical_number(value.magnitude()),
                "unit": value.compound_unit().to_string(),
            }),
            Self::Dice(value) => json!({
                "type": "dice_distribution",
                "count": value.count(),
                "sides": value.sides(),
                "modifier": value.modifier(),
                "minimum": value.minimum(),
                "maximum": value.maximum(),
                "mean": canonical_number(&value.mean()),
                "outcomes": value
                    .outcome_probabilities()
                    .into_iter()
                    .map(|(total, probability)| json!({
                        "value": total,
                        "probability": canonical_number(probability.ratio()),
                    }))
                    .collect::<Vec<_>>(),
            }),
            Self::Coin(value) => textual_json("coin_side", value),
            Self::Date(value) => json!({
                "type": "date",
                "value": value.to_string(),
                "year": value.year(),
                "month": value.month(),
                "day": value.day(),
            }),
            Self::Clock(value) => json!({
                "type": "clock_time",
                "value": value.to_string(),
                "hour": value.hour(),
                "minute": value.minute(),
                "second": value.second(),
                "nanosecond": value.nanosecond(),
            }),
            Self::OffsetClock(value) => json!({
                "type": "offset_clock_time",
                "value": value.to_string(),
                "hour": value.time().hour(),
                "minute": value.time().minute(),
                "second": value.time().second(),
                "nanosecond": value.time().nanosecond(),
                "offset_seconds": value.offset().seconds(),
            }),
            Self::ZonedClock(value) => json!({
                "type": "zoned_clock_time",
                "value": value.to_string(),
                "hour": value.time().hour(),
                "minute": value.time().minute(),
                "second": value.time().second(),
                "nanosecond": value.time().nanosecond(),
                "zone": value.zone(),
            }),
            Self::OffsetDateTime(value) => json!({
                "type": "offset_date_time",
                "value": value.to_string(),
                "year": value.date().year(),
                "month": value.date().month(),
                "day": value.date().day(),
                "hour": value.time().hour(),
                "minute": value.time().minute(),
                "second": value.time().second(),
                "nanosecond": value.time().nanosecond(),
                "offset_seconds": value.offset().seconds(),
            }),
            Self::ZonedDateTime(value) => json!({
                "type": "zoned_date_time",
                "value": value.to_string(),
                "year": value.datetime().date().year(),
                "month": value.datetime().date().month(),
                "day": value.datetime().date().day(),
                "hour": value.datetime().time().hour(),
                "minute": value.datetime().time().minute(),
                "second": value.datetime().time().second(),
                "nanosecond": value.datetime().time().nanosecond(),
                "offset_seconds": value.datetime().offset().seconds(),
                "zone": value.zone(),
            }),
            Self::TimeSpan(value) => json!({
                "type": "time_span",
                "value": value.to_string(),
                "nanoseconds": value.nanoseconds().to_string(),
                "seconds": exact_seconds(value.nanoseconds()),
            }),
            Self::CalendarDuration(value) => json!({
                "type": "calendar_duration",
                "value": value.to_string(),
                "months": value.total_months(),
                "unit": if matches!(value.unit(), CalendarDurationUnit::Year) { "years" } else { "months" },
            }),
            Self::CalendarSpan(value) => json!({
                "type": "calendar_span",
                "value": value.to_string(),
                "months": value.total_months(),
                "days": value.remaining_days(),
            }),
            Self::WorkdayDuration(value) => json!({
                "type": "workday_duration",
                "value": value.to_string(),
                "days": value.days(),
            }),
            Self::Weekday(value) => json!({
                "type": "weekday",
                "value": value.to_string(),
                "index_from_monday": value.number_from_monday(),
            }),
            Self::Boolean(value) => json!({ "type": "boolean", "value": value }),
        }
    }
}

fn number_json(value: &Number) -> serde_json::Value {
    json!({
        "type": "number",
        "value": canonical_number(value),
        "exact": value.is_exact(),
    })
}

fn canonical_number(value: &Number) -> String {
    value.exact_fraction().unwrap_or_else(|| value.to_string())
}

fn exact_seconds(nanoseconds: i128) -> String {
    let sign = if nanoseconds < 0 { "-" } else { "" };
    let magnitude = nanoseconds.unsigned_abs();
    let seconds = magnitude / 1_000_000_000;
    let fraction = format!("{:09}", magnitude % 1_000_000_000);
    let fraction = fraction.trim_end_matches('0');
    if fraction.is_empty() {
        format!("{sign}{seconds}")
    } else {
        format!("{sign}{seconds}.{fraction}")
    }
}

fn textual_json(kind: &str, value: &impl ToString) -> serde_json::Value {
    json!({ "type": kind, "value": value.to_string() })
}
