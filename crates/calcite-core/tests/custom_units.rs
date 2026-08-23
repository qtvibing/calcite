use calcite_core::{
    CalculationContext, CompoundUnit, CustomUnitError, Number, Unit, calculate_with_context,
};

fn number(source: &str) -> Number {
    Number::parse_decimal(source).unwrap()
}

#[test]
fn caller_defined_units_are_explicit_and_deterministic() {
    let mut context = CalculationContext::default();
    context
        .custom_units
        .define("smoot", number("1.7"), CompoundUnit::from_unit(Unit::Meter))
        .unwrap();
    context
        .custom_units
        .define(
            "smoots",
            number("1.7"),
            CompoundUnit::from_unit(Unit::Meter),
        )
        .unwrap();

    assert_eq!(
        calculate_with_context("2 smoots", &context).unwrap().output,
        "2 smoots"
    );
    assert_eq!(
        calculate_with_context("2 smoots in meters", &context)
            .unwrap()
            .output,
        "3.4 m"
    );
    assert_eq!(
        calculate_with_context("3.4 meters in smoots", &context)
            .unwrap()
            .output,
        "2 smoots"
    );
    assert_eq!(
        calculate_with_context("2 smoots + 1.7 meters", &context)
            .unwrap()
            .output,
        "5.1 m"
    );
}

#[test]
fn custom_unit_definitions_reject_invalid_names_scales_and_affine_units() {
    let mut context = CalculationContext::default();
    assert_eq!(
        context
            .custom_units
            .define("meters", number("2"), CompoundUnit::from_unit(Unit::Meter)),
        Err(CustomUnitError::InvalidName)
    );
    assert_eq!(
        context
            .custom_units
            .define("widget", number("0"), CompoundUnit::from_unit(Unit::Meter)),
        Err(CustomUnitError::InvalidDefinition)
    );
    assert_eq!(
        context.custom_units.define(
            "warmth",
            number("2"),
            CompoundUnit::from_unit(Unit::Celsius)
        ),
        Err(CustomUnitError::InvalidDefinition)
    );
}
