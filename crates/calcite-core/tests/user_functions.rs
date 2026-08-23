use calcite_core::{
    CalculationContext, CalculationSession, DiagnosticKind, calculate, calculate_document,
};

fn output(source: &str) -> String {
    calculate(source).unwrap().output
}

#[test]
fn anonymous_lambdas_are_typed_and_invokable() {
    assert_eq!(output("(x -> x * x)(5)"), "25");
    assert_eq!(output("((x, y) -> x + y)(2, 3)"), "5");
    assert_eq!(output("(x → x + 1)(2)"), "3");
    assert_eq!(output("(x -> x + 1)"), "<function(x)>");
}

#[test]
fn named_functions_work_in_documents_and_sessions() {
    let document =
        calculate_document("apply_rate(amount, rate) = amount * (1 + rate)\napply_rate(100, 20%)")
            .unwrap();
    assert_eq!(document.lines[0].output, "<function(amount, rate)>");
    assert_eq!(document.lines[1].output, "120");

    let mut session = CalculationSession::new();
    let context = CalculationContext::default();
    session.calculate("double(x) = x * 2", &context).unwrap();
    assert_eq!(
        session.calculate("double(21)", &context).unwrap().output,
        "42"
    );
}

#[test]
fn unknown_names_suggest_nearby_session_variables() {
    let mut session = CalculationSession::new();
    let context = CalculationContext::default();
    session.calculate("subtotal = 40", &context).unwrap();
    let error = session.calculate("subtotla + 2", &context).unwrap_err();
    assert_eq!(error.kind(), DiagnosticKind::UnknownVariable);
    assert_eq!(error.detail(), Some("subtotla; did you mean 'subtotal'?"));
}

#[test]
fn functions_validate_arity_parameters_and_recursion() {
    let duplicate = calculate_document("f(x, X) = x").unwrap_err();
    assert_eq!(duplicate.kind(), DiagnosticKind::InvalidAssignment);

    let mut session = CalculationSession::new();
    let context = CalculationContext::default();
    session.calculate("loop(x) = loop(x)", &context).unwrap();
    assert_eq!(
        session.calculate("loop(1)", &context).unwrap_err().kind(),
        DiagnosticKind::RecursiveFunction
    );
    assert_eq!(
        session.calculate("loop()", &context).unwrap_err().kind(),
        DiagnosticKind::InvalidFunctionArguments
    );
}
