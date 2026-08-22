use std::{
    io::Write,
    process::{Command, Stdio},
};

fn calcite() -> Command {
    Command::new(env!("CARGO_BIN_EXE_calcite"))
}

#[test]
fn evaluates_exact_integer_modulo() {
    let output = calcite()
        .args(["123456789012345678901234567890", "mod", "97"])
        .output()
        .expect("calcite should run");

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "52\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn evaluates_an_expression_from_arguments() {
    let output = calcite()
        .args(["10%", "of", "50", "km"])
        .output()
        .expect("calcite should run");

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "5 km\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn evaluates_an_expression_from_standard_input() {
    let mut child = calcite()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("calcite should run");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"90 km / 3 h * 2 h\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "60 km\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn omits_document_headings_and_comments_from_output() {
    let mut child = calcite()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("calcite should run");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"# Totals\n// setup\nBase: amount = 10\namount + 5 // answer\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "10\n15\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn evaluates_multiline_variables_from_standard_input() {
    let mut child = calcite()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("calcite should run");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"distance = 120 km\ntime = 2 h\ndistance / time\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "120 km\n2 h\n60 km/h\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn evaluates_deterministic_workday_arithmetic() {
    let output = calcite()
        .args(["2025-03-07", "+", "1", "workday"])
        .output()
        .expect("calcite should run");

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "2025-03-10\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn evaluates_natural_workday_ranges() {
    let output = calcite()
        .args(["workdays", "from", "2025-03-10", "to", "2025-03-17"])
        .output()
        .expect("calcite should run");

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "5 workdays\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn relative_weekdays_require_explicit_temporal_context() {
    let output = calcite()
        .args(["next", "Monday"])
        .output()
        .expect("calcite should run");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "error: current date or time was not provided\n  next Monday\n  ^^^^^^^^^^^\n"
    );
}

#[test]
fn evaluates_natural_calendar_arithmetic() {
    let output = calcite()
        .args(["March", "8,", "2025", "+", "1", "month"])
        .output()
        .expect("calcite should run");

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "2025-04-08\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn evaluates_constants_and_exponentials() {
    let output = calcite()
        .args(["round", "(ln(e),", "10)"])
        .output()
        .expect("calcite should run");

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "1\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn evaluates_angle_aware_trigonometry() {
    let output = calcite()
        .args(["asin", "(1)", "in", "deg"])
        .output()
        .expect("calcite should run");

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "90 deg\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn evaluates_roots_and_logarithms_from_the_command_line() {
    let root = calcite()
        .args(["cube", "root", "of", "-8"])
        .output()
        .expect("calcite should run");
    assert!(root.status.success());
    assert_eq!(String::from_utf8(root.stdout).unwrap(), "-2\n");

    let logarithm = calcite()
        .args(["log", "(81,", "3)"])
        .output()
        .expect("calcite should run");
    assert!(logarithm.status.success());
    assert_eq!(String::from_utf8(logarithm.stdout).unwrap(), "4\n");
}

#[test]
fn rounds_typed_values_from_the_command_line() {
    let output = calcite()
        .args(["round", "($12.345,", "2)"])
        .output()
        .expect("calcite should run");

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "12.35 USD\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn evaluates_remaining_aggregates() {
    let output = calcite()
        .args(["median", "of", "1", "km,", "500", "m"])
        .output()
        .expect("calcite should run");

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "0.75 km\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn evaluates_natural_typed_functions() {
    let output = calcite()
        .args(["average", "of", "1", "km,", "500", "m"])
        .output()
        .expect("calcite should run");

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "0.75 km\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn converts_area_and_volume_units() {
    let output = calcite()
        .args(["1", "US", "gallon", "in", "L"])
        .output()
        .expect("calcite should run");

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "3.785411784 L\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn evaluates_typed_money_without_hiding_currency() {
    let output = calcite()
        .args(["$100", "+", "10%"])
        .output()
        .expect("calcite should run");

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "110 USD\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn converts_offset_date_times() {
    let output = calcite()
        .args(["2025-03-08T14:30:00+02:00", "in", "UTC"])
        .output()
        .expect("calcite should run");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "2025-03-08T12:30Z\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn reports_spanned_diagnostics_on_standard_error() {
    let output = calcite()
        .args(["1", "km", "+", "1", "kg"])
        .output()
        .expect("calcite should run");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "error: units have incompatible dimensions\n  1 km + 1 kg\n       ^\n"
    );
}

#[test]
fn multiline_diagnostics_only_print_the_failing_source_line() {
    let mut child = calcite()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("calcite should run");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"subtotal = 10\ntotal = missing + subtotal\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "error: unknown variable: missing\n  total = missing + subtotal\n          ^^^^^^^\n"
    );
}

#[test]
fn empty_input_prints_usage_and_uses_a_distinct_exit_code() {
    let output = calcite()
        .stdin(Stdio::null())
        .output()
        .expect("calcite should run");

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "usage: calcite <expression>\n"
    );
}
