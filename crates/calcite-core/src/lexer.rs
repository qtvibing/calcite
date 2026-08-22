use crate::{Diagnostic, DiagnosticKind, NumberFormat, Span};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LexemeKind {
    Number,
    Word,
    CurrencySymbol,
    Whitespace,
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Colon,
    Comma,
    Percent,
    Equal,
    EqualEqual,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    LogicalAnd,
    LogicalOr,
    BitwiseAnd,
    BitwiseOr,
    ShiftLeft,
    ShiftRight,
    LogicalNot,
    LeftParenthesis,
    RightParenthesis,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lexeme {
    pub kind: LexemeKind,
    pub span: Span,
}

pub fn lex(source: &str) -> Result<Vec<Lexeme>, Diagnostic> {
    lex_with_number_format(source, NumberFormat::default())
}

pub fn lex_with_number_format(
    source: &str,
    number_format: NumberFormat,
) -> Result<Vec<Lexeme>, Diagnostic> {
    let mut lexemes = Vec::new();
    let mut index = 0;

    while index < source.len() {
        let character = source[index..]
            .chars()
            .next()
            .expect("valid character boundary");
        let start = index;

        let kind = if character.is_whitespace() {
            index += character.len_utf8();
            while index < source.len() {
                let next = source[index..]
                    .chars()
                    .next()
                    .expect("valid character boundary");
                if !next.is_whitespace() {
                    break;
                }
                index += next.len_utf8();
            }
            LexemeKind::Whitespace
        } else if character.is_ascii_digit()
            || (character == number_format.decimal_separator()
                && next_character_is_digit(source, index + character.len_utf8()))
        {
            index = scan_number(source, index, number_format);
            LexemeKind::Number
        } else if character.is_alphabetic() || character == '_' {
            index += character.len_utf8();
            while index < source.len() {
                let next = source[index..]
                    .chars()
                    .next()
                    .expect("valid character boundary");
                if !next.is_alphanumeric() && next != '_' {
                    break;
                }
                index += next.len_utf8();
            }
            LexemeKind::Word
        } else {
            index += character.len_utf8();
            match character {
                '+' => LexemeKind::Plus,
                '-' | '−' | '–' => LexemeKind::Minus,
                '*' => {
                    if source[index..].starts_with('*') {
                        index += 1;
                        LexemeKind::Caret
                    } else {
                        LexemeKind::Star
                    }
                }
                '×' => LexemeKind::Star,
                '/' | '÷' => LexemeKind::Slash,
                '^' => LexemeKind::Caret,
                ':' => LexemeKind::Colon,
                ',' => LexemeKind::Comma,
                '%' => LexemeKind::Percent,
                '$' | '€' | '£' | '¥' | '₹' | '₩' => LexemeKind::CurrencySymbol,
                '=' if source[index..].starts_with('=') => {
                    index += 1;
                    LexemeKind::EqualEqual
                }
                '=' => LexemeKind::Equal,
                '!' if source[index..].starts_with('=') => {
                    index += 1;
                    LexemeKind::NotEqual
                }
                '!' => LexemeKind::LogicalNot,
                '>' if source[index..].starts_with('=') => {
                    index += 1;
                    LexemeKind::GreaterEqual
                }
                '>' if source[index..].starts_with('>') => {
                    index += 1;
                    LexemeKind::ShiftRight
                }
                '>' => LexemeKind::Greater,
                '<' if source[index..].starts_with('=') => {
                    index += 1;
                    LexemeKind::LessEqual
                }
                '<' if source[index..].starts_with('<') => {
                    index += 1;
                    LexemeKind::ShiftLeft
                }
                '<' => LexemeKind::Less,
                '&' if source[index..].starts_with('&') => {
                    index += 1;
                    LexemeKind::LogicalAnd
                }
                '&' => LexemeKind::BitwiseAnd,
                '|' if source[index..].starts_with('|') => {
                    index += 1;
                    LexemeKind::LogicalOr
                }
                '|' => LexemeKind::BitwiseOr,
                '(' => LexemeKind::LeftParenthesis,
                ')' => LexemeKind::RightParenthesis,
                _ => {
                    return Err(Diagnostic::with_detail(
                        DiagnosticKind::UnexpectedCharacter,
                        Span::new(start, index),
                        character,
                    ));
                }
            }
        };

        lexemes.push(Lexeme {
            kind,
            span: Span::new(start, index),
        });
    }

    Ok(lexemes)
}

fn scan_number(source: &str, start: usize, number_format: NumberFormat) -> usize {
    let decimal = number_format.decimal_separator();
    let mut index = start;

    if ["0b", "0B", "0o", "0O", "0x", "0X"]
        .iter()
        .any(|prefix| source[start..].starts_with(prefix))
    {
        index += 2;
        while index < source.len() {
            let character = source[index..]
                .chars()
                .next()
                .expect("valid character boundary");
            if !character.is_ascii_alphanumeric() && character != '_' {
                break;
            }
            index += character.len_utf8();
        }
        return index;
    }

    if source[index..].starts_with(decimal) {
        index += decimal.len_utf8();
        index = scan_digits(source, index, None);
    } else {
        index = scan_digits(source, index, number_format.grouping_separator());
        if source[index..].starts_with(decimal) {
            index += decimal.len_utf8();
            index = scan_digits(source, index, None);
        }
    }

    if matches!(source.as_bytes().get(index), Some(b'e' | b'E')) {
        let mut exponent_end = index + 1;
        if matches!(source.as_bytes().get(exponent_end), Some(b'+' | b'-')) {
            exponent_end += 1;
        }
        let digits_start = exponent_end;
        exponent_end = scan_digits(source, exponent_end, None);
        if source.as_bytes()[digits_start..exponent_end]
            .iter()
            .any(u8::is_ascii_digit)
        {
            index = exponent_end;
        }
    }

    index
}

fn scan_digits(source: &str, mut index: usize, separator: Option<char>) -> usize {
    while index < source.len() {
        let character = source[index..]
            .chars()
            .next()
            .expect("valid character boundary");
        if character.is_ascii_digit() || character == '_' {
            index += character.len_utf8();
            continue;
        }
        if Some(character) == separator {
            let after_separator = index + character.len_utf8();
            if let Some(after_group) = scan_group(source, after_separator) {
                index = after_group;
                continue;
            }
        }
        break;
    }
    index
}

fn scan_group(source: &str, mut index: usize) -> Option<usize> {
    for _ in 0..3 {
        let character = source.get(index..)?.chars().next()?;
        if !character.is_ascii_digit() {
            return None;
        }
        index += character.len_utf8();
    }
    if source
        .get(index..)
        .and_then(|remainder| remainder.chars().next())
        .is_some_and(|character| character.is_ascii_digit() || character == '_')
    {
        None
    } else {
        Some(index)
    }
}

fn next_character_is_digit(source: &str, index: usize) -> bool {
    source[index..]
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retains_whitespace_and_byte_spans() {
        let tokens = lex("2 × 10").unwrap();
        assert_eq!(tokens[2].kind, LexemeKind::Star);
        assert_eq!(tokens[2].span, Span::new(2, 4));
        assert_eq!(tokens[3].kind, LexemeKind::Whitespace);
    }

    #[test]
    fn recognizes_bitwise_symbols_and_shifts() {
        let tokens = lex("1 & 2 | 3 << 4 >> 1").unwrap();
        assert!(
            tokens
                .iter()
                .any(|token| token.kind == LexemeKind::BitwiseAnd)
        );
        assert!(
            tokens
                .iter()
                .any(|token| token.kind == LexemeKind::BitwiseOr)
        );
        assert!(
            tokens
                .iter()
                .any(|token| token.kind == LexemeKind::ShiftLeft)
        );
        assert!(
            tokens
                .iter()
                .any(|token| token.kind == LexemeKind::ShiftRight)
        );
    }

    #[test]
    fn recognizes_prefixed_integer_literals_as_single_tokens() {
        let tokens = lex("0xff 0B1010 0o755").unwrap();
        assert_eq!(tokens[0].kind, LexemeKind::Number);
        assert_eq!(tokens[0].span, Span::new(0, 4));
        assert_eq!(tokens[2].kind, LexemeKind::Number);
        assert_eq!(tokens[2].span, Span::new(5, 11));
        assert_eq!(tokens[4].kind, LexemeKind::Number);
        assert_eq!(tokens[4].span, Span::new(12, 17));
    }

    #[test]
    fn recognizes_scientific_numbers() {
        let tokens = lex("1.25e-3").unwrap();
        assert_eq!(
            tokens,
            vec![Lexeme {
                kind: LexemeKind::Number,
                span: Span::new(0, 7)
            }]
        );
    }
}
