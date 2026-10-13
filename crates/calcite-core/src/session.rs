use std::collections::HashMap;

use crate::assignment::parse_assignment;
use crate::evaluator::{Value, evaluate_expression};
use crate::syntax::identifier_key;
use crate::{
    Calculation, CalculationContext, Diagnostic, EntropySource, Expression, LexemeKind,
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
        let mut entropy = None;
        self.calculate_with_optional_entropy(source, context, &mut entropy)
    }

    /// Evaluates one stateful expression with caller-supplied entropy.
    pub fn calculate_with_entropy(
        &mut self,
        source: &str,
        context: &CalculationContext,
        entropy: &mut impl EntropySource,
    ) -> Result<Calculation, Diagnostic> {
        let mut entropy = Some(entropy as &mut dyn EntropySource);
        self.calculate_with_optional_entropy(source, context, &mut entropy)
    }

    fn calculate_with_optional_entropy(
        &mut self,
        source: &str,
        context: &CalculationContext,
        entropy: &mut Option<&mut dyn EntropySource>,
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
        let value = evaluate_expression(&expression, context, &self.variables, entropy)
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

    /// Returns the session's variable names, including `ans` and `_`.
    pub fn variable_names(&self) -> impl Iterator<Item = &str> {
        self.variables.keys().map(String::as_str)
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.variables.get(&identifier_key(name))
    }
}
