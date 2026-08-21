use crate::{Diagnostic, DiagnosticKind, Span};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LexemeKind {
    Number,
    Word,
    Whitespace,
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Percent,
    LeftParenthesis,
    RightParenthesis,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lexeme {
    pub kind: LexemeKind,
    pub span: Span,
}

pub fn lex(source: &str) -> Result<Vec<Lexeme>, Diagnostic> {
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
            || (character == '.' && next_byte_is_digit(source, index + 1))
        {
            index = scan_number(source, index);
            LexemeKind::Number
        } else if character.is_alphabetic() {
            index += character.len_utf8();
            while index < source.len() {
                let next = source[index..]
                    .chars()
                    .next()
                    .expect("valid character boundary");
                if !next.is_alphabetic() {
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
                '%' => LexemeKind::Percent,
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

fn scan_number(source: &str, start: usize) -> usize {
    let bytes = source.as_bytes();
    let mut index = start;

    if bytes[index] == b'.' {
        index += 1;
        index = scan_digits(bytes, index);
    } else {
        index = scan_digits(bytes, index);
        if bytes.get(index) == Some(&b'.') {
            index += 1;
            index = scan_digits(bytes, index);
        }
    }

    if matches!(bytes.get(index), Some(b'e' | b'E')) {
        let mut exponent_end = index + 1;
        if matches!(bytes.get(exponent_end), Some(b'+' | b'-')) {
            exponent_end += 1;
        }
        let digits_start = exponent_end;
        exponent_end = scan_digits(bytes, exponent_end);
        if bytes[digits_start..exponent_end]
            .iter()
            .any(u8::is_ascii_digit)
        {
            index = exponent_end;
        }
    }

    index
}

fn scan_digits(bytes: &[u8], mut index: usize) -> usize {
    while matches!(bytes.get(index), Some(byte) if byte.is_ascii_digit() || *byte == b'_') {
        index += 1;
    }
    index
}

fn next_byte_is_digit(source: &str, index: usize) -> bool {
    source.as_bytes().get(index).is_some_and(u8::is_ascii_digit)
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
