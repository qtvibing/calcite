use std::collections::HashMap;

use crate::evaluator::evaluate_expression;
use crate::{
    CalculationContext, Diagnostic, DiagnosticKind, Expression, LexemeKind, Span, Value,
    format_with_context, lex_with_number_format, parse_with_context,
};

#[derive(Clone, Debug, PartialEq)]
pub struct DocumentLine {
    pub name: Option<String>,
    pub span: Span,
    pub value: Value,
    pub output: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DocumentCalculation {
    pub lines: Vec<DocumentLine>,
}

pub fn calculate_document(source: &str) -> Result<DocumentCalculation, Diagnostic> {
    calculate_document_with_context(source, &CalculationContext::default())
}

pub fn calculate_document_with_context(
    source: &str,
    context: &CalculationContext,
) -> Result<DocumentCalculation, Diagnostic> {
    let mut variables = HashMap::new();
    let mut lines = Vec::new();
    let mut source_offset = 0;

    for raw_line in source.split_inclusive('\n') {
        let line = raw_line.strip_suffix('\n').unwrap_or(raw_line);
        evaluate_line(line, source_offset, context, &mut variables, &mut lines)?;
        source_offset += raw_line.len();
    }

    Ok(DocumentCalculation { lines })
}

fn evaluate_line(
    line: &str,
    source_offset: usize,
    context: &CalculationContext,
    variables: &mut HashMap<String, Value>,
    lines: &mut Vec<DocumentLine>,
) -> Result<(), Diagnostic> {
    let content = line.trim();
    if content.is_empty() {
        return Ok(());
    }
    let content_offset =
        source_offset + line.find(content).expect("trimmed content came from line");
    let tokens: Vec<_> = lex_with_number_format(content, context.number_format)
        .map_err(|error| error.shifted(content_offset))?
        .into_iter()
        .filter(|token| token.kind != LexemeKind::Whitespace)
        .collect();

    let assignment = tokens
        .iter()
        .position(|token| token.kind == LexemeKind::Equal);
    let (name, expression_source, expression_offset) = match assignment {
        None => (None, content, content_offset),
        Some(1)
            if tokens
                .first()
                .is_some_and(|token| token.kind == LexemeKind::Word) =>
        {
            let name_token = tokens[0];
            let name = &content[name_token.span.start..name_token.span.end];
            let valid_name = matches!(
                parse_with_context(name, context),
                Ok(Expression::Variable { .. })
            );
            if !valid_name {
                return Err(Diagnostic::with_detail(
                    DiagnosticKind::InvalidAssignment,
                    name_token.span.shifted(content_offset),
                    "the name conflicts with calculator syntax",
                ));
            }

            let equals = tokens[1];
            let remainder = &content[equals.span.end..];
            let expression_source = remainder.trim_start();
            if expression_source.is_empty() {
                return Err(Diagnostic::with_detail(
                    DiagnosticKind::InvalidAssignment,
                    equals.span.shifted(content_offset),
                    "expected an expression after '='",
                ));
            }
            let expression_offset =
                content_offset + equals.span.end + remainder.find(expression_source).unwrap();
            (Some(name.to_owned()), expression_source, expression_offset)
        }
        Some(position) => {
            return Err(Diagnostic::with_detail(
                DiagnosticKind::InvalidAssignment,
                tokens[position].span.shifted(content_offset),
                "expected a variable name before '='",
            ));
        }
    };

    let expression = parse_with_context(expression_source, context)
        .map_err(|error| error.shifted(expression_offset))?;
    let value = evaluate_expression(&expression, context, variables)
        .map_err(|error| error.shifted(expression_offset))?;
    let output = format_with_context(&value, context);
    if let Some(name) = &name {
        variables.insert(name.clone(), value.clone());
    }
    lines.push(DocumentLine {
        name,
        span: Span::new(content_offset, content_offset + content.len()),
        value,
        output,
    });
    Ok(())
}
