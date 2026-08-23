use crate::{
    CalculationContext, Diagnostic, DiagnosticKind, Expression, Function, Lexeme, LexemeKind,
    parse_with_context,
};

pub(crate) struct Assignment<'a> {
    pub name: Option<&'a str>,
    pub parameters: Option<Vec<String>>,
    pub expression_source: &'a str,
    pub expression_offset: usize,
}

pub(crate) fn parse_assignment<'a>(
    source: &'a str,
    tokens: &[Lexeme],
    context: &CalculationContext,
) -> Result<Assignment<'a>, Diagnostic> {
    let Some(equals_index) = tokens
        .iter()
        .position(|token| token.kind == LexemeKind::Equal)
    else {
        return Ok(Assignment {
            name: None,
            parameters: None,
            expression_source: source,
            expression_offset: 0,
        });
    };

    let name_token = tokens.first().copied().ok_or_else(|| {
        Diagnostic::new(DiagnosticKind::InvalidAssignment, tokens[equals_index].span)
    })?;
    if name_token.kind != LexemeKind::Word {
        return Err(invalid_target(tokens[equals_index]));
    }
    let name = &source[name_token.span.start..name_token.span.end];
    let parameters = if equals_index == 1 {
        None
    } else {
        Some(parse_parameters(source, tokens, equals_index, context)?)
    };

    let valid_name = matches!(
        parse_with_context(name, context),
        Ok(Expression::Variable { .. })
    ) && parameters
        .as_ref()
        .is_none_or(|_| Function::from_name(name).is_none());
    if !valid_name {
        return Err(Diagnostic::with_detail(
            DiagnosticKind::InvalidAssignment,
            name_token.span,
            "the name conflicts with calculator syntax",
        ));
    }

    let equals = tokens[equals_index];
    let remainder = &source[equals.span.end..];
    let expression_source = remainder.trim_start();
    if expression_source.is_empty() {
        return Err(Diagnostic::with_detail(
            DiagnosticKind::InvalidAssignment,
            equals.span,
            "expected an expression after '='",
        ));
    }
    Ok(Assignment {
        name: Some(name),
        parameters,
        expression_source,
        expression_offset: equals.span.end + remainder.find(expression_source).unwrap_or(0),
    })
}

fn parse_parameters(
    source: &str,
    tokens: &[Lexeme],
    equals_index: usize,
    context: &CalculationContext,
) -> Result<Vec<String>, Diagnostic> {
    if tokens.get(1).map(|token| token.kind) != Some(LexemeKind::LeftParenthesis)
        || tokens.get(equals_index - 1).map(|token| token.kind)
            != Some(LexemeKind::RightParenthesis)
    {
        return Err(invalid_target(tokens[equals_index]));
    }
    let mut parameters = Vec::new();
    let mut index = 2;
    while index < equals_index - 1 {
        let token = tokens[index];
        if token.kind != LexemeKind::Word {
            return Err(invalid_target(tokens[equals_index]));
        }
        let parameter = &source[token.span.start..token.span.end];
        if !matches!(
            parse_with_context(parameter, context),
            Ok(Expression::Variable { .. })
        ) || parameters
            .iter()
            .any(|existing: &String| existing.eq_ignore_ascii_case(parameter))
        {
            return Err(Diagnostic::with_detail(
                DiagnosticKind::InvalidAssignment,
                token.span,
                "function parameters must be unique variable names",
            ));
        }
        parameters.push(parameter.to_owned());
        index += 1;
        if index < equals_index - 1 {
            if tokens[index].kind != LexemeKind::Comma {
                return Err(invalid_target(tokens[equals_index]));
            }
            index += 1;
        }
    }
    Ok(parameters)
}

fn invalid_target(equals: Lexeme) -> Diagnostic {
    Diagnostic::with_detail(
        DiagnosticKind::InvalidAssignment,
        equals.span,
        "expected a variable or function signature before '='",
    )
}
