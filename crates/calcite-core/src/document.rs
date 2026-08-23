use std::collections::HashMap;

use crate::assignment::parse_assignment;
use crate::evaluator::evaluate_expression;
use crate::syntax::identifier_key;
use crate::{
    CalculationContext, Diagnostic, Expression, LexemeKind, Span, Value, format_with_context,
    lex_with_number_format, parse_with_context,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentDependency {
    pub name: String,
    pub source_result: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DocumentLine {
    pub name: Option<String>,
    pub dependencies: Vec<DocumentDependency>,
    pub span: Span,
    pub value: Value,
    pub output: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DocumentEntryKind {
    Blank,
    Heading {
        text: String,
    },
    Comment {
        text: String,
    },
    Calculation {
        result_index: usize,
        label: Option<String>,
        comment: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentEntry {
    pub span: Span,
    pub kind: DocumentEntryKind,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DocumentCalculation {
    pub lines: Vec<DocumentLine>,
    pub entries: Vec<DocumentEntry>,
}

pub fn calculate_document(source: &str) -> Result<DocumentCalculation, Diagnostic> {
    calculate_document_with_context(source, &CalculationContext::default())
}

pub fn calculate_document_with_context(
    source: &str,
    context: &CalculationContext,
) -> Result<DocumentCalculation, Diagnostic> {
    let mut variables = HashMap::new();
    let mut definitions = HashMap::new();
    let mut lines = Vec::new();
    let mut entries = Vec::new();
    let mut source_offset = 0;

    for raw_line in source.split_inclusive('\n') {
        let line = raw_line.strip_suffix('\n').unwrap_or(raw_line);
        let line = line.strip_suffix('\r').unwrap_or(line);
        evaluate_line(
            line,
            source_offset,
            context,
            &mut variables,
            &mut definitions,
            &mut lines,
            &mut entries,
        )?;
        source_offset += raw_line.len();
    }

    Ok(DocumentCalculation { lines, entries })
}

fn evaluate_line(
    line: &str,
    source_offset: usize,
    context: &CalculationContext,
    variables: &mut HashMap<String, Value>,
    definitions: &mut HashMap<String, usize>,
    lines: &mut Vec<DocumentLine>,
    entries: &mut Vec<DocumentEntry>,
) -> Result<(), Diagnostic> {
    let raw_content = line.trim();
    if raw_content.is_empty() {
        entries.push(DocumentEntry {
            span: Span::new(source_offset, source_offset + line.len()),
            kind: DocumentEntryKind::Blank,
        });
        return Ok(());
    }
    let raw_content_offset = source_offset
        + line
            .find(raw_content)
            .expect("trimmed content came from line");
    let entry_span = Span::new(raw_content_offset, raw_content_offset + raw_content.len());

    if let Some(heading) = raw_content.strip_prefix('#') {
        entries.push(DocumentEntry {
            span: entry_span,
            kind: DocumentEntryKind::Heading {
                text: heading.trim_start_matches('#').trim().to_owned(),
            },
        });
        return Ok(());
    }

    let (content_before_comment, comment) = match comment_separator(raw_content) {
        Some(position) => (
            raw_content[..position].trim_end(),
            Some(raw_content[position + 2..].trim().to_owned()),
        ),
        None => (raw_content, None),
    };
    if content_before_comment.is_empty() {
        entries.push(DocumentEntry {
            span: entry_span,
            kind: DocumentEntryKind::Comment {
                text: comment.unwrap_or_default(),
            },
        });
        return Ok(());
    }

    let mut content = content_before_comment;
    let mut content_offset = raw_content_offset;
    let mut label = None;
    if let Some(position) = label_separator(content) {
        label = Some(content[..position].trim().to_owned());
        let remainder = &content[position + 1..];
        content = remainder.trim_start();
        content_offset += position + 1 + remainder.find(content).unwrap_or(0);
    }

    let tokens: Vec<_> = lex_with_number_format(content, context.number_format)
        .map_err(|error| error.shifted(content_offset))?
        .into_iter()
        .filter(|token| token.kind != LexemeKind::Whitespace)
        .collect();

    let assignment = parse_assignment(content, &tokens, context)
        .map_err(|error| error.shifted(content_offset))?;
    let name = assignment.name.map(str::to_owned);
    let expression_offset = content_offset + assignment.expression_offset;
    let expression = parse_with_context(assignment.expression_source, context)
        .map_err(|error| error.shifted(expression_offset))?;
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
    let mut references = Vec::new();
    collect_variable_references(&expression, &mut references);
    let dependencies = references
        .into_iter()
        .map(|name| DocumentDependency {
            source_result: definitions.get(&identifier_key(&name)).copied(),
            name,
        })
        .collect();
    let value = evaluate_expression(&expression, context, variables)
        .map_err(|error| error.shifted(expression_offset))?;
    let output = format_with_context(&value, context);
    let result_index = lines.len();
    if let Some(name) = &name {
        let key = identifier_key(name);
        variables.insert(key.clone(), value.clone());
        definitions.insert(key, result_index);
    }
    lines.push(DocumentLine {
        name,
        dependencies,
        span: Span::new(content_offset, content_offset + content.len()),
        value,
        output,
    });
    entries.push(DocumentEntry {
        span: entry_span,
        kind: DocumentEntryKind::Calculation {
            result_index,
            label,
            comment,
        },
    });
    Ok(())
}

fn collect_variable_references(expression: &Expression, references: &mut Vec<String>) {
    match expression {
        Expression::Variable { name, .. } => {
            if !references.contains(name) {
                references.push(name.clone());
            }
        }
        Expression::Call { arguments, .. } => {
            for argument in arguments {
                collect_variable_references(argument, references);
            }
        }
        Expression::UserCall {
            name, arguments, ..
        } => {
            if !references.contains(name) {
                references.push(name.clone());
            }
            for argument in arguments {
                collect_variable_references(argument, references);
            }
        }
        Expression::Lambda {
            parameters, body, ..
        } => {
            let mut body_references = Vec::new();
            collect_variable_references(body, &mut body_references);
            for reference in body_references {
                if !parameters
                    .iter()
                    .any(|parameter| identifier_key(parameter) == identifier_key(&reference))
                    && !references.contains(&reference)
                {
                    references.push(reference);
                }
            }
        }
        Expression::Invoke {
            function,
            arguments,
            ..
        } => {
            collect_variable_references(function, references);
            for argument in arguments {
                collect_variable_references(argument, references);
            }
        }
        Expression::Unary { operand, .. }
        | Expression::Group {
            expression: operand,
            ..
        }
        | Expression::Percentage {
            expression: operand,
            ..
        }
        | Expression::Conversion {
            expression: operand,
            ..
        } => collect_variable_references(operand, references),
        Expression::Conditional {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            collect_variable_references(condition, references);
            collect_variable_references(then_branch, references);
            collect_variable_references(else_branch, references);
        }
        Expression::Binary { left, right, .. } => {
            collect_variable_references(left, references);
            collect_variable_references(right, references);
        }
        Expression::Number { .. }
        | Expression::String { .. }
        | Expression::RadixInteger { .. }
        | Expression::RomanInteger { .. }
        | Expression::Complex { .. }
        | Expression::Boolean { .. }
        | Expression::Constant { .. }
        | Expression::Unit { .. }
        | Expression::CustomUnit { .. }
        | Expression::Money { .. }
        | Expression::Dice { .. }
        | Expression::Date { .. }
        | Expression::PartialDate { .. }
        | Expression::Clock { .. }
        | Expression::OffsetClock { .. }
        | Expression::ZonedClock { .. }
        | Expression::OffsetDateTime { .. }
        | Expression::CalendarDuration { .. }
        | Expression::WorkdayDuration { .. }
        | Expression::TemporalKeyword { .. }
        | Expression::RelativeWeekday { .. } => {}
    }
}

fn comment_separator(content: &str) -> Option<usize> {
    let mut quote = None;
    let mut escaped = false;
    let mut characters = content.char_indices().peekable();
    while let Some((position, character)) = characters.next() {
        if escaped {
            escaped = false;
            continue;
        }
        if quote.is_some() && character == '\\' {
            escaped = true;
            continue;
        }
        if matches!(character, '\'' | '"') {
            if quote == Some(character) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(character);
            }
            continue;
        }
        if quote.is_none()
            && character == '/'
            && characters.peek().is_some_and(|(_, next)| *next == '/')
        {
            return Some(position);
        }
    }
    None
}

fn label_separator(content: &str) -> Option<usize> {
    content.char_indices().find_map(|(position, character)| {
        if character != ':' {
            return None;
        }
        let label = content[..position].trim();
        let remainder = &content[position + 1..];
        let starts_like_label = label.chars().next().is_some_and(char::is_alphabetic)
            && label.chars().any(char::is_alphabetic);
        let separated = remainder.chars().next().is_some_and(char::is_whitespace);
        (starts_like_label && separated && !remainder.trim().is_empty()).then_some(position)
    })
}
