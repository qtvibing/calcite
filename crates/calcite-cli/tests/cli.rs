use std::{
    io::Write,
    process::{Command, Stdio},
};

#[test]
fn resolves_zone_abbreviations_from_host_zone_data() {
    let output = calcite()
        .args(["2026-10-14 at 9pm CET to Tokyo time"])
        .output()
        .expect("calcite should run");
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        "2026-10-15T04:00+09:00[Asia/Tokyo]"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn literal_zone_codes_use_fixed_offsets() {
    let output = calcite()
        .args(["--literal-zone-codes", "2026-10-14 at 21:00 CET"])
        .output()
        .expect("calcite should run");
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        "2026-10-14T21:00+01:00"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn ambiguous_zone_codes_list_candidates() {
    let output = calcite()
        .args(["2026-10-14 at 21:00 CST"])
        .output()
        .expect("calcite should run");
    assert!(!output.status.success());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("could mean"),
        "the diagnostic lists candidate zones"
    );
}

fn calcite() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_calcite"));
    command.env("CALCITE_CONFIG", "");
    command.env(
        "CALCITE_RATE_CACHE",
        std::env::temp_dir().join(format!("calcite-test-{}-rates.json", std::process::id())),
    );
    command
}

#[test]
fn converts_currencies_from_the_local_rate_cache() {
    let path = std::env::temp_dir().join(format!(
        "calcite-test-{}-currency-rates.json",
        std::process::id()
    ));
    std::fs::write(
        &path,
        r#"{
            "as_of": "2025-03-08T00:00:00Z",
            "rates": [
                {"source": "USD", "target": "EUR", "rate": "0.8"},
                {"source": "USD", "target": "JPY", "rate": "150"}
            ]
        }"#,
    )
    .unwrap();
    let output = calcite()
        .env("CALCITE_RATE_CACHE", &path)
        .args(["80", "EUR", "in", "JPY"])
        .output()
        .expect("calcite should run");
    let _ = std::fs::remove_file(path);

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "15000 JPY\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn supplies_os_entropy_for_natural_dice_and_coin_actions() {
    let roll = calcite()
        .args(["roll", "me", "a", "3d12"])
        .output()
        .expect("calcite should run");
    assert!(roll.status.success());
    let total = String::from_utf8(roll.stdout)
        .unwrap()
        .trim()
        .parse::<i64>()
        .unwrap();
    assert!((3..=36).contains(&total));
    assert!(roll.stderr.is_empty());

    let flip = calcite()
        .args(["flip", "a", "coin"])
        .output()
        .expect("calcite should run");
    assert!(flip.status.success());
    let side = String::from_utf8(flip.stdout).unwrap();
    assert!(matches!(side.trim(), "heads" | "tails"));
    assert!(flip.stderr.is_empty());
}

#[test]
fn conversational_input_is_normalized_to_answers() {
    for (source, expected) in [
        ("what is 5% of 200", "10"),
        ("What's 2+2?", "4"),
        ("convert 5 miles to km", "8.04672 km"),
        ("how many km in 5 miles", "8.04672 km"),
    ] {
        let output = calcite()
            .args([source])
            .output()
            .expect("calcite should run");
        assert!(output.status.success(), "{source} failed");
        assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), expected);
        assert!(output.stderr.is_empty(), "{source}");
    }
}

