use calcite_core::{DiagnosticKind, calculate};

fn output(source: &str) -> String {
    calculate(source).unwrap().output
}

#[test]
fn converts_frequency_force_energy_power_and_pressure() {
    assert_eq!(output("1 kilohertz in hertz"), "1000 Hz");
    assert_eq!(output("1 kilonewton in newtons"), "1000 newton");
    assert_eq!(output("1 kilowatt hour in joules"), "3600000 joule");
    assert_eq!(output("2 megawatts in kilowatts"), "2000 kW");
    assert_eq!(output("1 bar in kilopascals"), "100 kPa");
    assert_eq!(output("60 hertz * 2 seconds"), "120");
    assert_eq!(output("1 newton in kg*m/s^2"), "1 m·kg/s²");
}

#[test]
fn converts_electrical_units_structurally() {
    assert_eq!(output("2 amperes * 3 seconds in coulombs"), "6 coulomb");
    assert_eq!(output("1 kilovolt in volts"), "1000 volt");
    assert_eq!(output("1 volt / 1 ampere in ohms"), "1 ohm");
    assert_eq!(output("1 watt / 1 ampere in volts"), "1 volt");
}

#[test]
fn data_units_use_unambiguous_case_insensitive_aliases() {
    assert_eq!(output("1 byte in bits"), "8 bit");
    assert_eq!(output("1 mebibyte in bytes"), "1048576 byte");
    assert_eq!(output("1 GIBIBYTE in MEBIBYTES"), "1024 MiB");
    assert_eq!(
        output("100 megabits / second in megabytes/minute"),
        "750 Mbyte/min"
    );
}

#[test]
fn rejects_cross_dimension_conversions() {
    assert_eq!(
        calculate("1 megabyte in megahertz").unwrap_err().kind(),
        DiagnosticKind::UnitMismatch
    );
}
