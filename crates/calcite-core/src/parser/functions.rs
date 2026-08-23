use super::Parser;
use crate::{Constant, Diagnostic, DiagnosticKind, Expression, Function, Lexeme, LexemeKind, Span};

impl Parser<'_> {
    pub(super) fn constant_at(&self) -> Option<Constant> {
        let token = self.current()?;
        if token.kind != LexemeKind::Word {
            return None;
        }
        let name = &self.source[token.span.start..token.span.end];
        if name.eq_ignore_ascii_case("pi") || name == "π" {
            Some(Constant::Pi)
        } else if name.eq_ignore_ascii_case("tau") || name == "τ" {
            Some(Constant::Tau)
        } else if name.eq_ignore_ascii_case("e") {
            Some(Constant::E)
        } else {
            None
        }
    }

    pub(super) fn function_call_at(&self) -> Option<Function> {
        let token = self.current()?;
        if token.kind != LexemeKind::Word
            || self.tokens.get(self.position + 1)?.kind != LexemeKind::LeftParenthesis
        {
            return None;
        }
        Function::from_name(&self.source[token.span.start..token.span.end])
    }

    pub(super) fn natural_function_at(&self) -> Option<(Function, usize)> {
        const PHRASES: &[(Function, &[&str])] = &[
            (
                Function::GreatestCommonDivisor,
                &["greatest", "common", "divisor", "of"],
            ),
            (
                Function::LeastCommonMultiple,
                &["least", "common", "multiple", "of"],
            ),
            (Function::SquareRoot, &["square", "root", "of"]),
            (Function::CubicRoot, &["cube", "root", "of"]),
            (Function::NaturalLogarithm, &["natural", "log", "of"]),
            (Function::NaturalLogarithm, &["natural", "logarithm", "of"]),
            (Function::CommonLogarithm, &["common", "log", "of"]),
            (Function::CommonLogarithm, &["common", "logarithm", "of"]),
            (Function::BinaryLogarithm, &["binary", "log", "of"]),
            (Function::BinaryLogarithm, &["binary", "logarithm", "of"]),
            (Function::HyperbolicSine, &["hyperbolic", "sine", "of"]),
            (Function::HyperbolicCosine, &["hyperbolic", "cosine", "of"]),
            (
                Function::HyperbolicTangent,
                &["hyperbolic", "tangent", "of"],
            ),
            (Function::NextWorkday, &["next", "business", "day", "after"]),
            (Function::NextWorkday, &["next", "workday", "after"]),
            (Function::NextWorkday, &["next", "weekday", "after"]),
            (
                Function::PreviousWorkday,
                &["previous", "business", "day", "before"],
            ),
            (
                Function::PreviousWorkday,
                &["last", "business", "day", "before"],
            ),
            (
                Function::PreviousWorkday,
                &["previous", "workday", "before"],
            ),
            (
                Function::PreviousWorkday,
                &["previous", "weekday", "before"],
            ),
            (Function::PreviousWorkday, &["last", "workday", "before"]),
            (Function::PreviousWorkday, &["last", "weekday", "before"]),
            (Function::Weekday, &["day", "of", "the", "week", "on"]),
            (Function::Weekday, &["weekday", "on"]),
            (Function::AbsoluteValue, &["absolute", "value", "of"]),
            (Function::Exponential, &["exponential", "of"]),
            (Function::IsPrime, &["is", "prime"]),
        ];

        if let Some((function, words)) = PHRASES.iter().find(|(_, words)| self.phrase_is(words)) {
            return Some((*function, words.len()));
        }
        if self.word_is(1, "of") {
            let token = self.current()?;
            return Function::from_name(&self.source[token.span.start..token.span.end])
                .map(|function| (function, 2));
        }
        self.word_is(0, "round").then_some((Function::Round, 1))
    }

    pub(super) fn phrase_is(&self, words: &[&str]) -> bool {
        words
            .iter()
            .enumerate()
            .all(|(offset, word)| self.word_is(offset, word))
    }

    pub(super) fn workday_range_at(&self) -> Option<usize> {
        if (self.word_is(0, "workdays") || self.word_is(0, "weekdays")) && self.word_is(1, "from") {
            Some(2)
        } else if self.word_is(0, "business") && self.word_is(1, "days") && self.word_is(2, "from")
        {
            Some(3)
        } else {
            None
        }
    }

    pub(super) fn parse_workday_range(&mut self) -> Result<Expression, Diagnostic> {
        let consumed = self
            .workday_range_at()
            .expect("guard checked the workday range");
        let function_span = self.phrase_span(consumed);
        self.position += consumed;
        let start = self.parse_expression(1)?;
        let Some(separator) = self.current() else {
            return Err(Diagnostic::with_detail(
                DiagnosticKind::UnexpectedEnd,
                Span::empty(self.source.len()),
                "expected 'to' and an ending date",
            ));
        };
        if !self.word_is(0, "to") {
            return Err(Diagnostic::with_detail(
                DiagnosticKind::UnexpectedToken,
                separator.span,
                "expected 'to' between the dates",
            ));
        }
        self.position += 1;
        let end = self.parse_expression(0)?;
        let span = function_span.join(end.span());
        Ok(Expression::Call {
            function: Function::WorkdaysBetween,
            function_span,
            arguments: vec![start, end],
            span,
        })
    }

    pub(super) fn parse_function_call(&mut self) -> Result<Expression, Diagnostic> {
        let function_token = self.current().expect("function name is present");
        let function = self
            .function_call_at()
            .expect("guard checked the function call");
        self.position += 2;
        let mut arguments = Vec::new();
        if !self
            .current()
            .is_some_and(|token| token.kind == LexemeKind::RightParenthesis)
        {
            loop {
                arguments.push(self.parse_expression(0)?);
                let Some(separator) = self.current() else {
                    return Err(Diagnostic::with_detail(
                        DiagnosticKind::UnexpectedEnd,
                        Span::empty(self.source.len()),
                        "expected ')'",
                    ));
                };
                if separator.kind == LexemeKind::Comma {
                    self.position += 1;
                    continue;
                }
                if separator.kind != LexemeKind::RightParenthesis {
                    return Err(Diagnostic::with_detail(
                        DiagnosticKind::UnexpectedToken,
                        separator.span,
                        "expected ',' or ')'",
                    ));
                }
                break;
            }
        }
        let closing = self.current().expect("closing parenthesis was checked");
        self.position += 1;
        Ok(Expression::Call {
            function,
            function_span: function_token.span,
            arguments,
            span: function_token.span.join(closing.span),
        })
    }

    pub(super) fn consume_word(&mut self, word: &str, detail: &str) -> Result<Lexeme, Diagnostic> {
        let token = self.current().ok_or_else(|| {
            Diagnostic::with_detail(
                DiagnosticKind::UnexpectedEnd,
                Span::empty(self.source.len()),
                detail,
            )
        })?;
        if !self.word_is(0, word) {
            return Err(Diagnostic::with_detail(
                DiagnosticKind::UnexpectedToken,
                token.span,
                detail,
            ));
        }
        self.position += 1;
        Ok(token)
    }

    pub(super) fn parse_natural_remainder(&mut self) -> Result<Expression, Diagnostic> {
        let start = self.consume_word("remainder", "expected 'remainder'")?.span;
        self.consume_word("of", "expected 'of'")?;
        let dividend = self.parse_expression(18)?;
        let operator = self.consume_word("divided", "expected 'divided by'")?;
        self.consume_word("by", "expected 'divided by'")?;
        let divisor = self.parse_expression(0)?;
        let span = start.join(divisor.span());
        Ok(Expression::Binary {
            left: Box::new(dividend),
            operator: crate::BinaryOperator::Remainder,
            operator_span: operator.span,
            right: Box::new(divisor),
            span,
        })
    }

    pub(super) fn parse_natural_clamp(&mut self) -> Result<Expression, Diagnostic> {
        let start = self.consume_word("clamp", "expected 'clamp'")?.span;
        let value = self.parse_expression(0)?;
        let separator = self.current().ok_or_else(|| {
            Diagnostic::with_detail(
                DiagnosticKind::UnexpectedEnd,
                Span::empty(self.source.len()),
                "expected 'between' or 'from'",
            )
        })?;
        let uses_between = self.word_is(0, "between");
        if !uses_between && !self.word_is(0, "from") {
            return Err(Diagnostic::with_detail(
                DiagnosticKind::UnexpectedToken,
                separator.span,
                "expected 'between' or 'from'",
            ));
        }
        self.position += 1;
        let lower = self.parse_expression(4)?;
        self.consume_word(
            if uses_between { "and" } else { "to" },
            if uses_between {
                "expected 'and'"
            } else {
                "expected 'to'"
            },
        )?;
        let upper = self.parse_expression(0)?;
        let span = start.join(upper.span());
        Ok(Expression::Call {
            function: Function::Clamp,
            function_span: start,
            arguments: vec![value, lower, upper],
            span,
        })
    }

    pub(super) fn parse_natural_midpoint(&mut self) -> Result<Expression, Diagnostic> {
        let start = self.consume_word("midpoint", "expected 'midpoint'")?.span;
        self.consume_word("between", "expected 'between'")?;
        let first = self.parse_expression(4)?;
        self.consume_word("and", "expected 'and'")?;
        let second = self.parse_expression(0)?;
        let span = start.join(second.span());
        Ok(Expression::Call {
            function: Function::Midpoint,
            function_span: start,
            arguments: vec![first, second],
            span,
        })
    }

    pub(super) fn parse_proportion(&mut self, first: Expression) -> Result<Expression, Diagnostic> {
        let start = first.span();
        self.consume_word("is", "expected 'is'")?;
        self.consume_word("to", "expected 'to'")?;
        let second = self.parse_expression(1)?;
        self.consume_word("as", "expected 'as'")?;
        let (function, third, end) = if self.word_is(0, "what") {
            self.position += 1;
            self.consume_word("is", "expected 'is'")?;
            self.consume_word("to", "expected 'to'")?;
            let third = self.parse_expression(0)?;
            let end = third.span();
            (Function::ProportionNumerator, third, end)
        } else {
            let third = self.parse_expression(1)?;
            self.consume_word("is", "expected 'is'")?;
            self.consume_word("to", "expected 'to'")?;
            let what = self.consume_word("what", "expected 'what'")?;
            (Function::ProportionDenominator, third, what.span)
        };
        let span = start.join(end);
        Ok(Expression::Call {
            function,
            function_span: start,
            arguments: vec![first, second, third],
            span,
        })
    }

    pub(super) fn natural_finance_at(&self) -> Option<(Function, usize)> {
        const PHRASES: &[(Function, &[&str])] = &[
            (Function::MortgagePayment, &["mortgage", "payment", "on"]),
            (Function::TotalRepayment, &["total", "repayment", "on"]),
            (Function::TotalInterest, &["total", "interest", "on"]),
            (Function::FutureValue, &["future", "value", "of"]),
            (Function::PresentValue, &["present", "value", "of"]),
            (Function::CompoundInterest, &["compound", "interest", "on"]),
            (Function::PurchasingPower, &["purchasing", "power", "of"]),
            (
                Function::ReturnOnInvestment,
                &["return", "on", "investment", "from"],
            ),
            (Function::CompoundAnnualGrowthRate, &["cagr", "from"]),
            (Function::SalesTax, &["sales", "tax", "on"]),
            (Function::AddTax, &["add", "tax", "to"]),
            (Function::RemoveTax, &["remove", "tax", "from"]),
            (Function::TaxFromTotal, &["tax", "from", "total"]),
        ];
        PHRASES
            .iter()
            .find(|(_, words)| self.phrase_is(words))
            .map(|(function, words)| (*function, words.len()))
    }

    pub(super) fn parse_natural_finance(&mut self) -> Result<Expression, Diagnostic> {
        let (function, consumed) = self
            .natural_finance_at()
            .expect("guard checked the finance phrase");
        let function_span = self.phrase_span(consumed);
        self.position += consumed;
        let first = self.parse_expression(14)?;
        let mut arguments = vec![first];

        if matches!(
            function,
            Function::ReturnOnInvestment | Function::CompoundAnnualGrowthRate
        ) {
            self.consume_word("to", "expected 'to' and a final value")?;
            let final_binding_power = if function == Function::CompoundAnnualGrowthRate {
                18
            } else {
                0
            };
            arguments.push(self.parse_expression(final_binding_power)?);
            if function == Function::CompoundAnnualGrowthRate {
                self.consume_word("over", "expected 'over' and a period count")?;
                arguments.push(self.parse_expression(0)?);
            }
        } else {
            self.consume_word("at", "expected 'at' and a percentage rate")?;
            arguments.push(self.parse_expression(1)?);
            if matches!(
                function,
                Function::FutureValue
                    | Function::PresentValue
                    | Function::CompoundInterest
                    | Function::PurchasingPower
                    | Function::MortgagePayment
                    | Function::TotalRepayment
                    | Function::TotalInterest
            ) {
                self.consume_word("for", "expected 'for' and a term")?;
                arguments.push(self.parse_expression(0)?);
            }
        }

        let span = function_span.join(
            arguments
                .last()
                .expect("finance phrases parse at least one argument")
                .span(),
        );
        Ok(Expression::Call {
            function,
            function_span,
            arguments,
            span,
        })
    }

    pub(super) fn parse_natural_function(&mut self) -> Result<Expression, Diagnostic> {
        let (function, consumed) = self
            .natural_function_at()
            .expect("guard checked the natural function");
        let function_span = self.phrase_span(consumed);
        self.position += consumed;
        let accepts_list = matches!(
            function,
            Function::Sum
                | Function::GreatestCommonDivisor
                | Function::LeastCommonMultiple
                | Function::Average
                | Function::Minimum
                | Function::Maximum
                | Function::Median
                | Function::Count
                | Function::Product
        );
        let argument_binding_power = if accepts_list { 4 } else { 0 };
        let mut arguments = vec![self.parse_expression(argument_binding_power)?];
        if accepts_list {
            loop {
                if self
                    .current()
                    .is_some_and(|token| token.kind == LexemeKind::Comma)
                {
                    self.position += 1;
                    let has_final_and = self.word_is(0, "and");
                    if has_final_and {
                        self.position += 1;
                    }
                    arguments.push(self.parse_expression(argument_binding_power)?);
                    if has_final_and {
                        break;
                    }
                } else if self.word_is(0, "and") {
                    self.position += 1;
                    arguments.push(self.parse_expression(argument_binding_power)?);
                    break;
                } else {
                    break;
                }
            }
        }
        let span = function_span.join(
            arguments
                .last()
                .expect("natural functions parse at least one argument")
                .span(),
        );
        Ok(Expression::Call {
            function,
            function_span,
            arguments,
            span,
        })
    }
}