#[test]
fn conversational_input_is_normalized_in_the_repl_too() {
    let mut child = calcite()
        .arg("--repl")
        .arg("--no-history")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("calcite should run");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"convert 5 miles to km\nwhat is 5% of 200\n:quit\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "8.04672 km\n10\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn evaluates_deterministic_finance_functions() {
    let output = calcite()
        .args(["round(mortgage_payment($10000,", "6%,", "6),", "2)"])
        .output()
        .expect("calcite should run");

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "165.73 USD\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn evaluates_and_converts_compound_units() {
    let output = calcite()
        .args(["10", "m/s^2", "in", "km/h^2"])
        .output()
        .expect("calcite should run");

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "129600 km/h²\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn evaluates_natural_aggregate_lists_with_final_and() {
    let output = calcite()
        .args(["sum", "of", "1,", "2,", "and", "3"])
        .output()
        .expect("calcite should run");

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "6\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn strips_typed_magnitudes_explicitly() {
    let output = calcite()
        .args(["40/day", "to", "decimal"])
        .output()
        .expect("calcite should run");

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "40\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn evaluates_natural_rule_of_three_expressions() {
    let output = calcite()
        .args(["6", "is", "to", "60", "as", "8", "is", "to", "what"])
        .output()
        .expect("calcite should run");

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "80\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn distinguishes_spaced_remainder_from_attached_percentage() {
    let remainder = calcite()
        .args(["17", "%", "5"])
        .output()
        .expect("calcite should run");
    assert!(remainder.status.success());
    assert_eq!(String::from_utf8(remainder.stdout).unwrap(), "2\n");

    let percentage = calcite()
        .args(["10%"])
        .output()
        .expect("calcite should run");
    assert!(percentage.status.success());
    assert_eq!(String::from_utf8(percentage.stdout).unwrap(), "10%\n");
}

#[test]
fn evaluates_exact_number_theory_functions() {
    let output = calcite()
        .args(["is_prime(18446744073709551557)"])
        .output()
        .expect("calcite should run");

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "true\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn evaluates_arbitrary_precision_bitwise_shifts() {
    let output = calcite()
        .args(["1", "<<", "100"])
        .output()
        .expect("calcite should run");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "1267650600228229401496703205376\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn evaluates_exact_recurring_decimals() {
    let output = calcite()
        .args(["0.(3)", "as", "fraction"])
        .output()
        .expect("calcite should run");

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "1/3\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn converts_arbitrary_precision_integer_bases() {
    let output = calcite()
        .args(["0xffffffffffffffffffffffffffffffff", "in", "decimal"])
        .output()
        .expect("calcite should run");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "340282366920938463463374607431768211455\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn evaluates_exact_combinatorics() {
    let output = calcite()
        .args(["3", "combinations", "of", "25"])
        .output()
        .expect("calcite should run");

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "2300\n");
    assert!(output.stderr.is_empty());
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
fn supports_help_version_and_explicit_eval_modes() {
    let help = calcite()
        .arg("--help")
        .output()
        .expect("calcite should run");
    assert!(help.status.success());
    assert!(
        String::from_utf8(help.stdout)
            .unwrap()
            .contains("Usage: calcite")
    );

    let version = calcite()
        .arg("--version")
        .output()
        .expect("calcite should run");
    assert!(version.status.success());
    assert_eq!(
        String::from_utf8(version.stdout).unwrap(),
        format!("calcite {}\n", env!("CARGO_PKG_VERSION"))
    );

    let evaluation = calcite()
        .args(["--eval", "2 + 3 * 4"])
        .output()
        .expect("calcite should run");
    assert!(evaluation.status.success());
    assert_eq!(String::from_utf8(evaluation.stdout).unwrap(), "14\n");
}

#[test]
fn evaluates_a_document_from_a_file() {
    let path = std::env::temp_dir().join(format!(
        "calcite-test-{}-document.calcite",
        std::process::id()
    ));
    std::fs::write(&path, "subtotal = 10\nsubtotal * 2\n").unwrap();
    let output = calcite()
        .arg("--file")
        .arg(&path)
        .output()
        .expect("calcite should run");
    let _ = std::fs::remove_file(path);

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "10\n20\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn supports_plain_and_json_output_modes() {
    let plain = calcite()
        .args(["--plain", "first", "=", "2", "\n", "first", "+", "3"])
        .output()
        .expect("calcite should run");
    assert!(plain.status.success());
    assert_eq!(String::from_utf8(plain.stdout).unwrap(), "5\n");

    let json = calcite()
        .args(["--json", "2", "+", "3"])
        .output()
        .expect("calcite should run");
    assert!(json.status.success());
    let value: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(value["ok"], true);
    assert_eq!(value["results"][0]["output"], "5");
    assert_eq!(value["results"][0]["value"]["type"], "number");
    assert_eq!(value["results"][0]["value"]["value"], "5");
    assert_eq!(value["results"][0]["value"]["exact"], true);

    let error = calcite()
        .args(["--json", "1", "/", "0"])
        .output()
        .expect("calcite should run");
    assert!(!error.status.success());
    let value: serde_json::Value = serde_json::from_slice(&error.stdout).unwrap();
    assert_eq!(value["ok"], false);
    assert_eq!(value["error"]["kind"], "DivisionByZero");
}

#[test]
fn loads_configuration_and_allows_command_line_overrides() {
    let path =
        std::env::temp_dir().join(format!("calcite-test-{}-config.json", std::process::id()));
    std::fs::write(
        &path,
        r#"{
            "locale": "de-DE",
            "format": "decimal",
            "precision": 3,
            "approximate": true,
            "default_currency": "CAD",
            "date_order": "dmy",
            "history": false,
            "custom_units": { "smoot": "1,7 meters" }
        }"#,
    )
    .unwrap();

    let configured = calcite()
        .env("CALCITE_CONFIG", &path)
        .args(["--eval", "1,0 / 3"])
        .output()
        .expect("calcite should run");
    assert!(configured.status.success());
    assert_eq!(String::from_utf8(configured.stdout).unwrap(), "≈0,333\n");

    let custom_unit = calcite()
        .env("CALCITE_CONFIG", &path)
        .args(["--eval", "2 smoot in meters"])
        .output()
        .expect("calcite should run");
    assert!(custom_unit.status.success());
    assert_eq!(String::from_utf8(custom_unit.stdout).unwrap(), "3,4 m\n");

    let overridden = calcite()
        .env("CALCITE_CONFIG", &path)
        .args(["--locale", "en-US", "--eval", "1.5 + 1"])
        .output()
        .expect("calcite should run");
    let _ = std::fs::remove_file(path);
    assert!(overridden.status.success());
    assert_eq!(String::from_utf8(overridden.stdout).unwrap(), "2.5\n");
}

