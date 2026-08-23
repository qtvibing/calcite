use std::collections::HashMap;

use crate::evaluator::{Value, evaluate_expression};
use crate::syntax::identifier_key;
use crate::{
    Calculation, CalculationContext, Diagnostic, DiagnosticKind, Expression, LexemeKind,
    format_with_context, lex_with_number_format, parse_with_context,
};

#[derive(Clone, Debug, Default)]
pub struct CalculationSession {
    variables: HashMap<String, Value>,
}

impl CalculationSession {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn calculate(
        &mut self,
        source: &str,
        context: &CalculationContext,
    ) -> Result<Calculation, Diagnostic> {
        let tokens = lex_with_number_format(source, context.number_format)?
            .into_iter()
            .filter(|token| token.kind != LexemeKind::Whitespace)
            .collect::<Vec<_>>();
        let assignment = tokens
            .iter()
            .position(|token| token.kind == LexemeKind::Equal);
        let (name, expression_source, expression_offset) = match assignment {
            None => (None, source, 0),
            Some(1)
                if tokens
                    .first()
                    .is_some_and(|token| token.kind == LexemeKind::Word) =>
            {
                let name_token = tokens[0];
                let name = &source[name_token.span.start..name_token.span.end];
                if !matches!(
                    parse_with_context(name, context),
                    Ok(Expression::Variable { .. })
                ) {
                    return Err(Diagnostic::with_detail(
                        DiagnosticKind::InvalidAssignment,
                        name_token.span,
                        "the name conflicts with calculator syntax",
                    ));
                }
                let equals = tokens[1];
                let remainder = &source[equals.span.end..];
                let expression_source = remainder.trim_start();
                if expression_source.is_empty() {
                    return Err(Diagnostic::with_detail(
                        DiagnosticKind::InvalidAssignment,
                        equals.span,
                        "expected an expression after '='",
                    ));
                }
                (
                    Some(name.to_owned()),
                    expression_source,
                    equals.span.end + remainder.find(expression_source).unwrap_or(0),
                )
            }
            Some(position) => {
                return Err(Diagnostic::with_detail(
                    DiagnosticKind::InvalidAssignment,
                    tokens[position].span,
                    "expected a variable name before '='",
                ));
            }
        };

        let expression = parse_with_context(expression_source, context)
            .map_err(|error| error.shifted(expression_offset))?;
        let value = evaluate_expression(&expression, context, &self.variables)
            .map_err(|error| error.shifted(expression_offset))?;
        if let Some(name) = name {
            self.variables.insert(identifier_key(&name), value.clone());
        }
        self.variables.insert("ans".into(), value.clone());
        self.variables.insert("_".into(), value.clone());
        let output = format_with_context(&value, context);
        Ok(Calculation { value, output })
    }

    pub fn clear(&mut self) {
        self.variables.clear();
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.variables.get(&identifier_key(name))
    }
}
