use std::{
    io::Write,
    process::{Command, Stdio},
};

fn calcite() -> Command {
    Command::new(env!("CARGO_BIN_EXE_calcite"))
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
