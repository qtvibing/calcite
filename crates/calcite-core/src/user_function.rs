use crate::Expression;

#[derive(Clone, Debug, PartialEq)]
pub struct UserFunction {
    parameters: Vec<String>,
    body: Box<Expression>,
    active: bool,
}

impl UserFunction {
    pub fn new(parameters: Vec<String>, body: Expression) -> Self {
        Self {
            parameters,
            body: Box::new(body),
            active: false,
        }
    }

    pub fn parameters(&self) -> &[String] {
        &self.parameters
    }

    pub fn body(&self) -> &Expression {
        &self.body
    }

    pub(crate) const fn is_active(&self) -> bool {
        self.active
    }

    pub(crate) fn activated(&self) -> Self {
        let mut function = self.clone();
        function.active = true;
        function
    }
}
