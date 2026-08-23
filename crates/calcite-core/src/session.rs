use std::collections::HashMap;

use crate::assignment::parse_assignment;
use crate::evaluator::{Value, evaluate_expression};
use crate::syntax::identifier_key;
use crate::{
    Calculation, CalculationContext, Diagnostic, Expression, LexemeKind, format_with_context,
    lex_with_number_format, parse_with_context,
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
        let assignment = parse_assignment(source, &tokens, context)?;
        let name = assignment.name.map(str::to_owned);
        let expression = parse_with_context(assignment.expression_source, context)
            .map_err(|error| error.shifted(assignment.expression_offset))?;
        let expression = if let Some(parameters) = assignment.parameters {
            let span = expression.span();
            Expression::Lambda {
                parameters,
                body: Box::new(expression),
                span,
            }
        } else {
            expression
        };
        let value = evaluate_expression(&expression, context, &self.variables)
            .map_err(|error| error.shifted(assignment.expression_offset))?;
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