#[test]
fn configures_numeric_output_modes() {
    let exact = calcite()
        .args(["--format", "exact", "1", "/", "3"])
        .output()
        .expect("calcite should run");
    assert!(exact.status.success());
    assert_eq!(String::from_utf8(exact.stdout).unwrap(), "1/3\n");

    let scientific = calcite()
        .args(["--format", "scientific", "--precision", "4", "12345.67"])
        .output()
        .expect("calcite should run");
    assert!(scientific.status.success());
    assert_eq!(String::from_utf8(scientific.stdout).unwrap(), "1.235e4\n");
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
fn repl_preserves_variables_answers_and_session_history() {
    let mut child = calcite()
        .arg("--repl")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("calcite should run");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"2 + 2\nans * 3\nx = 5\nX + ans\n:history\n:quit\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "4\n12\n5\n10\n1  2 + 2\n2  ans * 3\n3  x = 5\n4  X + ans\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn repl_replays_history_entries_without_ambient_persistence() {
    let mut child = calcite()
        .arg("--repl")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("calcite should run");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"2 + 2\n!!\n:quit\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "4\n4\n");
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
fn cli_supplies_the_current_system_time() {
    let output = calcite()
        .env("TZ", "UTC")
        .args(["now", "in", "UTC"])
        .output()
        .expect("calcite should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains('T'));
    assert!(stdout.ends_with("Z\n"));
    assert!(output.stderr.is_empty());
}

#[test]
fn configures_numeric_date_order_explicitly() {
    let mdy = calcite()
        .args(["--date-order", "mdy", "3/8/2025"])
        .output()
        .expect("calcite should run");
    assert!(mdy.status.success());
    assert_eq!(String::from_utf8(mdy.stdout).unwrap(), "2025-03-08\n");

    let dmy = calcite()
        .args(["--date-order", "DMY", "3.8.2025"])
        .output()
        .expect("calcite should run");
    assert!(dmy.status.success());
    assert_eq!(String::from_utf8(dmy.stdout).unwrap(), "2025-08-03\n");
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
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .starts_with("Usage: calcite")
    );
}
